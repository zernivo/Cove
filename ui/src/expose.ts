import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Tile { hwnd: number; title: string; x: number; y: number; w: number; h: number }
interface Min { hwnd: number; title: string }
interface ExposeState { name: string; tiles: Tile[]; minimized: Min[]; scale: number }

const inTauri = "__TAURI_INTERNALS__" in window;
const invoke = (cmd: string, args: Record<string, unknown>) => (inTauri ? tauriInvoke(cmd, args) : Promise.resolve());
const tilesEl = document.getElementById("tiles")!;
const minsEl = document.getElementById("mins")!;
let order: number[] = [];
let els: HTMLElement[] = [];
let sel = -1;

function pick(hwnd: number): void { invoke("expose_pick", { hwnd }); }
function close(): void { invoke("expose_close", { refocus: true }); }

function select(i: number): void {
  els.forEach((e, j) => e.classList.toggle("sel", j === i));
  sel = i;
}

function render(s: ExposeState): void {
  document.getElementById("title")!.textContent = s.name;
  tilesEl.innerHTML = "";
  minsEl.innerHTML = "";
  order = [];
  els = [];
  const k = s.scale || 1;
  for (const t of s.tiles) {
    const el = document.createElement("div");
    el.className = "tile";
    Object.assign(el.style, { left: `${t.x / k}px`, top: `${t.y / k}px`, width: `${t.w / k}px`, height: `${t.h / k}px` });
    el.appendChild(Object.assign(document.createElement("div"), { className: "cap", textContent: t.title || s.name }));
    el.addEventListener("click", (e) => { e.stopPropagation(); pick(t.hwnd); });
    tilesEl.appendChild(el);
    order.push(t.hwnd);
    els.push(el);
  }
  for (const m of s.minimized) {
    const el = Object.assign(document.createElement("div"), { className: "min", textContent: m.title || s.name });
    el.addEventListener("click", (e) => { e.stopPropagation(); pick(m.hwnd); });
    minsEl.appendChild(el);
    order.push(m.hwnd);
    els.push(el);
  }
  if (!order.length) tilesEl.appendChild(Object.assign(document.createElement("div"), { className: "empty", textContent: "No open windows" }));
  select(order.length ? 0 : -1);
}

document.addEventListener("click", close);
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") close();
  else if (e.key === "Enter" && sel >= 0) pick(order[sel]);
  else if (["ArrowRight", "ArrowDown", "Tab"].includes(e.key) && order.length) { e.preventDefault(); select((sel + 1) % order.length); }
  else if (["ArrowLeft", "ArrowUp"].includes(e.key) && order.length) { e.preventDefault(); select((sel - 1 + order.length) % order.length); }
});

if (inTauri) listen<ExposeState>("expose-state", (e) => render(e.payload));
else render({
  name: "Microsoft Edge", scale: 1,
  tiles: [
    { hwnd: 1, title: "Cove – GitHub", x: 80, y: 120, w: 520, h: 325 },
    { hwnd: 2, title: "Docs", x: 680, y: 120, w: 520, h: 325 },
  ],
  minimized: [{ hwnd: 3, title: "Inbox" }],
});
