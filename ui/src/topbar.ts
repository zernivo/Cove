import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TopState { app: string; battery: { percent: number; charging: boolean } | null; volume: number | null; muted: boolean }
interface Controls { volume: number | null; muted: boolean; brightness: number | null; wifi: string | null; dark: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const demo: Record<string, unknown> = {
    topbar_state: { app: "Google Chrome", battery: { percent: 53, charging: false }, volume: 40, muted: false },
    controls_get: { volume: 40, muted: false, brightness: 70, wifi: "HomeNet", dark: true },
  };
  return Promise.resolve((demo[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

// The menu needs room below the 30px bar, so the window grows while it's open.
async function setTall(tall: boolean): Promise<void> {
  if (!inTauri) return;
  const w = getCurrentWindow();
  const size = await w.innerSize();
  const sf = await w.scaleFactor();
  const { PhysicalSize } = await import("@tauri-apps/api/dpi");
  await w.setSize(new PhysicalSize(size.width, Math.round((tall ? 400 : 30) * sf)));
}

// ---- Control Centre ----
const panel = $("panel");
const vol = $("vol") as HTMLInputElement;
const bright = $("bright") as HTMLInputElement;

function renderControls(c: Controls): void {
  $("wifi-name").textContent = c.wifi ?? "Not connected";
  $("t-wifi").classList.toggle("on", !!c.wifi);
  $("t-dark").classList.toggle("on", c.dark);
  $("dark-state").textContent = c.dark ? "On" : "Off";
  $("bright-card").classList.toggle("hidden", c.brightness === null);
  if (c.brightness !== null) bright.value = String(c.brightness);
  $("vol-card").classList.toggle("hidden", c.volume === null);
  if (c.volume !== null) vol.value = String(c.volume);
  $("mute").classList.toggle("on", c.muted);
}

async function togglePanel(open?: boolean): Promise<void> {
  const show = open ?? panel.classList.contains("hidden");
  if (show) {
    await toggleMenu(false);
    await setTall(true);
    panel.classList.remove("hidden");
    renderControls(await invoke<Controls>("controls_get"));
  } else if (!panel.classList.contains("hidden")) {
    panel.classList.add("hidden");
    await setTall(false);
  }
  $("control").classList.toggle("open", show);
}

// Sliders: apply while dragging, at most every 120 ms.
function throttled(what: string, el: HTMLInputElement, ms: number): void {
  let t = 0;
  let pending = false;
  const send = () => { pending = false; t = Date.now(); invoke("controls_set", { what, value: Number(el.value) }); };
  el.addEventListener("input", () => {
    if (Date.now() - t > ms) send();
    else if (!pending) { pending = true; setTimeout(send, ms); }
  });
}
throttled("volume", vol, 80);
throttled("brightness", bright, 300);
$("mute").addEventListener("click", () => {
  invoke("controls_set", { what: "mute", value: 0 });
  $("mute").classList.toggle("on");
});
$("t-dark").addEventListener("click", async () => {
  const on = !$("t-dark").classList.contains("on");
  await invoke("controls_set", { what: "dark", value: on ? 1 : 0 });
  $("t-dark").classList.toggle("on", on);
  $("dark-state").textContent = on ? "On" : "Off";
});
const panelGo = (id: string, what: string) => $(id).addEventListener("click", async () => { await togglePanel(false); invoke("topbar_open", { what }); });
panelGo("t-wifi", "wifi");
panelGo("t-bt", "bluetooth");
panelGo("t-night", "nightlight");
panel.querySelectorAll<HTMLButtonElement>(".foot button[data-go]").forEach((b) =>
  b.addEventListener("click", async () => { await togglePanel(false); invoke("topbar_open", { what: b.dataset.go }); }),
);

const menu = $("menu");
const mark = $("mark");
async function toggleMenu(open?: boolean): Promise<void> {
  const show = open ?? menu.classList.contains("hidden");
  if (show && !panel.classList.contains("hidden")) await togglePanel(false);
  if (!show && menu.classList.contains("hidden")) return;
  if (show) await setTall(true);
  menu.classList.toggle("hidden", !show);
  mark.classList.toggle("open", show);
  if (!show) await setTall(false);
}
mark.addEventListener("click", (e) => { e.stopPropagation(); toggleMenu(); });
menu.querySelectorAll<HTMLButtonElement>("button[data-go]").forEach((b) =>
  b.addEventListener("click", async () => { await toggleMenu(false); invoke("topbar_open", { what: b.dataset.go }); }),
);
document.addEventListener("click", (e) => {
  const t = e.target as Node;
  if (!menu.contains(t)) toggleMenu(false);
  if (!panel.contains(t) && !["control", "wifi-btn", "vol-btn"].some((id) => $(id).contains(t))) togglePanel(false);
});
window.addEventListener("blur", () => { toggleMenu(false); togglePanel(false); });

const go = (id: string, what: string) => $(id).addEventListener("click", () => invoke("topbar_open", { what }));
go("search", "search");
for (const id of ["control", "wifi-btn", "vol-btn"]) $(id).addEventListener("click", () => togglePanel());
go("battery", "battery");
go("clock", "notifications");
document.querySelector(".app")!.addEventListener("click", () => invoke("topbar_open", { what: "cove" }));

function clock(): void {
  const d = new Date();
  const day = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  $("clock").textContent = `${day}  ${time}`;
}

async function refresh(): Promise<void> {
  const s = await invoke<TopState>("topbar_state");
  if (!s) return;
  $("app").textContent = s.app || "Cove";
  const b = s.battery;
  $("battery").classList.toggle("hidden", !b);
  if (b) {
    $("batt-text").textContent = `${b.percent}%`;
    const f = $("batt-fill");
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 17))}px`;
    f.className = `fill${b.charging ? " charging" : b.percent <= 20 ? " low" : ""}`;
    $("battery").title = b.charging ? `Charging, ${b.percent}%` : `Battery ${b.percent}%`;
  }
  $("vol-btn").classList.toggle("hidden", s.volume === null);
  const silent = s.muted || s.volume === 0;
  $("vol-waves").classList.toggle("hidden", silent);
  $("vol-x").classList.toggle("hidden", !silent);
  $("vol-btn").title = silent ? "Sound: muted" : `Sound: ${s.volume}%`;
}

clock();
refresh();
setInterval(clock, 1000);
setInterval(refresh, 1000);
