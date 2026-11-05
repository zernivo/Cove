# Cove desktop redesign: Cove takes over the desktop

Decided with the user on 2026-11-03, refined after real-PC tests on 2026-11-04
and 2026-11-05, shipped in **v0.9.0**. Cove is no longer a set of extras beside
Windows' own UI. When **Cove desktop** is on, it replaces the shell surfaces
you see; when it's off, Windows is exactly as before.

## What Cove replaces (as shipped in v0.9.0)
| Windows | Cove | Notes |
| --- | --- | --- |
| Taskbar (all monitors) | Dock | Taskbar hidden with `ShowWindow(SW_HIDE)` on `Shell_TrayWnd` / `Shell_SecondaryTrayWnd` plus its documented auto-hide, so the work area is freed. Restored on off, quit, safe mode, Restore and by the watchdog after a crash; hidden again after an Explorer restart. No injection. |
| Taskbar pins | Dock pins | The dock starts with the apps pinned to the Windows taskbar. Apps group by AppID, like the taskbar. |
| Clock, Wi-Fi, sound, battery | Status island at the dock's right end | Search, Wi-Fi, sound, battery, Control Centre and a two-line clock. No magnification; graphite hover pill. |
| Quick Settings | Control Centre | Opens above the island. Wi-Fi, Bluetooth, Focus, Dark mode, Display, Sound, Now Playing. |
| Tray icons of other apps | Control Centre → Other apps, plus up to 4 pinned in the island | A stand-in tray window forwards every message to Explorer unchanged (see docs/SAFETY.md). "›" in the island opens Control Centre at Other apps. |
| Start's power and account items | The Cove menu: left-click the Cove dock icon | About This PC, System Settings, Cove Settings, Recent Items, Force Quit, Sleep, Restart, Shut Down, Lock Screen, Sign Out, Turn Cove Off. The Win key still opens Windows Start. |
| App menus | Each app's dock right-click menu | New Window, Show All Windows, Hide, Options, Quit. |
| Caption buttons | Mac traffic lights | On the focused window only, covering the native buttons in the title bar's colour. |
| Settings window | "Your desktop is the settings" | Live miniature of the desktop; click the dock, menu bar, windows or wallpaper to edit that part. Scenes: Desktop, Dock, Menu bar, Windows, Keys, Quiet. |

**No menu bar by default** (user decision 2026-11-05, after testing: Windows
users aren't used to a bar across the top). Settings → Menu bar: **Off**
(default), **Show at the top edge** (slides in when the pointer touches the
top 2 DIP, reserves nothing) or **Always** (24 DIP, reserved as an AppBar).
A full-screen app in front hides the dock and the bar.

**Dock:** 40 DIP icons, 56 DIP tall, floats 6 DIP, radius 15, 8 DIP gaps,
magnification at most 1.25x. Hides when a window touches it (or Always /
Never). Dock popovers stand 8 DIP above the dock, centred on their item.

Not built (still ideas): a launch tile / app grid in the dock, and a global
menu bar for every app (impossible for Chrome, Electron and Store apps).

## Design rules
- Ink #111215 for Cove's own windows; glass only where layers overlap (dock, island, menus, Control Centre, Search, the optional menu bar).
- No gradients, glow blobs, purple.
- Colour: teal #2cc9b9 and the logo's warm faces (#f7dccd / #c48b82 / #8c5a64) are **brand only**. Every selection, highlight, "on" state, slider fill and focus ring is **graphite**: a soft white glass with an edge sheen and white text (dark glass in light mode), in the Liquid Glass style (user pick, 2026-11-04).
- Glass on Windows: native blur-behind backdrops with a faint neutral tint (see GLASS_AND_APPLE_GUIDE.md §0b), never DWM's system backdrop and never CSS `backdrop-filter` over the desktop.
- Every control applies instantly; no Apply buttons, no checkbox+pill duplication. No Windows default title bar on Cove's windows.
- Springs, interruptible motion, respond on press (see GLASS_AND_APPLE_GUIDE.md §2.7).
- Real loading states (skeletons the size of the real thing).
- Show mockups before building major UI, and let the maintainer choose.

## Intro
Design B, "Blocks settle" (`ui/src/intro-core.ts`, `IntroScene`): a faint
isometric grid draws in, logo B's cubes drop in rows and settle (critically
damped), the teal cube last, then "Cove" and the tagline. Pre-rendered sprites,
time-based, smooth at 60 and 120 Hz. Full intro once per installed build; a
~1 s mark on later launches. Window page: `ui/intro.html`.

## Mockups
docs/design/gen.py renders concept screens with headless Chromium (they show
the 2026-11-03 concept, which had a top menu bar; the shipped design above
wins).
