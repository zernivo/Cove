//! Mac-style keyboard habits, done with documented Win32 calls only:
//! RegisterHotKey + standard window messages. Nothing is injected.
//!
//!   Alt+W  close window        Alt+M  minimize window
//!   Alt+H  hide app            Alt+Q  quit app (closes its windows)
//!   Alt+`  next window of the same app

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CloseWindow,
    Minimize,
    HideApp,
    QuitApp,
    CycleAppWindows,
}

pub struct Binding {
    pub id: i32,
    pub label: &'static str,
    pub key: u32, // virtual-key code
    pub action: Action,
}

pub static BINDINGS: &[Binding] = &[
    Binding { id: 1, label: "Alt+W", key: b'W' as u32, action: Action::CloseWindow },
    Binding { id: 2, label: "Alt+M", key: b'M' as u32, action: Action::Minimize },
    Binding { id: 3, label: "Alt+H", key: b'H' as u32, action: Action::HideApp },
    Binding { id: 4, label: "Alt+Q", key: b'Q' as u32, action: Action::QuitApp },
    Binding { id: 5, label: "Alt+`", key: 0xC0, action: Action::CycleAppWindows }, // VK_OEM_3
];

#[derive(Debug, Clone, Serialize)]
pub struct HotkeyStatus {
    pub running: bool,
    /// Shortcuts another app already owns.
    pub conflicts: Vec<&'static str>,
}

#[cfg(windows)]
mod imp {
    use super::{Action, HotkeyStatus, BINDINGS};
    use std::ffi::c_void;
    use std::sync::{mpsc, Mutex};
    use windows_sys::Win32::Foundation::{BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        RegisterHotKey, UnregisterHotKey, MOD_ALT, MOD_NOREPEAT,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetForegroundWindow, GetMessageW, GetWindow,
        GetWindowLongW, GetWindowRect, GetWindowTextLengthW, GWL_EXSTYLE, WS_EX_TOOLWINDOW,
        GetWindowThreadProcessId, IsIconic, IsWindowVisible, PostMessageW, PostThreadMessageW,
        SetForegroundWindow, ShowWindow, GW_OWNER, MSG, SW_MINIMIZE, SW_RESTORE, WM_CLOSE,
        WM_HOTKEY, WM_QUIT,
    };

    static THREAD: Mutex<Option<(u32, Vec<&'static str>)>> = Mutex::new(None);

    /// Never act on the desktop or taskbar (WM_CLOSE on the desktop opens
    /// the shutdown dialog).
    fn is_shell(hwnd: HWND) -> bool {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        let class = String::from_utf16_lossy(&buf[..n.max(0) as usize]);
        matches!(
            class.as_str(),
            "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"
        )
    }

    fn pid_of(hwnd: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(hwnd, &mut pid) };
        pid
    }

    struct Collect {
        pid: u32,
        found: Vec<HWND>,
    }

    /// A "real" window, the kind Alt+Tab shows: visible, not cloaked
    /// (hidden by DWM, e.g. on another virtual desktop), not a tool
    /// window, has a title and a size. Apps like Chrome own many invisible
    /// helper windows that would otherwise be picked.
    unsafe fn is_real_window(hwnd: HWND) -> bool {
        if IsWindowVisible(hwnd) == 0 || !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        if (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32) & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED as _,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        );
        if hr == 0 && cloaked != 0 {
            return false;
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) != 0 && (r.right - r.left <= 1 || r.bottom - r.top <= 1) {
            return IsIconic(hwnd) != 0; // minimized windows still count
        }
        true
    }

    unsafe extern "system" fn enum_cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let c = &mut *(lparam as *mut Collect);
        if pid_of(hwnd) == c.pid && is_real_window(hwnd) && !is_shell(hwnd) {
            c.found.push(hwnd);
        }
        1
    }

    /// Top-level windows of a process, front to back.
    fn app_windows(pid: u32) -> Vec<HWND> {
        let mut c = Collect { pid, found: Vec::new() };
        unsafe { EnumWindows(Some(enum_cb), &mut c as *mut Collect as LPARAM) };
        c.found
    }

    fn run(action: Action) {
        let fg = unsafe { GetForegroundWindow() };
        if fg.is_null() || is_shell(fg) {
            return;
        }
        unsafe {
            match action {
                Action::CloseWindow => {
                    PostMessageW(fg, WM_CLOSE, 0, 0);
                }
                Action::Minimize => {
                    ShowWindow(fg, SW_MINIMIZE);
                }
                Action::HideApp => {
                    for w in app_windows(pid_of(fg)) {
                        ShowWindow(w, SW_MINIMIZE);
                    }
                }
                Action::QuitApp => {
                    // Polite: each window gets WM_CLOSE, so apps can ask to save.
                    for w in app_windows(pid_of(fg)) {
                        PostMessageW(w, WM_CLOSE, 0, 0);
                    }
                }
                Action::CycleAppWindows => {
                    let wins = app_windows(pid_of(fg));
                    if let Some(&next) = wins.last() {
                        if next != fg {
                            if IsIconic(next) != 0 {
                                ShowWindow(next, SW_RESTORE);
                            }
                            SetForegroundWindow(next);
                        }
                    }
                }
            }
        }
    }

    pub fn start() -> HotkeyStatus {
        if let Some((_, conflicts)) = THREAD.lock().unwrap().as_ref() {
            return HotkeyStatus { running: true, conflicts: conflicts.clone() };
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let null: HWND = std::ptr::null_mut::<c_void>();
            let mut conflicts = Vec::new();
            for b in BINDINGS {
                let ok = unsafe { RegisterHotKey(null, b.id, MOD_ALT | MOD_NOREPEAT, b.key) };
                if ok == 0 {
                    conflicts.push(b.label);
                }
            }
            let _ = tx.send((unsafe { GetCurrentThreadId() }, conflicts));
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, null, 0, 0) } > 0 {
                if msg.message == WM_HOTKEY {
                    if let Some(b) = BINDINGS.iter().find(|b| b.id == msg.wParam as i32) {
                        run(b.action);
                    }
                }
            }
            for b in BINDINGS {
                unsafe { UnregisterHotKey(null, b.id) };
            }
        });
        let (tid, conflicts) = rx.recv().unwrap_or((0, Vec::new()));
        *THREAD.lock().unwrap() = Some((tid, conflicts.clone()));
        HotkeyStatus { running: true, conflicts }
    }

    pub fn stop() -> HotkeyStatus {
        if let Some((tid, _)) = THREAD.lock().unwrap().take() {
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
        }
        HotkeyStatus { running: false, conflicts: Vec::new() }
    }

    pub fn status() -> HotkeyStatus {
        match THREAD.lock().unwrap().as_ref() {
            Some((_, c)) => HotkeyStatus { running: true, conflicts: c.clone() },
            None => HotkeyStatus { running: false, conflicts: Vec::new() },
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::HotkeyStatus;
    pub fn start() -> HotkeyStatus {
        HotkeyStatus { running: false, conflicts: Vec::new() }
    }
    pub fn stop() -> HotkeyStatus {
        start()
    }
    pub fn status() -> HotkeyStatus {
        start()
    }
}

pub use imp::{start, status, stop};
