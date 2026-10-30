//! Window snapping presets (opt-in), Rectangle-style, with Ctrl+Alt keys.
//! Documented Win32 only: RegisterHotKey + SetWindowPos on the window in
//! front. Moves glide over ~200 ms with an ease-out curve; a new snap or a
//! mouse press interrupts the glide at once. Nothing is injected.
//!
//!   Ctrl+Alt+← → ↑ ↓   left / right / top / bottom half (press again: 2/3, 1/3)
//!   Ctrl+Alt+U I J K   top-left / top-right / bottom-left / bottom-right quarter
//!   Ctrl+Alt+D F G     left / centre / right third
//!   Ctrl+Alt+E T       left / right two-thirds
//!   Ctrl+Alt+Enter     fill the screen     Ctrl+Alt+C  centre
//!   Ctrl+Alt+Backspace back to where it was before snapping
//!   Ctrl+Alt+Shift+← → move to the previous / next display

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset {
    Left,
    Right,
    Top,
    Bottom,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    FirstThird,
    CenterThird,
    LastThird,
    FirstTwoThirds,
    LastTwoThirds,
    Fill,
    Center,
    Restore,
    PrevDisplay,
    NextDisplay,
}

pub struct Binding {
    pub id: i32,
    pub label: &'static str,
    pub shift: bool,
    pub key: u32,
    pub preset: Preset,
}

pub static BINDINGS: &[Binding] = &[
    Binding { id: 101, label: "Ctrl+Alt+←", shift: false, key: 0x25, preset: Preset::Left },
    Binding { id: 102, label: "Ctrl+Alt+→", shift: false, key: 0x27, preset: Preset::Right },
    Binding { id: 103, label: "Ctrl+Alt+↑", shift: false, key: 0x26, preset: Preset::Top },
    Binding { id: 104, label: "Ctrl+Alt+↓", shift: false, key: 0x28, preset: Preset::Bottom },
    Binding { id: 105, label: "Ctrl+Alt+U", shift: false, key: b'U' as u32, preset: Preset::TopLeft },
    Binding { id: 106, label: "Ctrl+Alt+I", shift: false, key: b'I' as u32, preset: Preset::TopRight },
    Binding { id: 107, label: "Ctrl+Alt+J", shift: false, key: b'J' as u32, preset: Preset::BottomLeft },
    Binding { id: 108, label: "Ctrl+Alt+K", shift: false, key: b'K' as u32, preset: Preset::BottomRight },
    Binding { id: 109, label: "Ctrl+Alt+D", shift: false, key: b'D' as u32, preset: Preset::FirstThird },
    Binding { id: 110, label: "Ctrl+Alt+F", shift: false, key: b'F' as u32, preset: Preset::CenterThird },
    Binding { id: 111, label: "Ctrl+Alt+G", shift: false, key: b'G' as u32, preset: Preset::LastThird },
    Binding { id: 112, label: "Ctrl+Alt+E", shift: false, key: b'E' as u32, preset: Preset::FirstTwoThirds },
    Binding { id: 113, label: "Ctrl+Alt+T", shift: false, key: b'T' as u32, preset: Preset::LastTwoThirds },
    Binding { id: 114, label: "Ctrl+Alt+Enter", shift: false, key: 0x0D, preset: Preset::Fill },
    Binding { id: 115, label: "Ctrl+Alt+C", shift: false, key: b'C' as u32, preset: Preset::Center },
    Binding { id: 116, label: "Ctrl+Alt+Backspace", shift: false, key: 0x08, preset: Preset::Restore },
    Binding { id: 117, label: "Ctrl+Alt+Shift+←", shift: true, key: 0x25, preset: Preset::PrevDisplay },
    Binding { id: 118, label: "Ctrl+Alt+Shift+→", shift: true, key: 0x27, preset: Preset::NextDisplay },
];

#[derive(Debug, Clone, Serialize, Default)]
pub struct SnapStatus {
    pub running: bool,
    /// Shortcuts another app already owns.
    pub conflicts: Vec<&'static str>,
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct R {
    pub l: i32,
    pub t: i32,
    pub r: i32,
    pub b: i32,
}

impl R {
    pub fn w(&self) -> i32 {
        self.r - self.l
    }
    pub fn h(&self) -> i32 {
        self.b - self.t
    }
    pub fn near(&self, o: &R, tol: i32) -> bool {
        (self.l - o.l).abs() <= tol && (self.t - o.t).abs() <= tol && (self.r - o.r).abs() <= tol && (self.b - o.b).abs() <= tol
    }
}

/// Halves cycle through these widths when pressed again.
const CYCLE: [(i32, i32); 3] = [(1, 2), (2, 3), (1, 3)];

/// Where a preset puts a window, in the visible-frame space of `work`.
/// `cur` is the window's current visible frame; `step` picks the cycle size for halves.
pub fn target(p: Preset, work: R, cur: R, step: usize) -> Option<R> {
    let (x, y, w, h) = (work.l, work.t, work.w(), work.h());
    let (n, d) = CYCLE[step % CYCLE.len()];
    let part = |num: i32, den: i32, len: i32| len * num / den;
    Some(match p {
        Preset::Left => R { l: x, t: y, r: x + part(n, d, w), b: work.b },
        Preset::Right => R { l: work.r - part(n, d, w), t: y, r: work.r, b: work.b },
        Preset::Top => R { l: x, t: y, r: work.r, b: y + part(n, d, h) },
        Preset::Bottom => R { l: x, t: work.b - part(n, d, h), r: work.r, b: work.b },
        Preset::TopLeft => R { l: x, t: y, r: x + w / 2, b: y + h / 2 },
        Preset::TopRight => R { l: x + w / 2, t: y, r: work.r, b: y + h / 2 },
        Preset::BottomLeft => R { l: x, t: y + h / 2, r: x + w / 2, b: work.b },
        Preset::BottomRight => R { l: x + w / 2, t: y + h / 2, r: work.r, b: work.b },
        Preset::FirstThird => R { l: x, t: y, r: x + w / 3, b: work.b },
        Preset::CenterThird => R { l: x + w / 3, t: y, r: x + 2 * w / 3, b: work.b },
        Preset::LastThird => R { l: x + 2 * w / 3, t: y, r: work.r, b: work.b },
        Preset::FirstTwoThirds => R { l: x, t: y, r: x + 2 * w / 3, b: work.b },
        Preset::LastTwoThirds => R { l: x + w / 3, t: y, r: work.r, b: work.b },
        Preset::Fill => work,
        Preset::Center => {
            let (cw, ch) = (cur.w().min(w), cur.h().min(h));
            let (l, t) = (x + (w - cw) / 2, y + (h - ch) / 2);
            R { l, t, r: l + cw, b: t + ch }
        }
        Preset::Restore | Preset::PrevDisplay | Preset::NextDisplay => return None,
    })
}

/// Keep a window's place and size proportionally when moving it to another display.
pub fn remap(cur: R, from: R, to: R) -> R {
    let sx = |v: i32| to.l + ((v - from.l) as i64 * to.w() as i64 / from.w().max(1) as i64) as i32;
    let sy = |v: i32| to.t + ((v - from.t) as i64 * to.h() as i64 / from.h().max(1) as i64) as i32;
    let mut r = R { l: sx(cur.l), t: sy(cur.t), r: sx(cur.r), b: sy(cur.b) };
    // Stay fully on the new display.
    if r.r > to.r {
        let d = r.r - to.r;
        r.l -= d;
        r.r -= d;
    }
    if r.b > to.b {
        let d = r.b - to.b;
        r.t -= d;
        r.b -= d;
    }
    r.l = r.l.max(to.l);
    r.t = r.t.max(to.t);
    r
}

/// Ease-out cubic: quick start, soft landing.
pub fn ease(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t).powi(3)
}

pub fn lerp(a: R, b: R, t: f64) -> R {
    let f = |x: i32, y: i32| x + ((y - x) as f64 * t).round() as i32;
    R { l: f(a.l, b.l), t: f(a.t, b.t), r: f(a.r, b.r), b: f(a.b, b.b) }
}

pub const GLIDE_MS: u64 = 200;

#[cfg(windows)]
mod imp {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
    use std::sync::{mpsc, Mutex};
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Dwm::{DwmFlush, DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
    use windows_sys::Win32::Graphics::Gdi::{
        EnumDisplayMonitors, GetMonitorInfoW, MonitorFromWindow, HDC, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST,
    };
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetClassNameW, GetForegroundWindow, GetMessageW, GetWindowRect, IsWindow, IsZoomed, PostThreadMessageW,
        SetWindowPos, ShowWindow, SystemParametersInfoW, MSG, SPI_GETCLIENTAREAANIMATION, SWP_NOACTIVATE,
        SWP_NOOWNERZORDER, SWP_NOZORDER, SW_RESTORE, WM_HOTKEY, WM_QUIT,
    };

    struct Running {
        tid: u32,
        status: SnapStatus,
    }
    static THREAD: Mutex<Option<Running>> = Mutex::new(None);
    static ANIMATE: AtomicBool = AtomicBool::new(true);
    /// Bumped on every snap; a glide stops when it is no longer the newest.
    static GEN: AtomicU64 = AtomicU64::new(0);
    /// Where each window was before Cove first snapped it.
    static ORIGINAL: Mutex<Option<HashMap<isize, R>>> = Mutex::new(None);
    /// Last snap: window, preset, cycle step, the frame Cove set.
    static LAST: Mutex<Option<(isize, Preset, usize, R)>> = Mutex::new(None);

    fn from(r: RECT) -> R {
        R { l: r.left, t: r.top, r: r.right, b: r.bottom }
    }

    fn is_shell(h: HWND) -> bool {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        matches!(
            String::from_utf16_lossy(&buf[..n.max(0) as usize]).as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        )
    }

    fn window_rect(h: HWND) -> R {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetWindowRect(h, &mut r) };
        from(r)
    }

    /// The visible frame (without Windows 10/11's invisible resize borders).
    fn frame(h: HWND) -> R {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        let hr = unsafe {
            DwmGetWindowAttribute(
                h,
                DWMWA_EXTENDED_FRAME_BOUNDS as _,
                &mut r as *mut RECT as *mut c_void,
                std::mem::size_of::<RECT>() as u32,
            )
        };
        if hr == 0 {
            from(r)
        } else {
            window_rect(h)
        }
    }

    /// Visible frame -> the rect SetWindowPos needs, adding back the invisible borders.
    fn outer(h: HWND, vis: R) -> R {
        let (w, f) = (window_rect(h), frame(h));
        R { l: vis.l - (f.l - w.l), t: vis.t - (f.t - w.t), r: vis.r + (w.r - f.r), b: vis.b + (w.b - f.b) }
    }

    fn mon_info(m: HMONITOR) -> (R, R) {
        let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        unsafe { GetMonitorInfoW(m, &mut mi) };
        (from(mi.rcMonitor), from(mi.rcWork))
    }

    unsafe extern "system" fn mon_cb(m: HMONITOR, _: HDC, _: *mut RECT, lp: LPARAM) -> BOOL {
        (*(lp as *mut Vec<HMONITOR>)).push(m);
        1
    }

    /// Work areas of all displays, left to right.
    fn work_areas() -> Vec<R> {
        let mut ms: Vec<HMONITOR> = Vec::new();
        unsafe { EnumDisplayMonitors(std::ptr::null_mut(), std::ptr::null(), Some(mon_cb), &mut ms as *mut _ as LPARAM) };
        let mut v: Vec<(R, R)> = ms.into_iter().map(mon_info).collect();
        v.sort_by_key(|(m, _)| (m.l, m.t));
        v.into_iter().map(|(_, w)| w).collect()
    }

    fn system_animations() -> bool {
        let mut on: BOOL = 1;
        unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut on as *mut BOOL as *mut c_void, 0) };
        on != 0
    }

    fn place(h: HWND, r: R) {
        unsafe {
            SetWindowPos(h, std::ptr::null_mut(), r.l, r.t, r.w(), r.h(), SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
    }

    /// Glide window `h` to outer rect `to`. Interrupted by a newer snap or a mouse press.
    fn glide(h: HWND, to: R) {
        let gen = GEN.fetch_add(1, Ordering::SeqCst) + 1;
        let start = window_rect(h);
        if !(ANIMATE.load(Ordering::SeqCst) && system_animations()) || start == to {
            place(h, to);
            return;
        }
        let hw = h as isize;
        std::thread::spawn(move || {
            let h = hw as HWND;
            let t0 = Instant::now();
            loop {
                if GEN.load(Ordering::SeqCst) != gen || unsafe { IsWindow(h) } == 0 {
                    return; // a newer snap took over
                }
                // VK_LBUTTON: the user grabbed something; let go of the window.
                if unsafe { GetAsyncKeyState(0x01) } as u16 & 0x8000 != 0 {
                    return;
                }
                let t = t0.elapsed().as_millis() as f64 / GLIDE_MS as f64;
                if t >= 1.0 {
                    place(h, to);
                    return;
                }
                place(h, lerp(start, to, ease(t)));
                if unsafe { DwmFlush() } != 0 {
                    std::thread::sleep(Duration::from_millis(8));
                }
            }
        });
    }

    fn run(p: Preset) {
        let h = unsafe { GetForegroundWindow() };
        if h.is_null() || is_shell(h) {
            return;
        }
        crate::tour_signal("snap");
        let key = h as isize;
        if unsafe { IsZoomed(h) } != 0 {
            // Un-maximize without a visible jump, then glide from the full-screen frame.
            let full = window_rect(h);
            unsafe { ShowWindow(h, SW_RESTORE) };
            place(h, full);
        }
        let cur = frame(h);
        let (_, work) = mon_info(unsafe { MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST) });

        if p == Preset::Restore {
            let orig = ORIGINAL.lock().unwrap().as_mut().and_then(|m| m.remove(&key));
            if let Some(o) = orig {
                *LAST.lock().unwrap() = None;
                glide(h, outer(h, o));
            }
            return;
        }

        let vis = if p == Preset::PrevDisplay || p == Preset::NextDisplay {
            let areas = work_areas();
            let Some(i) = areas.iter().position(|a| *a == work) else { return };
            if areas.len() < 2 {
                return;
            }
            let j = if p == Preset::NextDisplay { (i + 1) % areas.len() } else { (i + areas.len() - 1) % areas.len() };
            remap(cur, work, areas[j])
        } else {
            let mut last = LAST.lock().unwrap();
            let step = match *last {
                Some((lh, lp, ls, lr)) if lh == key && lp == p && lr.near(&cur, 12) => ls + 1,
                _ => 0,
            };
            let Some(t) = target(p, work, cur, step) else { return };
            *last = Some((key, p, step, t));
            t
        };

        ORIGINAL.lock().unwrap().get_or_insert_with(HashMap::new).entry(key).or_insert(cur);
        glide(h, outer(h, vis));
    }

    fn stop_thread() {
        if let Some(r) = THREAD.lock().unwrap().take() {
            unsafe { PostThreadMessageW(r.tid, WM_QUIT, 0, 0) };
        }
    }

    pub fn configure(enabled: bool, animate: bool) -> SnapStatus {
        ANIMATE.store(animate, Ordering::SeqCst);
        stop_thread();
        if !enabled {
            return SnapStatus::default();
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let null: HWND = std::ptr::null_mut();
            let mut conflicts = Vec::new();
            let mut ids = Vec::new();
            for b in BINDINGS {
                let mods = MOD_CONTROL | MOD_ALT | MOD_NOREPEAT | (if b.shift { MOD_SHIFT } else { 0 });
                if unsafe { RegisterHotKey(null, b.id, mods, b.key) } == 0 {
                    conflicts.push(b.label);
                } else {
                    ids.push(b.id);
                }
            }
            let _ = tx.send((unsafe { GetCurrentThreadId() }, conflicts));
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, null, 0, 0) } > 0 {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam as i32;
                    if let Some(b) = BINDINGS.iter().find(|b| b.id == id) {
                        run(b.preset);
                    }
                }
            }
            for id in ids {
                unsafe { UnregisterHotKey(null, id) };
            }
        });
        let (tid, conflicts) = rx.recv().unwrap_or((0, Vec::new()));
        let status = SnapStatus { running: true, conflicts };
        *THREAD.lock().unwrap() = Some(Running { tid, status: status.clone() });
        status
    }

    pub fn status() -> SnapStatus {
        THREAD.lock().unwrap().as_ref().map(|r| r.status.clone()).unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::SnapStatus;
    pub fn configure(_enabled: bool, _animate: bool) -> SnapStatus {
        SnapStatus::default()
    }
    pub fn status() -> SnapStatus {
        SnapStatus::default()
    }
}

pub use imp::{configure, status};

#[cfg(test)]
mod tests {
    use super::*;
    const W: R = R { l: 0, t: 40, r: 1920, b: 1080 };

    #[test]
    fn halves_cycle() {
        let c = R { l: 100, t: 100, r: 900, b: 700 };
        assert_eq!(target(Preset::Left, W, c, 0).unwrap().r, 960);
        assert_eq!(target(Preset::Left, W, c, 1).unwrap().r, 1280);
        assert_eq!(target(Preset::Left, W, c, 2).unwrap().r, 640);
        assert_eq!(target(Preset::Right, W, c, 0).unwrap().l, 960);
        assert_eq!(target(Preset::Bottom, W, c, 0).unwrap().t, 560);
    }

    #[test]
    fn quarters_and_thirds_tile() {
        let c = R::default();
        let tl = target(Preset::TopLeft, W, c, 0).unwrap();
        let br = target(Preset::BottomRight, W, c, 0).unwrap();
        assert_eq!((tl.r, tl.b), (br.l, br.t));
        let a = target(Preset::FirstThird, W, c, 0).unwrap();
        let b = target(Preset::CenterThird, W, c, 0).unwrap();
        let z = target(Preset::LastThird, W, c, 0).unwrap();
        assert_eq!((a.r, b.r, z.r), (b.l, z.l, 1920));
    }

    #[test]
    fn center_keeps_size() {
        let c = R { l: 0, t: 0, r: 800, b: 600 };
        let t = target(Preset::Center, W, c, 0).unwrap();
        assert_eq!((t.w(), t.h()), (800, 600));
        assert_eq!(t.l, 560);
    }

    #[test]
    fn remap_stays_on_screen() {
        let to = R { l: 1920, t: 0, r: 3200, b: 1024 };
        let r = remap(R { l: 960, t: 40, r: 1920, b: 1080 }, W, to);
        assert!(r.l >= to.l && r.r <= to.r && r.t >= to.t && r.b <= to.b);
    }

    #[test]
    fn glide_ends_on_target() {
        let a = R { l: 0, t: 0, r: 100, b: 100 };
        let b = R { l: 500, t: 300, r: 900, b: 800 };
        assert_eq!(lerp(a, b, ease(1.0)), b);
        assert_eq!(lerp(a, b, ease(0.0)), a);
        assert!(ease(0.5) > 0.5);
    }
}
