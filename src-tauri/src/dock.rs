//! Cove Dock: pinned apps plus whatever is running, built from the same
//! "real window" rules Alt+Tab uses. Taskbar auto-hide goes only through
//! its documented switch (SHAppBarMessage) and is put back afterwards.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DockItem {
    /// "pin" | "running" | "trash"
    pub kind: &'static str,
    pub name: String,
    /// AppID for pins, exe path for running apps.
    pub target: String,
    /// Window handles (front to back) when running.
    pub windows: Vec<isize>,
    pub active: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockState {
    pub items: Vec<DockItem>,
}

pub struct Win {
    pub hwnd: isize,
    pub exe: String,
    pub title: String,
}

fn stem(exe: &str) -> String {
    std::path::Path::new(exe)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Does a running exe belong to a pinned AppID / name? A heuristic: Start
/// AppIDs usually carry the exe or product name ("Chrome", "MSEdge",
/// "...\\Code.exe", "Microsoft.Windows.Explorer").
pub fn matches(pin_target: &str, pin_name: &str, exe: &str) -> bool {
    let s = stem(exe);
    if s.len() < 3 || s == "applicationframehost" {
        return false;
    }
    let t = pin_target.to_lowercase();
    let n = pin_name.to_lowercase().replace(' ', "");
    t.contains(&s) || n.contains(&s) || (s == "code" && t.contains("visualstudiocode"))
}

fn pretty(exe: &str, title: &str) -> String {
    let s = std::path::Path::new(exe).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if s.eq_ignore_ascii_case("applicationframehost") || s.is_empty() {
        return title.to_string();
    }
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => title.to_string(),
    }
}

/// Pins are Start AppIDs, or an exe path for apps kept from the running list.
pub fn is_exe_pin(target: &str) -> bool {
    target.to_lowercase().ends_with(".exe") && std::path::Path::new(target).is_absolute()
}

fn pin_icon_path(target: &str) -> String {
    if is_exe_pin(target) {
        target.to_string()
    } else {
        format!("shell:AppsFolder\\{target}")
    }
}

pub fn launch(target: &str) -> Result<(), String> {
    if is_exe_pin(target) {
        crate::search::open("file", target)
    } else {
        crate::search::open("app", target)
    }
}

pub fn state(pins: &[crate::settings::Pin], stacks: &[String]) -> DockState {
    let wins = imp::windows();
    let fg = imp::foreground();
    let mut used = vec![false; wins.len()];
    let mut items = Vec::new();
    let mut icon_jobs = Vec::new();

    for p in pins {
        let mut hw = Vec::new();
        for (i, w) in wins.iter().enumerate() {
            if !used[i] && matches(&p.target, &p.name, &w.exe) {
                used[i] = true;
                hw.push(w.hwnd);
            }
        }
        let path = pin_icon_path(&p.target);
        let icon = crate::icons::get(&path);
        if icon.is_none() {
            icon_jobs.push(path);
        }
        items.push(DockItem {
            kind: "pin",
            name: p.name.clone(),
            target: p.target.clone(),
            active: hw.contains(&fg),
            windows: hw,
            icon,
        });
    }

    // Running apps that aren't pinned, grouped by exe (Store apps by window).
    let mut groups: Vec<DockItem> = Vec::new();
    for (i, w) in wins.iter().enumerate() {
        if used[i] {
            continue;
        }
        let uwp = stem(&w.exe) == "applicationframehost";
        if let Some(g) = groups.iter_mut().find(|g| !uwp && g.target.eq_ignore_ascii_case(&w.exe)) {
            g.windows.push(w.hwnd);
            g.active |= w.hwnd == fg;
            continue;
        }
        let icon = crate::icons::get(&w.exe);
        if icon.is_none() && !uwp {
            icon_jobs.push(w.exe.clone());
        }
        groups.push(DockItem {
            kind: "running",
            name: pretty(&w.exe, &w.title),
            target: w.exe.clone(),
            windows: vec![w.hwnd],
            active: w.hwnd == fg,
            icon,
        });
    }
    items.extend(groups);

    // Folder stacks sit between the apps and the Recycle Bin.
    for key in stacks {
        let (Some(label), Some(dir)) = (crate::stacks::label(key), crate::stacks::folder(key)) else { continue };
        let path = dir.to_string_lossy().to_string();
        let icon = crate::icons::get(&path);
        if icon.is_none() {
            icon_jobs.push(path);
        }
        items.push(DockItem { kind: "stack", name: label.into(), target: key.clone(), windows: vec![], active: false, icon });
    }

    let trash = "::{645FF040-5081-101B-9F08-08002B2CF60E}".to_string();
    let icon = crate::icons::get(&trash);
    if icon.is_none() {
        icon_jobs.push(trash.clone());
    }
    items.push(DockItem { kind: "trash", name: "Recycle Bin".into(), target: trash, windows: vec![], active: false, icon });

    if !icon_jobs.is_empty() {
        std::thread::spawn(move || crate::icons::fill(icon_jobs));
    }
    DockState { items }
}

pub use imp::{activate, close_windows, cursor, minimize, monitor_at, taskbar_autohide, work_area};

/// One display: its full rectangle and its work area (physical px,
/// left/top/right/bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mon {
    pub monitor: (i32, i32, i32, i32),
    pub work: (i32, i32, i32, i32),
}

impl Mon {
    pub fn centre(&self) -> (i32, i32) {
        ((self.monitor.0 + self.monitor.2) / 2, (self.monitor.1 + self.monitor.3) / 2)
    }
}

/// The main display (the point 0,0 always lies on it).
pub fn primary() -> Option<Mon> {
    monitor_at(0, 0)
}

/// The display the dock sits on, and its scale (x100) when it was placed.
static DOCK_MON: std::sync::Mutex<Option<(Mon, i32)>> = std::sync::Mutex::new(None);

pub fn dock_monitor() -> Option<Mon> {
    DOCK_MON.lock().ok().and_then(|g| g.map(|(m, _)| m))
}

/// Record where the dock should be; true when that changed (move or re-size it).
pub fn set_dock_monitor(m: Mon, scale100: i32) -> bool {
    let mut g = DOCK_MON.lock().unwrap();
    let changed = *g != Some((m, scale100));
    *g = Some((m, scale100));
    changed
}

/// Pointer pushed against the bottom edge of a display for long enough.
pub struct EdgeWatch {
    ticks: u32,
    mon: Option<Mon>,
}

impl EdgeWatch {
    pub const fn new() -> Self {
        EdgeWatch { ticks: 0, mon: None }
    }
    /// Feed one pointer sample; returns the display once the pointer has
    /// rested at its bottom edge for `need` samples in a row.
    pub fn feed(&mut self, y: i32, mon: Option<Mon>, need: u32) -> Option<Mon> {
        match mon {
            Some(m) if y >= m.monitor.3 - 3 => {
                if self.mon.map(|p| p.monitor) == Some(m.monitor) {
                    self.ticks += 1;
                } else {
                    self.ticks = 1;
                    self.mon = Some(m);
                }
            }
            _ => {
                self.ticks = 0;
                self.mon = None;
            }
        }
        if self.ticks >= need {
            self.mon
        } else {
            None
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::Win;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, FindWindowW, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongW,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, SystemParametersInfoW,
        GWL_EXSTYLE, GW_OWNER, SPI_GETWORKAREA, SW_MINIMIZE, SW_RESTORE, WM_CLOSE, WS_EX_TOOLWINDOW,
    };

    fn class(hwnd: HWND) -> String {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    unsafe fn is_real(hwnd: HWND) -> bool {
        if IsWindowVisible(hwnd) == 0 || !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        if (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32) & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as _, &mut cloaked as *mut u32 as *mut c_void, 4);
        if hr == 0 && cloaked != 0 {
            return false;
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) != 0 && (r.right - r.left <= 1 || r.bottom - r.top <= 1) {
            return IsIconic(hwnd) != 0;
        }
        !matches!(class(hwnd).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
    }

    fn exe_of(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = &mut *(lparam as *mut Vec<HWND>);
        if is_real(hwnd) {
            out.push(hwnd);
        }
        1
    }

    pub fn windows() -> Vec<Win> {
        let mut hs: Vec<HWND> = Vec::new();
        unsafe { EnumWindows(Some(cb), &mut hs as *mut Vec<HWND> as LPARAM) };
        let me = unsafe { GetCurrentProcessId() };
        hs.into_iter()
            .filter_map(|h| {
                let mut pid = 0u32;
                unsafe { GetWindowThreadProcessId(h, &mut pid) };
                if pid == me {
                    return None;
                }
                let exe = exe_of(pid)?;
                let n = unsafe { GetWindowTextLengthW(h) };
                let mut buf = vec![0u16; n as usize + 1];
                let got = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
                Some(Win { hwnd: h as isize, exe, title: String::from_utf16_lossy(&buf[..got.max(0) as usize]) })
            })
            .collect()
    }

    pub fn foreground() -> isize {
        unsafe { GetForegroundWindow() as isize }
    }

    fn valid(h: isize) -> Option<HWND> {
        let h = h as HWND;
        (unsafe { is_real(h) }).then_some(h)
    }

    pub fn activate(h: isize) {
        if let Some(h) = valid(h) {
            unsafe {
                if IsIconic(h) != 0 {
                    ShowWindow(h, SW_RESTORE);
                }
                SetForegroundWindow(h);
            }
        }
    }

    pub fn minimize(h: isize) {
        if let Some(h) = valid(h) {
            unsafe { ShowWindow(h, SW_MINIMIZE) };
        }
    }

    pub fn close_windows(hs: &[isize]) {
        for &h in hs {
            if let Some(h) = valid(h) {
                unsafe { PostMessageW(h, WM_CLOSE, 0, 0) };
            }
        }
    }

    pub fn cursor() -> Option<(i32, i32)> {
        let mut p = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
    }

    /// The display containing (or nearest to) a point.
    pub fn monitor_at(x: i32, y: i32) -> Option<super::Mon> {
        unsafe {
            let hm = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hm, &mut mi) == 0 {
                return None;
            }
            let r = |r: RECT| (r.left, r.top, r.right, r.bottom);
            Some(super::Mon { monitor: r(mi.rcMonitor), work: r(mi.rcWork) })
        }
    }

    /// Primary monitor work area (physical px): left, top, right, bottom.
    pub fn work_area() -> Option<(i32, i32, i32, i32)> {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut c_void, 0) };
        (ok != 0).then_some((r.left, r.top, r.right, r.bottom))
    }

    const ABM_GETSTATE: u32 = 4;
    const ABM_SETSTATE: u32 = 10;
    const ABS_AUTOHIDE: isize = 1;
    const ABS_ALWAYSONTOP: isize = 2;

    /// Read (None) or set the taskbar's own "Automatically hide" switch.
    /// Returns the state before the call.
    pub fn taskbar_autohide(set: Option<bool>) -> bool {
        unsafe {
            let cls: Vec<u16> = "Shell_TrayWnd".encode_utf16().chain(std::iter::once(0)).collect();
            let tray = FindWindowW(cls.as_ptr(), std::ptr::null());
            let mut d: APPBARDATA = std::mem::zeroed();
            d.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
            d.hWnd = tray;
            let prev = SHAppBarMessage(ABM_GETSTATE, &mut d) as isize & ABS_AUTOHIDE != 0;
            if let Some(on) = set {
                d.lParam = if on { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP };
                SHAppBarMessage(ABM_SETSTATE, &mut d);
            }
            prev
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Win;
    pub fn cursor() -> Option<(i32, i32)> {
        None
    }
    pub fn monitor_at(_x: i32, _y: i32) -> Option<super::Mon> {
        None
    }
    pub fn windows() -> Vec<Win> {
        Vec::new()
    }
    pub fn foreground() -> isize {
        0
    }
    pub fn activate(_h: isize) {}
    pub fn minimize(_h: isize) {}
    pub fn close_windows(_h: &[isize]) {}
    pub fn work_area() -> Option<(i32, i32, i32, i32)> {
        None
    }
    pub fn taskbar_autohide(_set: Option<bool>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::matches;
    #[test]
    fn edge_watch_needs_a_rest() {
        let m = super::Mon { monitor: (0, 0, 1920, 1080), work: (0, 0, 1920, 1040) };
        let mut e = super::EdgeWatch::new();
        assert!(e.feed(1079, Some(m), 2).is_none());
        assert_eq!(e.feed(1079, Some(m), 2), Some(m));
        assert!(e.feed(500, Some(m), 2).is_none());
    }

    #[test]
    fn matches_pins_to_running_exes() {
        assert!(matches("Chrome", "Google Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"));
        assert!(matches("MSEdge", "Microsoft Edge", r"C:\x\msedge.exe"));
        assert!(matches("Microsoft.Windows.Explorer", "File Explorer", r"C:\Windows\explorer.exe"));
        assert!(matches("Microsoft.VisualStudioCode", "Visual Studio Code", r"C:\x\Code.exe"));
        assert!(!matches("MSEdge", "Microsoft Edge", r"C:\x\notepad.exe"));
    }
}
