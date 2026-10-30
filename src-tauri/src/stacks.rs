//! Dock folder stacks: a folder (Downloads by default) shown in the Dock,
//! opening to its newest files. Reads only the user's own folders, and
//! opens files only when they really live in one of them.

use serde::Serialize;
use std::path::{Path, PathBuf};

/// The folders a stack can show: (key, folder name).
pub const KINDS: [(&str, &str); 4] =
    [("downloads", "Downloads"), ("documents", "Documents"), ("desktop", "Desktop"), ("pictures", "Pictures")];

pub fn label(key: &str) -> Option<&'static str> {
    KINDS.iter().find(|(k, _)| *k == key).map(|(_, n)| *n)
}

/// The folder on disk, also when OneDrive backs it up.
pub fn folder(key: &str) -> Option<PathBuf> {
    let name = label(key)?;
    let home = PathBuf::from(std::env::var_os("USERPROFILE")?);
    [home.join(name), home.join("OneDrive").join(name)].into_iter().find(|p| p.is_dir())
}

#[derive(Debug, Clone, Serialize)]
pub struct StackFile {
    pub name: String,
    pub path: String,
    pub dir: bool,
    pub icon: Option<String>,
}

fn skip(name: &str) -> bool {
    name.starts_with('.') || name.starts_with('~') || name.eq_ignore_ascii_case("desktop.ini")
}

/// Newest first (by when the file arrived in the folder), at most `limit`.
pub fn list(key: &str, limit: usize) -> Vec<StackFile> {
    let Some(dir) = folder(key) else { return Vec::new() };
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut v: Vec<(std::time::SystemTime, PathBuf, bool)> = rd
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if skip(&name) {
                return None;
            }
            let md = e.metadata().ok()?;
            let t = md.created().or_else(|_| md.modified()).ok()?;
            Some((t, e.path(), md.is_dir()))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.truncate(limit);
    let mut jobs = Vec::new();
    let out = v
        .into_iter()
        .map(|(_, p, dir)| {
            let path = p.to_string_lossy().to_string();
            let icon = crate::icons::get(&path);
            if icon.is_none() {
                jobs.push(path.clone());
            }
            StackFile { name: p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), path, dir, icon }
        })
        .collect();
    if !jobs.is_empty() {
        std::thread::spawn(move || crate::icons::fill(jobs));
    }
    out
}

/// True when `path` sits directly in one of the stack folders.
pub fn allowed(path: &str) -> bool {
    let p = Path::new(path);
    let (Some(parent), true) = (p.parent(), p.exists()) else { return false };
    let Ok(parent) = parent.canonicalize() else { return false };
    KINDS.iter().filter_map(|(k, _)| folder(k)).filter_map(|f| f.canonicalize().ok()).any(|f| f == parent)
}

/// Open a file or subfolder from a stack, or show it in File Explorer.
pub fn open(path: &str, reveal: bool) -> Result<(), String> {
    if !allowed(path) {
        return Err("That isn't in a Dock folder.".into());
    }
    let mut cmd = std::process::Command::new("explorer.exe");
    if reveal {
        cmd.arg(format!("/select,{path}"));
    } else {
        cmd.arg(path);
    }
    cmd.spawn().map(|_| ()).map_err(|e| e.to_string())
}

pub fn open_folder(key: &str) -> Result<(), String> {
    let f = folder(key).ok_or("Folder not found.")?;
    std::process::Command::new("explorer.exe").arg(f).spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Keep only known keys, in the canonical order, without repeats.
pub fn clean(keys: &[String]) -> Vec<String> {
    KINDS.iter().filter(|(k, _)| keys.iter().any(|x| x == k)).map(|(k, _)| k.to_string()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn cleans_keys() {
        let k = vec!["pictures".to_string(), "bogus".into(), "downloads".into(), "downloads".into()];
        assert_eq!(super::clean(&k), vec!["downloads", "pictures"]);
        assert!(super::label("music").is_none());
    }
}
