# Architecture

Cove is a Rust core on Tauri 2 with a TypeScript UI (Vite, no framework) in
WebView2. The core owns every Windows call; the pages only draw and send
commands. This page describes the code at v0.9.0.

```
Cove/
├─ src-tauri/              Rust core + Tauri 2 shell
│  ├─ src/                 one module per feature (below)
│  ├─ build.rs             embeds COVE_BUILD_ID (GIT_SHA / GITHUB_SHA / git HEAD)
│  ├─ tauri.conf.json      windows, bundle (NSIS + MSI), installer art
│  ├─ capabilities/        what each page may call
│  ├─ wallpapers/          bundled Cove wallpapers (light + dark)
│  └─ windows/hooks.nsh    NSIS hooks (icon cache refresh)
├─ ui/                     the pages (one HTML entry per window) + src/*.ts, *.css
├─ scripts/                check-bundle-size.mjs, check-wallpapers.mjs
├─ signatures/             offline update signatures (*.sig)
├─ .github/                ci.yml, release.yml, preview.yml, smoke.yml, scripts/visual.ps1
├─ brand/                  logo, icon and installer-art generators
├─ docs/                   vision, roadmap, safety, testing, design, research
└─ packaging/winget/       draft winget manifests
```

## Rust modules (`src-tauri/src`)

| Module | Responsibility | Windows APIs |
|---|---|---|
| `main.rs` | App entry, setup, every Tauri command, the tray menu, safe mode, the watchdog entry (`--watchdog <pid> <settings>`), the menu bar modes, popover placement, `display_watch`, the hidden test hook `--cove-open=cc\|wifi\|cove\|menu\|cal` | Tauri, Win32 |
| `dock.rs` | The dock's app list (real windows by Alt+Tab rules, AppID grouping, pins), hiding and restoring the taskbar (`ShowWindow` on `Shell_TrayWnd` / `Shell_SecondaryTrayWnd`, its documented auto-hide), full-screen detection, attention via the shell hook, the watchdog | `EnumWindows`, `SHAppBarMessage`, `RegisterShellHookWindow` |
| `appbar.rs` | Reserves screen space for the menu bar (Always) and the dock (Hide the dock: Never) as documented AppBars; records each registration to a file so the watchdog can release it | `SHAppBarMessage` |
| `topbar.rs` | The optional menu bar: its window, the app in front, battery, clock | AppBar via `appbar.rs` |
| `glass.rs` | `backdrop`: one native blur-behind window per glass panel, kept under its Cove window; rounded window regions; small desktop actions | `SetWindowCompositionAttribute` (ACCENT_ENABLE_BLURBEHIND, acrylic as fallback), `DwmSetWindowAttribute`, out-of-context `SetWinEventHook` on Cove's own windows |
| `tray.rs` | Other apps' tray icons: the stand-in tray window, UI Automation names, icon PNGs, pins for the status island, handing icons back to Explorer on exit | `Shell_NotifyIcon` messages (`WM_COPYDATA`), `TaskbarCreated`, UI Automation |
| `traffic.rs` | Mac traffic lights on the focused window: measures the caption buttons, places the `traffic` overlay over them, turns clicks into the window's own `WM_SYSCOMMAND` | out-of-context WinEvents, `DWMWA_CAPTION_BUTTON_BOUNDS`, UI Automation |
| `icons.rs` | Real icons at 256 px for the dock, Search and the switcher, served from memory through the `coveicon` URL scheme | `IShellItemImageFactory`, AppxManifest logos, stock icons |
| `wallpaper.rs` | Bundled and gallery wallpapers (download on pick, checksum, cache), restore of your old wallpaper, the light/dark schedule | `SystemParametersInfo(SPI_SETDESKWALLPAPER)` |
| `controls.rs` | Control Centre: volume, mute, brightness, Wi-Fi, Bluetooth, Focus, dark mode, Now Playing | Core Audio, WMI, `netsh`, WinRT Radios, GSMTC |
| `search.rs`, `files.rs`, `winindex.rs`, `clipboard.rs` | Search index (apps, files, Settings pages, actions), the background file walk, Windows Search results, clipboard history | `Get-StartApps`, `search-ms:`, clipboard sequence number |
| `switcher.rs`, `expose.rs`, `snap.rs`, `hotkeys.rs` | Alt+Tab switcher (low-level keyboard hook, opt-in), App Exposé (DWM thumbnails), snapping presets, Mac shortcuts | `WH_KEYBOARD_LL`, `DwmRegisterThumbnail`, `RegisterHotKey`, `SetWindowPos` |
| `stacks.rs` | Dock folder stacks | file system (your own folders only) |
| `appnames.rs`, `shellprops.rs` | Friendly app names; AppIDs, shortcut targets, taskbar pins | `FileDescription`, `IPropertyStore` |
| `journal.rs`, `registry.rs`, `tweaks/` | Change journal (previous value of everything Cove changes), HKCU access, the Quiet Windows tweaks | HKCU registry, `WM_SETTINGCHANGE` |
| `settings.rs`, `theme.rs`, `shell.rs` | Cove's own preferences (JSON), appearance, change broadcasts | none / `WM_SETTINGCHANGE` |
| `updater.rs` | Checks the latest GitHub release, downloads the setup, checks size, SHA-256 and the minisign signature from `signatures/`, installs silently | HTTPS (`ureq`) |

## Windows and pages

| Window label | Page | What it is |
|---|---|---|
| `main` | `ui/index.html` | Settings: Cove's own title bar, the live desktop miniature, six scenes, About, the tour |
| `intro` | `ui/intro.html` | Full-screen intro (shown only when the page signals `intro_ready`) |
| `dock` | `ui/dock.html` | The dock and its status island (transparent, no-activate) |
| `topbar` | `ui/topbar.html` | The optional menu bar |
| `popover` | `ui/popover.html` | Every menu, Control Centre, the calendar and confirmations (`?k=cove\|app\|cc\|…`) |
| `search` | `ui/search.html` | Cove Search |
| `switcher` | `ui/switcher.html` | Alt+Tab switcher |
| `expose` | `ui/expose.html` | App Exposé overlay |
| `traffic` | `ui/traffic.html` | The traffic-light patch, created at run time by `traffic.rs` |

Glass panels (dock, popovers, Search) each get native **backdrop windows**
from `glass.rs`, directly under the page's window. They are plain Win32
windows (no WebView) that never take the pointer or focus. Pages tell the core
their panel shapes with `glass_shape(label, shapes)`; CSS paints the corners,
tint (`--c-lg-*` in `ui/src/chrome.css`), rim and shadow. The menu bar still
uses the older wallpaper plate (`ui/src/glass.ts`). See
[design/GLASS_AND_APPLE_GUIDE.md](design/GLASS_AND_APPLE_GUIDE.md) §0b.

Data lives in `%APPDATA%\app.cove.desktop\`: `settings.json`, `journal.json`,
the AppBar record (`appbars.txt`), the launch marker (`launch.marker`) for safe mode, cached wallpapers and
`cove.log`.

## The setup rule

`AppState` (journal + settings) is **managed first** in `.setup()`, before
anything that can pump window messages (sent messages, broadcasts, window
creation). A page's command can run while setup waits on Windows, and every
command reads this state: managing it late crashed a fresh install at launch
(fixed in v0.9.0, PR #14).

On any path a page or a window event can reach, read state with
`app.try_state::<T>()` and return quietly (or with "Cove is still starting.")
when it isn't there yet. Never call `app.state()` there. `TrayChecks` follows
the same rule.

Setup order after that: release AppBars a crashed run still held, start the
tray module, spawn the watchdog copy, decide safe mode, then apply features.

## Performance budget
- About 0% CPU at idle: react to window events (`SetWinEventHook`) rather than polling where Windows allows it. Remaining polls are short and only while needed (the top-edge reveal while the menu bar is in that mode, the dock refresh).
- About 150 MB of memory total, one shared WebView2 environment.
- UI bundle size gate in CI (`scripts/check-bundle-size.mjs`).
- Animations: transform and opacity only, following Windows' "Animation effects" setting.

## Fallback
If idle memory can't stay within budget on Tauri, the UI moves to WinUI 3 and the Rust core stays.
