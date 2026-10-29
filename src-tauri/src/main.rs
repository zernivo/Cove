// Cove: Windows, made calm.
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hotkeys;
mod journal;
mod registry;
mod search;
mod settings;
mod shell;
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
fn restore_all(state: tauri::State<'_, AppState>) -> Result<RestoreReport, String> {
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
    Ok(report)
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
fn search_hide(app: AppHandle) {
    if let Some(w) = app.get_webview_window("search") {
        let _ = w.hide();
    }
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
    let path = tauri::async_runtime::spawn_blocking(updater::download)
        .await
        .map_err(|e| e.to_string())??;
    updater::launch(&path)?;
    hotkeys::stop();
    app.exit(0);
    Ok(())
}

fn main() {
    let app = tauri::Builder::default()
        // Opening Cove again just brings the running copy forward.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| show_main(app)))
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
            search::warm();
            app.manage(AppState {
                journal: Mutex::new(journal),
                settings: Mutex::new(store),
            });

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
            search_hide
        ])
        .build(tauri::generate_context!())
        .expect("error while building Cove");

    app.run(|_handle, event| {
        // Hotkeys live only while Cove runs.
        if let RunEvent::Exit = event {
            hotkeys::stop();
        }
    });
}
