// Cove: Windows, made calm.
// Hide the console window in release builds on Windows.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod journal;
mod tweaks;

use std::sync::Mutex;

use serde::Serialize;
use tauri::Manager;

/// Shared app state. Every system change goes through the journal.
struct AppState {
    journal: Mutex<journal::Journal>,
}

#[derive(Serialize)]
struct Status {
    version: String,
    journal_entries: usize,
}

#[tauri::command]
fn cove_status(state: tauri::State<'_, AppState>) -> Status {
    let entries = state.journal.lock().map(|j| j.len()).unwrap_or(0);
    Status {
        version: env!("CARGO_PKG_VERSION").to_string(),
        journal_entries: entries,
    }
}

#[tauri::command]
fn cove_planned_tweaks() -> Vec<tweaks::TweakInfo> {
    tweaks::calm::planned()
}

fn main() {
    tauri::Builder::default()
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
        .invoke_handler(tauri::generate_handler![cove_status, cove_planned_tweaks])
        .run(tauri::generate_context!())
        .expect("error while running Cove");
}
