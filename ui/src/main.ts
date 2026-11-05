import { applyTheme, type ThemeState } from "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { getVersion } from "@tauri-apps/api/app";
import { initTraySetting, MOCK_STATUS } from "./tray";

interface Status { version: string; journal_entries: number; windows: boolean }
interface Tweak { id: string; title: string; description: string; settings_uri: string; applied: boolean | null }
interface ApplyReport { applied: string[]; failed: [string, string][] }
interface RestoreReport { restored: number; features?: number; failed: [string, string][] }
interface HotkeyStatus { running: boolean; search: boolean; conflicts: string[] }
interface UpdateInfo { current: string; latest: string; available: boolean; notes: string; page: string }

const inTauri = "__TAURI_INTERNALS__" in window;

// Browser preview: fake backend so the UI can be designed without Windows.
const mock: Record<string, (a?: any) => unknown> = {
  cove_status: () => ({ version: "0.1.0", journal_entries: 0, windows: false }),
  calm_list: () => [
    { id: "calm.taskbar-taskview", title: "Hide the Task View button", description: "Cove's overview replaces it.", settings_uri: "", applied: false },
    { id: "calm.taskbar-search", title: "Hide the taskbar search box", description: "Cove Search takes over (Alt+Space).", settings_uri: "", applied: true },
  ],
  calm_apply: (a) => ({ applied: a.ids, failed: [] }),
  restore_all: () => ({ restored: 0, features: 0, failed: [] }),
  hotkeys_set: (a) => ({ running: a.enabled, search: true, conflicts: [] }),
  search_set: (a) => ({ running: false, search: a.enabled, conflicts: [] }),
  hotkeys_status: () => ({ running: false, search: true, conflicts: [] }),
  open_settings: () => null,
  update_check: () => ({ current: "0.1.2", latest: "0.1.2", available: false, notes: "", page: "" }),
  update_install: () => null,
  just_updated: () => false,
  clipboard_set: (a) => a.enabled,
  topbar_status: () => false,
  topbar_set: (a) => a.enabled,
  dock_status: () => ({ enabled: false, hide_taskbar: false, stacks: ["downloads"], display: "main" }),
  dock_set: (a) => ({ enabled: a.enabled, hide_taskbar: false, stacks: ["downloads"], display: "main" }),
  dock_hide_taskbar_set: (a) => ({ enabled: true, hide_taskbar: a.enabled, stacks: ["downloads"], display: "main" }),
  dock_stacks_set: (a) => ({ enabled: true, hide_taskbar: false, stacks: a.keys, display: "main" }),
  dock_display_set: (a) => ({ enabled: true, hide_taskbar: false, stacks: ["downloads"], display: a.display }),
  dock_hide_set: (a) => ({ enabled: true, hide_taskbar: false, stacks: ["downloads"], display: "main", hide: a.mode }),
  tray_status: () => MOCK_STATUS,
  tray_set: (a) => ({ ...MOCK_STATUS, enabled: a.enabled }),
  clipboard_status: () => true,
  switcher_status: () => false,
  switcher_set: (a) => a.enabled,
  search_info: () => ({ winindex: true, timing: { last: null, median: null, count: 0 } }),
  search_winindex_set: (a) => ({ winindex: a.enabled, timing: { last: null, median: null, count: 0 } }),
  theme_get: () => ({ theme: "tidepool", mode: "system", accent_source: "cove", accent: null }),
  theme_set: (a) => ({ theme: a.theme, mode: a.mode, accent_source: a.accent, accent: a.accent === "windows" ? "#0078d4" : null }),
  wallpaper_status: () => mockWall,
  wallpaper_set: (a) => ({ ...(mockWall = { ...mockWall, current: a.scene as string, has_prev: true }) }),
  wallpaper_restore: () => ({ ...(mockWall = { ...mockWall, current: "", has_prev: false }) }),
  schedule_set: (a) => ({ ...(mockWall = { ...mockWall, schedule: a.schedule as string, light_at: a.light_at as string, dark_at: a.dark_at as string }) }),
  tour_status: () => false,
  tour_done_set: (a) => a.done,
  snap_status: () => ({ enabled: false, animate: true, conflicts: [] }),
  snap_set: (a) => ({ enabled: a.enabled, animate: a.animate, conflicts: [] }),
};

function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  return inTauri ? tauriInvoke<T>(cmd, args) : Promise.resolve(mock[cmd](args) as T);
}

const $ = <T extends HTMLElement>(id: string) => document.getElementById(id) as T;

function toast(text: string): void {
  const t = $("toast");
  t.textContent = text;
  t.classList.remove("hidden");
  clearTimeout((t as any)._timer);
  (t as any)._timer = setTimeout(() => t.classList.add("hidden"), 2600);
}

// ---- scenes: your desktop is the settings ----
const SCENES = ["desktop", "dock", "menubar", "windows", "keys", "quiet"];
// Older names (tray menu, tour) still land on the right scene.
const SCENE_ALIAS: Record<string, string> = { calm: "quiet", habits: "keys", look: "desktop", restore: "quiet", topbar: "menubar" };
// Switching scenes swaps already-rendered content: nothing is cleared and
// re-fetched on the way in (that was the blank flash between scenes). The
// refresh after a switch only repaints what actually changed.
function showScene(name: string, focusHeading = false): void {
  const sc = SCENE_ALIAS[name] ?? name;
  if (name === "updates") return openAbout();
  if (!SCENES.includes(sc)) return;
  const was = $("stage").dataset.scene;
  document.querySelectorAll<HTMLElement>(".scene").forEach((el) => {
    const on = el.id === `scene-${sc}`;
    el.classList.toggle("hidden", !on);
    el.classList.toggle("enter", on && was !== sc);
  });
  document.querySelectorAll<HTMLElement>("#scenes button").forEach((b) => {
    const on = b.dataset.scene === sc;
    b.classList.toggle("on", on);
    if (on) b.setAttribute("aria-current", "page"); else b.removeAttribute("aria-current");
  });
  $("stage").dataset.scene = sc;
  localStorage.setItem("cove.scene", sc);
  if (focusHeading) document.querySelector<HTMLElement>(`#scene-${sc} h1`)?.focus();
  if (sc === "desktop") loadLook();
  if (sc === "dock") renderPins();
}
document.querySelectorAll<HTMLElement>("[data-scene]").forEach((el) =>
  // A keyboard press (detail 0) moves focus to the scene, so Tab continues there.
  el.addEventListener("click", (e) => showScene(el.dataset.scene!, (e as MouseEvent).detail === 0 && el.closest("#scenes") !== null)),
);
document.querySelectorAll<HTMLElement>(".scene").forEach((el) => el.addEventListener("animationend", () => el.classList.remove("enter")));

// ---- status ----
// The version label never shows a placeholder: "About Cove" until the version
// is known (the app's own version answers at once; cove_status confirms it).
if (inTauri) getVersion().then((v) => { $("status").textContent = `Cove ${v}`; }).catch(() => {});
async function refreshStatus(): Promise<void> {
  const s = await invoke<Status>("cove_status");
  $("status").textContent = `Cove ${s.version}${s.windows ? "" : " · preview"}`;
  $("journal-count").textContent =
    s.journal_entries === 0 ? "Nothing to put back." : `${s.journal_entries} change${s.journal_entries === 1 ? "" : "s"} recorded.`;
}

// ---- Quiet (calm tweaks): one switch each, applied at once ----
// Tweaks Windows currently write-protects: shown as "Coming later", never ticked.
const LATER = new Set(["calm.taskbar-widgets"]);
async function renderCalm(): Promise<void> {
  const tweaks = await invoke<Tweak[]>("calm_list");
  const list = $("calm-list");
  list.innerHTML = "";
  if (!tweaks.length) {
    list.innerHTML = `<div class="row"><p class="note">These need Windows 11.</p></div>`;
    return;
  }
  for (const t of tweaks) {
    const row = document.createElement("div");
    row.className = "row";
    row.innerHTML = `<div class="label"><h2></h2><p class="note"></p></div>`;
    row.querySelector("h2")!.textContent = t.title;
    if (LATER.has(t.id)) {
      // Windows locks this setting on current builds, so Cove can't change it
      // yet. Say so plainly: unticked, no switch, a way to do it by hand.
      row.querySelector("p")!.textContent = "Windows locks this one for now. You can turn it off in Taskbar settings.";
      row.insertAdjacentHTML("beforeend", `<span class="later">Coming later</span>`);
      const open = Object.assign(document.createElement("button"), { className: "btn link", textContent: "Taskbar settings" });
      open.addEventListener("click", () => invoke("open_settings", { uri: t.settings_uri || "ms-settings:taskbar" }));
      row.appendChild(open);
      list.appendChild(row);
      continue;
    }
    if (t.applied) {
      // Done: a plain state, not a greyed-out ticked switch.
      row.querySelector("p")!.textContent = "Done. Restore everything puts it back.";
      row.insertAdjacentHTML("beforeend", `<span class="done"><svg viewBox="0 0 12 12" aria-hidden="true"><path d="M2.5 6.2l2.3 2.3 4.7-5"/></svg>Done</span>`);
      list.appendChild(row);
      continue;
    }
    row.querySelector("p")!.textContent = t.description;
    const sw = document.createElement("label");
    sw.className = "switch";
    sw.innerHTML = `<input type="checkbox" role="switch" /><span aria-hidden="true"></span>`;
    const input = sw.querySelector("input")!;
    input.addEventListener("change", async () => {
      input.disabled = true;
      row.classList.add("busy");
      const r = await invoke<ApplyReport>("calm_apply", { ids: [t.id] });
      if (r.failed.length) {
        toast("Windows didn't allow that one. Its Settings page can change it.");
        if (t.settings_uri) invoke("open_settings", { uri: t.settings_uri });
      }
      await Promise.all([renderCalm(), refreshStatus()]);
    });
    row.appendChild(sw);
    list.appendChild(row);
  }
  labelSwitches(list);
}

// ---- habits ----
const SHORTCUTS: [string, string][] = [
  ["Alt+W", "Close window"],
  ["Alt+M", "Minimize window"],
  ["Alt+H", "Hide app"],
  ["Alt+Q", "Quit app"],
  ["Alt+`", "Next window of the same app"],
];

function renderHabits(s: HotkeyStatus): void {
  ($("search-toggle") as HTMLInputElement).checked = s.search;
  $("search-note").textContent = s.search && s.conflicts.includes("Alt+Space")
    ? "Alt+Space is taken by another app (PowerToys Run?)"
    : "Apps, files and settings. Press Alt+Space anywhere.";
  ($("hk-toggle") as HTMLInputElement).checked = s.running;
  $("hk-note").textContent = s.running && s.conflicts.length ? `In use by another app: ${s.conflicts.join(", ")}` : "Close, minimise, hide and quit, Mac style.";
  $("hk-list").classList.toggle("dim", !s.running);
  $("hk-list").innerHTML = SHORTCUTS.map(
    ([k, d]) => `<div class="key"><span>${d}</span>${s.conflicts.includes(k) ? '<em>In use</em>' : ""}${keycaps(k)}</div>`,
  ).join("");
}

$("search-toggle").addEventListener("change", async (e) => {
  const enabled = (e.target as HTMLInputElement).checked;
  renderHabits(await invoke<HotkeyStatus>("search_set", { enabled }));
});

interface SearchInfo { winindex: boolean; timing: { last: number | null; median: number | null; count: number } }
function renderSearchInfo(i: SearchInfo): void {
  ($("wi-toggle") as HTMLInputElement).checked = i.winindex;
  const t = i.timing;
  $("wi-note").textContent = "Uses File Explorer's own index. Read-only." + (t.median != null ? ` Opens in about ${t.median} ms.` : "");
}
$("wi-toggle").addEventListener("change", async (e) => {
  renderSearchInfo(await invoke<SearchInfo>("search_winindex_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<SearchInfo>("search_info").then(renderSearchInfo);
window.addEventListener("focus", () => invoke<SearchInfo>("search_info").then(renderSearchInfo));

// ---- Wallpaper gallery ----
interface WallStatus { scenes: string[]; current: string; has_prev: boolean; schedule: string; light_at: string; dark_at: string; dark_now: boolean }
interface WallItem {
  id: string; category: string; mode: "dark" | "light" | "both"; title: string; author: string;
  license: string; licenseUrl: string; source: string; sourceUrl: string; thumb: string; url?: string; bundled?: string;
}
interface WallManifest { categories: { id: string; name: string; soon?: boolean }[]; items: WallItem[] }
interface WallProgress { id: string; received: number; total: number }
let mockWall: WallStatus = { scenes: ["tidepool", "lagoon", "dune", "slate"], current: "", has_prev: false, schedule: "off", light_at: "07:00", dark_at: "19:00", dark_now: false };
let wall: WallStatus = mockWall;
let gallery: WallManifest | null = null;
let galCat = localStorage.getItem("cove.wall.cat") ?? "all";
let galMode: "dark" | "light" | null = null; // null: follow Cove's appearance
let galCached = new Set<string>();
const galLoading = new Map<string, number>(); // id -> fraction (or -1 when the size is unknown)
const galErrors = new Map<string, string>();
let galBusy = ""; // the id being downloaded

fetch("/wallpapers/manifest.json").then((r) => r.json()).then((m: WallManifest) => { gallery = m; renderWall(wall); }).catch(() => {
  $("wall-note").textContent = "Couldn't load the wallpaper list.";
});
if (inTauri) invoke<string[]>("wallpaper_cached").then((ids) => { galCached = new Set(ids); }).catch(() => {});

function currentId(w: WallStatus): string {
  if (w.current.startsWith("gallery:")) return w.current.slice(8);
  if (gallery) return gallery.items.find((i) => i.bundled && i.bundled === w.current)?.id ?? "";
  return "";
}
function thumbUrl(it: WallItem, mode: "dark" | "light"): string {
  return "/wallpapers/" + it.thumb.replace("{mode}", mode);
}
function fmtMB(n: number): string { return (n / 1048576).toFixed(1); }
const thumbsSeen = new Set<string>();

/** The wallpaper grid is one tab stop; arrows move between tiles. */
function rovingTiles(grid: HTMLElement, focusId?: string): void {
  const tiles = [...grid.querySelectorAll<HTMLElement>(".wt[data-id]")];
  const start = tiles.find((t) => t.dataset.id === focusId) ?? tiles.find((t) => t.classList.contains("on")) ?? tiles[0];
  tiles.forEach((t) => { t.tabIndex = t === start ? 0 : -1; });
  if (focusId && start) start.focus();
}
$("walls").addEventListener("keydown", (e) => {
  const tiles = [...$("walls").querySelectorAll<HTMLElement>(".wt[data-id]")];
  const i = tiles.indexOf(e.target as HTMLElement);
  if (i < 0) return;
  const cols = getComputedStyle($("walls")).gridTemplateColumns.split(" ").length || 3;
  const step: Record<string, number> = { ArrowRight: 1, ArrowLeft: -1, ArrowDown: cols, ArrowUp: -cols };
  let j = i;
  if (e.key in step) j = Math.min(tiles.length - 1, Math.max(0, i + step[e.key]));
  else if (e.key === "Home") j = 0;
  else if (e.key === "End") j = tiles.length - 1;
  else return;
  e.preventDefault();
  tiles[i].tabIndex = -1;
  tiles[j].tabIndex = 0;
  tiles[j].focus();
});

function renderWall(w: WallStatus): void {
  wall = w;
  const mode = galMode ?? ((document.documentElement.dataset.mode === "dark") ? "dark" : "light");
  chipGroup("gal-mode", [["dark", "Dark"], ["light", "Light"]], mode, (k) => { galMode = k as "dark" | "light"; renderWall(wall); });
  const chips = $("gal-chips");
  const chipFocus = chips.contains(document.activeElement) ? document.activeElement?.textContent : null;
  chips.innerHTML = "";
  const cats = [{ id: "all", name: "All" }, ...(gallery?.categories ?? [])];
  for (const c of cats) {
    const b = document.createElement("button");
    b.className = `chip${galCat === c.id ? " on" : ""}`;
    b.setAttribute("role", "radio");
    b.setAttribute("aria-checked", String(galCat === c.id));
    b.tabIndex = galCat === c.id ? 0 : -1;
    b.textContent = c.name;
    if ("soon" in c && c.soon) {
      b.disabled = true;
      b.title = "Live wallpapers are in beta and coming soon";
      b.append(Object.assign(document.createElement("sup"), { textContent: "beta \u2013 coming soon" }));
    } else {
      b.addEventListener("click", () => { galCat = c.id; localStorage.setItem("cove.wall.cat", c.id); renderWall(wall); });
    }
    chips.appendChild(b);
    if (chipFocus && b.textContent === chipFocus) b.focus();
  }
  const grid = $("walls");
  // Rebuild the grid only when what it shows changed. Re-rendering identical
  // tiles reloaded every thumbnail (the flash when coming back to Desktop).
  const gridKey = JSON.stringify([!!gallery, galCat, mode, currentId(w), [...galLoading.keys()], [...galErrors]]);
  if (grid.dataset.key !== gridKey) {
  const focusId = (document.activeElement as HTMLElement | null)?.closest?.<HTMLElement>(".wt")?.dataset.id;
  grid.dataset.key = gridKey;
  grid.innerHTML = "";
  if (!gallery) {
    for (let i = 0; i < 6; i++) grid.insertAdjacentHTML("beforeend", '<div class="wt skel" aria-hidden="true"><div class="im"></div><span class="wcap"><b>&nbsp;</b></span></div>');
  } else {
    const on = currentId(w);
    const list = gallery.items.filter((it) => (galCat === "all" || it.category === galCat) && (it.mode === "both" || it.mode === mode));
    if (!list.length) grid.innerHTML = '<p class="gal-empty">Nothing here in this mode yet.</p>';
    const catName = new Map(gallery.categories.map((c) => [c.id, c.name]));
    for (const it of list) grid.appendChild(wallTile(it, mode, it.id === on, catName.get(it.category) ?? ""));
    rovingTiles(grid, focusId);
  }
  }
  $("wall-restore").classList.toggle("hidden", !w.has_prev);
  chipGroup("sched-mode", [["off", "Off"], ["times", "On"]], w.schedule, (k) => saveSched(k));
  ($("sched-light") as HTMLInputElement).value = w.light_at;
  ($("sched-dark") as HTMLInputElement).value = w.dark_at;
  timeSync.forEach((f) => f());
  $("sched-times").classList.toggle("off", w.schedule !== "times");
}

function wallTile(it: WallItem, mode: "dark" | "light", on: boolean, cat: string): HTMLElement {
  const b = document.createElement("div");
  b.className = `wt${on ? " on" : ""}`;
  b.tabIndex = -1;
  b.setAttribute("role", "option");
  b.setAttribute("aria-selected", String(on));
  b.dataset.id = it.id;
  b.setAttribute("aria-label", `${it.title}, ${it.author === "Cove" ? "Cove original art" : `${it.author}, ${it.license}`}`);
  const im = document.createElement("div");
  im.className = "im";
  const img = new Image();
  img.alt = "";
  img.decoding = "async";
  img.onload = () => { img.classList.add("ready"); thumbsSeen.add(img.src); };
  img.src = thumbUrl(it, mode);
  if (img.complete || thumbsSeen.has(img.src)) img.classList.add("ready", "now"); // cached: no fade
  im.appendChild(img);
  if (on) im.insertAdjacentHTML("beforeend", '<span class="tick"><svg viewBox="0 0 12 12"><path d="M2.5 6.2l2.3 2.3 4.7-5"/></svg></span>');
  if (it.mode === "both") im.insertAdjacentHTML("beforeend", '<span class="pair" title="Has a light and a dark version and follows Cove\'s appearance"></span>');
  // The credit, on hover and keyboard focus.
  const cr = document.createElement("div");
  cr.className = "credit";
  cr.append(Object.assign(document.createElement("b"), { textContent: it.title }));
  if (it.author === "Cove") {
    cr.append(Object.assign(document.createElement("em"), { textContent: it.bundled ? "Cove original art, ships with Cove" : "Cove original art" }));
  } else {
    cr.append(Object.assign(document.createElement("span"), { textContent: `${it.author} \u00b7 ${it.license}` }));
    const a = Object.assign(document.createElement("a"), { textContent: `${it.source} \u2197`, href: it.sourceUrl, title: it.sourceUrl, tabIndex: -1 });
    a.addEventListener("click", (e) => {
      e.preventDefault(); e.stopPropagation();
      if (inTauri) invoke("wallpaper_open_source", { id: it.id }).catch(() => {});
      else window.open(it.sourceUrl, "_blank", "noopener");
    });
    cr.append(a);
  }
  im.appendChild(cr);
  const frac = galLoading.get(it.id);
  if (frac !== undefined) im.appendChild(loadOverlay(frac));
  const err = galErrors.get(it.id);
  if (err) {
    const e = document.createElement("div");
    e.className = "err";
    e.append(Object.assign(document.createElement("b"), { textContent: "Couldn't set this one" }), Object.assign(document.createElement("span"), { textContent: `${err} Click to try again.` }));
    im.appendChild(e);
  }
  const cap = document.createElement("span");
  cap.className = "wcap";
  cap.append(Object.assign(document.createElement("b"), { textContent: it.title }), Object.assign(document.createElement("i"), { textContent: cat }));
  b.append(im, cap);
  const pick = () => pickWall(it);
  b.addEventListener("click", pick);
  b.addEventListener("keydown", (e) => { if (e.key === "Enter" || e.key === " ") { e.preventDefault(); pick(); } });
  return b;
}

function loadOverlay(frac: number, text?: string): HTMLElement {
  const l = document.createElement("div");
  l.className = `load${frac < 0 ? " indet" : ""}`;
  l.innerHTML = '<span class="pct"></span><div class="bar"><i></i></div>';
  (l.querySelector(".pct") as HTMLElement).textContent = text ?? (frac < 0 ? "Downloading\u2026" : `${Math.round(frac * 100)}%`);
  if (frac >= 0) (l.querySelector(".bar i") as HTMLElement).style.width = `${Math.round(frac * 100)}%`;
  return l;
}

function showProgress(p: WallProgress): void {
  if (!galLoading.has(p.id)) return;
  const frac = p.total > 0 ? Math.min(1, p.received / p.total) : -1;
  galLoading.set(p.id, frac);
  const tile = document.querySelector<HTMLElement>(`.wt[data-id="${p.id}"] .im`);
  if (!tile) return;
  tile.querySelector(".load")?.remove();
  const text = frac < 0 ? `${fmtMB(p.received)} MB` : `${Math.round(frac * 100)}%  \u00b7  ${fmtMB(p.received)} of ${fmtMB(p.total)} MB`;
  tile.appendChild(loadOverlay(frac, text));
}

async function pickWall(it: WallItem): Promise<void> {
  if (galBusy && galBusy !== it.id) galLoading.delete(galBusy); // the newer pick wins
  galErrors.delete(it.id);
  const needsDownload = !it.bundled && !galCached.has(it.id);
  galBusy = it.id;
  if (needsDownload) { galLoading.set(it.id, -1); renderWall(wall); }
  try {
    const st = inTauri ? await invoke<WallStatus>("wallpaper_pick", { id: it.id }) : await mockPick(it, needsDownload);
    if (!it.bundled) galCached.add(it.id);
    if (galBusy === it.id) galBusy = "";
    if (!inTauri) $("mini-wall").style.backgroundImage = `url("${thumbUrl(it, galMode ?? "dark")}")`; // preview only
    galLoading.delete(it.id);
    renderWall(st);
  } catch (e) {
    galLoading.delete(it.id);
    if (galBusy === it.id) galBusy = "";
    if (String(e) !== "cancelled") galErrors.set(it.id, String(e).replace(/\.?$/, "."));
    renderWall(wall);
  }
}

// Browser preview: pretend to download so the loading state can be designed.
function mockPick(it: WallItem, slow: boolean): Promise<WallStatus> {
  return new Promise((done) => {
    const total = 4.2 * 1048576;
    let got = 0;
    const step = () => {
      if (!slow || got >= total) {
        mockWall = { ...mockWall, current: it.bundled ? it.bundled : `gallery:${it.id}`, has_prev: true };
        return done({ ...mockWall });
      }
      got = Math.min(total, got + total / 14);
      showProgress({ id: it.id, received: got, total });
      setTimeout(step, 120);
    };
    step();
  });
}

$("wall-own").addEventListener("click", () => ($("wall-own-file") as HTMLInputElement).click());
$("wall-own-file").addEventListener("change", async (e) => {
  const input = e.target as HTMLInputElement;
  const f = input.files?.[0];
  input.value = "";
  if (!f) return;
  $("wall-note").textContent = `Setting ${f.name}\u2026`;
  try {
    const buf = new Uint8Array(await f.arrayBuffer());
    let bin = "";
    for (let i = 0; i < buf.length; i += 0x8000) bin += String.fromCharCode(...buf.subarray(i, i + 0x8000));
    const st = inTauri ? await invoke<WallStatus>("wallpaper_own", { data: btoa(bin) }) : { ...(mockWall = { ...mockWall, current: "own:mock", has_prev: true }) };
    $("wall-note").textContent = `Your picture is on the desktop.`;
    renderWall(st);
  } catch (err) {
    $("wall-note").textContent = String(err);
  }
});
if (inTauri) listen<WallProgress>("wallpaper-progress", (e) => showProgress(e.payload));

// ---- Time fields (light/dark schedule) ----
// Cove's own control instead of the native <input type="time">, whose parts
// can't be styled to Ink and graphite. Hour, minute (and AM/PM where the
// locale uses it) are spinbuttons: arrows step, digits type, Left/Right move,
// and the small stepper does the same for the mouse. The value lives in a
// hidden input that fires "change" once you stop adjusting.
const timeSync: (() => void)[] = [];
const hour12 = new Intl.DateTimeFormat(undefined, { hour: "numeric" }).resolvedOptions().hour12 ?? false;
const pad2 = (n: number) => String(n).padStart(2, "0");
function timeField(hostId: string, inputId: string): void {
  const host = $(hostId);
  const input = $(inputId) as HTMLInputElement;
  host.setAttribute("role", "group");
  host.setAttribute("aria-labelledby", `${hostId}-l`);
  const seg = (label: string) => {
    const s = document.createElement("span");
    s.className = "ts";
    s.tabIndex = 0;
    s.setAttribute("role", "spinbutton");
    s.setAttribute("aria-label", label);
    return s;
  };
  const h = seg("Hour"), m = seg("Minutes"), ap = hour12 ? seg("AM or PM") : null;
  const segs = [h, m, ...(ap ? [ap] : [])];
  const colon = Object.assign(document.createElement("span"), { className: "tc", textContent: ":" });
  colon.setAttribute("aria-hidden", "true");
  const stepper = document.createElement("span");
  stepper.className = "tstep";
  stepper.setAttribute("aria-hidden", "true");
  stepper.innerHTML = '<button tabindex="-1" data-d="1"><svg viewBox="0 0 10 6"><path d="M1.5 4.5L5 1.5l3.5 3"/></svg></button><button tabindex="-1" data-d="-1"><svg viewBox="0 0 10 6"><path d="M1.5 1.5L5 4.5l3.5-3"/></svg></button>';
  host.append(h, colon, m, ...(ap ? [ap] : []), stepper);
  let last: HTMLElement = h;
  let typed = "";
  let timer = 0;
  const get = () => { const [H, M] = (input.value || "00:00").split(":").map(Number); return { H: H || 0, M: M || 0 }; };
  const paint = () => {
    const { H, M } = get();
    h.textContent = hour12 ? String(H % 12 || 12) : pad2(H);
    h.setAttribute("aria-valuemin", hour12 ? "1" : "0");
    h.setAttribute("aria-valuemax", hour12 ? "12" : "23");
    h.setAttribute("aria-valuenow", hour12 ? String(H % 12 || 12) : String(H));
    m.textContent = pad2(M);
    m.setAttribute("aria-valuemin", "0");
    m.setAttribute("aria-valuemax", "59");
    m.setAttribute("aria-valuenow", String(M));
    if (ap) { ap.textContent = H < 12 ? "AM" : "PM"; ap.setAttribute("aria-valuetext", ap.textContent); }
  };
  const flush = () => { if (timer) { clearTimeout(timer); timer = 0; input.dispatchEvent(new Event("change")); } };
  const set = (H: number, M: number) => {
    input.value = `${pad2(((H % 24) + 24) % 24)}:${pad2(((M % 60) + 60) % 60)}`;
    paint();
    clearTimeout(timer);
    timer = window.setTimeout(flush, 700);
  };
  const step = (el: HTMLElement, d: number) => {
    const { H, M } = get();
    if (el === h) set(H + d, M);
    else if (el === m) set(H, M + d);
    else set(H + 12, M);
  };
  const typeDigit = (el: HTMLElement, digit: string) => {
    const { H, M } = get();
    typed = (typed + digit).slice(-2);
    const n = Number(typed);
    if (el === h) {
      if (hour12) { const v = Math.min(12, Math.max(1, n || 12)); set((v % 12) + (H >= 12 ? 12 : 0), M); }
      else set(Math.min(23, n), M);
      if (typed.length === 2 || n * 10 > (hour12 ? 12 : 23)) { typed = ""; m.focus(); }
    } else if (el === m) {
      set(H, Math.min(59, n));
      if (typed.length === 2 || n * 10 > 59) { typed = ""; (ap ?? m).focus(); }
    }
  };
  segs.forEach((el, i) => {
    el.addEventListener("focus", () => { last = el; typed = ""; });
    el.addEventListener("keydown", (e) => {
      const k = e.key;
      if (k === "ArrowUp" || k === "ArrowDown") step(el, k === "ArrowUp" ? 1 : -1);
      else if (k === "PageUp" || k === "PageDown") step(el, (k === "PageUp" ? 1 : -1) * (el === m ? 10 : 1));
      else if (k === "ArrowRight" && i < segs.length - 1) segs[i + 1].focus();
      else if (k === "ArrowLeft" && i > 0) segs[i - 1].focus();
      else if (/^[0-9]$/.test(k) && el !== ap) typeDigit(el, k);
      else if (el === ap && /^[ap]$/i.test(k)) { const { H, M } = get(); if ((k.toLowerCase() === "p") !== (H >= 12)) set(H + 12, M); }
      else if (k === "Enter") flush();
      else return;
      e.preventDefault();
    });
  });
  stepper.addEventListener("pointerdown", (e) => e.preventDefault()); // keep focus where it is
  stepper.querySelectorAll<HTMLButtonElement>("button").forEach((b) =>
    b.addEventListener("click", () => { step(last, Number(b.dataset.d)); last.focus(); }));
  host.addEventListener("focusout", (e) => { if (!host.contains(e.relatedTarget as Node)) flush(); });
  timeSync.push(paint);
  paint();
}
timeField("tf-light", "sched-light");
timeField("tf-dark", "sched-dark");

// ---- Look (Cove Glass) ----
async function saveSched(mode?: string): Promise<void> {
  try {
    renderWall(await invoke<WallStatus>("schedule_set", {
      schedule: mode ?? wall.schedule,
      light_at: ($("sched-light") as HTMLInputElement).value || "07:00",
      dark_at: ($("sched-dark") as HTMLInputElement).value || "19:00",
    }));
    invoke<ThemeState>("theme_get").then(renderLook);
  } catch (e) { $("sched-note").textContent = String(e); }
}
$("sched-light").addEventListener("change", () => saveSched());
$("sched-dark").addEventListener("change", () => saveSched());
$("wall-restore").addEventListener("click", async () => renderWall(await invoke<WallStatus>("wallpaper_restore")));
if (inTauri) {
  listen("wallpaper-changed", () => {
    invoke<WallStatus>("wallpaper_status").then(renderWall);
    invoke<ThemeState>("theme_get").then(renderLook);
  });
}

let look: ThemeState = { theme: "tidepool", mode: "system", accent_source: "cove", accent: null };
const LOOK_NOTES: Record<string, string> = {
  tidepool: "Clear, lets your wallpaper's colour through.",
  slate: "Mostly solid graphite. Quieter.",
};
/** A segmented control. Single choice: a radio group (one tab stop, arrows
 *  move and pick). Multiple choice (`on` is a list): toggle buttons. Focus
 *  stays on the same option when the group re-renders. */
function chipGroup(id: string, opts: [string, string][], on: string | string[], pick: (k: string) => void): void {
  const box = $(id);
  const multi = Array.isArray(on);
  const had = box.contains(document.activeElement) ? (document.activeElement as HTMLElement).dataset.k : undefined;
  box.innerHTML = "";
  box.setAttribute("role", multi ? "group" : "radiogroup");
  if (!box.hasAttribute("aria-label") && !box.hasAttribute("aria-labelledby")) {
    const h = box.closest(".row, .gal-head")?.querySelector("h2");
    if (h) { h.id ||= `${id}-h`; box.setAttribute("aria-labelledby", h.id); }
  }
  const isOn = (k: string) => (multi ? (on as string[]).includes(k) : on === k);
  const anyOn = opts.some(([k]) => isOn(k));
  opts.forEach(([k, label], i) => {
    const b = document.createElement("button");
    b.className = `chip${isOn(k) ? " on" : ""}`;
    b.textContent = label;
    b.dataset.k = k;
    if (multi) b.setAttribute("aria-pressed", String(isOn(k)));
    else {
      b.setAttribute("role", "radio");
      b.setAttribute("aria-checked", String(isOn(k)));
      b.tabIndex = isOn(k) || (!anyOn && i === 0) ? 0 : -1;
    }
    b.addEventListener("click", () => pick(k));
    box.appendChild(b);
  });
  if (had) box.querySelector<HTMLElement>(`[data-k="${had}"]`)?.focus();
}
// Arrow keys inside any radio group (chips and the wallpaper categories).
document.addEventListener("keydown", (e) => {
  const t = e.target as HTMLElement;
  const group = t.closest<HTMLElement>('[role="radiogroup"]');
  if (!group || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown", "Home", "End"].includes(e.key)) return;
  const items = [...group.querySelectorAll<HTMLButtonElement>('[role="radio"]:not(:disabled)')];
  const i = items.indexOf(t as HTMLButtonElement);
  if (i < 0) return;
  e.preventDefault();
  const n = items.length;
  const j = e.key === "Home" ? 0 : e.key === "End" ? n - 1 : (i + (e.key === "ArrowRight" || e.key === "ArrowDown" ? 1 : n - 1)) % n;
  items[j].focus();
  items[j].click();
});

/** Give each switch the name of its row (its h2) and the row's note. */
let swSeq = 0;
function labelSwitches(root: ParentNode = document): void {
  root.querySelectorAll<HTMLInputElement>(".row .switch input").forEach((input) => {
    const row = input.closest(".row")!;
    const h = row.querySelector("h2");
    const note = row.querySelector(".note");
    const base = input.id || `sw${++swSeq}`;
    if (h) { h.id ||= `${base}-h`; input.setAttribute("aria-labelledby", h.id); }
    if (note) { note.id ||= `${base}-n`; input.setAttribute("aria-describedby", note.id); }
  });
}
labelSwitches();
async function setLook(p: Partial<{ theme: string; mode: string; accent: string }>): Promise<void> {
  const args = { theme: look.theme, mode: look.mode, accent: look.accent_source, ...p };
  renderLook(await invoke<ThemeState>("theme_set", args));
}
function renderLook(t: ThemeState): void {
  look = t;
  applyTheme(t);
  if (wall) renderWall(wall);
  chipGroup("look-theme", [["tidepool", "Clear"], ["slate", "Solid"]], t.theme, (k) => setLook({ theme: k }));
  chipGroup("look-mode", [["system", "Auto"], ["light", "Light"], ["dark", "Dark"]], t.mode, (k) => setLook({ mode: k }));
  chipGroup("look-accent", [["cove", "Graphite"], ["windows", "Windows'"]], t.accent_source, (k) => setLook({ accent: k }));
  $("look-theme-note").textContent = LOOK_NOTES[t.theme] ?? "";
  $("look-accent-note").textContent = t.accent_source === "windows" && !t.accent ? "Couldn't read Windows' accent, so Graphite is used." : "";
}
function loadLook(): void {
  invoke<ThemeState>("theme_get").then(renderLook).catch(() => renderLook(look));
  invoke<WallStatus>("wallpaper_status").then(renderWall).catch(() => renderWall(wall));
}
renderLook(look); // never show an empty Look page, even before the backend answers
loadLook();
window.addEventListener("focus", () => { if (look.accent_source === "windows") invoke<ThemeState>("theme_get").then(renderLook); });

interface DockStatus { enabled: boolean; hide_taskbar: boolean; stacks: string[]; display: string; hide?: string }
const STACKS: [string, string][] = [["downloads", "Downloads"], ["documents", "Documents"], ["desktop", "Desktop"], ["pictures", "Pictures"]];
let dockStacks: string[] = [];
function renderDock(d: DockStatus): void {
  dockStacks = d.stacks;
  chipGroup("dock-display", [["main", "Main screen"], ["follow", "Follows the pointer"]], d.display,
    async (k) => renderDock(await invoke<DockStatus>("dock_display_set", { display: k })));
  chipGroup("dock-hide", [["smart", "When a window touches it"], ["always", "Always"], ["never", "Never"]], d.hide ?? "smart",
    async (k) => renderDock(await invoke<DockStatus>("dock_hide_set", { mode: k })));
  chipGroup("dock-stacks", STACKS, d.stacks, async (k) => {
    const keys = dockStacks.includes(k) ? dockStacks.filter((x) => x !== k) : [...dockStacks, k];
    renderDock(await invoke<DockStatus>("dock_stacks_set", { keys }));
  });
  ($("dock-toggle") as HTMLInputElement).checked = d.enabled;
  ($("dock-taskbar") as HTMLInputElement).checked = d.hide_taskbar;
  ($("dock-taskbar") as HTMLInputElement).disabled = !d.enabled;
  $("dock-note").textContent = d.enabled ? "" : "The Windows taskbar is in charge.";
  ($("desk-toggle") as HTMLInputElement).checked = d.enabled;
  renderPins();
}
$("dock-toggle").addEventListener("change", async (e) => {
  renderDock(await invoke<DockStatus>("dock_set", { enabled: (e.target as HTMLInputElement).checked }));
});
$("dock-taskbar").addEventListener("change", async (e) => {
  renderDock(await invoke<DockStatus>("dock_hide_taskbar_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<DockStatus>("dock_status").then(renderDock).then(() => initTraySetting(invoke));

function renderSwitcher(on: boolean): void {
  ($("sw-toggle") as HTMLInputElement).checked = on;
  $("sw-note").textContent = on ? "Alt+Tab shows one icon per app. ↓ shows that app's windows." : "Alt+Tab by app instead of by window.";
}
$("sw-toggle").addEventListener("change", async (e) => {
  renderSwitcher(await invoke<boolean>("switcher_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<boolean>("switcher_status").then(renderSwitcher);

interface SnapInfo { enabled: boolean; animate: boolean; conflicts: string[] }
const SNAPS: [string, string][] = [
  ["Ctrl+Alt+← / →", "Left or right half · press again for ⅔, then ⅓"],
  ["Ctrl+Alt+↑ / ↓", "Top or bottom half · press again for ⅔, then ⅓"],
  ["Ctrl+Alt+U / I / J / K", "Top-left, top-right, bottom-left, bottom-right quarter"],
  ["Ctrl+Alt+D / F / G", "Left, centre, right third"],
  ["Ctrl+Alt+E / T", "Left or right two-thirds"],
  ["Ctrl+Alt+Enter", "Fill the screen"],
  ["Ctrl+Alt+C", "Centre, same size"],
  ["Ctrl+Alt+Backspace", "Back to where it was before snapping"],
  ["Ctrl+Alt+Shift+← / →", "Move to the previous or next display"],
];
let snap: SnapInfo = { enabled: false, animate: true, conflicts: [] };
function renderSnap(s: SnapInfo): void {
  snap = s;
  ($("snap-toggle") as HTMLInputElement).checked = s.enabled;
  ($("snap-animate") as HTMLInputElement).checked = s.animate;
  $("snap-anim-row").classList.toggle("dim", !s.enabled);
  $("snap-note").textContent = s.enabled && s.conflicts.length
    ? `${s.conflicts.length} shortcut${s.conflicts.length > 1 ? "s are" : " is"} in use by another app.`
    : "Halves, thirds, quarters, centre and fill, from the keyboard.";
  $("snap-list").classList.toggle("dim", !s.enabled);
  $("snap-list").innerHTML = SNAPS.map(
    ([k, d]) => {
      const prefix = k.slice(0, k.lastIndexOf("+") + 1);
      const taken = k.slice(prefix.length).split(" / ").some((p) => s.conflicts.includes(prefix + p));
      return `<div class="key"><span>${d}</span>${taken ? "<em>In use</em>" : ""}${keycaps(k)}</div>`;
    },
  ).join("");
}
$("snap-toggle").addEventListener("change", async (e) => {
  renderSnap(await invoke<SnapInfo>("snap_set", { enabled: (e.target as HTMLInputElement).checked, animate: snap.animate }));
});
$("snap-animate").addEventListener("change", async (e) => {
  renderSnap(await invoke<SnapInfo>("snap_set", { enabled: snap.enabled, animate: (e.target as HTMLInputElement).checked }));
});
invoke<SnapInfo>("snap_status").then(renderSnap);

function renderTopbar(on: boolean): void {
  ($("topbar-toggle") as HTMLInputElement).checked = on;
  $("topbar-note").textContent = on ? "" : "Windows' clock and icons stay on the taskbar.";
}
$("topbar-toggle").addEventListener("change", async (e) => {
  renderTopbar(await invoke<boolean>("topbar_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<boolean>("topbar_status").then(renderTopbar);

function renderClip(on: boolean): void {
  ($("clip-toggle") as HTMLInputElement).checked = on;
  $("clip-note").textContent = "Text you copy, in Search. Kept in memory only; skips password managers.";
}
$("clip-toggle").addEventListener("change", async (e) => {
  renderClip(await invoke<boolean>("clipboard_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<boolean>("clipboard_status").then(renderClip);

$("hk-toggle").addEventListener("change", async (e) => {
  const enabled = (e.target as HTMLInputElement).checked;
  renderHabits(await invoke<HotkeyStatus>("hotkeys_set", { enabled }));
});

// ---- restore ----
$("restore-btn").addEventListener("click", async () => {
  const r = await invoke<RestoreReport>("restore_all");
  const f = r.features ?? 0;
  const parts = [
    r.restored ? `Restored ${r.restored} Windows setting${r.restored === 1 ? "" : "s"}` : "",
    f ? `turned off ${f} Cove feature${f === 1 ? "" : "s"}` : "",
  ].filter(Boolean);
  const msg = parts.length ? parts.join(" and ").replace(/^t/, "T") + "." : "Already stock.";
  toast(r.failed.length ? `${msg} ${r.failed.length} couldn't be restored.` : msg);
  await Promise.all([renderCalm(), refreshStatus()]);
});

// ---- updates ----
async function checkUpdates(quiet = false): Promise<void> {
  const note = $("up-note");
  const btn = $("up-install");
  if (!quiet) note.textContent = "Checking…";
  try {
    const u = await invoke<UpdateInfo>("update_check");
    $("up-title").textContent = u.available ? `Cove ${u.latest}` : `Cove ${u.current}`;
    btn.classList.toggle("hidden", !u.available);
    $("update-dot").classList.toggle("hidden", !u.available);
    const notes = $("up-notes");
    renderNotes(notes, u.notes);
    notes.classList.toggle("hidden", !u.available || !u.notes);
    note.textContent = u.available ? `Ready to install. You have ${u.current}.` : "You're on the latest version.";
    btn.textContent = `Update to ${u.latest}`;
  } catch (e) {
    note.textContent = String(e);
  }
}

/** Release notes are a little markdown: headings, bullets, **bold** and `code`.
 *  Built with DOM nodes only, never innerHTML. */
function renderNotes(box: HTMLElement, md: string): void {
  box.innerHTML = "";
  let ul: HTMLUListElement | null = null;
  const inline = (el: HTMLElement, text: string) => {
    for (const part of text.split(/(\*\*[^*]+\*\*|`[^`]+`)/)) {
      if (!part) continue;
      if (part.startsWith("**")) el.appendChild(Object.assign(document.createElement("strong"), { textContent: part.slice(2, -2) }));
      else if (part.startsWith("`")) el.appendChild(Object.assign(document.createElement("code"), { textContent: part.slice(1, -1) }));
      else el.appendChild(document.createTextNode(part.replace(/\*/g, "")));
    }
  };
  for (const raw of md.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) { ul = null; continue; }
    if (/^[-*] /.test(line)) {
      if (!ul) ul = box.appendChild(document.createElement("ul"));
      inline(ul.appendChild(document.createElement("li")), line.slice(2));
      continue;
    }
    ul = null;
    const h = line.match(/^#+\s*(.*)/);
    const el = document.createElement(h ? "h3" : "p");
    inline(el, h ? h[1] : line);
    box.appendChild(el);
  }
}

$("up-check").addEventListener("click", () => checkUpdates());
if (inTauri) {
  listen<[number, number]>("update-progress", (e) => {
    const [done, total] = e.payload;
    const pct = total ? Math.round((done / total) * 100) : 0;
    ($("up-bar") as HTMLElement).style.width = `${pct}%`;
    $("up-progress").setAttribute("aria-valuenow", String(pct));
    $("up-install").textContent = `Downloading… ${pct}%`;
  });
  listen("update-installing", () => {
    $("up-install").textContent = "Installing…";
    $("up-note").textContent = "Cove will close for a few seconds and reopen on its own.";
  });
}

$("up-install").addEventListener("click", async () => {
  const btn = $("up-install") as HTMLButtonElement;
  btn.disabled = true;
  btn.textContent = "Downloading…";
  $("up-progress").classList.remove("hidden");
  $("up-note").textContent = "Downloading and checking the update.";
  try {
    await invoke("update_install");
  } catch (e) {
    $("up-note").textContent = String(e);
    btn.disabled = false;
    btn.textContent = "Try again";
    $("up-progress").classList.add("hidden");
  }
});

// ---- boot ----
(async () => {
  // The window starts hidden; the app shows it (after its own intro window
  // on a first run, or when you open Settings), so the page just loads.
  await Promise.all([refreshStatus(), renderCalm(), invoke<HotkeyStatus>("hotkeys_status").then(renderHabits)]).catch(() => {});
  if (!(await invoke<boolean>("tour_status").catch(() => true))) tourOpen();
  checkUpdates(true);
  if (await invoke<boolean>("just_updated")) toast(`Updated to Cove ${await currentVersion()} ✓`);
})();

async function currentVersion(): Promise<string> {
  return (await invoke<UpdateInfo>("update_check").catch(() => null))?.current ?? "";
}

// ---- Cove Tour: first run, replayable from the sidebar ----
interface TourStep {
  id: string;
  title: string;
  body: string;
  keys?: string[];
  signal?: string; // backend "tour-step" that completes the step
  on?: () => Promise<boolean>; // is the feature already on?
  enable?: () => Promise<unknown>;
  extra?: () => HTMLElement[];
}
type Preset = "light" | "mac" | "none";
let tourPreset: Preset = "mac";
const PRESETS: [Preset, string, string][] = [
  ["mac", "Mac-like", "Calm taskbar, Search, Mac shortcuts, dock, top bar, app switcher and window snapping"],
  ["light", "Light touch", "Calm taskbar and Search (Alt+Space). Everything else stays Windows"],
  ["none", "Keep things as they are", "Change nothing now. You can switch any feature on later"],
];
async function applyPreset(p: Preset): Promise<void> {
  if (p === "none") return;
  const tweaks = await invoke<Tweak[]>("calm_list");
  const ids = tweaks.filter((t) => t.applied === false && !LATER.has(t.id)).map((t) => t.id);
  if (ids.length) await invoke("calm_apply", { ids });
  await invoke("search_set", { enabled: true });
  if (p === "mac") {
    await invoke("hotkeys_set", { enabled: true });
    await invoke("dock_set", { enabled: true });
    await invoke("topbar_set", { enabled: true });
    await invoke("switcher_set", { enabled: true });
    await invoke("snap_set", { enabled: true, animate: true });
  }
}
const kbd = (k: string[]) => k;
const TOUR: TourStep[] = [
  {
    id: "hello", title: "Welcome to Cove",
    body: "Cove makes Windows calmer and adds a few Mac habits. Everything is reversible from Restore. Pick a starting point; you can change any of it later.",
    extra: () => PRESETS.map(([k, name, desc]) => {
      const b = document.createElement("button");
      b.className = `tour-preset${tourPreset === k ? " on" : ""}`;
      b.setAttribute("aria-pressed", String(tourPreset === k));
      b.innerHTML = `<b></b><span></span>`;
      (b.querySelector("b") as HTMLElement).textContent = name;
      (b.querySelector("span") as HTMLElement).textContent = desc;
      b.addEventListener("click", async () => {
        tourPreset = k;
        await tourShow();
        $("tour-extra").querySelectorAll<HTMLElement>(".tour-preset")[PRESETS.findIndex((p) => p[0] === k)]?.focus();
      });
      return b;
    }),
  },
  {
    id: "search", title: "Search anything", keys: kbd(["Alt", "Space"]), signal: "search",
    body: "Press Alt+Space from anywhere, type a few letters of an app, file or setting, then Enter. Esc closes it.",
    on: async () => (await invoke<HotkeyStatus>("hotkeys_status")).search,
    enable: () => invoke("search_set", { enabled: true }),
  },
  {
    id: "habits", title: "Mac shortcuts", keys: kbd(["Alt", "M"]), signal: "habits",
    body: "Click any other window, then press Alt+M to minimise it. Alt+W closes a window, Alt+H hides an app, Alt+Q quits it, Alt+` cycles its windows.",
    on: async () => (await invoke<HotkeyStatus>("hotkeys_status")).running,
    enable: () => invoke("hotkeys_set", { enabled: true }),
  },
  {
    id: "snap", title: "Snap windows", keys: kbd(["Ctrl", "Alt", "←"]), signal: "snap",
    body: "Click a window and press Ctrl+Alt+← to glide it to the left half. Press it again for 2/3 and 1/3. Ctrl+Alt+Enter fills the screen, Ctrl+Alt+Backspace puts it back.",
    on: async () => (await invoke<SnapInfo>("snap_status")).enabled,
    enable: () => invoke("snap_set", { enabled: true, animate: true }),
  },
  {
    id: "switcher", title: "Switch apps", keys: kbd(["Alt", "Tab"]), signal: "switcher",
    body: "Hold Alt and tap Tab to move between apps, grouped by app like a Mac. Let go of Alt to switch.",
    on: () => invoke<boolean>("switcher_status"),
    enable: () => invoke("switcher_set", { enabled: true }),
  },
  {
    id: "expose", title: "See an app's windows", keys: kbd(["Alt", "Tab", "↓"]), signal: "expose",
    body: "In the switcher, press ↓ to see every window of that app side by side. You can also right-click a running app in the dock and choose Show All Windows.",
    on: () => invoke<boolean>("switcher_status"),
    enable: () => invoke("switcher_set", { enabled: true }),
  },
  {
    id: "dock", title: "Dock and top bar",
    body: "The dock sits at the bottom with your apps and folders; the top bar shows the app in front, the clock and Control Centre. Change them in Dock & top bar, and the look in Look.",
    on: async () => (await invoke<DockStatus>("dock_status")).enabled && (await invoke<boolean>("topbar_status")),
    enable: async () => { await invoke("dock_set", { enabled: true }); await invoke("topbar_set", { enabled: true }); },
  },
  {
    id: "end", title: "You're set",
    body: "That's Cove. Replay this tour any time from Take the tour in the sidebar, and undo everything from Restore.",
  },
];
let tourIdx = 0;
let tourDone = new Set<string>();
let tourChanged = false;
async function tourShow(): Promise<void> {
  const st = TOUR[tourIdx];
  $("tour-dots").innerHTML = "";
  TOUR.forEach((t, i) => {
    const d = document.createElement("i");
    if (i === tourIdx) d.className = "on";
    else if (tourDone.has(t.id) || i < tourIdx) d.className = "done";
    $("tour-dots").appendChild(d);
  });
  $("tour-dots").setAttribute("aria-label", `Step ${tourIdx + 1} of ${TOUR.length}`);
  $("tour-keys").innerHTML = "";
  for (const k of st.keys ?? []) {
    const e = document.createElement("kbd");
    e.textContent = k;
    $("tour-keys").appendChild(e);
  }
  $("tour-title").textContent = st.title;
  $("tour-body").textContent = st.body;
  $("tour-extra").innerHTML = "";
  for (const e of st.extra?.() ?? []) $("tour-extra").appendChild(e);
  $("tour-check").classList.toggle("hidden", !tourDone.has(st.id));
  $("tour-back").classList.toggle("hidden", tourIdx === 0);
  $("tour-skip").classList.toggle("hidden", tourIdx === TOUR.length - 1);
  $("tour-next").textContent = tourIdx === 0 ? "Start" : tourIdx === TOUR.length - 1 ? "Finish" : tourDone.has(st.id) || !st.signal ? "Next" : "Skip this one";
  $("tour-enable").classList.add("hidden");
  if (st.on && st.enable) {
    const on = await st.on().catch(() => true);
    if (TOUR[tourIdx] === st) $("tour-enable").classList.toggle("hidden", on);
  }
}
let tourOpener: HTMLElement | null = null;
function tourOpen(): void {
  tourOpener = document.activeElement as HTMLElement | null;
  tourIdx = 0;
  tourDone = new Set();
  $("tour").classList.remove("hidden");
  tourShow();
  ($("tour-next") as HTMLButtonElement).focus();
}
async function tourClose(): Promise<void> {
  $("tour").classList.add("hidden");
  tourOpener?.focus?.();
  await invoke("tour_done_set", { done: true });
  if (tourChanged) location.reload();
}
$("tour-open").addEventListener("click", tourOpen);
$("tour-skip").addEventListener("click", tourClose);
$("tour-back").addEventListener("click", () => { tourIdx = Math.max(0, tourIdx - 1); tourShow(); });
$("tour-next").addEventListener("click", async () => {
  if (tourIdx === 0) {
    ($("tour-next") as HTMLButtonElement).disabled = true;
    try { await applyPreset(tourPreset); tourChanged ||= tourPreset !== "none"; }
    finally { ($("tour-next") as HTMLButtonElement).disabled = false; }
  }
  if (tourIdx >= TOUR.length - 1) return tourClose();
  tourIdx++;
  tourShow();
});
$("tour-enable").addEventListener("click", async () => {
  const st = TOUR[tourIdx];
  await st.enable?.();
  tourChanged = true;
  tourShow();
});
document.addEventListener("keydown", (e) => {
  if (e.key === "Escape" && !$("tour").classList.contains("hidden")) tourClose();
});
if (inTauri) {
  listen<string>("tour-step", (e) => {
    if ($("tour").classList.contains("hidden")) return;
    tourDone.add(e.payload);
    const st = TOUR[tourIdx];
    if (st.signal === e.payload) {
      tourShow();
      // Give the check mark a moment, then move on.
      setTimeout(() => { if (TOUR[tourIdx] === st) { tourIdx++; tourShow(); } }, 1200);
    }
  });
}


// ---- Tray menu hooks ----
if (inTauri) {
  listen("tour-open", () => tourOpen());
  listen<string>("nav", (e) => showScene(e.payload));
  listen("settings-changed", () => {
    invoke<DockStatus>("dock_status").then(renderDock);
    invoke<boolean>("switcher_status").then(renderSwitcher);
    invoke<SnapInfo>("snap_status").then(renderSnap);
    invoke<boolean>("topbar_status").then(renderTopbar);
    invoke<ThemeState>("theme_get").then(renderLook);
    invoke<HotkeyStatus>("hotkeys_status").then(renderHabits);
  });
}

// ---- Displays ----
interface DisplayInfo { name: string; width: number; height: number; x: number; y: number; scale: number; main: boolean }
async function renderDisplays(): Promise<void> {
  const list = inTauri ? await invoke<DisplayInfo[]>("displays") : [{ name: "", width: 2560, height: 1440, x: 0, y: 0, scale: 150, main: true }];
  const box = $("displays");
  box.innerHTML = "";
  list
    .sort((a, b) => Number(b.main) - Number(a.main) || a.x - b.x)
    .forEach((d, i) => {
      const chip = document.createElement("span");
      chip.className = "chip" + (d.main ? " on" : "");
      chip.setAttribute("role", "listitem");
      chip.textContent = `${d.main ? "Main" : `Screen ${i + 1}`}  ${d.width}×${d.height}  ${d.scale}%`;
      chip.title = d.name;
      box.appendChild(chip);
    });
}
renderDisplays();
if (inTauri) listen("displays-changed", () => { renderDisplays(); invoke<DockStatus>("dock_status").then(renderDock); });

// ---- Safe mode ----
interface SafeStatus { safe: boolean; reason: string }
const SAFE_WHY: Record<string, string> = {
  shift: "You held Shift as Cove started, so its features are paused. Your settings are kept.",
  crash: "Cove didn't start cleanly twice in a row, so it opened with its features paused. Your settings are kept.",
  flag: "Cove was started with --safe, so its features are paused. Your settings are kept.",
  manual: "Cove's features are paused. Your settings are kept.",
};
function renderSafe(s: SafeStatus): void {
  $("safe-banner").classList.toggle("hidden", !s.safe);
  $("safe-why").textContent = SAFE_WHY[s.reason] ?? SAFE_WHY.manual;
  ($("safe-toggle") as HTMLInputElement).checked = s.safe;
}
async function setSafe(on: boolean): Promise<void> {
  if (!inTauri) return renderSafe({ safe: on, reason: on ? "manual" : "" });
  renderSafe(await invoke<SafeStatus>("safe_set", { on }));
  toast(on ? "Safe mode on. Features paused." : "Safe mode off. Everything's back.");
}
if (inTauri) {
  invoke<SafeStatus>("safe_status").then(renderSafe);
  listen("safe-changed", () => invoke<SafeStatus>("safe_status").then(renderSafe));
}
$("safe-leave").addEventListener("click", () => setSafe(false));
$("safe-toggle").addEventListener("change", (e) => setSafe((e.target as HTMLInputElement).checked));


// ---- Keys drawn as keycaps ----
function keycaps(combo: string): string {
  const esc = (t: string) => t.replace(/&/g, "&amp;").replace(/</g, "&lt;");
  return `<span class="caps">${combo.split(" / ").map((alt) => alt.split("+").map((k) => `<kbd>${esc(k)}</kbd>`).join("")).join('<i>or</i>')}</span>`;
}

// ---- Title bar ----
if (inTauri) {
  const w = getCurrentWindow();
  $("cap-min").addEventListener("click", () => w.minimize());
  $("cap-max").addEventListener("click", () => w.toggleMaximize());
  $("cap-close").addEventListener("click", () => w.close());
  document.querySelector(".titlebar")!.addEventListener("dblclick", (e) => {
    if ((e.target as HTMLElement).closest("button, label")) return;
    w.toggleMaximize();
  });
}
// Cove desktop: the dock and menu bar together replace the Windows taskbar.
$("desk-toggle").addEventListener("change", async (e) => {
  const on = (e.target as HTMLInputElement).checked;
  const [d, bar] = await Promise.all([invoke<DockStatus>("dock_set", { enabled: on }), invoke<boolean>("topbar_set", { enabled: on })]);
  renderDock(d);
  renderTopbar(bar);
  toast(on ? "Cove desktop is on." : "Back to the Windows taskbar.");
});

// ---- About sheet ----
let aboutOpener: HTMLElement | null = null;
function openAbout(): void {
  aboutOpener = document.activeElement as HTMLElement | null;
  $("about").classList.remove("hidden");
  ($("about-close") as HTMLButtonElement).focus();
  checkUpdates(true);
}
function closeAbout(): void {
  if ($("about").classList.contains("hidden")) return;
  $("about").classList.add("hidden");
  aboutOpener?.focus?.();
}
$("about-open").addEventListener("click", openAbout);
$("about-close").addEventListener("click", closeAbout);
$("about").addEventListener("click", (e) => { if (e.target === $("about")) closeAbout(); });
addEventListener("keydown", (e) => { if (e.key === "Escape") closeAbout(); });
// Modal sheets keep Tab inside themselves.
addEventListener("keydown", (e) => {
  if (e.key !== "Tab") return;
  const modal = [$("tour"), $("about")].find((m) => !m.classList.contains("hidden"));
  if (!modal) return;
  const f = [...modal.querySelectorAll<HTMLElement>("button:not(:disabled), [tabindex='0']")].filter((el) => el.offsetParent !== null);
  if (!f.length) return;
  const first = f[0], last = f[f.length - 1];
  if (!modal.contains(document.activeElement)) { e.preventDefault(); first.focus(); }
  else if (e.shiftKey && document.activeElement === first) { e.preventDefault(); last.focus(); }
  else if (!e.shiftKey && document.activeElement === last) { e.preventDefault(); first.focus(); }
});
$("intro-replay").addEventListener("click", () => {
  closeAbout();
  // The intro plays in its own full-screen window, over everything.
  if (inTauri) invoke("intro_play").catch(() => {});
  else window.open("/intro.html?full=1", "_blank");
});
$("tour-open").addEventListener("click", () => $("about").classList.add("hidden"));

// ---- The miniature desktop ----
interface DockItemLite { kind: string; name: string; icon?: string; windows: number[] }
// The dock rows keep their skeleton tiles until every icon has loaded, then
// swap in all at once; a refresh with the same apps changes nothing (no
// flash of empty or greyed tiles each time the window gains focus).
const preloaded = new Map<string, Promise<boolean>>();
function preload(src: string): Promise<boolean> {
  let p = preloaded.get(src);
  if (!p) {
    p = new Promise<boolean>((done) => {
      const img = new Image();
      img.onload = () => { img.decode().then(() => done(true), () => done(true)); };
      img.onerror = () => done(false);
      img.src = src;
    });
    preloaded.set(src, p);
  }
  return p;
}
async function fillIcons(box: HTMLElement, items: DockItemLite[], size: string): Promise<void> {
  const key = JSON.stringify(items.map((i) => [i.name, i.icon ?? "", i.windows.length > 0]));
  if (box.dataset.key === key) return;
  box.dataset.want = key;
  const ok = await Promise.race([
    Promise.all(items.map((i) => (i.icon ? preload(i.icon) : Promise.resolve(false)))),
    new Promise<boolean[]>((done) => setTimeout(() => done(items.map(() => true)), 2500)),
  ]);
  if (box.dataset.want !== key) return; // a newer render won
  const frag = document.createDocumentFragment();
  items.forEach((it, n) => {
    const el = document.createElement("span");
    el.className = `pi ${size}${it.windows.length ? " open" : ""}`;
    el.title = it.name;
    el.setAttribute("role", "listitem");
    el.setAttribute("aria-label", it.windows.length ? `${it.name}, open` : it.name);
    if (it.icon && ok[n]) el.appendChild(Object.assign(document.createElement("img"), { src: it.icon, alt: "" }));
    else el.classList.add("noicon"); // a quiet neutral tile, not a pulsing placeholder
    frag.appendChild(el);
  });
  box.replaceChildren(frag);
  box.dataset.key = key;
  box.removeAttribute("aria-busy");
  if (!items.length && box.id === "dock-pins") box.insertAdjacentHTML("beforeend", '<p class="note">No pinned apps yet.</p>');
}
let pinsBusy = false;
async function renderPins(): Promise<void> {
  if (pinsBusy) return;
  pinsBusy = true;
  try {
    const st = inTauri
      ? await invoke<{ items: DockItemLite[] }>("dock_state").catch(() => null)
      : { items: ((window as any).__mockItems ?? []) as DockItemLite[] }; // browser preview only
    if (!st) return; // keep what's shown (or the skeleton) rather than blanking it
    const apps = st.items.filter((i) => i.kind === "pin" || i.kind === "running");
    await Promise.all([
      fillIcons($("mini-dock"), apps.slice(0, 14), "s"),
      fillIcons($("dock-pins"), st.items.filter((i) => i.kind === "pin"), "m"),
    ]);
  } finally { pinsBusy = false; }
}
function miniClock(): void {
  const d = new Date();
  $("mini-clock").textContent = d.toLocaleDateString(undefined, { weekday: "short", day: "numeric", month: "short" }) + "  " + d.toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" });
}
function miniWall(): void {
  // The real desktop picture, served by Cove itself (falls back to Ink).
  const url = inTauri ? `${navigator.userAgent.includes("Windows") ? "http://coveicon.localhost" : "coveicon://localhost"}/wallpaper?t=${Date.now()}` : "/wallpapers/tidepool-dark.jpg";
  const img = new Image();
  img.onload = () => { $("mini-wall").style.backgroundImage = `url("${url}")`; $("mini-wall").classList.add("ready"); };
  img.src = url;
}
miniClock();
setInterval(miniClock, 30000);
miniWall();
if (inTauri) listen("wallpaper-changed", () => setTimeout(miniWall, 400));
addEventListener("focus", () => { renderPins(); miniWall(); });
showScene(localStorage.getItem("cove.scene") ?? "desktop");
