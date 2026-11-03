# Cove 1.0 redesign: Cove takes over the desktop

Decided with the user on 2026-11-03. Cove is no longer a set of extras beside Windows' own UI.
When **Cove desktop** is on, it replaces the shell surfaces you see; when it's off, Windows is exactly as before.

## What Cove replaces
| Windows | Cove | Notes |
| --- | --- | --- |
| Taskbar (all monitors) | Dock | Taskbar hidden with ShowWindow(SW_HIDE) on Shell_TrayWnd / Shell_SecondaryTrayWnd plus auto-hide so the work area is freed. Restored on off, quit, Restore, crash (watchdog) and re-applied on TaskbarCreated (Explorer restart). Same approach as lumabar / Seelen UI; no injection. |
| Taskbar pins | Dock pins | Imported from `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\*.lnk` the first time. |
| Start button | Launch tile (app grid + Cove Search) | Win key still opens Windows Start. |
| Clock, Wi-Fi, sound, battery | Menu bar | Control Centre panel for toggles and sliders. |
| Tray icons of other apps | Menu bar tray | Hardest piece; needs a safe, documented approach (investigate). |
| Settings window | "Your desktop is the settings" | Live miniature of the desktop; click the dock, menu bar, windows or wallpaper to edit that part. Scenes: Desktop, Dock, Menu bar, Windows, Keys, Quiet. |

## Design rules
- Ink #111215 for Cove's own windows; glass only where layers overlap (menu bar, dock, panels).
- No gradients, glow blobs, purple. Colour = teal #2cc9b9 accent + the logo's warm faces.
- Every control applies instantly; no Apply buttons, no checkbox+pill duplication.
- Springs, interruptible motion, respond on press (see apple-design skills in docs/design/README).

## Mockups
docs/design/gen.py renders s1 (desktop), s2 (Control Centre), s3 (settings) with headless Chromium.
Intro: ui/src/intro-core.ts (voxel C that pours in block by block, bounces; teal cube drops last), preview harness brand/intro-harness.html.
