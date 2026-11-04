import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TopState { app: string; kind: "app" | "cove" | "desktop" | "shell"; hwnd: number; battery: { percent: number; charging: boolean } | null; volume: number | null; muted: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const demo: Record<string, unknown> = {
    topbar_state: { app: new URLSearchParams(location.search).get("app") ?? "Google Chrome", kind: "app", hwnd: 1, battery: { percent: 53, charging: false }, volume: 40, muted: false },
  };
  return Promise.resolve((demo[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

// The Cove menu, the app menu and Control Centre open in their own small
// popover window (popover.ts), sized to the popover: the bar window never
// grows, so nothing but the popover appears over the apps.
type Kind = "cove" | "app" | "cc";
const mark = $("mark");
const appBtn = $("app");
let front: TopState | null = null;
let open: string | null = null;

function openPopover(kind: Kind): void {
  if (kind === "app" && !front) return;
  const right = kind === "cc";
  const anchor = kind === "cove" ? mark : kind === "app" ? appBtn : null;
  // Menus hang from their title; Control Centre from the bar's right end.
  const x = anchor ? anchor.getBoundingClientRect().left : innerWidth - 2;
  invoke("popover_toggle", { kind, x, right, data: kind === "app" ? front : null }).catch(() => {});
}

function showOpen(): void {
  mark.classList.toggle("open", open === "cove" || open === "confirm");
  appBtn.classList.toggle("open", open === "app");
  $("control").classList.toggle("open", open === "cc");
}
if (inTauri) listen<string | null>("popover-state", (e) => { open = e.payload; showOpen(); });
else refresh().then(() => { open = new URLSearchParams(location.search).get("open"); showOpen(); });

// Press-to-open, like the Mac menu bar: menus respond on pointer-down.
mark.addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover("cove"); });
appBtn.addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover("app"); });
// With a menu open, sliding to the other title switches menus (as on a Mac).
mark.addEventListener("pointerenter", () => { if (open === "app") openPopover("cove"); });
appBtn.addEventListener("pointerenter", () => { if (open === "cove") openPopover("app"); });
for (const id of ["control", "wifi-btn"]) $(id).addEventListener("pointerdown", (e) => { if (e.button === 0) openPopover("cc"); });

const go = (id: string, what: string) => $(id).addEventListener("click", () => invoke("topbar_open", { what }));
go("search", "search");
go("battery", "battery");
go("clock", "notifications");

function clock(): void {
  const d = new Date();
  const day = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  $("clock").textContent = `${day.replace(/,/g, "")}  ${time}`;
}

async function refresh(): Promise<void> {
  const s = await invoke<TopState>("topbar_state");
  if (!s) return;
  // Keep the open app menu about the app it was opened for.
  if (open !== "app") {
    front = s;
    appBtn.textContent = s.app || "Desktop";
  }
  const b = s.battery;
  $("battery").classList.toggle("hidden", !b);
  if (b) {
    const f = $("batt-fill");
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 16.4))}px`;
    f.className = `fill${b.charging ? " charging" : b.percent <= 20 ? " low" : ""}`;
    $("battery").title = b.charging ? `Charging, ${b.percent}%` : `Battery ${b.percent}%`;
  }
}

// ---- Wallpaper plate: the desktop picture, blurred, under the bar's tint.
const plate = $("plate-img") as HTMLImageElement;
function loadPlate(): void {
  document.documentElement.style.setProperty("--screen-w", `${screen.width}px`);
  document.documentElement.style.setProperty("--screen-h", `${screen.height}px`);
  const base = inTauri ? (navigator.userAgent.includes("Windows") ? "http://coveicon.localhost" : "coveicon://localhost") + "/wallpaper" : (new URLSearchParams(location.search).get("wall") ?? "/wallpapers/tidepool-dark.jpg");
  const img = new Image();
  img.onload = () => { plate.src = img.src; plate.classList.add("ready"); };
  // No picture (solid colour desktop): the opaque Ink base alone.
  img.onerror = () => plate.classList.remove("ready");
  img.src = inTauri ? `${base}?t=${Date.now()}` : base;
}
loadPlate();
if (inTauri) listen("wallpaper-changed", loadPlate);
// Windows can change the picture by itself (slideshow, Spotlight).
setInterval(loadPlate, 5 * 60 * 1000);

clock();
refresh();
setInterval(clock, 1000);
setInterval(refresh, 1000);
