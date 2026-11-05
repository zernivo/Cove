<h1 align="center">Cove</h1>
<p align="center"><b>Windows, made calm.</b></p>

<p align="center"><img src="docs/images/hero.png" alt="The Cove desktop on Windows 11: a Liquid Glass dock with its status island at the right end, and Control Centre open above it" width="100%"></p>

<p align="center">
  <a href="https://github.com/zernivo/Cove/releases/tag/v0.9.0"><b>Download Cove 0.9.0 for Windows 11</b></a> ·
  <a href="#safety">Safety</a> ·
  <a href="docs/TESTING.md">Test guide</a> ·
  <a href="docs/ROADMAP.md">Roadmap</a> ·
  <a href="CONTRIBUTING.md">Contributing</a>
</p>

<p align="center">
  <a href="https://github.com/zernivo/Cove/actions/workflows/ci.yml"><img src="https://github.com/zernivo/Cove/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/zernivo/Cove/releases/latest"><img src="https://img.shields.io/github/v/release/zernivo/Cove?label=download" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2011-111215" alt="Windows 11">
  <a href="LICENSE"><img src="https://img.shields.io/badge/licence-MIT-111215" alt="MIT licence"></a>
</p>

Cove gives Windows 11 a calm, Mac-style desktop. Turn on the **Cove desktop**
and a Liquid Glass dock replaces the taskbar, with Wi-Fi, sound, battery,
Control Centre and the clock in a status island at its right end. Search opens
with Alt+Space, and the focused window gets Mac-style window buttons. Nothing
sits across the top of the screen unless you ask for a menu bar. Turn Cove off
and Windows comes back exactly as it was: Cove only uses documented per-user
settings and records every change it makes.

Cove is an early preview. Version 0.9.0 is the first release with the full
desktop takeover.

## Features (v0.9.0)

| | |
|---|---|
| **The Cove desktop** | One switch in Settings. On: the Windows taskbar is hidden completely (not covered), and Cove's dock takes its place. Off, quitting Cove or Restore everything: the taskbar comes back exactly as it was. |
| **Dock** | Pinned and running apps with their real icons, grouped by AppID like the taskbar (so Chrome profiles and Store apps stay separate), running dots, gentle magnification (at most 1.25x), a bounce while an app opens, folder stacks and the Recycle Bin. It starts with your taskbar pins. It slides away when a window touches it and comes back at the bottom edge (or choose Always or Never), and full-screen apps hide it. |
| **Status island** | The right end of the dock: up to four of your other apps' tray icons, Search, Wi-Fi, sound, battery, Control Centre and a two-line clock. **›** opens Control Centre at Other apps. |
| **Cove menu and app menus** | Click the Cove icon in the dock for the Cove menu: About This PC, System Settings, Cove Settings, Recent Items, Force Quit, Sleep, Restart, Shut Down, Lock Screen, Sign Out and Turn Cove Off (Restart, Shut Down and Sign Out ask first). Right-click a running app for its menu: New Window, Show All Windows, Hide, Options, Quit. |
| **Menu bar, if you want one** | Off by default, so maximised apps get the whole screen. Settings → Menu bar: **Show at the top edge** (slides in when the pointer touches the top, takes no space) or **Always** (24 points tall; maximised apps stop below it). |
| **Liquid Glass** | The dock, menus, Control Centre and Search sit on a clear blur of what is behind them, with a faint neutral tint and a soft bright rim. The glass stays glass when the panel isn't the active window. Highlights and "on" states are graphite, a soft white glass, never a colour. |
| **Control Centre** | Wi-Fi, Bluetooth, Focus and Dark mode with their real state, Display (brightness, Night light) and Sound sliders with the output's name, Now Playing with media buttons, and **Other apps**: the tray icons of your other apps, so nothing is lost without a taskbar. |
| **Mac window buttons** | Red, yellow and green buttons on the focused window's title bar, covering Windows' own caption buttons, in the title bar's colour. Each click is the window's own close, minimise or zoom; resting on green opens Snap Layouts. Background windows keep their Windows buttons. One switch in Settings → Windows, plus a per-app skip list. |
| **Spotlight-style Search** | Alt+Space from anywhere: apps, files (including words inside documents, through Windows' own index), Settings pages, quick actions, a calculator and clipboard history. Fully local. |
| **App Exposé and switcher** | Alt+Tab shows one icon per app (opt-in). Press ↓, or right-click an app in the dock → Show All Windows, to see all of its windows side by side. |
| **Snapping** | Ctrl+Alt presets for halves, thirds, quarters, two-thirds, centre and fill, and moving to another screen. Windows glide into place. |
| **Wallpaper gallery** | Cove's own art plus free-licence photos from Wikimedia Commons and NASA in nine categories, each credited on hover, downloaded only when you pick one. Use your own picture, put back your old wallpaper, or switch light and dark on a schedule. |
| **Settings** | Your desktop is the settings: a live miniature of your desktop you click on, six scenes (Desktop, Dock, Menu bar, Windows, Keys, Quiet) and controls that apply at once. Keyboard, Narrator, reduced motion and reduced transparency are supported throughout. |
| **Intro and tour** | A short "blocks settle" intro the first time a new version starts, then a tour you learn by doing. Replay both from About. |
| **Mac shortcuts** | Alt+W closes, Alt+M minimises, Alt+H hides, Alt+Q quits and Alt+` cycles an app's windows. |
| **Quiet Windows** | Hide Task View, the search box, Copilot, Start's Recommended section, lock-screen tips and suggestions, one switch each. |
| **Safe mode** | Hold Shift while Cove starts (or run `Cove.exe --safe`) to pause everything without losing your settings. Cove also starts in safe mode by itself after two launches in a row that didn't last 30 seconds. |
| **Signed updates** | One click in About. Cove checks each update's SHA-256 and its signature against Cove's own update key before installing it. |

<p align="center">
  <img src="docs/images/cc.png" alt="Control Centre: Wi-Fi, Bluetooth, Focus and dark mode tiles, display and sound sliders, the track that's playing and other apps' tray icons" width="49%">
  <img src="docs/images/search.png" alt="Cove Search: a 640-wide glass field with app, file and setting results" width="49%">
</p>

## Install

1. Open the [Cove 0.9.0 release](https://github.com/zernivo/Cove/releases/tag/v0.9.0)
   (or the [latest release](https://github.com/zernivo/Cove/releases/latest)).
2. Download **`Cove_0.9.0_x64-setup.exe`**. An MSI (`Cove_0.9.0_x64_en-US.msi`)
   is there too, for managed installs.
3. Run it. The setup installs for your account only, with no admin rights.
4. Cove opens with its intro and tour. Turn on the **Cove desktop** switch at
   the top of Settings when you're ready.

**SmartScreen:** the installer is not code-signed yet, so Windows may say it
protected your PC. Click **More info**, then **Run anyway**. Code signing is
planned (see the [roadmap](docs/ROADMAP.md)); a winget package will follow it
(draft in [`packaging/winget`](packaging/winget)).

Already on an older Cove? About → **Check for updates** installs 0.9.0 in place.

Requires Windows 11 (x64).

## Turning things off and putting Windows back

Every part of Cove is its own switch, and nothing stays behind.

- **The Cove desktop off** (the **Cove desktop** switch in Settings' title bar,
  or Cove menu → **Turn Cove Off**): the dock, status island and menu bar go,
  and the taskbar returns with its own position and auto-hide setting.
- **Quit Cove** (tray → Quit Cove): the same, plus every shortcut, hook and overlay
  ends with the app.
- **Restore everything** (Settings → Quiet): turns off what Cove turned on and
  puts back every Windows setting Cove changed, including your old wallpaper,
  and tells you what it restored.
- **Safe mode:** hold **Shift** while Cove starts, run `Cove.exe --safe`, or use
  the switch in Settings → Quiet or the tray. Everything pauses; your settings
  are kept. **Leave safe mode** brings it all back without a restart.
- **Uninstall** from Windows Settings → Apps like any other app.

If Cove ever ends without tidying up, a small watchdog copy of Cove brings the
taskbar back, releases the screen space Cove reserved and hands other apps'
tray icons back to Windows.

## Safety

These are promises, not goals. A change that breaks one doesn't get merged.

- **No injection, no patching.** Cove never injects code into Explorer or any
  other process, never patches system files or themes, and installs no drivers.
  Other apps' tray icons come through a stand-in tray window that forwards
  every message to Explorer unchanged; the window buttons are Cove's own small
  window over the title bar, not a change to the app.
- **Documented, per-user settings only.** Cove changes what Windows Settings
  itself can change, for your account only. If something can't be changed
  safely, Cove opens the right Settings page instead.
- **Every change is journaled** with its previous value, so Restore is exact.
- **Fully reversible.** Turning the Cove desktop off, quitting, or **Restore
  everything** puts Windows back the way it was.
- **Hands off protections.** Defender, SmartScreen, UAC and Windows Update are
  never touched. Cove never asks for admin rights.
- **Private.** No account and no telemetry. Cove only goes online for the
  wallpaper gallery and update checks.
- **No Apple assets.** No Apple fonts, symbols, wallpapers or artwork.

Full rules and how each feature keeps them: [docs/SAFETY.md](docs/SAFETY.md).

## Testing

[docs/TESTING.md](docs/TESTING.md) walks through every feature of 0.9.0: where
it is, how to try it and what you should see. Reports with a screenshot or a
short clip are the most useful.

## Building from source

Prerequisites: Windows 11, Rust stable (MSVC), Node 20+, WebView2 (built into Windows 11).

```bash
git clone https://github.com/zernivo/Cove.git
cd Cove
npm install
npm run tauri dev      # run from source
npm run tauri build    # build the NSIS and MSI installers
```

Before pushing, run `bash notes/windows-check.sh`, `npx tsc --noEmit`,
`npx vite build` and `npm run check:size` (the UI bundle size gate CI also
enforces). Previews, the smoke and visual check, design rules and safety rules
are in [CONTRIBUTING.md](CONTRIBUTING.md).

Stack: a Rust core on Tauri 2 (WebView2) with a TypeScript UI. See
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Project docs

- [Vision](docs/VISION.md) · [Roadmap](docs/ROADMAP.md) · [Architecture](docs/ARCHITECTURE.md) · [Safety](docs/SAFETY.md) · [Testing](docs/TESTING.md)
- [Glass and Apple design guide](docs/design/GLASS_AND_APPLE_GUIDE.md) · [Desktop redesign](docs/design/REDESIGN.md)
- [Changelog](CHANGELOG.md) · [Third-party notices](THIRD_PARTY_NOTICES.md)

## Licence

[MIT](LICENSE) © 2026 Khubaib Nazeer (zernivo). Cove builds on open-source
software listed in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
Cove is not affiliated with Apple or Microsoft.
