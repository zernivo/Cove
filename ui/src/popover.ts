// Menu bar popovers: the Cove menu, the app menu, the Restart / Shut down
// confirmation and Control Centre. Each lives in this one small window,
// which the app sizes to exactly the popover shown (popover_place), so the
// rest of the screen stays untouched. Clicking elsewhere closes it (the
// window loses focus); Escape too.
import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface TopState { app: string; kind: "app" | "cove" | "desktop" | "shell"; hwnd: number }
interface Controls { volume: number | null; muted: boolean; brightness: number | null; wifi: string | null; dark: boolean }
type Kind = "cove" | "app" | "confirm" | "cc";

const inTauri = "__TAURI_INTERNALS__" in window;
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  const demo: Record<string, unknown> = { controls_get: { volume: 40, muted: false, brightness: 70, wifi: "TECNO CAMON 30", dark: true } };
  return Promise.resolve((demo[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;

const views: Record<Kind, HTMLElement> = { cove: $("menu"), app: $("appmenu"), confirm: $("confirm"), cc: $("cc") };
let current: Kind | null = null;

/** Show one popover, then have the app size the window to it and show it. */
async function show(kind: Kind): Promise<void> {
  for (const [k, el] of Object.entries(views)) el.classList.toggle("hidden", k !== kind);
  current = kind;
  // Restart the entrance fade.
  const el = views[kind];
  el.style.animation = "none";
  void el.offsetWidth;
  el.style.animation = "";
  await new Promise(requestAnimationFrame);
  const r = el.getBoundingClientRect();
  await invoke("popover_place", { kind, width: Math.ceil(r.width), height: Math.ceil(r.height) }).catch(() => {});
}

function hide(): void {
  current = null;
  invoke("popover_hide").catch(() => {});
}

// ---- The Cove menu ----
const POWER_TEXT: Record<string, [string, string]> = {
  restart: ["Restart your PC now? Apps will be asked to close, so you can save your work.", "Restart"],
  shutdown: ["Shut down your PC now? Apps will be asked to close, so you can save your work.", "Shut Down"],
};
views.cove.querySelectorAll<HTMLButtonElement>("button[data-cove]").forEach((b) =>
  b.addEventListener("click", async () => {
    const what = b.dataset.cove!;
    hide();
    try {
      await invoke("topbar_menu", { what });
    } catch (err) {
      // Windows couldn't show its own Restart / Shut down prompt: ask here.
      if (String(err) === "confirm" && POWER_TEXT[what]) askPower(what);
    }
  }),
);
function askPower(what: string): void {
  const [text, yes] = POWER_TEXT[what];
  $("confirm-text").textContent = text;
  $("confirm-yes").textContent = yes;
  views.confirm.dataset.what = what;
  show("confirm");
}
$("confirm-no").addEventListener("click", hide);
$("confirm-yes").addEventListener("click", () => {
  const what = views.confirm.dataset.what!;
  hide();
  invoke("topbar_power_now", { what }).catch(() => {});
});

// ---- The app menu: what makes sense for the app in front ----
function buildAppMenu(s: TopState): void {
  const m = views.app;
  m.innerHTML = "";
  const add = (label: string, run: () => unknown, enabled = true) => {
    const b = document.createElement("button");
    b.textContent = label;
    b.disabled = !enabled;
    b.addEventListener("click", () => { hide(); run(); });
    m.appendChild(b);
  };
  const sep = () => m.appendChild(document.createElement("hr"));
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
  const name = s.app || "Desktop";
  add("Show All Windows", () => invoke("topbar_app", { what: "expose", hwnd: s.hwnd, name }), has);
  add(`Hide ${name}`, () => invoke("topbar_app", { what: "hide", hwnd: s.hwnd, name }), has);
  sep();
  add(`Quit ${name}`, () => invoke("topbar_app", { what: "quit", hwnd: s.hwnd, name }), has);
}

// ---- Control Centre ----
const vol = $("vol") as HTMLInputElement;
const bright = $("bright") as HTMLInputElement;
const paint = (input: HTMLInputElement) => (input.parentElement as HTMLElement).style.setProperty("--v", `${input.value}%`);

function renderControls(c: Controls): void {
  $("wifi-name").textContent = c.wifi ?? "Not connected";
  $("t-wifi").classList.toggle("on", !!c.wifi);
  $("t-dark").classList.toggle("on", c.dark);
  $("dark-state").textContent = c.dark ? "On" : "Off";
  // Desktop monitors set their brightness themselves: keep Night light.
  $("bright-slider").classList.toggle("hidden", c.brightness === null);
  if (c.brightness !== null) { bright.value = String(c.brightness); paint(bright); }
  $("vol-card").classList.toggle("hidden", c.volume === null);
  if (c.volume !== null) { vol.value = String(c.volume); paint(vol); }
  $("mute").classList.toggle("on", c.muted);
}

// Sliders: apply while dragging, at most every `ms`.
function throttled(what: string, el: HTMLInputElement, ms: number): void {
  let t = 0;
  let pending = false;
  const send = () => { pending = false; t = Date.now(); invoke("controls_set", { what, value: Number(el.value) }).catch(() => {}); };
  el.addEventListener("input", () => {
    paint(el);
    if (Date.now() - t > ms) send();
    else if (!pending) { pending = true; setTimeout(send, ms); }
  });
}
throttled("volume", vol, 80);
throttled("brightness", bright, 300);
$("mute").addEventListener("click", () => {
  invoke("controls_set", { what: "mute", value: 0 }).catch(() => {});
  $("mute").classList.toggle("on");
});
$("t-dark").addEventListener("click", async () => {
  const on = !$("t-dark").classList.contains("on");
  await invoke("controls_set", { what: "dark", value: on ? 1 : 0 }).catch(() => {});
  $("t-dark").classList.toggle("on", on);
  $("dark-state").textContent = on ? "On" : "Off";
});
// These open Windows' own settings page: close first, then open.
const go = (el: HTMLElement, what: string) => el.addEventListener("click", () => { hide(); invoke("topbar_open", { what }).catch(() => {}); });
go($("t-wifi"), "wifi");
go($("t-bt"), "bluetooth");
go($("t-focus"), "focus");
go($("t-night"), "nightlight");
go($("t-sound"), "sound");
views.cc.querySelectorAll<HTMLButtonElement>(".foot button[data-go]").forEach((b) => go(b, b.dataset.go!));

// ---- Open / close ----
if (inTauri) {
  listen<{ kind: Kind; data: TopState | null }>("popover-show", async (e) => {
    const { kind, data } = e.payload;
    if (kind === "app") {
      if (!data) return;
      buildAppMenu(data);
    }
    if (kind === "cc") renderControls(await invoke<Controls>("controls_get"));
    show(kind);
  });
} else {
  // Browser preview: ?k=cc|cove|app
  const k = (new URLSearchParams(location.search).get("k") ?? "cc") as Kind;
  if (k === "app") buildAppMenu({ app: "Google Chrome", kind: "app", hwnd: 1 });
  if (k === "cc") invoke<Controls>("controls_get").then(renderControls);
  show(k);
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") hide();
  // Arrow keys move through a menu, like a native one.
  if (current && current !== "cc" && (e.key === "ArrowDown" || e.key === "ArrowUp")) {
    const items = [...views[current].querySelectorAll<HTMLButtonElement>("button:not(:disabled)")];
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    const n = items.length;
    items[(i + (e.key === "ArrowDown" ? 1 : n - 1) + n) % n]?.focus();
    e.preventDefault();
  }
});
