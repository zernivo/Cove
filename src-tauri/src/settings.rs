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
    /// The build whose first launch played the full intro ("0.9.0+046d323").
    #[serde(default)]
    pub intro_seen: Option<String>,
    /// Cove desktop: while the dock is on, the Windows taskbar is hidden
    /// completely and the dock + menu bar replace it.
    #[serde(default = "yes")]
    pub takeover: bool,
    /// The dock's pins were brought over from the Windows taskbar once.
    #[serde(default)]
    pub pins_imported: bool,
    /// Version of the taskbar import; 2 adds AppIDs (Chrome profiles, Store apps).
    #[serde(default)]
    pub pins_v: u8,
    /// Apps pinned to the dock, in order.
    pub dock_pins: Vec<Pin>,
    /// Which display the dock uses: "main", or "follow" (moves to the
    /// display where you rest the pointer on the bottom edge).
    pub dock_display: String,
    /// Folder stacks in the dock (keys from stacks::KINDS), in order.
    pub dock_stacks: Vec<String>,
    /// When the dock gets out of the way: "smart" (slides down while a
    /// window touches it, back up at the bottom edge), "always" (only at
    /// the bottom edge) or "never" (always shown, reserving its strip).
    #[serde(default = "smart")]
    pub dock_hide: String,
    /// The taskbar's auto-hide state before Cove changed it.
    pub taskbar_prev_autohide: Option<bool>,
    /// Alt+Tab shows Cove's app switcher (one icon per app).
    pub switcher: bool,
    /// The menu bar is on in some form (menu_bar != "off"). Kept in step
    /// with `menu_bar` by `set_menu_bar`; older code paths read it.
    pub topbar: bool,
    /// The menu bar (2026-11-05): "off" (default: no bar, nothing
    /// reserved), "edge" (hidden, slides in when the pointer touches the
    /// top edge, reserves nothing) or "always" (the 24 DIP reserved bar).
    /// Empty only in settings saved before this existed: `Store::load`
    /// migrates those to "off".
    #[serde(default)]
    pub menu_bar: String,
    /// Cove Glass theme: "tidepool" or "slate".
    #[serde(default = "tidepool")]
    pub theme: String,
    /// "system", "light" or "dark".
    #[serde(default = "system_mode")]
    pub appearance: String,
    /// "cove" (the theme's accent) or "windows" (follow the Windows accent).
    #[serde(default = "cove_accent")]
    pub accent: String,
    /// The first-run Cove Tour was finished or skipped.
    #[serde(default)]
    pub tour_done: bool,
    /// Cove wallpaper scene in use, or "" for the user's own.
    #[serde(default)]
    pub wallpaper: String,
    /// The wallpaper path from before Cove first changed it.
    #[serde(default)]
    pub wallpaper_prev: Option<String>,
    /// Light/dark schedule: "off" or "times".
    #[serde(default = "off")]
    pub schedule: String,
    #[serde(default = "light_at")]
    pub light_at: String,
    #[serde(default = "dark_at")]
    pub dark_at: String,
    /// Cove Search also asks the Windows Search index (read-only).
    #[serde(default = "yes")]
    pub search_winindex: bool,
    /// Ctrl+Alt window snapping presets.
    #[serde(default)]
    pub snap: bool,
    /// Snapped windows glide (~200 ms) instead of jumping.
    #[serde(default = "yes")]
    pub snap_animate: bool,
    /// Mac window buttons: close/minimise/zoom dots at the left of the
    /// focused window's title bar, shown when the pointer comes near.
    #[serde(default = "yes")]
    pub traffic: bool,
    /// Programs (lower-case file names) that never get the window buttons.
    #[serde(default)]
    pub traffic_skip: Vec<String>,
    /// Last version that ran (to refresh icons once after an update).
    pub last_version: String,
    /// Show other apps' tray icons (Control Centre's "Other apps") while
    /// the takeover hides the taskbar. Off = no stand-in tray at all.
    #[serde(default = "yes")]
    pub tray_icons: bool,
    /// Tray icons pinned to the dock's status island (stable keys, tray.rs).
    #[serde(default)]
    pub tray_pins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pin {
    pub name: String,
    /// Start-menu AppID (opened through shell:AppsFolder), or a path.
    pub target: String,
    /// The program a shortcut pin opens, so its windows group under it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exe: Option<String>,
    /// The AppUserModelID a shortcut pin carries (a Chrome profile, a PWA):
    /// windows with the same ID belong to this pin, as on the taskbar.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aumid: Option<String>,
}

fn off() -> String {
    "off".into()
}
fn light_at() -> String {
    "07:00".into()
}
fn dark_at() -> String {
    "19:00".into()
}

fn tidepool() -> String {
    "tidepool".into()
}
fn system_mode() -> String {
    "system".into()
}
fn cove_accent() -> String {
    "cove".into()
}

fn yes() -> bool {
    true
}

fn smart() -> String {
    "smart".into()
}

/// The menu bar modes Settings offers.
pub const MENU_BAR: &[&str] = &["off", "edge", "always"];

impl Settings {
    /// Set the menu bar mode (unknown values mean "off").
    pub fn set_menu_bar(&mut self, mode: &str) {
        let m = MENU_BAR.iter().find(|m| **m == mode).unwrap_or(&"off");
        self.menu_bar = m.to_string();
        self.topbar = *m != "off";
    }

    /// Settings from before the menu bar modes had a bar that reserved the
    /// top of the screen; everyone starts with no bar now (user decision
    /// 2026-11-05: Windows users aren't used to a bar across the top).
    fn migrate(&mut self) {
        let m = if self.menu_bar.is_empty() { "off".to_string() } else { self.menu_bar.clone() };
        self.set_menu_bar(&m);
    }
}

/// The dock hide modes Settings offers.
pub const DOCK_HIDE: &[&str] = &["smart", "always", "never"];

pub fn default_pins() -> Vec<Pin> {
    vec![
        Pin { name: "File Explorer".into(), target: "Microsoft.Windows.Explorer".into(), exe: None, aumid: None },
        Pin { name: "Microsoft Edge".into(), target: "MSEdge".into(), exe: None, aumid: None },
        Pin {
            name: "Settings".into(),
            target: "windows.immersivecontrolpanel_cw5n1h2txyewy!microsoft.windows.immersivecontrolpanel".into(),
            exe: None,
            aumid: None,
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
            intro_seen: None,
            takeover: true,
            pins_imported: false,
            pins_v: 0,
            dock_pins: default_pins(),
            dock_stacks: vec!["downloads".into()],
            dock_display: "main".into(),
            dock_hide: smart(),
            taskbar_prev_autohide: None,
            topbar: false,
            menu_bar: "off".into(),
            switcher: false,
            snap: false,
            search_winindex: true,
            theme: tidepool(),
            appearance: system_mode(),
            accent: cove_accent(),
            tour_done: false,
            wallpaper: String::new(),
            wallpaper_prev: None,
            schedule: off(),
            light_at: light_at(),
            dark_at: dark_at(),
            snap_animate: true,
            traffic: true,
            traffic_skip: Vec::new(),
            last_version: String::new(),
            tray_icons: true,
            tray_pins: Vec::new(),
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
            .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
            .unwrap_or_default();
        let mut data = data;
        data.migrate();
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

    #[test]
    fn old_menu_bar_migrates_to_off() {
        let path = std::env::temp_dir().join(format!("cove-settings-mb-{}.json", std::process::id()));
        std::fs::write(&path, r#"{"topbar": true, "dock": true}"#).unwrap();
        let s = Store::load(path.clone());
        assert_eq!(s.data.menu_bar, "off");
        assert!(!s.data.topbar);
        assert!(s.data.dock);
        std::fs::write(&path, r#"{"topbar": false, "menu_bar": "edge"}"#).unwrap();
        let s = Store::load(path.clone());
        assert_eq!(s.data.menu_bar, "edge");
        assert!(s.data.topbar);
        std::fs::write(&path, r#"{"menu_bar": "sideways"}"#).unwrap();
        assert_eq!(Store::load(path.clone()).data.menu_bar, "off");
        let _ = std::fs::remove_file(path);
        assert_eq!(Settings::default().menu_bar, "off");
    }
}
