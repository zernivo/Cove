# Cove v0.9.0 test guide

Every feature in v0.9.0: where it is, how to try it, what you should see.
Written for the v0.9.0 release (commit f99364d and later): no menu bar by
default, a status island in the dock, the Cove menu on the Cove dock icon, the
Settings "desktop miniature", graphite highlights, clear Liquid Glass and Mac
traffic lights on the focused window.

Mark each section **Pass / Fail** and add a screenshot or short clip of anything
odd. Times are rough targets, not hard limits.

Developers: the automated smoke and visual check is described in
[CONTRIBUTING.md](../CONTRIBUTING.md#smoke-and-visual-check). It can't see the
blur (the CI runner has none), so the glass sections here need a real PC.

## 0. Before you start
- **Install:** download `Cove_0.9.0_x64-setup.exe` from the
  [v0.9.0 release](https://github.com/zernivo/Cove/releases/tag/v0.9.0) and run it
  (or update in place: About → Check for updates).
  SmartScreen may warn because the installer isn't code-signed yet: **More info → Run anyway**.
- **Check the version:** open Cove Settings. The button at the top right of the
  title bar reads **Cove 0.9.0** (never "…"). Click it to open **About**.
- **Have ready:** 3+ apps open (one with 2 windows, e.g. two File Explorer
  windows), a laptop or a second screen if you have one, a password manager.
- **Fresh-start tip:** to see the first-run intro and tour again, use About →
  **Play the intro** and **Take the tour**.

---

## A. First run

### 1. Intro
- **Where:** first launch after install, or About → **Play the intro**.
- **How:** watch it through once, then replay and click or press Esc halfway.
- **Expected:** a full-screen Ink intro where the Cove "C" assembles from blocks
  that settle smoothly ("blocks settle"), then fades to the desktop. No stutter,
  no white or transparent flash at start or end, no Windows title bar. Click or
  Esc skips at once. With **Settings → Accessibility → Visual effects →
  Animation effects** off, it's a short calm fade instead of motion.

### 2. Cove Tour
- **Where:** opens after the intro on first run; replay from About → **Take the tour**.
- **How:** pick **Mac-like**, press **Start**, then try each step (Alt+Space,
  Alt+M on another window, Ctrl+Alt+←, Alt+Tab, ↓ in the switcher).
- **Expected:**
  - Three presets: Mac-like, Light touch, Keep things as they are. The picked one
    has a graphite outline.
  - Each step ticks itself off ("✓ Nice, that's it.") and moves on after ~1 s.
    A feature that's off shows **Turn it on**.
  - **Skip tour** is a full-size button (at least 32 px tall; easy to hit).
    Skip or Esc closes the tour and it doesn't come back on next launch.
  - Keyboard: Tab stays inside the tour card; after closing, focus returns to
    where you were.
  - Restore everything undoes what the tour turned on.

---

## B. The Cove desktop

### 3. Turn the Cove desktop on and off
- **Where:** Settings → the **Cove desktop** switch in the middle of the title bar.
- **How:** turn it On, use the PC for a minute, turn it Off. Then On again and
  **Quit Cove** from the tray.
- **Expected:**
  - On: the Windows taskbar is completely gone (hidden, not covered) and the Cove
    dock appears at the bottom. Nothing appears at the top of the screen by default.
  - Off, or Quit: the taskbar comes back exactly as it was (same position,
    auto-hide setting, icons). No leftover gap at the bottom or top.
  - Toast: "Cove desktop is on." / "Back to the Windows taskbar."

### 4. Dock basics
- **Where:** bottom centre of the screen (Cove desktop on).
- **How:** click an app that isn't open; click the app in front; click it again;
  move the pointer slowly along the dock.
- **Expected:**
  - A smaller dock than before (about 40 px icons, ~56 px tall) with real app
    icons: pinned apps, a divider, open apps, folder stacks and the Recycle Bin.
  - Opening an app: a short bounce until its window appears. A dot under each open app.
  - Clicking the app in front minimises it; clicking again brings it back.
  - Gentle magnification near the pointer with a name label above; everything
    settles smoothly when you leave. No icon flicker, no letter placeholders.

### 5. Liquid Glass
- **Where:** the dock, its menus, Control Centre and Search, over different
  wallpapers (Settings → Desktop) and over app windows.
- **How:** switch between a dark, a light and a busy wallpaper; drag a window
  under the dock (Hide the dock: Never) and open Control Centre over it; click
  another app so the panel isn't the active window.
- **Expected:** clear glass: a plain blur of whatever is behind the panel
  (wallpaper or app) with only a faint neutral tint, a soft bright rim and
  round corners. It stays glass when another window is active: no flat grey
  slab, no milky haze, no square grey corners poking out. No purple or teal
  tint. With **Settings → Accessibility → Visual effects → Transparency
  effects** off, the glass becomes a solid Ink (or paper) surface with the
  same shape.

### 6. Dock hides when a window touches it
- **Where:** Settings → **Dock** → Hide the dock: *When a window touches it* (default), *Always*, *Never*.
- **How:** maximise a window; move the pointer to the bottom edge; try each option.
- **Expected:** *When a window touches it*: the dock slides away when a window
  overlaps it and returns when you touch the bottom edge. *Always*: hidden until
  you touch the edge. *Never*: always visible and maximised windows stop above it.
  Maximised and full-screen apps get the whole screen.

### 7. Status island
- **Where:** the right end of the dock, after the divider.
- **How:** look at it; click each part.
- **Expected:** up to four of your other apps' tray icons (in their own
  colours) and a **›**, then Search, Wi-Fi, sound, battery (laptops), Control
  Centre and the two-line clock/date, in the same glass as the dock. No
  magnification here; a graphite pill on hover. **›** opens Control Centre at
  Other apps. Search opens Cove Search. Wi-Fi and
  sound reflect the real state (muted shows a crossed speaker). Battery shows
  charging. The clock shows the right time and updates each minute; clicking it
  opens a calendar. Control Centre and the other popovers open just above the
  dock, centred on what you clicked.

### 8. Cove menu and app menus from the dock
- **Where:** click the **Cove** icon in the dock; right-click any running app.
- **Expected:**
  - Cove menu: About This PC, System Settings…, Cove Settings…, Recent Items ›,
    Force Quit…, Sleep, Restart…, Shut Down…, Lock Screen, Sign Out… and Turn
    Cove Off (Restart, Shut Down and Sign Out ask first). Graphite highlight on
    hover, never teal.
  - App menu (right-click a running app): New Window where the app supports it,
    Show All Windows, Hide, Options › (Keep in Dock, Open at Login, Show in File
    Explorer), Quit. Keyboard shortcuts shown on the right.
  - Keyboard: ↑/↓ move, Home/End jump, → opens a submenu, ← closes it, Enter
    picks, Esc closes. Clicking elsewhere closes the menu.

### 9. Optional menu bar
- **Where:** Settings → **Menu bar** → *Off* (default) / *Show at the top edge* / *Always*.
- **How:** try each option; maximise a window with the bar on.
- **Expected:**
  - Off: nothing at the top of the screen.
  - Show at the top edge: a thin bar slides down when the pointer touches the
    top edge and hides again when you leave.
  - Always: the bar stays (24 points), with the app in front, its menus (File,
    View, Go, Window) and the status icons, and maximised windows stop below it.
    A full-screen app (a video, a game) hides it and the dock.
  - The bar is an Ink bar over a soft blur of your wallpaper, not yet the same
    clear glass as the dock.

### 10. Folder stacks
- **Where:** the Downloads icon near the Recycle Bin; Settings → Dock → **Folders**.
- **How:** click Downloads; Ctrl+click a file; toggle Documents/Desktop/Pictures.
- **Expected:** a grid of the newest files with real icons; click opens, Ctrl+click
  shows it in File Explorer; Esc closes. Toggled folders appear and disappear in
  the dock. The Folders chips are toggle buttons (several can be on).

### 11. Pin and unpin
- **How A:** Alt+Space, type an app's name, press **Ctrl+D** → "Added to Dock", it
  appears among the pins within ~2 s.
- **How B:** right-click a running app → Options → **Keep in Dock**.
- **How C:** right-click a pin → Options → untick **Keep in Dock**.
- **Expected:** the dock and Settings → Dock → **Pinned apps** match.

---

## C. Search, Control Centre, windows

### 12. Spotlight-style Search
- **Where:** anywhere, press **Alt+Space** (or the Search icon in the status island).
- **How:** type `blu`, `vsc`, `128*4+7`, part of a file name, a word inside a
  document; use ↑/↓, Enter, Ctrl+Enter, Tab, Esc.
- **Expected:** a 640-wide glass panel ~20% down the screen. Results as you type:
  apps first with real icons, then files (folder shown in grey), Settings pages,
  quick actions; sums show the answer (Enter copies it). Selection is a graphite
  pill. Tab cycles All / Apps / Files / Settings / Clipboard. Esc clears, then
  closes. Opens in under ~100 ms after the first time (Settings → Keys shows the
  measured time).

### 13. Search inside files and clipboard history
- **Where:** Settings → **Keys** → Search inside files; Clipboard history.
- **How:** make a .txt in Documents containing a unique word, wait a minute, search
  that word. Copy three bits of text, then Alt+Space → Tab to **Clipboard** →
  Enter on the second.
- **Expected:** the file appears marked as found by Windows Search. Clipboard:
  all three, newest first; Enter pastes into the app you were in, Ctrl+Enter only
  copies. Password-manager copies never appear. Quit Cove → the list is empty.

### 14. Control Centre
- **Where:** the Control Centre icon in the status island (or the menu bar, if on).
- **How:** drag Sound and Display; tap Dark mode; mute; play music and use the
  media buttons; click Wi-Fi, Bluetooth, Focus, Night light.
- **Expected:** a glass panel with Wi-Fi (network name), Bluetooth (on/devices),
  Focus, Dark mode tiles, a Display slider (screens Windows can dim; others show a
  note), Sound with the output device and mute, and Now Playing when media plays.
  Sliders change things live; Dark mode switches Windows' theme within ~1 s; the
  Wi-Fi/Bluetooth/Focus/Night light tiles open the matching Windows settings.
  Keyboard: Tab moves through every control with a visible graphite ring; ←/→ move
  the sliders. Click outside closes it.

### 15. Other apps' tray icons
- **Where:** Control Centre → **Other apps**, and the pinned icons in the status
  island; Settings → Dock → Show other apps' tray icons.
- **How:** with Cove desktop on (no taskbar), open apps that live in the
  notification area (Discord, OneDrive, Steam, a VPN). Click, right-click and
  double-click their icons.
- **Expected:** each app's tray icon appears in Control Centre with its real icon
  and tooltip name. Clicks do what they do on the taskbar (open the app, show its
  own menu). Icons appear and disappear as apps start and quit. If no app has a
  tray icon, a short calm note says so. **Windows tray** opens Windows' own
  overflow. Turn the Cove desktop off: every icon is back in Windows' tray.
  (Known issue: some white icons are hard to see in light mode.)

### 16. Mac window buttons (traffic lights)
- **Where:** any normal app window's title bar (Settings → **Windows** → Mac
  window buttons, with a list of apps to skip).
- **How:** click between Notepad, File Explorer, Chrome or Edge and an Office
  app; click each button; rest the pointer on green; try a maximised window, a
  window on a second screen and an admin (elevated) window.
- **Expected:** the **focused** window shows red, yellow and green buttons at
  the right end of its title bar, always (no hover needed), on a patch in the
  title bar's own colour that covers Windows' caption buttons exactly. Red
  closes (unsaved work still asks), yellow minimises, green maximises and
  restores; resting on green (or Alt+click) opens Snap Layouts. Background
  windows keep their normal Windows buttons. The patch hides while a window
  moves and comes back when it settles. Nothing appears over full-screen apps,
  games, elevated windows or in High Contrast. Off in Settings → the Windows
  buttons are back everywhere.

### 17. App-grouped switcher
- **Where:** Settings → **Windows** → App switcher.
- **How:** hold Alt, tap Tab; Shift+Tab / ←; ↓; Esc; release Alt.
- **Expected:** one icon per app, most recent first, a count badge on apps with
  several windows. Release Alt switches; a quick Alt+Tab tap jumps to the previous
  app with no panel. Esc cancels. Off → Windows' own Alt+Tab returns.

### 18. App Exposé
- **Where:** right-click a running app in the dock → **Show All Windows**, or ↓ in the switcher.
- **Expected:** the screen dims, live previews of each of that app's windows
  (minimised ones listed underneath). Click or arrows + Enter brings one forward;
  Esc or a click on the background goes back. Correct at 125–200% scaling.

### 19. Snapping
- **Where:** Settings → **Windows** → Snap windows, Glide into place.
- **How:** focus Notepad, then Ctrl+Alt+← (again, again), →, ↑, ↓, U/I/J/K, D/F/G,
  E/T, Enter, C, Backspace, Ctrl+Alt+Shift+→.
- **Expected:** halves (then ⅔, ⅓), quarters, thirds, two-thirds, fill, centre,
  back to where it was, and move to the next screen. Windows glide in ~0.2 s and
  fill edge to edge with no gaps; Glide off (or Windows animation effects off) →
  instant. "In use" shows for shortcuts another app owns.

### 20. Mac shortcuts
- **Where:** Settings → **Keys** → Window shortcuts.
- **How:** in another app: Alt+W, Alt+M, Alt+H, Alt+Q, Alt+`.
- **Expected:** close window, minimise, hide the app, quit politely (unsaved
  documents still ask), cycle the app's windows. They stop the moment Cove quits.

---

## D. Settings window

### 21. Settings layout and the desktop miniature
- **Where:** tray → Open Cove (or Cove menu → Cove Settings…).
- **Expected:** Cove's own title bar (no Windows caption), a live miniature of your
  desktop on the left (your real wallpaper and dock icons), the settings for the
  chosen scene on the right, and the scene bar at the bottom: Desktop, Dock, Menu
  bar, Windows, Keys, Quiet. Clicking a part of the miniature opens its scene.
  Every control applies at once (no Apply button). The window never shows the
  desktop through it on first paint.

### 22. No flashes between scenes or on the Dock scene
- **How:** click through every scene quickly, twice. Close and reopen Settings.
  Open the Dock scene right after starting Cove.
- **Expected:** switching scenes never shows a blank page or a fade from empty;
  content settles a few pixels into place. Coming back to Desktop doesn't reload
  the wallpaper thumbnails. Reopening Settings shows the scene you left (no flash
  of Desktop first). The Dock scene and the miniature dock show grey skeleton
  tiles the size of icons until the icons are ready, then all icons appear at
  once; refocusing the window doesn't make them blink.

### 23. Desktop scene: wallpaper gallery and schedule
- **Where:** Settings → **Desktop**.
- **How:** pick a category, then a Cove wallpaper, then a photo; hover a photo;
  use **Put back my old wallpaper**, **Use my own wallpaper**; turn the schedule On
  and set times a minute or two ahead.
- **Expected:**
  - Cove art applies within ~1 s; photos show a real download progress bar, then
    apply. Hover shows the title, author, licence and a source link.
  - "Live" is shown as coming soon and can't be picked.
  - Keyboard: the category chips are one Tab stop (←/→ to move); the wallpaper
    grid is one Tab stop (arrow keys move, Enter or Space picks) with a graphite ring.
  - The schedule time fields are Cove's own (Ink field, graphite pill on the part
    you're editing, small ▲▼ stepper): click a part and press ↑/↓ or type digits;
    ←/→ move between hour, minutes and AM/PM. The change saves when you stop.
    Cove and the wallpaper switch within ~60 s of each time.

### 24. Quiet scene
- **Where:** Settings → **Quiet**.
- **How:** turn on a couple of tweaks; look at the Widgets row; press **Restore everything**.
- **Expected:**
  - Each tweak has one switch. A tweak that's done shows a **✓ Done** label and
    "Restore everything puts it back" (no greyed-out ticked switch).
  - **Hide the Widgets button** shows **Coming later**, unticked, with a
    **Taskbar settings** link (Windows locks that setting on current builds). The
    Mac-like tour preset no longer tries to change it.
  - Restore everything reports what it restored and turned off; the count matches
    what actually changed.

### 25. About, version and updates
- **Where:** the **Cove 0.9.0** button at the top right of Settings.
- **Expected:** the version label is never "…" (it reads "About Cove" for a moment
  at most). About shows the version, **Check for updates**, release notes as
  formatted text (headings, bullets), **Take the tour**, **Play the intro**, **Done**.
  Esc or Done closes and focus goes back to the button.

### 26. Signed updates
- **Where:** About → Check for updates → Update (once a newer signed release exists).
- **Expected:** a progress bar while downloading, then a silent install; Cove
  closes for a few seconds, reopens and shows "Updated to Cove x.y.z ✓". An
  unsigned release says it isn't signed yet and installs nothing.

---

## E. Safety and reversibility

### 27. Safe mode
- **Where:** hold **Shift** while Cove starts; Settings → Quiet → Safe mode; tray;
  `Cove.exe --safe`.
- **Expected:** the dock, status island, menu bar, other apps' tray icons,
  shortcuts, Alt+Space, switcher, snapping, clipboard history, traffic lights
  and wallpaper schedule all pause; the
  taskbar comes back; a banner in Settings explains why; your settings are kept.
  **Leave safe mode** brings everything back without a restart. Killing Cove
  within 30 s of launch twice → the third launch opens in safe mode on its own.

### 28. Fully reversible takeover
- **How:** with the Cove desktop, menu bar, Quiet tweaks and a Cove wallpaper on:
  (a) turn the Cove desktop off; (b) turn it on and Quit Cove; (c) Settings → Quiet
  → **Restore everything**; (d) uninstall Cove.
- **Expected:** each time, Windows looks exactly as before Cove: taskbar in place
  with its own settings, no reserved strip at the top or bottom, original
  wallpaper (after Restore), Quiet tweaks reverted (after Restore). Nothing in
  `C:\Windows` was changed; Cove never asked for admin rights.
- **Crash check:** with the Cove desktop on, end `Cove.exe` in Task Manager.
  Within a few seconds the taskbar comes back with its tray icons and no
  reserved strip is left (the watchdog).

### 29. Tray
- **Where:** the Cove icon in the notification area (Cove desktop off) or
  Control Centre's tray section (desktop on).
- **Expected:** left-click opens Settings. Right-click: Search, switches for the
  features (ticks match Settings both ways), Appearance, Take the tour, Check for
  updates, Restore…, Quit. Restore never runs without pressing the button in Settings.

### 30. Displays and scaling
- **Where:** two screens at different scaling (e.g. 100% and 150%).
- **Expected:** dock, status island, Search, switcher, menus and Control Centre are
  the same visual size on each screen and refit within ~1 s when scaling or
  resolution changes or a screen is unplugged. Settings → Menu bar → **Your
  screens** lists each screen live. Dock "Follows the pointer" moves the dock to
  the screen whose bottom edge you touch.

---

## F. Accessibility (new in v0.9.0)

### 31. Keyboard only
- **How:** open Settings and put the mouse away. Tab, Shift+Tab, Enter, Space, arrows.
- **Expected:** a graphite focus ring on everything you reach (never teal, never
  invisible). Order: title bar → safe-mode banner (if shown) → the scene's
  controls → scene bar. Pressing Enter on a scene in the scene bar moves focus into
  that scene. Switches toggle with Space. Segmented choices (Appearance, Glass,
  Accent, Hide the dock…) are one Tab stop each; ←/→ change the choice. The About
  sheet and the tour keep Tab inside themselves; Esc closes them.

### 32. Screen reader (Narrator: Ctrl+Win+Enter)
- **Expected:** switches are read with their row title and state ("Show the dock,
  switch, off") plus the row's note; segmented choices as radio buttons; menus as
  menus with menu items (and "checked" for ticked items, "has submenu" for ›);
  Search as a combo box with the result count announced and the selected result
  read as you arrow; Control Centre's Dark mode as a switch, Mute as a toggle
  button, sliders with a percentage. The desktop miniature is skipped (the scene
  bar covers it).

### 33. Reduced motion and transparency, contrast
- **How:** Windows Settings → Accessibility → Visual effects: turn **Animation
  effects** off, then **Transparency effects** off. Also try **Contrast themes**.
- **Expected:** with animations off, nothing slides, bounces or magnifies (fades
  of ~150 ms at most). With transparency off, the dock, menus, Control Centre,
  Search and Settings' miniature become solid. Contrast themes also hide the
  traffic lights, so the Windows buttons show. Grey secondary text stays easy to
  read everywhere (at least 4.5:1). Contrast themes: outlines on panels and the
  selected item use the system colours.

---

## Known issues in v0.9.0
- The installer isn't code-signed: SmartScreen asks (More info → Run anyway).
- Blur, high DPI / mixed scaling and clicks on other apps' tray icons have been
  checked on few PCs: these are the most useful things to report.
- Some white tray icons are hard to see in light mode.
- Live wallpapers are off ("coming soon").
- Control Centre can take a few seconds to open the first time.

## Report back
For each section: Pass/Fail, plus a screenshot or short clip of anything odd. Most
useful: the dock and status island on your wallpaper, any flash or flicker, any
app missing from the dock or tray section, menu placement near screen edges, and
anything that didn't come back after Restore or Quit.
