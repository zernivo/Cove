//! Friendly app names for the menu bar and the dock: "Google Chrome", not
//! "chrome"; "Notifications", not "ShellExperienceHost".
//!
//! Order: Windows' own shell hosts (mapped to what you see), a short list of
//! well-known programs, the app's AUMID display name (Store apps), the exe's
//! FileDescription (the name Task Manager shows), then the exe name.

use std::collections::HashMap;
use std::sync::Mutex;

static CACHE: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

fn stem(exe: &str) -> String {
    std::path::Path::new(exe)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Windows' shell processes, named for what is on screen. `None` = not a shell host.
pub fn shell_host(stem: &str) -> Option<&'static str> {
    Some(match stem {
        "shellexperiencehost" => "Notifications",
        "startmenuexperiencehost" => "Start",
        "searchhost" | "searchapp" | "searchui" => "Search",
        "shellhost" => "Quick Settings",
        "lockapp" => "Lock screen",
        "textinputhost" => "Emoji and keyboard",
        "screenclippinghost" => "Snipping Tool",
        "credentialuibroker" => "Windows Security",
        "consent" => "User Account Control",
        "widgets" | "widgetservice" => "Widgets",
        "pickerhost" => "Windows",
        _ => return None,
    })
}

const KNOWN: &[(&str, &str)] = &[
    ("msedge", "Microsoft Edge"),
    ("chrome", "Google Chrome"),
    ("firefox", "Firefox"),
    ("explorer", "File Explorer"),
    ("code", "Visual Studio Code"),
    ("winword", "Word"),
    ("excel", "Excel"),
    ("powerpnt", "PowerPoint"),
    ("outlook", "Outlook"),
    ("olk", "Outlook"),
    ("windowsterminal", "Terminal"),
    ("notepad", "Notepad"),
    ("systemsettings", "Settings"),
    ("taskmgr", "Task Manager"),
    ("mmc", "Management Console"),
    ("regedit", "Registry Editor"),
    ("cmd", "Command Prompt"),
    ("powershell", "Windows PowerShell"),
    ("pwsh", "PowerShell"),
    ("mspaint", "Paint"),
    ("chatgpt", "ChatGPT"),
    ("cove", "Cove"),
];

pub fn known(stem: &str) -> Option<&'static str> {
    KNOWN.iter().find(|(k, _)| *k == stem).map(|(_, n)| *n)
}

/// "chrome" -> "Chrome": the last resort.
pub fn capitalise(stem: &str) -> String {
    let mut c = stem.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A FileDescription worth showing (some exes ship "", "-" or a file name).
pub fn good_description(d: &str) -> Option<String> {
    let d = d.trim();
    let l = d.to_lowercase();
    if d.len() < 2 || d.len() > 48 || l.ends_with(".exe") || l.ends_with(".dll") || !d.chars().any(|c| c.is_alphabetic()) {
        return None;
    }
    Some(d.to_string())
}

/// The name to show for a window's app. `title` is used only for windows
/// whose process Windows won't name (an app running as admin, a Store app
/// frame with no AppID).
pub fn friendly(exe: &str, aumid: Option<&str>, title: &str) -> String {
    let key = format!("{}|{}", exe.to_lowercase(), aumid.unwrap_or("").to_lowercase());
    if let Some(n) = CACHE.lock().ok().and_then(|c| c.as_ref().and_then(|m| m.get(&key).cloned())) {
        return n;
    }
    let s = stem(exe);
    let frame = s == "applicationframehost";
    let packaged = frame || exe.to_lowercase().contains("\\windowsapps\\");
    let aumid_name = || aumid.and_then(crate::shellprops::app_name);
    let found = shell_host(&s)
        .or_else(|| known(&s))
        .map(String::from)
        .or_else(|| if packaged { aumid_name() } else { None })
        .or_else(|| {
            if exe.is_empty() || frame {
                None
            } else {
                crate::shellprops::file_description(exe).and_then(|d| good_description(&d))
            }
        })
        .or_else(aumid_name)
        .or_else(|| (!s.is_empty() && !frame).then(|| capitalise(&s)));
    match found {
        Some(n) => {
            if let Ok(mut c) = CACHE.lock() {
                c.get_or_insert_with(HashMap::new).insert(key, n.clone());
            }
            n
        }
        None => title.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shell_hosts_have_names() {
        assert_eq!(shell_host("shellexperiencehost"), Some("Notifications"));
        assert_eq!(shell_host("startmenuexperiencehost"), Some("Start"));
        assert_eq!(shell_host("obsidian"), None);
    }
    #[test]
    fn known_and_fallback_names() {
        assert_eq!(known("msedge"), Some("Microsoft Edge"));
        assert_eq!(capitalise("obsidian"), "Obsidian");
        assert_eq!(good_description("Google Chrome").as_deref(), Some("Google Chrome"));
        assert_eq!(good_description("setup.exe"), None);
        assert_eq!(good_description(" - "), None);
    }
}
