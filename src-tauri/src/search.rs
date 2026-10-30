//! Cove Search: a local index of apps, recent files, Settings pages and
//! a few actions. Ranking happens in the search window (instant, no IPC
//! per keystroke). Nothing leaves the machine.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    /// app | file | setting | action
    pub kind: &'static str,
    pub name: String,
    pub subtitle: String,
    /// AppID (app), path (file), ms-settings uri (setting), action id
    pub target: String,
    /// PNG data URL of the real icon, once rendered.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

fn icon_path(it: &Item) -> Option<String> {
    match it.kind {
        "app" => Some(format!("shell:AppsFolder\\{}", it.target)),
        "file" => Some(it.target.clone()),
        _ => None,
    }
}

static APPS: Mutex<Option<(Instant, Vec<Item>)>> = Mutex::new(None);
const APPS_TTL: Duration = Duration::from_secs(600);

pub static SETTINGS: &[(&str, &str, &str)] = &[
    ("Display", "ms-settings:display", "screen brightness resolution scale night light"),
    ("Sound", "ms-settings:sound", "volume audio speakers microphone"),
    ("Bluetooth & devices", "ms-settings:bluetooth", "bluetooth headphones mouse printer"),
    ("Wi-Fi", "ms-settings:network-wifi", "wifi wireless internet network"),
    ("Network & internet", "ms-settings:network", "ethernet vpn proxy hotspot"),
    ("Personalization", "ms-settings:personalization", "theme look"),
    ("Background", "ms-settings:personalization-background", "wallpaper desktop picture"),
    ("Colors", "ms-settings:personalization-colors", "accent dark mode light mode"),
    ("Taskbar", "ms-settings:taskbar", "taskbar widgets"),
    ("Start", "ms-settings:personalization-start", "start menu recommended"),
    ("Lock screen", "ms-settings:lockscreen", "lock screen"),
    ("Installed apps", "ms-settings:appsfeatures", "uninstall programs apps"),
    ("Default apps", "ms-settings:defaultapps", "browser default"),
    ("Startup apps", "ms-settings:startupapps", "startup boot login"),
    ("Notifications", "ms-settings:notifications", "alerts do not disturb"),
    ("Focus", "ms-settings:focus", "focus do not disturb"),
    ("Power & battery", "ms-settings:powersleep", "battery sleep power saver"),
    ("Storage", "ms-settings:storagesense", "disk space cleanup"),
    ("Windows Update", "ms-settings:windowsupdate", "update upgrade"),
    ("Privacy & security", "ms-settings:privacy", "privacy permissions"),
    ("Mouse", "ms-settings:mousetouchpad", "mouse pointer scroll"),
    ("Touchpad", "ms-settings:devices-touchpad", "touchpad gestures trackpad"),
    ("Keyboard", "ms-settings:keyboard", "keyboard typing"),
    ("Date & time", "ms-settings:dateandtime", "clock time zone"),
    ("Language & region", "ms-settings:regionlanguage", "language region"),
    ("Accounts", "ms-settings:yourinfo", "account profile user"),
    ("About this PC", "ms-settings:about", "about pc specs system"),
];

pub static ACTIONS: &[(&str, &str, &str)] = &[
    ("Lock screen", "lock", "lock computer"),
    ("Open Recycle Bin", "recycle", "trash bin deleted"),
    ("Open Downloads", "downloads", "downloads folder"),
    ("Open Documents", "documents", "documents folder"),
];

fn static_items() -> Vec<Item> {
    let mut v: Vec<Item> = SETTINGS
        .iter()
        .map(|(n, uri, kw)| Item { kind: "setting", name: n.to_string(), subtitle: kw.to_string(), target: uri.to_string(), icon: None })
        .collect();
    v.extend(ACTIONS.iter().map(|(n, id, kw)| Item {
        kind: "action",
        name: n.to_string(),
        subtitle: kw.to_string(),
        target: id.to_string(),
        icon: None,
    }));
    v
}

#[derive(Deserialize)]
struct StartApp {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "AppID")]
    app_id: String,
}

/// Every app Start shows (desktop and Store), via the documented
/// Get-StartApps cmdlet. Run in the background; cached for 10 minutes.
#[cfg(windows)]
fn load_apps() -> Vec<Item> {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let out = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-StartApps | ConvertTo-Json -Compress",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();
    let Ok(out) = out else { return Vec::new() };
    let text = String::from_utf8_lossy(&out.stdout);
    let apps: Vec<StartApp> = serde_json::from_str(&text)
        .or_else(|_| serde_json::from_str::<StartApp>(&text).map(|a| vec![a]))
        .unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    apps.into_iter()
        .filter(|a| {
            let n = a.name.to_lowercase();
            !n.starts_with("uninstall") && !n.contains("readme") && seen.insert(n)
        })
        .map(|a| Item { kind: "app", subtitle: "App".into(), name: a.name, target: a.app_id, icon: None })
        .collect()
}

#[cfg(not(windows))]
fn load_apps() -> Vec<Item> {
    Vec::new()
}

/// Recently opened items (the shortcuts Windows keeps in Recent).
fn recent_files() -> Vec<Item> {
    let Some(appdata) = std::env::var_os("APPDATA") else { return Vec::new() };
    let dir = std::path::Path::new(&appdata).join("Microsoft\\Windows\\Recent");
    newest(&dir, |p| p.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false), 60)
        .into_iter()
        .filter_map(|p| {
            let name = p.file_stem()?.to_string_lossy().to_string();
            Some(Item { kind: "file", subtitle: "Recent".into(), name, target: p.to_string_lossy().to_string(), icon: None })
        })
        .collect()
}

/// Files sitting in Desktop, Downloads and Documents (top level only).
fn home_files() -> Vec<Item> {
    let Some(home) = std::env::var_os("USERPROFILE") else { return Vec::new() };
    let home = std::path::PathBuf::from(home);
    let mut v = Vec::new();
    for folder in ["Desktop", "Downloads", "Documents"] {
        let dir = home.join(folder);
        for p in newest(&dir, |p| p.is_file() && !is_hidden_name(p), 150) {
            let Some(name) = p.file_name().map(|n| n.to_string_lossy().to_string()) else { continue };
            v.push(Item { kind: "file", subtitle: folder.into(), name, target: p.to_string_lossy().to_string(), icon: None });
        }
    }
    v
}

fn is_hidden_name(p: &std::path::Path) -> bool {
    let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
    n.starts_with('.') || n == "desktop.ini" || n.starts_with("~$")
}

fn newest(dir: &std::path::Path, keep: impl Fn(&std::path::Path) -> bool, limit: usize) -> Vec<std::path::PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<(std::time::SystemTime, std::path::PathBuf)> = rd
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| keep(p))
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    files.sort_by(|a, b| b.0.cmp(&a.0));
    files.into_iter().take(limit).map(|(_, p)| p).collect()
}

/// Refresh the app list in the background if it's stale.
/// Deep file matches for a query, with icons where ready.
pub fn files(query: &str) -> Vec<Item> {
    let mut v: Vec<Item> = crate::files::find(query, 30)
        .into_iter()
        .map(|(name, path, folder)| Item { kind: "file", subtitle: folder, name, target: path, icon: None })
        .collect();
    // Windows Search index: adds matches by contents and in folders Cove doesn't walk.
    let seen: std::collections::HashSet<String> = v.iter().map(|i| i.target.to_lowercase()).collect();
    for (name, path, folder) in crate::winindex::find(query) {
        if !seen.contains(&path.to_lowercase()) {
            let subtitle = if folder.is_empty() { "Windows Search".to_string() } else { format!("{folder} · Windows Search") };
            v.push(Item { kind: "file", subtitle, name, target: path, icon: None });
        }
    }
    let mut missing = Vec::new();
    for it in v.iter_mut() {
        match crate::icons::get(&it.target) {
            Some(i) => it.icon = Some(i),
            None => missing.push(it.target.clone()),
        }
    }
    if !missing.is_empty() {
        render_icons(missing);
    }
    v
}

pub fn warm() {
    crate::files::warm();
    let stale = match APPS.lock().unwrap().as_ref() {
        Some((t, _)) => t.elapsed() > APPS_TTL,
        None => true,
    };
    if stale {
        std::thread::spawn(|| {
            let apps = load_apps();
            if !apps.is_empty() {
                *APPS.lock().unwrap() = Some((Instant::now(), apps));
            }
        });
    }
}

pub fn index() -> Vec<Item> {
    warm();
    let mut v = APPS.lock().unwrap().as_ref().map(|(_, a)| a.clone()).unwrap_or_default();
    v.extend(recent_files());
    v.extend(home_files());
    // Attach icons we have; render the rest in the background.
    let mut missing = Vec::new();
    for it in v.iter_mut() {
        if let Some(p) = icon_path(it) {
            match crate::icons::get(&p) {
                Some(i) => it.icon = Some(i),
                None => missing.push(p),
            }
        }
    }
    if !missing.is_empty() {
        render_icons(missing);
    }
    v.extend(static_items());
    v
}

static RENDERING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn render_icons(paths: Vec<String>) {
    use std::sync::atomic::Ordering;
    if RENDERING.swap(true, Ordering::SeqCst) {
        return;
    }
    std::thread::spawn(move || {
        crate::icons::fill(paths);
        RENDERING.store(false, Ordering::SeqCst);
    });
}

pub fn explorer(arg: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(arg).spawn()?;
    }
    #[cfg(not(windows))]
    let _ = arg;
    Ok(())
}

fn valid_app_id(id: &str) -> bool {
    !id.is_empty() && !id.contains(['"', '\n', '\r']) && id.len() < 512
}

pub fn open(kind: &str, target: &str) -> Result<(), String> {
    let r = match kind {
        "app" if valid_app_id(target) => explorer(&format!("shell:AppsFolder\\{target}")),
        "setting" if target.starts_with("ms-settings:") => explorer(target),
        "file" if std::path::Path::new(target).is_file() => explorer(target),
        "action" => return action(target),
        _ => return Err("Can't open that.".into()),
    };
    r.map_err(|e| e.to_string())
}

/// Show a recent file in its folder (follows the Recent shortcut).
pub fn reveal(target: &str) -> Result<(), String> {
    if !std::path::Path::new(target).is_file() {
        return Err("File not found.".into());
    }
    explorer(&format!("/select,{target}")).map_err(|e| e.to_string())
}

pub fn action(id: &str) -> Result<(), String> {
    let r = match id {
        "lock" => {
            #[cfg(windows)]
            unsafe {
                windows_sys::Win32::System::Shutdown::LockWorkStation();
            }
            Ok(())
        }
        "recycle" => explorer("shell:RecycleBinFolder"),
        "downloads" => explorer("shell:Downloads"),
        "documents" => explorer("shell:Personal"),
        _ => return Err("Unknown action.".into()),
    };
    r.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timing_median() {
        assert_eq!(timing_of(&[]), Timing::default());
        let t = timing_of(&[120, 40, 60]);
        assert_eq!((t.last, t.median, t.count), (Some(60), Some(60), 3));
    }

    #[test]
    fn static_items_are_safe() {
        for s in SETTINGS {
            assert!(s.1.starts_with("ms-settings:"));
        }
        assert!(open("setting", "https://evil.example").is_err());
        assert!(open("nope", "x").is_err());
        assert!(!valid_app_id("a\"b"));
    }
}

// ---- How fast Search opens (hotkey -> first painted frame) ----

static OPEN_AT: Mutex<Option<Instant>> = Mutex::new(None);
static OPEN_MS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

/// Alt+Space pressed.
pub fn mark_open() {
    *OPEN_AT.lock().unwrap() = Some(Instant::now());
}

/// Search's first frame painted after opening; returns the time it took.
pub fn mark_shown() -> Option<u32> {
    let t = OPEN_AT.lock().unwrap().take()?;
    let ms = t.elapsed().as_millis().min(60_000) as u32;
    let mut v = OPEN_MS.lock().unwrap();
    v.push(ms);
    if v.len() > 20 {
        v.remove(0);
    }
    Some(ms)
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct Timing {
    pub last: Option<u32>,
    pub median: Option<u32>,
    pub count: usize,
}

pub fn timing_of(v: &[u32]) -> Timing {
    let mut s = v.to_vec();
    s.sort_unstable();
    Timing { last: v.last().copied(), median: s.get(s.len() / 2).copied(), count: v.len() }
}

pub fn timing() -> Timing {
    timing_of(&OPEN_MS.lock().unwrap())
}
