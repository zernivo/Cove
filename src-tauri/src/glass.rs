//! Window-level glass helpers for the popovers and Search: Windows acrylic
//! tinted for the current look, rounded window regions (DWM itself only
//! rounds to 8 px; Cove's menus are 11, Control Centre 20, Search 14 DIP),
//! plus two small desktop actions the bar's menus use. Documented Win32
//! only: no injection, nothing patched.

/// Acrylic behind a popover window, Ink- or paper-tinted. On Windows 11
/// 22H2+ window-vibrancy uses the system transient backdrop, which follows
/// the window's theme, so the theme is set too.
#[cfg(windows)]
pub fn acrylic(w: &tauri::WebviewWindow, dark: bool) {
    let _ = w.set_theme(Some(if dark { tauri::Theme::Dark } else { tauri::Theme::Light }));
    let tint = if dark { (30, 31, 36, 150) } else { (247, 243, 239, 150) };
    let _ = window_vibrancy::apply_acrylic(w, Some(tint));
}
#[cfg(not(windows))]
pub fn acrylic(_w: &tauri::WebviewWindow, _dark: bool) {}

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

/// Shape a window to the union of rounded rectangles (x, y, w, h, radius;
/// physical px, window coordinates). Windows owns the region afterwards.
#[cfg(windows)]
pub fn round_region(hwnd: isize, shapes: &[(i32, i32, i32, i32, i32)]) {
    use windows_sys::Win32::Graphics::Gdi::{CombineRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, HRGN, RGN_OR};
    if hwnd == 0 || shapes.is_empty() {
        return;
    }
    unsafe {
        let mut acc: HRGN = std::ptr::null_mut();
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
