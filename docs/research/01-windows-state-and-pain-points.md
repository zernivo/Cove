# Windows state & pain points (researched 2026-10-27)

## Current Windows
- Windows 11 26H2 ("2026 Update") GA 2026-09-29, enablement package on 24H2/25H2 base (build 26300 / 25H2 = 26200). Few exclusive features; features ship monthly.
- 26H1 only for new 2026 devices (Germanium-based), not an upgrade.
- 24H2 Home/Pro end of updates 2026-10-13.
- Win10 ended 2025-10-14; consumer ESU through 2027-10-12. Share (StatCounter, Sep/Oct 2026): Win11 ~71.5%, Win10 ~27.8%.

## UI changes 2026
- Taskbar movable to top/left/right + resizable (Sept 2026). Limits: search box and small taskbar only at bottom; no multi-row.
- Start redesign: scrollable All, Category/Grid views, small/large size, toggle Pinned/Recent/All. WinUI3. PCWorld grade C.
- Search rebuilt on WinUI3: actions in results (dark mode, Bluetooth), less Bing clutter, Copilot optional. PCWorld A-.
- Copilot pulled out of Notepad/Photos/Snipping Tool; Copilot key remappable to Right Ctrl.
- File Explorer: customizable context menu (preview), dark Folder Options, preserved section state, preview for downloaded files.
- Widgets quieter; updates simplified, can pause to a date.

## Pain points still standing
- Ads/"suggestions" in Start, Settings (M365 upsell), lock screen, OneDrive & Edge nags.
- Forced Microsoft account at setup; OneDrive backup on by default.
- Inconsistent UI: Control Panel + legacy dialogs beside modern Settings.
- Performance on 8GB RAM; background bloat, telemetry.
- Start menu still disliked; taskbar customization stops one step short.
- AI pushed without being asked.

## Implications for the app
- Use native options where they now exist (top taskbar, Start size) = safer, no patching.
- Gaps to fill: Mac dock, global menu bar, window controls/rounded+blur consistency, Spotlight-style launcher (native search now good, maybe wrap it), quiet mode (ads/nag toggles via documented settings), unified look over legacy dialogs (hard; theme-level only).
- Must stay light (8GB PCs) and revert cleanly.

Sources: learn.microsoft.com 26H2 what's new; blogs.windows.com 2026-10-15; PCWorld report card 2026-10-01; PCMag 2026-10-02; Neowin KB5127936; HowToGeek 2026-04-15 & 2026-10-18; Windows Central; The Register 2026-10-05; BleepingComputer 2026-10-13.
