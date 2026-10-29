import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface DockItem { kind: "pin" | "running" | "trash"; name: string; target: string; windows: number[]; active: boolean; icon?: string }

const inTauri = "__TAURI_INTERNALS__" in window;
const MOCK: DockItem[] = [
  { kind: "pin", name: "File Explorer", target: "explorer", windows: [1], active: false },
  { kind: "pin", name: "Microsoft Edge", target: "edge", windows: [2], active: true },
  { kind: "pin", name: "Settings", target: "settings", windows: [], active: false },
  { kind: "running", name: "Code", target: "code.exe", windows: [3], active: false },
  { kind: "trash", name: "Recycle Bin", target: "trash", windows: [], active: false },
];
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  return Promise.resolve((cmd === "dock_state" ? { items: MOCK } : null) as T);
}

const SIZE = 48, GAP = 6, MAX = 1.55, RANGE = 150, HEAD = 46; // magnification
const dock = document.getElementById("dock")!;
const menu = document.getElementById("menu")!;
let items: DockItem[] = [];
let sig = "";
let lastWidth = 0;

function place(): void {
  const width = Math.ceil(dock.getBoundingClientRect().width) + 160; // room for magnified neighbours
  const height = SIZE + 7 + 9 + 8 + HEAD + 30;
  if (Math.abs(width - lastWidth) > 2) {
    lastWidth = width;
    invoke("dock_place", { width, height });
  }
}

function render(): void {
  dock.innerHTML = "";
  let sepDone = false;
  items.forEach((it, i) => {
    if ((it.kind === "running" && !sepDone && i > 0 && items[i - 1].kind === "pin") || (it.kind === "trash" && i > 0)) {
      dock.appendChild(Object.assign(document.createElement("div"), { className: "sep" }));
      if (it.kind === "running") sepDone = true;
    }
    const el = document.createElement("div");
    el.className = `item${it.active ? " active" : ""}`;
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
document.addEventListener("click", (e) => { if (!menu.contains(e.target as Node)) hideMenu(); });
window.addEventListener("blur", hideMenu);

async function refresh(): Promise<void> {
  const s = await invoke<{ items: DockItem[] }>("dock_state");
  if (!s) return;
  const next = JSON.stringify(s.items.map((i) => [i.kind, i.target, i.windows, i.active, !!i.icon]));
  if (next === sig) return;
  sig = next;
  items = s.items;
  render();
}

if (inTauri) listen("dock-changed", refresh);
refresh();
setInterval(refresh, 1200);
