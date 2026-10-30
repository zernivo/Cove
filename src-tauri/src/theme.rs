//! Cove Glass themes: Tidepool (default) and Slate, light/dark/system
//! appearance, and Cove's own accent or the Windows accent colour.
//! Reading the Windows accent is read-only; Cove never changes it.

use serde::Serialize;

pub const THEMES: &[&str] = &["tidepool", "slate"];
pub const MODES: &[&str] = &["system", "light", "dark"];
pub const ACCENTS: &[&str] = &["cove", "windows"];

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct ThemeState {
    pub theme: String,
    pub mode: String,
    pub accent_source: String,
    /// "#rrggbb" when following the Windows accent; None means the theme's own.
    pub accent: Option<String>,
}

pub fn pick<'a>(v: &str, allowed: &[&'a str]) -> &'a str {
    allowed.iter().copied().find(|a| *a == v).unwrap_or(allowed[0])
}

/// DWM stores the accent as 0xAABBGGRR.
pub fn abgr_to_hex(v: u32) -> String {
    let r = v & 0xff;
    let g = (v >> 8) & 0xff;
    let b = (v >> 16) & 0xff;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

#[cfg(windows)]
pub fn windows_accent() -> Option<String> {
    use winreg::enums::HKEY_CURRENT_USER;
    let key = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Microsoft\\Windows\\DWM").ok()?;
    let v: u32 = key.get_value("AccentColor").ok()?;
    Some(abgr_to_hex(v))
}

#[cfg(not(windows))]
pub fn windows_accent() -> Option<String> {
    None
}

pub fn state(theme: &str, mode: &str, accent: &str) -> ThemeState {
    let accent_source = pick(accent, ACCENTS).to_string();
    ThemeState {
        theme: pick(theme, THEMES).to_string(),
        mode: pick(mode, MODES).to_string(),
        accent: if accent_source == "windows" { windows_accent() } else { None },
        accent_source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_values_fall_back() {
        assert_eq!(pick("neon", THEMES), "tidepool");
        assert_eq!(pick("slate", THEMES), "slate");
        assert_eq!(pick("", MODES), "system");
        let s = state("x", "dark", "cove");
        assert_eq!((s.theme.as_str(), s.mode.as_str(), s.accent), ("tidepool", "dark", None));
    }

    #[test]
    fn accent_colour_order() {
        // 0xAABBGGRR: red 0x11, green 0x22, blue 0x33
        assert_eq!(abgr_to_hex(0xff332211), "#112233");
    }
}
