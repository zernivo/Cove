import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface TopState { app: string; kind: "app" | "cove" | "desktop" | "shell"; hwnd: number; battery: { percent: number; charging: boolean } | null; volume: number | null; muted: boolean }
interface Controls { volume: number | null; muted: boolean; brightness: number | null; wifi: string | null; dark: boolean }

const inTauri = "__TAURI_INTERNALS__" in window;
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const demo: Record<string, unknown> = {
    topbar_state: { app: "Google Chrome", kind: "app", hwnd: 1, battery: { percent: 53, charging: false }, volume: 40, muted: false },
    controls_get: { volume: 40, muted: false, brightness: 70, wifi: "HomeNet", dark: true },
  };
  return Promise.resolve((demo[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

// Bar height in DIP; must match TOPBAR_HEIGHT in main.rs (the reserved strip).
const BAR_H = 25;
// Menus need room below the bar, so the window grows while one is open
// (the reserved strip stays BAR_H: only the window grows, over the apps).
async function setTall(tall: boolean): Promise<void> {
  if (!inTauri) return;
  const w = getCurrentWindow();
  const size = await w.innerSize();
  const sf = await w.scaleFactor();
  const { PhysicalSize } = await import("@tauri-apps/api/dpi");
  await w.setSize(new PhysicalSize(size.width, Math.round((tall ? 400 : BAR_H) * sf)));
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

// ---- Menus: the Cove menu (mark) and the app menu (app name) ----
const menu = $("menu");
const mark = $("mark");
const appBtn = $("app");
const appMenu = $("appmenu");
const confirmBox = $("confirm");
let front: TopState | null = null;
let openMenuEl: HTMLElement | null = null;

async function closeMenus(): Promise<void> {
  if (!openMenuEl) return;
  for (const m of [menu, appMenu, confirmBox]) m.classList.add("hidden");
  mark.classList.remove("open");
  appBtn.classList.remove("open");
  openMenuEl = null;
  await setTall(false);
}
async function openMenu(m: HTMLElement, anchor: HTMLElement): Promise<void> {
  if (!panel.classList.contains("hidden")) await togglePanel(false);
  for (const o of [menu, appMenu, confirmBox]) if (o !== m) o.classList.add("hidden");
  mark.classList.toggle("open", anchor === mark);
  appBtn.classList.toggle("open", anchor === appBtn);
  await setTall(true);
  m.style.left = `${Math.max(6, anchor.getBoundingClientRect().left)}px`;
  m.classList.remove("hidden");
  openMenuEl = m;
}
// Kept for the Control Centre, which closes any open menu first.
async function toggleMenu(open?: boolean): Promise<void> {
  if (open === false) return closeMenus();
  if (openMenuEl === menu) return closeMenus();
  return openMenu(menu, mark);
}
// Press-to-open, like the Mac menu bar: menus respond on pointer-down.
mark.addEventListener("pointerdown", (e) => { if (e.button === 0) { e.stopPropagation(); toggleMenu(); } });
mark.addEventListener("click", (e) => e.stopPropagation());

const POWER_TEXT: Record<string, [string, string]> = {
  restart: ["Restart your PC now? Apps will be asked to close, so you can save your work.", "Restart"],
  shutdown: ["Shut down your PC now? Apps will be asked to close, so you can save your work.", "Shut Down"],
};
function askPower(what: string): void {
  const [text, yes] = POWER_TEXT[what];
  $("confirm-text").textContent = text;
  $("confirm-yes").textContent = yes;
  confirmBox.dataset.what = what;
  openMenu(confirmBox, mark);
}
$("confirm-no").addEventListener("click", () => closeMenus());
$("confirm-yes").addEventListener("click", async () => {
  const what = confirmBox.dataset.what!;
  await closeMenus();
  invoke("topbar_power_now", { what }).catch(() => {});
});

menu.querySelectorAll<HTMLButtonElement>("button[data-cove]").forEach((b) =>
  b.addEventListener("click", async (e) => {
    e.stopPropagation();
    const what = b.dataset.cove!;
    await closeMenus();
    try {
      await invoke("topbar_menu", { what });
    } catch (err) {
      // Windows couldn't show its own Restart / Shut down prompt: ask here.
      if (String(err) === "confirm" && POWER_TEXT[what]) askPower(what);
    }
  }),
);

// The app menu lists what makes sense for what's in front.
function buildAppMenu(s: TopState): void {
  appMenu.innerHTML = "";
  const add = (label: string, run: () => unknown, enabled = true) => {
    const b = document.createElement("button");
    b.textContent = label;
    b.disabled = !enabled;
    b.addEventListener("click", async (e) => { e.stopPropagation(); await closeMenus(); run(); });
    appMenu.appendChild(b);
  };
  const sep = () => appMenu.appendChild(document.createElement("hr"));
  if (s.kind === "cove") {
    add("About Cove", () => invoke("topbar_menu", { what: "about" }));
    add("Cove Settings…", () => invoke("topbar_menu", { what: "settings" }));
    sep();
    add("Hide Cove", () => invoke("topbar_app", { what: "hide", hwnd: s.hwnd, name: s.app }));
    sep();
    add("Quit Cove", () => invoke("topbar_menu", { what: "quit" }));
    return;
  }
  const has = s.kind === "app" && s.hwnd !== 0;
  add("Show All Windows", () => invoke("topbar_app", { what: "expose", hwnd: s.hwnd, name: s.app }), has);
  add(`Hide ${s.app}`, () => invoke("topbar_app", { what: "hide", hwnd: s.hwnd, name: s.app }), has);
  sep();
  add(`Quit ${s.app}`, () => invoke("topbar_app", { what: "quit", hwnd: s.hwnd, name: s.app }), has);
}
appBtn.addEventListener("pointerdown", (e) => {
  if (e.button !== 0) return;
  e.stopPropagation();
  if (openMenuEl === appMenu) { closeMenus(); return; }
  if (!front) return;
  buildAppMenu(front);
  openMenu(appMenu, appBtn);
});
appBtn.addEventListener("click", (e) => e.stopPropagation());
// With a menu open, sliding to the other title switches menus (as on a Mac).
mark.addEventListener("pointerenter", () => { if (openMenuEl === appMenu) openMenu(menu, mark); });
appBtn.addEventListener("pointerenter", () => {
  if (openMenuEl === menu && front) { buildAppMenu(front); openMenu(appMenu, appBtn); }
});

document.addEventListener("click", (e) => {
  const t = e.target as Node;
  if (openMenuEl && !openMenuEl.contains(t)) closeMenus();
  if (!panel.contains(t) && !["control", "wifi-btn", "vol-btn"].some((id) => $(id).contains(t))) togglePanel(false);
});
document.addEventListener("keydown", (e) => { if (e.key === "Escape") { closeMenus(); togglePanel(false); } });
window.addEventListener("blur", () => { closeMenus(); togglePanel(false); });

const go = (id: string, what: string) => $(id).addEventListener("click", () => invoke("topbar_open", { what }));
go("search", "search");
for (const id of ["control", "wifi-btn", "vol-btn"]) $(id).addEventListener("click", () => togglePanel());
go("battery", "battery");
go("clock", "notifications");

function clock(): void {
  const d = new Date();
  const day = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" });
  const time = d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
  $("clock").textContent = `${day}  ${time}`;
}

async function refresh(): Promise<void> {
  const s = await invoke<TopState>("topbar_state");
  if (!s) return;
  // Keep the open app menu about the app it was opened for.
  if (openMenuEl !== appMenu) {
    front = s;
    appBtn.textContent = s.app || "Desktop";
  }
  const b = s.battery;
  $("battery").classList.toggle("hidden", !b);
  if (b) {
    $("batt-text").textContent = `${b.percent}%`;
    const f = $("batt-fill");
    f.style.width = `${Math.max(2, Math.round((b.percent / 100) * 16))}px`;
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
