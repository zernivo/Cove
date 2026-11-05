import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { makePlate, refractionFilter, tintFor, wallpaperUrl } from "./glass";

interface TopState { app: string; kind: "app" | "cove" | "desktop" | "shell"; hwnd: number; battery: { percent: number; charging: boolean } | null; volume: number | null; muted: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
const q = new URLSearchParams(location.search);
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const kind = (q.get("kind") ?? "desktop") as TopState["kind"];
  const demo: Record<string, unknown> = {
    topbar_state: { app: q.get("app") ?? (kind === "desktop" ? "Desktop" : "Google Chrome"), kind, hwnd: 1, battery: q.get("batt") ? { percent: 84, charging: false } : null, volume: 40, muted: false },
  };
  return Promise.resolve((demo[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

// Menus open in the popover window (popover.ts), sized to the menu: the bar
// window never grows, so nothing but the menu appears over the apps.
type Kind = "cove" | "app" | "file" | "view" | "go" | "window" | "cc";
const BAR_MENUS: Kind[] = ["cove", "app", "file", "view", "go", "window"];
const MENU_TITLES: Record<string, string> = { file: "File", view: "View", go: "Go", window: "Window" };
const appBtn = $("app");
let front: TopState | null = null;
let open: string | null = null;

/** Cove-owned menus after the bold app name: the desktop gets File, View,
 *  Go and Window; any other app gets Window (Cove can't host a Win32 app's
 *  own menus, so it doesn't fake them). */
function setMenus(kind: TopState["kind"]): void {
  const want = kind === "desktop" || kind === "shell" ? ["file", "view", "go", "window"] : ["window"];
  const box = $("menus");
  if (box.dataset.k === want.join()) return;
  box.dataset.k = want.join();
  box.innerHTML = "";
  for (const m of want) {
    const b = document.createElement("button");
    b.className = "title";
    b.dataset.menu = m;
    b.textContent = MENU_TITLES[m];
    wire(b);
    box.appendChild(b);
  }
  showOpen();
}

function openPopover(kind: Kind): void {
  if (kind === "app" && !front) return;
  const btn = document.querySelector<HTMLElement>(`[data-menu="${kind}"]`);
  const right = kind === "cc";
  // Menus hang from their title's pill; Control Centre from the bar's right end.
  const x = kind === "cc" ? innerWidth - 8 : (btn?.getBoundingClientRect().left ?? 0);
  invoke("popover_toggle", { kind, x, right, data: front }).catch(() => {});
}

function showOpen(): void {
  document.querySelectorAll<HTMLElement>("[data-menu]").forEach((b) => b.classList.toggle("open", b.dataset.menu === open || (b.dataset.menu === "cove" && open === "confirm")));
}

function wire(b: HTMLElement): void {
  const kind = b.dataset.menu as Kind;
  // Press-to-open, like the Mac menu bar: menus respond on pointer-down.
  b.addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover(kind); });
  // With a bar menu open, sliding to another title switches menus.
  if (BAR_MENUS.includes(kind)) b.addEventListener("pointerenter", () => { if (open && open !== kind && BAR_MENUS.includes(open as Kind)) openPopover(kind); });
}
document.querySelectorAll<HTMLElement>("[data-menu]").forEach(wire);
$("wifi-btn").addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover("cc"); });
$("sound-btn").addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover("cc"); });

if (inTauri) listen<string | null>("popover-state", (e) => { open = e.payload; showOpen(); });
// "Show at the top edge": the bar is tucked above the screen (and lets
// clicks through) until the pointer touches the top edge; the app slides it.
if (inTauri) {
  listen<boolean>("topbar-slide", (e) => document.body.classList.toggle("tucked", e.payload));
  listen<string>("topbar-mode", (e) => { document.body.dataset.bar = e.payload; });
} else if (q.get("tucked")) document.body.classList.add("tucked");

const go = (id: string, what: string) => $(id).addEventListener("click", () => invoke("topbar_open", { what }));
go("search", "search");
go("battery", "battery");
go("clock", "notifications");

function clock(): void {
  const d = new Date();
  const day = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  $("clock").textContent = q.get("clock") ?? `${day.replace(/,/g, "")}  ${time}`;
}

async function refresh(): Promise<void> {
  const s = await invoke<TopState>("topbar_state");
  if (!s) return;
  // Keep the open app menu about the app it was opened for.
  if (!open || open === "cc") {
    front = s;
    appBtn.textContent = s.kind === "desktop" || s.kind === "shell" ? "Desktop" : s.app || "Desktop";
    setMenus(s.kind);
  }
  const b = s.battery;
  $("battery").classList.toggle("hidden", !b);
  if (b) {
    const f = $("batt-fill");
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 16.6))}px`;
    f.className = `fill${b.charging ? " charging" : b.percent <= 20 ? " low" : ""}`;
    $("battery").title = b.charging ? `Charging, ${b.percent}%` : `Battery ${b.percent}%`;
  }
  const snd = $("sound-btn");
  snd.classList.toggle("muted", s.muted || s.volume === 0);
  snd.classList.toggle("low", (s.volume ?? 100) < 34);
  snd.title = s.volume === null ? "Sound" : s.muted ? "Sound (muted)" : `Sound ${s.volume}%`;
}

// ---- Wallpaper plate: frosted once per wallpaper change, aligned to the
// screen (the bar sits at its display's top-left).
const plate = $("plate");
const glassEl = $("glass");
async function loadPlate(): Promise<void> {
  // The bar spans its display, so its width is the display's width.
  const mw = innerWidth || screen.width, mh = screen.height;
  const root = document.documentElement.style;
  root.setProperty("--mon-w", `${mw}px`);
  root.setProperty("--mon-h", `${mh}px`);
  const dark = document.documentElement.dataset.mode === "dark";
  const floor = parseFloat(getComputedStyle(document.documentElement).getPropertyValue("--c-bar-alpha")) || 0.59;
  const src = wallpaperUrl();
  try {
    const p = await makePlate(src, mw, mh);
    plate.classList.remove("css-blur");
    plate.style.backgroundImage = `url("${p.url}")`;
    root.setProperty("--bar-a", tintFor(p.lum(0, 24), dark, floor).toFixed(3));
    // Edge band only (bezel 6 DIP): the centre stays exact.
    const r = glassEl.getBoundingClientRect();
    plate.style.filter = `url(#${refractionFilter(r.width, r.height, 0, { bezel: 6, thickness: 14, strength: 0.6 })})`;
    plate.classList.add("ready");
  } catch {
    // The picture couldn't be read into a canvas: blur it with CSS instead
    // (still a static layer), with the tint at its floor.
    const img = new Image();
    img.onload = () => { plate.classList.add("css-blur", "ready"); plate.style.backgroundImage = `url("${src}")`; plate.style.filter = ""; };
    img.onerror = () => plate.classList.remove("ready"); // solid colour desktop: the opaque base alone
    img.src = src;
    root.removeProperty("--bar-a");
  }
}
loadPlate();
if (inTauri) listen("wallpaper-changed", loadPlate);
new MutationObserver(loadPlate).observe(document.documentElement, { attributes: true, attributeFilter: ["data-mode"] });
addEventListener("resize", () => { plate.style.filter = ""; loadPlate(); });
// Windows can change the picture by itself (slideshow, Spotlight).
setInterval(loadPlate, 5 * 60 * 1000);

clock();
refresh().then(() => { if (!inTauri) { open = q.get("open"); showOpen(); } });
setInterval(clock, 1000);
setInterval(refresh, 1000);
