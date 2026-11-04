//! Space for Cove's bars, reserved the documented way: each bar registers as
//! an AppBar (SHAppBarMessage), like the Windows taskbar, so maximised
//! windows stop below the menu bar and above the dock instead of sliding
//! under them. Removing the AppBar gives the space back, and Explorer
//! recomputes the work area itself.
//!
//! Every registered bar is also written to a small file next to the
//! settings, so the watchdog can give the space back if Cove crashes.

use std::sync::Mutex;

pub const TOP: u32 = 1; // ABE_TOP
pub const BOTTOM: u32 = 3; // ABE_BOTTOM

/// hwnd -> (edge, reserved rectangle) for every bar Cove holds right now.
static BARS: Mutex<Vec<(isize, u32, (i32, i32, i32, i32))>> = Mutex::new(Vec::new());
static FILE: Mutex<Option<std::path::PathBuf>> = Mutex::new(None);

/// Where the crash record lives (app data dir).
pub fn set_record(path: std::path::PathBuf) {
    if let Ok(mut f) = FILE.lock() {
        *f = Some(path);
    }
}

pub fn record_path(settings: &std::path::Path) -> std::path::PathBuf {
    settings.with_file_name("appbars.txt")
}

fn save() {
    let Some(path) = FILE.lock().ok().and_then(|f| f.clone()) else { return };
    let list: Vec<String> = BARS.lock().map(|b| b.iter().map(|(h, _, _)| h.to_string()).collect()).unwrap_or_default();
    if list.is_empty() {
        let _ = std::fs::remove_file(path);
    } else {
        let _ = std::fs::write(path, list.join("\n"));
    }
}

/// The rectangle a bar of `thickness` px takes on `edge` of `monitor`
/// (physical px, left/top/right/bottom), before Explorer adjusts it.
pub fn proposed(edge: u32, monitor: (i32, i32, i32, i32), thickness: i32) -> (i32, i32, i32, i32) {
    let (l, t, r, b) = monitor;
    if edge == TOP {
        (l, t, r, t + thickness)
    } else {
        (l, b - thickness, r, b)
    }
}

/// Reserve `thickness` px along `edge` of `monitor`. Returns the rectangle
/// Windows gave the bar (it sits inside any other bar on that edge, such as
/// a taskbar that isn't hidden). Re-registering with the same values is a no-op.
pub fn register(hwnd: isize, edge: u32, monitor: (i32, i32, i32, i32), thickness: i32) -> (i32, i32, i32, i32) {
    let want = proposed(edge, monitor, thickness);
    let prev = BARS.lock().ok().and_then(|b| b.iter().find(|(h, _, _)| *h == hwnd).copied());
    if let Some((_, e, rc)) = prev {
        if e == edge && rc_matches(rc, want, edge, thickness) {
            return rc;
        }
    }
    let rc = imp::register(hwnd, edge, want, thickness);
    if let Ok(mut b) = BARS.lock() {
        b.retain(|(h, _, _)| *h != hwnd);
        b.push((hwnd, edge, rc));
    }
    save();
    // The work area changed: windows that are already maximised re-fit it
    // (once Explorer has applied the new work area).
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(200));
        imp::refit_maximized(monitor);
    });
    rc
}

/// Put a bar's window exactly on the rectangle ABM_SETPOS gave it, in
/// screen coordinates, straight through SetWindowPos. Never derived from the
/// work area: once a bar is reserved the work area already excludes it, so
/// "work area top" is the bar's *bottom* (the classic AppBar double offset).
/// Also marks the window as a tool window, whose placement is kept in screen
/// coordinates (a normal window's restore placement is in workspace
/// coordinates, which shift down by the menu bar's height once it reserves
/// the top edge).
pub fn place(hwnd: isize, rc: (i32, i32, i32, i32)) {
    imp::place(hwnd, rc)
}

/// The window's rectangle on screen now (physical px).
pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    imp::window_rect(hwnd)
}

/// The rectangle a maximised window should have on a monitor whose work
/// area is `work`: the work area grown by the window's own invisible frame
/// `frame` on every side (pure: unit-tested). None when it already fits.
pub fn refit_rect(have: (i32, i32, i32, i32), work: (i32, i32, i32, i32)) -> Option<(i32, i32, i32, i32)> {
    let f = (work.0 - have.0).clamp(0, 32);
    let want = (work.0 - f, work.1 - f, work.2 + f, work.3 + f);
    let off = (have.0 - want.0).abs().max((have.1 - want.1).abs()).max((have.2 - want.2).abs()).max((have.3 - want.3).abs());
    (off > 1).then_some(want)
}

/// Same monitor span and thickness: Explorer may have nudged the outer edge
/// (inside a taskbar), which is still the same reservation.
fn rc_matches(have: (i32, i32, i32, i32), want: (i32, i32, i32, i32), edge: u32, thickness: i32) -> bool {
    have.0 == want.0 && have.2 == want.2 && have.3 - have.1 == thickness && (edge != TOP || have.1 == want.1)
}

/// The rectangle a bar holds now, if it's registered.
pub fn held(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
    BARS.lock().ok().and_then(|b| b.iter().find(|(h, _, _)| *h == hwnd).map(|(_, _, rc)| *rc))
}

/// Give the space back. Safe to call when the bar isn't registered.
pub fn unregister(hwnd: isize) {
    imp::unregister(hwnd);
    if let Ok(mut b) = BARS.lock() {
        b.retain(|(h, _, _)| *h != hwnd);
    }
    save();
}

/// Forget a registration (Explorer restarted and dropped it) so the next
/// `register` really registers again.
pub fn forget(hwnd: isize) {
    if let Ok(mut b) = BARS.lock() {
        b.retain(|(h, _, _)| *h != hwnd);
    }
}

/// Watchdog: Cove has ended. Remove every bar it still held, so the work
/// area goes back to exactly what Windows had without Cove.
pub fn release_recorded(settings: &std::path::Path) {
    let path = record_path(settings);
    let Ok(text) = std::fs::read_to_string(&path) else { return };
    for h in text.lines().filter_map(|l| l.trim().parse::<isize>().ok()) {
        imp::unregister(h);
    }
    let _ = std::fs::remove_file(path);
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::{HWND, RECT};
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_USER;

    const ABM_NEW: u32 = 0;
    const ABM_REMOVE: u32 = 1;
    const ABM_QUERYPOS: u32 = 2;
    const ABM_SETPOS: u32 = 3;

    fn data(hwnd: isize) -> APPBARDATA {
        let mut d: APPBARDATA = unsafe { std::mem::zeroed() };
        d.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
        d.hWnd = hwnd as HWND;
        d.uCallbackMessage = WM_USER + 0x43;
        d
    }

    pub fn register(hwnd: isize, edge: u32, want: (i32, i32, i32, i32), thickness: i32) -> (i32, i32, i32, i32) {
        unsafe {
            let mut d = data(hwnd);
            SHAppBarMessage(ABM_REMOVE, &mut d); // clean up any leftover reservation
            SHAppBarMessage(ABM_NEW, &mut d);
            d.uEdge = edge;
            d.rc = RECT { left: want.0, top: want.1, right: want.2, bottom: want.3 };
            SHAppBarMessage(ABM_QUERYPOS, &mut d);
            // Explorer moved the proposed edge clear of other bars; keep our thickness.
            if edge == super::TOP {
                d.rc.bottom = d.rc.top + thickness;
            } else {
                d.rc.top = d.rc.bottom - thickness;
            }
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

    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::Foundation::{BOOL, LPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowLongPtrW, GetWindowRect, IsWindowVisible, IsZoomed, SetWindowLongPtrW, SetWindowPos,
        GWL_EXSTYLE, HWND_TOPMOST, SWP_ASYNCWINDOWPOS, SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_NOZORDER, WS_EX_APPWINDOW,
        WS_EX_TOOLWINDOW,
    };

    pub fn place(hwnd: isize, rc: (i32, i32, i32, i32)) {
        unsafe {
            let h = hwnd as HWND;
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            let want = (ex | WS_EX_TOOLWINDOW as isize) & !(WS_EX_APPWINDOW as isize);
            if want != ex {
                SetWindowLongPtrW(h, GWL_EXSTYLE, want);
            }
            SetWindowPos(h, HWND_TOPMOST, rc.0, rc.1, (rc.2 - rc.0).max(1), (rc.3 - rc.1).max(1), SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
    }

    pub fn window_rect(hwnd: isize) -> Option<(i32, i32, i32, i32)> {
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        (unsafe { GetWindowRect(hwnd as HWND, &mut r) } != 0).then_some((r.left, r.top, r.right, r.bottom))
    }

    fn mon_info(h: HWND) -> Option<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
        unsafe {
            let hm = MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hm, &mut mi) == 0 {
                return None;
            }
            let r = |r: RECT| (r.left, r.top, r.right, r.bottom);
            Some((r(mi.rcMonitor), r(mi.rcWork)))
        }
    }

    unsafe extern "system" fn collect(h: HWND, l: LPARAM) -> BOOL {
        let v = &mut *(l as *mut Vec<HWND>);
        if IsWindowVisible(h) != 0 && IsZoomed(h) != 0 {
            v.push(h);
        }
        1
    }

    /// Re-fit every maximised window on `monitor` to its current work area.
    /// Asynchronous moves only, so a hung app can't stall Cove.
    pub fn refit_maximized(monitor: (i32, i32, i32, i32)) {
        let mut hs: Vec<HWND> = Vec::new();
        unsafe { EnumWindows(Some(collect), &mut hs as *mut Vec<HWND> as LPARAM) };
        for h in hs {
            let Some((mon, work)) = mon_info(h) else { continue };
            if mon != monitor {
                continue;
            }
            let Some(have) = window_rect(h as isize) else { continue };
            if let Some(r) = super::refit_rect(have, work) {
                unsafe {
                    SetWindowPos(h, std::ptr::null_mut(), r.0, r.1, r.2 - r.0, r.3 - r.1, SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_ASYNCWINDOWPOS);
                }
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn register(_h: isize, _edge: u32, want: (i32, i32, i32, i32), _t: i32) -> (i32, i32, i32, i32) {
        want
    }
    pub fn unregister(_h: isize) {}
    pub fn place(_h: isize, _rc: (i32, i32, i32, i32)) {}
    pub fn window_rect(_h: isize) -> Option<(i32, i32, i32, i32)> {
        None
    }
    pub fn refit_maximized(_m: (i32, i32, i32, i32)) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proposes_strips_on_the_edge() {
        let m = (0, 0, 2880, 1800);
        assert_eq!(proposed(TOP, m, 38), (0, 0, 2880, 38));
        assert_eq!(proposed(BOTTOM, m, 111), (0, 1689, 2880, 1800));
        // A second display to the left of the main one.
        assert_eq!(proposed(BOTTOM, (-1920, 0, 0, 1080), 74), (-1920, 1006, 0, 1080));
    }
    #[test]
    fn maximised_windows_refit_the_work_area() {
        // Maximised under the old work area (no menu bar), 8 px frame.
        let work = (0, 38, 1920, 1080);
        assert_eq!(refit_rect((-8, -8, 1928, 1088), work), Some((-8, 30, 1928, 1088)));
        // Already fits (with or without an invisible frame).
        assert_eq!(refit_rect((-8, 30, 1928, 1088), work), None);
        assert_eq!(refit_rect((0, 38, 1920, 1080), work), None);
    }
    #[test]
    fn same_reservation_is_recognised() {
        // Sitting above a visible 48 px taskbar is still "the same" bar.
        assert!(rc_matches((0, 958, 1920, 1032), (0, 1006, 1920, 1080), BOTTOM, 74));
        assert!(!rc_matches((0, 958, 1920, 1030), (0, 1006, 1920, 1080), BOTTOM, 74));
        assert!(!rc_matches((0, 0, 1920, 38), (0, 0, 1920, 38), TOP, 30));
    }
}
