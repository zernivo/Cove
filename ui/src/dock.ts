import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { makePlate, refractionFilter, wallpaperUrl, reducedMotion } from "./glass";
import { MOCK_STATUS, type TrayStatus, type TrayItem } from "./tray";

interface DockItem { kind: "pin" | "running" | "cove" | "stack" | "trash"; name: string; target: string; windows: number[]; active: boolean; attention?: boolean; icon?: string; no_icon?: boolean; full?: boolean }
interface MItem { id?: string; label?: string; kbd?: string; check?: boolean; disabled?: boolean; sub?: MItem[]; sep?: boolean; header?: string; count?: string }

const inTauri = "__TAURI_INTERNALS__" in window;
const q = new URLSearchParams(location.search);
const DEMO: DockItem[] = [
  ["File Explorer", "explorer", 1], ["Google Chrome", "chrome", 1], ["Visual Studio Code", "vscode", 1], ["Spotify", "spotify", 1], ["Discord", "discord", 0],
  ["WhatsApp", "whatsapp", 0], ["Notion", "notion", 0], ["Figma", "figma", 0], ["Cove", "cove", 0],
].map(([name, id, run]) => ({ kind: id === "cove" ? "cove" : "pin", name: name as string, target: id as string, windows: run ? [1] : [], active: false, icon: `/mock/${id}.png` }) as DockItem)
  .concat([{ kind: "stack", name: "Downloads", target: "downloads", windows: [], active: false, icon: "/mock/downloads.png" }, { kind: "trash", name: "Recycle Bin", target: "trash", windows: [], active: false, icon: "/mock/bin.png", full: false }]);
const MOCK: DockItem[] = q.get("demo") ? DEMO : [
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
  if (cmd === "popover_at") (window.parent as Window)?.postMessage({ coveDockMenu: args }, "*");
  if (cmd === "popover_dock") (window.parent as Window)?.postMessage({ coveDockPop: args }, "*");
  if (cmd === "tray_status") return Promise.resolve((q.get("tray") === "off" ? { ...MOCK_STATUS, mode: "off", items: [] } : MOCK_STATUS) as T);
  if (cmd === "island_state") {
    const b = q.get("batt");
    return Promise.resolve({ battery: b ? { percent: Number(b) || 84, charging: false } : null, volume: 46, muted: false, wifi: q.get("wired") ? null : "Nazeer-5G" } as T);
  }
  return Promise.resolve((cmd === "dock_state" ? { items: MOCK } : null) as T);
}

// ---- Geometry (dock.css): variant A proportions, smaller (2026-11-05) ----
const SIZE = 40, GAP = 8, PAD_X = 8, SEP_W = 1 - 2; // .sep: 1 px wide, -1 px margins each side
const DOCK_H = 56, FLOAT = 6, RADIUS = 15;
const HEADROOM = 48; // magnified icons (+10 + 6 DIP) and the tooltip above them
const SIDE = 36;     // spread (+~10 DIP each side) and shadow reach
const STACK_H = 360; // extra height while a folder stack is open
// ---- Magnification (user pick): smooth and gentle. At most 1.25x, a lift
// of 6 DIP, neighbours with a cosine-squared falloff over two icons, and a
// critically damped spring (damping ratio 1.0) settling in ~250 ms. Every
// icon's scale follows its own spring, so motion is interruptible and never
// jumps; distances are measured on the resting layout, so growing icons
// never feed back into the pointer maths (no jitter).
const MAX = 1.25, LIFT = 6, RANGE = (SIZE + GAP) * 2, OMEGA = 24;

const ICON_WAIT = 5000; // after this, a still-missing icon shows its letter tile
const waitingSince = new Map<string, number>();
function letterTile(name: string): HTMLElement {
  const t = document.createElement("div");
  t.className = "letter tile";
  t.textContent = (name.trim()[0] ?? "?").toUpperCase();
  return t;
}
const dock = document.getElementById("dock")!;
const glass = document.getElementById("glass")!;
const plateEl = document.getElementById("plate")!;
const stack = document.getElementById("stack")!;
const island = document.getElementById("island")!;
const islandSep = Object.assign(document.createElement("div"), { className: "sep isep" });
let openStack = "";
let items: DockItem[] = [];
let sig = "";
let lastWidth = 0;
let lastHeight = 0;
const launching = new Map<string, number>();
let shown = new Set<string>();
let firstRender = true;
const keyOf = (it: DockItem) => `${it.kind}:${it.target}`;

function place(): void {
  // The clock drops its date line when the dock would not fit its display.
  const compact = document.body.classList.contains("compact");
  const wide = Math.ceil(dock.getBoundingClientRect().width) + SIDE * 2;
  if (!compact && wide > geom.mw - 16) { document.body.classList.add("compact"); measure(); }
  else if (compact && wide + 40 < geom.mw - 16) { document.body.classList.remove("compact"); measure(); }
  const pill = Math.ceil(dock.getBoundingClientRect().width);
  let width = pill + SIDE * 2;
  let height = FLOAT + DOCK_H + HEADROOM;
  if (openStack) { width = Math.max(width, 460); height += STACK_H; }
  if (Math.abs(width - lastWidth) > 2 || height !== lastHeight) {
    lastWidth = width;
    lastHeight = height;
    invoke("dock_place", { width, height, pillWidth: pill + 8 });
  }
  layoutGlass();
}

// ---- Glass: plate aligned to the screen, refraction on the edge band ----
let geom = { x: 0, y: 0, mw: screen.width, mh: screen.height };
let plateUrl = "";
function layoutGlass(): void {
  const r = dock.getBoundingClientRect();
  if (!r.width) return;
  // Where the slab sits on its monitor (CSS px), ignoring the slide transform.
  const left = geom.x + dock.offsetLeft;
  const top = geom.y + (innerHeight - FLOAT - DOCK_H);
  const root = document.documentElement.style;
  root.setProperty("--mon-w", `${geom.mw}px`);
  root.setProperty("--mon-h", `${geom.mh}px`);
  glass.style.setProperty("--px", `${-left}px`);
  glass.style.setProperty("--py", `${-top}px`);
  if (plateUrl) plateEl.style.filter = `url(#${refractionFilter(r.width, DOCK_H, RADIUS, { bezel: 13, thickness: 28, strength: 0.7 })})`;
}
async function loadPlate(): Promise<void> {
  try {
    const p = await makePlate(wallpaperUrl(), geom.mw, geom.mh, 24, 1.7);
    plateUrl = p.url;
    plateEl.style.backgroundImage = `url("${p.url}")`;
    plateEl.classList.add("ready");
    glass.classList.add("has-plate");
    layoutGlass();
  } catch {
    // No readable picture (solid colour desktop): a solid clear-dark slab.
    plateUrl = "";
    plateEl.classList.remove("ready");
    glass.classList.remove("has-plate");
  }
}

// ---- Render ----
function render(): void {
  const before = new Map<string, { el: HTMLElement; idx: number }>();
  dock.querySelectorAll<HTMLElement>(".item:not(.leaving)").forEach((el, idx) => { if (el.dataset.k) before.set(el.dataset.k, { el, idx }); });
  const now = Date.now();
  for (const [k, until] of launching) if (until < now) launching.delete(k);
  dock.querySelectorAll(".item, .sep").forEach((e) => e.remove());
  island.remove();
  let sepDone = false;
  items.forEach((it, i) => {
    // One divider: between the apps and Downloads + Bin.
    if (!sepDone && (it.kind === "stack" || it.kind === "trash") && i > 0) {
      dock.appendChild(Object.assign(document.createElement("div"), { className: "sep" }));
      sepDone = true;
    }
    const el = document.createElement("div");
    const k = keyOf(it);
    if (it.windows.length) launching.delete(k);
    const motion = [launching.has(k) ? " launching" : "", it.attention ? " attention" : "", !firstRender && !shown.has(k) ? " arriving" : ""].join("");
    el.className = `item${it.active ? " active" : ""}${it.kind === "stack" && it.target === openStack ? " open" : ""}${motion}`;
    el.dataset.i = String(i);
    el.dataset.k = k;
    const pic = document.createElement("div");
    pic.className = "pic";
    if (it.icon) {
      const img = document.createElement("img");
      img.src = it.icon;
      img.alt = "";
      img.draggable = false;
      img.addEventListener("error", () => img.replaceWith(letterTile(it.name)));
      pic.appendChild(img);
    } else if (it.no_icon || (waitingSince.get(k) ?? now) < now - ICON_WAIT) {
      pic.appendChild(letterTile(it.name));
    } else {
      if (!waitingSince.has(k)) {
        waitingSince.set(k, now);
        setTimeout(() => { sig = ""; refresh(); }, ICON_WAIT + 100);
      }
      pic.appendChild(Object.assign(document.createElement("div"), { className: "letter pending" }));
    }
    if (it.icon || it.no_icon) waitingSince.delete(k);
    el.appendChild(pic);
    if (it.windows.length) el.appendChild(Object.assign(document.createElement("span"), { className: "dot" }));
    el.appendChild(Object.assign(document.createElement("span"), { className: "label", textContent: it.name }));
    el.addEventListener("click", (e) => open(it, el, e.shiftKey));
    if (it.kind === "cove") el.addEventListener("pointerdown", (e) => { if (e.button === 0) popDock("cove", el); });
    el.addEventListener("contextmenu", (e) => { e.preventDefault(); showMenu(it, el); });
    el.addEventListener("pointerdown", (e) => { if (e.button === 0) el.classList.add("pressed"); });
    el.addEventListener("pointerup", () => el.classList.remove("pressed"));
    el.addEventListener("pointerenter", () => tipEnter(el));
    el.addEventListener("pointerleave", () => { el.classList.remove("pressed"); tipLeave(el); });
    el.addEventListener("animationend", (e) => { if (e.animationName === "arrive") { el.classList.remove("arriving"); measure(); } });
    dock.appendChild(el);
  });
  // The status island closes the dock, after its own divider.
  dock.appendChild(islandSep);
  dock.appendChild(island);
  const keep = new Set(items.map(keyOf));
  if (!reducedMotion.matches) {
    for (const [k, { el, idx }] of before) {
      if (keep.has(k)) continue;
      const ghost = el.cloneNode(true) as HTMLElement;
      ghost.className = "item leaving";
      const at = dock.querySelectorAll(".item")[idx] ?? islandSep;
      dock.insertBefore(ghost, at);
      ghost.addEventListener("animationend", () => { ghost.remove(); measure(); place(); });
      setTimeout(() => ghost.remove(), 600);
    }
  }
  shown = keep;
  firstRender = false;
  measure();
  place();
}

// ---- Magnification springs ----
interface Slot { el: HTMLElement; pic: HTMLElement | null; c: number; s: number; v: number; t: number; item: boolean }
let slots: Slot[] = [];
let restW = 0;
let pointerX: number | null = null;
let raf = 0;
let lastT = 0;

/** Resting centres, from layout (transforms don't move offsetLeft). */
function measure(): void {
  const old = new Map(slots.map((s) => [s.el, s]));
  slots = [];
  for (const el of Array.from(dock.children) as HTMLElement[]) {
    const isItem = el.classList.contains("item");
    if (!isItem && !el.classList.contains("sep") && el !== island) continue;
    if (el.classList.contains("leaving")) continue;
    const prev = old.get(el);
    slots.push({ el, pic: el.querySelector<HTMLElement>(".pic"), c: el.offsetLeft + el.offsetWidth / 2, s: prev?.s ?? 1, v: prev?.v ?? 0, t: 1, item: isItem });
  }
  restW = dock.offsetWidth;
  void SEP_W; void PAD_X;
}

function targets(): void {
  for (const sl of slots) {
    if (!sl.item || pointerX === null || reducedMotion.matches) { sl.t = 1; continue; }
    const u = Math.abs(pointerX - sl.c) / RANGE;
    const f = u < 1 ? Math.cos((u * Math.PI) / 2) ** 2 : 0;
    sl.t = 1 + (MAX - 1) * f;
  }
}

function apply(): void {
  // Each icon's extra width pushes the ones after it outward; the slab
  // grows by the total, symmetrically, so the dock stays centred.
  let total = 0;
  for (const sl of slots) total += sl.item ? SIZE * (sl.s - 1) : 0;
  let run = 0;
  for (const sl of slots) {
    const e = sl.item ? SIZE * (sl.s - 1) : 0;
    const shift = run + e / 2 - total / 2;
    run += e;
    sl.el.style.transform = Math.abs(shift) < 0.01 ? "" : `translateX(${shift.toFixed(2)}px)`;
    if (sl.pic) {
      const k = (sl.s - 1) / (MAX - 1);
      const lift = LIFT * Math.max(0, k);
      sl.pic.style.transform = sl.s <= 1.0005 ? "" : `translateY(${(-lift).toFixed(2)}px) scale(${sl.s.toFixed(4)})`;
      sl.el.style.setProperty("--lift", `${(SIZE * (sl.s - 1) + lift).toFixed(1)}px`);
    }
  }
  glass.style.transform = total < 0.05 ? "" : `scaleX(${((restW + total) / Math.max(1, restW)).toFixed(5)})`;
}

function step(now: number): void {
  const dt = Math.min(0.034, (now - (lastT || now)) / 1000 || 0.016);
  lastT = now;
  targets();
  let moving = false;
  for (const sl of slots) {
    // Critically damped spring: x'' = w^2 (t - x) - 2 w x'
    const a = OMEGA * OMEGA * (sl.t - sl.s) - 2 * OMEGA * sl.v;
    sl.v += a * dt;
    sl.s += sl.v * dt;
    if (Math.abs(sl.t - sl.s) < 0.0004 && Math.abs(sl.v) < 0.002) { sl.s = sl.t; sl.v = 0; } else moving = true;
  }
  apply();
  raf = moving ? requestAnimationFrame(step) : 0;
  if (!moving) lastT = 0;
}
function kick(): void { if (!raf) { lastT = 0; raf = requestAnimationFrame(step); } }

dock.addEventListener("pointermove", (e) => {
  if (reducedMotion.matches) return;
  pointerX = e.clientX - dock.getBoundingClientRect().left; // slab is never transformed sideways
  kick();
});
dock.addEventListener("pointerleave", () => { pointerX = null; kick(); });

// Browser preview only: a deterministic driver for the verification
// harness (runs the same spring for `ms` of simulated time, 16 ms steps).
if (!inTauri) {
  (window as unknown as { coveDockSim?: (x: number | null, ms: number) => void }).coveDockSim = (x, ms) => {
    cancelAnimationFrame(raf); raf = 0;
    pointerX = x;
    for (let t = 0; t < ms; t += 16) { lastT = 1000 + t; step(1016 + t); cancelAnimationFrame(raf); raf = 0; }
  };
}

// ---- Tooltip: after ~300 ms, soft fade; instant when moving between icons
// while one is showing (as on a Mac). ----
let tipTimer = 0;
let tipShownAt = 0;
function tipEnter(el: HTMLElement): void {
  clearTimeout(tipTimer);
  const warm = Date.now() - tipShownAt < 250 || !!dock.querySelector(".tip");
  dock.querySelectorAll(".tip").forEach((e) => { if (e !== el) e.classList.remove("tip"); });
  tipTimer = window.setTimeout(() => { el.classList.add("tip"); tipShownAt = Date.now(); }, warm ? 0 : 300);
}
function tipLeave(el: HTMLElement): void {
  clearTimeout(tipTimer);
  if (el.classList.contains("tip")) tipShownAt = Date.now();
  el.classList.remove("tip");
}

async function open(it: DockItem, el: HTMLElement, fresh: boolean): Promise<void> {
  if (it.kind === "stack") { toggleStack(it, el); return; }
  closeStack();
  // The Cove icon is the Cove menu (About This PC, Settings, Sleep, ...).
  if (it.kind === "cove") return;
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

// ---- Context menu: the shared glass menu in the popover window, centred
// on the icon, 10 DIP above the dock. Picks come back as dock-menu-pick. ----
const actions = new Map<string, () => Promise<unknown>>();
let menuOpen = ""; // the popover open now ("dock", "cove", "cc", "cal"), for the island's open pill
function showMenu(it: DockItem, el: HTMLElement): void {
  actions.clear();
  const list: MItem[] = [{ header: it.name }];
  const act = (id: string, label: string, run: () => Promise<unknown>, kbd?: string, extra: Partial<MItem> = {}): MItem => { actions.set(id, run); return { id, label, kbd, ...extra }; };
  const has = it.windows.length > 0;
  if (it.kind === "pin" || it.kind === "running") {
    // The app menu the menu bar used to hold lives here now, per app.
    const app = (what: string) => () => invoke("topbar_app", { what, hwnd: it.windows[0], name: it.name });
    list.push(act("new", "New Window", () => invoke("dock_launch", { target: it.target })));
    list.push({ sep: true });
    if (has) {
      list.push(act("expose", "Show All Windows", () => invoke("expose_open", { name: it.name, windows: it.windows }), undefined, it.windows.length > 1 ? { count: String(it.windows.length) } : {}));
      list.push({ sep: true });
      list.push(act("hide", "Hide", app("hide"), "Alt+H"));
      list.push(act("hide-others", "Hide Others", app("hide-others")));
      list.push({ sep: true });
      list.push(act("minimize", "Minimise", app("minimize"), "Alt+M"));
      list.push(act("zoom", "Zoom", app("zoom")));
      list.push(act("front", "Bring All to Front", app("front")));
    }
    const opts: MItem[] = [];
    opts.push(it.kind === "pin"
      ? act("unpin", "Keep in Dock", () => invoke("dock_unpin", { target: it.target }), undefined, { check: true })
      : act("pin", "Keep in Dock", () => invoke("dock_pin", { name: it.name, target: it.target }), undefined, { check: false }));
    if (/[\\/]/.test(it.target)) opts.push(act("reveal", "Show in File Explorer", () => invoke("search_reveal", { target: it.target })));
    if (list[list.length - 1]?.sep !== true) list.push({ sep: true });
    list.push({ label: "Options", sub: opts });
    if (has) { list.push({ sep: true }); list.push(act("quit", it.windows.length > 1 ? `Quit (${it.windows.length} windows)` : "Quit", () => invoke("dock_close", { windows: it.windows }), "Alt+Q")); }
  } else if (it.kind === "cove") {
    list.push(act("settings", "Cove Settings…", () => invoke("topbar_menu", { what: "settings" })));
    list.push({ sep: true });
    list.push(act("quit", "Quit Cove", () => invoke("topbar_menu", { what: "quit" })));
  } else if (it.kind === "stack") {
    list.push(act("open", `Open ${it.name}`, () => invoke("stack_folder", { key: it.target })));
    list.push({ sep: true });
    list.push(act("remove", "Remove from Dock", async () => {
      const st = await invoke<{ stacks: string[] }>("dock_status");
      await invoke("dock_stacks_set", { keys: (st?.stacks ?? []).filter((k) => k !== it.target) });
    }));
  } else if (it.kind === "trash") {
    list.push(act("open", "Open", () => invoke("dock_open", { kind: "trash", target: it.target, windows: [], active: false })));
    list.push({ sep: true });
    list.push(act("empty", "Empty Recycle Bin…", () => invoke("bin_empty"), undefined, { disabled: it.full === false }));
  }
  const r = el.getBoundingClientRect();
  const top = innerHeight - FLOAT - DOCK_H;
  menuOpen = "dock";
  invoke("popover_at", { kind: "dock", x: r.left + r.width / 2, y: top, data: { items: list } }).catch(() => {});
}

// ---- The dock's own popovers: the Cove menu (left-click the Cove icon),
// Control Centre (Wi-Fi, sound, battery, Control Centre) and the calendar
// (the clock). They stand 8 DIP above the dock, centred on the item, and a
// second click on the same item closes them. ----
let popFrom: HTMLElement | null = null;
function popDock(kind: "cove" | "cc" | "cal", el: HTMLElement, data: unknown = null): void {
  const r = el.getBoundingClientRect();
  const top = innerHeight - FLOAT - DOCK_H;
  popFrom = el;
  menuOpen = kind;
  invoke("popover_dock", { kind, x: r.left + r.width / 2, y: top, data }).catch(() => {});
  if (!inTauri) showOpen(kind);
}
function showOpen(open: string | null): void {
  island.querySelectorAll<HTMLElement>(".st").forEach((b) => b.classList.toggle("open", !!open && b === popFrom));
}
if (inTauri) {
  listen<string>("dock-menu-pick", async (e) => {
    const run = actions.get(e.payload);
    if (run) await run().catch(() => {});
    refresh();
  });
  // The app keeps the dock up while one of its popovers is open (it knows
  // which popover hangs from the dock); here it only drives the open pill.
  listen<string | null>("popover-state", (e) => {
    menuOpen = e.payload ?? "";
    showOpen(menuOpen && popFrom && (popFrom.dataset.k === "clock" ? menuOpen === "cal" : menuOpen === "cc") ? menuOpen : null);
  });
}

// Hidden test hook (the visual CI run, `Cove.exe --cove-open=<kind>`):
// open one of the dock's popovers as a click would.
if (inTauri) {
  listen<string>("dock-debug-open", (e) => {
    const k = e.payload;
    const cove = dock.querySelector<HTMLElement>('.item[data-k^="cove:"]');
    const first = items.findIndex((i) => i.kind === "pin" || i.kind === "running");
    const firstEl = first >= 0 ? dock.querySelector<HTMLElement>(`.item[data-k="${keyOf(items[first])}"]`) : null;
    if (k === "cc") popDock("cc", st("cc"));
    else if (k === "wifi") popDock("cc", st("wifi"));
    else if (k === "cal") popDock("cal", st("clock"));
    else if (k === "cove" && cove) popDock("cove", cove);
    else if (k === "menu" && firstEl) showMenu(items[first], firstEl);
  });
}

// ---- Status island ----
interface Island { battery: { percent: number; charging: boolean } | null; volume: number | null; muted: boolean; wifi: string | null }
const st = (k: string) => island.querySelector<HTMLElement>(`.st[data-k="${k}"]`)!;
const label = (k: string, text: string) => { st(k).querySelector(".label")!.textContent = text; st(k).setAttribute("aria-label", text); };
island.querySelectorAll<HTMLElement>(".st").forEach((b) => {
  const k = b.dataset.k!;
  // Respond on press, like the menu bar did.
  b.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    closeStack();
    if (k === "search") invoke("topbar_open", { what: "search" }).catch(() => {});
    else if (k === "more") popDock("cc", b, { focus: "tray" });
    else popDock(k === "clock" ? "cal" : "cc", b);
  });
  b.addEventListener("pointerenter", () => tipEnter(b));
  b.addEventListener("pointerleave", () => tipLeave(b));
});
async function refreshIsland(): Promise<void> {
  const s = await invoke<Island>("island_state").catch(() => null);
  if (!s) return;
  const b = s.battery;
  const batt = st("battery");
  const had = !batt.classList.contains("hidden");
  batt.classList.toggle("hidden", !b);
  if (b) {
    const f = document.getElementById("batt-fill")!;
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 16.6))}px`;
    f.className = `fill${b.charging ? " charging" : b.percent <= 20 ? " low" : ""}`;
    label("battery", b.charging ? `Charging, ${b.percent}%` : `Battery ${b.percent}%`);
  }
  const wifi = st("wifi");
  wifi.classList.toggle("wired", !s.wifi);
  label("wifi", s.wifi ? `Wi-Fi: ${s.wifi}` : "Network");
  const snd = st("sound");
  snd.classList.toggle("muted", s.muted || s.volume === 0);
  snd.classList.toggle("low", (s.volume ?? 100) < 34);
  label("sound", s.volume === null ? "Sound" : s.muted ? "Sound (muted)" : `Sound ${s.volume}%`);
  if (had !== !!b) { measure(); place(); }
}
// ---- Pinned tray icons: up to 4 of other apps' tray icons (Discord,
// Steam...) in their own colours, before Wi-Fi. The pins are the ones the
// user chose in Control Centre; with none chosen, the first 4 icons. Clicks
// go to the app exactly as the CC grid sends them (its own menu or window).
const TRAY_MAX = 4;
const tpins = document.getElementById("tpins")!;
let traySig = "";
function trayPicks(s: TrayStatus): TrayItem[] {
  if (s.mode === "off") return [];
  const byKey = new Map(s.items.map((i) => [i.key, i]));
  const chosen = s.pins.map((k) => byKey.get(k)).filter((i): i is TrayItem => !!i);
  return (s.pins.length ? chosen : s.items).slice(0, TRAY_MAX);
}
async function refreshTray(): Promise<void> {
  const s = await invoke<TrayStatus>("tray_status").catch(() => null);
  const picks = s ? trayPicks(s) : [];
  const more = st("more");
  const show = !!s && s.mode !== "off" && s.items.length > 0;
  const nextSig = JSON.stringify([show, picks.map((i) => [i.id, i.icon, i.tip])]);
  if (nextSig === traySig) return;
  traySig = nextSig;
  tpins.innerHTML = "";
  for (const it of picks) {
    const b = document.createElement("button");
    b.className = "st tp";
    b.setAttribute("aria-label", it.tip || it.name);
    if (it.icon) {
      const img = Object.assign(document.createElement("img"), { src: it.icon, alt: "", draggable: false });
      img.onerror = () => img.replaceWith(Object.assign(document.createElement("span"), { className: "tp-letter", textContent: (it.name || "?").slice(0, 1).toUpperCase() }));
      b.appendChild(img);
    } else {
      b.appendChild(Object.assign(document.createElement("span"), { className: "tp-letter", textContent: (it.name || "?").slice(0, 1).toUpperCase() }));
    }
    b.appendChild(Object.assign(document.createElement("span"), { className: "label", textContent: (it.tip || it.name).split("\n")[0] }));
    const send = (how: "left" | "right" | "double") => { closeStack(); tipLeave(b); invoke("tray_click", { id: it.id, how }).catch(() => {}); };
    b.addEventListener("click", (e) => { if (e.detail <= 1) send("left"); });
    b.addEventListener("dblclick", () => send("double"));
    b.addEventListener("contextmenu", (e) => { e.preventDefault(); send("right"); });
    b.addEventListener("pointerenter", () => tipEnter(b));
    b.addEventListener("pointerleave", () => tipLeave(b));
    tpins.appendChild(b);
  }
  tpins.classList.toggle("hidden", picks.length === 0);
  more.classList.toggle("hidden", !show);
  measure();
  place();
}

function clock(): void {
  const d = new Date();
  const fixed = q.get("clock")?.split("|");
  const t = fixed?.[0] ?? d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  const day = fixed?.[1] ?? d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" }).replace(/,/g, "");
  const te = document.getElementById("clock-t")!, de = document.getElementById("clock-d")!;
  if (te.textContent !== t || de.textContent !== day) {
    te.textContent = t;
    de.textContent = day;
    document.getElementById("clock-label")!.textContent = d.toLocaleDateString(undefined, { weekday: "long", day: "numeric", month: "long", year: "numeric" });
    measure();
    place();
  }
}

// A folder stack reaches above the dock's strip: keep the whole window
// clickable while one is open (the app does the same for dock popovers).
let wasBusy = false;
function busy(): void {
  const now = !!openStack;
  if (now !== wasBusy) { wasBusy = now; invoke("dock_busy", { busy: now }); }
}
document.addEventListener("click", (e) => {
  const t = e.target as Node;
  if (openStack && !stack.contains(t) && !(t as HTMLElement).closest?.(".item.open")) closeStack();
});
window.addEventListener("blur", () => closeStack());
document.addEventListener("keydown", (e) => { if (e.key === "Escape") closeStack(); });

// ---- Folder stacks: a grid of the newest files ----
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
  if (inTauri) getCurrentWindow().setFocus().catch(() => {});
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
  const r = el.getBoundingClientRect();
  const w = stack.getBoundingClientRect().width;
  stack.style.left = `${Math.max(6, Math.min(r.left + r.width / 2 - w / 2, innerWidth - w - 6))}px`;
}

async function refresh(): Promise<void> {
  const s = await invoke<{ items: DockItem[] }>("dock_state");
  if (!s) return;
  const next = JSON.stringify(s.items.map((i) => [i.kind, i.target, i.name, i.windows, i.active, !!i.attention, i.icon ?? "", !!i.no_icon, i.full ?? null]));
  if (next === sig || openStack) return;
  sig = next;
  items = s.items;
  render();
}

async function bounce(k: string): Promise<void> {
  await refresh();
  const el = dock.querySelector<HTMLElement>(`.item[data-k="${k}"]`);
  if (!el || reducedMotion.matches) return;
  el.classList.remove("nudge");
  void el.offsetWidth;
  el.classList.add("nudge");
  el.addEventListener("animationend", () => el.classList.remove("nudge"), { once: true });
}

if (inTauri) {
  listen("dock-changed", refresh);
  listen<string>("dock-bounce", (e) => bounce(`${e.payload}:${e.payload}`));
  listen("dock-reflow", () => { lastWidth = 0; lastHeight = 0; place(); });
  // Where the window sits on its display (DIP), for the wallpaper plate.
  listen<{ x: number; y: number; mw: number; mh: number }>("dock-geom", (e) => {
    const g = e.payload;
    const changed = g.mw !== geom.mw || g.mh !== geom.mh;
    geom = g;
    if (changed || !plateUrl) loadPlate(); else layoutGlass();
  });
  listen("wallpaper-changed", loadPlate);
  listen<boolean>("dock-slide", (e) => {
    document.body.classList.toggle("tucked", e.payload);
    if (e.payload) { closeStack(); pointerX = null; kick(); }
  });
} else {
  // Browser preview: the window sits centred on the bottom of the screen.
  const gx = Number(q.get("gx") ?? NaN), gy = Number(q.get("gy") ?? NaN);
  geom = { x: Number.isNaN(gx) ? 0 : gx, y: Number.isNaN(gy) ? 0 : gy, mw: Number(q.get("mw") ?? screen.width), mh: Number(q.get("mh") ?? screen.height) };
}
loadPlate();
new MutationObserver(() => loadPlate()).observe(document.documentElement, { attributes: true, attributeFilter: ["data-mode"] });
addEventListener("resize", () => { measure(); layoutGlass(); });
refresh();
setInterval(refresh, 1200);
clock();
setInterval(clock, 1000);
refreshIsland();
setInterval(refreshIsland, 2000);
refreshTray();
if (inTauri) listen("tray-changed", refreshTray);
setInterval(refreshTray, 5000);
