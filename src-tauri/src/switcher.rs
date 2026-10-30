//! App switcher: Alt+Tab shows one icon per app (not per window), Mac
//! style. Opt-in. Uses a documented low-level keyboard hook
//! (WH_KEYBOARD_LL): nothing is injected into other processes, and the
//! hook is removed the moment the switcher is turned off or Cove quits.
//!
//! Hold Alt, press Tab to move right (Shift+Tab or ← to move left, → right),
//! release Alt to switch, Esc to cancel. Clicking an icon also switches.

use serde::Serialize;
use std::sync::{Mutex, OnceLock};

#[derive(Debug, Clone, Serialize)]
pub struct SwitchApp {
    pub name: String,
    pub exe: String,
    /// Front-to-back.
    pub windows: Vec<isize>,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SwitchState {
    pub apps: Vec<SwitchApp>,
    pub sel: usize,
}

/// What the switcher window should do (handled on the UI side in main.rs).
pub enum Ui {
    /// New list; show the window if the user is still holding Alt shortly.
    Open(SwitchState),
    Update(SwitchState),
    Hide,
}

#[derive(Debug, Clone, Copy)]
enum Ev {
    Open { back: bool },
    Next,
    Prev,
    Commit,
    Cancel,
}

static UI: OnceLock<Box<dyn Fn(Ui) + Send + Sync>> = OnceLock::new();
static STATE: Mutex<Option<SwitchState>> = Mutex::new(None);

pub fn set_ui_handler(f: impl Fn(Ui) + Send + Sync + 'static) {
    let _ = UI.set(Box::new(f));
}

fn ui(u: Ui) {
    if let Some(f) = UI.get() {
        f(u);
    }
}

/// Running apps, grouped by program, most recently used first.
pub fn apps() -> Vec<SwitchApp> {
    let mut out: Vec<SwitchApp> = Vec::new();
    let mut jobs = Vec::new();
    for w in crate::dock::windows() {
        let st = crate::dock::stem(&w.exe);
        let uwp = st == "applicationframehost";
        if !uwp {
            if let Some(a) = out.iter_mut().find(|a| a.exe.eq_ignore_ascii_case(&w.exe)) {
                a.windows.push(w.hwnd);
                continue;
            }
        }
        let icon = if uwp { None } else { crate::icons::get(&w.exe) };
        if icon.is_none() && !uwp {
            jobs.push(w.exe.clone());
        }
        let name = if uwp || st.is_empty() { w.title.clone() } else { crate::topbar::nice_name(&st) };
        out.push(SwitchApp { name, exe: w.exe.clone(), windows: vec![w.hwnd], icon });
    }
    // The app in front comes first even if a tool window sorted above it.
    let fg = crate::dock::foreground();
    if let Some(i) = out.iter().position(|a| a.windows.contains(&fg)) {
        let a = out.remove(i);
        out.insert(0, a);
    }
    if !jobs.is_empty() {
        std::thread::spawn(move || crate::icons::fill(jobs));
    }
    out
}

fn step(sel: usize, n: usize, fwd: bool) -> usize {
    if n == 0 {
        0
    } else if fwd {
        (sel + 1) % n
    } else {
        (sel + n - 1) % n
    }
}

fn handle(ev: Ev) {
    let mut g = STATE.lock().unwrap();
    match ev {
        Ev::Open { back } => {
            let apps = apps();
            let n = apps.len();
            let sel = if n < 2 { 0 } else if back { n - 1 } else { 1 };
            let s = SwitchState { apps, sel };
            *g = Some(s.clone());
            drop(g);
            ui(Ui::Open(s));
        }
        Ev::Next | Ev::Prev => {
            if let Some(s) = g.as_mut() {
                s.sel = step(s.sel, s.apps.len(), matches!(ev, Ev::Next));
                let c = s.clone();
                drop(g);
                ui(Ui::Update(c));
            }
        }
        Ev::Commit => {
            let s = g.take();
            drop(g);
            ui(Ui::Hide);
            if let Some(a) = s.and_then(|s| s.apps.get(s.sel).cloned()) {
                bring_forward(&a);
            }
        }
        Ev::Cancel => {
            let s = g.take();
            drop(g);
            ui(Ui::Hide);
            // Give focus back to the app that was in front.
            if let Some(a) = s.and_then(|s| s.apps.first().cloned()) {
                bring_forward(&a);
            }
        }
    }
}

pub fn is_open() -> bool {
    STATE.lock().map(|g| g.is_some()).unwrap_or(false)
}

/// A click on an icon in the switcher.
pub fn pick(i: usize) {
    imp::set_active(false);
    let s = STATE.lock().unwrap().take();
    ui(Ui::Hide);
    if let Some(a) = s.and_then(|s| s.apps.get(i).cloned()) {
        bring_forward(&a);
    }
}

fn bring_forward(a: &SwitchApp) {
    if let Some(&first) = a.windows.first() {
        imp::unlock_foreground();
        crate::dock::activate(first);
    }
}

pub use imp::{running, set_enabled};

#[cfg(windows)]
mod imp {
    use super::{handle, Ev};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use windows_sys::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::GetCurrentThreadId;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
        MOUSEINPUT, VK_ESCAPE, VK_LEFT, VK_LMENU, VK_MENU, VK_RIGHT, VK_RMENU, VK_SHIFT, VK_TAB,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CallNextHookEx, GetMessageW, PostThreadMessageW, SetWindowsHookExW, UnhookWindowsHookEx, KBDLLHOOKSTRUCT,
        LLKHF_ALTDOWN, LLKHF_INJECTED, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_QUIT, WM_SYSKEYDOWN,
    };

    static ACTIVE: AtomicBool = AtomicBool::new(false);
    static THREAD: AtomicU32 = AtomicU32::new(0);
    static TX: OnceLock<Mutex<Sender<Ev>>> = OnceLock::new();

    pub fn set_active(on: bool) {
        ACTIVE.store(on, Ordering::SeqCst);
    }

    pub fn running() -> bool {
        THREAD.load(Ordering::SeqCst) != 0
    }

    fn send(ev: Ev) {
        if let Some(tx) = TX.get() {
            let _ = tx.lock().unwrap().send(ev);
        }
    }

    /// An unassigned key tap, so releasing Alt doesn't open the app's menu bar.
    fn dummy_key() {
        let k = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: 0xE8, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
            },
        };
        let i = [k(false), k(true)];
        unsafe { SendInput(2, i.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    /// A zero-length mouse input lets Windows accept our SetForegroundWindow
    /// (the same documented-input approach PowerToys uses).
    pub fn unlock_foreground() {
        let i = [INPUT {
            r#type: INPUT_MOUSE,
            Anonymous: INPUT_0 { mi: MOUSEINPUT { dx: 0, dy: 0, mouseData: 0, dwFlags: 0, time: 0, dwExtraInfo: 0 } },
        }];
        unsafe { SendInput(1, i.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    unsafe extern "system" fn proc(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        if code >= 0 {
            let k = &*(lp as *const KBDLLHOOKSTRUCT);
            let vk = k.vkCode as u16;
            let down = wp as u32 == WM_KEYDOWN || wp as u32 == WM_SYSKEYDOWN;
            if k.flags & LLKHF_INJECTED == 0 {
                let active = ACTIVE.load(Ordering::SeqCst);
                if vk == VK_TAB && (k.flags & LLKHF_ALTDOWN != 0 || active) {
                    if down {
                        let back = GetAsyncKeyState(VK_SHIFT as i32) < 0;
                        if !ACTIVE.swap(true, Ordering::SeqCst) {
                            dummy_key();
                            send(Ev::Open { back });
                        } else {
                            send(if back { Ev::Prev } else { Ev::Next });
                        }
                    }
                    return 1;
                }
                if active {
                    if vk == VK_ESCAPE {
                        if down {
                            ACTIVE.store(false, Ordering::SeqCst);
                            send(Ev::Cancel);
                        }
                        return 1;
                    }
                    if vk == VK_LEFT || vk == VK_RIGHT {
                        if down {
                            send(if vk == VK_LEFT { Ev::Prev } else { Ev::Next });
                        }
                        return 1;
                    }
                    if (vk == VK_LMENU || vk == VK_RMENU || vk == VK_MENU) && !down {
                        ACTIVE.store(false, Ordering::SeqCst);
                        send(Ev::Commit);
                    }
                }
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, wp, lp)
    }

    /// Install (true) or remove (false) the keyboard hook.
    pub fn set_enabled(on: bool) {
        if on {
            if running() {
                return;
            }
            let rx = {
                let (tx, rx) = channel::<Ev>();
                match TX.get() {
                    Some(m) => *m.lock().unwrap() = tx,
                    None => {
                        let _ = TX.set(Mutex::new(tx));
                    }
                }
                rx
            };
            // Events are handled off the hook thread so the hook stays instant.
            std::thread::spawn(move || {
                while let Ok(ev) = rx.recv() {
                    handle(ev);
                }
            });
            std::thread::spawn(|| unsafe {
                let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc), GetModuleHandleW(std::ptr::null()), 0);
                if hook.is_null() {
                    return;
                }
                THREAD.store(GetCurrentThreadId(), Ordering::SeqCst);
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {}
                UnhookWindowsHookEx(hook);
                THREAD.store(0, Ordering::SeqCst);
            });
        } else {
            let t = THREAD.load(Ordering::SeqCst);
            if t != 0 {
                unsafe { PostThreadMessageW(t, WM_QUIT, 0, 0) };
            }
            if ACTIVE.swap(false, Ordering::SeqCst) {
                send(Ev::Cancel);
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn set_active(_on: bool) {}
    pub fn running() -> bool {
        false
    }
    pub fn unlock_foreground() {}
    pub fn set_enabled(_on: bool) {}
}

#[cfg(test)]
mod tests {
    #[test]
    fn steps_wrap() {
        assert_eq!(super::step(2, 3, true), 0);
        assert_eq!(super::step(0, 3, false), 2);
        assert_eq!(super::step(0, 0, true), 0);
    }
}
