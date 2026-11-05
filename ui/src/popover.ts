// Popovers: the Cove menu, the bar's app / File / View / Go / Window menus,
// dock context menus, the Restart / Shut Down confirmation and Control
// Centre. They all live in this one window, which the app sizes and shapes
// (a rounded window region) to exactly what's shown, so the rest of the
// screen stays untouched. Clicking elsewhere closes it; Escape too.
//
// Motion (user pick, 2026-11-04): open with a calm fade + scale 0.97 -> 1
// and a 4 DIP slide, 180 ms ease-out; close faster (120 ms). The glass
// itself is there from the first frame (Windows acrylic can't scale), and
// the content only becomes visible once the window is on screen, so there
// is never an empty or stale first frame.
import "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { mountTrayGrid } from "./tray-grid";
import { MOCK_STATUS } from "./tray";

interface TopState { app: string; kind: "app" | "cove" | "desktop" | "shell"; hwnd: number; count?: number; icon?: string | null; user?: string }
interface Media { title: string; artist: string; app: string; playing: boolean; icon: string | null }
interface Controls { volume: number | null; muted: boolean; brightness: number | null; wifi: string | null; dark: boolean; bluetooth?: boolean | null; bt_devices?: number | null; focus?: boolean | null; device?: string | null; media?: Media | null }
export interface MItem { id?: string; label?: string; kbd?: string; icon?: string | null; check?: boolean; disabled?: boolean; sub?: MItem[]; sep?: boolean; header?: string; headerIcon?: string | null; danger?: boolean; count?: string; run?: () => unknown }
type Kind = "cove" | "app" | "file" | "view" | "go" | "window" | "dock" | "confirm" | "cc" | "cal";

const inTauri = "__TAURI_INTERNALS__" in window;
const onWindows = inTauri && navigator.userAgent.includes("Windows");
const q = new URLSearchParams(location.search);
const MOCK: Record<string, unknown> = {
  controls_get: { volume: 46, muted: false, brightness: 72, wifi: "Nazeer-5G", dark: matchMedia("(prefers-color-scheme: dark)").matches, bluetooth: true, bt_devices: 1, focus: false, device: "Speakers (Realtek)", media: { title: "Golden Hour", artist: "JVKE", app: "Spotify", playing: true, icon: "/mock/spotify.png" } },
  icons_system: { settings: "/mock/settings.png", taskmgr: "/mock/taskmgr.png", explorer: "/mock/explorer.png", cove: "/cove-mark.png", downloads: "/mock/downloads.png", documents: "/mock/folder.png", desktop: "/mock/folder.png", pictures: "/mock/folder.png", home: "/mock/folder.png", thispc: "/mock/thispc.png", recycler: "/mock/bin.png", controlpanel: "/mock/settings.png" },
  // Preview: ?tray=empty | note shows those states.
  tray_status: q.get("tray") === "empty" ? { ...MOCK_STATUS, items: [] } : q.get("tray") === "note" ? { ...MOCK_STATUS, mode: "windows", note: "TranslucentTB also manages the tray, so Cove lists the icons Windows shows instead. Quit TranslucentTB to see every icon here.", items: MOCK_STATUS.items.slice(0, 3) } : MOCK_STATUS,
  recent_items: [{ name: "CA_Assignment_2.pdf", target: "x", icon: null }, { name: "visualiser.py", target: "y", icon: null }, { name: "Vision statement.docx", target: "z", icon: null }],
};
function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (inTauri) return tauriInvoke<T>(cmd, args);
  return Promise.resolve((MOCK[cmd] ?? null) as T);
}
const $ = (id: string) => document.getElementById(id)!;
if (onWindows) document.documentElement.classList.add("acrylic");

const stage = $("stage");
const menuEl = $("menu");
const subEl = $("sub");
const views: Record<"menu" | "confirm" | "cc" | "cal", HTMLElement> = { menu: menuEl, confirm: $("confirm"), cc: $("cc"), cal: $("cal") };
let current: Kind | null = null;
/** The popover stands above the dock (rises from below) instead of
 *  hanging from the menu bar. Set by the app with each popover-show. */
let up = false;
let icons: Record<string, string | null> = {};
const reduced = matchMedia("(prefers-reduced-motion: reduce)");

// Let the app tint the acrylic for the current look (so the glass is Ink,
// or paper, from its very first frame).
function syncGlass(): void { invoke("glass_tint", { dark: document.documentElement.dataset.mode === "dark" }).catch(() => {}); }
syncGlass();
new MutationObserver(syncGlass).observe(document.documentElement, { attributes: true, attributeFilter: ["data-mode"] });

async function loadIcons(): Promise<void> {
  icons = (await invoke<Record<string, string | null>>("icons_system").catch(() => null)) ?? {};
}
loadIcons();

// ---- Layout, shape and placement ----
let subFor: HTMLElement | null = null;
const bottomAnchored = () => up;
const viewOf = (k: Kind | null): "menu" | "confirm" | "cc" | "cal" => (k === "confirm" || k === "cc" || k === "cal" ? k : "menu");

function layout(): { w: number; h: number; shapes: number[][]; anchorW: number } {
  const main = views[viewOf(current)];
  const mr = main.getBoundingClientRect();
  const mw = Math.ceil(mr.width), mh = Math.ceil(mr.height);
  const radius = (el: HTMLElement) => parseFloat(getComputedStyle(el).borderTopLeftRadius) || 0;
  const shapes = [[0, 0, mw, mh, radius(main)]];
  let w = mw, h = mh;
  main.style.position = "absolute";
  main.style.left = "0px";
  main.style.top = "0px";
  if (subFor && !subEl.classList.contains("hidden")) {
    const sr = subEl.getBoundingClientRect();
    const sw = Math.ceil(sr.width), sh = Math.ceil(sr.height);
    const gap = 3;
    const rowTop = subFor.offsetTop - 5;
    let mainTop = 0, subTop: number;
    if (bottomAnchored()) {
      h = Math.max(mh, sh);
      mainTop = h - mh;
      subTop = Math.min(Math.max(0, mainTop + rowTop), h - sh);
    } else {
      subTop = Math.max(0, rowTop);
      h = Math.max(mh, subTop + sh);
    }
    w = mw + gap + sw;
    main.style.top = `${mainTop}px`;
    subEl.style.left = `${mw + gap}px`;
    subEl.style.top = `${subTop}px`;
    shapes[0][1] = mainTop;
    shapes.push([mw + gap, subTop, sw, sh, radius(subEl)]);
  }
  return { w, h, shapes, anchorW: mw };
}

async function place(): Promise<void> {
  document.body.classList.remove("fit");
  const { w, h, shapes, anchorW } = layout();
  if (!inTauri) parent?.postMessage({ coveShapes: shapes, w, h, kind: current }, "*");
  await invoke("popover_place", { kind: current, width: w, height: h, shapes, anchorW }).catch(() => {});
  document.body.classList.add("fit");
  repaint();
}

/** Nudge a paint-only property so the compositor produces a new frame. */
function repaint(): void {
  document.body.classList.toggle("tick");
  requestAnimationFrame(() => requestAnimationFrame(() => document.body.classList.toggle("tick")));
}

let anim: Animation | null = null;
function enter(): void {
  anim?.cancel();
  const up = bottomAnchored();
  stage.style.opacity = "";
  const frames = reduced.matches
    ? [{ opacity: 0 }, { opacity: 1 }]
    : [{ opacity: 0, transform: `translateY(${up ? 4 : -4}px) scale(0.97)` }, { opacity: 1, transform: "none" }];
  for (const el of Object.values(views)) {
    el.style.setProperty("--origin", up ? "bottom center" : current === "cc" ? "top right" : "top left");
  }
  anim = stage.animate(frames, { duration: reduced.matches ? 150 : 180, easing: "cubic-bezier(0.2, 0.8, 0.2, 1)" });
  // Belt and braces: whatever happens to frames, the content ends visible.
  setTimeout(() => anim?.finish(), 400);
}

/** Render one popover, then have the app size, shape and show the window. */
async function show(kind: Kind): Promise<void> {
  current = kind;
  const view = viewOf(kind);
  for (const [k, el] of Object.entries(views)) el.classList.toggle("hidden", k !== view);
  closeSub(false);
  stage.style.opacity = "0"; // nothing visible until the window is on screen
  await place();
  enter();
}

let closing = false;
function hide(): void {
  if (closing) return;
  closing = true;
  current = null;
  const done = () => { closing = false; stage.style.opacity = "0"; invoke("popover_hide").catch(() => {}); };
  if (reduced.matches || !inTauri) { done(); return; }
  anim?.cancel();
  anim = stage.animate([{ opacity: 1 }, { opacity: 0 }], { duration: 120, easing: "ease-in", fill: "forwards" });
  anim.onfinish = done;
  setTimeout(() => { if (closing) done(); }, 250);
}

// ---- One menu component for every menu ----
const CHECK = `<svg viewBox="0 0 12 12"><path d="M2.4 6.4l2.3 2.3 4.9-5.2"/></svg>`;
function rows(box: HTMLElement, items: MItem[], isSub: boolean): void {
  box.innerHTML = "";
  const withIcons = items.some((i) => i.icon !== undefined && !i.sep && !i.header);
  for (const it of items) {
    if (it.sep) { box.appendChild(document.createElement("hr")).setAttribute("role", "separator"); continue; }
    if (it.header !== undefined) {
      const h = document.createElement("div");
      h.className = "header";
      h.setAttribute("role", "presentation");
      if (it.headerIcon) h.appendChild(Object.assign(document.createElement("img"), { src: it.headerIcon, alt: "" }));
      h.appendChild(document.createTextNode(it.header));
      box.appendChild(h);
      continue;
    }
    const b = document.createElement("button");
    b.className = `row${it.danger ? " danger" : ""}`;
    b.setAttribute("role", it.check !== undefined ? "menuitemcheckbox" : "menuitem");
    if (it.check !== undefined) b.setAttribute("aria-checked", String(!!it.check));
    if (it.sub) { b.setAttribute("aria-haspopup", "menu"); b.setAttribute("aria-expanded", "false"); }
    if (it.kbd) b.setAttribute("aria-keyshortcuts", it.kbd.replace(/Win/g, "Meta"));
    if (withIcons) {
      const ic = document.createElement("span");
      ic.className = `ic${it.icon ? "" : " blank"}`;
      if (it.icon) {
        const img = Object.assign(document.createElement("img"), { src: it.icon, alt: "" });
        img.addEventListener("error", () => { ic.classList.add("blank"); img.remove(); });
        ic.appendChild(img);
      }
      b.appendChild(ic);
    }
    b.appendChild(Object.assign(document.createElement("span"), { className: "t", textContent: it.label ?? "" }));
    if (it.count) b.appendChild(Object.assign(document.createElement("span"), { className: "count", textContent: it.count }));
    if (it.kbd) b.appendChild(Object.assign(document.createElement("kbd"), { textContent: it.kbd }));
    if (it.check !== undefined) { const c = document.createElement("span"); c.className = "check"; if (it.check) c.innerHTML = CHECK; b.appendChild(c); }
    if (it.sub) b.appendChild(Object.assign(document.createElement("span"), { className: "chev", textContent: "›" }));
    b.disabled = !!it.disabled;
    if (it.disabled) b.setAttribute("aria-disabled", "true");
    if (it.sub) {
      const open = () => openSub(b, it.sub!);
      b.addEventListener("pointerenter", open);
      b.addEventListener("click", open);
    } else {
      if (!isSub) b.addEventListener("pointerenter", () => closeSub(true));
      b.addEventListener("click", () => pick(it));
    }
    box.appendChild(b);
  }
}

function pick(it: MItem): void {
  if (it.disabled) return;
  const origin = current;
  hide();
  if (it.run) Promise.resolve(it.run()).catch(() => {});
  else if (origin === "dock" && it.id) invoke("dock_menu_pick", { id: it.id }).catch(() => {});
}

function openSub(row: HTMLElement, items: MItem[]): void {
  if (subFor === row && !subEl.classList.contains("hidden")) return;
  menuEl.querySelectorAll(".row.on").forEach((r) => r.classList.remove("on"));
  row.classList.add("on");
  row.setAttribute("aria-expanded", "true");
  subFor = row;
  rows(subEl, items, true);
  subEl.classList.remove("hidden");
  place();
  subEl.animate([{ opacity: 0 }, { opacity: 1 }], { duration: 120, easing: "ease-out" });
}
function closeSub(replace: boolean): void {
  if (!subFor && subEl.classList.contains("hidden")) return;
  subFor?.classList.remove("on");
  subFor?.setAttribute("aria-expanded", "false");
  subFor = null;
  subEl.classList.add("hidden");
  if (replace) place();
}

function showMenu(kind: Kind, items: MItem[], cls = ""): void {
  menuEl.className = `menu${cls ? ` ${cls}` : ""}`;
  rows(menuEl, items, false);
  show(kind);
}

// ---- Menu contents ----
const cove = (what: string) => () => invoke("topbar_menu", { what });
const openWhat = (what: string) => () => invoke("topbar_open", { what });
const folder = (what: string) => () => invoke("topbar_go", { what });

async function coveMenu(s: TopState | null): Promise<MItem[]> {
  const recent = (await invoke<{ name: string; target: string; icon: string | null }[]>("recent_items").catch(() => null)) ?? [];
  const user = s?.user ? ` ${s.user}` : "";
  return [
    { label: "About This PC", run: cove("about-pc") },
    { sep: true },
    { label: "System Settings…", kbd: "Win+I", icon: icons.settings ?? null, run: openWhat("settings") },
    { label: "Cove Settings…", icon: icons.cove ?? "/cove-mark.png", run: cove("settings") },
    { label: "Recent Items", icon: null, sub: recent.length ? recent.slice(0, 10).map((r) => ({ label: r.name, icon: r.icon, run: () => invoke("search_open", { kind: "file", target: r.target }) })) : [{ label: "No recent items", disabled: true }] },
    { sep: true },
    { label: "Force Quit…", kbd: "Ctrl+Shift+Esc", icon: icons.taskmgr ?? null, run: openWhat("taskmgr") },
    { sep: true },
    { label: "Sleep", run: power("sleep") },
    { label: "Restart…", run: power("restart") },
    { label: "Shut Down…", run: power("shutdown") },
    { sep: true },
    { label: "Lock Screen", kbd: "Win+L", run: cove("lock") },
    { label: `Sign Out${user}…`, run: power("signout") },
    { sep: true },
    { label: "Turn Cove Off", run: cove("desktop-off") },
  ];
}

const POWER_TEXT: Record<string, [string, string]> = {
  restart: ["Restart your PC now? Apps will be asked to close, so you can save your work.", "Restart"],
  shutdown: ["Shut down your PC now? Apps will be asked to close, so you can save your work.", "Shut Down"],
  signout: ["Sign out of Windows now? Apps will be asked to close, so you can save your work.", "Sign Out"],
};
function power(what: string): () => Promise<void> {
  return async () => {
    try {
      await invoke("topbar_menu", { what });
    } catch (err) {
      // Windows couldn't show its own prompt (or it's sign out): ask here.
      if (String(err) === "confirm" && POWER_TEXT[what]) setTimeout(() => askPower(what), 280);
    }
  };
}
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

function appMenu(s: TopState): MItem[] {
  if (s.kind === "cove") {
    return [
      { label: "About Cove", run: cove("about") },
      { sep: true },
      { label: "Cove Settings…", run: cove("settings") },
      { sep: true },
      { label: "Hide Cove", run: () => invoke("topbar_app", { what: "hide", hwnd: s.hwnd, name: s.app }) },
      { label: "Hide Others", run: () => invoke("topbar_app", { what: "hide-others", hwnd: s.hwnd, name: s.app }) },
      { sep: true },
      { label: "Quit Cove", run: cove("quit") },
    ];
  }
  if (s.kind === "desktop" || s.kind === "shell") {
    return [
      { label: "About This PC", run: cove("about-pc") },
      { sep: true },
      { label: "Change Wallpaper…", run: cove("wallpaper") },
      { label: "Cove Settings…", run: cove("settings") },
      { sep: true },
      { label: "Show Desktop", kbd: "Win+D", run: () => invoke("topbar_view", { what: "show-desktop" }) },
    ];
  }
  const has = s.hwnd !== 0;
  const name = s.app || "App";
  const app = (what: string) => () => invoke("topbar_app", { what, hwnd: s.hwnd, name });
  return [
    { header: name, headerIcon: s.icon ?? null },
    { label: "Show All Windows", count: s.count ? String(s.count) : undefined, disabled: !has, run: app("expose") },
    { sep: true },
    { label: `Hide ${name}`, kbd: "Alt+H", disabled: !has, run: app("hide") },
    { label: "Hide Others", disabled: !has, run: app("hide-others") },
    { sep: true },
    { label: `Quit ${name}`, kbd: "Alt+Q", disabled: !has, run: app("quit") },
  ];
}

function fileMenu(): MItem[] {
  return [
    { label: "New File Explorer Window", icon: icons.explorer ?? null, run: folder("explorer") },
    { label: "New Folder on Desktop", kbd: "Ctrl+Shift+N", icon: null, run: () => invoke("topbar_view", { what: "new-folder" }) },
    { sep: true },
    { label: "Open Downloads", icon: icons.downloads ?? null, run: folder("downloads") },
    { label: "Open Recycle Bin", icon: icons.recycler ?? null, run: folder("recycler") },
  ];
}
function viewMenu(): MItem[] {
  return [
    { label: "Show Desktop", kbd: "Win+D", run: () => invoke("topbar_view", { what: "show-desktop" }) },
    { label: "Task View", kbd: "Win+Tab", run: () => invoke("topbar_view", { what: "task-view" }) },
    { sep: true },
    { label: "Show Desktop Icons", run: () => invoke("topbar_view", { what: "toggle-icons" }) },
  ];
}
function goMenu(): MItem[] {
  return [
    { label: "Home", icon: icons.home ?? null, run: folder("home") },
    { label: "Desktop", icon: icons.desktop ?? null, run: folder("desktop") },
    { label: "Documents", icon: icons.documents ?? null, run: folder("documents") },
    { label: "Downloads", icon: icons.downloads ?? null, run: folder("downloads") },
    { label: "Pictures", icon: icons.pictures ?? null, run: folder("pictures") },
    { sep: true },
    { label: "This PC", icon: icons.thispc ?? null, run: folder("thispc") },
    { label: "Recycle Bin", icon: icons.recycler ?? null, run: folder("recycler") },
    { label: "Control Panel", icon: icons.controlpanel ?? null, run: folder("controlpanel") },
  ];
}
function windowMenu(s: TopState | null): MItem[] {
  const has = !!s && s.kind === "app" && s.hwnd !== 0;
  const app = (what: string) => () => invoke("topbar_app", { what, hwnd: s?.hwnd ?? 0, name: s?.app ?? "" });
  return [
    { label: "Minimise", kbd: "Alt+M", disabled: !has, run: app("minimize") },
    { label: "Zoom", disabled: !has, run: app("zoom") },
    { sep: true },
    { label: "Tile Window to Left of Screen", disabled: !has, run: app("tile-left") },
    { label: "Tile Window to Right of Screen", disabled: !has, run: app("tile-right") },
    { sep: true },
    { label: "Bring All to Front", disabled: !has, run: app("front") },
  ];
}

// ---- Control Centre ----
const vol = $("vol") as HTMLInputElement;
const bright = $("bright") as HTMLInputElement;
const paint = (input: HTMLInputElement) => {
  (input.parentElement as HTMLElement).style.setProperty("--n", String(Number(input.value) / 100));
  input.setAttribute("aria-valuetext", `${input.value}%`);
};
const MUSIC = `<svg viewBox="0 0 16 16"><path d="M6 12V3.6l7-1.4v8.2"/><circle cx="4.4" cy="12" r="1.7"/><circle cx="11.4" cy="10.4" r="1.7"/></svg>`;

function renderControls(c: Controls): void {
  $("wifi-name").textContent = c.wifi ?? "Not connected";
  $("t-wifi").classList.toggle("on", !!c.wifi);
  const bt = c.bluetooth;
  $("t-bt").classList.toggle("on", !!bt);
  $("bt-state").textContent = bt === null || bt === undefined ? "Not available" : bt ? (c.bt_devices ? `On · ${c.bt_devices} device${c.bt_devices > 1 ? "s" : ""}` : "On") : "Off";
  $("t-focus").classList.toggle("on", !!c.focus);
  $("focus-state").textContent = c.focus ? "Do not disturb" : "Off";
  $("t-dark").classList.toggle("on", c.dark);
  $("t-dark").setAttribute("aria-checked", String(c.dark));
  $("dark-state").textContent = c.dark ? "On" : "Off";
  // Desktop monitors set their brightness themselves: keep Night light.
  $("bright-slider").classList.toggle("hidden", c.brightness === null);
  $("bright-note").classList.toggle("hidden", c.brightness !== null);
  if (c.brightness !== null) { bright.value = String(c.brightness); paint(bright); }
  $("vol-card").classList.toggle("hidden", c.volume === null);
  if (c.volume !== null) { vol.value = String(c.volume); paint(vol); }
  $("device").textContent = c.device ?? "Sound settings";
  $("mute").classList.toggle("on", c.muted);
  $("mute").setAttribute("aria-pressed", String(c.muted));
  const m = c.media;
  $("media").classList.toggle("hidden", !m);
  if (m) {
    $("media-title").textContent = m.title || "Not playing";
    $("media-sub").textContent = [m.artist, m.app].filter(Boolean).join(" · ");
    const art = $("media-art");
    art.innerHTML = "";
    if (m.icon) art.appendChild(Object.assign(document.createElement("img"), { src: m.icon, alt: "" }));
    else art.innerHTML = MUSIC;
    $("m-play").classList.toggle("paused", !m.playing);
  }
}

// Sliders: apply while dragging, at most every `ms`; the wheel nudges 2 %.
function throttled(what: string, el: HTMLInputElement, ms: number): void {
  let t = 0;
  let pending = false;
  const send = () => { pending = false; t = Date.now(); invoke("controls_set", { what, value: Number(el.value) }).catch(() => {}); };
  const changed = () => {
    paint(el);
    if (Date.now() - t > ms) send();
    else if (!pending) { pending = true; setTimeout(send, ms); }
  };
  el.addEventListener("input", changed);
  el.parentElement!.addEventListener("wheel", (e) => {
    e.preventDefault();
    el.value = String(Math.min(100, Math.max(0, Number(el.value) + (e.deltaY < 0 ? 2 : -2))));
    changed();
  }, { passive: false });
}
throttled("volume", vol, 80);
throttled("brightness", bright, 300);
$("mute").addEventListener("click", () => {
  invoke("controls_set", { what: "mute", value: 0 }).catch(() => {});
  $("mute").setAttribute("aria-pressed", String($("mute").classList.toggle("on")));
});
$("t-dark").addEventListener("click", async () => {
  const on = !$("t-dark").classList.contains("on");
  $("t-dark").classList.toggle("on", on);
  $("t-dark").setAttribute("aria-checked", String(on));
  $("dark-state").textContent = on ? "On" : "Off";
  await invoke("controls_set", { what: "dark", value: on ? 1 : 0 }).catch(() => {});
});
const media = (what: string) => async () => {
  await invoke("media_control", { what }).catch(() => {});
  setTimeout(async () => { if (current === "cc") renderControls(await invoke<Controls>("controls_get")); }, 400);
};
$("m-prev").addEventListener("click", media("prev"));
$("m-next").addEventListener("click", media("next"));
$("m-play").addEventListener("click", () => { $("m-play").classList.toggle("paused"); media("toggle")(); });
// These open Windows' own settings page: close first, then open.
const go = (el: HTMLElement, what: string) => el.addEventListener("click", () => { hide(); invoke("topbar_open", { what }).catch(() => {}); });
go($("t-wifi"), "wifi");
go($("t-bt"), "bluetooth");
go($("t-focus"), "focus");
go($("t-night"), "nightlight");
go($("t-sound"), "sound");
$("edit-controls").addEventListener("click", () => { hide(); invoke("topbar_menu", { what: "settings" }).catch(() => {}); });
views.cc.querySelectorAll<HTMLButtonElement>(".foot button[data-go]").forEach((b) => go(b, b.dataset.go!));

// Other apps' tray icons. Drawn from the last known state at once, then
// refreshed; the window is re-fitted if the grid changed size.
const tray = mountTrayGrid($("tray"), invoke, { onResize: () => { if (current === "cc") place(); } });
async function trayRefresh(): Promise<void> {
  const before = views.cc.offsetHeight;
  await tray.refresh();
  if (current === "cc" && views.cc.offsetHeight !== before) place();
}
if (inTauri) listen("tray-changed", () => { if (current === "cc") trayRefresh(); });

// ---- Calendar (the dock clock) ----
let calMonth = new Date();
function firstWeekday(): number {
  // 0 = Sunday ... 6 = Saturday; Monday unless the locale says otherwise.
  try {
    const loc = new Intl.Locale(navigator.language) as Intl.Locale & { weekInfo?: { firstDay: number }; getWeekInfo?: () => { firstDay: number } };
    const fd = loc.getWeekInfo?.().firstDay ?? loc.weekInfo?.firstDay;
    if (fd) return fd % 7;
  } catch { /* older engine */ }
  return 1;
}
function renderCal(): void {
  const now = q.get("today") ? new Date(q.get("today")!) : new Date();
  $("cal-day").textContent = now.toLocaleDateString(undefined, { weekday: "long" });
  $("cal-date").textContent = now.toLocaleDateString(undefined, { day: "numeric", month: "long", year: "numeric" });
  const y = calMonth.getFullYear(), m = calMonth.getMonth();
  $("cal-title").textContent = new Date(y, m, 1).toLocaleDateString(undefined, { month: "long", year: "numeric" });
  const fd = firstWeekday();
  const grid = $("cal-grid");
  grid.innerHTML = "";
  for (let i = 0; i < 7; i++) {
    const d = new Date(2024, 0, 7 + ((fd + i) % 7)); // 7 Jan 2024 was a Sunday
    grid.appendChild(Object.assign(document.createElement("span"), { className: "wd", textContent: d.toLocaleDateString(undefined, { weekday: "narrow" }) }));
  }
  const lead = (new Date(y, m, 1).getDay() - fd + 7) % 7;
  for (let i = 0; i < 42; i++) {
    const d = new Date(y, m, 1 - lead + i);
    const cell = document.createElement("span");
    const today = d.toDateString() === now.toDateString();
    cell.className = `dd${d.getMonth() !== m ? " out" : ""}${today ? " today" : ""}`;
    cell.textContent = String(d.getDate());
    if (today) cell.setAttribute("aria-current", "date");
    grid.appendChild(cell);
  }
}
$("cal-prev").addEventListener("click", () => { calMonth = new Date(calMonth.getFullYear(), calMonth.getMonth() - 1, 1); renderCal(); });
$("cal-next").addEventListener("click", () => { calMonth = new Date(calMonth.getFullYear(), calMonth.getMonth() + 1, 1); renderCal(); });
$("cal-now").addEventListener("click", () => { calMonth = new Date(); renderCal(); });
go($("cal-notes"), "notifications");
go($("cal-settings"), "datetime");

// ---- Open / close ----
async function open(kind: Kind, data: unknown): Promise<void> {
  if (!Object.keys(icons).length) await loadIcons();
  const s = (data ?? null) as TopState | null;
  switch (kind) {
    case "cove": showMenu(kind, await coveMenu(s), "wide"); break;
    case "app": if (s) showMenu(kind, appMenu(s)); break;
    case "file": showMenu(kind, fileMenu()); break;
    case "view": showMenu(kind, viewMenu()); break;
    case "go": showMenu(kind, goMenu()); break;
    case "window": showMenu(kind, windowMenu(s)); break;
    case "dock": showMenu(kind, ((data as { items?: MItem[] })?.items) ?? [], "dockmenu"); break;
    case "cc": {
      tray.paintCached();
      const t = trayRefresh();
      renderControls(await invoke<Controls>("controls_get"));
      // Wait briefly for the icons so the panel opens at its final size.
      await Promise.race([t, new Promise((r) => setTimeout(r, 120))]);
      show("cc");
      // The dock's "›" opens Control Centre at "Other apps".
      if ((data as { focus?: string } | null)?.focus === "tray") {
        const sec = $("tray");
        sec.scrollIntoView({ block: "nearest" });
        sec.querySelector<HTMLElement>(".tray-cell")?.focus({ preventScroll: true });
      }
      break;
    }
    case "cal": calMonth = q.get("today") ? new Date(q.get("today")!) : new Date(); renderCal(); show("cal"); break;
    default: break;
  }
}

if (inTauri) {
  listen("popover-repaint", repaint);
  listen<{ kind: Kind; data: unknown; up?: boolean }>("popover-show", (e) => { up = !!e.payload.up; open(e.payload.kind, e.payload.data); });
  // The window was hidden by the app (clicked elsewhere): reset for next time.
  listen("popover-hidden", () => { current = null; stage.style.opacity = "0"; closeSub(false); });
} else {
  // Browser preview: ?k=cc|cove|app|file|go|window|dock|cal[&sub=1][&up=1]
  const k = (q.get("k") ?? "cc") as Kind;
  up = k === "dock" || !!q.get("up");
  const demo: TopState = { app: q.get("app") ?? "Google Chrome", kind: (q.get("kind") as TopState["kind"]) ?? "app", hwnd: 1, count: 3, icon: "/mock/chrome.png", user: "Khubaib" };
  const dockDemo = { items: [
    { header: "Google Chrome" },
    { id: "new", label: "New Window" },
    { id: "incog", label: "New Incognito Window", kbd: "Ctrl+Shift+N" },
    { sep: true },
    { id: "expose", label: "Show All Windows" },
    { id: "hide", label: "Hide", kbd: "Alt+H" },
    { sep: true },
    { label: "Options", sub: [{ id: "keep", label: "Keep in Dock", check: true }, { id: "login", label: "Open at Login", check: false }, { id: "reveal", label: "Show in File Explorer" }] },
    { sep: true },
    { id: "quit", label: "Quit", kbd: "Alt+Q" },
  ] as MItem[] };
  // The harness passes the dock's real menu (built by dock.ts) as window.name.
  let dockData: unknown = dockDemo;
  try { if (k === "dock" && window.name) dockData = JSON.parse(window.name); } catch { /* demo */ }
  open(k, k === "dock" ? dockData : demo).then(() => {
    const hl = q.get("hl");
    if (hl) {
      const row = [...menuEl.querySelectorAll<HTMLElement>(".row")].find((r) => r.textContent?.startsWith(hl));
      if (row) { row.classList.add("on"); if (q.get("sub")) row.dispatchEvent(new Event("pointerenter")); }
    }
  });
}

document.addEventListener("keydown", (e) => {
  if (e.key === "Escape") { if (subFor) closeSub(true); else hide(); return; }
  if (!current || current === "cc" || current === "confirm" || current === "cal") return;
  const box = subFor && subEl.contains(document.activeElement) ? subEl : menuEl;
  const items = [...box.querySelectorAll<HTMLButtonElement>(".row:not(:disabled)")];
  const i = items.indexOf(document.activeElement as HTMLButtonElement);
  const n = items.length;
  if (e.key === "ArrowDown" || e.key === "ArrowUp") {
    items[(i + (e.key === "ArrowDown" ? 1 : n - 1) + n) % n]?.focus();
    e.preventDefault();
  } else if (e.key === "Home" || e.key === "End") {
    items[e.key === "Home" ? 0 : n - 1]?.focus();
    e.preventDefault();
  } else if (e.key === "ArrowRight" && box === menuEl && i >= 0 && items[i].querySelector(".chev")) {
    items[i].click();
    subEl.querySelector<HTMLButtonElement>(".row:not(:disabled)")?.focus();
  } else if (e.key === "ArrowLeft" && box === subEl) {
    const row = subFor;
    closeSub(true);
    row?.focus();
  }
});
