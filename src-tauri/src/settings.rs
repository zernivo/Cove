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
}

impl Default for Settings {
    fn default() -> Self {
        Settings { hotkeys: false, search: true }
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
