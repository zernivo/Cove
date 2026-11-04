//! Cove wallpapers and the light/dark schedule.
//!
//! The wallpapers are original art bundled with Cove (brand/wallpapers.py),
//! one light and one dark version per scene. They are applied through the
//! documented SystemParametersInfo(SPI_SETDESKWALLPAPER). The wallpaper you had
//! before is remembered the first time and put back by "Restore my wallpaper"
//! or Restore all.
//!
//! The gallery (ui/public/wallpapers/manifest.json, compiled in here) adds
//! credited photos from Wikimedia Commons and NASA plus more Cove art hosted on
//! the `wallpapers-v1` release. Nothing is fetched until the user picks one:
//! the file is downloaded once with a descriptive User-Agent, checked, cached
//! in Cove's data folder and set with the same SystemParametersInfo call.

use serde::{Deserialize, Serialize};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;

pub const SCENES: &[&str] = &["tidepool", "lagoon", "dune", "slate"];

#[derive(Debug, Clone, Serialize)]
pub struct WallpaperStatus {
    pub scenes: Vec<String>,
    /// The Cove scene in use, or "" when the user's own wallpaper is shown.
    pub current: String,
    pub has_prev: bool,
    /// "off" or "times".
    pub schedule: String,
    pub light_at: String,
    pub dark_at: String,
    pub dark_now: bool,
}

pub fn variant(dark: bool) -> &'static str {
    if dark {
        "dark"
    } else {
        "light"
    }
}

pub fn file_name(scene: &str, dark: bool) -> String {
    format!("{}-{}.jpg", scene, variant(dark))
}

/// "HH:MM" -> minutes after midnight.
pub fn parse_hm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let h: u32 = h.trim().parse().ok()?;
    let m: u32 = m.trim().parse().ok()?;
    if h < 24 && m < 60 {
        Some(h * 60 + m)
    } else {
        None
    }
}

/// Light from `light_at` until `dark_at`, dark otherwise; either order works.
pub fn scheduled_dark(now: u32, light_at: u32, dark_at: u32) -> bool {
    if light_at == dark_at {
        return false;
    }
    if light_at < dark_at {
        !(now >= light_at && now < dark_at)
    } else {
        now >= dark_at && now < light_at
    }
}

// ---- Gallery ----

/// The curated list. Rust only ever downloads URLs from this compiled-in copy,
/// so the UI can name a wallpaper by id but never hand Cove an arbitrary URL.
pub const MANIFEST: &str = include_str!("../../ui/public/wallpapers/manifest.json");

/// Largest download Cove accepts for one wallpaper.
const MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Item {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub sha256: String,
    /// A Cove scene shipped with the app ("tidepool"), instead of a download.
    #[serde(default)]
    pub bundled: String,
    #[serde(rename = "sourceUrl", default)]
    pub source_url: String,
}

#[derive(Deserialize)]
struct Manifest {
    items: Vec<Item>,
}

pub fn items() -> &'static [Item] {
    static ITEMS: OnceLock<Vec<Item>> = OnceLock::new();
    ITEMS.get_or_init(|| serde_json::from_str::<Manifest>(MANIFEST).map(|m| m.items).unwrap_or_default())
}

pub fn item(id: &str) -> Option<&'static Item> {
    items().iter().find(|i| i.id == id)
}

pub fn user_agent() -> String {
    format!("Cove/{} (https://github.com/zernivo/Cove) ureq/2", env!("CARGO_PKG_VERSION"))
}

/// Ids are lowercase words joined by hyphens; anything else never reaches the disk.
pub fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 80 && id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn cache_file(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{}.jpg", id))
}

/// The file type Windows can show, from its first bytes.
pub fn image_ext(head: &[u8]) -> Option<&'static str> {
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if head.starts_with(&[0x89, b'P', b'N', b'G']) {
        Some("png")
    } else if head.starts_with(b"BM") {
        Some("bmp")
    } else {
        None
    }
}

/// Bumped by every pick, so a download the user has moved on from stops.
static PICK: AtomicU64 = AtomicU64::new(0);

pub fn next_pick() -> u64 {
    PICK.fetch_add(1, Ordering::SeqCst) + 1
}

pub fn still_wanted(ticket: u64) -> bool {
    PICK.load(Ordering::SeqCst) == ticket
}

pub const CANCELLED: &str = "cancelled";

/// Download `item` into `dir` (once), reporting (received, total) as it goes.
/// Writes to a .part file and renames it only after the checks pass.
pub fn fetch(item: &Item, dir: &Path, ticket: u64, mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, String> {
    if !safe_id(&item.id) || !item.url.starts_with("https://") {
        return Err("This wallpaper can't be downloaded".into());
    }
    let dst = cache_file(dir, &item.id);
    if dst.exists() {
        return Ok(dst);
    }
    std::fs::create_dir_all(dir).map_err(|e| format!("Couldn't make the wallpaper folder: {}", e))?;
    let agent = ureq::AgentBuilder::new()
        .user_agent(&user_agent())
        .timeout_connect(std::time::Duration::from_secs(15))
        .timeout_read(std::time::Duration::from_secs(30))
        .redirects(5)
        .build();
    let resp = agent.get(&item.url).call().map_err(|e| match e {
        ureq::Error::Status(429, _) => "The library is busy. Try again in a minute.".to_string(),
        ureq::Error::Status(code, _) => format!("The library answered {}", code),
        ureq::Error::Transport(_) => "Couldn't reach the library. Check your connection.".to_string(),
    })?;
    let total: u64 = resp.header("Content-Length").and_then(|v| v.parse().ok()).unwrap_or(0);
    if total > MAX_BYTES {
        return Err("That file is too large".into());
    }
    let part = dir.join(format!("{}.part", item.id));
    let mut out = std::fs::File::create(&part).map_err(|e| format!("Couldn't save the wallpaper: {}", e))?;
    let mut reader = resp.into_reader().take(MAX_BYTES + 1);
    let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
    let mut buf = vec![0u8; 64 * 1024];
    let (mut got, mut head) = (0u64, Vec::with_capacity(8));
    let mut last = std::time::Instant::now();
    let result = (|| -> Result<(), String> {
        loop {
            if !still_wanted(ticket) {
                return Err(CANCELLED.into());
            }
            let n = reader.read(&mut buf).map_err(|_| "The download stopped. Check your connection.".to_string())?;
            if n == 0 {
                break;
            }
            if head.len() < 8 {
                head.extend_from_slice(&buf[..n.min(8 - head.len())]);
            }
            sha2::Digest::update(&mut hasher, &buf[..n]);
            out.write_all(&buf[..n]).map_err(|e| format!("Couldn't save the wallpaper: {}", e))?;
            got += n as u64;
            if got > MAX_BYTES {
                return Err("That file is too large".into());
            }
            if last.elapsed().as_millis() >= 80 {
                progress(got, total);
                last = std::time::Instant::now();
            }
        }
        if total > 0 && got != total {
            return Err("The download was cut short. Try again.".into());
        }
        if image_ext(&head) != Some("jpg") {
            return Err("That wasn't a picture Cove can use".into());
        }
        if !item.sha256.is_empty() {
            let sum: String = sha2::Digest::finalize(hasher).iter().map(|b| format!("{:02x}", b)).collect();
            if !sum.eq_ignore_ascii_case(&item.sha256) {
                return Err("The file didn't match its checksum".into());
            }
        }
        Ok(())
    })();
    drop(out);
    if let Err(e) = result {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    progress(got, got.max(total));
    std::fs::rename(&part, &dst).map_err(|e| format!("Couldn't save the wallpaper: {}", e))?;
    Ok(dst)
}

/// Ids already downloaded, so the gallery can show them as ready.
pub fn cached(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(".jpg")).map(|s| s.to_string()))
                .filter(|id| item(id).is_some())
                .collect()
        })
        .unwrap_or_default()
}

/// Open a credit's source page (only pages listed in the manifest).
pub fn open_source(id: &str) -> Result<(), String> {
    let it = item(id).ok_or("Unknown wallpaper")?;
    if !it.source_url.starts_with("https://") {
        return Err("No source page".into());
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(&it.source_url).spawn().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::SystemInformation::GetLocalTime;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPIF_SENDWININICHANGE, SPIF_UPDATEINIFILE, SPI_GETDESKWALLPAPER, SPI_SETDESKWALLPAPER,
    };

    pub fn current() -> Option<String> {
        let mut buf = [0u16; 1024];
        let ok = unsafe { SystemParametersInfoW(SPI_GETDESKWALLPAPER, buf.len() as u32, buf.as_mut_ptr() as *mut _, 0) };
        if ok == 0 {
            return None;
        }
        let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Some(String::from_utf16_lossy(&buf[..n]))
    }

    pub fn set(path: &str) -> Result<(), String> {
        let mut w: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let ok = unsafe {
            SystemParametersInfoW(SPI_SETDESKWALLPAPER, 0, w.as_mut_ptr() as *mut _, SPIF_UPDATEINIFILE | SPIF_SENDWININICHANGE)
        };
        if ok == 0 {
            Err(std::io::Error::last_os_error().to_string())
        } else {
            Ok(())
        }
    }

    pub fn local_minutes() -> u32 {
        let mut t = unsafe { std::mem::zeroed() };
        unsafe { GetLocalTime(&mut t) };
        t.wHour as u32 * 60 + t.wMinute as u32
    }

    /// Read-only: whether Windows apps are in dark mode.
    pub fn windows_dark() -> bool {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize")
            .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
            .map(|v| v == 0)
            .unwrap_or(false)
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn current() -> Option<String> {
        None
    }
    pub fn set(_path: &str) -> Result<(), String> {
        Err("Wallpapers need Windows".into())
    }
    pub fn local_minutes() -> u32 {
        12 * 60
    }
    pub fn windows_dark() -> bool {
        false
    }
}

pub use imp::{current, local_minutes, set, windows_dark};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses() {
        let all = items();
        assert!(all.len() >= 50, "manifest has {} items", all.len());
        for it in all {
            assert!(safe_id(&it.id), "bad id {}", it.id);
            if it.bundled.is_empty() {
                assert!(it.url.starts_with("https://"), "{} has no https url", it.id);
            } else {
                assert!(SCENES.contains(&it.bundled.as_str()), "{} names an unknown scene", it.id);
            }
        }
        assert!(item("cove-tidepool").is_some());
    }

    #[test]
    fn ids_and_types() {
        assert!(safe_id("f1-red-bull-montreal-2019"));
        assert!(!safe_id("../evil"));
        assert!(!safe_id("A"));
        assert_eq!(image_ext(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(image_ext(b"\x89PNG\r\n"), Some("png"));
        assert_eq!(image_ext(b"<html>"), None);
    }

    #[test]
    fn times_parse() {
        assert_eq!(parse_hm("07:00"), Some(420));
        assert_eq!(parse_hm("19:30"), Some(1170));
        assert_eq!(parse_hm("24:00"), None);
        assert_eq!(parse_hm("7"), None);
    }

    #[test]
    fn schedule_windows() {
        let (l, d) = (420, 1140); // 07:00 light, 19:00 dark
        assert!(scheduled_dark(60, l, d));
        assert!(!scheduled_dark(420, l, d));
        assert!(!scheduled_dark(1139, l, d));
        assert!(scheduled_dark(1140, l, d));
        // reversed: light in the evening
        assert!(!scheduled_dark(1200, 1140, 420));
        assert!(scheduled_dark(600, 1140, 420));
        assert!(!scheduled_dark(600, 600, 600));
    }

    #[test]
    fn names() {
        assert_eq!(file_name("dune", true), "dune-dark.jpg");
        assert_eq!(file_name("slate", false), "slate-light.jpg");
    }
}
