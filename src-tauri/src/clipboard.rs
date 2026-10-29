//! Clipboard history for Cove Search. Text only, kept in memory only
//! (gone when Cove quits), and it skips anything a password manager marks
//! as private.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;

const MAX: usize = 50;
const MAX_LEN: usize = 20_000;

static ENABLED: AtomicBool = AtomicBool::new(false);
static STARTED: AtomicBool = AtomicBool::new(false);
static HISTORY: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::SeqCst);
    if !on {
        clear();
    }
    if on && !STARTED.swap(true, Ordering::SeqCst) {
        std::thread::spawn(watch);
    }
}

pub fn clear() {
    HISTORY.lock().unwrap().clear();
}

/// Newest first.
pub fn history() -> Vec<String> {
    HISTORY.lock().unwrap().clone()
}

fn remember(text: String) {
    if text.trim().is_empty() || text.len() > MAX_LEN {
        return;
    }
    let mut h = HISTORY.lock().unwrap();
    h.retain(|t| t != &text);
    h.insert(0, text);
    h.truncate(MAX);
}

fn watch() {
    let mut last = imp::sequence();
    loop {
        std::thread::sleep(Duration::from_millis(400));
        if !ENABLED.load(Ordering::SeqCst) {
            continue;
        }
        let now = imp::sequence();
        if now != last {
            last = now;
            if let Some(t) = imp::read_text() {
                remember(t);
            }
        }
    }
}

/// Put text on the clipboard; with `paste`, also send Ctrl+V to the window
/// that gets focus back once Search hides.
pub fn put(text: &str, paste: bool) -> Result<(), String> {
    imp::write_text(text)?;
    remember(text.to_string());
    if paste {
        std::thread::sleep(Duration::from_millis(150));
        imp::ctrl_v();
    }
    Ok(())
}

#[cfg(windows)]
mod imp {
    use windows::core::w;
    use windows::Win32::Foundation::{HANDLE, HGLOBAL};
    use windows::Win32::System::DataExchange::{
        CloseClipboard, EmptyClipboard, GetClipboardData, GetClipboardSequenceNumber,
        IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    };
    use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, VIRTUAL_KEY,
        VK_CONTROL,
    };

    const CF_UNICODETEXT: u32 = 13;

    pub fn sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    fn open() -> bool {
        for _ in 0..5 {
            if unsafe { OpenClipboard(None) }.is_ok() {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        false
    }

    pub fn read_text() -> Option<String> {
        unsafe {
            // Password managers and other apps flag private copies with these.
            let private = [
                RegisterClipboardFormatW(w!("ExcludeClipboardContentFromMonitorProcessing")),
                RegisterClipboardFormatW(w!("Clipboard Viewer Ignore")),
            ];
            if private.iter().any(|&f| f != 0 && IsClipboardFormatAvailable(f).is_ok()) {
                return None;
            }
            if IsClipboardFormatAvailable(CF_UNICODETEXT).is_err() || !open() {
                return None;
            }
            let mut out = None;
            if let Ok(h) = GetClipboardData(CF_UNICODETEXT) {
                let g = HGLOBAL(h.0);
                let p = GlobalLock(g) as *const u16;
                if !p.is_null() {
                    let max = GlobalSize(g) / 2;
                    let mut n = 0;
                    while n < max && *p.add(n) != 0 {
                        n += 1;
                    }
                    out = Some(String::from_utf16_lossy(std::slice::from_raw_parts(p, n)));
                    let _ = GlobalUnlock(g);
                }
            }
            let _ = CloseClipboard();
            out
        }
    }

    pub fn write_text(text: &str) -> Result<(), String> {
        let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            if !open() {
                return Err("The clipboard is busy".into());
            }
            let res = (|| {
                EmptyClipboard().map_err(|e| e.to_string())?;
                let g = GlobalAlloc(GMEM_MOVEABLE, wide.len() * 2).map_err(|e| e.to_string())?;
                let p = GlobalLock(g) as *mut u16;
                if p.is_null() {
                    return Err("Out of memory".to_string());
                }
                std::ptr::copy_nonoverlapping(wide.as_ptr(), p, wide.len());
                let _ = GlobalUnlock(g);
                SetClipboardData(CF_UNICODETEXT, Some(HANDLE(g.0))).map_err(|e| e.to_string())?;
                Ok(())
            })();
            let _ = CloseClipboard();
            res
        }
    }

    pub fn ctrl_v() {
        let key = |vk: VIRTUAL_KEY, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: vk, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, ..Default::default() },
            },
        };
        let v = VIRTUAL_KEY(0x56);
        let inputs = [key(VK_CONTROL, false), key(v, false), key(v, true), key(VK_CONTROL, true)];
        unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn sequence() -> u32 {
        0
    }
    pub fn read_text() -> Option<String> {
        None
    }
    pub fn write_text(_t: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn ctrl_v() {}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dedupes_newest_first() {
        clear();
        remember("a".into());
        remember("b".into());
        remember("a".into());
        remember("   ".into());
        assert_eq!(history(), vec!["a".to_string(), "b".to_string()]);
        clear();
    }
}
