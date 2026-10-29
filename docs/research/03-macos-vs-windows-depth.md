# 03 · macOS vs Windows 11, surface by surface (researched 2026-10-29)

For **Cove**: a safe, reversible Windows 11 app that gives Windows a calm, macOS-inspired UI and Mac habits through its own interface. It is not a macOS clone, uses no Apple assets, and does not patch system files.
Builds on `01-windows-state-and-pain-points.md` and `02-habits-pillars-tutorial.md`.

**Guiding principle (from the user, 2026-10-29):** Cove's defining trait is **smoothness and flexibility**, and it removes Windows UI clutter nobody uses. Section 15 lists the clutter and whether each item can be hidden safely. Section 16 covers what makes macOS feel smooth.

## 0. Versions as of Oct 2026 (one correction to the brief)
- **macOS 27 "Golden Gate" is no longer a beta.** It shipped **2026-09-14**. 27.0.1 is out, and 27.2 notes are on Apple's update page. ([Macworld](https://www.macworld.com/article/3139330/macos-27-mac-features-siri-apple-intelligence-release-date-compatibility.html), [Apple 127257](https://support.apple.com/en-us/127257)) Tahoe 26 is the previous release. Both are covered below.
- macOS 27 UI changes ([MacRumors, 50 features](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/), [MacRumors 250-change list](https://www.macrumors.com/2026/06/10/apple-lists-250-changes-ios-27-and-more/)):
  - Liquid Glass transparency slider ("ultra-clear to fully tinted")
  - Uniform toolbars, edge-to-edge sidebars, **consistent corner radii** (less rounded than Tahoe), and stronger window shadows so the active window is easier to see
  - A "Show Borders" accessibility option
  - Spotlight becomes a "Search or Ask" bar with Siri AI
  - A light-grey Dock indicator for apps that still run in the background after quitting
  - Extra-large desktop widgets
  - Finder column autosize/reset
- **Windows 11 26H2** reached general availability 2026-09-29 as an enablement package. It includes:
  - Movable taskbar (top, left, right) and a smaller taskbar
  - Redesigned Start: scrollable All, category and grid views, two sizes, and toggles for Pinned, Recent and All
  - Search with previews, typo tolerance, result-type labels and a web/Store toggle
  - File Explorer speed and AI actions
  - Sources: [MS Learn 26H2](https://learn.microsoft.com/en-us/windows/whats-new/whats-new-windows-11-version-26h2), [Insider blog May 15](https://blogs.windows.com/windows-insider/2026/05/15/improving-windows-quality-making-taskbar-and-start-more-personal/)
  - Limit: the search box and small taskbar are supported only at top and bottom ([Release Preview notes](https://learn.microsoft.com/en-us/windows-insider/release-notes/release-preview-24h2-25h2/build-26100-9267-26200-9267))
- **New Windows Search (WinUI 3)** is in Insider builds from 2026-10-07 and reaches everyone "later this year". It adds **inline actions** such as dark mode, Bluetooth, Do Not Disturb and arranging or minimizing windows. ([Insider blog](https://blogs.windows.com/windows-insider/2026/10/07/from-searching-to-doing-building-a-faster-more-streamlined-windows-search/), [Experience blog 10/15](https://blogs.windows.com/windowsexperience/2026/10/15/whats-new-in-windows-11-everyday-experiences-reimagined/))
- **Quieter Widgets by default** ([Neowin](https://www.neowin.net/news/latest-windows-11-26h1-release-preview-build-adds-quieter-widgets-and-new-touchpad-controls/), [Windows Central](https://www.windowscentral.com/microsoft/windows-11/i-tested-5-major-changes-and-improvements-coming-to-windows-11-this-year-microsoft-is-clearly-serious-about-fixing-windows-and-its-doing-things-right-this-time)):
  - No open-on-hover and no taskbar badges
  - Opens to Widgets rather than the MSN feed
  - Accent-coloured rather than red badges
  - On 8 GB PCs, Widgets load only when opened

**Safe-implementation vocabulary used below** (documented, user-space, reversible):
- Settings toggles and documented policies
- `RegisterHotKey`
- Window enumeration and events: `EnumWindows`, `SetWinEventHook`
- `ShowWindow` / `SetWindowPos` / `PostMessage(WM_CLOSE)` on other apps' top-level windows
- UI Automation (UIA) for reading menus and controls
- `SHAppBarMessage` for reserving screen edges ([Learn](https://learn.microsoft.com/en-us/windows/win32/shell/application-desktop-toolbars))
- DWM thumbnails, plus `DwmSetWindowAttribute` on **Cove's own** windows for corners and Mica ([Learn](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-rounded-corners), [backdrop enum](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/ne-dwmapi-dwm_systembackdrop_type))
- WinUI 3 / Windows App SDK for Cove's own UI

Never used: uxtheme/DLL patching, injection into explorer.exe, or undocumented COM. Cove never calls `TerminateProcess` except through an explicit "Force Quit" the user chooses.

---

## 1. Menu bar and global app menus
- **macOS:**
  - One fixed bar at the top. It holds the Apple menu, the bold active-app menu, standard menus (App, File, Edit, Format, View, app menus, Window, Help), and menu bar extras on the trailing side.
  - Apps must keep menu items visible (disabled rather than hidden), so the menu bar is how people discover what an app can do and its keyboard shortcuts. The Help menu has a search field that finds menu items.
  - The bar is 24 pt tall. It hides in full screen. ([HIG: The menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar))
  - Tahoe made the bar fully transparent, with an option to restore a background. ([Apple Newsroom](https://www.apple.com/newsroom/2025/06/macos-tahoe-26-makes-the-mac-more-capable-productive-and-intelligent-than-ever/), [AppleInsider](https://appleinsider.com/inside/macos-tahoe/tips/inside-control-center-on-macos-tahoe))
- **Windows now:**
  - No global menu. Each window draws its own commands: classic Win32 menus, ribbons, hamburger menus, or web UIs (Chrome, Electron, Office).
  - The taskbar can now sit at the **top** (26H2), but it is a task bar, not a menu bar.
- **Gap:** Windows has no single, predictable home for the current app's commands, and no consistent "app name" anchor.
- **Cove fill:**
  - A Cove **top bar** that reserves the top edge with `SHAppBarMessage`. The left side shows the bold foreground-app name (from the process and window) with a Cove app menu: Hide, Hide Others, Show All, Quit, Force Quit…
  - Where the foreground app exposes a **classic Win32 menu or a UIA MenuBar**, Cove mirrors it read-only and invokes items through UIA `InvokePattern`.
  - Otherwise the bar shows a universal command set: File/Edit actions sent as standard shortcuts (Ctrl+N/O/S/Z/C/V/F), View (full screen F11), Window (minimize, zoom, tile, "Bring All to Front").
  - **Honest limit:** most modern apps (Chrome/Edge, Electron, UWP) expose no menu tree, so a "true" global menu works for only a minority of apps. Present it as an **app strip + universal actions**, not a promise of every app's menus.
  - The right side holds Cove's own extras (see section 6).
  - Undo: unregister the appbar.

## 2. Dock vs taskbar
- **macOS Dock** ([Apple: Desktop & Dock settings](https://support.apple.com/guide/mac-help/change-desktop-dock-settings-mchlp1119/mac), [Use the Dock](https://support.apple.com/guide/mac-help/dock-mh35859/11.0/mac/11.0)):
  - Size and **magnification** sliders; position bottom, left or right; auto-hide; minimize effect; bouncing on launch
  - **Open-app indicator dots**; "show suggested and recent apps" (up to 3); a divider, then **Stacks** (folders shown as fan, grid or list), Downloads and Trash
  - Option-click hides the current app; Option-Cmd-click hides all others
  - macOS 27: a grey dot means "still running in the background after quit", with a "Stop Running in Background" option ([MacRumors](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/))
  - Tahoe changed the look, not what the Dock can do ([DockGroups](https://dockgroups.com/guides/macos-tahoe-dock))
- **Windows now** ([Customize the taskbar](https://support.microsoft.com/en-US/Windows/Experience/Personalization/customize-the-taskbar-in-windows)):
  - Taskbar on any edge (26H2), small taskbar, centred or left icons, and a "never combine" option
  - Running underline, jump lists, app-specific taskbar actions, an AI-agent progress monitor, and Snap groups on hover
  - Search, Task View, Widgets and the tray share the same bar
- **Gap:** the taskbar mixes launcher, running apps, system tray, search and widgets. It has no magnification and no folder Stacks. Its recents are jump lists rather than Dock recents.
- **Cove fill:**
  - A Cove **Dock**: its own appbar or floating layer window with Mica/acrylic on its own window via `DWMWA_SYSTEMBACKDROP_TYPE`
  - Pinned apps, a divider, then Stacks (any folder shown as fan, grid or list), Downloads and Trash (opens Recycle Bin)
  - Running dots come from `EnumWindows` + `SetWinEventHook`; launch bounce; optional "recent apps" slot
  - Click-to-focus and Option-style clicks: Alt-click hides the app, Alt+Ctrl-click hides the others
  - The native taskbar uses the **documented auto-hide setting** plus small taskbar. It stays recoverable, with no hacks to remove it.
  - Undo restores the user's saved taskbar settings.

## 3. Window controls and the window model
- **macOS** ([Mac keyboard shortcuts](https://support.apple.com/en-us/102650), [HIG menu bar](https://developer.apple.com/design/human-interface-guidelines/the-menu-bar)):
  - Controls sit at the top-left (close, minimize, zoom).
  - **The app and its windows are separate things.** Closing the last window often leaves the app running (Cmd-W vs Cmd-Q).
  - **Hide** (Cmd-H) removes all of an app's windows at once. Option-Cmd-H hides the others.
  - Cmd-M minimizes; Option-Cmd-W closes all of an app's windows; Cmd-` cycles one app's windows; Cmd-Tab switches **apps**.
- **Windows:**
  - Controls sit at the top-right. Hovering Maximize opens Snap layouts.
  - Closing the last window usually exits the app (some apps keep running in the tray).
  - No Hide. Alt+Tab switches **windows**. Corners are 8 px and become square when snapped or maximized ([Learn: Geometry](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry)).
- **Gap:** no app-level actions (Hide, Quit all windows, cycle same-app windows) and no app-grouped switcher.
- **Cove fill** (keys per `02`):
  - Alt+W sends `WM_CLOSE` to the foreground window. Alt+Q **gracefully** closes every top-level window of that process, so apps can still ask to save.
  - Alt+H minimizes all windows of the app. "Hide Others" does the same for every other app.
  - Alt+` cycles same-app windows. An app-grouped switcher draws Cove's own overlay from DWM thumbnails.
  - Cove does **not** move other apps' title-bar buttons. Left-side controls appear only in Cove-drawn surfaces such as the switcher and Exposé.
  - Hotkeys use `RegisterHotKey`.
  - Watch for conflicts: Alt+Space is Windows' system menu shortcut and is also used by PowerToys Run; Alt+H/W/Q may collide with app accelerators. Hotkeys should be remappable, with a "Win+" alternative profile.

## 4. Window management: tiling, Mission Control, Stage Manager, Spaces, App Exposé
- **macOS** ([Tile windows](https://support.apple.com/guide/mac-help/tile-app-windows-mchlef287e5d/mac), [Tiling shortcuts](https://support.apple.com/guide/mac-help/mac-window-tiling-icons-keyboard-shortcuts-mchl9674d0b0/mac), [Tiling settings](https://support.apple.com/guide/mac-help/change-window-tiling-settings-on-mac-mchl118087b0/mac), [TidBITS](https://tidbits.com/2024/11/08/how-to-tame-sequoias-window-tiling/), [Stage Manager](https://support.apple.com/guide/mac-help/use-stage-manager-mchl534ba392/mac)):
  - **Tiling (Sequoia 15+):** drag to an edge or corner; drag to the menu bar to fill; Option-drag for faster targets; the green button's hover menu (Halves, Quarters, Arrange, with Option for more)
  - Window ▸ Move & Resize menu; Fn-Ctrl-F (fill), Fn-Ctrl-C (centre), Fn-Ctrl-arrows (halves), Fn-Ctrl-Shift-arrows (arrange two)
  - Setting: "Tiled windows have margins"
  - **Mission Control** (Ctrl-↑ or 3/4-finger swipe up) shows every window plus the **Spaces** strip
  - **App Exposé** (Ctrl-↓) shows one app's windows
  - **Stage Manager** keeps the current app or group centred and recent apps as thumbnails on the left
  - Full screen and Split View create their own Space. macOS 27 adds smoother Mission Control animations and better restoring of windows and Spaces when displays reconnect.
- **Windows** ([Snap your windows](https://support.microsoft.com/en-us/windows/experience/snap-your-windows), [Group apps into desktops](https://support.microsoft.com/en-us/windows/group-apps-into-desktops-38d0c951-085c-e78d-52df-b41cfa835a37)):
  - Snap: drag to edge or corner, Snap bar at the top, Maximize hover flyout, Win+Z, Win+arrows, Snap Assist, **Snap groups**
  - Task View (Win+Tab) with **virtual desktops** (Win+Ctrl+D; Win+Ctrl+←/→)
  - New Search can "arrange windows"
  - Windows' snapping is *stronger* than macOS (layouts, groups). What's missing is an **app-centric overview** and a Stage-Manager-style focus mode.
- **Gap:** no App Exposé; no single gesture or overview that blends desktops and windows the way Mission Control does; no Stage Manager; no Option-drag style quick tiling; no "fill / centre" commands; no tile margins.
- **Cove fill:**
  - **Exposé for one app:** Cove overlay with live DWM thumbnails of one process's windows; click to focus via `SetForegroundWindow` rules.
  - **Overview:** Ctrl+↑ opens native Task View (documented hotkey passthrough); Cove does not rebuild desktops.
  - **Tiling extras:** Fill, Centre and tiles with margins via `SetWindowPos`, plus a hover menu on the Cove Dock.
  - **Focus mode (Stage-like):** minimize all other apps' windows and show a Cove side strip of recent apps. It is fully reversible because it only minimizes and restores.
  - Avoid duplicating Snap. Let Snap stay the base layer, and leave FancyZones to PowerToys users.
  - Virtual desktop **moves** use only the documented `IVirtualDesktopManager` (`MoveWindowToDesktop` works only for the caller's own windows). Cross-app desktop moves therefore stay with Task View; this is an honest limit.

## 5. Spotlight vs Windows Search, Start and launchers
- **macOS Tahoe 26 Spotlight** ([Apple features PDF](https://www.apple.com/os/pdf/All_New_Features_macOS_Tahoe_Sept_2025.pdf), [Ars review](https://arstechnica.com/gadgets/2025/09/macos-26-tahoe-the-ars-technica-review/), [MacStories](https://www.macstories.net/stories/macos-26-tahoe-the-macstories-review/3), [MacRumors](https://www.macrumors.com/2025/06/09/macos-tahoe-launchpad-app-library/)):
  - Cmd-Space, then browse modes: **Cmd-1 Apps, Cmd-2 Files, Cmd-3 Actions, Cmd-4 Clipboard** (history)
  - **Actions** come from App Intents, plus the frontmost app's **menu-bar items**
  - **Quick keys**: user-assignable short strings with no modifier (e.g. "sm" = send message, "ar" = add reminder)
  - Shortcuts run like apps; ranking is personalized
  - **macOS 27**: a "Search or Ask" bar with Siri AI (switchable to ChatGPT), multi-file ask, and faster indexing of actions ([MacRumors](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/), [The Verge](https://www.theverge.com/tech/943695/apple-wwdc-2026-macos-27-macbook-mac-announcement-features))
- **Windows now:**
  - 26H2 Search: previews, typo handling, type labels, local results first, web/Store toggle
  - **New WinUI 3 Search (Insider, rolling out late 2026)**: inline actions such as dark mode, night light, Bluetooth, DND, arranging windows, and soon phone actions
  - **PowerToys Command Palette** (Win+Alt+Space): apps, commands, files, calculator, clipboard history, extensions ([Learn](https://learn.microsoft.com/en-us/windows/powertoys/command-palette/overview))
  - Win+V clipboard history
- **Gap:** smaller than in 2025. What's still missing:
  - A centred, instant, keyboard-first launcher
  - **User quick keys/aliases**
  - Browse modes in one surface (apps / files / actions / clipboard)
  - Actions inside other apps, because few Windows apps expose intents
- **Cove fill:**
  - **Cove Search** (default Alt+Space, remappable): a centred panel with tabs for Apps, Files, Actions and Clipboard via Alt+1..4
  - **Quick keys**: user aliases for apps, folders, URLs, Cove actions and system actions. Example: "dm" toggles dark mode by writing the documented personalization setting.
  - Sources:
    - Start-menu app list (shell:AppsFolder)
    - Windows Search index through the documented Windows Search OLE DB / `SystemIndex` SQL interface
    - Clipboard via the `Windows.ApplicationModel.DataTransfer.Clipboard` history APIs (only when the user has enabled history)
    - Cove's own action registry
  - Optional "Open in Windows Search" fallback.
  - Don't compete on web or AI answers; keep it local and fast.

## 6. Control Center and menu bar extras vs Quick Settings and the tray
- **macOS Tahoe:**
  - Fully customizable **Control Center**: resize and rearrange controls, add **pages**, add third-party controls; any control can be **pinned directly to the menu bar**
  - All set in System Settings ▸ Menu Bar, with an option to hide menu bar extras ([Apple Newsroom](https://www.apple.com/newsroom/2025/06/macos-tahoe-26-makes-the-mac-more-capable-productive-and-intelligent-than-ever/), [Six Colors](https://sixcolors.com/post/2025/09/macos-26-tahoe-review-power-under-glass), [AppleInsider](https://appleinsider.com/inside/macos-tahoe/tips/inside-control-center-on-macos-tahoe))
  - macOS 27 adds Ethernet status to the menu bar
  - HIG: an extra should open a menu, not a popover; the system may hide extras when space is tight
- **Windows:**
  - **Quick Settings** (Win+A) with editable tiles; in Insider builds a "Mobile devices" phone section ([Insider build 26340.9767](https://learn.microsoft.com/en-us/windows-insider/release-notes/experimental/preview-build-26340-9767))
  - **System tray** plus an overflow chevron; per-icon show/hide in Settings ▸ Personalization ▸ Taskbar
- **Gap:** no pages and no way to pin one control (e.g. Bluetooth) as its own tray item. Third-party tray icons are inconsistent and have no unified layout.
- **Cove fill:**
  - **Cove Control Center**: Cove's own popover from the top bar with Wi-Fi, Bluetooth, volume, brightness, DND/Focus, dark mode, night light and battery.
  - It uses documented WinRT/Win32 APIs where they exist: `Windows.Devices.Radios`, `Windows.Media` / Core Audio endpoint volume, WMI monitor brightness for laptops. Otherwise it **deep-links** to the `ms-settings:` page.
  - Users can pin any control as a top-bar item.
  - Other apps' tray icons cannot be cleanly re-hosted through documented APIs. Cove leaves them in the native tray (reachable from a top-bar "⋯" that opens the overflow) rather than scraping them.

## 7. Notification Center and widgets
- **macOS:**
  - Notification Center opens from the clock (Fn-N) and holds grouped notifications and widgets
  - Desktop widgets since Sonoma; macOS 27 adds **Extra-Large Portrait** desktop widgets and real-time updates while the parent app is open ([MacRumors](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/))
  - Focus modes filter notifications
- **Windows:**
  - Notification Center plus calendar from the clock; Do Not Disturb and Focus
  - **Widgets board**, now quieter: no hover, no badges, MSN "Discover" feed can be turned off ([Widgets support](https://support.microsoft.com/en-us/windows/experience/personalization/stay-up-to-date-with-widgets-in-windows))
  - Lock-screen widgets reduced to Weather by default
  - Desktop widgets are not native in shipping Windows
- **Gap:** widgets live in a separate board rather than on the desktop or next to notifications, and history mixes in news.
- **Cove fill:**
  - Use the **native Notification Center** (the clock in Cove's top bar opens it through Win+N passthrough). Rebuilding notifications would need `UserNotificationListener` access, which is a privacy and permission burden.
  - Offer a small set of **Cove desktop widgets** (calendar, weather, battery, clock, now playing) as Cove-owned windows: calm, few, optional.
  - One-click "calm notifications" preset: DND schedule plus turning off tips and suggestions (section 15).

## 8. Finder vs File Explorer
| Feature | macOS Finder | Windows File Explorer (2026) | Gap and Cove option |
|---|---|---|---|
| Sidebar | Favourites, iCloud, Locations, **Tags** | Home, Gallery, OneDrive, This PC, network; customizable pins | Explorer is noisier (Gallery, OneDrive). Hide what isn't needed through documented navigation-pane options; no patching |
| Tabs | Yes (Cmd-T) | Yes, since 2022 | Parity |
| **Quick Look (Space)** | Space previews almost anything; markup, trim, rotate ([Apple](https://support.apple.com/en-in/guide/mac-help/mh14119/mac)) | Preview pane (Alt+P); **PowerToys Peek** can now use **Space** ([Learn: Peek](https://learn.microsoft.com/en-us/windows/powertoys/peek), [devblog 0.95](https://devblogs.microsoft.com/commandline/powertoys-0-95-is-here-new-light-switch-utility-faster-command-palette-and-peek-with-spacebar/)) | Recommend and configure PowerToys Peek (Space) rather than writing a previewer. A Cove previewer would need Explorer selection via Shell COM `IShellWindows`, which is documented but fiddly |
| Column view | Miller columns, preview column; macOS 27 autosize/reset | **None**; open PowerToys request ([#46064](https://github.com/microsoft/PowerToys/issues/46064)) | Real gap. Only buildable as a separate Cove browser window; defer |
| Tags | Coloured tags, sidebar, sort/group ([Apple](https://support.apple.com/guide/mac-help/tag-files-and-folders-mchlp15236/mac)) | No tags. NTFS "Tags" property only for some formats (Office, JPEG) | Gap. A Cove tag store (its own DB, or NTFS alternate data streams) is possible but invisible to Explorer; low priority |
| Gallery / icon look | Folder colours, emoji/symbols (Tahoe) | Gallery view; folder icon via Properties ▸ Customize (desktop.ini) | Folder colouring through the **documented** desktop.ini route is reversible |
| Context menu | One menu | Split menu ("Show more options"); customizable menu in preview | Wait for the native fix; no shell-extension hacks |

## 9. Launchpad / Apps vs Start
- **macOS Tahoe** removed Launchpad. **Apps** (Dock icon, pinch gesture, or Spotlight Cmd-1) shows suggested apps on top, then all apps, auto-categorized, in grid or list. There are no custom folders. ([MacRumors](https://www.macrumors.com/2025/09/24/all-the-new-macos-tahoe-features/), [9to5Mac](https://9to5mac.com/2025/09/15/launchpad-macos-tahoe-26-alternative/), [AppleInsider](https://appleinsider.com/inside/macos-tahoe/tips/how-to-bring-back-launchpad-on-macos-tahoe))
- **Windows 26H2 Start** is now close to this: scrollable All with **category and grid views**, sizes, and section toggles ([Customize Start](https://support.microsoft.com/en-us/windows/experience/personalization/customize-the-windows-start-menu)). Since July it shows no ads in Recent once the tips toggle is off.
- **Gap:** small. Windows already has the category grid. What's missing is a calm, full-screen grid with **user folders** (which Apple itself removed).
- **Cove fill:** make Cove Search ▸ Apps the launcher, with a Dock "Apps" tile and optional user groups (an advantage over Tahoe). Recommend the native Start setup: hide Pinned or Recent and turn off recommendations.

## 10. Keyboard shortcuts and trackpad gestures
- **Mac shortcut mapping:** see `02` (Cmd-W/Q/H/M/`, Cmd-Space, Ctrl-↑/↓, Cmd-Shift-4, Ctrl-Cmd-Q). Sources: [Apple 102650](https://support.apple.com/en-us/102650); Windows: [keyboard shortcuts](https://support.microsoft.com/en-us/accessibility/windows/keyboard-shortcuts-in-windows).
- **Key structural difference:** the Mac's Cmd key separates *app* shortcuts from Ctrl. On Windows, Ctrl does app work and Win does system work, so Cove should map Mac **system** habits to Alt or Win chords and never remap Ctrl app shortcuts globally.
- **Gestures:**
  - Mac: 3/4-finger up for Mission Control, down for App Exposé, sideways to switch Spaces, pinch for Apps, two-finger Notification Center swipe; macOS 27 adds pull-to-refresh in Apple apps.
  - Windows precision touchpads already map 3/4-finger up to Task View, down to desktop, and sideways to switch apps or desktops. Each gesture is **configurable in Settings ▸ Bluetooth & devices ▸ Touchpad ▸ Advanced gestures**, including assigning a keyboard shortcut.
- **Cove fill:** a "Mac gestures" preset that points the documented advanced-gesture slots at Cove's hotkeys (e.g. 3-finger down = Cove App Exposé). This is pure settings and fully reversible.
  - I did not re-fetch the touchpad settings page this run; the pane is long-standing in Windows 11. Verify the exact labels when implementing.

## 11. Typography, spacing, corner radii, visual hierarchy
| | macOS | Windows 11 |
|---|---|---|
| System font | SF Pro (variable, dynamic optical sizes) ([HIG Typography](https://developer.apple.com/design/human-interface-guidelines/typography)) | Segoe UI Variable (weight + optical axes) ([Learn Typography](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/typography)) |
| Body text | **13 pt** regular (min 10); headline 13 bold; title 1 22 pt | **14 epx** regular body; caption 12; subtitle 20; title 28 semibold |
| Weights | Avoid light weights; bold for headline | Regular + Semibold only; no Bold or Italic in the ramp |
| Menu bar | 24 pt high | Taskbar 48 epx default (smaller option in 26H2) |
| Corners | Tahoe large and varied; **27 makes window radii consistent and less rounded** | **8 px** top-level/flyouts, **4 px** controls, 0 when snapped or maximized ([Geometry](https://learn.microsoft.com/en-us/windows/apps/design/signature-experiences/geometry)) |

**Why macOS feels clean and Windows feels cluttered:**
1. **One design system everywhere.** Mac apps use AppKit/SwiftUI controls, and the HIG requires a standard menu order. Windows mixes Win32, WinForms, WPF, WinUI, Electron and web, plus legacy Control Panel next to Settings, so fonts, density and corners all vary.
2. **Fixed positions.** Menus are always at the top, window controls always top-left, the Dock always holds apps. Windows puts commands inside each window, in different places.
3. **Fewer competing surfaces.** Windows ships extras that ask for attention: the search box, Widgets, Copilot, account and tip badges, lock-screen facts, recommendations (section 15).
4. **Quiet hierarchy.** Small body text with weight and tone carrying hierarchy; translucency used for chrome and content kept opaque. Windows apps often use more borders, separators and coloured icons.
5. **Restraint by default.** Mac defaults are minimal. Windows defaults lean toward discovery and promotion, although 2026 Windows is undoing much of this.

**Cove design rules:**
- Use Segoe UI Variable, not SF, which is Apple-licensed and must not be bundled.
- Body 13–14 epx; Semibold for titles; **one** corner scale (e.g. 10 px panels, 6 px controls) applied across all Cove surfaces.
- An 8-pt spacing grid; Mica/acrylic only on Cove's own chrome; content stays opaque.
- Monochrome symbols from Segoe Fluent Icons or Cove's own set, never SF Symbols.
- Respect Windows light/dark and accent colour.

## 12–14. Quick reference: gap size
| Surface | Native Windows (Oct 2026) | Gap | Cove priority |
|---|---|---|---|
| Menu bar / app menu | none | **large** | P1 (app strip + universal actions; menus where UIA allows) |
| Dock | taskbar (movable) | **large** (look and model) | P1 |
| App model (hide/quit/cycle) | none | **large** | P1 |
| Spotlight-style launcher | strong and improving | medium (quick keys, browse modes) | P1 |
| Control Center | Quick Settings | medium | P2 |
| App Exposé / focus mode | Task View, Snap | medium | P2 |
| Quick Look | PowerToys Peek | small | P2 (configure, don't build) |
| Notifications / widgets | native + quieter | small | P3 |
| Column view / tags | none | real but costly | P3 / later |
| Launchpad/Apps | Start grid/category | small | fold into Search |

## 15. Clutter: Windows UI elements that cost attention for little use
"Safe" means it can be hidden through a **documented Settings toggle or Group Policy**, and Cove can record the old value and restore it.

| Element | Why it's clutter | Hide via documented setting? | Path / note |
|---|---|---|---|
| Taskbar **Search box** | Large, duplicates Win key and Cove Search | **Yes** | Settings ▸ Personalization ▸ Taskbar ▸ Search (hide / icon) ([MS](https://support.microsoft.com/en-US/Windows/Experience/Personalization/customize-the-taskbar-in-windows)) |
| **Task View** button | Win+Tab / gesture does the same | **Yes** | Taskbar items |
| **Widgets** button / weather | Rarely used, takes memory | **Yes** | Taskbar items. Inside Widgets: Discover feed **Off**, badges **Off**, open on hover **Off** ([MS](https://support.microsoft.com/en-us/windows/experience/personalization/stay-up-to-date-with-widgets-in-windows)) |
| Copilot entry points | Unrequested AI | **Partly** | Copilot key remappable (Settings ▸ Personalization ▸ Text input); toggles vary by build. Verify per build |
| Start **Recommended/Recent** tips and app promos | Ads and noise | **Yes** | Settings ▸ Personalization ▸ Start ▸ "Show recommendations for tips, shortcuts, new apps, and more"; 26H2 can hide Pinned/Recent/All sections; GPO "Remove Recommended section" (Edu/Ent) ([Windows Central](https://www.windowscentral.com/microsoft/windows-11/windows-11s-september-update-reshapes-key-parts-of-the-os-and-these-3-customizations-instantly-make-the-new-release-feel-faster-cleaner-and-more-personal-to-use), [windows11.io](https://windows11.io/disable-windows-11-ads-and-suggestions/)) |
| Start **account notifications** badge | Nag | **Yes** | Personalization ▸ Start ▸ account-related notifications |
| **Search highlights** / web & Store suggestions | Trending/web noise | **Yes** | Privacy & security ▸ Search ▸ Show search highlights; 26H2 web/Store toggle ([Engadget](https://www.engadget.com/2265283/improve-windows-11-experience-change-these-settings/)) |
| Lock screen **fun facts**, Spotlight links, widgets | Promo text before sign-in | **Yes** | Personalization ▸ Lock screen: Picture/Slideshow, uncheck fun facts, Widgets off, "Suggest widgets" off ([Windows Central](https://www.windowscentral.com/microsoft/windows-11/windows-11-features-i-disable-immediately-for-a-quieter-experience)) |
| Notification **tips & suggestions**, welcome experience | Nag toasts | **Yes** | System ▸ Notifications ▸ Additional settings (all boxes) ([pascalreischl](https://pascalreischl.com/en/blog/distraction-free-windows-setup/)) |
| **Personalized offers** / Device usage | Feeds the above | **Yes** | Privacy & security ▸ Recommendations & offers; Personalization ▸ Device usage (all off) ([MS](https://support.microsoft.com/en-us/windows/experience/personalization/personalize-your-windows-experience-with-device-usage-settings)) |
| Explorer **sync provider notifications** (OneDrive promos) | Upsell | **Yes** | Folder Options ▸ View ▸ "Show sync provider notifications" |
| Explorer roomy spacing | Low density, busy | **Yes** | Explorer ▸ View ▸ Compact view |
| Tray **overflow** icons | Visual noise | **Yes** | Taskbar ▸ Other system tray icons (per app) |
| Desktop icons (Recycle Bin etc.) | Busy desktop | **Yes** | Personalization ▸ Themes ▸ Desktop icon settings; or right-click desktop ▸ View ▸ hide icons |
| Snap flyout on Maximize hover | Pops up by accident | **Yes** (but useful) | System ▸ Multitasking ▸ Snap windows options; default **on** in Cove |
| Taskbar app/agent actions (26H2) | New surface | **Unverified** | Check for a toggle before shipping |
| Split context menu ("Show more options") | Two menus | **No** documented unify switch | Wait for the native customizable menu; **don't** use the registry CLSID hack |
| Settings Home M365/Game Pass cards | Upsell | **Unverified/partial** | No documented toggle found this run; leave alone |
| Edge/OneDrive setup nags | Upsell | **Partial** | Mostly the offers and notifications toggles above |

**Cove "Calm" preset:** one switch that applies every **Yes** row and saves each prior value to a restore manifest. "Restore" writes them back. Nothing marked No or Unverified ships until it is verified.

## 16. What makes macOS feel smooth, and Cove's smoothness rules
**Sources of smoothness:**
- **Purposeful, short, cancellable motion** (HIG): "Add motion purposefully… Aim for brevity and precision… avoid adding motion to UI interactions that occur frequently… Let people cancel motion… don't make people wait for an animation to complete." ([HIG Motion](https://developer.apple.com/design/human-interface-guidelines/motion))
- **Motion that matches the input:** Liquid Glass responds more strongly to touch and more subtly to trackpad (HIG Motion). Trackpad gestures track the finger 1:1, so Mission Control and Space swipes follow the fingers, and the animation *is* the feedback.
- **One motion language across the system:** Dock magnification, Genie/scale minimize, Mission Control, the Spotlight fade. macOS 27 focused on performance, with "smoother Mission Control and Spaces animations" and a faster Apps drawer fade ([MacRumors](https://www.macrumors.com/2026/09/17/new-features-changes-macos-27/), [Low End Mac](https://lowendmac.com/2026/macos-27-beta-2-is-out-a-glance-at-build-26a5368g/)).
- **Low perceived latency on frequent actions:** Spotlight and Quick Look open instantly and are keyboard-first; the user never waits on the network for a local task.
- **Consistency:** the same places, shortcuts and menu order everywhere (HIG), so muscle memory removes friction.
- **Windows' own diagnosis:** Microsoft's 2026 work targets responsiveness ("up to 20% more responsive", Search rebuilt in WinUI 3 for speed and lower resource use, Widgets lazy-loaded on 8 GB PCs) ([Experience blog](https://blogs.windows.com/windowsexperience/2026/10/15/whats-new-in-windows-11-everyday-experiences-reimagined/)). Jank on Windows comes from mixed frameworks, web-backed shell surfaces and background load.

**Cove smoothness rules** (design targets, not measured facts):
1. Hot surfaces (Search, Dock, switcher, Exposé, Control Center) stay **preloaded and hidden**. Target: Search visible in ≤100 ms of the keypress, and first results from local data only.
2. **One motion system:** about 150–250 ms ease-out/spring for opening; dismissals faster than openings; every animation interruptible; no animation on high-frequency actions such as typing in Search or hotkey window moves.
3. Follow the **Windows "Animation effects" setting** (`SystemParametersInfo` `SPI_GETCLIENTAREAANIMATION`) and reduce motion when it's off.
4. Render on the **display's refresh rate** with GPU composition (WinUI 3 / Composition). No polling loops; react to `SetWinEventHook` events.
5. **Light footprint:** Cove must not cost what Widgets did. Budget roughly ≤150 MB RAM and near-zero idle CPU, which matters for 8 GB PCs per `01`.
6. **Flexibility:** every feature is a toggle, every hotkey remappable, every preset reversible (restore manifest) — plus the replayable tutorial from `02`.

## 17. Top 5 opportunities for Cove (ranked)
1. **App model + Mac habits layer:** Hide, Quit, cycle same-app windows, app-grouped switcher and App Exposé. This is the largest gap, uses only safe APIs, and shows immediately.
2. **Top bar + Dock pair, built on documented settings:** the Cove top bar (app name, app actions, Cove Control Center, clock that opens native notifications) plus the Cove Dock (magnification, dots, Stacks), with the native taskbar auto-hidden by its own setting.
3. **Calm preset (section 15):** one-click clutter removal through documented toggles, with exact restore. This is the "smoothness and flexibility" principle made concrete.
4. **Cove Search with quick keys and browse modes:** a launcher with Apps, Files, Actions and Clipboard tabs, user aliases, and instant local results. Windows' new Search covers system actions, so Cove's angle is speed, aliases and one keyboard surface.
5. **One consistent visual and motion system across every Cove surface** (section 11 and 16 rules), plus a "Mac gestures" touchpad preset. Configure PowerToys Peek (Space) rather than rebuilding Quick Look.

## Open items / not verified this run
- Apple Support tiling pages returned HTTP 429. Their facts come from search excerpts of support.apple.com, corroborated by Macworld, TidBITS and Computerworld.
- Not fetched this run; verify before shipping: the exact label of the touchpad "Advanced gestures" settings pane, a Copilot taskbar toggle on 26H2, a toggle for taskbar agent actions, and Settings-home upsell controls.
- I did not re-open the Learn pages for these standard Win32/WinRT APIs this run: RegisterHotKey, SetWinEventHook, UIA, IVirtualDesktopManager, Windows Search SystemIndex, Clipboard history API, SPI_GETCLIENTAREAANIMATION.
- Global-menu coverage depends on each app exposing UIA menus. Prototype against Notepad, Paint, Office, Chrome, VS Code and Explorer before promising anything.
