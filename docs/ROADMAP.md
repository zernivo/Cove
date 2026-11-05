# Cove roadmap

Every phase is judged against one rule: **smoothness and flexibility**. Strip what nobody uses, keep every Windows function, and make all of it reversible.

Legend: ✅ done · 🔨 in progress · ⬜ planned

Current release: **v0.9.0** (the desktop takeover). Status below is as of that release.

---

## Phase 0: Foundations ✅ (v0.0.x)
- ✅ Research: Windows 11 pain points, macOS vs Windows in depth, 21 existing tools, Liquid Glass design
- ✅ Name, tagline and logo (the voxel C)
- ✅ Repo, Rust + Tauri 2 scaffold, Windows CI
- ✅ Change journal: every setting stored with its previous value, restored newest-first

## Phase 1: Calm + Mac habits ✅ (v0.1)
- ✅ Quiet Windows (was the Calm preset): per-user switches for Task View, the search box, Copilot, Start Recommended, lock-screen tips and suggestions
- ✅ Restore everything: one-click exact restore
- ✅ Mac shortcuts: Alt+W, Alt+M, Alt+H, Alt+Q, Alt+` (opt-in, conflict detection)
- ✅ Remembered settings, tray icon, single instance, one-click updater, release pipeline
- ⬜ Hide the Widgets button: Windows locks that setting on current builds, so Cove links to Taskbar settings instead ("Coming later")
- ⬜ Check every setting name on real Windows 11 24H2 / 25H2 / 26H2 machines

## Phase 2: Cove Search ✅ (v0.2–v0.4)
- ✅ Alt+Space launcher on the screen under the pointer, 640-wide glass panel
- ✅ Apps (desktop + Store) with real icons, files (Cove's own walk plus the Windows Search index, including file contents), Settings pages, quick actions, calculator
- ✅ Tabs and prefixes: All / Apps / Files / Settings / Clipboard
- ✅ Clipboard history: text only, memory only, skips password-manager copies
- ✅ Learns what you open most; fully local
- 🔨 Open time under 100 ms: measured in Settings → Keys, still to confirm on more hardware

## Phase 3: Dock + menu bar ✅ (v0.3, rebuilt in v0.9)
- ✅ Dock: pinned and running apps, AppID grouping like the taskbar, real icons, running dots, gentle magnification, launch and attention bounces, folder stacks, Recycle Bin (v0.3.0, v0.9.0)
- ✅ Dock hides when a window touches it (or Always / Never); full-screen apps hide it (v0.9.0)
- ✅ Status island at the dock's right end: pinned tray icons, Search, Wi-Fi, sound, battery, Control Centre, clock (v0.9.0)
- ✅ Cove menu on the Cove dock icon; app menus in each app's dock right-click menu (v0.9.0)
- ✅ Menu bar modes: Off (default), Show at the top edge, Always (24 DIP reserved AppBar) (v0.9.0)
- ✅ Multi-monitor and per-DPI placement; dock on the main display or follows the pointer (v0.3.4, v0.7.3)
- ⬜ Menu bar glass: the optional menu bar still paints Ink over a wallpaper plate; move it to the same clear blur-behind glass as the dock
- Note: a true global menu bar can't work for Chrome, Electron or Store apps, so the bar shows the app in front and common actions instead

## Phase 4: Window overview ✅ (v0.4)
- ✅ App-grouped switcher (Alt+Tab replacement, opt-in)
- ✅ App Exposé: all windows of one app, live DWM thumbnails
- ✅ Window snapping presets with smooth, interruptible motion

## Phase 5: Calm look ✅ (v0.5, rebuilt in v0.9)
- ✅ Wallpaper gallery: Cove art plus credited Commons and NASA photos in nine categories, downloaded on pick (v0.9.0)
- ✅ Light/dark schedule at set times; wallpaper follows the appearance (v0.5.1)
- ⬜ Live wallpapers: listed as "coming soon" in the gallery, off in v0.9.0

## Phase 6: Cove Tour ✅ (v0.6)
- ✅ Interactive first-run tour you learn by doing, replayable from About

## Phase 7: UI polish ✅ (v0.7)
- ✅ Type scale, focus rings, empty states, refined tray menu, final icon and installer artwork, DPI and multi-monitor pass

## Phase 8: Desktop takeover and the Liquid Glass redesign ✅ (v0.8–v0.9)
- ✅ Safe mode: Shift at start, `--safe`, tray or Settings, or automatic after two short launches (v0.8.0)
- ✅ Signed updates: every installer must match a minisign signature made offline with Cove's key (v0.8.1)
- ✅ Cove desktop: the taskbar is hidden completely while on and restored exactly when off, on quit, in safe mode and by the watchdog after a crash (v0.9.0)
- ✅ Liquid Glass redesign with graphite highlights; clear blur-behind glass that works on inactive windows (v0.9.0)
- ✅ Control Centre with other apps' tray icons through a stand-in tray, no injection (v0.9.0)
- ✅ Mac traffic lights on the focused window, covering the native buttons (v0.9.0)
- ✅ Intro B "Blocks settle" (v0.9.0)
- ✅ Settings redesign: the live desktop miniature, six scenes, instant controls (v0.9.0)
- ✅ Accessibility pass: keyboard-only, Narrator roles, reduced motion and transparency, contrast (v0.9.0)
- ✅ MIT licence, contributing guide, UI bundle size gate, smoke and visual check on a Windows runner (v0.9.0)

## Next (towards 1.0)
- ⬜ **Code signing** so SmartScreen stops warning: SignPath Foundation (free for open source) for the NSIS and MSI installers, or a **Microsoft Store MSIX** package, which Microsoft signs
- ⬜ **winget** package `zernivo.Cove`: draft manifests in `packaging/winget`, submitted once the installer is signed
- ⬜ **Live wallpapers**: original scenes that shift gently, near-zero CPU, paused on battery and under full-screen apps
- ⬜ **Phase 9: Feel & delight** (from community research; order not set): calm widgets (clock, calendar, now playing, battery; weather opt-in) in Cove's own windows, more dock motion polish, dynamic wallpapers
- ⬜ **Menu bar glass**: the optional menu bar on the same clear blur glass as the dock
- ⬜ Known issues from v0.9.0: blur, DPI and tray clicks checked on more PCs; light-mode contrast for white tray icons; a faster first Control Centre open
- ⬜ Performance gates in CI: ~0% idle CPU, ≤150 MB idle memory

---

### Never on the roadmap
System-file patching, Explorer injection, drivers, disabling Defender or updates, HKLM tweaks for normal use, restyling the real Start menu or taskbar by injecting into Explorer, or any Apple fonts, icons or wallpapers.
