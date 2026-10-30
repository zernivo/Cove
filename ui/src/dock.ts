import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface DockItem { kind: "pin" | "running" | "stack" | "trash"; name: string; target: string; windows: number[]; active: boolean; icon?: string }

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
const dock = document.getElementById("dock")!;
const menu = document.getElementById("menu")!;
const stack = document.getElementById("stack")!;
const STACK_H = 360; // extra height while a stack is open
let openStack = "";
let items: DockItem[] = [];
let sig = "";
let lastWidth = 0;

let lastHeight = 0;
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
  dock.innerHTML = "";
  let sepDone = false;
  items.forEach((it, i) => {
    const prev = i > 0 ? items[i - 1].kind : "";
    const firstStack = it.kind === "stack" && prev !== "stack" && prev !== "";
    if ((it.kind === "running" && !sepDone && prev === "pin") || firstStack || (it.kind === "trash" && prev !== "stack" && i > 0)) {
      dock.appendChild(Object.assign(document.createElement("div"), { className: "sep" }));
      if (it.kind === "running") sepDone = true;
    }
    const el = document.createElement("div");
    el.className = `item${it.active ? " active" : ""}${it.kind === "stack" && it.target === openStack ? " open" : ""}`;
    el.dataset.i = String(i);
    if (it.icon) {
      const img = document.createElement("img");
      img.src = it.icon;
      img.alt = "";
      el.appendChild(img);
    } else {
      const l = document.createElement("div");
      l.className = "letter";
      l.textContent = (it.name.trim()[0] || "?").toUpperCase();
      el.appendChild(l);
    }
    if (it.windows.length) el.appendChild(Object.assign(document.createElement("span"), { className: "dot" }));
    el.appendChild(Object.assign(document.createElement("span"), { className: "label", textContent: it.name }));
    el.addEventListener("click", (e) => open(it, el, e.shiftKey));
    el.addEventListener("contextmenu", (e) => { e.preventDefault(); showMenu(it, e.clientX, e.clientY); });
    dock.appendChild(el);
  });
  place();
}

// Gentle magnification: icons swell by distance from the pointer.
dock.addEventListener("mousemove", (e) => {
  for (const el of dock.querySelectorAll<HTMLElement>(".item")) {
    const r = el.getBoundingClientRect();
    const d = Math.abs(e.clientX - (r.left + r.width / 2));
    const s = 1 + (MAX - 1) * Math.max(0, Math.cos(Math.min(d / RANGE, 1) * (Math.PI / 2)));
    el.style.transform = `scale(${s.toFixed(3)})`;
    el.style.margin = `0 ${((s - 1) * SIZE) / 2}px`;
  }
});
dock.addEventListener("mouseleave", () => {
  for (const el of dock.querySelectorAll<HTMLElement>(".item")) { el.style.transform = ""; el.style.margin = ""; }
});

async function open(it: DockItem, el: HTMLElement, fresh: boolean): Promise<void> {
  hideMenu();
  if (it.kind === "stack") { toggleStack(it, el); return; }
  closeStack();
  try {
    if (fresh && it.kind === "pin") await invoke("dock_launch", { target: it.target });
    else await invoke("dock_open", { kind: it.kind, target: it.target, windows: it.windows, active: it.active });
    if (!it.windows.length && it.kind === "pin") { el.classList.add("bounce"); setTimeout(() => el.classList.remove("bounce"), 1300); }
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
  const r = menu.getBoundingClientRect();
  menu.style.left = `${Math.max(4, Math.min(x - r.width / 2, innerWidth - r.width - 4))}px`;
  menu.style.top = `${Math.max(4, y - r.height - 12)}px`;
}
function hideMenu(): void { menu.classList.add("hidden"); }
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
  dock.querySelectorAll(".item.open").forEach((e) => e.classList.remove("open"));
  place();
}

async function toggleStack(it: DockItem, el: HTMLElement): Promise<void> {
  if (openStack === it.target) { closeStack(); return; }
  openStack = it.target;
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
  const next = JSON.stringify(s.items.map((i) => [i.kind, i.target, i.windows, i.active, !!i.icon]));
  if (next === sig || openStack) return; // don't reshuffle under an open stack
  sig = next;
  items = s.items;
  render();
}

if (inTauri) listen("dock-changed", refresh);
refresh();
setInterval(refresh, 1200);
