import { applyTheme, type ThemeState } from "./theme";
import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface Status { version: string; journal_entries: number; windows: boolean }
interface Tweak { id: string; title: string; description: string; settings_uri: string; applied: boolean | null }
interface ApplyReport { applied: string[]; failed: [string, string][] }
interface RestoreReport { restored: number; failed: [string, string][] }
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
  restore_all: () => ({ restored: 0, failed: [] }),
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

// ---- navigation ----
document.querySelectorAll<HTMLAnchorElement>(".nav-item").forEach((a) =>
  a.addEventListener("click", (e) => {
    e.preventDefault();
    document.querySelectorAll(".nav-item").forEach((n) => n.classList.remove("active"));
    a.classList.add("active");
    document.querySelectorAll(".view").forEach((v) => v.classList.add("hidden"));
    $(`view-${a.dataset.view}`).classList.remove("hidden");
  }),
);

// ---- status ----
async function refreshStatus(): Promise<void> {
  const s = await invoke<Status>("cove_status");
  $("status").textContent = `v${s.version}${s.windows ? "" : " · preview"}`;
  $("journal-count").textContent =
    s.journal_entries === 0 ? "Nothing changed yet." : `${s.journal_entries} recorded change${s.journal_entries === 1 ? "" : "s"}`;
}

// ---- calm ----
async function renderCalm(): Promise<void> {
  const tweaks = await invoke<Tweak[]>("calm_list");
  const list = $("calm-list");
  list.innerHTML = "";
  for (const t of tweaks) {
    const row = document.createElement("label");
    row.className = "row item";
    row.innerHTML = `
      <input type="checkbox" value="${t.id}" ${t.applied ? "checked disabled" : ""} />
      <div class="grow"><h2></h2><p class="muted small"></p></div>
      <span class="pill ${t.applied ? "on" : ""}">${t.applied ? "On" : t.applied === null ? "—" : "Off"}</span>
      ${t.applied || !t.settings_uri ? "" : `<button class="btn link" data-uri="${t.settings_uri}" title="Change it in Windows Settings">Settings ›</button>`}`;
    row.querySelector("h2")!.textContent = t.title;
    row.querySelector("p")!.textContent = t.description;
    row.querySelector<HTMLButtonElement>("button.link")?.addEventListener("click", (e) => {
      e.preventDefault();
      invoke("open_settings", { uri: t.settings_uri });
    });
    list.appendChild(row);
  }
  if (!tweaks.length) list.innerHTML = `<p class="empty-state">Calm tweaks are available on Windows 11.</p>`;
  else if (tweaks.every((t) => t.applied)) {
    const p = document.createElement("p");
    p.className = "empty-state";
    p.textContent = "All calm. Restore puts everything back.";
    list.appendChild(p);
  }
}

$("calm-all").addEventListener("click", () =>
  document.querySelectorAll<HTMLInputElement>("#calm-list input:not(:disabled)").forEach((i) => (i.checked = true)),
);

$("calm-apply").addEventListener("click", async () => {
  const ids = [...document.querySelectorAll<HTMLInputElement>("#calm-list input:checked:not(:disabled)")].map((i) => i.value);
  if (ids.length === 0) return toast("Pick something to tidy first.");
  const r = await invoke<ApplyReport>("calm_apply", { ids });
  toast(r.failed.length
    ? `${r.applied.length} applied. Windows blocked ${r.failed.length}: use its Settings button.`
    : `${r.applied.length} applied. Restore anytime.`);
  await Promise.all([renderCalm(), refreshStatus()]);
});

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
  $("search-note").textContent = s.search
    ? s.conflicts.includes("Alt+Space") ? "Alt+Space is taken by another app (PowerToys Run?)" : "On · press Alt+Space anywhere"
    : "Off";
  ($("hk-toggle") as HTMLInputElement).checked = s.running;
  $("hk-note").textContent = s.running
    ? s.conflicts.length ? `On · taken by another app: ${s.conflicts.join(", ")}` : "On"
    : "Off · Cove remembers this, and keeps running in the tray when you close the window";
  $("hk-list").classList.toggle("dim", !s.running);
  $("hk-list").innerHTML = SHORTCUTS.map(
    ([k, d]) => `<div class="row item"><kbd>${k}</kbd><div class="grow">${d}</div>${s.conflicts.includes(k) ? '<span class="pill warn">In use</span>' : ""}</div>`,
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
  const speed = t.median != null ? ` · Search opens in about ${t.median} ms (last ${t.last} ms, ${t.count} open${t.count > 1 ? "s" : ""})` : "";
  $("wi-note").textContent = (i.winindex
    ? "On · also finds files by their contents, from the same index as File Explorer's search box. Read-only"
    : "Off · Cove searches your personal folders by name only") + speed;
}
$("wi-toggle").addEventListener("change", async (e) => {
  renderSearchInfo(await invoke<SearchInfo>("search_winindex_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<SearchInfo>("search_info").then(renderSearchInfo);
window.addEventListener("focus", () => invoke<SearchInfo>("search_info").then(renderSearchInfo));

// ---- Look (Cove Glass) ----
interface WallStatus { scenes: string[]; current: string; has_prev: boolean; schedule: string; light_at: string; dark_at: string; dark_now: boolean }
let mockWall: WallStatus = { scenes: ["tidepool", "lagoon", "dune", "slate"], current: "", has_prev: false, schedule: "off", light_at: "07:00", dark_at: "19:00", dark_now: false };
const WALL_NAMES: Record<string, string> = { tidepool: "Tidepool", lagoon: "Lagoon", dune: "Dune", slate: "Slate" };
let wall: WallStatus = mockWall;
function renderWall(w: WallStatus): void {
  wall = w;
  const v = (document.documentElement.dataset.mode === "dark") ? "dark" : "light";
  $("walls").innerHTML = "";
  for (const sc of w.scenes) {
    const b = document.createElement("button");
    b.className = `wall${w.current === sc ? " on" : ""}`;
    const img = document.createElement("img");
    img.src = `/wallpapers/${sc}-${v}.jpg`;
    img.alt = "";
    const t = document.createElement("span");
    t.textContent = WALL_NAMES[sc] ?? sc;
    b.append(img, t);
    b.addEventListener("click", async () => {
      try { renderWall(await invoke<WallStatus>("wallpaper_set", { scene: sc })); }
      catch (e) { $("wall-note").textContent = String(e); }
    });
    $("walls").appendChild(b);
  }
  $("wall-restore").classList.toggle("hidden", !w.has_prev);
  chipGroup("sched-mode", [["off", "Off"], ["times", "On a schedule"]], w.schedule, (k) => saveSched(k));
  ($("sched-light") as HTMLInputElement).value = w.light_at;
  ($("sched-dark") as HTMLInputElement).value = w.dark_at;
  $("sched-times").classList.toggle("off", w.schedule !== "times");
}
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
invoke<WallStatus>("wallpaper_status").then(renderWall);
if (inTauri) {
  listen("wallpaper-changed", () => {
    invoke<WallStatus>("wallpaper_status").then(renderWall);
    invoke<ThemeState>("theme_get").then(renderLook);
  });
}

let look: ThemeState = { theme: "tidepool", mode: "system", accent_source: "cove", accent: null };
const LOOK_NOTES: Record<string, string> = {
  tidepool: "Tidepool: warm glass that lets your wallpaper's colour through, with a soft glow on the app in front",
  slate: "Slate: quiet and mostly solid, graphite with one blue focus colour. Good for work",
};
function chipGroup(id: string, opts: [string, string][], on: string, pick: (k: string) => void): void {
  $(id).innerHTML = "";
  for (const [k, label] of opts) {
    const b = document.createElement("button");
    b.className = `chip${on === k ? " on" : ""}`;
    b.textContent = label;
    b.addEventListener("click", () => pick(k));
    $(id).appendChild(b);
  }
}
async function setLook(p: Partial<{ theme: string; mode: string; accent: string }>): Promise<void> {
  const args = { theme: look.theme, mode: look.mode, accent: look.accent_source, ...p };
  renderLook(await invoke<ThemeState>("theme_set", args));
}
function renderLook(t: ThemeState): void {
  look = t;
  applyTheme(t);
  if (wall) renderWall(wall);
  chipGroup("look-theme", [["tidepool", "Tidepool"], ["slate", "Slate"]], t.theme, (k) => setLook({ theme: k }));
  chipGroup("look-mode", [["system", "Follow Windows"], ["light", "Light"], ["dark", "Dark"]], t.mode, (k) => setLook({ mode: k }));
  chipGroup("look-accent", [["cove", "Theme's own"], ["windows", "Windows accent"]], t.accent_source, (k) => setLook({ accent: k }));
  $("look-theme-note").textContent = LOOK_NOTES[t.theme] ?? "";
  $("look-accent-note").textContent = t.accent_source === "windows"
    ? t.accent ? `Following Windows (${t.accent}). Change it in Settings → Personalization → Colors` : "Couldn't read the Windows accent, so the theme's own is used"
    : "The theme's own, or the accent you chose in Windows Settings";
}
invoke<ThemeState>("theme_get").then(renderLook);
window.addEventListener("focus", () => { if (look.accent_source === "windows") invoke<ThemeState>("theme_get").then(renderLook); });

interface DockStatus { enabled: boolean; hide_taskbar: boolean; stacks: string[]; display: string }
const STACKS: [string, string][] = [["downloads", "Downloads"], ["documents", "Documents"], ["desktop", "Desktop"], ["pictures", "Pictures"]];
let dockStacks: string[] = [];
function renderDock(d: DockStatus): void {
  dockStacks = d.stacks;
  $("dock-display").innerHTML = "";
  for (const [k, label] of [["main", "Main display"], ["follow", "Follows the pointer"]]) {
    const b = document.createElement("button");
    b.className = `chip${d.display === k ? " on" : ""}`;
    b.textContent = label;
    b.addEventListener("click", async () => renderDock(await invoke<DockStatus>("dock_display_set", { display: k })));
    $("dock-display").appendChild(b);
  }
  $("dock-stacks").innerHTML = "";
  for (const [k, label] of STACKS) {
    const b = document.createElement("button");
    b.className = `chip${d.stacks.includes(k) ? " on" : ""}`;
    b.textContent = label;
    b.addEventListener("click", async () => {
      const keys = dockStacks.includes(k) ? dockStacks.filter((x) => x !== k) : [...dockStacks, k];
      renderDock(await invoke<DockStatus>("dock_stacks_set", { keys }));
    });
    $("dock-stacks").appendChild(b);
  }
  ($("dock-toggle") as HTMLInputElement).checked = d.enabled;
  ($("dock-taskbar") as HTMLInputElement).checked = d.hide_taskbar;
  ($("dock-taskbar") as HTMLInputElement).disabled = !d.enabled;
  $("dock-note").textContent = d.enabled ? (d.hide_taskbar ? "On · taskbar auto-hidden" : "On") : "Off";
}
$("dock-toggle").addEventListener("change", async (e) => {
  renderDock(await invoke<DockStatus>("dock_set", { enabled: (e.target as HTMLInputElement).checked }));
});
$("dock-taskbar").addEventListener("change", async (e) => {
  renderDock(await invoke<DockStatus>("dock_hide_taskbar_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<DockStatus>("dock_status").then(renderDock);

function renderSwitcher(on: boolean): void {
  ($("sw-toggle") as HTMLInputElement).checked = on;
  $("sw-note").textContent = on
    ? "On · hold Alt and tap Tab: one icon per app. Shift+Tab goes back, ↓ shows all that app's windows, Esc cancels, release Alt to switch"
    : "Off · Windows' own Alt+Tab. Turn on for Mac-style switching by app instead of by window";
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
  $("snap-note").textContent = s.enabled
    ? s.conflicts.length
      ? `On · ${s.conflicts.length} shortcut${s.conflicts.length > 1 ? "s are" : " is"} taken by another app`
      : "On · works on the window in front. Windows' own Win+arrow snapping still works too"
    : "Off · turn on for keyboard presets: halves, quarters, thirds, centre, fill";
  $("snap-list").classList.toggle("dim", !s.enabled);
  $("snap-list").innerHTML = SNAPS.map(
    ([k, d]) => {
      const prefix = k.slice(0, k.lastIndexOf("+") + 1);
      const taken = k.slice(prefix.length).split(" / ").some((p) => s.conflicts.includes(prefix + p));
      return `<div class="row item"><kbd>${k}</kbd><div class="grow">${d}</div>${taken ? '<span class="pill warn">In use</span>' : ""}</div>`;
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
  $("topbar-note").textContent = on
    ? "On · the app in front, quick settings, battery and clock; windows make room for it"
    : "Off · a thin bar across the top with the app in front, quick settings, battery and clock";
}
$("topbar-toggle").addEventListener("change", async (e) => {
  renderTopbar(await invoke<boolean>("topbar_set", { enabled: (e.target as HTMLInputElement).checked }));
});
invoke<boolean>("topbar_status").then(renderTopbar);

function renderClip(on: boolean): void {
  ($("clip-toggle") as HTMLInputElement).checked = on;
  $("clip-note").textContent = on
    ? "On · text only, kept in memory until Cove quits, skips password managers"
    : "Off";
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
  toast(r.failed.length ? `Restored ${r.restored}, ${r.failed.length} failed` : r.restored ? `Restored ${r.restored} settings.` : "Already stock.");
  await Promise.all([renderCalm(), refreshStatus()]);
});

// ---- updates ----
async function checkUpdates(quiet = false): Promise<void> {
  const note = $("up-note");
  const btn = $("up-install");
  if (!quiet) note.textContent = "Checking…";
  try {
    const u = await invoke<UpdateInfo>("update_check");
    $("up-title").textContent = `Cove ${u.current}`;
    btn.classList.toggle("hidden", !u.available);
    $("update-dot").classList.toggle("hidden", !u.available);
    const notes = $("up-notes");
    notes.textContent = u.notes;
    notes.classList.toggle("hidden", !u.available || !u.notes);
    note.textContent = u.available ? `Version ${u.latest} is available.` : "You're on the latest version.";
    btn.textContent = `Update to ${u.latest}`;
  } catch (e) {
    note.textContent = String(e);
  }
}

$("up-check").addEventListener("click", () => checkUpdates());
if (inTauri) {
  listen<[number, number]>("update-progress", (e) => {
    const [done, total] = e.payload;
    const pct = total ? Math.round((done / total) * 100) : 0;
    ($("up-bar") as HTMLElement).style.width = `${pct}%`;
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
  await Promise.all([refreshStatus(), renderCalm(), invoke<HotkeyStatus>("hotkeys_status").then(renderHabits)]);
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
  const ids = tweaks.filter((t) => t.applied === false).map((t) => t.id);
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
      b.innerHTML = `<b></b><span></span>`;
      (b.querySelector("b") as HTMLElement).textContent = name;
      (b.querySelector("span") as HTMLElement).textContent = desc;
      b.addEventListener("click", () => { tourPreset = k; tourShow(); });
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
function tourOpen(): void {
  tourIdx = 0;
  tourDone = new Set();
  $("tour").classList.remove("hidden");
  tourShow();
  ($("tour-next") as HTMLButtonElement).focus();
}
async function tourClose(): Promise<void> {
  $("tour").classList.add("hidden");
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
invoke<boolean>("tour_status").then((done) => { if (!done) tourOpen(); });

// ---- Tray menu hooks ----
if (inTauri) {
  listen("tour-open", () => tourOpen());
  listen<string>("nav", (e) => document.querySelector<HTMLAnchorElement>(`.nav-item[data-view="${e.payload}"]`)?.click());
  listen("settings-changed", () => {
    invoke<DockStatus>("dock_status").then(renderDock);
    invoke<boolean>("switcher_status").then(renderSwitcher);
    invoke<SnapInfo>("snap_status").then(renderSnap);
    invoke<boolean>("topbar_status").then(renderTopbar);
    invoke<ThemeState>("theme_get").then(renderLook);
    invoke<HotkeyStatus>("hotkeys_status").then(renderHabits);
  });
}
