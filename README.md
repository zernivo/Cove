<p align="center"><img src="brand/cove-banner.png" alt="Cove: Windows, made calm." width="100%"></p>

<p align="center">
  <a href="https://github.com/zernivo/Cove/releases/latest"><b>⬇ Download for Windows 11</b></a> ·
  <a href="docs/ROADMAP.md">Roadmap</a> ·
  <a href="docs/SAFETY.md">Safety</a> ·
  <a href="docs/VISION.md">Vision</a>
</p>

<p align="center">
  <a href="https://github.com/zernivo/Cove/actions/workflows/ci.yml"><img src="https://github.com/zernivo/Cove/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="https://github.com/zernivo/Cove/releases/latest"><img src="https://img.shields.io/github/v/release/zernivo/Cove?label=download" alt="Latest release"></a>
  <img src="https://img.shields.io/badge/platform-Windows%2011-3BA7FF" alt="Windows 11">
</p>

**Cove** gives Windows 11 a calm, Mac-inspired way of working: smooth, flexible and uncluttered. It keeps every Windows feature, hides the clutter, and teaches Mac habits like Alt+W to close and Alt+H to hide an app. Everything it changes can be undone in one click.

> **Status: early preview (v0.1).** Nothing changes until you press **Apply**, and **Back to stock** puts everything back.

---

## Contents
- [Download](#download)
- [What it does today](#what-it-does-today)
- [Mac shortcuts](#mac-shortcuts)
- [Safety](#safety)
- [Roadmap](#roadmap)
- [Build from source](#build-from-source)
- [Project docs](#project-docs)

## Download
1. Open the [latest release](https://github.com/zernivo/Cove/releases/latest).
2. Download **`Cove_<version>_x64-setup.exe`** (or the `.msi`).
3. Run it. The installer isn't code-signed yet, so SmartScreen may warn you: click **More info → Run anyway**.

Requires Windows 11 (x64). No admin rights needed to use Cove's features.

## What it does today
| Area | What you get |
|---|---|
| **Calm** | 7 toggles that hide Task View, the taskbar search box, Widgets, the Copilot button, Start's Recommended section, lock-screen tips, and tips & suggestions. |
| **Cove Search** | Alt+Space opens a launcher: every app (desktop and Store), recent files, Settings pages, quick actions and a calculator. It learns what you open most. Fully local. |
| **Mac habits** | Five window shortcuts, opt-in, active only while Cove runs. |
| **Back to stock** | One click restores every setting Cove changed, exactly as it was. |
| **Updates** | Cove checks for new versions and updates itself in one click (installer verified by SHA-256). |

## Mac shortcuts
Alt sits where Cmd does on a Mac keyboard.

| Shortcut | Mac equivalent | Does |
|---|---|---|
| `Alt+W` | Cmd+W | Close the current window |
| `Alt+M` | Cmd+M | Minimize the current window |
| `Alt+H` | Cmd+H | Hide every window of the current app |
| `Alt+Q` | Cmd+Q | Quit the app (closes each window politely, so you can still save) |
| ``Alt+` `` | Cmd+` | Jump between windows of the same app |

Cove remembers whether shortcuts are on. Closing the window keeps Cove in the system tray so shortcuts keep working; quit from the tray icon. Cove never acts on the desktop or taskbar, and marks any shortcut another app already uses as **In use**.

## Cove Search
Press **Alt+Space** anywhere.

| Key | Does |
|---|---|
| Type | Instant results as you type (apps first, then files, settings) |
| `↑` `↓` | Move |
| `Enter` | Open (on a sum: copy the answer) |
| `Ctrl+Enter` | Show a file in its folder |
| `Tab` | Switch tab: All, Apps, Files, Settings |
| `Esc` | Clear, then close |

Try `blu` for Bluetooth, `vsc` for Visual Studio Code, or `128*4+7`. Turn it off in **Mac habits** if Alt+Space is taken (e.g. by PowerToys Run).

## Safety
- **Per-user only:** Cove writes documented settings under your own account (HKCU), never system-wide.
- **Journaled:** every change is saved with its previous value, so restore is exact.
- **Never:** system-file patching, code injected into Explorer, drivers, or disabling Defender or Windows Update.
- **No Apple assets:** no SF fonts, SF Symbols, wallpapers or traffic-light artwork.

Calm settings stay applied after you close Cove (like changing them in Settings) until you press **Back to stock**. Shortcuts stop the moment Cove closes. Full rules: [docs/SAFETY.md](docs/SAFETY.md).

## Roadmap
| Version | Milestone | Status |
|---|---|---|
| v0.1 | Calm preset, Mac shortcuts, restore, installer, updater | ✅ |
| v0.2 | Cove Search (Alt+Space) | ✅ |
| v0.3 | Dock + top bar | ⬜ |
| v0.4 | App-grouped switcher, App Exposé | ⬜ |
| v0.5 | Cove Glass theme, wallpaper gallery | ⬜ |
| v0.6 | Cove Tour (interactive tutorial) | ⬜ |
| 1.0 | Safe mode, code signing, auto-update, performance gates | ⬜ |

Details: [docs/ROADMAP.md](docs/ROADMAP.md)

## Build from source
Prerequisites: Windows 11, Rust (stable, MSVC), Node 20+, WebView2 runtime.
```bash
git clone https://github.com/zernivo/Cove.git
cd Cove
npm install
npm run tauri dev      # run in development
npm run tauri build    # build the installer
```
Stack: Rust core + Tauri 2 (WebView2), Mica for the main window. Budget: ~0% CPU idle, ~150 MB memory. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

**Releasing:** push a tag such as `v0.1.0` and GitHub Actions builds and publishes the installer.

## Project docs
- [Vision & pillars](docs/VISION.md)
- [Roadmap](docs/ROADMAP.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Safety rules](docs/SAFETY.md)
- [Research](docs/research)

## License
Not chosen yet. Until a license is added, all rights are reserved.
