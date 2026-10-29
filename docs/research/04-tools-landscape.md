# 04 · Tools landscape: making Windows look and work like macOS (researched 2026-10-29)

Context: Cove is a safe, reversible Windows 11 app (also Windows 10 where it's cheap to support). It gives Windows a calm, macOS-inspired UI and Mac habits through **its own interface**. Rules: no system-file patching, no admin-only hacks, no Apple assets, full restore to stock.
**Guiding principle (user, 2026-10-29): smoothness and flexibility first, and remove the clutter of Windows UI elements nobody uses.**

Windows baseline (see 01): Windows 11 26H2 went GA on 2026-09-29. Since Sept 2026 the native taskbar can sit at the top, left or right ([TechSpot, 2026-10-26](https://www.techspot.com/article/3187-windows-taskbar-customization/)). Win10 still has about 28% share, so a Win10 fallback matters.

Legend: **Admin?** = needs elevation to install or run. **Patches?** = changes system files, injects code into other processes, or loads a driver. **Break risk** = how likely a Windows update breaks it.

---

## 1. Tool-by-tool survey

### Full "macOS look" suites

**MyDockFinder** (Steam, about $4.99)
- **What it offers:** the most convincing macOS imitation. Dock with GPU magnification and adjustable blur, a menu bar, a Launchpad grid, hot corners, Stage-Manager-style view, minimise animations and a Steam Workshop of skins ([TechSpot](https://www.techspot.com/article/3187-windows-taskbar-customization/), [Steam](https://store.steampowered.com/app/1787090/MyDockFinder/)).
- **Tech:** closed source, built by one developer, WinUI/D3D rendering, needs VC++ runtimes and .NET.
- **Admin and patching:** helper processes `dockmod.exe`, `dockmod64.exe` and `Mydock.exe` run as admin; the developer advises not starting in admin mode because it breaks drag-and-drop ([issue #885](https://github.com/mydockfinder/mydockfinder-for-Win10-Win11/issues/885)). It ships `Temperature.sys`, a WinRing0 driver that Defender quarantines as `VulnerableDriver:WinNT/Winring0` ([Steam thread](https://steamcommunity.com/app/1787090/discussions/0/595160802290499698/), [issue #913](https://github.com/mydockfinder/mydockfinder-for-Win10-Win11/issues/913)).
- **Reverting:** poor. Steam reviewers say uninstalling leaves the core running and they had to delete files by hand ([Steam reviews](https://steamcommunity.com/app/1787090/reviews/?browsefilter=toprated)).
- **UX quality:** the best eye candy of any tool here.
- **Complaints:** layer bugs, Explorer crashes when opening the Finder icon ([#791](https://github.com/mydockfinder/mydockfinder-for-Win10-Win11/issues/791)), and about 2.5% CPU while idle ([#913](https://github.com/mydockfinder/mydockfinder-for-Win10-Win11/issues/913)). All-time Steam score is "Mostly Positive", about 72–73% of roughly 7.5k reviews ([Steambase](https://steambase.io/games/mydockfinder/reviews)).

**Seelen UI** (free, **AGPL-3.0**, about 18k stars, v2.8.10 released 2026-10-22)
- **What it offers:** a whole desktop environment. Dock on any edge, top "Fancy Toolbar", app launcher, tiling window manager, live wallpapers, widgets, custom Alt+Tab, and CSS/JSON theming ([seelen.io](https://seelen.io/apps/seelen-ui), [GitHub](https://github.com/eythaann/Seelen-UI)).
- **Tech:** Rust + Tauri 2 (WebView2) with a Svelte/React UI, plus `hook_dll` and a background `slu-service`.
- **Admin and patching:** no system-file changes ([FAQ](https://seelen.io/faq)), but it does load hook DLLs. Its v2.8.8 security release fixed an out-of-bounds read inside explorer.exe and widgets acting without permission ([release](https://seelen.io/apps/seelen-ui/releases/v2.8.8)).
- **Break risk:** medium. Updates are frequent and regressions slip in, for example a blank tray in 2.8.8 and 2.8.9 ([#1917](https://github.com/eythaann/Seelen-UI/issues)).
- **Reverting:** uninstalling did not restore taskbar or font settings ([#891](https://github.com/eythaann/Seelen-UI/issues/891), closed as not planned).
- **Complaints:** RAM. 12 WebViews on two monitors use about 1.29 GB, and `--process-per-site` cuts that to 512 MB ([#1827](https://github.com/eythaann/Seelen-UI/issues/1827)). Out-of-memory crashes ([#1416](https://github.com/eythaann/Seelen-UI/issues/1416)). A panic on every sleep takes out the bar and hotkeys until a watchdog restarts it ([#1874](https://github.com/eythaann/Seelen-UI/issues/1874)). Lots of options make setup heavy.

**lumabar** (lumabar.xyz, $5.99 one-time at launch, list price $15.99, closed source, one developer)
- **What it offers:** one dock on the top or bottom edge with your apps plus 100+ live widgets: music, calendar, AI usage limits, revenue, screen time. Glass and "Liquid Glass" styles, quiet hours ([site](https://lumabar.xyz/), [about](https://lumabar.xyz/about)).
- **Tech:** Rust + React on Tauri.
- **Admin and patching:** "needs no administrator rights and changes no Windows files." It auto-hides or hides the taskbar while it runs, and "close lumabar and the taskbar is back as it was."
- **Track record:** 25 releases between 2026-09-28 and 2026-10-28. No account, no server, no telemetry.
- **Gaps:** no window management, no keyboard or habit layer. **This is the closest model to Cove's safety stance and proves a solo developer can ship a Tauri dock without admin.**

**Windows→Mac skin packs** (skinpacks.com: "macOS 27 Golden Gate", "Monterey" and others)
- **What they are:** bundles of a visual style, icons, cursors and a dock. They depend on UltraUXThemePatcher or SecureUxTheme and StartIsBack/WindowBlinds. Archives are passworded and work best at 100% scaling ([Monterey pack](https://skinpacks.com/macos-monterey-skinpack-for-windows-11/), [FAQ](https://skinpacks.com/help-faq)).
- **Risks:** they patch theme DLLs, can fail to boot unless a stock theme is set first, need `sfc /scannow` after big updates, and ship Apple-derived artwork (copyright risk).
- **For Cove:** exactly what Cove must not be.

### Mod platforms and shell restorers

**Windhawk** (free, **GPL-3.0**, about 9.6k stars, v1.7.3)
- **What it offers:** a "marketplace" of small C++ mods with source shown in-app ([windhawk.net](https://windhawk.net/)). The macOS-relevant mods:
  - Windows 11 Taskbar Styler, about 1.08M users, with DockLike and FluentGlass themes
  - Taskbar Dock Animation (macOS hover magnification), about 500k users
  - MacOS Minimize Animation (genie effect)
  - Start Menu, Notification Center and File Explorer stylers ([mods](https://windhawk.net/mods/windows-11-taskbar-styler), [styling guide](https://github.com/ramensoftware/windows-11-taskbar-styling-guide))
- **Admin and patching:** installs as a service and **injects its engine into almost every process**, with system-critical and well-known game paths excluded ([wiki](https://github.com/ramensoftware/windhawk/wiki/Injection-targets-and-critical-system-processes)).
- **Antivirus and anti-cheat:** EDR products such as SentinelOne and Panda clash with it; one Panda user couldn't log in at all ([#495](https://github.com/ramensoftware/windhawk/issues/495), [troubleshooting](https://github.com/ramensoftware/windhawk/wiki/Troubleshooting)). Anti-cheat clashes too ([#670](https://github.com/ramensoftware/windhawk/issues/670), [#479](https://github.com/ramensoftware/windhawk/issues/479)).
- **Break risk:** medium to high. Mods hook Explorer internals that change with each build ([TechSpot](https://www.techspot.com/article/3187-windows-taskbar-customization/)). Mods also conflict with each other: two taskbar mods cut the Search button in half (HowToGeek 2026-10-27, noted in 02).
- **Reverting:** disable the mod, or use Windhawk's safe mode.
- **UX quality:** a clean VS Code-like UI, but every mod is configured on its own, often through YAML or JSON style strings.

**ExplorerPatcher** (free, **GPL-2.0**, about 34k stars)
- **What it offers:** injects a DLL into explorer.exe to bring back a Windows 10 taskbar (now its own reimplementation, since 24H2 removed the old code), the Win10 Start menu and the old flyouts.
- **Break risk:** very high. Updates broke it repeatedly in 2026: April KB5083769 crash ([#4930](https://github.com/valinet/ExplorerPatcher/issues/4930)), May KB5089573 Explorer crash loop ([#4987](https://github.com/valinet/ExplorerPatcher/issues/4987)), June KB5094126 removed the Win10 Start menu ([#5031](https://github.com/valinet/ExplorerPatcher/issues/5031)).
- **Antivirus:** Defender false positives on new releases ([discussion #5038](https://github.com/valinet/ExplorerPatcher/discussions/5038)). Release notes say to turn off Smart App Control and not to install alongside SentinelOne ([release notes](https://github.com/valinet/ExplorerPatcher/releases/tag/26100.8457.70.4)).
- **UX quality:** wordy, dated settings.

**StartAllBack** ($4.99 per PC one-time, 100-day trial, closed source)
- **What it offers:** Win7/10-style Start, taskbar, Explorer ribbon and context menus. Polished and light. v3.9.28 shipped 2026-10-12, with formal 26H2 support on 2026-09-21 and a dark-mode fix for 26H2 three days later ([startallback.com](https://www.startallback.com/)).
- **Admin and patching:** reported to install without admin ([UpdateStar](https://startallback.updatestar.com/)). It loads into Explorer, and Microsoft listed it, alongside ExplorerPatcher, as breaking explorer.exe on 22H2 Moment 2 ([Neowin 2023](https://www.neowin.net/news/microsoft-explorerpatcher-startallback-start11-break-on-windows-11-moment-2-kb5022913/)).
- **Pattern worth borrowing:** **hold Escape at login to disable everything** ([StartIsBack FAQ](https://startisback.com/)).
- **Cove relevance:** restores old Windows, not a Mac look.

**Start11** (Stardock, $10, 30-day trial, closed source)
- **What it offers:** seven Start menu styles, taskbar tweaks, works on Win10 and Win11 ([TechSpot](https://www.techspot.com/article/3187-windows-taskbar-customization/)).
- **Risks:** shell hooking, same update risk as above.

**Open-Shell** (free, **MIT**)
- **What it offers:** Start menu replacement only. Dated UI, clean uninstall.

### Docks and bars

**Winstep Nexus** (free for personal use, $12.95 commercial; Nexus Ultimate $29.95; v26.9 released 2026-09-21)
- **What it offers:** a 26-year-old veteran. Widgets, skins, in-dock tray, PWA support, "Window Dodge" auto-hide ([winstep.net](https://www.winstep.net/nexus.asp)).
- **Tech:** closed source, runs Win7–11.
- **Complaints:** defaults are busy, with "30+ mouse-over effects… spend a few minutes switching things off" ([TechSpot](https://www.techspot.com/article/3187-windows-taskbar-customization/)). Antivirus flags its executables now and then ([WinCustomize](https://www.wincustomize.com/article/542515/winstep-nexus-nexus-ultimate-and-winstep-xtreme-v269-released)).

**MacDock** (the name covers several unrelated projects)
- **skinpacks macDock:** $3.99, a RocketDock-style skin ([skinpacks](https://skinpacks.com/macdock/)).
- **simon-kramer/MacDock:** WPF/.NET 8, **MIT** ([GitHub](https://github.com/simon-kramer/MacDock)).
- **labwhiz/MacDock:** Win11, **MIT** ([GitHub](https://github.com/labwhiz/MacDock)).
- **yudhisthirabala/MacDock:** C++ Win32/Direct2D single exe ([GitHub](https://github.com/yudhisthirabala/MacDock)).
- **Chinese "macdock":** written in 易语言 and needs admin plus antivirus exclusions ([fxxz](https://www.fxxz.com/soft/350311.html)).
- All are tiny hobby projects with no restore story. The MIT ones are useful as reference code only.
- New paid entrant: **Dockable** on Steam, with Liquid Glass and Genie effects ([dockable.app](https://dockable.app/)).

**Rainmeter** (free, **GPL-2.0**) + **Droptop Four** (free Basic; Supporter is pay-what-you-want, $6–8)
- **What it offers:** Droptop is a floating macOS-style menu bar with dropdown folders, app grid and hot corners, and now has a macOS Tahoe theme ([droptopfour.com](https://www.droptopfour.com/), [download](https://www.droptopfour.com/download/)).
- **Tech:** Rainmeter skins plus compiled AutoHotkey helpers.
- **Smoothness:** needs Rainmeter's hardware acceleration turned on or it is slow ([FAQ](https://docs.droptopfour.com/help/faqs)). Its v4.420.51 changelog admits "jitter and jank" in animations ([release](https://github.com/Droptop-Four/Droptop-Four/releases/tag/v4.420.51)).
- **Other complaints:** it reset a user's Documents path in the registry on first run ([Rainmeter forum](https://forum.rainmeter.net/viewtopic.php?t=44513)). Drag-and-drop was turned off because it "was causing more harm than help."

**TranslucentTB** (free, **GPL-3.0**, Microsoft Store, release 2026.2)
- **What it offers:** clear, acrylic or blur taskbar that changes dynamically, for example when a window is maximised.
- **Patching:** the repo contains `ExplorerHooks` and `ExplorerTAP`, i.e. it injects into Explorer to style the Win11 XAML taskbar ([repo](https://github.com/TranslucentTB/TranslucentTB)).
- **Break risk:** medium.

**RoundedTB** (free, **GPL-3.0**)
- **What it offers:** margins, rounded corners and "dynamic mode", a dock-like segmented taskbar. No admin needed.
- **Status:** **archived since 2023** ([GitHub](https://github.com/RoundedTB/RoundedTB)). Feature updates keep breaking dynamic mode ([Unstore](https://unstore.io/discover/best-roundedtb-alternatives-desktop/)).

**TaskbarXI** (free, **GPL-3.0**)
- **What it offers:** turns the Win11 taskbar into a dock using window regions; a tray icon reverts it. The original repo ([ChrisAnd1998/TaskbarXI](https://github.com/ChrisAnd1998/TaskbarXI)) is **archived (2022)**.
- **TaskbarX (Win10, MIT):** stops working past Win11 22621.1265 ([TaskbarX README](https://github.com/ChrisAnd1998/TaskbarX)).

**PowerToys Command Palette Dock** (MIT, a new competitor)
- **What it offers:** since 0.98 (Mar 2026), a dock on any edge. Multi-monitor in 0.100, auto-hide in 0.101, keyboard focus with **Win+Alt+J** in 0.102 ([0.98 blog](https://devblogs.microsoft.com/commandline/powertoys-0-98-is-here-new-keyboard-manager-ux-the-command-palette-dock-and-better-cursorwrap/), [docs](https://learn.microsoft.com/en-us/windows/powertoys/command-palette/dock)).
- **Gap:** it docks *commands*, not running windows. Not a Mac dock.

### Habit and keyboard tools

**PowerToys** (free, **MIT**, about 140k stars, 0.102 released 2026-10-21)
- **Command Palette:** the successor to Run. Launcher, Win+Alt+Space, can be bound to the Copilot key, quick-access shelf with Alt+1…9.
- **Keyboard Manager:** remaps; tap-alone remaps added in 0.102.
- **FancyZones:** layouts and window rotation.
- **Peek:** Quick Look, Ctrl+Space by default.
- **Shortcut Guide:** rebuilt in 0.100.
- Sources: [0.102 blog](https://devblogs.microsoft.com/commandline/powertoys-0-102-is-here-new-mouse-utilities-command-palette-improvements-and-much-more/), [releases](https://github.com/microsoft/powertoys/releases).
- **Admin:** per-user installer available, no admin needed.
- **Gaps:** about 30 utilities, each configured separately, with no "Mac mode" preset. Peek sometimes needs several keystrokes, and users prefer QuickLook ([#49330](https://github.com/microsoft/PowerToys/issues/49330)).

**QuickLook** (free, **GPL-3.0**, about 25.6k stars, v4.5.0)
- **What it offers:** press Space in Explorer to preview, with plugins.
- **Complaint:** the preview window gets buried and must be refocused ([#1296](https://github.com/QL-Win/QuickLook/issues/1296)).
- **Gap:** Explorer-only; launchers swallow Space ([Flow #3437](https://github.com/Flow-Launcher/Flow.Launcher/issues/3437)).

**Flow Launcher** (free, **MIT**, about 16k stars, v2.1.4)
- **What it offers:** Spotlight/Alfred-style launcher with plugins. C#/WPF, Alt+Space.
- **Pick of reviewers:** recommended next to Raycast and Command Palette ([PCMag](https://uk.pcmag.com/migrated-3765-windows-10/166012/windows-11s-start-menu-is-getting-betterbut-these-3-launchers-still-run-circles-around-it)).

**AltSnap** (free, **GPL-3.0**, v1.68)
- **What it offers:** Alt+drag to move or resize windows from anywhere. Written in C.
- **Complaint:** feels sluggish compared with AltDrag on 144 Hz screens ([#129](https://github.com/RamonUnch/AltSnap/issues/129)).

**SecureUxTheme** (**LGPL-2.1**) and **UltraUXThemePatcher** (closed, v4.6.1 supports 26H2)
- **SecureUxTheme:** removes signature checks for visual styles in memory. Needs admin and a reboot ([GitHub](https://github.com/namazso/SecureUxTheme), [secureuxtheme.com](https://secureuxtheme.com/)). Breaks after feature updates ([#267](https://github.com/namazso/SecureUxTheme/issues/267), [discussion #240](https://github.com/namazso/SecureUxTheme/discussions/240)).
- **UltraUXThemePatcher:** **modifies system files**, and its own site warns this "can lead to… total breakdown of Windows" ([mhoefs.eu](https://mhoefs.eu/software_uxtheme.php?lang=en)). Microsoft community advisors call theming this way "a really bad idea" ([MS Q&A](https://learn.microsoft.com/en-us/answers/questions/3811213/is-ultrauxthemepatcher-safe)).
- **For Cove:** excluded on principle.

---

## (a) Summary table

| Tool | Offers | Price / licence | Open? | Stack | Admin? | Patches? | Break risk | Revert | UX / smoothness |
|---|---|---|---|---|---|---|---|---|---|
| MyDockFinder | Dock, menu bar, Launchpad, animations | ~$4.99 Steam | No | WinUI/D3D, native | Helpers run elevated | Kernel driver (WinRing0), global hooks | Med | Poor, leftovers | Best eye candy; idle CPU, layer bugs |
| Seelen UI | Full DE: dock, bar, launcher, tiling, wallpapers | Free, AGPL-3.0 | Yes | Rust+Tauri, Svelte/React, hook DLL | No | Hook DLLs, no file patching | Med | Settings not restored (#891) | Powerful, heavy RAM, crashes on sleep |
| lumabar | Top/bottom widget dock, 100+ widgets | $5.99 once | No | Rust+React, Tauri | No | No | Low | Close app = taskbar back | Polished, focused, no keyboard layer |
| Windhawk + mac mods | Taskbar→dock styler, magnify, genie | Free, GPL-3.0 | Yes | C++ mods, global injection | Yes (service) | Injects into all processes | Med–High | Disable mod / safe mode | Clean app, fiddly per-mod config, mods conflict |
| MacDock (various) | Hobby docks | Free (MIT) / $3.99 | Some | WPF, Win32 | Varies | Mostly no | Med | Manual | Basic |
| Winstep Nexus | Veteran dock, widgets | Free personal / $29.95 | No | Native | No | No | Low | Uninstall | Busy defaults |
| Rainmeter + Droptop | Mac menu bar, widgets | Free / PWYW | Yes (GPL-2) | Rainmeter skins + AHK | No | No (but touched registry) | Low–Med | Uninstall | Jank w/o HW accel, .ini culture |
| TranslucentTB | Clear/acrylic taskbar | Free, GPL-3.0 | Yes | C++, ExplorerTAP | No | Injects into Explorer | Med | Exit app | Simple, good |
| RoundedTB | Rounded/segmented taskbar | Free, GPL-3.0 | Yes | C# | No | No | High (archived) | Exit app | Breaks on updates |
| TaskbarXI | Taskbar→dock | Free, GPL-3.0 | Yes | C++ | No | No (regions) | High (archived) | Tray revert | Dated |
| Start11 / StartAllBack | Classic Start/taskbar | $10 / $4.99 | No | Native, Explorer hooks | No (SAB) | Loads into Explorer | Med | Esc at logon (SAB), uninstall | Polished, but not a Mac look |
| ExplorerPatcher | Win10 taskbar/Start | Free, GPL-2.0 | Yes | C, DLL into Explorer | Yes | Injects into Explorer | Very high | Uninstall, sometimes from Safe Mode | Fast, ugly settings |
| PowerToys | Launcher, remaps, zones, Peek, Dock | Free, MIT | Yes | C#/C++, WinUI 3 | No (user install) | No | Low | Toggle module | Good but 30 separate tools |
| QuickLook | Space preview | Free, GPL-3.0 | Yes | C# WPF | No | Explorer hooks for selection | Low | Uninstall | Great previews, focus quirks |
| Flow Launcher | Spotlight-style launcher | Free, MIT | Yes | C# WPF | No | No | Low | Uninstall | Fast, plugin-rich |
| AltSnap | Alt-drag move/resize | Free, GPL-3.0 | Yes | C | No | Global hooks | Low | Exit | Laggy at high refresh |
| SecureUxTheme / UltraUX | Unsigned visual styles | LGPL-2.1 / closed | Partly | C++ | **Yes** | In-memory / **system files** | High | Set stock theme first, sfc | Risky, can block boot |
| Skin packs | Whole Mac makeover | ~$3–10 | No | Bundles + patchers | **Yes** | **Yes** | High | Manual, risky | Imperfect, DPI-sensitive, Apple assets |

---

## (b) What Cove should borrow

**Patterns**
1. **Taskbar via auto-hide, never killed** (lumabar). Set taskbar auto-hide with `SHAppBarMessage(ABM_SETSTATE)` while Cove runs. Save the prior state and put it back on exit, crash or uninstall ([SHAppBarMessage docs](https://learn.microsoft.com/en-us/windows/win32/api/shellapi/nf-shellapi-shappbarmessage)).
2. **A kill switch:** StartAllBack's "hold Esc at logon" plus Windhawk's safe mode. Cove: hold Esc at login, or a `--safe` flag, starts with every feature off.
3. **Per-feature toggles with source you can inspect:** Windhawk's per-mod model and PowerToys modules. Cove: each pillar is a toggle that is off-able on its own.
4. **Presets** like Taskbar Styler themes. Cove: "Mac", "Calm", "Focus" presets, previewed before applying.
5. **First-run guide and a shortcut cheat sheet:** PowerToys OOBE and Shortcut Guide. Cove: a replayable tutorial (see 02) and a cheat sheet when Cmd/Alt is held.
6. **Launcher:** Command Palette and Flow Launcher (fuzzy search, actions in results, plugins).
7. **Space-to-preview:** QuickLook and Peek. Use the native `IPreviewHandler` and thumbnail handlers so Office/PDF previews come free.
8. **Dock details:** MyDockFinder polish (magnification curve, minimise animation, unread badges, progress on icons). Winstep's "Window Dodge" auto-hide. lumabar's "drag a file over an app and it comes to the front."
9. **Widget permission model:** Seelen 2.8.8 started asking permission before third-party widgets can change settings or control windows. Cove should be sandboxed from day one.
10. **Behaviour around fullscreen and games:** MyDockFinder and MacDock hide in fullscreen. Windhawk excludes known game paths. Cove never injects, so anti-cheat isn't an issue, but the overlay should still step aside for fullscreen apps.

**Permissively licensed code Cove could legally reuse** (verified on GitHub 2026-10-29)

| Library | Licence | Use in Cove |
|---|---|---|
| [microsoft/PowerToys](https://github.com/microsoft/PowerToys) | MIT | Reference/port: Keyboard Manager hook logic, FancyZones layouts, Peek previewers, Command Palette ranking |
| [Flow-Launcher/Flow.Launcher](https://github.com/Flow-Launcher/Flow.Launcher) | MIT | Launcher fuzzy matching, plugin protocol ideas |
| [cairoshell/ManagedShell](https://github.com/cairoshell/ManagedShell) | Apache-2.0 | C# appbar registration, task list, tray icon hosting (if .NET) |
| [dremin/RetroBar](https://github.com/dremin/RetroBar), [Cairo](https://github.com/cairoshell/cairoshell) | Apache-2.0 | Examples of a full taskbar replacement that hides the native one |
| [Open-Shell](https://github.com/Open-Shell/Open-Shell-Menu) | MIT | Start-menu enumeration reference |
| [tauri-apps/tauri](https://github.com/tauri-apps/tauri), [window-vibrancy](https://github.com/tauri-apps/window-vibrancy), [global-hotkey](https://github.com/tauri-apps/global-hotkey), [tray-icon](https://github.com/tauri-apps/tray-icon) | Apache-2.0 / MIT (dual on crates.io) | App shell, Mica/Acrylic, hotkeys, tray |
| [microsoft/windows-rs](https://github.com/microsoft/windows-rs) | MIT / Apache-2.0 | All Win32 calls from Rust |
| [microsoft/CsWin32](https://github.com/microsoft/CsWin32), [WinUIEx](https://github.com/dotMorten/WinUIEx), [H.NotifyIcon](https://github.com/HavenDV/H.NotifyIcon), [WPF-UI](https://github.com/lepoco/wpfui), [WindowsAppSDK](https://github.com/microsoft/WindowsAppSDK) | MIT | The .NET/WinUI route |
| [seelen-corp/windows-keyboard-hook](https://github.com/seelen-corp/windows-keyboard-hook) | MIT | Small Rust global-hotkey/hook crate (3 stars; review before use) |
| simon-kramer / labwhiz MacDock | MIT | Tiny reference only |

**Licence terms**
- **MIT:** keep the copyright and licence notice in source and in an in-app "Open-source licences" screen.
- **Apache-2.0:** include the licence, keep any NOTICE file, mark files you changed, and note the patent-termination clause.
- **Do not copy code from:** GPL (Windhawk, ExplorerPatcher, TranslucentTB, QuickLook, AltSnap, RoundedTB, TaskbarXI, Rainmeter, GlazeWM, Zebar), AGPL (Seelen; it would force Cove's whole source open, even for network use), LGPL SecureUxTheme (excluded anyway), or Komorebi's source-available licence. Ideas and behaviour can be studied; code can't be copied unless Cove itself goes GPL.
- **No Apple assets:** no SF Pro, Apple icons, cursors or wallpapers. Use Inter or Geist (OFL), Fluent or Phosphor/Lucide icons (MIT/ISC), and Cove's own or generated wallpapers.

---

## (c) Where every tool falls short

1. **Stacking.** A real Mac-look setup uses 4–6 tools (dock + bar + launcher + preview + theme patcher + taskbar mod). Each has its own settings window, tray icon, updater and autostart entry. Viral guides all do this (see 02).
2. **Conflicts.** Windhawk taskbar mods break each other (the Search button cut in half). ExplorerPatcher conflicts with Taskbar Styler ([windhawk-mods #1028](https://github.com/ramensoftware/windhawk-mods/issues/1028)). TranslucentTB and RoundedTB need special compatibility modes. Two docks fight over the same work area.
3. **Shell injection = update roulette.** ExplorerPatcher broke in April, May and June 2026. StartAllBack and Windhawk ship fixes after each feature update.
4. **Antivirus, EDR and anti-cheat.** Global injection (Windhawk), unsigned binaries (ExplorerPatcher, MyDockFinder) and a kernel driver (MyDockFinder's WinRing0) cause false positives, blocked logins and game bans.
5. **Poor restore.** Seelen leaves settings changed. MyDockFinder leaves processes and files. Skin packs need sfc and a stock theme first. Droptop rewrote a registry path. **None keeps a journal of what it changed.**
6. **Look only, no habits.** Nothing besides PowerToys Keyboard Manager (raw remaps) touches Mac habits: Cmd-style Q, W, H, M, Space; Cmd-Tab per app; Cmd-` per window; close vs quit. Nobody teaches them.
7. **No onboarding.** Every tool drops you into a settings maze: Windhawk YAML, Rainmeter .ini, ExplorerPatcher's wordy properties, Nexus's 30+ effects.
8. **Clutter added, not removed.** Most tools add more surfaces (widgets, effects, stats) instead of hiding Windows' leftovers.
9. **Fragile maintenance.** Many are solo efforts (MyDockFinder, lumabar, Droptop) or archived (RoundedTB, TaskbarXI, TaskbarX).

### How other tools fail at feeling smooth (the user's priority)
- **Idle cost:** MyDockFinder uses about 2.5% CPU while idle ([#913](https://github.com/mydockfinder/mydockfinder-for-Win10-Win11/issues/913)). Seelen's RAM grows over time and spawns one WebView renderer per widget, 1.29 GB on two monitors ([#1081](https://github.com/eythaann/Seelen-UI/issues/1081), [#1827](https://github.com/eythaann/Seelen-UI/issues/1827)).
- **Frame pacing:** Droptop admits animation "jitter and jank" and needs hardware acceleration turned on. AltSnap lags on 144 Hz. window-vibrancy's acrylic "bad performance when resizing/dragging" ([README](https://github.com/tauri-apps/window-vibrancy)).
- **Material glitches:** acrylic stops when the window isn't focused ([window-vibrancy #139](https://github.com/tauri-apps/window-vibrancy/issues/139)). Taskbar Styler themes fail silently after updates.
- **Stalls and dropouts:** Seelen loses the bar and hotkeys after sleep until a watchdog restarts it. Explorer crash loops (ExplorerPatcher) or 10-second Explorer restarts (MyDockFinder #791).
- **Input that misses:** Peek needs several Ctrl+Space presses. QuickLook loses focus. Launchers swallow Space.
- **Layering:** MyDockFinder "layer issues". RetroBar overlaps the native taskbar unless auto-hide is set first.
- **Fiddly controls:** Taskbar Styler's dropdown does nothing until you press Save ([TechOnRamen](https://techonramen.com/make-windows-11-look-like-macos/)). Winstep makes you switch effects off one by one.

---

## (d) What Cove can offer more effectively and cleanly

**1. One app, three or four pillars, one switch each**
- **Dock & Bar:** a Cove dock plus an optional top bar.
- **Find:** launcher plus Space preview.
- **Mac Habits:** a keyboard layer plus a trainer.
- **Calm:** hide clutter, quiet mode, wallpapers, one consistent Cove look.

Presets (Mac, Calm, Focus, Stock) apply in one click, with a live preview first.

**2. Safety by construction**
- Per-user install, no admin, no drivers, no injection, no system-file writes.
- Only documented, per-user settings (HKCU and APIs).
- Signed MSIX or installer, so fewer antivirus false positives.
- Cove's own surfaces are ordinary top-level windows.

**3. A restore journal and one-click undo**
- Before any change, write `{setting, old value, new value, timestamp}` to a journal.
- "Restore to stock" replays it backwards. The uninstaller does the same.
- Safe Mode: hold Esc at login.
- A watchdog brings back the native taskbar if Cove crashes, because auto-hide is only flipped while Cove is alive.

**4. Clutter Cove should hide or replace** (per-user settings or Cove's own surfaces, each toggle reversible and journaled)

| Windows element | Why it's clutter | Cove action |
|---|---|---|
| Taskbar **Widgets** button and weather/news flyout | Rarely used, news feed | Hide (taskbar setting). Weather/time live in Cove's bar |
| **Task View** button | Duplicates Win+Tab | Hide. Cove's Mission-Control-style key does the job |
| Taskbar **Search box/icon** | Duplicates Start, Bing-flavoured | Hide. Cove launcher on Alt/Ctrl+Space |
| **Copilot** button and key | Unwanted by many (see 01) | Hide the button; offer remapping the key to the Cove launcher (Windows allows rebinding the Copilot key) |
| **Start "Recommended"** section, promoted apps, ads | Noise | Turn off recommendations/tips (Settings > Personalisation > Start). Cove launcher replaces Start for daily use |
| **System tray overflow chevron** and icon soup | Many unseen icons | Cove bar shows only status (Wi-Fi, battery, sound, clock), plus one "More" menu |
| Notification bell/date stack | Noisy, duplicated | Cove clock opens a calm notification/quiet-mode panel; native centre still available |
| "Show desktop" sliver | Invisible target | Replace with a hot corner (optional) |
| **Lock screen** "fun facts", tips, Spotlight ads | Distracting | Turn off tips on the lock screen; Cove wallpaper there instead |
| Desktop "Learn about this picture" icon (Spotlight) | Clutter | Swap Spotlight for a Cove gallery wallpaper |
| **Settings** Microsoft 365/OneDrive/Game Pass banners and "suggested content" | Upsell | Turn off suggested content in Settings > Privacy (documented toggles) |
| **Explorer** Home "Recommended", Gallery, OneDrive nag banners | Noise | Hide via Explorer options where documented. Cove's Finder-like shortcuts (Recents, Downloads) live in the dock |
| Two-step "Show more options" context menu | Friction | Leave the system alone (fixing it needs shell hooks or HKCU CLSID tricks). Cove's own dock/launcher menus are one level |
| Taskbar **labels/grouping noise** and pinned bloat (Store, Edge, Teams) | Clutter | Auto-hidden native taskbar; the Cove dock starts from the user's own pins (as Seelen and Dockable seed from the taskbar) |
| Edge/OneDrive startup nags, "Finish setting up your PC" | Interrupts | Offer to turn off "Suggest ways to finish setting up" and the welcome experience (Settings toggles) |

Note: some of these toggles move between builds, and some registry keys may be protected. Where Cove can't write a setting safely, it should open the exact `ms-settings:` page with a one-line hint instead of forcing it. (Inferred design rule. Specific key protections were not checked in this research.)

**5. Smoothness as a spec, not an afterthought**
- Target 120/144 Hz animations on the compositor: CSS transforms and opacity only (Tauri), or Composition animations (WinUI).
- **0% idle CPU:** event-driven via `SetWinEventHook` and `WM_*` messages, never polling.
- One shared WebView2 environment and renderer (if Tauri).
- Hide windows instead of destroying them. Surfaces created only when needed.
- Material choice: Acrylic only where content scrolls behind (dock, menus), Mica for Cove's main window, solid fallback with "reduce transparency" or on battery.
- Resume-from-sleep and display-change handling tested as first-class cases (Seelen's top failure).

**6. A habit layer nobody else has**
- Scoped shortcuts: e.g. Alt or Ctrl as "Cmd". Cmd+Q quits (WM_CLOSE to every window of the app), Cmd+W closes the window/tab, Cmd+H hides, Cmd+M minimises, Cmd+` cycles windows of the same app, Cmd+Tab switches apps.
- Per-app exceptions for games, terminals and IDEs.
- Built-in trainer: on-screen hints for the first few days, plus a cheat sheet when the modifier is held.
- The PowerToys Keyboard Manager concept, but curated and explained.

**7. Onboarding** (see 02)
- A 60-second first run: pick a preset → see before and after → try three shortcuts → done.
- Replayable any time.

**8. Flexibility without a config maze**
- Drag to arrange, per-monitor options, light/dark/auto.
- Import pins from the taskbar.
- Export and import a profile file.
- Advanced JSON for power users, hidden by default.

**Positioning in one line:** lumabar's no-admin, close-and-it's-gone safety, plus MyDockFinder's polish, plus PowerToys' habits, in one calm app with a real undo.

---

## (e) Recommended tech stack

**Recommendation: Rust core + Tauri 2 UI (WebView2), with strict footprint rules. WinUI 3 is the fallback if the footprint budget is missed.**

| Criterion | Tauri 2 (Rust + web UI) | WinUI 3 / .NET (C#) | C++ Win32 + Direct2D/DComp |
|---|---|---|---|
| Footprint | Small installer. RAM depends on the number of WebViews: one shared environment is fine, while one renderer per surface balloons (Seelen: 12 WebViews about 1.29 GB → 512 MB with shared process). lumabar shows a single dock is viable | Native XAML. Memory improving (WinAppSDK fixed a resource-growth bug, [Windows Latest 2026-10-01](https://www.windowslatest.com/2026/10/02/windows-11s-winui-just-got-basic-features-and-less-ram-hogging-after-years-of-losing-apps-to-the-web)). Self-contained builds are about 30 MB+ | Smallest RAM and binary; most code to write |
| Mica / Acrylic | `window-vibrancy`: `apply_mica` (Win11), `apply_acrylic` (Win10/11, laggy while dragging, a minor issue for a fixed dock). Or call `DwmSetWindowAttribute(DWMWA_SYSTEMBACKDROP_TYPE)` directly via windows-rs ([MS docs: Mica in Win32/WebView2](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-mica-win32)) | First-class `MicaController` / `DesktopAcrylicController`, theme-aware | DWM attributes by hand |
| Global hotkeys | `global-hotkey` crate (`RegisterHotKey`) plus a `WH_KEYBOARD_LL` hook via windows-rs for the Cmd layer | P/Invoke the same APIs (CsWin32), or port PowerToys' MIT Keyboard Manager | Native |
| Dock / appbar | Rust calls `SHAppBarMessage` (ABM_NEW / QUERYPOS / SETPOS) to reserve the bar's work area, `ABM_SETSTATE` to auto-hide the taskbar, `SetWinEventHook` for window lists | ManagedShell (Apache-2.0) already wraps appbar, tasks and tray | Native |
| Design freedom for "Cove's own interface" | Highest: matches the concept images, quick iteration, Framer-style motion | High but XAML-bound | Everything by hand |
| Win10 support | Yes (WebView2 Evergreen) | Yes (1809+) | Yes |
| Precedent | Seelen, lumabar, Booki | PowerToys (newer modules), Files | MyDockFinder-like native docks |

**Why Tauri 2 for Cove**
- The Rust core gives the full Win32 surface Cove needs: appbar, WinEvent hooks, LL keyboard hook, DWM backdrops, preview handlers, restore journal, all without elevation.
- The web UI gives the distinctive, non-Windows look the user wants, fast.
- Licences are permissive (MIT/Apache).
- lumabar (same stack, no admin, "close it and the taskbar is back") is proof it ships.

**Guardrails so Tauri stays smooth**
- One `CoreWebView2Environment`, shared by every surface (Microsoft's own guidance, [WebView2 perf](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/performance)).
- At most 2–3 always-on WebViews (dock, bar). Launcher and settings created on first use, then hidden rather than destroyed.
- Budget: under about 150 MB total private bytes idle, 0% idle CPU, 120 Hz animations. Measure every build, counting `msedgewebview2.exe` children.
- Native code handles all system work; the UI only renders.
- Handle sleep/resume and display changes in Rust. Never let a WebView crash take down the taskbar restore.

**Hotkey caveats (any stack)**
- `RegisterHotKey`: Win-key combos are reserved for the OS ([docs](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-registerhotkey)).
- So the Cmd layer must use a low-level hook (`WH_KEYBOARD_LL`) for Alt/Ctrl-based chords. Keep the callback tiny, or Windows drops the hook on timeout.
- A non-elevated Cove can't intercept keys in elevated windows (UIPI). Document it; don't elevate.
- Avoid Win+Space (input-language switcher) as the default launcher key. Offer Alt+Space or Ctrl+Space and detect conflicts with PowerToys or Flow.

**Switch to WinUI 3 if** measured idle RAM with two always-on surfaces can't get under budget, or the team prefers C#. Reuse ManagedShell, WinUIEx, H.NotifyIcon and the PowerToys code (all MIT/Apache).

---

## Gaps / not verified
- PowerToys per-module elevation needs; TranslucentTB injection details beyond the repo folder names.
- Exact registry/policy names for each clutter toggle on 26H2, and whether any are protected from third-party writes. To verify during build.
- MyDockFinder's current Steam score: "Very Positive" on the store page vs about 72% all-time on SteamDB/Steambase. The difference is likely recent vs all-time reviews.
- The "38 MB Tauri / 22 MB WinUI" benchmark ([Chameleon blog](https://chameleon-browser.com/blog/electron-vs-tauri-vs-native-presenter-architecture)) is a vendor blog. Treat as indicative only.
