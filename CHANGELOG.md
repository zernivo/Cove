# Changelog

Each release's notes come from its section here, and Cove shows them in **Updates**.

## v0.9.0
The first release where Cove takes over the desktop. Everything here is new or rebuilt since v0.8.1.

### The Cove desktop
- **Dock takeover:** one switch, **Cove desktop**, in Settings' title bar. On, the Windows taskbar is hidden completely (on every screen, not covered) and Cove's dock takes its place. Off, quitting Cove, safe mode or Restore everything brings the taskbar back exactly as it was. If Cove ever ends without tidying up, a watchdog brings it back.
- **Dock grouped like the taskbar:** apps group by their AppID, so Chrome profiles, Store apps and File Explorer windows sit where the taskbar would put them, with each profile's own icon. The dock starts with your taskbar pins.
- **A smaller, calmer dock:** 40-point icons, 56 points tall, floating 6 points above the edge, with real Windows icons (the Downloads folder, an empty or full Recycle Bin, Store apps' own logos), running dots, gentle magnification (at most 1.25x) and a bounce while an app opens. It slides away when a window touches it and comes back at the bottom edge (Settings → Dock → Hide the dock: When a window touches it, Always or Never). Full-screen apps hide the dock and the menu bar.
- **Status island:** the right end of the dock holds up to four of your other apps' tray icons, Search, Wi-Fi, sound, battery, Control Centre and a two-line clock. **›** opens Control Centre at Other apps. Its popovers open just above the dock.
- **The Cove menu from the dock:** click the Cove icon for About This PC, System Settings, Cove Settings, Recent Items, Force Quit, Sleep, Restart, Shut Down, Lock Screen, Sign Out and Turn Cove Off. Right-click a running app for its menu: New Window, Show All Windows, Hide, Options (Keep in Dock, Open at Login, Show in File Explorer) and Quit.
- **Menu bar modes:** no menu bar by default, so maximised apps get the whole screen. Settings → Menu bar: **Off**, **Show at the top edge** (slides in when the pointer touches the top and takes no space) or **Always** (24 points, maximised apps stop below it). Settings from older versions start at Off.

### Liquid Glass
- **The Liquid Glass redesign:** the dock, menus, Control Centre and Search are glass panels with a soft bright rim, rounded corners and a soft shadow. Selected rows, "on" toggles, sliders and focus rings are **graphite**, a soft white glass (dark glass in light mode), never teal or the logo's colours.
- **Clear blur glass:** each glass panel sits on a clear blur of whatever is behind it, with only a faint neutral tint. It stays glass when the panel isn't the active window (no more flat grey slab or square grey corners). With Windows' transparency effects off, the glass becomes a solid surface of the same shape.

### Control Centre
- Wi-Fi, Bluetooth, Focus and Dark mode with their real state; Display (brightness, Night light) and Sound sliders with the output's name; Now Playing with play/pause, next and previous.
- **Other apps:** the tray icons of your other apps (Discord, Steam, OneDrive, a VPN…), with their real icons and names. Clicks, right-clicks and double-clicks reach the app as they would on the taskbar. Cove gets them through a stand-in tray that forwards every message to Explorer unchanged, so nothing is injected and Windows keeps every icon. **Windows tray** opens Windows' own overflow.

### Windows
- **Mac traffic lights:** red, yellow and green buttons on the focused window's title bar. They cover Windows' own caption buttons, painted in the title bar's colour; each click is the window's own close, minimise or zoom, and resting on green opens Snap Layouts. Background windows keep their Windows buttons. Elevated, full-screen and High Contrast windows are skipped. One switch in Settings → Windows, plus a list of apps to skip.

### Start-up, wallpapers and Settings
- **Intro B, "Blocks settle":** a faint grid draws in, the Cove C's blocks drop and settle, then the name and tagline. It plays in its own full-screen window once per new version, smooth at 60 and 120 Hz; later launches get a quick mark. Click or Esc skips; reduced motion shows a calm fade.
- **Wallpaper gallery:** Settings → Desktop: Cove, Anime, Comic, F1, Cars, Coding, Nature, Space and Minimal, each in dark and light. Photos come from Wikimedia Commons and NASA under free licences, credited on hover with a link. Nothing downloads until you pick one; it shows real progress, then stays cached. **Use my own wallpaper** and **Put back my old wallpaper** are there too.
- **Settings redesign:** your desktop is the settings. A live miniature of your desktop you click on, six scenes (Desktop, Dock, Menu bar, Windows, Keys, Quiet), controls that apply at once, Cove's own title bar and time fields, and no blank flashes between scenes.
- **Accessibility:** a graphite focus ring on everything you can reach by keyboard, one Tab stop per segmented choice, menus, switches and sliders that Narrator reads by role and state, Search announced as a combo box, reduced motion and reduced transparency honoured everywhere, contrast of at least 4.5:1 and click targets of at least 32 points.

### Fixes and project
- **Launch crash fixed:** a fresh install could crash at launch because a page read Cove's state before setup had stored it. State is now stored first, and every path a page can reach checks for it.
- **MIT licence,** third-party notices, a contributing guide and a UI bundle size gate in CI.
- **Smoke test:** a manual workflow builds the installer, installs it silently on a Windows runner, checks Cove stays up for 30 seconds and screenshots the desktop, Control Centre and menus in dark and light.

### Known issues
- **Not code-signed:** SmartScreen may warn on install. Click More info, then Run anyway.
- **Unverified on many PCs:** the blur, high-DPI and multi-screen layouts and clicks on other apps' tray icons have been checked on few machines. Please report anything odd.
- **Light mode:** some apps' white tray icons are hard to see on light glass.
- **Live wallpapers** are off for now (shown as coming soon).
- **Control Centre** can take a few seconds to open the first time.

## v0.8.1
- **Signed updates:** every installer must match Cove's signature, or it is deleted and never run.

## v0.8.0
- **Safe mode:** hold Shift as Cove starts, use `--safe`, the tray or Restore. Features pause; settings stay.
