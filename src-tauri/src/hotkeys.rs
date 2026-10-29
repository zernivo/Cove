//! Mac-style keyboard habits, done with documented Win32 calls only:
//! RegisterHotKey + standard window messages. Nothing is injected.
//!
//!   Alt+W  close window        Alt+M  minimize window
//!   Alt+H  hide app            Alt+Q  quit app (closes its windows)
//!   Alt+`  next window of the same app
//!   Alt+Space  Cove Search (separate switch)

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

pub const SEARCH_ID: i32 = 10;
pub const SEARCH_LABEL: &str = "Alt+Space";

#[derive(Debug, Clone, Serialize, Default)]
pub struct HotkeyStatus {
    /// Mac habit shortcuts active.
    pub running: bool,
    /// Alt+Space search active.
    pub search: bool,
    /// Shortcuts another app already owns.
    pub conflicts: Vec<&'static str>,
}

/// Called (on the hotkey thread) when Alt+Space is pressed.
static ON_SEARCH: std::sync::OnceLock<Box<dyn Fn() + Send + Sync>> = std::sync::OnceLock::new();

pub fn set_search_handler(f: impl Fn() + Send + Sync + 'static) {
    let _ = ON_SEARCH.set(Box::new(f));
}

#[allow(dead_code)]
fn fire_search() {
    if let Some(f) = ON_SEARCH.get() {
        f();
    }
}

#[cfg(windows)]
mod imp {
    use super::{fire_search, Action, HotkeyStatus, BINDINGS, SEARCH_ID, SEARCH_LABEL};
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

    struct Running {
        tid: u32,
        status: HotkeyStatus,
    }
    static THREAD: Mutex<Option<Running>> = Mutex::new(None);

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

    fn stop_thread() {
        if let Some(r) = THREAD.lock().unwrap().take() {
            unsafe { PostThreadMessageW(r.tid, WM_QUIT, 0, 0) };
        }
    }

    /// (Re)start the hotkey thread with the chosen groups.
    pub fn configure(habits: bool, search: bool) -> HotkeyStatus {
        stop_thread();
        if !habits && !search {
            return HotkeyStatus::default();
        }
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let null: HWND = std::ptr::null_mut::<c_void>();
            let mut conflicts = Vec::new();
            let mut ids = Vec::new();
            if habits {
                for b in BINDINGS {
                    if unsafe { RegisterHotKey(null, b.id, MOD_ALT | MOD_NOREPEAT, b.key) } == 0 {
                        conflicts.push(b.label);
                    } else {
                        ids.push(b.id);
                    }
                }
            }
            if search {
                // VK_SPACE = 0x20
                if unsafe { RegisterHotKey(null, SEARCH_ID, MOD_ALT | MOD_NOREPEAT, 0x20) } == 0 {
                    conflicts.push(SEARCH_LABEL);
                } else {
                    ids.push(SEARCH_ID);
                }
            }
            let _ = tx.send((unsafe { GetCurrentThreadId() }, conflicts));
            let mut msg: MSG = unsafe { std::mem::zeroed() };
            while unsafe { GetMessageW(&mut msg, null, 0, 0) } > 0 {
                if msg.message == WM_HOTKEY {
                    let id = msg.wParam as i32;
                    if id == SEARCH_ID {
                        fire_search();
                    } else if let Some(b) = BINDINGS.iter().find(|b| b.id == id) {
                        run(b.action);
                    }
                }
            }
            for id in ids {
                unsafe { UnregisterHotKey(null, id) };
            }
        });
        let (tid, conflicts) = rx.recv().unwrap_or((0, Vec::new()));
        let status = HotkeyStatus { running: habits, search, conflicts };
        *THREAD.lock().unwrap() = Some(Running { tid, status: status.clone() });
        status
    }

    pub fn stop() {
        stop_thread();
    }

    pub fn status() -> HotkeyStatus {
        THREAD.lock().unwrap().as_ref().map(|r| r.status.clone()).unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    use super::HotkeyStatus;
    pub fn configure(_habits: bool, _search: bool) -> HotkeyStatus {
        HotkeyStatus::default()
    }
    pub fn stop() {}
    pub fn status() -> HotkeyStatus {
        HotkeyStatus::default()
    }
}

pub use imp::{configure, status, stop};
