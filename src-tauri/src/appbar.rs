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
    rc
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
}

#[cfg(not(windows))]
mod imp {
    pub fn register(_h: isize, _edge: u32, want: (i32, i32, i32, i32), _t: i32) -> (i32, i32, i32, i32) {
        want
    }
    pub fn unregister(_h: isize) {}
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
    fn same_reservation_is_recognised() {
        // Sitting above a visible 48 px taskbar is still "the same" bar.
        assert!(rc_matches((0, 958, 1920, 1032), (0, 1006, 1920, 1080), BOTTOM, 74));
        assert!(!rc_matches((0, 958, 1920, 1030), (0, 1006, 1920, 1080), BOTTOM, 74));
        assert!(!rc_matches((0, 0, 1920, 38), (0, 0, 1920, 38), TOP, 30));
    }
}
