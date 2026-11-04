import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface DockItem { kind: "pin" | "running" | "cove" | "stack" | "trash"; name: string; target: string; windows: number[]; active: boolean; attention?: boolean; icon?: string; no_icon?: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
const MOCK: DockItem[] = [
  { kind: "pin", name: "File Explorer", target: "explorer", windows: [1], active: false },
  { kind: "pin", name: "Microsoft Edge", target: "edge", windows: [2], active: true },
  { kind: "pin", name: "Settings", target: "settings", windows: [], active: false },
  { kind: "running", name: "Code", target: "code.exe", windows: [3], active: false },
  { kind: "stack", name: "Downloads", target: "downloads", windows: [], active: false },
  { kind: "trash", name: "Recycle Bin", target: "trash", windows: [], active: false },
];
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  if (cmd === "stack_list") {
    return Promise.resolve(["report.pdf", "photo.png", "setup.exe", "notes.txt", "Project", "song.mp3", "budget.xlsx"].map((n) => ({ name: n, path: n, dir: !n.includes("."), icon: null })) as T);
  }
  return Promise.resolve((cmd === "dock_state" ? { items: MOCK } : null) as T);
}

const SIZE = 48, GAP = 6, MAX = 1.55, RANGE = 150, HEAD = 46; // magnification
const PAD_X = 10, BORDER = 1, SEP_W = 9; // .dock padding / border, .sep width + margins (dock.css)
const ICON_WAIT = 5000; // after this, a still-missing icon shows its letter tile
const waitingSince = new Map<string, number>();
function letterTile(name: string): HTMLElement {
  const t = document.createElement("div");
  t.className = "letter tile";
  t.textContent = (name.trim()[0] ?? "?").toUpperCase();
  return t;
}
const dock = document.getElementById("dock")!;
const menu = document.getElementById("menu")!;
const stack = document.getElementById("stack")!;
const STACK_H = 360; // extra height while a stack is open
let openStack = "";
let items: DockItem[] = [];
let sig = "";
let lastWidth = 0;

let lastHeight = 0;
// Motion state: apps we just launched (bounce until a window shows up, at
// most 6 s), and which items were on the dock last render.
const launching = new Map<string, number>();
let shown = new Set<string>();
let firstRender = true;
const keyOf = (it: DockItem) => `${it.kind}:${it.target}`;
const calm = matchMedia("(prefers-reduced-motion: reduce)");

function place(): void {
  let width = Math.ceil(dock.getBoundingClientRect().width) + 160; // room for magnified neighbours
  let height = SIZE + 7 + 9 + 8 + HEAD + 30;
  if (openStack) { width = Math.max(width, 460); height += STACK_H; }
  if (Math.abs(width - lastWidth) > 2 || height !== lastHeight) {
    lastWidth = width;
    lastHeight = height;
    invoke("dock_place", { width, height });
  }
}

function render(): void {
  // Remember where the old icons were so leaving ones can shrink away in place.
  const before = new Map<string, { el: HTMLElement; idx: number }>();
  dock.querySelectorAll<HTMLElement>(".item:not(.leaving)").forEach((el, idx) => { if (el.dataset.k) before.set(el.dataset.k, { el, idx }); });
  const now = Date.now();
  for (const [k, until] of launching) if (until < now) launching.delete(k);
  dock.innerHTML = "";
  let sepDone = false;
  items.forEach((it, i) => {
    const prev = i > 0 ? items[i - 1].kind : "";
    const firstStack = it.kind === "stack" && prev !== "stack" && prev !== "";
    const runs = it.kind === "running" || it.kind === "cove";
    if ((runs && !sepDone && prev === "pin") || firstStack || (it.kind === "trash" && prev !== "stack" && i > 0)) {
      dock.appendChild(Object.assign(document.createElement("div"), { className: "sep" }));
      if (runs) sepDone = true;
    }
    const el = document.createElement("div");
    const k = keyOf(it);
    if (it.windows.length) launching.delete(k);
    const motion = [
      launching.has(k) ? " launching" : "",
      it.attention ? " attention" : "",
      !firstRender && !shown.has(k) ? " arriving" : "",
    ].join("");
    el.className = `item${it.active ? " active" : ""}${it.kind === "stack" && it.target === openStack ? " open" : ""}${motion}`;
    el.dataset.i = String(i);
    el.dataset.k = k;
    if (it.icon) {
      const img = document.createElement("img");
      img.src = it.icon;
      img.alt = "";
      // A picture that fails to load falls back to the letter tile.
      img.addEventListener("error", () => img.replaceWith(letterTile(it.name)));
      el.appendChild(img);
    } else if (it.no_icon || (waitingSince.get(k) ?? now) < now - ICON_WAIT) {
      // Windows had no picture for it (or never answered): a letter tile.
      el.appendChild(letterTile(it.name));
    } else {
      // The real icon is still being drawn by Windows: hold its place quietly.
      if (!waitingSince.has(k)) {
        waitingSince.set(k, now);
        setTimeout(() => { sig = ""; refresh(); }, ICON_WAIT + 100);
      }
      el.appendChild(Object.assign(document.createElement("div"), { className: "letter pending" }));
    }
    if (it.icon || it.no_icon) waitingSince.delete(k);
    if (it.windows.length) el.appendChild(Object.assign(document.createElement("span"), { className: "dot" }));
    el.appendChild(Object.assign(document.createElement("span"), { className: "label", textContent: it.name }));
    el.addEventListener("click", (e) => open(it, el, e.shiftKey));
    el.addEventListener("contextmenu", (e) => { e.preventDefault(); showMenu(it, e.clientX, e.clientY); });
    el.addEventListener("pointerdown", (e) => { if (e.button === 0) el.classList.add("pressed"); });
    el.addEventListener("pointerup", () => el.classList.remove("pressed"));
    el.addEventListener("pointerleave", () => el.classList.remove("pressed"));
    el.addEventListener("animationend", (e) => { if (e.animationName === "arrive") el.classList.remove("arriving"); });
    dock.appendChild(el);
  });
  // Icons that left the dock shrink and close the gap instead of vanishing.
  const keep = new Set(items.map(keyOf));
  if (!calm.matches) {
    for (const [k, { el, idx }] of before) {
      if (keep.has(k)) continue;
      const ghost = el.cloneNode(true) as HTMLElement;
      ghost.className = "item leaving";
      ghost.style.removeProperty("--w");
      const at = dock.querySelectorAll(".item")[idx] ?? null;
      dock.insertBefore(ghost, at);
      ghost.addEventListener("animationend", () => { ghost.remove(); place(); });
      setTimeout(() => ghost.remove(), 600);
    }
  }
  shown = keep;
  firstRender = false;
  place();
}

// Gentle magnification: icons swell by distance from the pointer. Each icon
// grows its own slot (width), so neighbours move apart instead of being
// covered; the swell rises above the dock. Distances are measured on the
// resting layout, so growing icons never feed back into the pointer maths.
function magnify(clientX: number | null): void {
  const kids = Array.from(dock.children) as HTMLElement[];
  const slots: { el: HTMLElement; c: number }[] = [];
  let x = PAD_X + BORDER;
  for (const el of kids) {
    if (el.classList.contains("leaving")) continue;
    const w = el.classList.contains("sep") ? SEP_W : SIZE;
    if (el.classList.contains("item")) slots.push({ el, c: x + w / 2 });
    x += w + GAP;
  }
  const rest = x - GAP + PAD_X + BORDER;
  const r = dock.getBoundingClientRect();
  // Pointer in resting coordinates: same share of the way along the dock.
  const px = clientX === null ? null : ((clientX - r.left) / Math.max(1, r.width)) * rest;
  for (const { el, c } of slots) {
    const d = px === null ? Infinity : Math.abs(px - c);
    const s = 1 + (MAX - 1) * Math.max(0, Math.cos(Math.min(d / RANGE, 1) * (Math.PI / 2)));
    if (s <= 1.001) {
      el.style.removeProperty("--w");
    } else {
      el.style.setProperty("--w", `${(SIZE * s).toFixed(1)}px`);
    }
  }
}
dock.addEventListener("mousemove", (e) => magnify(calm.matches ? null : e.clientX));
dock.addEventListener("mouseleave", () => magnify(null));

async function open(it: DockItem, el: HTMLElement, fresh: boolean): Promise<void> {
  hideMenu();
  if (it.kind === "stack") { toggleStack(it, el); return; }
  closeStack();
  try {
    if (fresh && it.kind === "pin") await invoke("dock_launch", { target: it.target });
    else await invoke("dock_open", { kind: it.kind, target: it.target, windows: it.windows, active: it.active });
    if (!it.windows.length && it.kind === "pin") {
      launching.set(keyOf(it), Date.now() + 6000);
      el.classList.add("launching");
      setTimeout(() => { launching.delete(keyOf(it)); el.classList.remove("launching"); }, 6000);
    }
  } catch (e) { console.error(e); }
  setTimeout(refresh, 400);
}

function showMenu(it: DockItem, x: number, y: number): void {
  const acts: [string, () => Promise<unknown>][] = [];
  if (it.kind === "pin") {
    acts.push(["Open new window", () => invoke("dock_launch", { target: it.target })]);
    acts.push(["Remove from Dock", () => invoke("dock_unpin", { target: it.target })]);
  }
  if (it.kind === "stack") {
    acts.push([`Open ${it.name}`, () => invoke("stack_folder", { key: it.target })]);
    acts.push(["Remove from Dock", async () => {
      const st = await invoke<{ stacks: string[] }>("dock_status");
      await invoke("dock_stacks_set", { keys: (st?.stacks ?? []).filter((k) => k !== it.target) });
    }]);
  }
  if (it.kind === "trash") acts.push(["Open Recycle Bin", () => invoke("dock_open", { kind: "trash", target: it.target, windows: [], active: false })]);
  if (it.kind === "cove") acts.push(["Quit Cove", () => invoke("topbar_menu", { what: "quit" })]);
  if (it.windows.length && it.kind !== "cove") acts.push(["Show All Windows", () => invoke("expose_open", { name: it.name, windows: it.windows })]);
  if (it.windows.length) acts.push([it.windows.length > 1 ? `Close ${it.windows.length} windows` : "Close window", () => invoke("dock_close", { windows: it.windows })]);
  if (it.kind === "running") acts.push(["Keep in Dock", () => invoke("dock_pin", { name: it.name, target: it.target })]);
  menu.innerHTML = "";
  for (const [label, run] of acts) {
    const b = document.createElement("button");
    b.textContent = label;
    b.addEventListener("click", async () => { hideMenu(); await run().catch(() => {}); refresh(); });
    menu.appendChild(b);
  }
  menu.classList.remove("hidden");
  busy();
  const r = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(4, Math.min(x - r.width / 2, innerWidth - r.width - 4))}px`;
  menu.style.top = `${Math.max(4, y - r.height - 12)}px`;
}
function hideMenu(): void { menu.classList.add("hidden"); busy(); }
// A menu or stack reaches above the dock's reserved strip: keep the whole
// window clickable while one is open (otherwise its headroom lets clicks
// through to the app below).
let wasBusy = false;
function busy(): void {
  const now = !menu.classList.contains("hidden") || !!openStack;
  if (now !== wasBusy) { wasBusy = now; invoke("dock_busy", { busy: now }); }
}
document.addEventListener("click", (e) => {
  const t = e.target as Node;
  if (!menu.contains(t)) hideMenu();
  if (openStack && !stack.contains(t) && !(t as HTMLElement).closest?.(".item.open")) closeStack();
});
window.addEventListener("blur", () => { hideMenu(); closeStack(); });
document.addEventListener("keydown", (e) => { if (e.key === "Escape") closeStack(); });

// ---- Folder stacks: a grid of the newest files, Mac-style ----
interface StackFile { name: string; path: string; dir: boolean; icon: string | null }

function closeStack(): void {
  if (!openStack) return;
  openStack = "";
  stack.classList.add("hidden");
  busy();
  dock.querySelectorAll(".item.open").forEach((e) => e.classList.remove("open"));
  place();
}

async function toggleStack(it: DockItem, el: HTMLElement): Promise<void> {
  if (openStack === it.target) { closeStack(); return; }
  openStack = it.target;
  busy();
  if (inTauri) getCurrentWindow().setFocus().catch(() => {}); // so clicking elsewhere closes it
  dock.querySelectorAll(".item.open").forEach((e) => e.classList.remove("open"));
  el.classList.add("open");
  place();
  const files = (await invoke<StackFile[]>("stack_list", { key: it.target })) ?? [];
  if (openStack !== it.target) return;
  stack.innerHTML = "";
  const head = document.createElement("div");
  head.className = "stack-head";
  head.innerHTML = `<b></b><button>Open in File Explorer ›</button>`;
  head.querySelector("b")!.textContent = it.name;
  head.querySelector("button")!.addEventListener("click", () => { closeStack(); invoke("stack_folder", { key: it.target }); });
  stack.appendChild(head);
  const grid = document.createElement("div");
  grid.className = "stack-grid";
  if (!files.length) grid.appendChild(Object.assign(document.createElement("p"), { className: "stack-empty", textContent: "This folder is empty" }));
  for (const f of files) {
    const b = document.createElement("button");
    b.className = "stack-file";
    b.title = `${f.name}\nClick to open · Ctrl+click to show in folder`;
    if (f.icon) b.appendChild(Object.assign(document.createElement("img"), { src: f.icon, alt: "" }));
    else b.appendChild(Object.assign(document.createElement("div"), { className: "letter", textContent: f.dir ? "▣" : (f.name.split(".").pop() || "?").slice(0, 4).toUpperCase() }));
    b.appendChild(Object.assign(document.createElement("span"), { textContent: f.name }));
    b.addEventListener("click", (e) => { closeStack(); invoke("stack_open", { path: f.path, reveal: e.ctrlKey }); });
    grid.appendChild(b);
  }
  stack.appendChild(grid);
  stack.classList.remove("hidden");
  // Point the panel at its dock icon.
  const r = el.getBoundingClientRect();
  const w = stack.getBoundingClientRect().width;
  stack.style.left = `${Math.max(6, Math.min(r.left + r.width / 2 - w / 2, innerWidth - w - 6))}px`;
}

async function refresh(): Promise<void> {
  const s = await invoke<{ items: DockItem[] }>("dock_state");
  if (!s) return;
  const next = JSON.stringify(s.items.map((i) => [i.kind, i.target, i.name, i.windows, i.active, !!i.attention, i.icon ?? "", !!i.no_icon]));
  if (next === sig || openStack) return; // don't reshuffle under an open stack
  sig = next;
  items = s.items;
  render();
}

// One short hop: Cove was opened again while it was already running.
async function bounce(k: string): Promise<void> {
  await refresh();
  const el = dock.querySelector<HTMLElement>(`.item[data-k="${k}"]`);
  if (!el || calm.matches) return;
  el.classList.remove("nudge");
  void el.offsetWidth; // restart the animation
  el.classList.add("nudge");
  el.addEventListener("animationend", () => el.classList.remove("nudge"), { once: true });
}

if (inTauri) {
  listen("dock-changed", refresh);
  listen<string>("dock-bounce", (e) => bounce(`${e.payload}:${e.payload}`));
  // Moved to another display, or the screen changed size or scale.
  listen("dock-reflow", () => { lastWidth = 0; lastHeight = 0; place(); });
}
refresh();
setInterval(refresh, 1200);
