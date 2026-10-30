// Cove: Windows, made calm.
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

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
mod shell;
mod stacks;
mod topbar;
mod tweaks;
mod updater;

use std::sync::Mutex;

use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WindowEvent};

use tweaks::{calm, TweakInfo};

/// Shared app state. Every system change goes through the journal.
struct AppState {
    journal: Mutex<journal::Journal>,
    settings: Mutex<settings::Store>,
    just_updated: std::sync::atomic::AtomicBool,
}

fn show_main(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
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
    restored: usize,
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
    let mut report = RestoreReport { restored: 0, failed: Vec::new() };
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
            report.restored += 1;
        }
    }
    if let Some(w) = app.get_webview_window("dock") {
        let _ = w.hide();
    }
    let had_bar = state.settings.lock().map(|mut s| {
        let was = s.data.topbar;
        s.data.topbar = false;
        let _ = s.save();
        was
    }).unwrap_or(false);
    if had_bar {
        report.restored += 1;
    }
    topbar_apply(&app, false);
    Ok(report)
}

// ---- Cove top bar ----

const TOPBAR_HEIGHT: f64 = 30.0;

fn topbar_apply(app: &AppHandle, on: bool) {
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
        let (l, t, r, b) = topbar::register(h, (TOPBAR_HEIGHT * sf).round() as i32);
        let _ = w.set_size(tauri::PhysicalSize::new((r - l).max(1) as u32, (b - t).max(1) as u32));
        let _ = w.set_position(PhysicalPosition::new(l, t));
        let _ = w.show();
    } else {
        topbar::unregister(h);
        let _ = w.hide();
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
    enabled
}

#[tauri::command]
async fn topbar_state() -> topbar::TopState {
    topbar::state()
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
        _ => return Err("Unknown item.".into()),
    };
    search::explorer(uri).map_err(|e| e.to_string())
}

// ---- Cove Dock ----

#[derive(Serialize)]
struct DockStatus {
    enabled: bool,
    hide_taskbar: bool,
    stacks: Vec<String>,
    display: String,
}

/// Make the taskbar's auto-hide match the settings, remembering (and later
/// restoring) what it was before Cove touched it.
fn sync_taskbar(s: &mut settings::Store) {
    let want = s.data.dock && s.data.dock_hide_taskbar;
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
    let (enabled, hide, stacks, display) = match state.settings.lock() {
        Ok(mut s) => {
            sync_taskbar(&mut s);
            (s.data.dock, s.data.dock_hide_taskbar, s.data.dock_stacks.clone(), s.data.dock_display.clone())
        }
        Err(_) => (false, false, Vec::new(), "main".into()),
    };
    if let Some(w) = app.get_webview_window("dock") {
        if enabled {
            let _ = w.show();
            let _ = w.emit("dock-changed", ());
        } else {
            let _ = w.hide();
        }
    }
    DockStatus { enabled, hide_taskbar: hide, stacks, display }
}

#[tauri::command]
fn dock_status(state: tauri::State<'_, AppState>) -> DockStatus {
    let s = state.settings.lock().unwrap();
    DockStatus {
        enabled: s.data.dock,
        hide_taskbar: s.data.dock_hide_taskbar,
        stacks: s.data.dock_stacks.clone(),
        display: s.data.dock_display.clone(),
    }
}

#[tauri::command]
fn dock_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock = enabled;
        let _ = s.save();
    }
    dock_apply(&app, &state)
}

#[tauri::command]
fn dock_hide_taskbar_set(enabled: bool, app: AppHandle, state: tauri::State<'_, AppState>) -> DockStatus {
    if let Ok(mut s) = state.settings.lock() {
        s.data.dock_hide_taskbar = enabled;
        let _ = s.save();
    }
    dock_apply(&app, &state)
}

#[tauri::command]
fn dock_state(state: tauri::State<'_, AppState>) -> dock::DockState {
    let (pins, stacks) = state
        .settings
        .lock()
        .map(|s| (s.data.dock_pins.clone(), s.data.dock_stacks.clone()))
        .unwrap_or_default();
    dock::state(&pins, &stacks)
}

// ---- Displays: keep the dock and top bar right when screens change ----

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
    loop {
        std::thread::sleep(std::time::Duration::from_millis(300));
        tick = tick.wrapping_add(1);
        let st = app.state::<AppState>();
        let (dock_on, follow, bar_on) = match st.settings.lock() {
            Ok(s) => (s.data.dock, s.data.dock_display == "follow", s.data.topbar),
            Err(_) => continue,
        };

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
            s.data.dock_pins.push(settings::Pin { name, target });
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

/// Size the dock window and park it at the bottom centre of the main
/// screen's work area.
#[tauri::command]
fn dock_place(width: f64, height: f64, app: AppHandle) {
    let Some(w) = app.get_webview_window("dock") else { return };
    let mon = dock::dock_monitor().or_else(dock::primary);
    let (l, _t, r, b) = mon.map(|m| m.work).or_else(dock::work_area).unwrap_or((0, 0, 1920, 1080));
    let sf = mon
        .and_then(|m| {
            let (cx, cy) = m.centre();
            w.monitor_from_point(cx as f64, cy as f64).ok().flatten()
        })
        .map(|m| m.scale_factor())
        .unwrap_or_else(|| w.scale_factor().unwrap_or(1.0));
    let (pw, ph) = ((width * sf).round() as i32, (height * sf).round() as i32);
    let pos = PhysicalPosition::new(l + (r - l - pw) / 2, b - ph);
    // Move first, so a display with a different scale doesn't resize us
    // after we've set the size; then size, then settle the position.
    let _ = w.set_position(pos);
    let _ = w.set_size(tauri::PhysicalSize::new(pw as u32, ph as u32));
    let _ = w.set_position(pos);
}

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
    hotkeys::configure(habits, search)
}

#[tauri::command]
fn hotkeys_set(enabled: bool, state: tauri::State<'_, AppState>) -> hotkeys::HotkeyStatus {
    reconfigure(&state, |s| s.hotkeys = enabled)
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
        let (mp, ms) = (m.position(), m.size());
        let x = mp.x + (ms.width as i32 - size.width as i32) / 2;
        let y = mp.y + (ms.height as f64 * 0.2) as i32;
        let _ = w.set_position(PhysicalPosition::new(x, y));
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

#[tauri::command]
fn search_files(query: String) -> Vec<search::Item> {
    search::files(&query)
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
    clipboard::set_enabled(enabled);
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
    let app = tauri::Builder::default()
        // Opening Cove again just brings the running copy forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
        .plugin(tauri_plugin_notification::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let journal = journal::Journal::load(dir.join("journal.json"))?;
            let store = settings::Store::load(dir.join("settings.json"));
            let handle = app.handle().clone();
            hotkeys::set_search_handler(move || {
                let h = handle.clone();
                // Window work belongs on the main thread.
                let _ = handle.run_on_main_thread(move || toggle_search(&h));
            });
            hotkeys::configure(store.data.hotkeys, store.data.search);
            clipboard::set_enabled(store.data.clipboard);
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
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("topbar") {
                let _ = window_vibrancy::apply_acrylic(&window, None);
            }
            if bar_on {
                topbar_apply(app.handle(), true);
            }
            if let Some(p) = dock::primary() {
                dock::set_dock_monitor(p, 0);
            }
            if dock_on {
                let h = app.handle().clone();
                let st = app.state::<AppState>();
                dock_apply(&h, &st);
            }
            {
                let h = app.handle().clone();
                std::thread::spawn(move || display_watch(h));
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
            let open = MenuItem::with_id(app, "open", "Open Cove", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit Cove", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&open, &quit])?;
            let mut tray = TrayIconBuilder::with_id("cove").tooltip("Cove").menu(&menu);
            if let Some(icon) = app.default_window_icon() {
                tray = tray.icon(icon.clone());
            }
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "open" => show_main(app),
                "quit" => app.exit(0),
                _ => {}
            })
            .build(app)?;

            // Mica: Windows' own cheap backdrop for the main window.
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_mica(&window, None);
            }
            // Search panel: acrylic blur of the desktop behind it.
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("search") {
                let _ = window_vibrancy::apply_acrylic(&window, None);
                if let Ok(hwnd) = window.hwnd() {
                    use windows_sys::Win32::Graphics::Dwm::{
                        DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE,
                    };
                    let round: u32 = 2; // DWMWCP_ROUND
                    unsafe {
                        DwmSetWindowAttribute(
                            hwnd.0 as _,
                            DWMWA_WINDOW_CORNER_PREFERENCE as _,
                            &round as *const u32 as *const _,
                            4,
                        );
                    }
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| match event {
            // Closing the window hides it to the tray instead of quitting.
            WindowEvent::CloseRequested { api, .. } => {
                let _ = window.hide();
                api.prevent_close();
            }
            // Search disappears as soon as you click elsewhere.
            WindowEvent::Focused(false) if window.label() == "search" => {
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
            topbar_status,
            topbar_set,
            topbar_state,
            topbar_open,
            controls_get,
            controls_set,
            dock_status,
            dock_set,
            dock_hide_taskbar_set,
            dock_state,
            dock_stacks_set,
            dock_display_set,
            stack_list,
            stack_open,
            stack_folder,
            dock_open,
            dock_launch,
            dock_close,
            dock_pin,
            dock_unpin,
            dock_place,
            clip_history,
            clip_use,
            clip_clear,
            clipboard_set,
            clipboard_status,
            just_updated
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cove");

    app.run(|handle, event| {
        // Hotkeys live only while Cove runs; the taskbar goes back to how
        // it was (Cove hides it again next launch if the dock is on).
        if let RunEvent::Exit = event {
            hotkeys::stop();
            if let Some(w) = handle.get_webview_window("topbar") {
                if let Ok(h) = w.hwnd() {
                    topbar::unregister(h.0 as isize);
                }
            }
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
