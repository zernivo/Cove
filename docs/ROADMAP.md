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
- ✅ Deeper file search across your personal folders (v0.2.3); Windows Search index still to come
- ✅ Real app and file icons (v0.2.2)
- ✅ Clipboard history tab, text only, memory only (v0.2.3)
- ⬜ Measure open time on real hardware (target under 100 ms)

## Phase 3: Dock + top bar 🟡 (v0.3)
- ✅ Top bar (AppBar API): active app name, Cove menu, quick settings, battery, clock (v0.3.1); Control Centre panel: sound/brightness sliders, mute, dark mode, Wi-Fi (v0.3.2)
- ✅ Dock: pinned + running apps, running dots, gentle magnification, Recycle Bin (v0.3.0); folder stacks: Downloads/Documents/Desktop/Pictures grid (v0.3.3)
- ✅ Taskbar auto-hide only through its own documented setting, restored when Cove quits (v0.3.0)
- ⬜ Multi-monitor and per-DPI support
- Note: a true global menu bar can't work for Chrome, Electron or Store apps, so the top bar shows the active app and common actions instead

## Phase 4: Window overview ⬜ (v0.4)
- ⬜ App-grouped switcher (Alt+Tab replacement, opt-in)
- ⬜ App Exposé: all windows of one app at a glance
- ⬜ Window snapping presets with smooth, interruptible motion (150–250 ms)

## Phase 5: Calm look ⬜ (v0.5)
- ⬜ Cove Glass theme for Cove's own surfaces (light/dark, accent #3BA7FF)
- ⬜ Wallpaper gallery: 4K, light/dark pairs, permissively licensed or original only
- ⬜ Accent, light/dark and wallpaper scheduling through documented settings

## Phase 6: Cove Tour ⬜ (v0.6)
- ⬜ Interactive first-run tutorial: try each shortcut live, pick a preset
- ⬜ Replayable anytime from settings

## Phase 7: UI polish ⬜ (v0.7)
- ⬜ One pass over every screen: spacing, type scale, colour, contrast, empty states
- ⬜ Premium window backgrounds (subtle gradient that sets off the logo), light and dark
- ⬜ Motion: page transitions, toggles, search pop-in, all at 60 fps and respecting reduced motion
- ⬜ Refined tray menu, onboarding visuals
- ⬜ Final app icon and installer artwork; consistency check across DPI and multiple monitors

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
