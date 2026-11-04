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
    /// Friendly name of the app in front ("Google Chrome", "Desktop").
    pub app: String,
    /// "app" | "cove" | "desktop" | "shell" (Notifications, Start, ...).
    pub kind: &'static str,
    /// The app's front window, for the app menu (0 when there's none).
    pub hwnd: isize,
    pub battery: Option<Battery>,
    pub volume: Option<u8>,
    pub muted: bool,
}

/// The app in front, as the menu bar names it.
#[derive(Debug, Clone, PartialEq)]
pub struct Front {
    pub name: String,
    pub kind: &'static str,
    pub hwnd: isize,
}

pub fn state() -> TopState {
    let (volume, muted) = crate::controls::sound();
    let f = imp::front();
    TopState { app: f.name, kind: f.kind, hwnd: f.hwnd, battery: imp::battery(), volume, muted }
}

/// Name a foreground window from its class and exe (pure: unit-tested).
pub fn classify(class: &str, exe: &str, aumid: Option<&str>, title: &str) -> (String, &'static str) {
    if matches!(class, "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") {
        return ("Desktop".into(), "desktop");
    }
    let stem = std::path::Path::new(exe).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    if let Some(n) = crate::appnames::shell_host(&stem) {
        return (n.into(), "shell");
    }
    // Explorer's own windows other than folders (tray overflow, Task View,
    // the desktop's helpers) are "the desktop", as on a Mac.
    if stem == "explorer" && class != "CabinetWClass" && class != "ExploreWClass" {
        return ("Desktop".into(), "desktop");
    }
    (crate::appnames::friendly(exe, aumid, title), "app")
}

pub use imp::{power, refresh_icons, register, unregister};

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
        GetWindowThreadProcessId, IsWindow, SM_CXSCREEN, WM_USER,
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

    fn title(h: HWND) -> String {
        unsafe {
            let n = GetWindowTextLengthW(h);
            let mut t = vec![0u16; n.max(0) as usize + 1];
            let got = GetWindowTextW(h, t.as_mut_ptr(), t.len() as i32);
            String::from_utf16_lossy(&t[..got.max(0) as usize])
        }
    }

    /// The last window in front that wasn't one of Cove's own bars, so
    /// clicking the menu bar or the dock doesn't rename the app to "Cove".
    static LAST: std::sync::Mutex<isize> = std::sync::Mutex::new(0);

    pub fn front() -> super::Front {
        unsafe {
            let mut h = GetForegroundWindow();
            let desktop = || super::Front { name: "Desktop".into(), kind: "desktop", hwnd: 0 };
            if h.is_null() {
                return desktop();
            }
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            if pid == GetCurrentProcessId() {
                // Cove's own window ("Cove") is an app; its bars are not.
                if title(h) == "Cove" {
                    return super::Front { name: "Cove".into(), kind: "cove", hwnd: h as isize };
                }
                let last = LAST.lock().map(|g| *g).unwrap_or(0) as HWND;
                if last.is_null() || IsWindow(last) == 0 {
                    return desktop();
                }
                h = last;
                GetWindowThreadProcessId(h, &mut pid);
            } else if let Ok(mut g) = LAST.lock() {
                *g = h as isize;
            }
            let mut cls = [0u16; 64];
            let n = GetClassNameW(h, cls.as_mut_ptr(), 64);
            let class = String::from_utf16_lossy(&cls[..n.max(0) as usize]);
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
            let aumid = crate::shellprops::window_aumid(h as isize);
            let (name, kind) = super::classify(&class, &exe, aumid.as_deref(), &title(h));
            let hwnd = if kind == "app" { h as isize } else { 0 };
            super::Front { name, kind, hwnd }
        }
    }

    /// Sleep, Restart or Shut down through Windows itself. Restart and Shut
    /// down ask Windows to show its own confirmation (shell32 RestartDialog,
    /// the prompt installers use); Err("confirm") when that isn't available,
    /// and the menu bar asks instead (then `power_now`).
    pub fn power(what: &str) -> Result<(), String> {
        use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
        match what {
            "sleep" => {
                // Sleep, not hibernate; wake events stay enabled.
                unsafe { windows_sys::Win32::System::Power::SetSuspendState(0, 0, 0) };
                Ok(())
            }
            "restart" | "shutdown" => unsafe {
                let dll: Vec<u16> = "shell32.dll\0".encode_utf16().collect();
                let m = LoadLibraryW(dll.as_ptr());
                if m.is_null() {
                    return Err("confirm".into());
                }
                // RestartDialog is exported by ordinal 59 only.
                let Some(f) = GetProcAddress(m, 59usize as *const u8) else { return Err("confirm".into()) };
                type RestartDialog = unsafe extern "system" fn(HWND, *const u16, u32) -> i32;
                let f: RestartDialog = std::mem::transmute(f);
                // EWX_REBOOT, or EWX_SHUTDOWN | EWX_POWEROFF.
                let flags: u32 = if what == "restart" { 0x2 } else { 0x1 | 0x8 };
                // The dialog waits for an answer: keep it off Cove's threads.
                let fp = f as usize;
                std::thread::spawn(move || {
                    let f: RestartDialog = std::mem::transmute(fp);
                    let empty = [0u16];
                    f(std::ptr::null_mut(), empty.as_ptr(), flags);
                });
                Ok(())
            },
            _ => Err("Unknown item.".into()),
        }
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
    pub fn front() -> super::Front {
        super::Front { name: "Desktop".into(), kind: "desktop", hwnd: 0 }
    }
    pub fn power(_what: &str) -> Result<(), String> {
        Err("confirm".into())
    }
}

#[cfg(test)]
mod tests {
    use super::classify;
    #[test]
    fn names_the_front_app() {
        assert_eq!(classify("Progman", r"C:\Windows\explorer.exe", None, ""), ("Desktop".into(), "desktop"));
        assert_eq!(classify("WorkerW", "", None, ""), ("Desktop".into(), "desktop"));
        let n = classify("Windows.UI.Core.CoreWindow", r"C:\Windows\SystemApps\ShellExperienceHost_cw5n1h2txyewy\ShellExperienceHost.exe", None, "");
        assert_eq!(n, ("Notifications".into(), "shell"));
        assert_eq!(classify("CabinetWClass", r"C:\Windows\explorer.exe", None, "Downloads"), ("File Explorer".into(), "app"));
        assert_eq!(classify("XamlExplorerHostIslandWindow", r"C:\Windows\explorer.exe", None, ""), ("Desktop".into(), "desktop"));
        assert_eq!(classify("Chrome_WidgetWin_1", r"C:\x\msedge.exe", None, "New tab"), ("Microsoft Edge".into(), "app"));
    }
}
