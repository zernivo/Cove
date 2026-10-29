# Safety rules

Cove must never break Windows or weaken its security.

1. **No elevation for normal use.** Per-user settings (HKCU) and user-space APIs only. Any future feature that needs admin is opt-in, explained, and isolated.
2. **No patching or injection.** No uxtheme patchers, no DLL injection into Explorer or other processes, no kernel drivers, no edits to system files.
3. **Documented surfaces only.** Use settings Microsoft documents or exposes in Settings. Where Cove can't change something safely, it opens the matching Settings page instead.
4. **Journal everything.** Before Cove changes a value it records the previous value (including "didn't exist"). Restore replays the journal in reverse. The journal survives crashes and lives in `%APPDATA%\Cove\journal.json`.
5. **Quit = stock.** Dock, top bar and hotkeys exist only while Cove runs. The taskbar's auto-hide is put back on exit.
6. **Safe mode.** Holding Esc at sign-in starts Cove with everything disabled.
7. **Never disable protections.** Defender, SmartScreen, UAC and Windows Update are never touched.
8. **Privacy.** No account, no telemetry. Network access is limited to the wallpaper gallery and update checks, and both can be turned off.
9. **No Apple assets.** No Apple fonts, icons, wallpapers or artwork.
