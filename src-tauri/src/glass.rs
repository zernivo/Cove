//! Window-level glass helpers: Liquid Glass backdrops under the dock, the
//! popovers and Search (see `backdrop`), rounded window regions (DWM itself
//! only rounds to 8 px; Cove's menus are 11, Control Centre 20, Search 14 DIP),
//! plus two small desktop actions the bar's menus use. Documented Win32
//! only: no injection, nothing patched.

/// Liquid Glass backdrops (2026-11-05, after the user's test of b44d0c1).
///
/// The old route, `window_vibrancy::apply_acrylic` on the popover window,
/// uses DWM's system backdrop on Windows 11, and Windows draws a system
/// backdrop on an INACTIVE window as a flat fallback colour: the grey slab
/// with grey square corners the user saw. The dock's wallpaper plate could
/// read the wrong picture (the saturated blue).
///
/// Now every glass surface gets a backdrop: a small native window of
/// Cove's own (no WebView), shaped by a rounded window region to exactly
/// the panels shown, kept directly under its Cove window, with
/// `SetWindowCompositionAttribute(ACCENT_ENABLE_ACRYLICBLURBEHIND)` and a
/// neutral, low-alpha tint. That accent blurs what is behind whether or not
/// the window is active (the TranslucentTB / ExplorerBlurMica route). The
/// Cove window above stays fully transparent and unshaped, so its CSS
/// paints the anti-aliased corners, the tint, the specular rim and a soft
/// drop shadow outside the panel; the region sits 1 px inside the CSS
/// edge so its stair-stepped edge is always under the painted rim.
/// Documented Win32 plus the long-standing (undocumented but stable since
/// Windows 10 1803) composition attribute; nothing injected, nothing hooked
/// in other processes: the WinEvent hook is for Cove's own process only.
pub type Shape = (i32, i32, i32, i32, i32);

static LOGGER: std::sync::OnceLock<Box<dyn Fn(&str) + Send + Sync>> = std::sync::OnceLock::new();
pub fn set_logger(f: impl Fn(&str) + Send + Sync + 'static) {
    let _ = LOGGER.set(Box::new(f));
}
#[allow(dead_code)]
fn log(m: &str) {
    if let Some(f) = LOGGER.get() {
        f(m);
    }
}

#[cfg(windows)]
pub mod backdrop {
    use super::Shape;
    use std::sync::Mutex;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
    use windows_sys::Win32::Graphics::Dwm::{DwmEnableBlurBehindWindow, DwmSetWindowAttribute, DWM_BB_BLURREGION, DWM_BB_ENABLE, DWM_BLURBEHIND};
    use windows_sys::Win32::Graphics::Gdi::{DeleteObject, FillRect, GetStockObject, BLACK_BRUSH, HBRUSH, HDC};
    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress, LoadLibraryW};
    use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, GetClientRect, GetWindowRect, IsWindow, IsWindowVisible, RegisterClassW, SetWindowPos,
        ShowWindow, EVENT_OBJECT_HIDE, EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_SHOW, HTTRANSPARENT, MA_NOACTIVATE, OBJID_WINDOW,
        SWP_NOACTIVATE, SWP_NOOWNERZORDER, SWP_SHOWWINDOW, SW_HIDE, WINEVENT_OUTOFCONTEXT, WM_ERASEBKGND, WM_MOUSEACTIVATE,
        WM_NCHITTEST, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    };

    struct Bd {
        owner: isize,
        hwnd: isize,
        shapes: Vec<Shape>,
        /// Region last applied (shapes + window size), to skip repeats.
        applied: Option<(Vec<Shape>, i32, i32)>,
    }

    static LIST: Mutex<Vec<Bd>> = Mutex::new(Vec::new());
    static DARK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
    static HOOKED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    unsafe extern "system" fn proc_(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            // Never takes the pointer or the focus: the Cove window above does.
            WM_NCHITTEST => HTTRANSPARENT as LRESULT,
            WM_MOUSEACTIVATE => MA_NOACTIVATE as LRESULT,
            // GDI black has zero alpha: the accent's blur shows through.
            WM_ERASEBKGND => {
                let mut rc: RECT = unsafe { std::mem::zeroed() };
                unsafe {
                    GetClientRect(h, &mut rc);
                    FillRect(wp as HDC, &rc, GetStockObject(BLACK_BRUSH) as HBRUSH);
                }
                1
            }
            _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
        }
    }

    #[repr(C)]
    struct AccentPolicy {
        state: u32,
        flags: u32,
        gradient: u32,
        animation: u32,
    }
    #[repr(C)]
    struct CompositionData {
        attrib: u32,
        data: *mut core::ffi::c_void,
        size: usize,
    }

    /// The neutral tint baked into the blur (AABBGGRR). Low alpha so the
    /// glass stays clear; the page adds the rest of the tint.
    fn gradient(dark: bool) -> u32 {
        if dark {
            0x30_1A_19_18
        } else {
            0x38_F4_F4_F6
        }
    }

    /// ACCENT_ENABLE_ACRYLICBLURBEHIND (4); falls back to plain blur (3)
    /// where acrylic isn't offered (Windows 10 before 1803).
    fn accent(h: HWND, dark: bool) -> bool {
        type Swca = unsafe extern "system" fn(HWND, *mut CompositionData) -> i32;
        static F: std::sync::OnceLock<Option<Swca>> = std::sync::OnceLock::new();
        let f = F.get_or_init(|| unsafe {
            let m = LoadLibraryW(wide("user32.dll").as_ptr());
            if m.is_null() {
                return None;
            }
            GetProcAddress(m, b"SetWindowCompositionAttribute\0".as_ptr()).map(|p| std::mem::transmute::<_, Swca>(p))
        });
        let Some(f) = f else { return false };
        for state in [4u32, 3] {
            let mut ap = AccentPolicy { state, flags: 0, gradient: gradient(dark), animation: 0 };
            let mut d = CompositionData { attrib: 19, data: &mut ap as *mut _ as *mut _, size: std::mem::size_of::<AccentPolicy>() };
            if unsafe { f(h, &mut d) } != 0 {
                return true;
            }
        }
        false
    }

    fn dwm_u32(h: HWND, attr: u32, v: u32) {
        unsafe { DwmSetWindowAttribute(h, attr as _, &v as *const u32 as *const _, 4) };
    }

    fn create() -> isize {
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let class = wide("CoveGlassBackdrop");
            static REG: std::sync::Once = std::sync::Once::new();
            REG.call_once(|| {
                let mut wc: WNDCLASSW = std::mem::zeroed();
                wc.lpfnWndProc = Some(proc_);
                wc.hInstance = inst;
                wc.hbrBackground = GetStockObject(BLACK_BRUSH) as HBRUSH;
                wc.lpszClassName = class.as_ptr();
                RegisterClassW(&wc);
            });
            let h = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_TOPMOST,
                class.as_ptr(),
                wide("Cove glass").as_ptr(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                inst,
                std::ptr::null(),
            );
            if h.is_null() {
                return 0;
            }
            // No DWM rounding (the region shapes it), no 1 px border, no
            // fade on show/hide (it must appear and vanish with its window).
            dwm_u32(h, 33, 1); // DWMWA_WINDOW_CORNER_PREFERENCE = DONOTROUND
            dwm_u32(h, 34, 0xFFFF_FFFE); // DWMWA_BORDER_COLOR = NONE
            dwm_u32(h, 3, 1); // DWMWA_TRANSITIONS_FORCEDISABLED
            let ok = accent(h, DARK.load(std::sync::atomic::Ordering::SeqCst));
            super::log(&format!("glass: backdrop window created (blur-behind {})", if ok { "on" } else { "unavailable" }));
            h as isize
        }
    }

    unsafe extern "system" fn on_event(_: HWINEVENTHOOK, _ev: u32, hwnd: HWND, id_object: i32, id_child: i32, _: u32, _: u32) {
        if id_object != OBJID_WINDOW || id_child != 0 {
            return;
        }
        sync_owner(hwnd as isize);
    }

    /// Follow Cove's own windows (shown, hidden, moved). Out-of-context and
    /// for this process only: nothing is loaded anywhere else.
    fn hook() {
        if HOOKED.swap(true, std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        unsafe {
            let pid = windows_sys::Win32::System::Threading::GetCurrentProcessId();
            SetWinEventHook(EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE, std::ptr::null_mut(), Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT);
            SetWinEventHook(EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_LOCATIONCHANGE, std::ptr::null_mut(), Some(on_event), pid, 0, WINEVENT_OUTOFCONTEXT);
        }
    }

    fn sync(b: &mut Bd) {
        let (owner, h) = (b.owner as HWND, b.hwnd as HWND);
        unsafe {
            if IsWindow(owner) == 0 || IsWindowVisible(owner) == 0 || b.shapes.is_empty() {
                if IsWindowVisible(h) != 0 {
                    ShowWindow(h, SW_HIDE);
                }
                return;
            }
            let mut rc: RECT = std::mem::zeroed();
            if GetWindowRect(owner, &mut rc) == 0 {
                return;
            }
            // 1 px inside the painted edge (see the module note).
            let inset: Vec<Shape> = b.shapes.iter().map(|&(x, y, sw, sh, r)| (x + 1, y + 1, sw - 2, sh - 2, (r - 1).max(0))).collect();
            // The backdrop window is only as big as the panels' bounding
            // box: nothing of it can ever reach outside them.
            let (l, t) = inset.iter().fold((i32::MAX, i32::MAX), |(l, t), s| (l.min(s.0), t.min(s.1)));
            let (r, bm) = inset.iter().fold((i32::MIN, i32::MIN), |(r, bm), s| (r.max(s.0 + s.2), bm.max(s.1 + s.3)));
            let (w, hh) = (r - l, bm - t);
            if w <= 0 || hh <= 0 {
                return;
            }
            let local: Vec<Shape> = inset.iter().map(|&(x, y, sw, sh, rr)| (x - l, y - t, sw, sh, rr)).collect();
            let key = (local.clone(), w, hh);
            if b.applied.as_ref() != Some(&key) {
                // The window region shapes the window; DWM's blur-behind
                // region shapes the accent's blur and tint (an accent
                // ignores the window region on its own).
                super::round_region(b.hwnd, &local);
                let rgn = super::make_region(&local);
                if !rgn.is_null() {
                    let bb = DWM_BLURBEHIND { dwFlags: DWM_BB_ENABLE | DWM_BB_BLURREGION, fEnable: 1, hRgnBlur: rgn, fTransitionOnMaximized: 0 };
                    DwmEnableBlurBehindWindow(h, &bb);
                    DeleteObject(rgn);
                }
                b.applied = Some(key);
            }
            // Directly under its window.
            SetWindowPos(h, owner, rc.left + l, rc.top + t, w, hh, SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW);
        }
    }

    fn sync_owner(owner: isize) {
        // try_lock: an event delivered while `set` holds the list is
        // covered by `set`'s own sync.
        let Ok(mut l) = LIST.try_lock() else { return };
        if let Some(b) = l.iter_mut().find(|b| b.owner == owner) {
            sync(b);
        }
    }

    /// Glass under `owner` shaped to `shapes` (physical px, owner window
    /// coordinates); empty hides it. Main thread only.
    pub fn set(owner: isize, shapes: &[Shape]) {
        if owner == 0 {
            return;
        }
        hook();
        let Ok(mut l) = LIST.lock() else { return };
        if !l.iter().any(|b| b.owner == owner) {
            if shapes.is_empty() {
                return;
            }
            let h = create();
            if h == 0 {
                return;
            }
            l.push(Bd { owner, hwnd: h, shapes: Vec::new(), applied: None });
        }
        if let Some(b) = l.iter_mut().find(|b| b.owner == owner) {
            b.shapes = shapes.iter().copied().filter(|s| s.2 > 2 && s.3 > 2).collect();
            sync(b);
        }
    }

    /// Light or dark tint for every backdrop.
    pub fn tint(dark: bool) {
        DARK.store(dark, std::sync::atomic::Ordering::SeqCst);
        let Ok(l) = LIST.lock() else { return };
        for b in l.iter() {
            accent(b.hwnd as HWND, dark);
        }
    }
}

#[cfg(not(windows))]
pub mod backdrop {
    use super::Shape;
    pub fn set(_owner: isize, _shapes: &[Shape]) {}
    pub fn tint(_dark: bool) {}
}

/// Turn DWM's own corner rounding off (the window region does it instead).
#[cfg(windows)]
pub fn no_dwm_rounding(w: &tauri::WebviewWindow) {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_WINDOW_CORNER_PREFERENCE};
    if let Ok(hwnd) = w.hwnd() {
        let pref: u32 = 1; // DWMWCP_DONOTROUND
        unsafe {
            DwmSetWindowAttribute(hwnd.0 as _, DWMWA_WINDOW_CORNER_PREFERENCE as _, &pref as *const u32 as *const _, 4);
        }
    }
}
#[cfg(not(windows))]
pub fn no_dwm_rounding(_w: &tauri::WebviewWindow) {}

/// The union of rounded rectangles (x, y, w, h, radius; physical px) as a
/// GDI region the caller owns (null if empty).
#[cfg(windows)]
pub fn make_region(shapes: &[(i32, i32, i32, i32, i32)]) -> windows_sys::Win32::Graphics::Gdi::HRGN {
    use windows_sys::Win32::Graphics::Gdi::{CombineRgn, CreateRoundRectRgn, DeleteObject, HRGN, RGN_OR};
    let mut acc: HRGN = std::ptr::null_mut();
    unsafe {
        for &(x, y, w, h, r) in shapes {
            if w <= 0 || h <= 0 {
                continue;
            }
            let rg = CreateRoundRectRgn(x, y, x + w + 1, y + h + 1, 2 * r, 2 * r);
            if rg.is_null() {
                continue;
            }
            if acc.is_null() {
                acc = rg;
            } else {
                CombineRgn(acc, acc, rg, RGN_OR);
                DeleteObject(rg);
            }
        }
    }
    acc
}

/// Shape a window to the union of rounded rectangles (x, y, w, h, radius;
/// physical px, window coordinates). Windows owns the region afterwards.
#[cfg(windows)]
pub fn round_region(hwnd: isize, shapes: &[(i32, i32, i32, i32, i32)]) {
    use windows_sys::Win32::Graphics::Gdi::{DeleteObject, SetWindowRgn};
    if hwnd == 0 || shapes.is_empty() {
        return;
    }
    let acc = make_region(shapes);
    unsafe {
        if !acc.is_null() && SetWindowRgn(hwnd as _, acc, 1) == 0 {
            DeleteObject(acc);
        }
    }
}
#[cfg(not(windows))]
pub fn round_region(_hwnd: isize, _shapes: &[(i32, i32, i32, i32, i32)]) {}

/// Press a key chord (virtual-key codes, held in order, released in reverse).
#[cfg(windows)]
pub fn keys(vks: &[u16]) -> Result<(), String> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
    let key = |vk: u16, up: bool| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
    };
    let mut v: Vec<INPUT> = vks.iter().map(|&k| key(k, false)).collect();
    v.extend(vks.iter().rev().map(|&k| key(k, true)));
    let sent = unsafe { SendInput(v.len() as u32, v.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    if sent as usize == v.len() { Ok(()) } else { Err("Windows didn't take the key press.".into()) }
}
#[cfg(not(windows))]
pub fn keys(_vks: &[u16]) -> Result<(), String> {
    Err("Not on Windows.".into())
}

/// Show or hide the desktop icons, as the desktop's own View menu does
/// (its documented "Show desktop icons" command, 0x7402, sent to the
/// desktop's folder view).
#[cfg(windows)]
pub fn toggle_desktop_icons() -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, SendMessageW, WM_COMMAND};
    let w = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
    unsafe {
        let progman = FindWindowW(w("Progman").as_ptr(), std::ptr::null());
        let mut view = FindWindowExW(progman, std::ptr::null_mut(), w("SHELLDLL_DefView").as_ptr(), std::ptr::null());
        if view.is_null() {
            // With a slideshow wallpaper the view lives under a WorkerW.
            let mut worker = FindWindowExW(std::ptr::null_mut(), std::ptr::null_mut(), w("WorkerW").as_ptr(), std::ptr::null());
            while !worker.is_null() && view.is_null() {
                view = FindWindowExW(worker, std::ptr::null_mut(), w("SHELLDLL_DefView").as_ptr(), std::ptr::null());
                worker = FindWindowExW(std::ptr::null_mut(), worker, w("WorkerW").as_ptr(), std::ptr::null());
            }
        }
        if view.is_null() {
            return Err("Couldn't find the desktop.".into());
        }
        SendMessageW(view, WM_COMMAND, 0x7402, 0);
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn toggle_desktop_icons() -> Result<(), String> {
    Err("Not on Windows.".into())
}
