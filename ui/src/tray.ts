// Other apps' tray icons: shared types, the browser-preview stand-ins and
// the Settings switch. The grid itself is tray-grid.ts (Control Centre).

export interface TrayItem {
  id: string;
  /** Short name: the tooltip's first line, or the app's name. */
  name: string;
  /** The whole tooltip. */
  tip: string;
  icon: string | null;
  /** "cove": Cove's stand-in tray has it (full clicks); "windows": only Windows' own tray lists it. */
  source: "cove" | "windows";
  /** Stable across restarts; what pins are stored by. */
  key: string;
}

export interface TrayStatus {
  enabled: boolean;
  /** "cove" = Cove's stand-in tray is running; "windows" = only Windows' list; "off". */
  mode: "cove" | "windows" | "off";
  note: string | null;
  items: TrayItem[];
  pins: string[];
}

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>;

// Simple, recognisable stand-ins for the browser preview only (not logos).
const svg = (body: string) => `data:image/svg+xml;utf8,${encodeURIComponent(`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">${body}</svg>`)}`;
export const MOCK_ICONS: Record<string, string> = {
  discord: svg(`<rect x="2" y="5" width="28" height="22" rx="9" fill="#5865F2"/><circle cx="11.5" cy="16" r="2.6" fill="#fff"/><circle cx="20.5" cy="16" r="2.6" fill="#fff"/>`),
  steam: svg(`<circle cx="16" cy="16" r="14" fill="#1b2838"/><circle cx="20" cy="12" r="4.6" fill="none" stroke="#c7d5e0" stroke-width="2.2"/><circle cx="11" cy="20.5" r="3.2" fill="#c7d5e0"/><path d="M12 19.6l6-5" stroke="#c7d5e0" stroke-width="2.2"/>`),
  spotify: svg(`<circle cx="16" cy="16" r="14" fill="#1ED760"/><path d="M8.5 12.2c5-1.6 10.6-1.2 15 1.2M9.4 16.3c4.2-1.2 8.8-.8 12.4 1.1M10.3 20.1c3.4-.9 6.8-.6 9.6.9" fill="none" stroke="#111" stroke-width="2.2" stroke-linecap="round"/>`),
  onedrive: svg(`<path d="M9 25h15.5a5.5 5.5 0 0 0 .8-10.9A8 8 0 0 0 10 12.4 6.3 6.3 0 0 0 9 25z" fill="#0F78D4"/>`),
  nvidia: svg(`<rect x="3" y="6" width="26" height="20" rx="3" fill="#76B900"/><path d="M9 20c3-7 11-8 15-4-4-2-9-1-11 4" fill="#fff"/>`),
  dropbox: svg(`<path d="M10 5l6 4-6 4-6-4zM22 5l6 4-6 4-6-4zM4 17l6-4 6 4-6 4zM28 17l-6-4-6 4 6 4zM10 22.5l6-4 6 4-6 4z" fill="#0061FF"/>`),
  defender: svg(`<path d="M16 3l11 4v8c0 7-5 12-11 14C10 27 5 22 5 15V7z" fill="#2b88d8"/><path d="M11 16l4 4 7-8" fill="none" stroke="#fff" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round"/>`),
  obs: svg(`<circle cx="16" cy="16" r="14" fill="#302E31"/><circle cx="16" cy="9.5" r="4" fill="#fff"/><circle cx="10.4" cy="19" r="4" fill="#fff"/><circle cx="21.6" cy="19" r="4" fill="#fff"/>`),
};

export const MOCK_STATUS: TrayStatus = {
  enabled: true,
  mode: "cove",
  note: null,
  pins: ["{discord}", "steam.exe#1", "spotify.exe#0"],
  items: [
    { id: "g1", name: "Discord", tip: "Discord\n3 unread messages", icon: MOCK_ICONS.discord, source: "cove", key: "{discord}" },
    { id: "h2", name: "Steam", tip: "Steam", icon: MOCK_ICONS.steam, source: "cove", key: "steam.exe#1" },
    { id: "h3", name: "Spotify", tip: "Spotify Premium", icon: MOCK_ICONS.spotify, source: "cove", key: "spotify.exe#0" },
    { id: "h4", name: "OneDrive", tip: "OneDrive - Personal\nUp to date", icon: MOCK_ICONS.onedrive, source: "cove", key: "onedrive.exe#1" },
    { id: "h5", name: "NVIDIA Settings", tip: "NVIDIA Settings", icon: MOCK_ICONS.nvidia, source: "cove", key: "nvcontainer.exe#1" },
    { id: "h6", name: "Dropbox", tip: "Dropbox 210.4.5730\nUp to date", icon: MOCK_ICONS.dropbox, source: "cove", key: "dropbox.exe#0" },
    { id: "h7", name: "Windows Security", tip: "Windows Security - no actions needed.", icon: MOCK_ICONS.defender, source: "cove", key: "securityhealthsystray.exe#1" },
    { id: "w8", name: "OBS Studio", tip: "OBS Studio", icon: MOCK_ICONS.obs, source: "windows", key: "w:obs studio" },
  ],
};

/** Settings > Dock: "Show other apps' tray icons". */
export async function initTraySetting(invoke: Invoke): Promise<void> {
  const toggle = document.getElementById("tray-toggle") as HTMLInputElement | null;
  const note = document.getElementById("tray-note");
  if (!toggle || !note) return;
  const render = (s: TrayStatus | null, takeover: boolean) => {
    if (!s) return;
    toggle.checked = s.enabled;
    toggle.disabled = !takeover;
    note.textContent = !takeover
      ? "The Windows taskbar shows them while it's on."
      : s.note ?? "Discord, Steam and the rest: the first four in the dock, all of them in Control Centre.";
    note.classList.toggle("warn", !!s.note && takeover);
  };
  const takeover = () => !(document.getElementById("dock-taskbar") as HTMLInputElement | null)?.disabled
    && !!(document.getElementById("dock-taskbar") as HTMLInputElement | null)?.checked;
  const refresh = async () => render(await invoke<TrayStatus>("tray_status").catch(() => null), takeover());
  toggle.addEventListener("change", async () => {
    render(await invoke<TrayStatus>("tray_set", { enabled: toggle.checked }).catch(() => null), takeover());
  });
  // The takeover switches change whether the setting applies.
  for (const id of ["dock-taskbar", "dock-toggle", "desk-toggle"]) {
    document.getElementById(id)?.addEventListener("change", () => setTimeout(refresh, 300));
  }
  await refresh();
}
