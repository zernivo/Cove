<p align="center"><img src="brand/cove-banner.png" alt="Cove: Windows, made calm." width="100%"></p>

Cove gives Windows 11 a calm, Mac-inspired way of working: smooth, flexible and uncluttered. It keeps every Windows feature but presents them through its own clean interface, and it teaches Mac habits (Alt+M to minimize, Alt+W to close, Alt+Space to search).

> **Status:** v0.1.0, early development. Nothing changes until you press Apply, and Restore puts it all back.

## Core principle
**Smoothness and flexibility.** Strip out the parts of the Windows UI nobody uses, and make everything that's left fluid and adaptable. Every feature is judged against this.

## The four pillars
1. **Mac habits:** keyboard window control (hide, quit app, cycle an app's windows), an app-grouped switcher, App Exposé-style overview.
2. **Dock + top bar:** a Cove Dock (magnification, running dots, folder stacks) and a top bar (active app, controls, clock). The native taskbar is only auto-hidden through its own setting.
3. **Cove Search:** keyboard-first launcher with quick keys, tabs for Apps / Files / Actions / Clipboard, and instant local results.
4. **Calm look:** one theme (Cove Glass, light and dark), a wallpaper gallery, and a one-click *Calm* preset that hides clutter through documented settings.

Underneath all four: **Cove Tour**, an interactive tutorial you can replay, and **one-click restore to stock**.

## Safety model (non-negotiable)
- No admin rights for normal use. No system-file patching, no code injected into Explorer, no drivers.
- Only documented Windows settings and APIs. Every change is written to a journal with its previous value, so restore puts back exactly what was there.
- Quit Cove and Windows is back as it was. A safe mode (hold Esc at sign-in) skips all Cove changes.
- No Apple assets: no SF fonts, SF Symbols, wallpapers or traffic-light artwork.

See [docs/SAFETY.md](docs/SAFETY.md).

## Stack
Rust core + Tauri 2 shell (WebView2), with Mica for the main window and custom Cove Glass for floating surfaces. Budget: about 0% CPU at idle and roughly 150 MB of memory. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Develop (Windows 11)
Prerequisites: Rust (stable, MSVC), Node 20+, WebView2 runtime.
```bash
npm install
npm run tauri dev
```

## Roadmap
- [x] Research: macOS vs Windows, the tools landscape, Liquid Glass ([docs/research](docs/research))
- [x] Repo scaffold, change journal
- [x] Calm preset (7 toggles) with journaled one-click restore
- [x] Habit layer: Alt+W, Alt+M, Alt+H, Alt+Q, Alt+` (opt-in)
- [x] Settings window: Calm, Mac habits, Back to stock
- [ ] Cove Search
- [ ] Top bar + Dock (AppBar)
- [ ] Cove Tour (interactive onboarding)
- [ ] Wallpaper gallery (permissively licensed sources)

## Docs
- [Vision & pillars](docs/VISION.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Safety rules](docs/SAFETY.md)
- [Research](docs/research)
