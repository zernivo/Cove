// Cove: Windows, made calm.
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod hotkeys;
mod journal;
mod registry;
mod shell;
mod tweaks;

use std::sync::Mutex;

use serde::Serialize;
use tauri::{Manager, RunEvent};

use tweaks::{calm, TweakInfo};

/// Shared app state. Every system change goes through the journal.
struct AppState {
    journal: Mutex<journal::Journal>,
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

#[tauri::command]
fn hotkeys_set(enabled: bool) -> hotkeys::HotkeyStatus {
    if enabled {
        hotkeys::start()
    } else {
        hotkeys::stop()
    }
}

#[tauri::command]
fn hotkeys_status() -> hotkeys::HotkeyStatus {
    hotkeys::status()
}

fn main() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let journal = journal::Journal::load(dir.join("journal.json"))?;
            app.manage(AppState {
                journal: Mutex::new(journal),
            });

            // Mica: Windows' own cheap backdrop for the main window.
            #[cfg(target_os = "windows")]
            if let Some(window) = app.get_webview_window("main") {
                let _ = window_vibrancy::apply_mica(&window, None);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            cove_status,
            calm_list,
            calm_apply,
            restore_all,
            open_settings,
            hotkeys_set,
            hotkeys_status
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
