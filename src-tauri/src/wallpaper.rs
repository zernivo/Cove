//! Cove wallpapers and the light/dark schedule.
//!
//! The wallpapers are original art bundled with Cove (brand/wallpapers.py),
//! one light and one dark version per scene. They are applied through the
//! documented SystemParametersInfo(SPI_SETDESKWALLPAPER). The wallpaper you had
//! before is remembered the first time and put back by "Restore my wallpaper"
//! or Restore all.

use serde::Serialize;

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
