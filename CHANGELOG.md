# Changelog

Each release's notes come from its section here, and Cove shows them in **Updates**.

## Unreleased
- **Liquid Glass desktop:** the menu bar (now 24 DIP) and the dock are glass made from your own wallpaper: a soft, frosted slice lined up with the screen, with a gentle lens at the edges. The bar never turns flat grey, and text on it always stays readable. Menus, Control Centre and Search are Windows acrylic with an Ink (or paper) tint and a soft rim, and open with a calm fade.
- **Graphite highlights:** the selected row, open title, "on" toggles, sliders, focus rings and the Search caret are now a soft frosted white (dark glass in light mode). No teal or logo colours for state.
- **Dock:** floats 6 DIP above the edge with 48 DIP icons and rounder corners, one divider before Downloads and the Recycle Bin, and real Windows icons (the Settings app's own logo, the Downloads folder, an empty or full Recycle Bin). Hovering magnifies gently (at most 1.25x, a small lift, smooth springs); labels fade in after a moment. Right-click menus are the same glass menus as the bar, with an Options submenu; the Recycle Bin can be emptied from its menu.
- **Menu bar menus:** on the desktop the bar shows Desktop, File, View, Go and Window; with an app in front, its name and Window. The Cove menu adds About This PC, System Settings, Recent Items, Force Quit and Sign Out, with real icons. Menus open right under the bar.
- **Control Centre:** Wi-Fi, Bluetooth, Focus and Dark mode with their real state, capsule sliders for Display and Sound (with the output's name), and Now Playing with play/pause, next and previous.
- **Search:** a 640 DIP Spotlight-style panel: clean names, the folder each file is in, its type (Word document, Python file…), real icons, and a short hint footer. Tab or a prefix ("f ", "s ", "c ") narrows it to files, settings or the clipboard.
- **Intro, every time it should:** Cove's intro now plays in its own full-screen window as Cove starts, before the menu bar and dock appear: the full build once per new version, a quick mark after that. Opening Cove again while it runs never replays it; **About › Play the intro** does. Settings opens by itself only on your first run.
- **The dock gives your apps the whole screen:** it no longer reserves the bottom of the screen. It slides down out of the way when a window touches it and comes back when you touch the bottom edge. Settings › Dock › **Hide the dock**: When a window touches it (default), Always or Never (Never keeps its space as before).
- **Menu bar:** a deliberate Ink bar (paper in light mode) over a soft blur of your wallpaper, instead of flat grey. Volume moved into Control Centre.
- **Menus and Control Centre:** the Cove menu no longer opens empty; sliders look like sliders; Control Centre's links are never cut off. The app menu adds Hide Others, Minimise, Zoom and Bring All to Front.
- **Wallpaper gallery:** Settings › Desktop now has a gallery: Cove, Anime, Comic, F1, Cars, Coding, Nature, Space and Minimal, each in dark and light. Photos come from Wikimedia Commons and NASA under free licences, with the photographer, licence and a link on hover. Cove's own art (Anime, Comic, Coding) is made for Cove. Nothing downloads until you pick one; it shows real progress, then stays cached. **Use my own wallpaper** sets any picture you choose. Live wallpapers are coming soon.

## v0.9.0
- **Launch intro:** the first time you open Cove, its C builds itself from small blocks and the name rises in. After that, a quick, quiet mark. Click or press any key to skip.
- **Dock motion:** apps bounce until their window opens, hop when they need you, and grow in or shrink away as they come and go. Reduce motion keeps the dock still.
- **Smoother start:** the settings window opens fully drawn, with no white or see-through flash.
- **Calm:** Apply shows how many you picked and switches each row to On right away.
- **Restore:** says how many Windows settings it put back and how many Cove features it turned off.
- **Updates:** shows the version you're getting, with readable notes.
- **Look:** Theme, Appearance and Accent controls always appear.

## v0.8.1
- **Signed updates:** every installer must match Cove's signature, or it is deleted and never run.

## v0.8.0
- **Safe mode:** hold Shift as Cove starts, use `--safe`, the tray or Restore. Features pause; settings stay.
