//! Deeper file search: walks your personal folders in the background and
//! matches names in Rust, so Search can find files beyond the newest few.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const TTL: Duration = Duration::from_secs(300);
const CAP: usize = 30_000;
const DEPTH: usize = 6;
const SKIP: &[&str] = &["node_modules", "appdata", "$recycle.bin", "target", "__pycache__", "venv", ".venv"];

struct Entry {
    lower: String,
    name: String,
    path: String,
    folder: String,
}

static INDEX: Mutex<Option<(Instant, Vec<Entry>)>> = Mutex::new(None);
static BUILDING: AtomicBool = AtomicBool::new(false);

pub fn warm() {
    let stale = INDEX.lock().unwrap().as_ref().map_or(true, |(t, _)| t.elapsed() > TTL);
    if stale && !BUILDING.swap(true, Ordering::SeqCst) {
        std::thread::spawn(|| {
            let v = build();
            *INDEX.lock().unwrap() = Some((Instant::now(), v));
            BUILDING.store(false, Ordering::SeqCst);
        });
    }
}

fn build() -> Vec<Entry> {
    let Some(home) = std::env::var_os("USERPROFILE") else { return Vec::new() };
    let home = PathBuf::from(home);
    let mut out = Vec::new();
    for f in ["Desktop", "Documents", "Downloads", "Pictures", "Videos", "Music", "OneDrive"] {
        walk(&home.join(f), f, 0, &mut out);
    }
    out
}

fn walk(dir: &Path, folder: &str, depth: usize, out: &mut Vec<Entry>) {
    if depth > DEPTH || out.len() >= CAP {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.filter_map(|e| e.ok()) {
        if out.len() >= CAP {
            return;
        }
        let name = e.file_name().to_string_lossy().to_string();
        let lower = name.to_lowercase();
        if lower.starts_with('.') || lower.starts_with("~$") || lower == "desktop.ini" {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let path = e.path();
        if ft.is_dir() {
            if !SKIP.contains(&lower.as_str()) {
                walk(&path, folder, depth + 1, out);
            }
        } else {
            out.push(Entry { lower, name, path: path.to_string_lossy().to_string(), folder: folder.to_string() });
        }
    }
}

/// (name, path, folder) of the best matches.
pub fn find(query: &str, limit: usize) -> Vec<(String, String, String)> {
    warm();
    let q = query.trim().to_lowercase();
    if q.len() < 2 {
        return Vec::new();
    }
    let g = INDEX.lock().unwrap();
    let Some((_, v)) = g.as_ref() else { return Vec::new() };
    let mut hits: Vec<(u32, usize, &Entry)> = v
        .iter()
        .filter_map(|e| score(&e.lower, &q).map(|s| (s, e.lower.len(), e)))
        .collect();
    hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    hits.into_iter().take(limit).map(|(_, _, e)| (e.name.clone(), e.path.clone(), e.folder.clone())).collect()
}

fn score(name: &str, q: &str) -> Option<u32> {
    if name.starts_with(q) {
        Some(3)
    } else if name.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(q)) {
        Some(2)
    } else if name.contains(q) {
        Some(1)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ranks_prefix_first() {
        assert_eq!(super::score("report.pdf", "rep"), Some(3));
        assert_eq!(super::score("ca_report.pdf", "rep"), Some(2));
        assert_eq!(super::score("prepare.txt", "rep"), Some(1));
        assert_eq!(super::score("notes.txt", "rep"), None);
    }
}
