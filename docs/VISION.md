# Cove: Vision

## What Cove is
One app that makes Windows feel calm and Mac-like to use, while keeping everything Windows can do. Cove has its own interface: it is not a skin, not a macOS clone, and not a pile of mods.

## Core principle
Smoothness and flexibility. Remove the UI nobody uses, and keep what's left fluid (fast, interruptible motion of about 150–250 ms, with Search on screen within 100 ms) and adaptable (presets, every toggle independent, everything reversible).

## Pillars
| Pillar | What it gives you | How (safely) |
|---|---|---|
| Mac habits | Alt+M minimize, Alt+W close window, Alt+Q quit app, Alt+H hide app, Alt+` cycle an app's windows, app-grouped switcher | `RegisterHotKey` + standard window messages (WM_CLOSE, ShowWindow) |
| Dock + top bar | Dock with magnification and running dots, top bar showing the active app and controls | AppBar API (`SHAppBarMessage`), with the taskbar set to auto-hide through its own setting |
| Cove Search | Quick keys, tabs for Apps / Files / Actions / Clipboard, local-only results | Own index + Windows Search APIs |
| Calm look | Cove Glass theme, wallpapers, *Calm* preset (hides Widgets, Task View, Copilot button, Start ads, lock-screen tips...) | Documented settings only, each journaled |

## Cove Tour
- First launch: a 60–120 second interactive tour that you learn by doing ("Press Alt+M now").
- Replay any time from Help, plus mini-tours on each pillar.
- Hold Alt to see a cheat sheet. An optional habit coach gives a few gentle tips, then stops.

## Brand
- Logo: an impossible isometric C (`brand/cove-logo.svg`). Palette: `#15171C`, `#4A5160`, `#AEB6C2`, accent `#3BA7FF`.
- UI font: Segoe UI Variable (native). Brand wordmark: Inter.
