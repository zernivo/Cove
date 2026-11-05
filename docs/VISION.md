# Cove: Vision

## What Cove is
One app that makes Windows feel calm and Mac-like to use, while keeping everything Windows can do. Cove has its own interface: it is not a skin, not a macOS clone, and not a pile of mods.

## Core principle
Smoothness and flexibility. Remove the UI nobody uses, and keep what's left fluid (fast, interruptible motion of about 150–250 ms, with Search on screen within 100 ms) and adaptable (presets, every toggle independent, everything reversible).

## Pillars
| Pillar | What it gives you | How (safely) |
|---|---|---|
| Mac habits | Alt+M minimize, Alt+W close window, Alt+Q quit app, Alt+H hide app, Alt+` cycle an app's windows, app-grouped switcher | `RegisterHotKey` + standard window messages (WM_CLOSE, ShowWindow) |
| The Cove desktop | A dock that replaces the taskbar, with a status island, Control Centre, other apps' tray icons and an optional menu bar | The taskbar hidden through `ShowWindow` and its own auto-hide setting, AppBars (`SHAppBarMessage`) only where space is reserved, a stand-in tray that forwards to Explorer; all restored on off, quit and by a watchdog |
| Cove Search | Quick keys, tabs for Apps / Files / Actions / Clipboard, local-only results | Own index + Windows Search APIs |
| Calm look | Liquid Glass, graphite highlights, the wallpaper gallery, *Quiet* Windows switches (Task View, Copilot button, Start ads, lock-screen tips...) | Documented settings only, each journaled |

## Cove Tour
- First launch: a 60–120 second interactive tour that you learn by doing ("Press Alt+M now").
- Replay any time from About. (Ideas not built yet: mini-tours on each pillar, a hold-Alt cheat sheet, a gentle habit coach.)

## Brand
- Logo B: a sharp-edged voxel C with a teal cube (`brand/voxel_logo.py`, `brand/logo_b.py`). Cove's windows sit on Ink `#111215`; brand accent teal `#2cc9b9`, logo faces `#f7dccd` / `#c48b82` / `#8c5a64`. Interface state (selection, "on", focus) is graphite, never a brand colour.
- UI font: Segoe UI Variable (native). Brand wordmark: Inter.
