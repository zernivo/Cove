//! Cove's own preferences (not Windows settings). Stored as JSON in the
//! app data folder so choices survive closing and reopening Cove.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Mac-style shortcuts on.
    pub hotkeys: bool,
    /// Alt+Space opens Cove Search.
    pub search: bool,
    /// Cove Search keeps a text clipboard history (memory only).
    pub clipboard: bool,
    /// Cove Dock on.
    pub dock: bool,
    /// Auto-hide the Windows taskbar while the dock is on.
    pub dock_hide_taskbar: bool,
    /// Apps pinned to the dock, in order.
    pub dock_pins: Vec<Pin>,
    /// Which display the dock uses: "main", or "follow" (moves to the
    /// display where you rest the pointer on the bottom edge).
    pub dock_display: String,
    /// Folder stacks in the dock (keys from stacks::KINDS), in order.
    pub dock_stacks: Vec<String>,
    /// The taskbar's auto-hide state before Cove changed it.
    pub taskbar_prev_autohide: Option<bool>,
    /// Alt+Tab shows Cove's app switcher (one icon per app).
    pub switcher: bool,
    /// Cove top bar on.
    pub topbar: bool,
    /// Cove Search also asks the Windows Search index (read-only).
    #[serde(default = "yes")]
    pub search_winindex: bool,
    /// Ctrl+Alt window snapping presets.
    #[serde(default)]
    pub snap: bool,
    /// Snapped windows glide (~200 ms) instead of jumping.
    #[serde(default = "yes")]
    pub snap_animate: bool,
    /// Last version that ran (to refresh icons once after an update).
    pub last_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub name: String,
    /// Start-menu AppID (opened through shell:AppsFolder).
    pub target: String,
}

fn yes() -> bool {
    true
}

pub fn default_pins() -> Vec<Pin> {
    vec![
        Pin { name: "File Explorer".into(), target: "Microsoft.Windows.Explorer".into() },
        Pin { name: "Microsoft Edge".into(), target: "MSEdge".into() },
        Pin {
            name: "Settings".into(),
            target: "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel".into(),
        },
    ]
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            hotkeys: false,
            search: true,
            clipboard: true,
            dock: false,
            dock_hide_taskbar: false,
            dock_pins: default_pins(),
            dock_stacks: vec!["downloads".into()],
            dock_display: "main".into(),
            taskbar_prev_autohide: None,
            topbar: false,
            switcher: false,
            snap: false,
            search_winindex: true,
            snap_animate: true,
            last_version: String::new(),
        }
    }
}

pub struct Store {
    path: PathBuf,
    pub data: Settings,
}

impl Store {
    pub fn load(path: PathBuf) -> Self {
        let data = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Store { path, data }
    }

    pub fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let json = serde_json::to_string_pretty(&self.data)?;
        std::fs::write(&self.path, json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembers_hotkeys_choice() {
        let path = std::env::temp_dir().join(format!("cove-settings-{}.json", std::process::id()));
        let mut s = Store::load(path.clone());
        assert!(!s.data.hotkeys);
        s.data.hotkeys = true;
        s.save().unwrap();
        assert!(Store::load(path.clone()).data.hotkeys);
        let _ = std::fs::remove_file(path);
    }
}
