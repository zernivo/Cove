// Settings › Windows: "Mac window buttons" on/off and the per-app
// "Don't show on this app" list. Instant controls, no Apply.

import { invoke as tauriInvoke } from "@tauri-apps/api/core";

interface AppEntry { exe: string; name: string }
interface TrafficInfo { enabled: boolean; skip: AppEntry[]; seen: AppEntry[] }

const inTauri = "__TAURI_INTERNALS__" in window;
let mock: TrafficInfo = {
  enabled: true,
  skip: [],
  seen: [
    { exe: "chrome.exe", name: "Google Chrome" },
    { exe: "explorer.exe", name: "File Explorer" },
    { exe: "notepad.exe", name: "Notepad" },
  ],
};

function invoke(cmd: string, args: Record<string, unknown> = {}): Promise<TrafficInfo> {
  if (inTauri) return tauriInvoke<TrafficInfo>(cmd, args);
  if (cmd === "traffic_set") mock = { ...mock, enabled: args.enabled as boolean };
  if (cmd === "traffic_skip_set") {
    const keys = args.apps as string[];
    const all = [...mock.seen, ...mock.skip];
    mock = { ...mock, skip: all.filter((a) => keys.includes(a.exe)), seen: all.filter((a) => !keys.includes(a.exe)) };
  }
  return Promise.resolve(mock);
}

const el = <T extends HTMLElement>(id: string) => document.getElementById(id) as T | null;

const style = document.createElement("style");
style.textContent = `
.tl-add { height: 28px; max-width: 220px; padding: 0 10px; border-radius: 7px; border: 0; background: var(--ink-2); color: var(--fg); box-shadow: inset 0 0 0 1px var(--hair); font: inherit; font-size: 12.5px; }
.tl-skip:empty { display: none; }
.tl-skip .chip { display: inline-flex; align-items: center; gap: 6px; }
.tl-skip .chip b { font-weight: 500; color: var(--fg-3); }
.tl-skip .chip:hover b { color: var(--fg); }
`;
document.head.appendChild(style);

let info: TrafficInfo = { enabled: true, skip: [], seen: [] };

function render(t: TrafficInfo): void {
  info = t;
  const toggle = el<HTMLInputElement>("tl-toggle");
  if (!toggle) return;
  toggle.checked = t.enabled;
  el("tl-note")!.textContent = t.enabled
    ? "Close, minimise and zoom appear at the top-left of the window you're using when the pointer comes near. Windows' own buttons stay as they are."
    : "Windows' own buttons only.";
  el("tl-skip-row")!.classList.toggle("dim", !t.enabled);
  el("tl-skip")!.classList.toggle("dim", !t.enabled);
  el("tl-skip-note")!.textContent = t.skip.length ? "" : "Apps you've used since Cove started can be added here.";
  const add = el<HTMLSelectElement>("tl-add")!;
  add.innerHTML = "";
  const first = document.createElement("option");
  first.value = "";
  first.textContent = t.seen.length ? "Add an app…" : "No apps yet";
  add.appendChild(first);
  for (const a of t.seen) {
    const o = document.createElement("option");
    o.value = a.exe;
    o.textContent = a.name;
    add.appendChild(o);
  }
  add.disabled = !t.enabled || !t.seen.length;
  const list = el("tl-skip")!;
  list.innerHTML = "";
  for (const a of t.skip) {
    const b = document.createElement("button");
    b.className = "chip";
    b.title = `Show the buttons on ${a.name} again`;
    b.innerHTML = `<span></span><b aria-hidden="true">×</b>`;
    b.querySelector("span")!.textContent = a.name;
    b.addEventListener("click", async () => {
      render(await invoke("traffic_skip_set", { apps: info.skip.map((s) => s.exe).filter((e) => e !== a.exe) }));
    });
    list.appendChild(b);
  }
}

el("tl-toggle")?.addEventListener("change", async (e) => {
  render(await invoke("traffic_set", { enabled: (e.target as HTMLInputElement).checked }));
});
el("tl-add")?.addEventListener("change", async (e) => {
  const exe = (e.target as HTMLSelectElement).value;
  if (!exe) return;
  render(await invoke("traffic_skip_set", { apps: [...info.skip.map((s) => s.exe), exe] }));
});
// The list of apps grows as you use them; refresh whenever Settings is shown.
addEventListener("focus", () => invoke("traffic_status").then(render).catch(() => {}));
invoke("traffic_status").then(render).catch(() => {});
