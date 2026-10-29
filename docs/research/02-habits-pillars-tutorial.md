# Habits, pillars & tutorial (researched 2026-10-29)

## User direction (2026-10-29)
- Cove keeps Windows' functionality but presents it in its OWN redesigned UI (not Windows chrome, not a macOS clone).
- Focus on 3-4 pillars. Seamless, effortless first use. A first-run tutorial that can be replayed any time.
- Cater to Mac habits: close/minimize from the keyboard, Cmd-style shortcuts, Mac workflows.

## Viral "Windows -> macOS" videos (what they cover)
- Unknown Tech, "I Built macOS 27 on Windows 11 With Only 2 FREE Apps!" (Short, 666K views, 2026-08-03) https://www.youtube.com/watch?v=JoP_4bWaH4I
  covers: theme, wallpaper, cursor, window buttons, taskbar, Start, File Explorer, icons, widgets, top bar, Spotlight search.
- "Make Windows 10/11 Look EXACTLY Like macOS" (2026-02-27) https://www.youtube.com/watch?v=hPEwBIqxCOM (Rainmeter + MyDockFinder, includes restoring to default)
- WinX Styler, "macOS 27 Golden Gate-Inspired Theme for Windows 11" (2026-06-22) https://www.youtube.com/watch?v=n0j-Fez7Bno (Windhawk mods, glass)
- Common tools: MyDockFinder (paid, Steam), Windhawk mods, Seelen UI, Rainmeter/Droptop, TaskbarXI, SecureUxTheme (patching; we avoid).
- What they get wrong for us: 4-6 separate tools stacked, uxtheme patching, conflicts (HowToGeek 2026-10-27: two Windhawk taskbar mods cut the Search button in half), breaks after updates, no real undo, look only and no change to habits.

## Why Windows feels messy vs macOS
- Two settings worlds (Settings + Control Panel/legacy dialogs), mixed fonts, densities and icon styles.
- Too many surfaces fighting for attention: widgets, Copilot, search box, ads/"suggestions", OneDrive/Edge nags, notification noise.
- Right-click menu split ("Show more options"), Explorer ribbon/command bar clutter.
- Taskbar mixes launcher + running apps + tray icons; no single home for app menus.
- macOS: one fixed place for menus (top bar), dock for apps, consistent spacing/type, calm defaults, fewer pop-ups.

## Habit gap: Mac vs Windows (core of pillar 1)
| Action | macOS | Windows stock | Cove plan |
|---|---|---|---|
| Close window | Cmd+W / red button (top-left) | Alt+F4 / X top-right | Alt+W hotkey; optional left-side controls only in Cove-made surfaces |
| Minimize | Cmd+M / yellow | Win+Down (twice) | Alt+M |
| Quit app | Cmd+Q | Alt+F4 (last window) | Alt+Q (closes all windows of the app) |
| Hide app | Cmd+H | none | Alt+H (minimize all windows of the app) |
| Switch apps | Cmd+Tab (apps) | Alt+Tab (windows) | keep Alt+Tab; add app-grouped switcher |
| Same-app windows | Cmd+` | none | Alt+` |
| Search/launcher | Cmd+Space | Win+S | Alt+Space (Cove Spotlight; note PowerToys Run uses Alt+Space too) |
| All windows | Ctrl+Up (Mission Control) | Win+Tab | Ctrl+Up -> Task View |
| Settings of app | Cmd+, | varies | n/a |
| Screenshot area | Cmd+Shift+4 | Win+Shift+S | Alt+Shift+4 -> Snipping |
| Lock | Ctrl+Cmd+Q | Win+L | keep both |
| Fullscreen | Ctrl+Cmd+F | F11 | Ctrl+Alt+F maximize |
| Copy/paste etc. | Cmd+C/V/Z | Ctrl+C/V/Z | "Mac keyboard mode": Alt acts as Cmd for text shortcuts (opt-in) |
Key behaviour difference: on Mac, closing a window keeps the app running; on Windows it usually quits. Mac switches apps, Windows switches windows.
Implementation: user-space global hotkeys (RegisterHotKey) + documented window messages (WM_CLOSE / ShowWindow SW_MINIMIZE). Text remap through PowerToys Keyboard Manager-style low-level hook, opt-in, one-click off. No system-file patching.

## Proposed 4 pillars
1. Mac habits: window control & keys (shortcuts above, app-grouped switcher, snap/tiling with Mac-style keys).
2. Dock + top bar (the sidebar/launcher layer): dock with magnification and running dots, top bar with clock/controls/active app name; native taskbar auto-hidden or moved to top as fallback.
3. Spotlight search & quick actions (wraps native search + apps, files, calculator, settings actions).
4. Calm look: one theme (light/dark, accent, rounded+blur where native), wallpaper gallery, icons/cursor, Quiet mode (turn off widgets, ads/suggestions, nags via documented settings), Finder-style Explorer layout via native options.
Foundation under all four: restore point + one-click "Back to stock".

## Tutorial ("Cove Tour")
- First launch: 2-3 minute interactive tour, one step per pillar. Learn by doing: "Press Alt+M to minimize this window", which checks off live when done.
- Replay any time: Help > Tour, plus per-pillar mini-tours from each settings page.
- Cheat sheet overlay: hold Alt for ~1s to see every Cove shortcut (like Mac's CheatSheet app). Printable/PDF version.
- "Habit coach" (optional): when the user clicks the X or uses Alt+F4, a tiny toast suggests the Cove shortcut, max a few times then stops.
- Skippable, remembers progress, works offline.

## Reference the user sent (2026-10-29): lumabar (FB reel)
- lumabar.xyz, github.com/Acadbek/lumabar. Solo dev Asadbek Nosirjonov (Uzbekistan). First release 0.1.4 on 2026-09-28, at v1.1.x by Oct 2026. $5.99 one-time (launch price; normally $15.99), 14-day refund.
- Dock/top bar for Win10/11 with 85-100+ live widgets: AI plan limits (Claude, ChatGPT, Gemini...), Stripe revenue, GitHub, calendar, music, CPU/RAM/battery, quick toggles, Screen Time, pomodoro. Widgets that act together (meeting mode, quiet hours). Liquid-glass styles that refract the wallpaper live. Win+T app switcher.
- Built with Tauri (Rust + React). No admin rights, changes no Windows files; the native taskbar auto-hides while it runs and comes back when you close it. No account, no telemetry.
- It explicitly isn't a Mac imitation: no magnification, no Launchpad.
- Lessons for Cove: copy the safety model (no admin, no file changes, close = stock) and consider the Tauri stack. Add a small set of useful top-bar widgets (now playing, battery, quick toggles, calendar), but don't compete on widget count. Cove's edge: Mac habits/keys, Spotlight, one calm theme, the Tour, full restore.
