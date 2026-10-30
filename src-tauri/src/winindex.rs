//! Read-only results from the Windows Search index, through the shell's
//! documented `search-ms:` protocol (IShellItem + BHID_EnumItems). This is
//! the same index File Explorer's search box uses, so it also finds files
//! by their contents. Nothing is changed in the index or its settings; if
//! indexing is off, this simply returns nothing and Cove's own file walk
//! still answers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

pub const LIMIT: usize = 15;
const WAIT: Duration = Duration::from_millis(450);

static ENABLED: AtomicBool = AtomicBool::new(true);
static BUSY: AtomicBool = AtomicBool::new(false);
/// Last finished query and its (name, path, folder) hits.
static CACHE: Mutex<Option<(String, Vec<(String, String, String)>)>> = Mutex::new(None);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
    if !on {
        *CACHE.lock().unwrap() = None;
    }
}

pub fn enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Percent-encode for a search-ms URL parameter.
pub fn enc(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{:02X}", b));
        }
    }
    out
}

pub fn url(query: &str, scope: &str) -> String {
    format!("search-ms:displayname=Cove&query={}&crumb=location:{}", enc(query), enc(scope))
}

/// Index hits for `query`, waiting briefly. A slow query keeps running and
/// its results are ready when Search asks again.
pub fn find(query: &str) -> Vec<(String, String, String)> {
    let q = query.trim().to_string();
    if !enabled() || q.chars().count() < 2 {
        return Vec::new();
    }
    if let Some((cq, hits)) = CACHE.lock().unwrap().as_ref() {
        if *cq == q {
            return hits.clone();
        }
    }
    if BUSY.swap(true, Ordering::SeqCst) {
        return Vec::new();
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let q2 = q.clone();
    std::thread::spawn(move || {
        let hits = imp::query(&q2);
        *CACHE.lock().unwrap() = Some((q2, hits.clone()));
        BUSY.store(false, Ordering::SeqCst);
        let _ = tx.send(hits);
    });
    rx.recv_timeout(WAIT).unwrap_or_default()
}

#[cfg(windows)]
mod imp {
    use super::{url, LIMIT};
    use windows::core::HSTRING;
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_MULTITHREADED};
    use windows::Win32::UI::Shell::{
        IEnumShellItems, IShellItem, SHCreateItemFromParsingName, BHID_EnumItems, SIGDN_FILESYSPATH, SIGDN_NORMALDISPLAY,
    };

    fn name_of(item: &IShellItem, kind: windows::Win32::UI::Shell::SIGDN) -> Option<String> {
        unsafe {
            let p = item.GetDisplayName(kind).ok()?;
            let s = p.to_string().ok();
            CoTaskMemFree(Some(p.0 as *const _));
            s
        }
    }

    pub fn query(q: &str) -> Vec<(String, String, String)> {
        let Some(home) = std::env::var_os("USERPROFILE") else { return Vec::new() };
        let home = home.to_string_lossy().to_string();
        let ok = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }.is_ok();
        let mut out = Vec::new();
        unsafe {
            let item: windows::core::Result<IShellItem> = SHCreateItemFromParsingName(&HSTRING::from(url(q, &home)), None);
            if let Ok(item) = item {
                if let Ok(en) = item.BindToHandler::<_, IEnumShellItems>(None, &BHID_EnumItems) {
                    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
                    while out.len() < LIMIT && std::time::Instant::now() < deadline {
                        let mut got: [Option<IShellItem>; 1] = [None];
                        let mut n = 0u32;
                        if en.Next(&mut got, Some(&mut n)).is_err() || n == 0 {
                            break;
                        }
                        let Some(it) = got[0].take() else { break };
                        let Some(path) = name_of(&it, SIGDN_FILESYSPATH) else { continue };
                        let lower = path.to_lowercase();
                        if lower.contains("\\appdata\\") || lower.contains("\\$recycle.bin\\") {
                            continue;
                        }
                        let p = std::path::Path::new(&path);
                        let name = name_of(&it, SIGDN_NORMALDISPLAY)
                            .or_else(|| p.file_name().map(|f| f.to_string_lossy().to_string()))
                            .unwrap_or_else(|| path.clone());
                        let folder = p
                            .parent()
                            .and_then(|f| f.file_name())
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_default();
                        out.push((name, path, folder));
                    }
                }
            }
        }
        if ok {
            unsafe { CoUninitialize() };
        }
        out
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn query(_q: &str) -> Vec<(String, String, String)> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_safe_urls() {
        let u = url("tax 2026&x", "C:\\Users\\Ana");
        assert_eq!(u, "search-ms:displayname=Cove&query=tax%202026%26x&crumb=location:C%3A%5CUsers%5CAna");
        assert!(!u.contains(' '));
    }

    #[test]
    fn short_queries_skip() {
        assert!(find("a").is_empty());
    }
}
