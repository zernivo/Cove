import { invoke as tauriInvoke } from "@tauri-apps/api/core";

interface Status { version: string; journal_entries: number; windows: boolean }
interface Tweak { id: string; title: string; description: string; settings_uri: string; applied: boolean | null }
interface ApplyReport { applied: string[]; failed: [string, string][] }
interface RestoreReport { restored: number; failed: [string, string][] }
interface HotkeyStatus { running: boolean; conflicts: string[] }

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
  hotkeys_set: (a) => ({ running: a.enabled, conflicts: [] }),
  hotkeys_status: () => ({ running: false, conflicts: [] }),
  open_settings: () => null,
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
  ($("hk-toggle") as HTMLInputElement).checked = s.running;
  $("hk-note").textContent = s.running
    ? s.conflicts.length ? `On · taken by another app: ${s.conflicts.join(", ")}` : "On"
    : "Off · Cove remembers this, and keeps running in the tray when you close the window";
  $("hk-list").innerHTML = SHORTCUTS.map(
    ([k, d]) => `<div class="row item"><kbd>${k}</kbd><div class="grow">${d}</div>${s.conflicts.includes(k) ? '<span class="pill warn">In use</span>' : ""}</div>`,
  ).join("");
}

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

// ---- boot ----
(async () => {
  await Promise.all([refreshStatus(), renderCalm(), invoke<HotkeyStatus>("hotkeys_status").then(renderHabits)]);
})();
