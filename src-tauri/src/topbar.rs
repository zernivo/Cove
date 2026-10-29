//! Cove top bar: a thin bar across the top of the main screen, registered
//! as a documented AppBar (SHAppBarMessage) so maximised windows sit below
//! it instead of under it. Shows the Cove menu, the app in front, battery
//! and the clock.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Battery {
    pub percent: u8,
    pub charging: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopState {
    pub app: String,
    pub battery: Option<Battery>,
}

pub fn state() -> TopState {
    TopState { app: imp::front_app(), battery: imp::battery() }
}

pub use imp::{refresh_icons, register, unregister};

#[cfg(windows)]
mod imp {
    use super::Battery;
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, RECT};
    use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, SHChangeNotify, APPBARDATA};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW,
        GetWindowThreadProcessId, SM_CXSCREEN, WM_USER,
    };

    const ABM_NEW: u32 = 0;
    const ABM_REMOVE: u32 = 1;
    const ABM_QUERYPOS: u32 = 2;
    const ABM_SETPOS: u32 = 3;
    const ABE_TOP: u32 = 1;

    fn data(hwnd: isize) -> APPBARDATA {
        let mut d: APPBARDATA = unsafe { std::mem::zeroed() };
        d.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        d.hWnd = hwnd as HWND;
        d.uCallbackMessage = WM_USER + 0x43;
        d
    }

    /// Reserve `height` physical px at the top of the main screen.
    /// Returns the rectangle the bar should occupy.
    pub fn register(hwnd: isize, height: i32) -> (i32, i32, i32, i32) {
        unsafe {
            let mut d = data(hwnd);
            SHAppBarMessage(ABM_REMOVE, &mut d); // clean up any leftover reservation
            SHAppBarMessage(ABM_NEW, &mut d);
            let w = GetSystemMetrics(SM_CXSCREEN);
            d.uEdge = ABE_TOP;
            d.rc = RECT { left: 0, top: 0, right: w, bottom: height };
            SHAppBarMessage(ABM_QUERYPOS, &mut d);
            d.rc.bottom = d.rc.top + height;
            SHAppBarMessage(ABM_SETPOS, &mut d);
            (d.rc.left, d.rc.top, d.rc.right, d.rc.bottom)
        }
    }

    pub fn unregister(hwnd: isize) {
        unsafe {
            let mut d = data(hwnd);
            SHAppBarMessage(ABM_REMOVE, &mut d);
        }
    }

    /// Tell Explorer to drop cached icons (fixes a stale desktop shortcut
    /// icon after an update).
    pub fn refresh_icons() {
        unsafe { SHChangeNotify(0x0800_0000, 0, std::ptr::null(), std::ptr::null()) };
    }

    pub fn battery() -> Option<Battery> {
        let mut s: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        if unsafe { GetSystemPowerStatus(&mut s) } == 0 || s.BatteryFlag & 128 != 0 || s.BatteryLifePercent > 100 {
            return None; // no battery (desktop PC) or unknown
        }
        Some(Battery { percent: s.BatteryLifePercent, charging: s.ACLineStatus == 1 })
    }

    pub fn front_app() -> String {
        unsafe {
            let h = GetForegroundWindow();
            if h.is_null() {
                return String::new();
            }
            let mut cls = [0u16; 64];
            let n = GetClassNameW(h, cls.as_mut_ptr(), 64);
            let class = String::from_utf16_lossy(&cls[..n.max(0) as usize]);
            if matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") {
                return "Desktop".into();
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            if pid == GetCurrentProcessId() {
                return "Cove".into();
            }
            let ph = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            let mut exe = String::new();
            if !ph.is_null() {
                let mut buf = [0u16; 1024];
                let mut len = buf.len() as u32;
                if QueryFullProcessImageNameW(ph, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len) != 0 {
                    exe = String::from_utf16_lossy(&buf[..len as usize]);
                }
                CloseHandle(ph);
            }
            let stem = std::path::Path::new(&exe).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            if !stem.is_empty() && !stem.eq_ignore_ascii_case("applicationframehost") {
                return super::nice_name(&stem);
            }
            let n = GetWindowTextLengthW(h);
            let mut t = vec![0u16; n as usize + 1];
            let got = GetWindowTextW(h, t.as_mut_ptr(), t.len() as i32);
            String::from_utf16_lossy(&t[..got.max(0) as usize])
        }
    }
}

/// Friendly names for common executables; otherwise capitalise the stem.
pub fn nice_name(stem: &str) -> String {
    let known = [
        ("msedge", "Microsoft Edge"),
        ("chrome", "Google Chrome"),
        ("firefox", "Firefox"),
        ("explorer", "File Explorer"),
        ("code", "Visual Studio Code"),
        ("winword", "Word"),
        ("excel", "Excel"),
        ("powerpnt", "PowerPoint"),
        ("windowsterminal", "Terminal"),
        ("notepad", "Notepad"),
        ("systemsettings", "Settings"),
        ("chatgpt", "ChatGPT"),
    ];
    let l = stem.to_lowercase();
    if let Some((_, n)) = known.iter().find(|(k, _)| *k == l) {
        return n.to_string();
    }
    let mut c = stem.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Battery;
    pub fn register(_h: isize, height: i32) -> (i32, i32, i32, i32) {
        (0, 0, 1920, height)
    }
    pub fn unregister(_h: isize) {}
    pub fn refresh_icons() {}
    pub fn battery() -> Option<Battery> {
        None
    }
    pub fn front_app() -> String {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn friendly_names() {
        assert_eq!(super::nice_name("msedge"), "Microsoft Edge");
        assert_eq!(super::nice_name("obsidian"), "Obsidian");
    }
}
