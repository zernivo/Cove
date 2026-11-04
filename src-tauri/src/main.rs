// Cove: Windows, made calm.
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod appbar;
mod appnames;
mod clipboard;
mod controls;
mod dock;
mod files;
mod hotkeys;
mod icons;
mod journal;
mod registry;
mod search;
mod settings;
mod shellprops;
mod shell;
mod stacks;
mod switcher;
mod expose;
mod glass;
mod snap;
mod winindex;
mod theme;
mod wallpaper;
mod topbar;
mod tweaks;
mod updater;

use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WindowEvent};

use tweaks::{calm, TweakInfo};

/// Shared app state. Every system change goes through the journal.
struct AppState {
    journal: Mutex<journal::Journal>,
    settings: Mutex<settings::Store>,
    just_updated: std::sync::atomic::AtomicBool,
}

// ---- Safe mode ----
// Cove starts with every system-touching feature paused (dock, top bar,
// shortcuts, switcher, snapping, clipboard history, wallpaper schedule).
// Saved settings are untouched; leaving safe mode puts everything back.
// Enter it by holding Shift while Cove starts, with `--safe`, from the tray,
// or automatically after two launches in a row that didn't last 30 seconds.

static SAFE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SAFE_REASON: Mutex<String> = Mutex::new(String::new());

fn safe() -> bool {
    SAFE.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(target_os = "windows")]
fn shift_held() -> bool {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
    (unsafe { GetAsyncKeyState(0x10) } as u16 & 0x8000) != 0
}

#[cfg(not(target_os = "windows"))]
fn shift_held() -> bool {
    false
}

/// Decide at startup. The marker counts launches that haven't yet lasted
/// 30 seconds; it's cleared once Cove has run that long or quits cleanly.
fn safe_decide(dir: &std::path::Path) -> Option<&'static str> {
    let marker = dir.join("launch.marker");
    let quick: u32 = std::fs::read_to_string(&marker).ok().and_then(|t| t.trim().parse().ok()).unwrap_or(0);
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&marker, (quick + 1).to_string());
    if std::env::args().any(|a| a == "--safe") {
        Some("flag")
    } else if shift_held() {
        Some("shift")
    } else if quick >= 2 {
        Some("crash")
    } else {
        None
    }
}

fn safe_marker_clear(app: &AppHandle) {
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::remove_file(dir.join("launch.marker"));
    }
}

/// Apply every feature from Settings, honouring safe mode.
fn apply_features(app: &AppHandle) {
    let state = app.state::<AppState>();
    let d = match state.settings.lock() {
        Ok(s) => s.data.clone(),
        Err(_) => return,
    };
    let on = !safe();
    hotkeys::configure(on && d.hotkeys, on && d.search);
    clipboard::set_enabled(on && d.clipboard);
    switcher::set_enabled(on && d.switcher);
    snap::configure(on && d.snap, d.snap_animate);
    topbar_apply(app, d.topbar);
    dock_apply(app, &state);
    tray_sync(app);
    let _ = app.emit("safe-changed", safe());
    let _ = app.emit("settings-changed", ());
}

#[derive(Serialize)]
struct SafeStatus {
    safe: bool,
    reason: String,
}

#[tauri::command]
fn safe_status() -> SafeStatus {
    SafeStatus { safe: safe(), reason: SAFE_REASON.lock().map(|r| r.clone()).unwrap_or_default() }
}

#[tauri::command]
fn safe_set(on: bool, app: AppHandle) -> SafeStatus {
    SAFE.store(on, std::sync::atomic::Ordering::SeqCst);
    if let Ok(mut r) = SAFE_REASON.lock() {
        *r = if on { "manual".into() } else { String::new() };
    }
    apply_features(&app);
    safe_status()
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        keep_on_screen(app, &w);
        let _ = w.set_focus();
    }
    dock_changed(app);
}

/// Cove was opened again while running: bring Settings forward at once and
/// give a short bounce of Cove's dock icon, so the click visibly did something.
fn second_launch(app: &AppHandle) {
    show_main(app);
    if let Some(d) = app.get_webview_window("dock") {
        let _ = d.emit("dock-bounce", "cove");
    }
}

#[derive(Serialize)]
struct Status {
    version: String,
    journal_entries: usize,
    windows: bool,
}

#[derive(Serialize)]
struct ApplyReport {
    applied: Vec<&'static str>,
    /// (tweak id, reason). The UI offers the Settings page for these.
    failed: Vec<(String, String)>,
}

#[derive(Serialize)]
struct RestoreReport {
    /// Windows settings put back from the journal.
    restored: usize,
    /// Cove features switched off (dock, top bar, switcher, ...).
    features: usize,
    failed: Vec<(String, String)>,
}

#[tauri::command]
fn cove_status(state: tauri::State<'_, AppState>) -> Status {
    let entries = state.journal.lock().map(|j| j.len()).unwrap_or(0);
    Status {
        version: env!("CARGO_PKG_VERSION").to_string(),
        journal_entries: entries,
        windows: cfg!(windows),
    }
}

#[tauri::command]
fn calm_list() -> Vec<TweakInfo> {
    calm::TWEAKS.iter().map(|t| t.info()).collect()
}

#[tauri::command]
fn calm_apply(ids: Vec<String>, state: tauri::State<'_, AppState>) -> ApplyReport {
    let mut journal = state.journal.lock().expect("journal lock");
    let mut report = ApplyReport { applied: Vec::new(), failed: Vec::new() };
    for id in ids {
        match calm::find(&id) {
            Some(t) => match t.apply(&mut journal) {
                Ok(()) => report.applied.push(t.id),
                Err(e) => report.failed.push((id, e.to_string())),
            },
            None => report.failed.push((id, "unknown tweak".into())),
        }
    }
    shell::notify_settings_changed();
    report
}

/// Put back every change Cove made, newest first.
#[tauri::command]
fn restore_all(app: AppHandle, state: tauri::State<'_, AppState>) -> Result<RestoreReport, String> {
    let mut journal = state.journal.lock().map_err(|e| e.to_string())?;
    let changes = journal.take_for_restore().map_err(|e| e.to_string())?;
    let mut report = RestoreReport { restored: 0, features: 0, failed: Vec::new() };
    for c in changes {
        match registry::write(&c.target, &c.previous) {
            Ok(()) => report.restored += 1,
            Err(e) => report.failed.push((c.target, e.to_string())),
        }
    }
    shell::notify_settings_changed();
    // The dock and its taskbar auto-hide count as Cove changes too.
    drop(journal);
    if let Ok(mut s) = state.settings.lock() {
        if s.data.dock || s.data.taskbar_prev_autohide.is_some() {
            s.data.dock = false;
            sync_taskbar(&mut s);
            report.features += 1;
        }
    }
    if let Some(w) = app.get_webview_window("dock") {
        let _ = w.hide();
    }
    dock_release(&app);
    let had_bar = state.settings.lock().map(|mut s| {
        let was = s.data.topbar;
        s.data.topbar = false;
        let _ = s.save();
        was
    }).unwrap_or(false);
    if had_bar {
        report.features += 1;
    }
    topbar_apply(&app, false);
    let had_switcher = state.settings.lock().map(|mut s| {
        let was = s.data.switcher;
        s.data.switcher = false;
        let _ = s.save();
        was
    }).unwrap_or(false);
    if had_switcher {
        switcher::set_enabled(false);
        report.features += 1;
    }
    let had_snap = state.settings.lock().map(|mut s| {
        let was = s.data.snap;
        s.data.snap = false;
        let _ = s.save();
        was
    }).unwrap_or(false);
    if had_snap {
        snap::configure(false, true);
        report.features += 1;
    }
    match wallpaper_put_back(&state) {
        Ok(true) => report.features += 1,
        Ok(false) => {}
        Err(e) => report.failed.push(("Wallpaper".into(), e)),
    }
    if let Ok(mut s) = state.settings.lock() {
        s.data.schedule = "off".into();
        let _ = s.save();
    }
    Ok(report)
}

// ---- Cove top bar ----

/// Menu bar height in DIP (macOS: 24 pt). The reserved AppBar strip is
/// exactly this, scaled to the main display; topbar.css/ts use the same 25.
const TOPBAR_HEIGHT: f64 = 24.0;

fn topbar_apply(app: &AppHandle, on: bool) {
    let on = on && !safe();
    let Some(w) = app.get_webview_window("topbar") else { return };
    let Ok(hwnd) = w.hwnd() else { return };
    let h = hwnd.0 as isize;
    if on {
        // The bar lives on the main display, so use that display's scale.
        let sf = app
            .primary_monitor()
            .ok()
            .flatten()
            .map(|m| m.scale_factor())
            .unwrap_or_else(|| w.scale_factor().unwrap_or(1.0));
        // The bar sits on exactly the strip ABM_SETPOS returned: the
        // monitor's top edge. Placed with a synchronous SetWindowPos after
        // showing (tao's own moves are asynchronous and in its coordinates),
        // and checked again by display_watch.
        let rc = topbar::register(h, (TOPBAR_HEIGHT * sf).round() as i32);
        let _ = w.show();
        appbar::place(h, rc);
    } else {
        topbar::unregister(h);
        let _ = w.hide();
        popover_close(app);
    }
}

#[tauri::command]
fn topbar_status(state: tauri::State<'_, AppState>) -> bool {
    state.settings.lock().map(|s| s.data.topbar).unwrap_or(false)
}

#[tauri::command]
fn topbar_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> bool {
    if let Ok(mut s) = state.settings.lock() {
        s.data.topbar = enabled;
        let _ = s.save();
    }
    topbar_apply(&app, enabled);
    tray_sync(&app);
    enabled
}

#[tauri::command]
async fn topbar_state() -> topbar::TopState {
    topbar::state()
}

// ---- Menu bar popovers: Control Centre and the bar's menus ----
//
// Each opens in its own small transparent window ("popover"), sized to the
// popover itself, so nothing but the popover shows. The menu bar window
// never grows. Clicking anywhere else (the popover loses focus) closes it.

struct PopState {
    /// What's open now ("cc", "cove", "app", "confirm").
    open: Option<String>,
    /// Where it hangs from: x in DIP from the bar's left, and which edge.
    x: f64,
    right: bool,
    /// Dock menus: the anchor point on screen (physical px): the icon's
    /// centre and the dock's top edge. None for bar menus.
    at: Option<(i32, i32)>,
    /// The last popover closed because you clicked away, and when: a click
    /// on the same bar item that caused it must not open it again.
    blurred: Option<(String, std::time::Instant)>,
}

static POP: Mutex<PopState> = Mutex::new(PopState { open: None, x: 0.0, right: false, at: None, blurred: None });

fn popover_state_changed(app: &AppHandle) {
    let open = POP.lock().ok().and_then(|p| p.open.clone());
    // The bar shows which title is open; the dock keeps itself up while
    // its own menu is open.
    let _ = app.emit("popover-state", open);
}

fn popover_close(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("popover") {
        let _ = w.hide();
    }
    if let Ok(mut p) = POP.lock() {
        p.open = None;
    }
    let _ = app.emit_to("popover", "popover-hidden", ());
    popover_state_changed(app);
}

/// Open (or close, when it's already open) a popover. `x` is the anchor in
/// DIP from the menu bar's left edge; `right` hangs the popover's right edge
/// there. `data` goes to the popover page (the app menu's front app).
#[tauri::command]
fn popover_toggle(kind: String, x: f64, right: bool, data: Option<serde_json::Value>, app: AppHandle) {
    let open_now = {
        let Ok(mut p) = POP.lock() else { return };
        if p.open.as_deref() == Some(kind.as_str()) {
            None
        } else if p.blurred.as_ref().is_some_and(|(k, t)| *k == kind && t.elapsed() < std::time::Duration::from_millis(350)) {
            // This very click closed it (the popover lost focus to the bar).
            p.blurred = None;
            return;
        } else {
            p.x = x;
            p.right = right;
            p.at = None;
            Some(())
        }
    };
    if open_now.is_none() {
        popover_close(&app);
        return;
    }
    // The app menu shows how many windows the app has ("Show All Windows 3")
    // and the app's real icon; the Cove menu says whose session it is.
    let data = match data {
        Some(mut d) => {
            if kind == "app" {
                let h = d.get("hwnd").and_then(|v| v.as_i64()).unwrap_or(0) as isize;
                let n = if h != 0 { app_windows(h).len() } else { 0 };
                d["count"] = serde_json::json!(n);
                if let Some(w) = dock::windows().into_iter().find(|w| w.hwnd == h) {
                    let mut keys: Vec<String> = w.aumid.as_deref().and_then(dock::package_logo_key).into_iter().collect();
                    keys.extend(w.icon_keys());
                    let mut jobs = Vec::new();
                    d["icon"] = serde_json::json!(dock::icon_from(&keys, &mut jobs));
                    if !jobs.is_empty() {
                        std::thread::spawn(move || icons::fill(jobs));
                    }
                }
            }
            if kind == "cove" {
                d["user"] = serde_json::json!(topbar::user_first_name());
            }
            Some(d)
        }
        None => None,
    };
    // The page renders, measures itself and calls popover_place to show.
    let _ = app.emit_to("popover", "popover-show", serde_json::json!({ "kind": kind, "data": data }));
}

/// A dock icon's context menu: the shared glass menu, centred on the icon
/// (`x`, dock window DIP) and hanging 10 DIP above the dock's top (`y`).
#[tauri::command]
fn popover_at(kind: String, x: f64, y: f64, data: Option<serde_json::Value>, app: AppHandle) {
    let Some(d) = app.get_webview_window("dock") else { return };
    let (Ok(pos), Ok(sf)) = (d.outer_position(), d.scale_factor()) else { return };
    if let Ok(mut p) = POP.lock() {
        p.at = Some((pos.x + (x * sf).round() as i32, pos.y + (y * sf).round() as i32));
        p.blurred = None;
    }
    let _ = app.emit_to("popover", "popover-show", serde_json::json!({ "kind": kind, "data": data }));
}

/// The popover page has rendered `kind` at `width` x `height` DIP: size the
/// window to exactly that, shape it to the panels shown (`shapes`: x, y, w,
/// h, radius in DIP; a menu and its open submenu are two), place it and
/// show it. Bar menus hang 3 DIP under the bar from their title (Control
/// Centre 6 DIP under it, 8 DIP from the right); dock menus centre their
/// main panel (`anchor_w` wide) on the icon, 10 DIP above the dock.
#[tauri::command]
fn popover_place(kind: String, width: f64, height: f64, shapes: Option<Vec<[f64; 5]>>, anchor_w: Option<f64>, app: AppHandle) {
    let Some(w) = app.get_webview_window("popover") else { return };
    let (x, right, at) = POP.lock().map(|p| (p.x, p.right, p.at)).unwrap_or((0.0, false, None));
    let (pw, ph, left, top, mon) = if let Some((ax, ay)) = at {
        let m = dock::monitor_at(ax, ay).or_else(dock::dock_monitor).or_else(dock::primary);
        let Some(m) = m else { return };
        let sf = w.monitor_from_point(ax as f64, ay as f64).ok().flatten().map(|m| m.scale_factor()).unwrap_or(1.0);
        let (pw, ph) = ((width * sf).ceil() as i32, (height * sf).ceil() as i32);
        let aw = (anchor_w.unwrap_or(width) * sf).round() as i32;
        let left = ax - aw / 2;
        let top = ay - (10.0 * sf).round() as i32 - ph;
        (pw, ph, left, top.max(m.monitor.1), m)
    } else {
        let Some(m) = dock::primary() else { return };
        let sf = app.primary_monitor().ok().flatten().map(|m| m.scale_factor()).unwrap_or(1.0);
        let bar = app
            .get_webview_window("topbar")
            .and_then(|t| t.hwnd().ok())
            .and_then(|h| appbar::held(h.0 as isize))
            .unwrap_or((m.monitor.0, m.monitor.1, m.monitor.2, m.monitor.1 + (TOPBAR_HEIGHT * sf).round() as i32));
        let (pw, ph) = ((width * sf).ceil() as i32, (height * sf).ceil() as i32);
        let ax = bar.0 + (x * sf).round() as i32;
        let left = if right { ax - pw } else { ax };
        // Menus hang right under the bar; Control Centre floats 6 DIP below it.
        let gap = if kind == "cc" { (6.0 * sf).round() as i32 } else { (3.0 * sf).round() as i32 };
        (pw, ph, left, bar.3 + gap, m)
    };
    let sf = w.scale_factor().unwrap_or(1.0);
    let edge = (4.0 * sf).round() as i32;
    let left = left.clamp(mon.monitor.0 + edge, (mon.monitor.2 - pw - edge).max(mon.monitor.0));
    let _ = w.show();
    let hwnd = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    appbar::place(hwnd, (left, top, left + pw, top + ph));
    // Clip the acrylic to the rounded panels (DWM only rounds to 8 px).
    if let Some(shapes) = shapes {
        let px: Vec<(i32, i32, i32, i32, i32)> = shapes
            .iter()
            .map(|s| ((s[0] * sf).round() as i32, (s[1] * sf).round() as i32, (s[2] * sf).round() as i32, (s[3] * sf).round() as i32, (s[4] * sf).round() as i32))
            .collect();
        glass::round_region(hwnd, &px);
    }
    let _ = w.set_focus();
    // Ask the page for a fresh paint once Windows has really shown the
    // window: a WebView can put the last frame it drew while hidden on
    // screen first, which was the empty grey rectangle of the Cove menu.
    {
        let h = app.clone();
        std::thread::spawn(move || {
            for ms in [40u64, 160] {
                std::thread::sleep(std::time::Duration::from_millis(ms));
                let _ = h.emit_to("popover", "popover-repaint", ());
            }
        });
    }
    if let Ok(mut p) = POP.lock() {
        p.open = Some(kind);
        p.blurred = None;
    }
    popover_state_changed(&app);
}

/// A dock menu item was picked in the popover: the dock runs it.
#[tauri::command]
fn dock_menu_pick(id: String, app: AppHandle) {
    let _ = app.emit_to("dock", "dock-menu-pick", id);
}

/// The look changed: tint the acrylic behind the popovers and Search to
/// match (Windows 10's acrylic takes the colour; Windows 11's transient
/// backdrop follows the window's dark/light theme).
#[tauri::command]
fn glass_tint(dark: bool, app: AppHandle) {
    for label in ["popover", "search"] {
        if let Some(w) = app.get_webview_window(label) {
            glass::acrylic(&w, dark);
        }
    }
}

/// Size Search to its panel (`height` DIP; 640 DIP wide), keeping its top
/// edge where it opened, and shape the window to the 14 DIP corners.
#[tauri::command]
fn search_fit(height: f64, app: AppHandle) {
    let Some(w) = app.get_webview_window("search") else { return };
    let sf = w.scale_factor().unwrap_or(1.0);
    let (pw, ph) = ((SEARCH_W * sf).round() as i32, (height.clamp(40.0, 900.0) * sf).ceil() as i32);
    if let Ok(pos) = w.outer_position() {
        let hwnd = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
        let _ = w.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32));
        let _ = w.set_position(pos);
        glass::round_region(hwnd, &[(0, 0, pw, ph, (14.0 * sf).round() as i32)]);
    }
}
const SEARCH_W: f64 = 640.0;

/// Real system icons for menus, Search and Control Centre, by name.
#[tauri::command]
fn icons_system() -> std::collections::HashMap<String, Option<String>> {
    let mut out = std::collections::HashMap::new();
    let mut jobs = Vec::new();
    for (name, keys) in icons::system_keys() {
        out.insert(name.to_string(), dock::icon_from(&keys, &mut jobs));
    }
    if !jobs.is_empty() {
        std::thread::spawn(move || icons::fill(jobs));
    }
    out
}

#[derive(Serialize)]
struct Recent {
    name: String,
    target: String,
    icon: Option<String>,
}

/// The Cove menu's Recent Items: recent files Windows lists.
#[tauri::command]
async fn recent_items() -> Vec<Recent> {
    search::index()
        .into_iter()
        .filter(|i| i.kind == "file" && i.subtitle == "Recent")
        .take(10)
        .map(|i| Recent { name: i.name, target: i.target, icon: i.icon })
        .collect()
}

#[tauri::command]
fn bin_empty(app: AppHandle) -> Result<(), String> {
    let r = dock::bin_empty();
    dock_changed(&app);
    r
}

/// The Go and File menus: a fixed list of folders, opened in File Explorer.
#[tauri::command]
fn topbar_go(what: String) -> Result<(), String> {
    let home = std::env::var("USERPROFILE").unwrap_or_default();
    let target = match what.as_str() {
        "explorer" => "shell:::{679f85cb-0220-4080-b29b-5540cc05aab6}".to_string(), // Home / Quick access
        "home" => home,
        "desktop" => "shell:Desktop".into(),
        "documents" => "shell:Personal".into(),
        "downloads" => "shell:Downloads".into(),
        "pictures" => "shell:My Pictures".into(),
        "thispc" => "shell:MyComputerFolder".into(),
        "recycler" => "shell:RecycleBinFolder".into(),
        "controlpanel" => "shell:ControlPanelFolder".into(),
        _ => return Err("Unknown folder.".into()),
    };
    search::explorer(&target).map_err(|e| e.to_string())
}

/// The desktop's View and File menu actions.
#[tauri::command]
fn topbar_view(what: String) -> Result<(), String> {
    match what.as_str() {
        "show-desktop" => glass::keys(&[0x5B, 0x44]), // Win+D
        "task-view" => glass::keys(&[0x5B, 0x09]),    // Win+Tab
        "toggle-icons" => glass::toggle_desktop_icons(),
        "new-folder" => {
            let desk = std::path::PathBuf::from(std::env::var("USERPROFILE").map_err(|e| e.to_string())?).join("Desktop");
            let mut p = desk.join("New folder");
            let mut n = 2;
            while p.exists() {
                p = desk.join(format!("New folder ({n})"));
                n += 1;
            }
            std::fs::create_dir(&p).map_err(|e| e.to_string())
        }
        _ => Err("Unknown item.".into()),
    }
}

/// Now Playing in Control Centre: play/pause, next, previous.
#[tauri::command]
async fn media_control(what: String) -> Result<(), String> {
    controls::media_control(&what)
}

#[tauri::command]
fn popover_hide(app: AppHandle) {
    popover_close(&app);
}

// ---- Control Centre (async: runs off the UI thread) ----

#[tauri::command]
async fn controls_get() -> controls::Controls {
    controls::get()
}

#[tauri::command]
async fn controls_set(what: String, value: u8) -> Result<(), String> {
    match what.as_str() {
        "volume" => controls::set_volume(value),
        "brightness" => controls::set_brightness(value),
        "mute" => {
            controls::toggle_mute();
            Ok(())
        }
        "dark" => controls::set_dark(value != 0),
        _ => Err("Unknown control.".into()),
    }
}

/// Top bar buttons. A fixed list: nothing else can be launched from here.
#[tauri::command]
fn topbar_open(what: String, app: AppHandle) -> Result<(), String> {
    let uri = match what.as_str() {
        "cove" => {
            show_main(&app);
            return Ok(());
        }
        "search" => {
            toggle_search(&app);
            return Ok(());
        }
        "lock" => return search::action("lock"),
        "settings" => "ms-settings:",
        "control" => "ms-actioncenter:controlcenter/&showFooter=true",
        "notifications" => "ms-actioncenter:",
        "battery" => "ms-settings:batterysaver",
        "network" => "ms-settings:network",
        "wifi" => "ms-availablenetworks:",
        "bluetooth" => "ms-settings:bluetooth",
        "nightlight" => "ms-settings:nightlight",
        "focus" => "ms-settings:quiethours",
        "sound" => "ms-settings:sound",
        "display" => "ms-settings:display",
        "taskmgr" => "taskmgr.exe",
        "about-pc" => "ms-settings:about",
        _ => return Err("Unknown item.".into()),
    };
    search::explorer(uri).map_err(|e| e.to_string())
}

/// The Cove menu (the mark at the left of the menu bar), like the Apple menu.
/// Restart and Shut down never act without a confirmation: Windows shows its
/// own, or the bar asks first and calls `topbar_power_now`.
#[tauri::command]
fn topbar_menu(what: String, app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    match what.as_str() {
        "about" => {
            show_main(&app);
            let _ = app.emit("nav", "updates");
        }
        "settings" => show_main(&app),
        "wallpaper" => {
            show_main(&app);
            let _ = app.emit("nav", "desktop");
        }
        "desktop-off" => {
            dock_set(false, app.clone(), state.clone());
            topbar_set(false, app.clone(), state);
            let _ = app.emit("settings-changed", ());
        }
        "about-pc" => return search::explorer("ms-settings:about").map_err(|e| e.to_string()),
        "lock" => return search::action("lock"),
        "sleep" | "restart" | "shutdown" => return topbar::power(&what),
        // Signing out always asks first (the popover's confirmation).
        "signout" => return Err("confirm".into()),
        "quit" => app.exit(0),
        _ => return Err("Unknown item.".into()),
    }
    Ok(())
}

/// Restart / Shut down after the menu bar's own confirmation (only used
/// when Windows' confirmation dialog isn't available). Apps can still
/// object: no force flag, so unsaved work stops it.
#[tauri::command]
fn topbar_power_now(what: String) -> Result<(), String> {
    let flag = match what.as_str() {
        "restart" => "/r",
        "shutdown" => "/s",
        "signout" => "/l",
        _ => return Err("Unknown item.".into()),
    };
    let mut cmd = std::process::Command::new("shutdown.exe");
    if flag == "/l" {
        cmd.arg(flag);
    } else {
        cmd.args([flag, "/t", "0"]);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// The app menu (click the app's name): every window of the app in front.
fn app_windows(hwnd: isize) -> Vec<isize> {
    let wins = dock::windows();
    match wins.iter().find(|w| w.hwnd == hwnd) {
        Some(w) => {
            let key = w.app_key();
            wins.iter().filter(|o| o.app_key() == key).map(|o| o.hwnd).collect()
        }
        None if hwnd != 0 => vec![hwnd],
        None => Vec::new(),
    }
}

#[tauri::command]
fn topbar_app(what: String, hwnd: isize, name: String, app: AppHandle) -> Result<(), String> {
    let wins = app_windows(hwnd);
    match what.as_str() {
        // App Exposé: just this app's windows.
        "expose" if !wins.is_empty() => open_expose(&app, name, wins),
        "hide" => wins.iter().for_each(|&h| dock::minimize(h)),
        // Every other app's windows go to the dock; this app stays.
        "hide-others" => {
            let keep: Vec<isize> = if wins.is_empty() { vec![hwnd] } else { wins.clone() };
            for w in dock::windows() {
                if !keep.contains(&w.hwnd) {
                    dock::minimize(w.hwnd);
                }
            }
            // Cove's own window too, unless it's the app in front.
            if let Some(w) = app.get_webview_window("main") {
                let mine = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
                if !keep.contains(&mine) && w.is_visible().unwrap_or(false) {
                    let _ = w.minimize();
                }
            }
        }
        "minimize" if hwnd != 0 => dock::minimize(hwnd),
        "zoom" if hwnd != 0 => dock::zoom(hwnd),
        // Every window of the app comes forward, the front one last.
        "front" => {
            for &h in wins.iter().rev() {
                dock::activate(h);
            }
            if hwnd != 0 {
                dock::activate(hwnd);
            }
        }
        "tile-left" | "tile-right" if hwnd != 0 => dock::tile(hwnd, what == "tile-left"),
        "minimize" | "zoom" | "tile-left" | "tile-right" => {}
        // Ask politely (WM_CLOSE), so the app can offer to save.
        "quit" => dock::close_windows(&wins),
        "expose" => {}
        _ => return Err("Unknown item.".into()),
    }
    Ok(())
}

// ---- Cove Dock ----

#[derive(Serialize)]
struct DockStatus {
    enabled: bool,
    hide_taskbar: bool,
    stacks: Vec<String>,
    display: String,
    /// "smart", "always" or "never" (settings::DOCK_HIDE).
    hide: String,
}

/// Make the taskbar's auto-hide match the settings, remembering (and later
/// restoring) what it was before Cove touched it.
fn sync_taskbar(s: &mut settings::Store) {
    // Cove desktop: the dock replaces the taskbar, so it's hidden outright
    // (and set to auto-hide, which frees the space it reserved).
    let want = s.data.dock && (s.data.takeover || s.data.dock_hide_taskbar) && !safe();
    dock::TASKBAR_GONE.store(want, std::sync::atomic::Ordering::SeqCst);
    dock::taskbar_visible(!want);
    match (want, s.data.taskbar_prev_autohide) {
        (true, None) => {
            s.data.taskbar_prev_autohide = Some(dock::taskbar_autohide(Some(true)));
        }
        (false, Some(prev)) => {
            dock::taskbar_autohide(Some(prev));
            s.data.taskbar_prev_autohide = None;
        }
        _ => {}
    }
    let _ = s.save();
}

fn dock_apply(app: &AppHandle, state: &AppState) -> DockStatus {
    let (enabled, hide, stacks, display, hide_mode) = match state.settings.lock() {
        Ok(mut s) => {
            // First time on: the dock starts with exactly the taskbar's pins.
            // (Preview builds imported without AppIDs; v2 imports again once.)
            if s.data.dock && (!s.data.pins_imported || s.data.pins_v < 2) && !safe() {
                let pins = shellprops::taskbar_pins();
                if !pins.is_empty() {
                    s.data.dock_pins = pins;
                }
                s.data.pins_imported = true;
                s.data.pins_v = 2;
            }
            sync_taskbar(&mut s);
            (
                s.data.dock,
                s.data.takeover || s.data.dock_hide_taskbar,
                s.data.dock_stacks.clone(),
                s.data.dock_display.clone(),
                s.data.dock_hide.clone(),
            )
        }
        Err(_) => (false, false, Vec::new(), "main".into(), "smart".into()),
    };
    if let Some(w) = app.get_webview_window("dock") {
        if enabled && !safe() {
            let _ = w.show();
            // Park it; only "Never hide" reserves its strip (dock_layout).
            dock_layout(app, &w, None);
            let _ = w.emit("dock-changed", ());
        } else {
            let _ = w.hide();
            dock_release(app);
        }
    }
    DockStatus { enabled, hide_taskbar: hide, stacks, display, hide: hide_mode }
}

#[tauri::command]
fn dock_status(state: tauri::State<'_, AppState>) -> DockStatus {
    let s = state.settings.lock().unwrap();
    DockStatus {
        enabled: s.data.dock,
        hide_taskbar: s.data.takeover || s.data.dock_hide_taskbar,
        stacks: s.data.dock_stacks.clone(),
        display: s.data.dock_display.clone(),
        hide: s.data.dock_hide.clone(),
    }
}

#[tauri::command]
fn dock_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock = enabled;
        let _ = s.save();
    }
    let st = dock_apply(&app, &state);
    tray_sync(&app);
    st
}

#[tauri::command]
fn dock_hide_taskbar_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        // One switch: hide the Windows taskbar while the dock is on.
        s.data.dock_hide_taskbar = enabled;
        s.data.takeover = enabled;
        let _ = s.save();
    }
    dock_apply(&app, &state)
}

#[tauri::command]
fn dock_state(app: AppHandle, state: tauri::State<'_, AppState>) -> dock::DockState {
    let (pins, stacks) = state
        .settings
        .lock()
        .map(|s| (s.data.dock_pins.clone(), s.data.dock_stacks.clone()))
        .unwrap_or_default();
    let mut st = dock::state(&pins, &stacks);
    // Cove is in the dock while its window is open (minimised counts).
    if let Some(w) = app.get_webview_window("main") {
        if w.is_visible().unwrap_or(false) {
            if let Ok(h) = w.hwnd() {
                dock::insert_cove(&mut st.items, dock::cove_item(h.0 as isize));
            }
        }
    }
    st
}

/// Tell the dock to redraw now (Cove's window opened or closed).
fn dock_changed(app: &AppHandle) {
    if let Some(d) = app.get_webview_window("dock") {
        let _ = d.emit("dock-changed", ());
    }
}

// ---- App switcher (Alt+Tab, opt-in) ----

#[tauri::command]
fn switcher_status(state: tauri::State<'_, AppState>) -> bool {
    state.settings.lock().map(|s| s.data.switcher).unwrap_or(false)
}

#[tauri::command]
fn switcher_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> bool {
    if let Ok(mut s) = state.settings.lock() {
        s.data.switcher = enabled;
        let _ = s.save();
    }
    switcher::set_enabled(enabled && !safe());
    tray_sync(&app);
    enabled
}

// ---- Cove Glass theme ----

fn theme_now(state: &AppState) -> theme::ThemeState {
    state
        .settings
        .lock()
        .map(|s| theme::state(&s.data.theme, &s.data.appearance, &s.data.accent))
        .unwrap_or_else(|_| theme::state("tidepool", "system", "cove"))
}

/// Light/dark for the OS material behind every Cove window, then tell the pages.
fn theme_apply(app: &AppHandle, st: &theme::ThemeState) {
    let t = match st.mode.as_str() {
        "light" => Some(tauri::Theme::Light),
        "dark" => Some(tauri::Theme::Dark),
        _ => None,
    };
    for (_, w) in app.webview_windows() {
        let _ = w.set_theme(t);
    }
    let _ = app.emit("theme-changed", st.clone());
}

#[tauri::command]
fn theme_get(state: tauri::State<'_, AppState>) -> theme::ThemeState {
    theme_now(&state)
}

#[tauri::command]
fn theme_set(theme: String, mode: String, accent: String, app: AppHandle, state: tauri::State<'_, AppState>) -> theme::ThemeState {
    if let Ok(mut s) = state.settings.lock() {
        s.data.theme = theme::pick(&theme, theme::THEMES).to_string();
        s.data.appearance = theme::pick(&mode, theme::MODES).to_string();
        s.data.accent = theme::pick(&accent, theme::ACCENTS).to_string();
        let _ = s.save();
    }
    let st = theme_now(&state);
    theme_apply(&app, &st);
    let _ = wallpaper_sync(&app, &state);
    tray_sync(&app);
    st
}

// ---- Tray menu ----

/// The tray's check marks, kept in step with Settings.
struct TrayChecks {
    dock: CheckMenuItem<tauri::Wry>,
    topbar: CheckMenuItem<tauri::Wry>,
    hotkeys: CheckMenuItem<tauri::Wry>,
    switcher: CheckMenuItem<tauri::Wry>,
    snap: CheckMenuItem<tauri::Wry>,
    system: CheckMenuItem<tauri::Wry>,
    light: CheckMenuItem<tauri::Wry>,
    dark: CheckMenuItem<tauri::Wry>,
    safe: CheckMenuItem<tauri::Wry>,
}

fn tray_sync(app: &AppHandle) {
    let Some(t) = app.try_state::<TrayChecks>() else { return };
    let state = app.state::<AppState>();
    let Ok(g) = state.settings.lock() else { return };
    let d = &g.data;
    let _ = t.dock.set_checked(d.dock);
    let _ = t.topbar.set_checked(d.topbar);
    let _ = t.hotkeys.set_checked(d.hotkeys);
    let _ = t.switcher.set_checked(d.switcher);
    let _ = t.snap.set_checked(d.snap);
    let _ = t.system.set_checked(d.appearance == "system");
    let _ = t.light.set_checked(d.appearance == "light");
    let _ = t.dark.set_checked(d.appearance == "dark");
    let _ = t.safe.set_checked(safe());
}

fn tray_action(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let cur = match state.settings.lock() {
        Ok(s) => s.data.clone(),
        Err(_) => return,
    };
    match id {
        "open" => show_main(app),
        "search" => toggle_search(app),
        "t-dock" => { dock_set(!cur.dock, app.clone(), state); }
        "t-topbar" => { topbar_set(!cur.topbar, app.clone(), state); }
        "t-hotkeys" => { hotkeys_set(!cur.hotkeys, app.clone(), state); }
        "t-switcher" => { switcher_set(!cur.switcher, app.clone(), state); }
        "t-snap" => { snap_set(!cur.snap, cur.snap_animate, app.clone(), state); }
        "m-system" | "m-light" | "m-dark" => {
            theme_set(cur.theme.clone(), id[2..].to_string(), cur.accent.clone(), app.clone(), state);
        }
        "tour" => {
            show_main(app);
            let _ = app.emit("tour-open", ());
        }
        "updates" | "restore" => {
            show_main(app);
            let _ = app.emit("nav", id.to_string());
        }
        "safe" => {
            safe_set(!safe(), app.clone());
            if safe() {
                show_main(app);
            }
        }
        "quit" => app.exit(0),
        _ => {}
    }
    tray_sync(app);
    let _ = app.emit("settings-changed", ());
}

// ---- Cove Tour ----

static TOUR_APP: std::sync::OnceLock<AppHandle> = std::sync::OnceLock::new();

/// Tell the tour that a feature was just used ("search", "habits", "snap",
/// "switcher", "expose"). Costs nothing when the tour isn't open.
pub fn tour_signal(what: &str) {
    if let Some(a) = TOUR_APP.get() {
        let _ = a.emit("tour-step", what.to_string());
    }
}

#[tauri::command]
fn tour_status(state: tauri::State<'_, AppState>) -> bool {
    state.settings.lock().map(|s| s.data.tour_done).unwrap_or(true)
}

#[tauri::command]
fn tour_done_set(done: bool, state: tauri::State<'_, AppState>) -> bool {
    if let Ok(mut s) = state.settings.lock() {
        s.data.tour_done = done;
        let _ = s.save();
    }
    done
}

// ---- Wallpapers and the light/dark schedule ----

/// The wallpaper file last handed to Windows, so the minute tick doesn't re-set it.
static WALL_LAST: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());
/// The schedule's last decision; the schedule only acts when it changes, so a
/// manual Light/Dark pick holds until the next scheduled switch.
static SCHED_LAST: std::sync::Mutex<Option<bool>> = std::sync::Mutex::new(None);

fn effective_dark(s: &settings::Settings) -> bool {
    match s.appearance.as_str() {
        "dark" => true,
        "light" => false,
        _ => wallpaper::windows_dark(),
    }
}

fn wallpaper_status_of(s: &settings::Settings) -> wallpaper::WallpaperStatus {
    wallpaper::WallpaperStatus {
        scenes: wallpaper::SCENES.iter().map(|x| x.to_string()).collect(),
        current: s.wallpaper.clone(),
        has_prev: s.wallpaper_prev.is_some(),
        schedule: s.schedule.clone(),
        light_at: s.light_at.clone(),
        dark_at: s.dark_at.clone(),
        dark_now: effective_dark(s),
    }
}

/// Put the right light/dark version of the chosen scene on the desktop.
fn wallpaper_sync(app: &AppHandle, state: &AppState) -> Result<(), String> {
    let (scene, dark) = {
        let s = state.settings.lock().map_err(|e| e.to_string())?;
        (s.data.wallpaper.clone(), effective_dark(&s.data))
    };
    if scene.is_empty() {
        return Ok(());
    }
    // A gallery pick or your own picture: one fixed file in Cove's data folder.
    if let Some(file) = wallpaper_fixed_file(app, &scene) {
        let f = file.to_string_lossy().to_string();
        if !file.exists() || WALL_LAST.lock().map(|l| *l == f).unwrap_or(false) {
            return Ok(());
        }
        wallpaper::set(&f)?;
        if let Ok(mut l) = WALL_LAST.lock() {
            *l = f;
        }
        return Ok(());
    }
    let name = wallpaper::file_name(&scene, dark);
    let src = app
        .path()
        .resolve(format!("wallpapers/{}", name), tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    // A copy in Cove's data folder keeps the path stable across updates.
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?.join("wallpapers");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let dst = dir.join(&name);
    let dst_s = dst.to_string_lossy().to_string();
    if WALL_LAST.lock().map(|l| *l == dst_s).unwrap_or(false) && dst.exists() {
        return Ok(());
    }
    if !dst.exists() {
        std::fs::copy(&src, &dst).map_err(|e| format!("Couldn't copy the wallpaper: {}", e))?;
    }
    wallpaper::set(&dst_s)?;
    if let Ok(mut l) = WALL_LAST.lock() {
        *l = dst_s;
    }
    Ok(())
}

fn gallery_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app.path().app_data_dir().map_err(|e| e.to_string())?.join("wallpapers").join("gallery"))
}

/// "gallery:<id>" and "own:<file>" name one file; Cove scenes return None.
fn wallpaper_fixed_file(app: &AppHandle, choice: &str) -> Option<std::path::PathBuf> {
    let dir = gallery_dir(app).ok()?;
    if let Some(id) = choice.strip_prefix("gallery:") {
        return wallpaper::safe_id(id).then(|| wallpaper::cache_file(&dir, id));
    }
    let f = choice.strip_prefix("own:")?;
    (!f.contains(['/', '\\']) && f.starts_with("own-")).then(|| dir.join(f))
}

/// Remember the wallpaper from before Cove, then record the new choice and show it.
fn wallpaper_choose(app: &AppHandle, choice: String) -> Result<wallpaper::WallpaperStatus, String> {
    let state = app.state::<AppState>();
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        if s.data.wallpaper_prev.is_none() {
            s.data.wallpaper_prev = Some(wallpaper::current().unwrap_or_default());
        }
        s.data.wallpaper = choice;
        let _ = s.save();
    }
    wallpaper_sync(app, &state)?;
    let _ = app.emit("wallpaper-changed", ());
    let s = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(wallpaper_status_of(&s.data))
}

#[derive(Clone, serde::Serialize)]
struct WallProgress {
    id: String,
    received: u64,
    total: u64,
}

/// Pick a gallery wallpaper: Cove scenes apply at once; photos and art are
/// downloaded on first use (progress as "wallpaper-progress"), then cached.
#[tauri::command]
async fn wallpaper_pick(id: String, app: AppHandle) -> Result<wallpaper::WallpaperStatus, String> {
    let item = wallpaper::item(&id).ok_or("Unknown wallpaper")?;
    let ticket = wallpaper::next_pick();
    if !item.bundled.is_empty() {
        let scene = item.bundled.clone();
        return wallpaper_choose(&app, scene);
    }
    let dir = gallery_dir(&app)?;
    let a = app.clone();
    let path = tauri::async_runtime::spawn_blocking(move || {
        wallpaper::fetch(item, &dir, ticket, |received, total| {
            let _ = a.emit("wallpaper-progress", WallProgress { id: item.id.clone(), received, total });
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    if !wallpaper::still_wanted(ticket) || !path.exists() {
        return Err(wallpaper::CANCELLED.into());
    }
    wallpaper_choose(&app, format!("gallery:{}", id))
}

/// "Use my own wallpaper": a JPEG, PNG or BMP the user chose, sent as base64.
#[tauri::command]
async fn wallpaper_own(data: String, app: AppHandle) -> Result<wallpaper::WallpaperStatus, String> {
    use base64::Engine;
    wallpaper::next_pick();
    let bytes = base64::engine::general_purpose::STANDARD.decode(data.as_bytes()).map_err(|_| "Couldn't read that file")?;
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("That picture is too large".into());
    }
    let ext = wallpaper::image_ext(&bytes).ok_or("Pick a JPEG, PNG or BMP picture")?;
    let dir = gallery_dir(&app)?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // A fresh name each time, so Windows doesn't keep showing a cached copy.
    if let Ok(rd) = std::fs::read_dir(&dir) {
        for e in rd.flatten() {
            if e.file_name().to_string_lossy().starts_with("own-") {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = format!("own-{}.{}", stamp, ext);
    std::fs::write(dir.join(&name), &bytes).map_err(|e| format!("Couldn't save the picture: {}", e))?;
    wallpaper_choose(&app, format!("own:{}", name))
}

#[tauri::command]
fn wallpaper_cached(app: AppHandle) -> Vec<String> {
    gallery_dir(&app).map(|d| wallpaper::cached(&d)).unwrap_or_default()
}

#[tauri::command]
fn wallpaper_open_source(id: String) -> Result<(), String> {
    wallpaper::open_source(&id)
}

fn wallpaper_put_back(state: &AppState) -> Result<bool, String> {
    let mut s = state.settings.lock().map_err(|e| e.to_string())?;
    s.data.wallpaper.clear();
    let prev = s.data.wallpaper_prev.take();
    let _ = s.save();
    drop(s);
    if let Ok(mut l) = WALL_LAST.lock() {
        l.clear();
    }
    match prev {
        Some(p) => wallpaper::set(&p).map(|_| true),
        None => Ok(false),
    }
}

/// Once a minute: follow the schedule, and keep the wallpaper's light/dark
/// version in step (also when Cove follows Windows' mode).
fn schedule_tick(app: &AppHandle) {
    if safe() {
        return;
    }
    let state = app.state::<AppState>();
    let decided = state.settings.lock().ok().and_then(|s| {
        if s.data.schedule != "times" {
            return None;
        }
        let l = wallpaper::parse_hm(&s.data.light_at)?;
        let d = wallpaper::parse_hm(&s.data.dark_at)?;
        Some(wallpaper::scheduled_dark(wallpaper::local_minutes(), l, d))
    });
    let mut changed = false;
    if let Some(dark) = decided {
        let fire = SCHED_LAST.lock().map(|mut last| {
            let f = *last != Some(dark);
            *last = Some(dark);
            f
        }).unwrap_or(false);
        if fire {
            if let Ok(mut s) = state.settings.lock() {
                let want = if dark { "dark" } else { "light" };
                if s.data.appearance != want {
                    s.data.appearance = want.to_string();
                    let _ = s.save();
                    changed = true;
                }
            }
        }
    }
    if changed {
        let st = theme_now(&state);
        theme_apply(app, &st);
        tray_sync(app);
        let _ = app.emit("wallpaper-changed", ());
    }
    let _ = wallpaper_sync(app, &state);
}

#[tauri::command]
fn wallpaper_status(state: tauri::State<'_, AppState>) -> Result<wallpaper::WallpaperStatus, String> {
    let s = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(wallpaper_status_of(&s.data))
}

#[tauri::command]
fn wallpaper_set(scene: String, app: AppHandle, state: tauri::State<'_, AppState>) -> Result<wallpaper::WallpaperStatus, String> {
    if !wallpaper::SCENES.contains(&scene.as_str()) {
        return Err("Unknown wallpaper".into());
    }
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        if s.data.wallpaper_prev.is_none() {
            s.data.wallpaper_prev = Some(wallpaper::current().unwrap_or_default());
        }
        s.data.wallpaper = scene;
        let _ = s.save();
    }
    wallpaper_sync(&app, &state)?;
    wallpaper_status(state)
}

#[tauri::command]
fn wallpaper_restore(state: tauri::State<'_, AppState>) -> Result<wallpaper::WallpaperStatus, String> {
    wallpaper_put_back(&state)?;
    wallpaper_status(state)
}

#[tauri::command]
fn schedule_set(
    schedule: String,
    light_at: String,
    dark_at: String,
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<wallpaper::WallpaperStatus, String> {
    if wallpaper::parse_hm(&light_at).is_none() || wallpaper::parse_hm(&dark_at).is_none() {
        return Err("Times look like 07:00".into());
    }
    {
        let mut s = state.settings.lock().map_err(|e| e.to_string())?;
        s.data.schedule = if schedule == "times" { "times".into() } else { "off".into() };
        s.data.light_at = light_at;
        s.data.dark_at = dark_at;
        let _ = s.save();
    }
    if let Ok(mut l) = SCHED_LAST.lock() {
        *l = None;
    }
    schedule_tick(&app);
    wallpaper_status(state)
}

// ---- Window snapping (Ctrl+Alt, opt-in) ----

#[derive(serde::Serialize)]
struct SnapInfo {
    enabled: bool,
    animate: bool,
    conflicts: Vec<&'static str>,
}

fn snap_info(state: &AppState) -> SnapInfo {
    let (enabled, animate) = state.settings.lock().map(|s| (s.data.snap, s.data.snap_animate)).unwrap_or((false, true));
    SnapInfo { enabled, animate, conflicts: snap::status().conflicts }
}

#[tauri::command]
fn snap_status(state: tauri::State<'_, AppState>) -> SnapInfo {
    snap_info(&state)
}

#[tauri::command]
fn snap_set(enabled: bool, animate: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> SnapInfo {
    if let Ok(mut s) = state.settings.lock() {
        s.data.snap = enabled;
        s.data.snap_animate = animate;
        let _ = s.save();
    }
    snap::configure(enabled && !safe(), animate);
    tray_sync(&app);
    snap_info(&state)
}

#[tauri::command]
fn switcher_pick(index: usize) {
    switcher::pick(index);
}

/// Centre the switcher on the screen under the pointer, sized to its icons.
fn place_switcher(app: &AppHandle, w: &tauri::WebviewWindow, n: usize) {
    let Some(m) = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
    else {
        return;
    };
    let sf = m.scale_factor();
    let (mp, ms) = (m.position(), m.size());
    let cols = n.clamp(1, 10) as f64;
    let (lw, lh) = (cols * 104.0 + 36.0, if n > 10 { 290.0 } else { 172.0 });
    let (pw, ph) = ((lw * sf).round() as i32, (lh * sf).round() as i32);
    let pw = pw.min(ms.width as i32 - 40);
    let ph = ph.min(ms.height as i32 - 40);
    let pos = PhysicalPosition::new(mp.x + (ms.width as i32 - pw) / 2, mp.y + (ms.height as i32 - ph) / 2);
    let _ = w.set_position(pos);
    let _ = w.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32));
    let _ = w.set_position(pos);
}

fn switcher_ui(app: &AppHandle, u: switcher::Ui) {
    let a = app.clone();
    match u {
        switcher::Ui::Open(st) => {
            let n = st.apps.len();
            let a2 = a.clone();
            let _ = a.run_on_main_thread(move || {
                if let Some(w) = a2.get_webview_window("switcher") {
                    let _ = w.emit("switch-state", &st);
                    place_switcher(&a2, &w, n);
                }
            });
            // A quick Alt+Tab tap just switches; show the panel only if Alt is held.
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(110));
                if switcher::is_open() {
                    let a3 = a.clone();
                    let _ = a.run_on_main_thread(move || {
                        if let Some(w) = a3.get_webview_window("switcher") {
                            let _ = w.show();
                        }
                    });
                }
            });
        }
        switcher::Ui::Update(st) => {
            let a2 = a.clone();
            let _ = a.run_on_main_thread(move || {
                if let Some(w) = a2.get_webview_window("switcher") {
                    let _ = w.emit("switch-state", &st);
                }
            });
        }
        switcher::Ui::Expose(sa) => {
            let a2 = a.clone();
            let _ = a.run_on_main_thread(move || open_expose(&a2, sa.name, sa.windows));
        }
        switcher::Ui::Hide => {
            let a2 = a.clone();
            let _ = a.run_on_main_thread(move || {
                if let Some(w) = a2.get_webview_window("switcher") {
                    let _ = w.hide();
                }
            });
        }
    }
}

// ---- App Exposé ----

static EXPOSE_PREV: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

/// Cover the screen under the pointer and lay out live previews of `windows`.
fn open_expose(app: &AppHandle, name: String, windows: Vec<isize>) {
    let Some(w) = app.get_webview_window("expose") else { return };
    let Some(m) = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| app.primary_monitor().ok().flatten())
    else {
        return;
    };
    let (mp, ms) = (*m.position(), *m.size());
    EXPOSE_PREV.store(dock::foreground(), std::sync::atomic::Ordering::SeqCst);
    let _ = w.set_position(mp);
    let _ = w.set_size(ms);
    let _ = w.set_position(mp);
    switcher::unlock_foreground();
    let _ = w.show();
    let _ = w.set_focus();
    let Ok(h) = w.hwnd() else { return };
    let mut st = expose::open(h.0 as isize, &name, &windows, ms.width as i32, ms.height as i32);
    st.scale = m.scale_factor();
    let _ = w.emit("expose-state", &st);
}

fn close_expose(app: &AppHandle) {
    expose::close();
    if let Some(w) = app.get_webview_window("expose") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn expose_open(name: String, windows: Vec<isize>, app: AppHandle) {
    tour_signal("expose");
    open_expose(&app, name, windows);
}

#[tauri::command]
fn expose_pick(hwnd: isize, app: AppHandle) {
    close_expose(&app);
    switcher::unlock_foreground();
    dock::activate(hwnd);
}

#[tauri::command]
fn expose_close(refocus: bool, app: AppHandle) {
    close_expose(&app);
    let prev = EXPOSE_PREV.swap(0, std::sync::atomic::Ordering::SeqCst);
    if refocus && prev != 0 {
        switcher::unlock_foreground();
        dock::activate(prev);
    }
}

// ---- Displays: keep the dock and top bar right when screens change ----

/// If a window's title bar is no longer on any display (a screen was
/// unplugged), bring it back to the centre of the main display.
fn keep_on_screen(app: &AppHandle, w: &tauri::WebviewWindow) {
    // A minimised window sits at -32000; leave it, Windows restores it.
    if w.is_minimized().unwrap_or(false) || w.is_maximized().unwrap_or(false) {
        return;
    }
    let (Ok(p), Ok(sz)) = (w.outer_position(), w.outer_size()) else { return };
    let mons = app.available_monitors().unwrap_or_default();
    // A strip of the title bar must be visible somewhere.
    let (tx, ty) = (p.x + (sz.width as i32) / 2, p.y + 12);
    let visible = mons.iter().any(|m| {
        let (mp, ms) = (m.position(), m.size());
        tx >= mp.x && tx < mp.x + ms.width as i32 && ty >= mp.y && ty < mp.y + ms.height as i32
    });
    if visible || mons.is_empty() {
        return;
    }
    if let Some(m) = app.primary_monitor().ok().flatten() {
        let (mp, ms) = (m.position(), m.size());
        let pw = (sz.width as i32).min(ms.width as i32 - 40);
        let ph = (sz.height as i32).min(ms.height as i32 - 80);
        let pos = PhysicalPosition::new(mp.x + (ms.width as i32 - pw) / 2, mp.y + (ms.height as i32 - ph) / 2);
        let _ = w.set_position(pos);
        let _ = w.set_size(tauri::PhysicalSize::new(pw.max(1) as u32, ph.max(1) as u32));
        let _ = w.set_position(pos);
    }
}

#[derive(Serialize, Clone, PartialEq)]
struct DisplayInfo {
    name: String,
    width: u32,
    height: u32,
    x: i32,
    y: i32,
    scale: u32,
    main: bool,
}

#[tauri::command]
fn displays(app: AppHandle) -> Vec<DisplayInfo> {
    let primary = app.primary_monitor().ok().flatten().map(|m| *m.position());
    app.available_monitors()
        .unwrap_or_default()
        .iter()
        .map(|m| DisplayInfo {
            name: m.name().cloned().unwrap_or_default(),
            width: m.size().width,
            height: m.size().height,
            x: m.position().x,
            y: m.position().y,
            scale: (m.scale_factor() * 100.0).round() as u32,
            main: Some(*m.position()) == primary,
        })
        .collect()
}

fn scale100(app: &AppHandle, m: &dock::Mon) -> i32 {
    let (cx, cy) = m.centre();
    app.monitor_from_point(cx as f64, cy as f64)
        .ok()
        .flatten()
        .map(|mm| (mm.scale_factor() * 100.0).round() as i32)
        .unwrap_or(100)
}

/// Every 300 ms: follow the pointer to another display (if asked), and
/// re-place the dock / re-register the top bar when a display is plugged
/// in, removed, re-sized or re-scaled. Cheap: a few Win32 queries.
fn display_watch(app: AppHandle) {
    let mut edge = dock::EdgeWatch::new();
    let mut bar_key: Option<((i32, i32, i32, i32), i32)> = None;
    let mut tick: u32 = 0;
    let mut layout: Option<Vec<DisplayInfo>> = None;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        tick = tick.wrapping_add(1);

        // Any display added, removed, moved or re-scaled: tell Settings and
        // rescue the main window if its screen went away.
        if tick % 3 == 1 {
            let now = displays(app.clone());
            if layout.is_some() && layout.as_ref() != Some(&now) {
                let _ = app.emit("displays-changed", ());
                let a = app.clone();
                let _ = app.run_on_main_thread(move || {
                    if let Some(w) = a.get_webview_window("main") {
                        keep_on_screen(&a, &w);
                    }
                });
            }
            layout = Some(now);
        }
        let st = app.state::<AppState>();
        // Nothing of the bars to look after until the startup intro is over.
        if safe() || !DESKTOP_REVEALED.load(std::sync::atomic::Ordering::SeqCst) {
            continue;
        }
        // Keep the taskbar gone if Explorer restarted or something showed it.
        if dock::TASKBAR_GONE.load(std::sync::atomic::Ordering::SeqCst) && dock::taskbar_is_visible() {
            dock::taskbar_visible(false);
        }
        let (dock_on, follow, bar_on) = match st.settings.lock() {
            Ok(s) => (s.data.dock, s.data.dock_display == "follow", s.data.topbar),
            Err(_) => continue,
        };

        // Explorer restarted (it forgets every AppBar): the work area runs
        // under a bar again, so reserve the strips anew (checked every 3 s).
        if tick % 10 == 5 {
            let held = |label: &str| {
                app.get_webview_window(label)
                    .and_then(|w| w.hwnd().ok())
                    .map(|h| h.0 as isize)
                    .and_then(|h| appbar::held(h).map(|rc| (h, rc)))
            };
            if let (true, Some((h, rc)), Some(p)) = (bar_on, held("topbar"), dock::primary()) {
                if p.work.1 < rc.3 {
                    appbar::forget(h);
                    let a = app.clone();
                    let _ = app.run_on_main_thread(move || topbar_apply(&a, true));
                }
            }
            if let (true, Some((h, rc))) = (dock_on, held("dock")) {
                let (cx, cy) = ((rc.0 + rc.2) / 2, (rc.1 + rc.3) / 2);
                if dock::monitor_at(cx, cy).is_some_and(|m| m.work.3 > rc.1) {
                    appbar::forget(h);
                    if let Some(w) = app.get_webview_window("dock") {
                        let _ = w.emit("dock-reflow", ());
                    }
                }
            }
        }

        // The bars must stay exactly on their reserved strips: if anything
        // moved a bar window (a DPI change, a show, Windows keeping it out
        // of the work area), put it back on the ABM_SETPOS rectangle.
        if tick % 3 == 2 {
            let h_of = |label: &str| app.get_webview_window(label).and_then(|w| w.hwnd().ok()).map(|h| h.0 as isize);
            if bar_on {
                if let Some(h) = h_of("topbar") {
                    if let (Some(rc), Some(now)) = (appbar::held(h), appbar::window_rect(h)) {
                        if now != rc {
                            let _ = app.run_on_main_thread(move || appbar::place(h, rc));
                        }
                    }
                }
            }
            if dock_on {
                // Not reserving (the default): keep it on the work area's
                // bottom edge if the display or the work area changed.
                if let (Some(h), Some(strip)) = (h_of("dock"), DOCK_STRIP.lock().ok().and_then(|s| *s)) {
                    let (cx, cy) = ((strip.0 + strip.2) / 2, (strip.1 + strip.3) / 2);
                    if appbar::held(h).is_none() && dock::monitor_at(cx, cy).is_some_and(|m| m.work.3 != strip.3) {
                        let a = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Some(w) = a.get_webview_window("dock") {
                                dock_layout(&a, &w, None);
                            }
                        });
                    }
                }
                if let Some(h) = h_of("dock") {
                    if let (Some(rc), Some(now)) = (appbar::held(h), appbar::window_rect(h)) {
                        if now.3 != rc.3 {
                            let a = app.clone();
                            let _ = app.run_on_main_thread(move || {
                                if let Some(w) = a.get_webview_window("dock") {
                                    dock_layout(&a, &w, None);
                                }
                            });
                        }
                    }
                }
            }
        }

        // Top bar: main display geometry or scale changed (checked each second).
        if bar_on && tick % 3 == 0 {
            if let Some(p) = dock::primary() {
                let key = (p.monitor, scale100(&app, &p));
                if bar_key.is_some() && bar_key != Some(key) {
                    let a = app.clone();
                    let _ = app.run_on_main_thread(move || topbar_apply(&a, true));
                }
                bar_key = Some(key);
            }
        } else if !bar_on {
            bar_key = None;
        }

        if !dock_on {
            continue;
        }
        let pushed = if follow {
            let c = dock::cursor();
            c.and_then(|(x, y)| edge.feed(y, dock::monitor_at(x, y), 2))
        } else {
            None
        };
        // Keep the current display (refreshed) unless the pointer moved us.
        let target = pushed.or_else(|| {
            if follow {
                dock::dock_monitor().and_then(|m| {
                    let (cx, cy) = m.centre();
                    dock::monitor_at(cx, cy)
                })
            } else {
                None
            }
        });
        let Some(target) = target.or_else(dock::primary) else { continue };
        if (pushed.is_some() || tick % 3 == 0) && dock::set_dock_monitor(target, scale100(&app, &target)) {
            if let Some(w) = app.get_webview_window("dock") {
                let _ = w.emit("dock-reflow", ());
            }
        }
    }
}

// ---- Dock folder stacks ----

#[tauri::command]
fn dock_stacks_set(keys: Vec<String>, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock_stacks = stacks::clean(&keys);
        let _ = s.save();
    }
    dock_apply(&app, &state)
}

/// "Hide the dock": "smart" (default), "always" or "never".
#[tauri::command]
fn dock_hide_set(mode: String, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock_hide = settings::DOCK_HIDE.iter().find(|m| **m == mode).unwrap_or(&"smart").to_string();
        let _ = s.save();
    }
    // Never takes the strip back; the other two give it up at once.
    dock_apply(&app, &state)
}

#[tauri::command]
fn dock_display_set(display: String, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock_display = if display == "follow" { "follow".into() } else { "main".into() };
        let _ = s.save();
    }
    dock_apply(&app, &state)
}

#[tauri::command]
async fn stack_list(key: String) -> Vec<stacks::StackFile> {
    stacks::list(&key, 30)
}

#[tauri::command]
fn stack_open(path: String, reveal: bool) -> Result<(), String> {
    stacks::open(&path, reveal)
}

#[tauri::command]
fn stack_folder(key: String) -> Result<(), String> {
    stacks::open_folder(&key)
}

/// Click: open a pinned app, bring its window forward, or minimise it if
/// it's already in front (like clicking the Windows taskbar).
#[tauri::command]
fn dock_open(kind: String, target: String, windows: Vec<isize>, active: bool) -> Result<(), String> {
    if let Some(&first) = windows.first() {
        if active && windows.len() == 1 {
            dock::minimize(first);
        } else {
            dock::activate(first);
        }
        return Ok(());
    }
    match kind.as_str() {
        "pin" => dock::launch(&target),
        "trash" => search::action("recycle"),
        _ => Err("Nothing to open.".into()),
    }
}

/// Shift-click / menu "New window": launch even when it's running.
#[tauri::command]
fn dock_launch(target: String) -> Result<(), String> {
    dock::launch(&target)
}

/// What the full intro is keyed on: this exact build (version + commit),
/// so every new build plays it once even when the version number stays the
/// same (preview builds). Falls back to the version when no build id was
/// embedded.
fn intro_key(version: &str, build: &str) -> String {
    if build.is_empty() {
        version.to_string()
    } else {
        format!("{version}+{build}")
    }
}

// ---- Intro: its own full-screen Ink window ("intro") ----
//
// At startup the intro window is shown before the dock, the menu bar and
// Settings, sized to the main display's full bounds. It plays the full
// voxel build once per installed build (intro_key), the ~1 s logo
// otherwise. When it ends (or a backstop fires) the window hides and Cove's
// desktop appears; Settings opens only on a first run. A second launch
// never replays it; "Play the intro" in About does. Every step is written to
// cove.log in the app data folder, so a real launch can be checked.

struct IntroState {
    full: bool,
    /// This run is the startup one: reveal the desktop when it ends.
    startup: bool,
    done: bool,
    /// Bumped on every show, so a stale backstop can't end a replay.
    run: u32,
}

static INTRO: Mutex<IntroState> = Mutex::new(IntroState { full: false, startup: false, done: true, run: 0 });
/// Open Settings when the startup intro ends (first run).
static OPEN_SETTINGS_AFTER_INTRO: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Append one line to cove.log in the app data folder (kept under 256 KB).
fn cove_log(app: &AppHandle, msg: &str) {
    let Ok(dir) = app.path().app_data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let path = dir.join("cove.log");
    if std::fs::metadata(&path).map(|m| m.len() > 256 * 1024).unwrap_or(false) {
        let _ = std::fs::rename(&path, dir.join("cove.old.log"));
    }
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0);
    let line = format!("{} {:.3} {}\n", log_stamp(secs as u64), secs.fract(), msg);
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// "2026-11-04T17:21:05Z" from Unix seconds (UTC; no time zone database).
fn log_stamp(t: u64) -> String {
    let (days, rem) = (t / 86_400, t % 86_400);
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

/// Show the intro window over the whole main display and let it play.
fn intro_show(app: &AppHandle, full: bool, startup: bool) {
    let run = {
        let Ok(mut st) = INTRO.lock() else { return };
        st.full = full;
        st.startup = startup;
        st.done = false;
        st.run = st.run.wrapping_add(1);
        st.run
    };
    let Some(w) = app.get_webview_window("intro") else {
        cove_log(app, "intro: no intro window; skipping");
        intro_finish(app, "no-window", run);
        return;
    };
    // The main display's full bounds (over where the bar and dock will be).
    let (x, y, pw, ph) = match app.primary_monitor().ok().flatten() {
        Some(m) => (m.position().x, m.position().y, m.size().width, m.size().height),
        None => dock::primary().map(|m| (m.monitor.0, m.monitor.1, (m.monitor.2 - m.monitor.0) as u32, (m.monitor.3 - m.monitor.1) as u32)).unwrap_or((0, 0, 1920, 1080)),
    };
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.set_size(tauri::PhysicalSize::new(pw, ph));
    if !startup {
        // A replay: start the page over.
        let _ = w.eval("location.reload()");
    }
    let _ = w.set_always_on_top(true);
    let shown = w.show().is_ok();
    let _ = w.set_position(PhysicalPosition::new(x, y));
    let _ = w.set_focus();
    cove_log(
        app,
        &format!("intro: window shown={shown} ({}, {pw}x{ph} at {x},{y}, {})", if full { "full" } else { "short" }, if startup { "startup" } else { "replay" }),
    );
    // Backstop: whatever the page does, the intro ends.
    let h = app.clone();
    let wait = if full { 9_000 } else { 4_000 };
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(wait));
        let h2 = h.clone();
        let _ = h.run_on_main_thread(move || intro_finish(&h2, "timeout", run));
    });
}

/// End the intro (once per run): hide its window, then on startup reveal
/// the dock and the menu bar and open Settings on a first run.
fn intro_finish(app: &AppHandle, why: &str, run: u32) {
    let startup = {
        let Ok(mut st) = INTRO.lock() else { return };
        if st.done || st.run != run {
            return;
        }
        st.done = true;
        st.startup
    };
    if let Some(w) = app.get_webview_window("intro") {
        let _ = w.hide();
    }
    cove_log(app, &format!("intro: finished ({why})"));
    if startup {
        reveal_desktop(app);
    }
}

/// The startup intro is over and the bars are up (display_watch waits).
static DESKTOP_REVEALED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// After the startup intro: the menu bar, the dock and (first run) Settings.
fn reveal_desktop(app: &AppHandle) {
    DESKTOP_REVEALED.store(true, std::sync::atomic::Ordering::SeqCst);
    let state = app.state::<AppState>();
    let (bar_on, dock_on) = state.settings.lock().map(|s| (s.data.topbar, s.data.dock)).unwrap_or((false, false));
    if bar_on {
        topbar_apply(app, true);
    }
    if dock_on {
        dock_apply(app, &state);
    }
    let settings = OPEN_SETTINGS_AFTER_INTRO.swap(false, std::sync::atomic::Ordering::SeqCst);
    if settings {
        show_main(app);
    }
    cove_log(app, &format!("startup: desktop revealed (menu bar {bar_on}, dock {dock_on}, settings {settings})"));
}

/// The intro page has loaded: which intro to play. The page can load
/// before startup has decided, so wait (briefly) for intro_show.
#[tauri::command]
async fn intro_begin(app: AppHandle) -> bool {
    for _ in 0..250 {
        if INTRO.lock().map(|s| s.run != 0).unwrap_or(true) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let full = INTRO.lock().map(|s| s.full).unwrap_or(false);
    cove_log(&app, &format!("intro: page playing ({})", if full { "full" } else { "short" }));
    full
}

/// The intro page has finished (or was skipped).
#[tauri::command]
fn intro_done(skipped: bool, app: AppHandle) {
    let run = INTRO.lock().map(|s| s.run).unwrap_or(0);
    intro_finish(&app, if skipped { "skipped" } else { "played" }, run);
}

/// "Play the intro" in About: the full intro again, over everything.
#[tauri::command]
fn intro_play(app: AppHandle) {
    intro_show(&app, true, false);
}

#[tauri::command]
fn dock_close(windows: Vec<isize>) {
    dock::close_windows(&windows);
}

#[tauri::command]
fn dock_pin(name: String, target: String, app: AppHandle, state: tauri::State<'_, AppState>) -> Result<(), String> {
    if target.is_empty() || target.contains(['"', '\n']) {
        return Err("Can't pin that.".into());
    }
    if let Ok(mut s) = state.settings.lock() {
        if !s.data.dock_pins.iter().any(|p| p.target == target) {
            s.data.dock_pins.push(settings::Pin { name, target, exe: None, aumid: None });
            let _ = s.save();
        }
    }
    if let Some(w) = app.get_webview_window("dock") {
        let _ = w.emit("dock-changed", ());
    }
    Ok(())
}

#[tauri::command]
fn dock_unpin(target: String, state: tauri::State<'_, AppState>) {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock_pins.retain(|p| p.target != target);
        let _ = s.save();
    }
}

/// The dock's own strip at the bottom of its display, in DIP: the dock
/// (48 + 16 padding + 2 border) and its 8 DIP of air. Only "Never hide"
/// reserves it (maximised windows then stop above it); by default the dock
/// takes no space, like a Mac's, and slides away when a window touches it.
/// Magnified icons and labels draw above the strip, over the apps, only
/// while the pointer is on the dock.
const DOCK_RESERVE: f64 = 72.0; // 66 DIP dock + 6 DIP float

fn dock_enabled(app: &AppHandle) -> bool {
    app.state::<AppState>().settings.lock().map(|s| s.data.dock).unwrap_or(false) && !safe()
}

/// "smart" | "always" | "never" (settings.dock_hide).
fn dock_hide_mode(app: &AppHandle) -> String {
    app.state::<AppState>().settings.lock().map(|s| s.data.dock_hide.clone()).unwrap_or_else(|_| "smart".into())
}

/// The dock's strip on screen (physical px) while the dock is shown,
/// reserved or not: what a window must touch to send it away, and where
/// the pointer arms it. Set by dock_layout.
static DOCK_STRIP: Mutex<Option<(i32, i32, i32, i32)>> = Mutex::new(None);

/// Park the dock at the bottom centre of its display and (while it's shown)
/// reserve its strip as an AppBar. `size` in DIP; None keeps the current size.
fn dock_layout(app: &AppHandle, w: &tauri::WebviewWindow, size: Option<(f64, f64)>) {
    let mon = dock::dock_monitor().or_else(dock::primary);
    let sf = mon
        .and_then(|m| {
            let (cx, cy) = m.centre();
            w.monitor_from_point(cx as f64, cy as f64).ok().flatten()
        })
        .map(|m| m.scale_factor())
        .unwrap_or_else(|| w.scale_factor().unwrap_or(1.0));
    let (pw, ph) = match size {
        Some((width, height)) => ((width * sf).round() as i32, (height * sf).round() as i32),
        None => match w.outer_size() {
            Ok(s) => (s.width as i32, s.height as i32),
            Err(_) => return,
        },
    };
    let hwnd = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let shown = dock_enabled(app) && w.is_visible().unwrap_or(false);
    let reserve = shown && dock_hide_mode(app) == "never";
    let (l, r, b) = match (reserve, mon) {
        (true, Some(m)) if hwnd != 0 => {
            let rc = appbar::register(hwnd, appbar::BOTTOM, m.monitor, (DOCK_RESERVE * sf).round() as i32);
            (rc.0, rc.2, rc.3)
        }
        _ => {
            if hwnd != 0 {
                appbar::unregister(hwnd);
            }
            let (l, _t, r, b) = mon.map(|m| m.work).or_else(dock::work_area).unwrap_or((0, 0, 1920, 1080));
            (l, r, b)
        }
    };
    let pos = PhysicalPosition::new(l + (r - l - pw) / 2, b - ph);
    if let Ok(mut st) = DOCK_STRIP.lock() {
        let strip = (DOCK_RESERVE * sf).round() as i32;
        // Only the slab itself (plus magnification room) is live.
        let pill = (DOCK_PILL.load(std::sync::atomic::Ordering::SeqCst) as f64 * sf).round() as i32;
        let inset = if pill > 0 && pill < pw { (pw - pill) / 2 } else { 0 };
        *st = shown.then_some((pos.x + inset, b - strip, pos.x + pw - inset, b));
    }
    // Where the window sits on its display, for the page's wallpaper plate.
    if let Some(m) = mon {
        let _ = w.emit(
            "dock-geom",
            serde_json::json!({
                "x": (pos.x - m.monitor.0) as f64 / sf,
                "y": (pos.y - m.monitor.1) as f64 / sf,
                "mw": (m.monitor.2 - m.monitor.0) as f64 / sf,
                "mh": (m.monitor.3 - m.monitor.1) as f64 / sf,
            }),
        );
    }
    // Move first, so a display with a different scale doesn't resize us
    // after we've set the size; then size, then settle the position.
    let _ = w.set_position(pos);
    let _ = w.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32));
    let _ = w.set_position(pos);
    // Bottom edge on the reserved strip's bottom (the monitor's edge), in
    // screen coordinates, synchronously: never derived from the work area.
    if hwnd != 0 && reserve {
        appbar::place(hwnd, (pos.x, pos.y, pos.x + pw, pos.y + ph));
    }
}

/// Give the dock's strip back (dock off, safe mode, quitting).
fn dock_release(app: &AppHandle) {
    if let Ok(mut st) = DOCK_STRIP.lock() {
        *st = None;
    }
    if let Some(w) = app.get_webview_window("dock") {
        if let Ok(h) = w.hwnd() {
            appbar::unregister(h.0 as isize);
        }
    }
}

/// A dock menu or folder stack is open above the dock (set by the dock page).
static DOCK_BUSY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

#[tauri::command]
fn dock_busy(busy: bool) {
    DOCK_BUSY.store(busy, std::sync::atomic::Ordering::SeqCst);
}

/// The dock window is taller than its strip (room for magnified icons,
/// labels, menus and stacks). That headroom must not catch clicks meant for
/// the app below it, so the window lets the pointer through until the
/// pointer reaches the dock itself, and keeps it while the pointer stays on
/// the dock window or a menu / stack is open.
///
/// It also hides the dock ("Hide the dock"): with "smart" (the default) the
/// dock slides down while a window touches its strip and comes back when
/// none does, or when the pointer touches the bottom edge under it (a 2 DIP
/// trigger); "always" waits for the edge; "never" keeps it up (and its strip
/// reserved). The page slides it (dock-slide); the window itself stays put
/// and lets every click through while hidden. Polls every 40 ms: cheap.
fn dock_hit_watch(app: AppHandle) {
    let mut through: Option<bool> = None;
    let mut on_dock = false;
    let mut hidden: Option<bool> = None;
    let mut revealed = false;
    let mut left_at: Option<std::time::Instant> = None;
    let mut covered = false;
    let mut tick: u32 = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_millis(40));
        tick = tick.wrapping_add(1);
        let Some(w) = app.get_webview_window("dock") else { continue };
        let Ok(h) = w.hwnd() else { continue };
        let strip = appbar::held(h.0 as isize).or_else(|| DOCK_STRIP.lock().ok().and_then(|s| *s));
        let Some(rc) = strip.filter(|_| w.is_visible().unwrap_or(false)) else {
            // Dock off: forget its state, so it starts shown next time.
            through = None;
            hidden = None;
            revealed = false;
            continue;
        };
        let (Ok(pos), Ok(size), Some((x, y))) = (w.outer_position(), w.outer_size(), dock::cursor()) else { continue };
        let inside = x >= pos.x && x < pos.x + size.width as i32 && y >= pos.y && y < pos.y + size.height as i32;
        let busy = DOCK_BUSY.load(std::sync::atomic::Ordering::SeqCst);
        let mode = dock_hide_mode(&app);

        // A window over the strip (checked every ~240 ms; EnumWindows is cheap
        // but not free).
        if mode == "smart" && tick % 6 == 0 {
            covered = dock::overlaps(rc);
        }
        // The bottom edge under the dock: the last 2 DIP of the screen.
        let sf = w.scale_factor().unwrap_or(1.0);
        let edge = (2.0 * sf).round().max(1.0) as i32;
        let at_edge = x >= rc.0 && x < rc.2 && y >= rc.3 - edge && y <= rc.3 + 1;
        let is_hidden = hidden.unwrap_or(false);
        if at_edge {
            revealed = true;
            left_at = None;
        } else if revealed && !inside && !busy {
            // Gone from the dock: put it away after a short grace.
            let t = *left_at.get_or_insert_with(std::time::Instant::now);
            if t.elapsed() >= std::time::Duration::from_millis(350) {
                revealed = false;
                left_at = None;
            }
        } else if inside {
            left_at = None;
        }
        let want_hidden = match mode.as_str() {
            "never" => false,
            "always" => true,
            _ => covered,
        } && !revealed
            && !busy
            && !(inside && !is_hidden);
        if hidden != Some(want_hidden) {
            let _ = w.emit("dock-slide", want_hidden);
            hidden = Some(want_hidden);
        }

        // Clicks: everything passes through while hidden; while shown,
        // reaching the strip arms the dock and leaving the window disarms it.
        // Only the slab arms it (not the transparent sides of the window).
        on_dock = !want_hidden && inside && (on_dock || (y >= rc.1 && x >= rc.0 && x < rc.2));
        let pass = want_hidden || !(on_dock || busy);
        if through != Some(pass) {
            let _ = w.set_ignore_cursor_events(pass);
            through = Some(pass);
        }
    }
}

/// Size the dock window (DIP) and park it on its display.
#[tauri::command]
fn dock_place(width: f64, height: f64, pill_width: Option<f64>, app: AppHandle) {
    let Some(w) = app.get_webview_window("dock") else { return };
    if let Some(p) = pill_width {
        DOCK_PILL.store(p.round().max(0.0) as u32, std::sync::atomic::Ordering::SeqCst);
    }
    dock_layout(&app, &w, Some((width, height)));
}

/// The slab's width (DIP): only this part of the dock window's bottom strip
/// catches the pointer; the transparent sides let clicks through.
static DOCK_PILL: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

#[tauri::command]
fn open_settings(uri: String) -> Result<(), String> {
    shell::open_settings(&uri).map_err(|e| e.to_string())
}

/// Save a switch and re-register shortcuts to match.
fn reconfigure(state: &AppState, f: impl FnOnce(&mut settings::Settings)) -> hotkeys::HotkeyStatus {
    let (habits, search) = match state.settings.lock() {
        Ok(mut s) => {
            f(&mut s.data);
            let _ = s.save();
            (s.data.hotkeys, s.data.search)
        }
        Err(_) => (false, true),
    };
    if safe() {
        return hotkeys::configure(false, false);
    }
    hotkeys::configure(habits, search)
}

#[tauri::command]
fn hotkeys_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> hotkeys::HotkeyStatus {
    let st = reconfigure(&state, |s| s.hotkeys = enabled);
    tray_sync(&app);
    st
}

#[tauri::command]
fn search_set(enabled: bool, state: tauri::State<'_, AppState>) -> hotkeys::HotkeyStatus {
    reconfigure(&state, |s| s.search = enabled)
}

// ---- Cove Search ----

/// Show the search panel on the monitor under the cursor, a fifth of the
/// way down (where the eye already is), or hide it if it's open.
fn toggle_search(app: &AppHandle) {
    let Some(w) = app.get_webview_window("search") else { return };
    if w.is_visible().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let monitor = app
        .cursor_position()
        .ok()
        .and_then(|p| app.monitor_from_point(p.x, p.y).ok().flatten())
        .or_else(|| w.current_monitor().ok().flatten());
    if let (Some(m), Ok(size)) = (monitor, w.outer_size()) {
        // Size in the target display's own scale, so a panel moved from a
        // 100% screen to a 150% one stays the same size and stays centred.
        let cur = w.scale_factor().unwrap_or(1.0).max(0.5);
        let sf = m.scale_factor();
        let (mp, ms) = (m.position(), m.size());
        let pw = ((size.width as f64 / cur * sf).round() as i32).min(ms.width as i32 - 16);
        let ph = ((size.height as f64 / cur * sf).round() as i32).min(ms.height as i32 - 16);
        let x = mp.x + (ms.width as i32 - pw) / 2;
        let y = mp.y + ((ms.height as f64 * 0.2) as i32).min(ms.height as i32 - ph);
        let pos = PhysicalPosition::new(x, y);
        let _ = w.set_position(pos);
        let _ = w.set_size(tauri::PhysicalSize::new(pw.max(1) as u32, ph.max(1) as u32));
        let _ = w.set_position(pos);
    }
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("search-open", ());
    search::warm();
}

#[tauri::command]
fn search_index() -> Vec<search::Item> {
    search::index()
}

#[tauri::command]
fn search_open(kind: String, target: String, app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
    search::open(&kind, &target)
}

#[tauri::command]
fn search_reveal(target: String, app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
    search::reveal(&target)
}

/// Async so the brief wait for the Windows Search index never blocks the UI thread.
#[tauri::command]
async fn search_files(query: String) -> Vec<search::Item> {
    search::files(&query)
}

/// Search window painted its first frame after Alt+Space.
#[tauri::command]
fn search_shown() -> Option<u32> {
    search::mark_shown()
}

#[derive(serde::Serialize)]
struct SearchInfo {
    winindex: bool,
    timing: search::Timing,
}

#[tauri::command]
fn search_info() -> SearchInfo {
    SearchInfo { winindex: winindex::enabled(), timing: search::timing() }
}

#[tauri::command]
fn search_winindex_set(enabled: bool, state: tauri::State<'_, AppState>) -> SearchInfo {
    if let Ok(mut s) = state.settings.lock() {
        s.data.search_winindex = enabled;
        let _ = s.save();
    }
    winindex::set_enabled(enabled);
    search_info()
}

#[tauri::command]
fn clip_history() -> Vec<String> {
    clipboard::history()
}

/// Enter pastes into the app you were in; Ctrl+Enter only copies.
#[tauri::command]
async fn clip_use(text: String, paste: bool, app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
    clipboard::put(&text, paste)
}

#[tauri::command]
fn clip_clear() {
    clipboard::clear();
}

#[tauri::command]
fn clipboard_set(enabled: bool, state: tauri::State<'_, AppState>) -> bool {
    if let Ok(mut s) = state.settings.lock() {
        s.data.clipboard = enabled;
        let _ = s.save();
    }
    clipboard::set_enabled(enabled && !safe());
    enabled
}

#[tauri::command]
fn clipboard_status(state: tauri::State<'_, AppState>) -> bool {
    state.settings.lock().map(|s| s.data.clipboard).unwrap_or(false)
}

#[tauri::command]
fn search_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
}

/// True once, on the first launch after a silent update.
#[tauri::command]
fn just_updated(state: tauri::State<'_, AppState>) -> bool {
    state.just_updated.swap(false, std::sync::atomic::Ordering::Relaxed)
}

#[tauri::command]
fn hotkeys_status() -> hotkeys::HotkeyStatus {
    hotkeys::status()
}

#[tauri::command]
async fn update_check() -> Result<updater::UpdateInfo, String> {
    tauri::async_runtime::spawn_blocking(updater::check)
        .await
        .map_err(|e| e.to_string())?
}

/// Download, verify, start the installer, then quit so it can replace Cove.
#[tauri::command]
async fn update_install(app: AppHandle) -> Result<(), String> {
    let a = app.clone();
    let path = tauri::async_runtime::spawn_blocking(move || {
        let mut last = 0u64;
        updater::download(|done, total| {
            // ~1% steps are plenty for a progress bar.
            if done == total || done - last > total / 100 {
                last = done;
                let _ = a.emit("update-progress", (done, total));
            }
        })
    })
    .await
    .map_err(|e| e.to_string())??;
    let _ = app.emit("update-installing", ());
    updater::launch(&path)?;
    hotkeys::stop();
    app.exit(0);
    Ok(())
}

fn main() {
    // Watchdog copy: wait for Cove to end, then put the taskbar back.
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 4 && args[1] == "--watchdog" {
        if let Ok(pid) = args[2].parse::<u32>() {
            dock::watchdog(pid, std::path::Path::new(&args[3]));
        }
        return;
    }
    // A second copy (double-clicking Cove again) hands its right to take the
    // foreground to the running copy, so Settings comes forward instead of
    // just flashing.
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::AllowSetForegroundWindow(u32::MAX); // ASFW_ANY
    }
    let app = tauri::Builder::default()
        // Opening Cove again just brings the running copy forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| second_launch(app)))
        .plugin(tauri_plugin_notification::init())
        // Full-resolution app icons, served from memory (see icons.rs).
        .register_uri_scheme_protocol("coveicon", |_ctx, req| {
            let res = tauri::http::Response::builder().header("Access-Control-Allow-Origin", "*");
            // "/wallpaper": the desktop picture itself, for Settings' miniature desktop.
            let path = req.uri().path();
            if path.starts_with("/wallpaper") {
                if let Some(bytes) = wallpaper::current().and_then(|p| std::fs::read(p).ok()) {
                    return res.header("Content-Type", "image/jpeg").header("Cache-Control", "no-cache").body(bytes).unwrap_or_default();
                }
            }
            match icons::serve(path) {
                Some(png) => res.header("Content-Type", "image/png").header("Cache-Control", "max-age=31536000, immutable").body(png),
                None => res.status(404).body(Vec::new()),
            }
            .unwrap_or_default()
        })
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let journal = journal::Journal::load(dir.join("journal.json"))?;
            let store = settings::Store::load(dir.join("settings.json"));
            // Bars a crashed run still held (the watchdog normally frees them).
            appbar::release_recorded(&dir.join("settings.json"));
            appbar::set_record(appbar::record_path(&dir.join("settings.json")));
            // If Cove ever ends without tidying up, this brings the taskbar back.
            #[cfg(windows)]
            if let Ok(me) = std::env::current_exe() {
                use std::os::windows::process::CommandExt;
                let _ = std::process::Command::new(me)
                    .arg("--watchdog")
                    .arg(std::process::id().to_string())
                    .arg(dir.join("settings.json"))
                    .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
                    .spawn();
            }
            let safe_why = safe_decide(&dir);
            if let Some(why) = safe_why {
                SAFE.store(true, std::sync::atomic::Ordering::SeqCst);
                if let Ok(mut r) = SAFE_REASON.lock() {
                    *r = why.into();
                }
            }
            {
                // 30 healthy seconds: this launch didn't crash.
                let h = app.handle().clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(30));
                    safe_marker_clear(&h);
                });
            }
            let _ = TOUR_APP.set(app.handle().clone());
            {
                let h = app.handle().clone();
                icons::set_logger(move |m| cove_log(&h, m));
            }
            let handle = app.handle().clone();
            hotkeys::set_search_handler(move || {
                search::mark_open();
                tour_signal("search");
                let h = handle.clone();
                // Window work belongs on the main thread.
                let _ = handle.run_on_main_thread(move || toggle_search(&h));
            });
            let on = safe_why.is_none();
            hotkeys::configure(on && store.data.hotkeys, on && store.data.search);
            clipboard::set_enabled(on && store.data.clipboard);
            winindex::set_enabled(store.data.search_winindex);
            {
                let h = app.handle().clone();
                switcher::set_ui_handler(move |u| switcher_ui(&h, u));
            }
            switcher::set_enabled(on && store.data.switcher);
            snap::configure(on && store.data.snap, store.data.snap_animate);
            theme_apply(app.handle(), &theme::state(&store.data.theme, &store.data.appearance, &store.data.accent));
            {
                let h = app.handle().clone();
                std::thread::spawn(move || loop {
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    let h2 = h.clone();
                    let _ = h.run_on_main_thread(move || schedule_tick(&h2));
                    std::thread::sleep(std::time::Duration::from_secs(55));
                });
            }
            search::warm();
            let dock_on = store.data.dock;
            let bar_on = store.data.topbar;
            // Once per new version: make Explorer drop stale cached icons.
            let mut store = store;
            if store.data.last_version != env!("CARGO_PKG_VERSION") {
                topbar::refresh_icons();
                store.data.last_version = env!("CARGO_PKG_VERSION").to_string();
                let _ = store.save();
            }
            let updated = std::env::args().any(|a| a == updater::UPDATED_FLAG);
            app.manage(AppState {
                journal: Mutex::new(journal),
                settings: Mutex::new(store),
                just_updated: std::sync::atomic::AtomicBool::new(updated),
            });
            // No DWM acrylic on the menu bar: the bar window is never the
            // active window, and Windows draws a system backdrop on an
            // inactive window as its flat fallback colour, which is why the
            // bar read as plain grey. The page paints an opaque Ink (or paper)
            // bar over a blurred slice of the wallpaper instead.
            if let Some(p) = dock::primary() {
                dock::set_dock_monitor(p, 0);
            }
            // The intro comes first, on its own: the menu bar, the dock and
            // Settings appear only once it ends (intro_finish).
            {
                let h = app.handle().clone();
                let st = app.state::<AppState>();
                let (full, first_run) = match st.settings.lock() {
                    Ok(mut s) => {
                        let key = intro_key(env!("CARGO_PKG_VERSION"), env!("COVE_BUILD_ID"));
                        let full = s.data.intro_seen.as_deref() != Some(key.as_str());
                        if full {
                            s.data.intro_seen = Some(key);
                            let _ = s.save();
                        }
                        // First run (tour not done yet), or nothing of Cove's
                        // would be visible otherwise: open Settings after.
                        (full, !s.data.tour_done || (!s.data.dock && !s.data.topbar))
                    }
                    Err(_) => (false, true),
                };
                OPEN_SETTINGS_AFTER_INTRO.store(first_run, std::sync::atomic::Ordering::SeqCst);
                cove_log(
                    &h,
                    &format!(
                        "startup: Cove {} build {:?}: intro {} (menu bar {bar_on}, dock {dock_on}, first run {first_run})",
                        env!("CARGO_PKG_VERSION"),
                        env!("COVE_BUILD_ID"),
                        if full { "full (new build)" } else { "short" }
                    ),
                );
                intro_show(&h, full, true);
            }
            {
                let h = app.handle().clone();
                std::thread::spawn(move || display_watch(h));
                let h = app.handle().clone();
                std::thread::spawn(move || dock_hit_watch(h));
            }
            if updated {
                use tauri_plugin_notification::NotificationExt;
                let _ = app
                    .notification()
                    .builder()
                    .title("Cove updated")
                    .body(format!("You're now on Cove {}.", env!("CARGO_PKG_VERSION")))
                    .show();
            }

            // Tray: Cove keeps running (and the shortcuts keep working)
            // after the window is closed. Quit from here.
            // Left-click opens Cove; right-click shows the menu.
            let open = MenuItem::with_id(app, "open", "Open Cove", true, None::<&str>)?;
            let search_item = MenuItem::with_id(app, "search", "Search\tAlt+Space", true, None::<&str>)?;
            let checks = {
                let st = app.state::<AppState>();
                let d = st.settings.lock().map(|s| s.data.clone()).unwrap_or_default();
                TrayChecks {
                    dock: CheckMenuItem::with_id(app, "t-dock", "Dock", true, d.dock, None::<&str>)?,
                    topbar: CheckMenuItem::with_id(app, "t-topbar", "Top bar", true, d.topbar, None::<&str>)?,
                    hotkeys: CheckMenuItem::with_id(app, "t-hotkeys", "Mac shortcuts", true, d.hotkeys, None::<&str>)?,
                    switcher: CheckMenuItem::with_id(app, "t-switcher", "App switcher", true, d.switcher, None::<&str>)?,
                    snap: CheckMenuItem::with_id(app, "t-snap", "Window snapping", true, d.snap, None::<&str>)?,
                    system: CheckMenuItem::with_id(app, "m-system", "Follow Windows", true, d.appearance == "system", None::<&str>)?,
                    light: CheckMenuItem::with_id(app, "m-light", "Light", true, d.appearance == "light", None::<&str>)?,
                    dark: CheckMenuItem::with_id(app, "m-dark", "Dark", true, d.appearance == "dark", None::<&str>)?,
                    safe: CheckMenuItem::with_id(app, "safe", "Safe mode", true, safe(), None::<&str>)?,
                }
            };
            let appearance = Submenu::with_items(app, "Appearance", true, &[&checks.system, &checks.light, &checks.dark])?;
            let tour = MenuItem::with_id(app, "tour", "Take the tour", true, None::<&str>)?;
            let updates = MenuItem::with_id(app, "updates", "Check for updates", true, None::<&str>)?;
            let restore = MenuItem::with_id(app, "restore", "Restore…", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Cove", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open,
                    &search_item,
                    &PredefinedMenuItem::separator(app)?,
                    &checks.dock,
                    &checks.topbar,
                    &checks.hotkeys,
                    &checks.switcher,
                    &checks.snap,
                    &appearance,
                    &PredefinedMenuItem::separator(app)?,
                    &tour,
                    &updates,
                    &restore,
                    &checks.safe,
                    &PredefinedMenuItem::separator(app)?,
                    &quit,
                ],
            )?;
            app.manage(checks);
            let mut tray = TrayIconBuilder::with_id("cove")
                .tooltip(format!("Cove {}", env!("CARGO_PKG_VERSION")))
                .menu(&menu)
                .show_menu_on_left_click(false);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| tray_action(app, event.id.as_ref()))
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        show_main(tray.app_handle());
                    }
                })
                .build(app)?;

            // The main window starts hidden; it opens after the intro on a
            // first run (reveal_desktop), from the tray, the Cove menu or a
            // second launch (show_main).
            // Mica: Windows' own cheap backdrop for the main window.
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                // Solid Ink window, no title bar of Windows' own: keep the
                // system shadow and rounded corners.
                let _ = window.set_shadow(true);
            }
            // Menu bar popovers and Search: acrylic, shaped by a rounded
            // window region to the panels (11 DIP menus, 20 DIP Control
            // Centre, 14 DIP Search), so DWM's own 8 px rounding is off.
            #[cfg(target_os = "windows")]
            for label in ["popover", "search"] {
                if let Some(window) = app.get_webview_window(label) {
                    let _ = window.set_shadow(false);
                    glass::acrylic(&window, true);
                    glass::no_dwm_rounding(&window);
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the window hides it to the tray instead of quitting.
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
                if window.label() == "main" {
                    dock_changed(window.app_handle());
                }
            }
            // A menu bar popover closes as soon as you click anywhere else.
            WindowEvent::Focused(false) if window.label() == "popover" => {
                let was = POP.lock().ok().and_then(|mut p| {
                    let k = p.open.take();
                    if let Some(k) = &k {
                        p.blurred = Some((k.clone(), std::time::Instant::now()));
                    }
                    k
                });
                if was.is_some() {
                    let _ = window.hide();
                    popover_state_changed(window.app_handle());
                }
            }
            // Search disappears as soon as you click elsewhere.
            WindowEvent::Focused(false) if window.label() == "search" => {
                let _ = window.hide();
            }
            // Exposé closes if you switch away some other way.
            WindowEvent::Focused(false) if window.label() == "expose" => {
                expose::close();
                let _ = window.hide();
            }
            _ => {}
        })
        .invoke_handler(tauri::generate_handler![
            cove_status,
            calm_list,
            calm_apply,
            restore_all,
            open_settings,
            hotkeys_set,
            hotkeys_status,
            update_check,
            update_install,
            search_set,
            search_index,
            search_open,
            search_reveal,
            search_hide,
            search_files,
            search_shown,
            search_info,
            search_winindex_set,
            topbar_status,
            topbar_set,
            topbar_state,
            topbar_open,
            topbar_menu,
            topbar_power_now,
            topbar_app,
            popover_toggle,
            popover_place,
            popover_hide,
            popover_at,
            dock_menu_pick,
            glass_tint,
            search_fit,
            icons_system,
            recent_items,
            bin_empty,
            topbar_go,
            topbar_view,
            media_control,
            controls_get,
            controls_set,
            dock_status,
            dock_set,
            dock_hide_taskbar_set,
            dock_state,
            dock_stacks_set,
            dock_display_set,
            dock_hide_set,
            switcher_status,
            switcher_set,
            safe_status,
            safe_set,
            displays,
            switcher_pick,
            snap_status,
            snap_set,
            theme_get,
            theme_set,
            tour_status,
            tour_done_set,
            wallpaper_status,
            wallpaper_set,
            wallpaper_restore,
            wallpaper_pick,
            wallpaper_own,
            wallpaper_cached,
            wallpaper_open_source,
            schedule_set,
            expose_open,
            expose_pick,
            expose_close,
            stack_list,
            stack_open,
            stack_folder,
            dock_open,
            dock_launch,
            dock_close,
            dock_pin,
            dock_unpin,
            dock_place,
            dock_busy,
            clip_history,
            clip_use,
            clip_clear,
            clipboard_set,
            clipboard_status,
            just_updated,
            intro_begin,
            intro_done,
            intro_play
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cove");

    app.run(|handle, event| {
        // Hotkeys live only while Cove runs; the taskbar goes back to how
        // it was (Cove hides it again next launch if the dock is on).
        if let RunEvent::Exit = event {
            safe_marker_clear(handle);
            hotkeys::stop();
            switcher::set_enabled(false);
            snap::configure(false, true);
            // Give the menu bar's and the dock's space back.
            if let Some(w) = handle.get_webview_window("topbar") {
                if let Ok(h) = w.hwnd() {
                    topbar::unregister(h.0 as isize);
                }
            }
            dock_release(handle);
            dock::TASKBAR_GONE.store(false, std::sync::atomic::Ordering::SeqCst);
            dock::taskbar_visible(true);
            if let Some(state) = handle.try_state::<AppState>() {
                if let Ok(mut s) = state.settings.lock() {
                    if let Some(prev) = s.data.taskbar_prev_autohide.take() {
                        dock::taskbar_autohide(Some(prev));
                        let _ = s.save();
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn intro_is_keyed_per_build() {
        assert_eq!(super::intro_key("0.9.0", "046d323abcde"), "0.9.0+046d323abcde");
        assert_ne!(super::intro_key("0.9.0", "046d323"), super::intro_key("0.9.0", "8a1edcc"));
        assert_eq!(super::intro_key("0.9.0", ""), "0.9.0");
    }
}
