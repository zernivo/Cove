# Safety rules

Cove must never break Windows or weaken its security. These rules hold for
every feature; the second half of this page explains how the v0.9.0 desktop
takeover keeps them.

## The rules

1. **No elevation for normal use.** Per-user settings (HKCU) and user-space APIs only. Cove never asks for admin rights; the installer is per-user.
2. **No patching or injection.** No uxtheme patchers, no DLL injection into Explorer or any other process, no `SetWindowsHookEx` into other processes, no kernel drivers, no edits to files under `C:\Windows`.
3. **Documented surfaces only.** Use settings and APIs Microsoft documents or exposes in Settings. Where Cove can't change something safely, it opens the matching Settings page instead (for example the Widgets button, which current builds lock). The one exception is the long-standing `SetWindowCompositionAttribute` blur, used only on Cove's own windows (see below).
4. **Journal everything.** Before Cove changes a Windows value it records the previous value (including "didn't exist"). Restore replays the journal in reverse. The journal survives crashes and lives in `%APPDATA%\app.cove.desktop\journal.json`.
5. **Quit = stock.** The dock, status island, menu bar, stand-in tray, traffic lights, shortcuts and hooks exist only while Cove runs. The taskbar and its auto-hide setting are put back on exit.
6. **Safe mode.** Hold **Shift** while Cove starts, run `Cove.exe --safe`, or use the switch in Settings → Quiet or the tray. Cove also starts in safe mode by itself after two launches in a row that ended within 30 seconds.
7. **Never disable protections.** Defender, SmartScreen, UAC and Windows Update are never touched.
8. **Privacy.** No account, no telemetry. Network access is limited to the wallpaper gallery (only when you pick a photo) and update checks.
9. **No Apple assets.** No Apple fonts, icons, wallpapers or artwork.

## How the desktop takeover keeps them

### Hiding the taskbar
With the Cove desktop on, Cove hides Explorer's taskbar windows
(`Shell_TrayWnd` and `Shell_SecondaryTrayWnd`) with `ShowWindow` and turns on
the taskbar's own documented auto-hide (`SHAppBarMessage`), remembering the
previous auto-hide state. Explorer keeps running and keeps the taskbar intact;
nothing is unloaded or patched. If Explorer restarts, Cove hides the new
taskbar again. Turning the desktop off, quitting, safe mode and Restore
everything show it again and put the auto-hide state back.

### The watchdog
At start-up Cove launches a second, windowless copy of itself
(`Cove.exe --watchdog <pid> <settings>`). It only waits for Cove to end. If
Cove ends for any reason, crash included, the watchdog:
1. releases every AppBar Cove registered (from its record file),
2. shows the taskbar again and puts back its previous auto-hide setting,
3. broadcasts `TaskbarCreated` if the stand-in tray was running, so apps
   re-add their tray icons to Explorer.

### AppBar release
The menu bar (mode **Always**) and the dock (Hide the dock: **Never**) reserve
space the documented way, as AppBars (`SHAppBarMessage`), like the taskbar
does. Each registration is written to `appbars.txt` next to the settings.
Cove removes its AppBars when a bar turns off, on quit, in safe mode and on
Restore; the watchdog removes them after a crash, and the next launch removes
any a previous run still held. Explorer recomputes the work area itself. The
default modes (menu bar Off or Show at the top edge, dock "When a window
touches it") reserve nothing.

### The stand-in tray
With the taskbar hidden, Cove shows other apps' tray icons in Control Centre
and the status island. Apps add tray icons by sending `WM_COPYDATA` to the
top-level window of class `Shell_TrayWnd`. While the Cove desktop is on and
**Show other apps' tray icons** is on, Cove keeps a hidden window of that class
above Explorer's. It reads each message and then **forwards it unchanged** to
Explorer's own taskbar and returns Explorer's answer, so Explorer has every
icon all the time. Nothing is injected, and no other process's memory is read
(the technique follows systray-util, MIT, without its memory-reading start-up
step). UI Automation reads the names of icons Windows itself shows.

The stand-in steps aside when another tray replacement is running (YASB,
Zebar, TranslucentTB, ExplorerPatcher, Seelen). Turning it off destroys the
window and broadcasts `TaskbarCreated`, so apps re-register with Explorer. If
Cove crashes, the window dies with it and the watchdog broadcasts.

### The traffic-light overlays
The Mac window buttons are Cove's own small no-activate, topmost window
(`traffic.html`) placed over the focused window's caption buttons. Cove never
changes the app's window:
- It watches with out-of-context WinEvents (`SetWinEventHook` with
  `WINEVENT_OUTOFCONTEXT`): nothing is loaded into other processes.
- It measures buttons with DWM (`DWMWA_CAPTION_BUTTON_BOUNDS`) or, for custom
  title bars, UI Automation when the window gains focus.
- A click is sent to the window as its own `WM_SYSCOMMAND` (close, minimise,
  maximise), the same message its own buttons send; unsaved work still asks.
- The native buttons are only covered, never changed: Alt+F4, Narrator and the
  app see them as before, and background windows show them as usual.
- Anything uncertain is skipped: elevated windows, full-screen, cloaked, tool
  windows, hung windows, High Contrast and Cove's own windows. The patch hides
  while a window moves or when something lies over the buttons.
- Turning the switch off or quitting destroys the overlay.

### Glass
The blur behind Cove's glass comes from small native backdrop windows that
belong to Cove, using `SetWindowCompositionAttribute` (stable since
Windows 10 1803) on those windows only. No other window is touched. With
Windows' transparency effects off, the glass is solid.

### Safe mode
Safe mode pauses, without changing your settings: the Cove desktop (the
taskbar comes back), the dock and status island, the menu bar, the stand-in
tray, the traffic lights, Mac shortcuts, Alt+Space, the switcher, snapping,
clipboard history and the wallpaper schedule. Settings shows a banner with
the reason. **Leave safe mode** applies everything again without a restart.
