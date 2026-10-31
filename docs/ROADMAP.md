# Cove roadmap

Every phase is judged against one rule: **smoothness and flexibility**. Strip what nobody uses, keep every Windows function, and make all of it reversible.

Legend: ✅ done · 🔨 in progress · ⬜ planned

---

## Phase 0: Foundations ✅ (v0.0.x)
- ✅ Research: Windows 11 pain points, macOS vs Windows in depth, 21 existing tools, Liquid Glass design
- ✅ Name, tagline and logo (impossible-C mark, centred blue cube)
- ✅ Repo, Rust + Tauri 2 scaffold, Windows CI
- ✅ Change journal: every setting stored with its previous value, restored newest-first

## Phase 1: Calm + Mac habits 🔨 (v0.1)
- ✅ Calm preset: 7 per-user toggles (Task View, search box, Widgets, Copilot, Start Recommended, lock-screen tips, suggestions)
- ✅ Back to stock: one-click exact restore
- ✅ Hotkeys: Alt+W close, Alt+M minimize, Alt+H hide app, Alt+Q quit app, Alt+` cycle app windows (opt-in, conflict detection)
- ✅ Settings window: Calm / Mac habits / Back to stock
- ✅ Release pipeline: Windows installer built on every version tag
- ✅ Remembered settings, close to tray, single instance, one-click updater, new app icon
- ⬜ Test on real Windows 11 24H2/25H2/26H2 and fix setting names that differ
- ⬜ Tray icon, launch at sign-in (optional), remember hotkey on/off

## Phase 2: Cove Search 🔨 (v0.2)
- ✅ Alt+Space launcher (pre-created hidden window, opens on the monitor under the cursor)
- ✅ Apps via Get-StartApps (desktop + Store), recent files, 27 Settings pages, quick actions, calculator
- ✅ Tabs: All / Apps / Files / Settings; Tab to switch, Enter to open, Ctrl+Enter to reveal
- ✅ Learns what you open most; fully local, no web results, no telemetry
- ⬜ Real app icons in results
- ✅ Deeper file search across your personal folders (v0.2.3); Windows Search index results, including file contents (v0.4.3)
- ✅ Real app and file icons (v0.2.2)
- ✅ Clipboard history tab, text only, memory only (v0.2.3)
- ✅ Built-in open-time measurement shown in Settings (v0.4.3); target under 100 ms still to confirm on real hardware

## Phase 3: Dock + top bar 🟡 (v0.3)
- ✅ Top bar (AppBar API): active app name, Cove menu, quick settings, battery, clock (v0.3.1); Control Centre panel: sound/brightness sliders, mute, dark mode, Wi-Fi (v0.3.2)
- ✅ Dock: pinned + running apps, running dots, gentle magnification, Recycle Bin (v0.3.0); folder stacks: Downloads/Documents/Desktop/Pictures grid (v0.3.3)
- ✅ Taskbar auto-hide only through its own documented setting, restored when Cove quits (v0.3.0)
- ✅ Multi-monitor and per-DPI support: dock on main display or follows the pointer (bottom-edge rest), dock + top bar re-fit on display plug/resize/scale changes (v0.3.4). Top bar stays on the main display.
- Note: a true global menu bar can't work for Chrome, Electron or Store apps, so the top bar shows the active app and common actions instead

## Phase 4: Window overview 🚧 (v0.4)
- ✅ App-grouped switcher (Alt+Tab replacement, opt-in, WH_KEYBOARD_LL hook removed when off) (v0.4.0)
- ✅ App Exposé: all windows of one app at a glance, live DWM thumbnails; Dock right-click or ↓ in the switcher (v0.4.1)
- ✅ Window snapping presets with smooth, interruptible motion (200 ms glide, Ctrl+Alt presets, opt-in) (v0.4.2)

## Phase 5: Calm look ✅ (v0.5)
- ✅ Cove Glass themes for Cove's own surfaces: Tidepool (default) and Slate, light/dark/follow Windows, theme accent or Windows accent (v0.5.0)
- ✅ Wallpaper gallery: four original 4K scenes (Tidepool, Lagoon, Dune, Slate), each in light and dark; previous wallpaper restorable (v0.5.1)
- ✅ Light/dark schedule at set times; wallpaper follows Cove's appearance; Windows accent option (v0.5.0–v0.5.1)

## Phase 6: Cove Tour ✅ (v0.6)
- ✅ Interactive first-run tour: pick a starting preset (Mac-like, Light touch, or change nothing), then try Search, Mac shortcuts, snapping, the switcher and App Exposé live; each step ticks itself off when you use it (v0.6.0)
- ✅ Replayable anytime from Take the tour in the sidebar (v0.6.0)

## Phase 7: UI polish 🚧 (v0.7)
- ✅ One pass over the settings screens: type scale, spacing, contrast-safe text on the accent (also with the Windows accent), keyboard focus rings, empty states (v0.7.0)
- ✅ Window backgrounds: a soft Tidepool wash that sets off the logo, a quiet gradient for Slate, light and dark (v0.7.0)
- ✅ Motion: page fade, sliding nav indicator, chip/card/toggle transitions, all off with reduced motion (v0.7.0); 60 fps still to confirm on hardware
- ✅ Refined tray menu: left-click opens Cove; right-click toggles Dock, Top bar, Mac shortcuts, App switcher, Window snapping, Appearance, plus Tour, Updates and Restore (v0.7.1)
- ✅ Onboarding visuals: Cove Tour in the Cove Glass style (v0.6.0–v0.7.0)
- ✅ Final app icon (Deep tide) and installer artwork for NSIS and MSI (v0.7.2)
- ⬜ Consistency check across DPI and multiple monitors

## Phase 8: Beta → 1.0 ⬜
- ⬜ Safe mode (hold a key at start to skip every Cove change)
- ⬜ Code signing (e.g. SignPath Foundation for open source) so SmartScreen stops warning
- ⬜ Auto-update (Tauri updater, signed)
- ⬜ Performance gates in CI: ~0% idle CPU, ≤150 MB idle memory (fall back to WinUI 3 if exceeded)
- ⬜ Accessibility pass (keyboard-only, screen reader, reduced motion)
- ⬜ Licence, contributor guide, winget package

---

### Never on the roadmap
System-file patching, Explorer injection, drivers, disabling Defender or updates, HKLM tweaks for normal use, or any Apple fonts, icons or wallpapers.
