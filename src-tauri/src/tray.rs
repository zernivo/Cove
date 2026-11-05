//! Other apps' tray icons (Discord, Steam, Spotify, OneDrive...).
//!
//! Cove hides the Windows taskbar, and with it the notification area, so
//! Cove shows those icons itself, in Control Centre's "Other apps" grid.
//! Nothing is injected and nothing is written into another process:
//!
//! 1. The stand-in tray. Apps add their icons with `Shell_NotifyIcon`, which
//!    finds the top-level window of class `Shell_TrayWnd` and sends it a
//!    `WM_COPYDATA` message. While Cove's desktop is on, Cove keeps a hidden
//!    window of that class above Explorer's, reads each message (add,
//!    modify, delete, set version) and then forwards it, unchanged, to
//!    Explorer's own taskbar and returns Explorer's answer, so Explorer stays
//!    complete and correct the whole time. A `TaskbarCreated` broadcast after
//!    it appears makes running apps add their icons again. The technique is
//!    the one in systray-util (MIT, glzr-io/zebar; see THIRD_PARTY_NOTICES),
//!    without its start-up step that reads Explorer's memory.
//! 2. UI Automation reads the names of the icons Windows itself shows, to
//!    name icons that came without a tooltip and to list ones the stand-in
//!    missed (apps that ignore `TaskbarCreated`).
//! 3. "Open Windows tray" briefly shows the taskbar and Windows' own overflow.
//!
//! The stand-in exists only while the takeover is on, the setting is on, and
//! no other tool that also replaces the tray (YASB, Zebar, TranslucentTB,
//! ExplorerPatcher, Seelen) is running. Turning it off destroys the window
//! and broadcasts `TaskbarCreated` again, so apps re-register with Explorer.
//! If Cove crashes, its window dies with it; the watchdog then broadcasts
//! `TaskbarCreated` (and Explorer had every icon all along anyway).

use serde::Serialize;
use std::sync::Mutex;

// ---- Shell_NotifyIcon's wire format (pure; unit-tested on any OS) ----

/// First DWORD of the `WM_COPYDATA` payload shell32 sends to `Shell_TrayWnd`.
pub const TRAY_SIGNATURE: u32 = 0x3475_3423;
/// `COPYDATASTRUCT.dwData` values sent to `Shell_TrayWnd`.
pub const CD_NOTIFY: usize = 1;
pub const CD_ICON_RECT: usize = 3;

pub const NIM_ADD: u32 = 0;
pub const NIM_MODIFY: u32 = 1;
pub const NIM_DELETE: u32 = 2;
pub const NIM_SETVERSION: u32 = 4;

pub const NIF_MESSAGE: u32 = 0x01;
pub const NIF_ICON: u32 = 0x02;
pub const NIF_TIP: u32 = 0x04;
pub const NIF_STATE: u32 = 0x08;
pub const NIF_GUID: u32 = 0x20;
pub const NIS_HIDDEN: u32 = 0x01;
pub const NOTIFYICON_VERSION_4: u32 = 4;

pub const WM_CONTEXTMENU: u32 = 0x007B;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_LBUTTONUP: u32 = 0x0202;
pub const WM_LBUTTONDBLCLK: u32 = 0x0203;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_RBUTTONUP: u32 = 0x0205;
/// `NIN_SELECT` (WM_USER + 0).
pub const NIN_SELECT: u32 = 0x0400;

/// Where each field sits inside the NOTIFYICONDATAW that follows the 8-byte
/// header (signature, message). shell32 always marshals it with 32-bit
/// handles (so 32- and 64-bit apps send the same thing); the native 64-bit
/// layout is accepted too, recognised by its size.
struct Layout {
    size: usize,
    hwnd: (usize, usize),
    uid: usize,
    flags: usize,
    callback: usize,
    hicon: (usize, usize),
    tip: usize,
    state: usize,
    state_mask: usize,
    version: usize,
    guid: usize,
}

/// 32-bit handles: 956 bytes.
const L32: Layout = Layout { size: 956, hwnd: (4, 4), uid: 8, flags: 12, callback: 16, hicon: (20, 4), tip: 24, state: 280, state_mask: 284, version: 800, guid: 936 };
/// Native x64 NOTIFYICONDATAW (8-byte handles, padded): 976 bytes.
const L64: Layout = Layout { size: 976, hwnd: (8, 8), uid: 16, flags: 20, callback: 24, hicon: (32, 8), tip: 40, state: 296, state_mask: 300, version: 816, guid: 952 };
const TIP_CHARS: usize = 128;

/// One `Shell_NotifyIcon` call, as Explorer receives it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Notify {
    pub message: u32,
    pub hwnd: u64,
    pub uid: u32,
    pub flags: u32,
    pub callback: u32,
    pub hicon: u64,
    /// Present when NIF_TIP is set.
    pub tip: Option<String>,
    pub state: u32,
    pub state_mask: u32,
    /// The uTimeout/uVersion union (meaningful for NIM_SETVERSION).
    pub version: u32,
    /// "{XXXXXXXX-...}" when NIF_GUID is set and the GUID isn't zero.
    pub guid: Option<String>,
    /// The native 64-bit layout was used.
    pub wide: bool,
}

/// Bounds-checked little-endian reads: a short or malformed message reads
/// as zeros instead of running off the end (never trust what arrives).
fn rd(b: &[u8], off: usize, n: usize) -> u64 {
    let mut v = 0u64;
    for i in 0..n {
        v |= (*b.get(off + i).unwrap_or(&0) as u64) << (8 * i);
    }
    v
}

fn guid_string(b: &[u8]) -> Option<String> {
    if b.len() < 16 || b.iter().all(|&x| x == 0) {
        return None;
    }
    let d1 = rd(b, 0, 4);
    let d2 = rd(b, 4, 2);
    let d3 = rd(b, 6, 2);
    Some(format!(
        "{{{:08X}-{:04X}-{:04X}-{:02X}{:02X}-{:02X}{:02X}{:02X}{:02X}{:02X}{:02X}}}",
        d1, d2, d3, b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
    ))
}

/// Parse a `COPYDATASTRUCT` payload sent with dwData 1.
pub fn parse(bytes: &[u8]) -> Option<Notify> {
    if bytes.len() < 8 + 24 || rd(bytes, 0, 4) as u32 != TRAY_SIGNATURE {
        return None;
    }
    let message = rd(bytes, 4, 4) as u32;
    let nid = &bytes[8..];
    let cb = rd(nid, 0, 4) as usize;
    let l = if cb == L64.size && nid.len() >= L64.size { &L64 } else { &L32 };
    let flags = rd(nid, l.flags, 4) as u32;
    let tip = (flags & NIF_TIP != 0).then(|| {
        let mut u = Vec::with_capacity(TIP_CHARS);
        for i in 0..TIP_CHARS {
            let c = rd(nid, l.tip + i * 2, 2) as u16;
            if c == 0 {
                break;
            }
            u.push(c);
        }
        String::from_utf16_lossy(&u).replace('\r', "")
    });
    let guid = if flags & NIF_GUID != 0 { nid.get(l.guid..l.guid + 16).and_then(guid_string) } else { None };
    Some(Notify {
        message,
        hwnd: rd(nid, l.hwnd.0, l.hwnd.1),
        uid: rd(nid, l.uid, 4) as u32,
        flags,
        callback: rd(nid, l.callback, 4) as u32,
        hicon: rd(nid, l.hicon.0, l.hicon.1),
        tip,
        state: rd(nid, l.state, 4) as u32,
        state_mask: rd(nid, l.state_mask, 4) as u32,
        version: rd(nid, l.version, 4) as u32,
        guid,
        wide: std::ptr::eq(l, &L64),
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Click {
    Left,
    Right,
    Double,
}

impl Click {
    pub fn from_str(s: &str) -> Option<Click> {
        match s {
            "left" => Some(Click::Left),
            "right" => Some(Click::Right),
            "double" => Some(Click::Double),
            _ => None,
        }
    }
}

/// The callback messages Explorer sends an icon for a click, as (wParam,
/// lParam) pairs for the app's `uCallbackMessage`, in order.
///
/// Version 4: wParam = the anchor point (x low, y high word), lParam = the
/// event in the low word and the icon ID in the high word. Older versions:
/// wParam = the icon ID, lParam = the mouse message. Version 3 and up also
/// get NIN_SELECT after a left click and WM_CONTEXTMENU after a right click
/// (Explorer sends them to version 3 as well).
pub fn click_messages(version: u32, uid: u32, click: Click, x: i32, y: i32) -> Vec<(usize, isize)> {
    let mut msgs: Vec<u32> = match click {
        Click::Left => vec![WM_LBUTTONDOWN, WM_LBUTTONUP],
        Click::Right => vec![WM_RBUTTONDOWN, WM_RBUTTONUP],
        Click::Double => vec![WM_LBUTTONDBLCLK, WM_LBUTTONUP],
    };
    if version >= 3 {
        match click {
            Click::Left => msgs.push(NIN_SELECT),
            Click::Right => msgs.push(WM_CONTEXTMENU),
            Click::Double => {}
        }
    }
    msgs.into_iter()
        .map(|m| {
            if version >= NOTIFYICON_VERSION_4 {
                let wp = ((x as u16 as u32) | ((y as u16 as u32) << 16)) as usize;
                let lp = ((m & 0xFFFF) | ((uid & 0xFFFF) << 16)) as i32 as isize;
                (wp, lp)
            } else {
                (uid as usize, m as isize)
            }
        })
        .collect()
}

/// LRESULT for a `Shell_NotifyIconGetRect` query (dwData 3): message 1 asks
/// for (left, top), message 2 for (right, bottom), each packed low/high.
pub fn icon_rect_reply(message: u32, rect: (i32, i32, i32, i32)) -> isize {
    let pack = |a: i32, b: i32| ((a as u16 as u32) | ((b as u16 as u32) << 16)) as i32 as isize;
    match message {
        1 => pack(rect.0, rect.1),
        2 => pack(rect.2, rect.3),
        _ => 0,
    }
}

// ---- The icons Cove knows about ----

#[derive(Debug, Clone)]
pub struct Icon {
    pub id: String,
    pub hwnd: u64,
    pub uid: u32,
    pub guid: Option<String>,
    pub callback: u32,
    pub version: u32,
    pub tip: String,
    pub hicon: u64,
    pub png: Option<std::sync::Arc<Vec<u8>>>,
    pub rev: u32,
    pub hidden: bool,
}

fn icon_id(guid: &Option<String>, hwnd: u64, uid: u32) -> String {
    match guid {
        Some(g) => format!("g{}", g.trim_matches(|c| c == '{' || c == '}').to_lowercase()),
        None => format!("h{hwnd:x}-{uid}"),
    }
}

fn same(i: &Icon, n: &Notify) -> bool {
    match (&i.guid, &n.guid) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => i.hwnd == n.hwnd && i.uid == n.uid,
    }
}

/// Fold one message into `icons`; `png` renders an HICON. Returns whether
/// anything visible changed. NIM_MODIFY for an unknown icon adds it (only
/// with an icon), which catches apps that missed `TaskbarCreated` but
/// update their icon now and then.
pub fn apply(icons: &mut Vec<Icon>, n: &Notify, png: &mut dyn FnMut(u64) -> Option<Vec<u8>>) -> bool {
    match n.message {
        NIM_DELETE => {
            let before = icons.len();
            icons.retain(|i| !same(i, n));
            icons.len() != before
        }
        NIM_SETVERSION => {
            if let Some(i) = icons.iter_mut().find(|i| same(i, n)) {
                i.version = n.version;
            }
            false
        }
        NIM_ADD | NIM_MODIFY => {
            let pos = icons.iter().position(|i| same(i, n));
            let idx = match pos {
                Some(p) => p,
                None => {
                    if n.message == NIM_MODIFY && (n.flags & NIF_ICON == 0 || n.hicon == 0) {
                        return false;
                    }
                    icons.push(Icon {
                        id: icon_id(&n.guid, n.hwnd, n.uid),
                        hwnd: n.hwnd,
                        uid: n.uid,
                        guid: n.guid.clone(),
                        callback: 0,
                        version: 0,
                        tip: String::new(),
                        hicon: 0,
                        png: None,
                        rev: 0,
                        hidden: false,
                    });
                    icons.len() - 1
                }
            };
            let i = &mut icons[idx];
            let mut changed = pos.is_none();
            if n.hwnd != 0 {
                i.hwnd = n.hwnd;
                i.uid = n.uid;
            }
            if n.flags & NIF_MESSAGE != 0 {
                i.callback = n.callback;
            }
            if let Some(t) = &n.tip {
                if *t != i.tip {
                    i.tip = t.clone();
                    changed = true;
                }
            }
            if n.flags & NIF_STATE != 0 && n.state_mask & NIS_HIDDEN != 0 {
                let h = n.state & NIS_HIDDEN != 0;
                changed |= h != i.hidden;
                i.hidden = h;
            }
            if n.flags & NIF_ICON != 0 && n.hicon != 0 {
                if let Some(p) = png(n.hicon) {
                    if i.png.as_deref() != Some(&p) {
                        i.png = Some(std::sync::Arc::new(p));
                        i.rev = i.rev.wrapping_add(1);
                        changed = true;
                    }
                }
                i.hicon = n.hicon;
            }
            changed
        }
        _ => false,
    }
}

/// A process name that means another tool already replaces or patches the
/// tray (two stand-ins fight, and TranslucentTB latches onto the wrong one).
pub fn conflict_name(exe: &str) -> Option<&'static str> {
    let e = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_lowercase();
    Some(match e.as_str() {
        "yasb.exe" => "YASB",
        "zebar.exe" => "Zebar",
        "komorebi-bar.exe" => "komorebi-bar",
        "translucenttb.exe" => "TranslucentTB",
        "seelen-ui.exe" | "slu-service.exe" => "Seelen UI",
        "ep_dwm.exe" | "ep_taskbar.exe" | "explorerpatcher.exe" => "ExplorerPatcher",
        _ => return None,
    })
}

/// The first line of a tooltip ("Discord\n3 unread" -> "Discord").
pub fn first_line(tip: &str) -> &str {
    tip.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("")
}

// ---- State shared with the UI ----

static ICONS: Mutex<Vec<Icon>> = Mutex::new(Vec::new());
/// Images for icons only Windows' own tray knows (from its icon cache).
static EXTRA_PNG: Mutex<Vec<(String, std::sync::Arc<Vec<u8>>)>> = Mutex::new(Vec::new());

struct Run {
    /// The stand-in should exist (setting on, takeover on, not safe mode).
    wanted: bool,
    /// Its window, while it runs.
    hwnd: isize,
    thread: Option<std::thread::JoinHandle<()>>,
    /// Why it isn't running although wanted.
    blocked: Option<String>,
}
static RUN: Mutex<Run> = Mutex::new(Run { wanted: false, hwnd: 0, thread: None, blocked: None });
static APP: std::sync::OnceLock<tauri::AppHandle> = std::sync::OnceLock::new();
static MARKER: std::sync::OnceLock<std::path::PathBuf> = std::sync::OnceLock::new();
/// Windows' own overflow is showing (the bars leave the taskbar alone).
pub static NATIVE_OPEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn log(msg: &str) {
    if let Some(a) = APP.get() {
        crate::cove_log(a, &format!("tray: {msg}"));
    }
}

/// Tell the popover and the dock to redraw (at most every 250 ms; the
/// stand-in's timer delivers a change that was held back).
static LAST_EMIT: Mutex<Option<std::time::Instant>> = Mutex::new(None);
static PENDING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn changed() {
    let mut last = lock(&LAST_EMIT);
    if last.is_some_and(|t| t.elapsed() < std::time::Duration::from_millis(250)) {
        PENDING.store(true, std::sync::atomic::Ordering::SeqCst);
        return;
    }
    *last = Some(std::time::Instant::now());
    PENDING.store(false, std::sync::atomic::Ordering::SeqCst);
    if let Some(a) = APP.get() {
        use tauri::Emitter;
        let _ = a.emit("tray-changed", ());
    }
}
fn flush() {
    if PENDING.load(std::sync::atomic::Ordering::SeqCst) {
        changed();
    }
}

fn marker_path_for(settings: &std::path::Path) -> std::path::PathBuf {
    settings.with_file_name("tray-standin.on")
}

/// Called once at start-up with the settings file. A marker left by a run
/// that died with the stand-in up means apps may still be looking for it:
/// have them re-register with Explorer.
pub fn init(app: &tauri::AppHandle, settings: &std::path::Path) {
    let _ = APP.set(app.clone());
    let m = marker_path_for(settings);
    if m.exists() {
        imp::broadcast_taskbar_created();
        let _ = std::fs::remove_file(&m);
        log("previous run ended with the stand-in tray up; asked apps to re-add their icons to Windows");
    }
    let _ = MARKER.set(m);
}

/// The watchdog copy, once Cove has ended (crash included).
pub fn after_exit(settings: &std::path::Path) {
    let m = marker_path_for(settings);
    if m.exists() {
        // Give Explorer's taskbar a moment to be shown again first.
        std::thread::sleep(std::time::Duration::from_millis(300));
        imp::broadcast_taskbar_created();
        let _ = std::fs::remove_file(&m);
    }
}

/// Start or stop the stand-in. `want` = the setting is on, the takeover is
/// on and Cove isn't in safe mode. Cheap when nothing changes.
pub fn sync(want: bool) {
    let mut r = lock(&RUN);
    r.wanted = want;
    let running = r.hwnd != 0;
    if want && !running {
        if let Some(c) = imp::conflict() {
            if r.blocked.as_deref() != Some(c.as_str()) {
                log(&format!("not starting the stand-in tray: {c} is running"));
            }
            r.blocked = Some(c);
            return;
        }
        r.blocked = None;
        let (tx, rx) = std::sync::mpsc::channel::<isize>();
        let t = std::thread::Builder::new().name("cove-tray".into()).spawn(move || imp::run(tx));
        match t {
            Ok(t) => {
                r.thread = Some(t);
                match rx.recv_timeout(std::time::Duration::from_secs(2)) {
                    Ok(h) if h != 0 => {
                        r.hwnd = h;
                        if let Some(m) = MARKER.get() {
                            let _ = std::fs::write(m, b"1");
                        }
                        log("stand-in tray started; asked running apps to add their icons");
                    }
                    _ => {
                        r.blocked = Some("the stand-in tray could not start".into());
                        log("stand-in tray failed to start");
                    }
                }
            }
            Err(e) => log(&format!("could not start the tray thread: {e}")),
        }
    } else if !want {
        r.blocked = None;
        if running {
            let h = r.hwnd;
            r.hwnd = 0;
            let t = r.thread.take();
            drop(r);
            stop(h, t);
        }
    }
}

/// Destroy the stand-in and wait (briefly) for its thread, which hands the
/// icons back to Explorer with a `TaskbarCreated` broadcast on its way out.
fn stop(hwnd: isize, thread: Option<std::thread::JoinHandle<()>>) {
    imp::close(hwnd);
    if let Some(t) = thread {
        let until = std::time::Instant::now() + std::time::Duration::from_millis(1500);
        while !t.is_finished() && std::time::Instant::now() < until {
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
    }
    if let Some(m) = MARKER.get() {
        let _ = std::fs::remove_file(m);
    }
    lock(&ICONS).clear();
    log("stand-in tray stopped; apps re-register with Windows' taskbar");
    changed();
}

/// Quit: stop the stand-in now.
pub fn shutdown() {
    sync(false);
}

/// PNG for `coveicon://.../tray/<id>.png`.
pub fn serve(path: &str) -> Option<Vec<u8>> {
    let id = path.trim_start_matches('/').split(['.', '?']).next()?;
    if let Some(p) = lock(&ICONS).iter().find(|i| i.id == id).and_then(|i| i.png.clone()) {
        return Some((*p).clone());
    }
    lock(&EXTRA_PNG).iter().find(|(k, _)| k == id).map(|(_, p)| (**p).clone())
}

fn icon_url(id: &str, rev: u32) -> String {
    let base = if cfg!(windows) { "http://coveicon.localhost" } else { "coveicon://localhost" };
    format!("{base}/tray/{id}.png?r={rev}")
}

/// One icon in Control Centre's "Other apps" grid.
#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub id: String,
    /// Short name (the tooltip's first line, or the app's name).
    pub name: String,
    /// The whole tooltip.
    pub tip: String,
    pub icon: Option<String>,
    /// "cove" (the stand-in) or "windows" (only Windows' tray has it).
    pub source: &'static str,
    /// Stable across restarts (GUID, or app + icon ID): used for pinning.
    pub key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Status {
    /// The setting.
    pub enabled: bool,
    /// "cove" (stand-in running), "windows" (only Windows' list), "off".
    pub mode: &'static str,
    /// Shown in Settings when something stops the stand-in.
    pub note: Option<String>,
    pub items: Vec<Item>,
    pub pins: Vec<String>,
}

/// Everything to show, newest state.
pub fn items() -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    let mut dead = false;
    {
        let icons = lock(&ICONS);
        for i in icons.iter() {
            if i.hidden {
                continue;
            }
            if !imp::window_alive(i.hwnd) {
                dead = true;
                continue;
            }
            let exe = imp::window_exe(i.hwnd);
            let mut name = first_line(&i.tip).to_string();
            if name.is_empty() {
                name = imp::app_name(&exe);
            }
            let key = i.guid.clone().unwrap_or_else(|| format!("{}#{}", exe.rsplit('\\').next().unwrap_or(&exe).to_lowercase(), i.uid));
            out.push(Item { id: i.id.clone(), name, tip: i.tip.clone(), icon: i.png.as_ref().map(|_| icon_url(&i.id, i.rev)), source: "cove", key });
        }
    }
    if dead {
        lock(&ICONS).retain(|i| imp::window_alive(i.hwnd));
    }
    // Windows' own list: name icons that came without a tooltip, and add the
    // ones the stand-in never heard about.
    for w in imp::windows_icons() {
        let lw = w.name.to_lowercase();
        if lw.is_empty() {
            continue;
        }
        let hit = out.iter().position(|o| {
            let n = o.name.to_lowercase();
            (!n.is_empty() && (lw == n || lw.starts_with(&n) || n.starts_with(first_line(&lw))))
                || o.key.starts_with('{') && w.automation_id.to_lowercase().contains(&o.key.to_lowercase())
        });
        match hit {
            Some(_) => {}
            None => {
                let id = format!("w{:x}", fnv(&format!("{}|{}", w.automation_id, w.name)));
                let icon = w.png.map(|p| {
                    let mut ex = lock(&EXTRA_PNG);
                    ex.retain(|(k, _)| k != &id);
                    ex.push((id.clone(), std::sync::Arc::new(p)));
                    icon_url(&id, 0)
                });
                out.push(Item { id: id.clone(), name: first_line(&w.name).to_string(), tip: w.name.clone(), icon, source: "windows", key: format!("w:{}", first_line(&w.name).to_lowercase()) });
            }
        }
    }
    out
}

fn fnv(s: &str) -> u64 {
    s.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100_0000_01b3))
}

/// Forward a click on an icon in Cove's grid to its app.
pub fn click(id: &str, how: Click) -> Result<(), String> {
    if let Some(name) = id.strip_prefix('w').and_then(|_| items().into_iter().find(|i| i.id == id && i.source == "windows")).map(|i| i.tip) {
        return imp::windows_click(&name, how);
    }
    let icon = lock(&ICONS).iter().find(|i| i.id == id).cloned().ok_or("That icon is gone.")?;
    if !imp::window_alive(icon.hwnd) {
        lock(&ICONS).retain(|i| i.id != id);
        changed();
        return Err("That app has quit.".into());
    }
    imp::send_click(&icon, how)
}

// ---- Tauri commands ----

fn status_from(enabled: bool, pins: Vec<String>, takeover: bool) -> Status {
    let (hwnd, blocked) = {
        let r = lock(&RUN);
        (r.hwnd, r.blocked.clone())
    };
    let mode = if hwnd != 0 {
        "cove"
    } else if enabled && takeover && cfg!(windows) {
        "windows"
    } else {
        "off"
    };
    let note = blocked.map(|b| {
        if b.starts_with("the stand-in") {
            "Cove couldn't start its tray, so it lists the icons Windows shows.".to_string()
        } else {
            format!("{b} also manages the tray, so Cove lists the icons Windows shows instead. Quit {b} to see every icon here.")
        }
    });
    Status { enabled, mode, note, items: if mode == "off" { Vec::new() } else { items() }, pins }
}

#[tauri::command]
pub async fn tray_status(app: tauri::AppHandle) -> Status {
    use tauri::Manager;
    let (enabled, pins, takeover) = app
        .try_state::<crate::AppState>()
        .and_then(|st| {
            st.settings
                .lock()
                .ok()
                .map(|s| (s.data.tray_icons, s.data.tray_pins.clone(), s.data.dock && (s.data.takeover || s.data.dock_hide_taskbar)))
        })
        .unwrap_or((true, Vec::new(), false));
    status_from(enabled, pins, takeover && !crate::safe())
}

#[tauri::command]
pub fn tray_set(enabled: bool, app: tauri::AppHandle, state: tauri::State<'_, crate::AppState>) -> Status {
    let (pins, takeover) = match state.settings.lock() {
        Ok(mut s) => {
            s.data.tray_icons = enabled;
            crate::sync_taskbar(&mut s);
            (s.data.tray_pins.clone(), s.data.dock && (s.data.takeover || s.data.dock_hide_taskbar))
        }
        Err(_) => (Vec::new(), false),
    };
    log(&format!("setting \"Show other apps' tray icons\" {}", if enabled { "on" } else { "off" }));
    changed();
    let _ = app;
    status_from(enabled, pins, takeover && !crate::safe())
}

/// Pin (or unpin) an icon to the dock's status island, by its stable key.
#[tauri::command]
pub fn tray_pin(key: String, pinned: bool, state: tauri::State<'_, crate::AppState>) -> Vec<String> {
    match state.settings.lock() {
        Ok(mut s) => {
            s.data.tray_pins.retain(|k| *k != key);
            if pinned {
                s.data.tray_pins.push(key);
            }
            let _ = s.save();
            changed();
            s.data.tray_pins.clone()
        }
        Err(_) => Vec::new(),
    }
}

#[tauri::command]
pub async fn tray_click(id: String, how: String) -> Result<(), String> {
    let how = Click::from_str(&how).ok_or("Unknown click.")?;
    click(&id, how)
}

/// Show Windows' own notification-area overflow for a moment.
#[tauri::command]
pub async fn tray_open_windows() -> Result<(), String> {
    log("opening Windows' own tray");
    imp::open_native()
}

/// An icon Windows' own tray shows (UI Automation), with the cached image
/// Windows keeps for it when there is one.
#[derive(Debug, Clone)]
pub struct WinIcon {
    pub name: String,
    pub automation_id: String,
    pub png: Option<Vec<u8>>,
}

#[cfg(windows)]
mod imp {
    use super::{lock, log, Click, Icon, WinIcon, CD_ICON_RECT, CD_NOTIFY};
    use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::System::DataExchange::COPYDATASTRUCT;
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    static OURS: AtomicIsize = AtomicIsize::new(0);
    static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);
    /// Where the last click happened (physical px): app menus that ask
    /// where their icon is get this, so they open next to Cove's grid.
    static LAST_CLICK: std::sync::Mutex<Option<(i32, i32)>> = std::sync::Mutex::new(None);

    fn taskbar_created() -> u32 {
        let m = TASKBAR_CREATED.load(Ordering::SeqCst);
        if m != 0 {
            return m;
        }
        let w = wide("TaskbarCreated");
        let m = unsafe { RegisterWindowMessageW(w.as_ptr()) };
        TASKBAR_CREATED.store(m, Ordering::SeqCst);
        m
    }

    pub fn broadcast_taskbar_created() {
        unsafe { SendNotifyMessageW(HWND_BROADCAST, taskbar_created(), 0, 0) };
    }

    fn pid_of(h: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        pid
    }

    fn exe_of_pid(pid: u32) -> String {
        unsafe {
            let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if p.is_null() {
                return String::new();
            }
            let mut buf = [0u16; 1024];
            let mut n = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n);
            CloseHandle(p);
            if ok == 0 {
                String::new()
            } else {
                String::from_utf16_lossy(&buf[..n as usize])
            }
        }
    }

    pub fn window_alive(h: u64) -> bool {
        unsafe { IsWindow(h as usize as HWND) != 0 }
    }

    pub fn window_exe(h: u64) -> String {
        exe_of_pid(pid_of(h as usize as HWND))
    }

    pub fn app_name(exe: &str) -> String {
        if exe.is_empty() {
            return "App".into();
        }
        crate::appnames::friendly(exe, None, "")
    }

    /// Every top-level `Shell_TrayWnd` that isn't Cove's.
    fn other_trays() -> Vec<HWND> {
        let cls = wide("Shell_TrayWnd");
        let me = unsafe { GetCurrentProcessId() };
        let mut out = Vec::new();
        let mut after: HWND = std::ptr::null_mut();
        for _ in 0..16 {
            after = unsafe { FindWindowExW(std::ptr::null_mut(), after, cls.as_ptr(), std::ptr::null()) };
            if after.is_null() {
                break;
            }
            if pid_of(after) != me {
                out.push(after);
            }
        }
        out
    }

    /// Explorer's own taskbar (never Cove's stand-in, never another tool's).
    pub fn explorer_tray() -> Option<HWND> {
        other_trays().into_iter().find(|&h| exe_of_pid(pid_of(h)).to_lowercase().ends_with("\\explorer.exe"))
    }

    /// Another tool that replaces or patches the tray, by name.
    pub fn conflict() -> Option<String> {
        // A second Shell_TrayWnd that isn't Explorer's: another stand-in.
        for h in other_trays() {
            let exe = exe_of_pid(pid_of(h));
            if !exe.to_lowercase().ends_with("\\explorer.exe") {
                let n = super::conflict_name(&exe).map(String::from).unwrap_or_else(|| {
                    let f = exe.rsplit('\\').next().unwrap_or("another app").to_string();
                    if f.is_empty() { "Another app".into() } else { f }
                });
                return Some(n);
            }
        }
        if let Some(n) = processes().iter().find_map(|p| super::conflict_name(p)) {
            return Some(n.into());
        }
        // ExplorerPatcher runs inside Explorer: look for its install.
        let pf = std::env::var("ProgramFiles").unwrap_or_else(|_| "C:\\Program Files".into());
        if std::path::Path::new(&pf).join("ExplorerPatcher").join("ExplorerPatcher.amd64.dll").exists() {
            return Some("ExplorerPatcher".into());
        }
        None
    }

    fn processes() -> Vec<String> {
        use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
        let mut out = Vec::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap.is_null() || snap == windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE {
                return out;
            }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e);
            while ok != 0 {
                let n = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                out.push(String::from_utf16_lossy(&e.szExeFile[..n]));
                ok = Process32NextW(snap, &mut e);
            }
            CloseHandle(snap);
        }
        out
    }

    /// First in line among `Shell_TrayWnd`s, and the same rectangle as
    /// Explorer's taskbar (still hidden), so an app that measures "the
    /// taskbar" through FindWindow gets the real answer.
    fn to_top(h: HWND) {
        let mut r = windows_sys::Win32::Foundation::RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let has = explorer_tray().is_some_and(|t| unsafe { GetWindowRect(t, &mut r) } != 0);
        let flags = SWP_NOACTIVATE | if has { 0 } else { SWP_NOMOVE | SWP_NOSIZE };
        unsafe { SetWindowPos(h, HWND_TOPMOST, r.left, r.top, r.right - r.left, r.bottom - r.top, flags) };
    }

    /// The stand-in's thread: create it, announce it, pump messages until
    /// it's closed, then hand the icons back to Explorer.
    pub fn run(tx: std::sync::mpsc::Sender<isize>) {
        let cls = wide("Shell_TrayWnd");
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let wc = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: 0,
                lpfnWndProc: Some(proc_),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: inst,
                hIcon: std::ptr::null_mut(),
                hCursor: std::ptr::null_mut(),
                hbrBackground: std::ptr::null_mut(),
                lpszMenuName: std::ptr::null(),
                lpszClassName: cls.as_ptr(),
                hIconSm: std::ptr::null_mut(),
            };
            // Registered once per process; a second start reuses it.
            RegisterClassExW(&wc);
            let h = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                cls.as_ptr(),
                std::ptr::null(),
                WS_POPUP,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                inst,
                std::ptr::null(),
            );
            if h.is_null() {
                let _ = tx.send(0);
                return;
            }
            OURS.store(h as isize, Ordering::SeqCst);
            // Elevated apps must still reach it (UIPI), or their icons would
            // never get to Explorer either.
            ChangeWindowMessageFilterEx(h, WM_COPYDATA, MSGFLT_ALLOW, std::ptr::null_mut());
            ChangeWindowMessageFilterEx(h, taskbar_created(), MSGFLT_ALLOW, std::ptr::null_mut());
            to_top(h);
            // Stay first in line: checked twice a second, re-raised only
            // when something got above it.
            SetTimer(h, 1, 500, None);
            let _ = tx.send(h as isize);
            broadcast_taskbar_created();
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            OURS.store(0, Ordering::SeqCst);
            UnregisterClassW(cls.as_ptr(), inst);
        }
        // Our window is gone: apps re-add their icons to whatever tray is
        // there now (Explorer's), which already has them.
        broadcast_taskbar_created();
    }

    pub fn close(h: isize) {
        if h != 0 {
            unsafe { PostMessageW(h as HWND, WM_CLOSE, 0, 0) };
        }
    }

    fn forward(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM, fallback: LRESULT) -> LRESULT {
        let Some(real) = explorer_tray().or_else(|| other_trays().into_iter().next()) else {
            return fallback;
        };
        let _ = h;
        let mut out: usize = 0;
        let ok = unsafe { SendMessageTimeoutW(real, msg, wp, lp, SMTO_ABORTIFHUNG | SMTO_NORMAL, 4000, &mut out) };
        if ok == 0 {
            fallback
        } else {
            out as LRESULT
        }
    }

    static TICKS: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn proc_(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        match msg {
            WM_COPYDATA => {
                let cds = lp as *const COPYDATASTRUCT;
                if cds.is_null() {
                    return 0;
                }
                let (kind, len, ptr) = unsafe { ((*cds).dwData, (*cds).cbData as usize, (*cds).lpData as *const u8) };
                let bytes: &[u8] = if ptr.is_null() || len == 0 || len > 64 * 1024 { &[] } else { unsafe { std::slice::from_raw_parts(ptr, len) } };
                match kind {
                    CD_NOTIFY => {
                        if let Some(n) = super::parse(bytes) {
                            let me = unsafe { GetCurrentProcessId() };
                            // Cove's own tray icon stays out of "Other apps".
                            if pid_of(n.hwnd as usize as HWND) != me {
                                let ch = {
                                    let mut icons = lock(&super::ICONS);
                                    let owner = window_exe(n.hwnd);
                                    super::apply(&mut icons, &n, &mut |hi| crate::icons::tray_icon_png(hi as isize, &owner))
                                };
                                if ch {
                                    super::changed();
                                }
                            }
                        }
                        forward(h, msg, wp, lp, 1)
                    }
                    CD_ICON_RECT => {
                        // NOTIFYICONIDENTIFIER: signature, message, ...
                        let m = super::rd(bytes, 4, 4) as u32;
                        let (x, y) = lock(&LAST_CLICK).unwrap_or_else(cursor);
                        super::icon_rect_reply(m, (x - 10, y - 10, x + 10, y + 10))
                    }
                    _ => forward(h, msg, wp, lp, 0),
                }
            }
            WM_TIMER => {
                let first = unsafe { FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null()) };
                if first != h {
                    to_top(h);
                }
                super::flush();
                if TICKS.fetch_add(1, Ordering::SeqCst) % 20 == 19 {
                    let before = lock(&super::ICONS).len();
                    lock(&super::ICONS).retain(|i| window_alive(i.hwnd));
                    if lock(&super::ICONS).len() != before {
                        super::changed();
                    }
                }
                0
            }
            WM_CLOSE => {
                unsafe { DestroyWindow(h) };
                0
            }
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                0
            }
            // Explorer restarted (or our own announcement): stay first.
            m if m == taskbar_created() && m != 0 => {
                to_top(h);
                0
            }
            // The taskbar's own commands and private messages go to the
            // real one. Registered (broadcast) messages don't: Explorer got
            // them already.
            WM_COMMAND => forward(h, msg, wp, lp, 0),
            m if (WM_USER..0xC000).contains(&m) => forward(h, msg, wp, lp, 0),
            _ => unsafe { DefWindowProcW(h, msg, wp, lp) },
        }
    }

    fn cursor() -> (i32, i32) {
        let mut p = POINT { x: 0, y: 0 };
        unsafe { GetCursorPos(&mut p) };
        (p.x, p.y)
    }

    pub fn send_click(icon: &Icon, how: Click) -> Result<(), String> {
        if icon.callback == 0 {
            return Err("That icon doesn't take clicks.".into());
        }
        let hwnd = icon.hwnd as usize as HWND;
        let (x, y) = cursor();
        *lock(&LAST_CLICK) = Some((x, y));
        unsafe {
            // Let the app bring its menu or window to the front.
            AllowSetForegroundWindow(pid_of(hwnd));
            for (wp, lp) in super::click_messages(icon.version, icon.uid, how, x, y) {
                SendNotifyMessageW(hwnd, icon.callback, wp, lp);
            }
        }
        Ok(())
    }

    // ---- UI Automation: the icons Windows' own tray shows ----

    static WIN_CACHE: std::sync::Mutex<Option<(std::time::Instant, Vec<WinIcon>)>> = std::sync::Mutex::new(None);

    fn with_uia<T>(f: impl FnOnce(&windows::Win32::UI::Accessibility::IUIAutomation) -> T) -> Option<T> {
        use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
        use windows::Win32::UI::Accessibility::{CUIAutomation, IUIAutomation};
        unsafe {
            let init = CoInitializeEx(None, COINIT_MULTITHREADED).is_ok();
            let out = CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok().map(|u| f(&u));
            if init {
                CoUninitialize();
            }
            out
        }
    }

    /// Notification icons under Windows' taskbar and its overflow: on
    /// Windows 11 they are `SystemTray.NormalButton`s whose AutomationId
    /// isn't "SystemTrayIcon" (that's the chevron, clock, volume...); on
    /// Windows 10 the buttons of the "Notification Area" toolbars.
    fn uia_buttons(u: &windows::Win32::UI::Accessibility::IUIAutomation) -> Vec<(String, String, windows::Win32::UI::Accessibility::IUIAutomationElement)> {
        use windows::Win32::UI::Accessibility::TreeScope_Descendants;
        let mut roots: Vec<HWND> = Vec::new();
        if let Some(t) = explorer_tray() {
            roots.push(t);
        }
        for c in ["TopLevelWindowForOverflowXamlIsland", "NotifyIconOverflowWindow"] {
            let h = unsafe { FindWindowW(wide(c).as_ptr(), std::ptr::null()) };
            if !h.is_null() {
                roots.push(h);
            }
        }
        let mut out = Vec::new();
        let Ok(cond) = (unsafe { u.CreateTrueCondition() }) else { return out };
        for r in roots {
            let Ok(el) = (unsafe { u.ElementFromHandle(windows::Win32::Foundation::HWND(r as _)) }) else { continue };
            let Ok(all) = (unsafe { el.FindAll(TreeScope_Descendants, &cond) }) else { continue };
            let n = unsafe { all.Length() }.unwrap_or(0).min(400);
            let mut parent_toolbar = String::new();
            for i in 0..n {
                let Ok(e) = (unsafe { all.GetElement(i) }) else { continue };
                let class = unsafe { e.CurrentClassName() }.map(|b| b.to_string()).unwrap_or_default();
                let aid = unsafe { e.CurrentAutomationId() }.map(|b| b.to_string()).unwrap_or_default();
                let name = unsafe { e.CurrentName() }.map(|b| b.to_string()).unwrap_or_default();
                if class == "ToolbarWindow32" {
                    parent_toolbar = name.clone();
                    continue;
                }
                let win11 = class == "SystemTray.NormalButton" && aid != "SystemTrayIcon";
                let win10 = class.is_empty() && parent_toolbar.ends_with("Notification Area") && !parent_toolbar.starts_with("System");
                if (win11 || win10) && !name.trim().is_empty() {
                    out.push((name, aid, e));
                }
            }
        }
        out
    }

    /// Windows keeps a PNG of each icon it has shown, with its first
    /// tooltip, under HKCU\Control Panel\NotifyIconSettings.
    fn snapshots() -> Vec<(String, Vec<u8>)> {
        use winreg::enums::HKEY_CURRENT_USER;
        let mut out = Vec::new();
        let Ok(root) = winreg::RegKey::predef(HKEY_CURRENT_USER).open_subkey("Control Panel\\NotifyIconSettings") else { return out };
        for k in root.enum_keys().flatten().take(400) {
            let Ok(sub) = root.open_subkey(&k) else { continue };
            let tip: String = sub.get_value("InitialTooltip").unwrap_or_default();
            if tip.is_empty() {
                continue;
            }
            if let Ok(v) = sub.get_raw_value("IconSnapshot") {
                if v.bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
                    out.push((tip, v.bytes));
                }
            }
        }
        out
    }

    pub fn windows_icons() -> Vec<WinIcon> {
        if let Some((t, v)) = lock(&WIN_CACHE).as_ref() {
            if t.elapsed() < std::time::Duration::from_secs(10) {
                return v.clone();
            }
        }
        let names: Vec<(String, String)> = with_uia(|u| uia_buttons(u).into_iter().map(|(n, a, _)| (n, a)).collect()).unwrap_or_default();
        let snaps = if names.is_empty() { Vec::new() } else { snapshots() };
        let v: Vec<WinIcon> = names
            .into_iter()
            .map(|(name, automation_id)| {
                let ln = super::first_line(&name).to_lowercase();
                let png = snaps.iter().find(|(t, _)| {
                    let lt = super::first_line(t).to_lowercase();
                    !lt.is_empty() && (lt == ln || ln.starts_with(&lt) || lt.starts_with(&ln))
                }).map(|(_, p)| p.clone());
                WinIcon { name, automation_id, png }
            })
            .collect();
        if !v.is_empty() {
            log(&format!("Windows' tray lists {} icon(s)", v.len()));
        }
        *lock(&WIN_CACHE) = Some((std::time::Instant::now(), v.clone()));
        v
    }

    /// Click an icon that only Windows' tray has, through UI Automation
    /// (Invoke for a left click, ShowContextMenu for a right click).
    pub fn windows_click(name: &str, how: Click) -> Result<(), String> {
        use windows::core::Interface;
        use windows::Win32::UI::Accessibility::{IUIAutomationElement3, IUIAutomationInvokePattern, UIA_InvokePatternId};
        let r = with_uia(|u| {
            let Some((_, _, e)) = uia_buttons(u).into_iter().find(|(n, _, _)| n == name) else {
                return Err("Windows' tray no longer shows that icon.".to_string());
            };
            unsafe {
                match how {
                    Click::Right => e.cast::<IUIAutomationElement3>().and_then(|e3| e3.ShowContextMenu()).map_err(|e| e.to_string()),
                    _ => e.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId).and_then(|p| p.Invoke()).map_err(|e| e.to_string()),
                }
            }
        });
        r.unwrap_or_else(|| Err("UI Automation isn't available.".into()))
    }

    // ---- Windows' own overflow, shown for a moment ----

    fn overflow_visible() -> bool {
        ["TopLevelWindowForOverflowXamlIsland", "NotifyIconOverflowWindow"].iter().any(|c| unsafe {
            let h = FindWindowW(wide(c).as_ptr(), std::ptr::null());
            !h.is_null() && IsWindowVisible(h) != 0
        })
    }

    fn class_of(h: HWND) -> String {
        let mut b = [0u16; 128];
        let n = unsafe { GetClassNameW(h, b.as_mut_ptr(), b.len() as i32) };
        String::from_utf16_lossy(&b[..n.max(0) as usize])
    }

    pub fn open_native() -> Result<(), String> {
        use std::sync::atomic::Ordering::SeqCst;
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_LWIN, VK_RETURN};
        if super::NATIVE_OPEN.swap(true, SeqCst) {
            return Ok(());
        }
        let tray = explorer_tray().ok_or_else(|| {
            super::NATIVE_OPEN.store(false, SeqCst);
            "Windows' taskbar isn't running.".to_string()
        })?;
        unsafe { ShowWindow(tray, SW_SHOWNOACTIVATE) };
        std::thread::sleep(std::time::Duration::from_millis(250));
        // The chevron ("Show hidden icons"): language-independent match.
        let invoked = with_uia(|u| {
            use windows::Win32::UI::Accessibility::{IUIAutomationInvokePattern, TreeScope_Descendants, UIA_InvokePatternId};
            let Ok(cond) = (unsafe { u.CreateTrueCondition() }) else { return false };
            let Ok(el) = (unsafe { u.ElementFromHandle(windows::Win32::Foundation::HWND(tray as _)) }) else { return false };
            let Ok(all) = (unsafe { el.FindAll(TreeScope_Descendants, &cond) }) else { return false };
            let n = unsafe { all.Length() }.unwrap_or(0).min(400);
            for i in 0..n {
                let Ok(e) = (unsafe { all.GetElement(i) }) else { continue };
                let class = unsafe { e.CurrentClassName() }.map(|b| b.to_string()).unwrap_or_default();
                let aid = unsafe { e.CurrentAutomationId() }.map(|b| b.to_string()).unwrap_or_default();
                let name = unsafe { e.CurrentName() }.map(|b| b.to_string()).unwrap_or_default();
                if (class == "SystemTray.NormalButton" && aid == "SystemTrayIcon") || name == "Notification Chevron" {
                    return unsafe { e.GetCurrentPatternAs::<IUIAutomationInvokePattern>(UIA_InvokePatternId).and_then(|p| p.Invoke()) }.is_ok();
                }
            }
            false
        })
        .unwrap_or(false);
        if !invoked {
            // Winstep's fallback: Win+B focuses the notification area, Enter opens it.
            unsafe {
                SetForegroundWindow(tray);
                keybd_event(VK_LWIN as u8, 0, 0, 0);
                keybd_event(b'B', 0, 0, 0);
                keybd_event(b'B', 0, KEYEVENTF_KEYUP, 0);
                keybd_event(VK_LWIN as u8, 0, KEYEVENTF_KEYUP, 0);
                std::thread::sleep(std::time::Duration::from_millis(120));
                keybd_event(VK_RETURN as u8, 0, 0, 0);
                keybd_event(VK_RETURN as u8, 0, KEYEVENTF_KEYUP, 0);
            }
        }
        log(&format!("Windows' tray opened ({})", if invoked { "chevron" } else { "Win+B" }));
        // Hide the taskbar again once the user is done with it (the overflow
        // or a menu from it closed), at most 90 s later.
        std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut idle = 0;
            while start.elapsed() < std::time::Duration::from_secs(90) {
                std::thread::sleep(std::time::Duration::from_millis(250));
                let fg = unsafe { GetForegroundWindow() };
                let fc = class_of(fg);
                let busy = overflow_visible() || matches!(fc.as_str(), "Shell_TrayWnd" | "TopLevelWindowForOverflowXamlIsland" | "NotifyIconOverflowWindow" | "#32768");
                if start.elapsed() < std::time::Duration::from_millis(1500) || busy {
                    idle = 0;
                } else {
                    idle += 1;
                    if idle >= 3 {
                        break;
                    }
                }
            }
            super::NATIVE_OPEN.store(false, std::sync::atomic::Ordering::SeqCst);
            if crate::dock::TASKBAR_GONE.load(std::sync::atomic::Ordering::SeqCst) {
                crate::dock::taskbar_visible(false);
            }
        });
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Click, Icon, WinIcon};
    pub fn broadcast_taskbar_created() {}
    pub fn conflict() -> Option<String> {
        None
    }
    pub fn run(tx: std::sync::mpsc::Sender<isize>) {
        let _ = tx.send(0);
    }
    pub fn close(_h: isize) {}
    pub fn window_alive(_h: u64) -> bool {
        true
    }
    pub fn window_exe(_h: u64) -> String {
        String::new()
    }
    pub fn app_name(_exe: &str) -> String {
        "App".into()
    }
    pub fn windows_icons() -> Vec<WinIcon> {
        Vec::new()
    }
    pub fn windows_click(_name: &str, _how: Click) -> Result<(), String> {
        Err("Not on Windows.".into())
    }
    pub fn send_click(_icon: &Icon, _how: Click) -> Result<(), String> {
        Err("Not on Windows.".into())
    }
    pub fn open_native() -> Result<(), String> {
        Err("Not on Windows.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut [u8], off: usize, v: u64, n: usize) {
        for i in 0..n {
            b[off + i] = (v >> (8 * i)) as u8;
        }
    }

    fn put_tip(b: &mut [u8], off: usize, s: &str) {
        for (i, c) in s.encode_utf16().enumerate() {
            put(b, off + i * 2, c as u64, 2);
        }
    }

    /// A payload as shell32 sends it (32-bit handles), for 32- and 64-bit apps alike.
    fn msg32(message: u32, hwnd: u32, uid: u32, flags: u32, cb: u32, hicon: u32, tip: &str) -> Vec<u8> {
        let mut b = vec![0u8; 8 + 956];
        put(&mut b, 0, TRAY_SIGNATURE as u64, 4);
        put(&mut b, 4, message as u64, 4);
        let n = 8;
        put(&mut b, n, 956, 4);
        put(&mut b, n + 4, hwnd as u64, 4);
        put(&mut b, n + 8, uid as u64, 4);
        put(&mut b, n + 12, flags as u64, 4);
        put(&mut b, n + 16, cb as u64, 4);
        put(&mut b, n + 20, hicon as u64, 4);
        put_tip(&mut b, n + 24, tip);
        b
    }

    /// The native x64 NOTIFYICONDATAW layout (8-byte handles).
    fn msg64(message: u32, hwnd: u64, uid: u32, flags: u32, cb: u32, hicon: u64, tip: &str) -> Vec<u8> {
        let mut b = vec![0u8; 8 + 976];
        put(&mut b, 0, TRAY_SIGNATURE as u64, 4);
        put(&mut b, 4, message as u64, 4);
        let n = 8;
        put(&mut b, n, 976, 4);
        put(&mut b, n + 8, hwnd, 8);
        put(&mut b, n + 16, uid as u64, 4);
        put(&mut b, n + 20, flags as u64, 4);
        put(&mut b, n + 24, cb as u64, 4);
        put(&mut b, n + 32, hicon, 8);
        put_tip(&mut b, n + 40, tip);
        b
    }

    #[test]
    fn parses_32bit_handle_layout() {
        let b = msg32(NIM_ADD, 0x0012_34AB, 7, NIF_MESSAGE | NIF_ICON | NIF_TIP, 0x8001, 0x0001_0F2C, "Discord\r\n3 unread");
        let n = parse(&b).unwrap();
        assert!(!n.wide);
        assert_eq!((n.message, n.hwnd, n.uid, n.callback, n.hicon), (NIM_ADD, 0x0012_34AB, 7, 0x8001, 0x0001_0F2C));
        assert_eq!(n.tip.as_deref(), Some("Discord\n3 unread"));
        assert_eq!(n.guid, None);
    }

    #[test]
    fn parses_native_64bit_layout() {
        let b = msg64(NIM_MODIFY, 0x0000_7FF6_1234_5678, 3, NIF_ICON | NIF_TIP, 0, 0x0000_0001_0002_0003, "Steam");
        let n = parse(&b).unwrap();
        assert!(n.wide);
        assert_eq!((n.message, n.hwnd, n.uid, n.hicon), (NIM_MODIFY, 0x0000_7FF6_1234_5678, 3, 0x0000_0001_0002_0003));
        assert_eq!(n.tip.as_deref(), Some("Steam"));
    }

    #[test]
    fn reads_version_state_and_guid() {
        let mut b = msg32(NIM_SETVERSION, 0x10, 1, NIF_GUID | NIF_STATE, 0, 0, "");
        put(&mut b, 8 + 800, 4, 4); // uVersion
        put(&mut b, 8 + 280, NIS_HIDDEN as u64, 4);
        put(&mut b, 8 + 284, NIS_HIDDEN as u64, 4);
        let g: [u8; 16] = [0x78, 0x56, 0x34, 0x12, 0xBC, 0x9A, 0xF0, 0xDE, 1, 2, 3, 4, 5, 6, 7, 8];
        b[8 + 936..8 + 952].copy_from_slice(&g);
        let n = parse(&b).unwrap();
        assert_eq!(n.version, 4);
        assert_eq!((n.state, n.state_mask), (NIS_HIDDEN, NIS_HIDDEN));
        assert_eq!(n.guid.as_deref(), Some("{12345678-9ABC-DEF0-0102-030405060708}"));
        assert_eq!(n.tip, None, "no NIF_TIP, no tooltip");
    }

    #[test]
    fn rejects_bad_or_short_messages_without_panicking() {
        assert!(parse(&[]).is_none());
        assert!(parse(&[0u8; 40]).is_none(), "wrong signature");
        let b = msg32(NIM_ADD, 1, 1, NIF_TIP, 0, 0, "Tip that gets cut off");
        // Truncated mid-tooltip: still parses, never reads past the end.
        let n = parse(&b[..8 + 40]).unwrap();
        assert_eq!(n.tip.as_deref(), Some("Tip that"));
        // A tooltip with no terminator stops at 128 characters.
        let mut b = msg32(NIM_ADD, 1, 1, NIF_TIP, 0, 0, "");
        for i in 0..200 {
            if 24 + i * 2 + 1 < 956 {
                put(&mut b, 8 + 24 + i * 2, b'x' as u64, 2);
            }
        }
        assert_eq!(parse(&b).unwrap().tip.unwrap().len(), 128);
    }

    #[test]
    fn version_4_click_mapping() {
        // Left: down, up, then NIN_SELECT; wParam = anchor x/y, lParam = event | uid << 16.
        let m = click_messages(4, 9, Click::Left, 1200, 700);
        let wp = (1200u32 | (700u32 << 16)) as usize;
        assert_eq!(m, vec![(wp, (WM_LBUTTONDOWN | 9 << 16) as isize), (wp, (WM_LBUTTONUP | 9 << 16) as isize), (wp, (NIN_SELECT | 9 << 16) as isize)]);
        // Right ends with WM_CONTEXTMENU.
        let m = click_messages(4, 2, Click::Right, 10, 20);
        assert_eq!(m.last().unwrap().1, (WM_CONTEXTMENU | 2 << 16) as isize);
        assert_eq!(m.len(), 3);
        // Double click: no extra select.
        let m = click_messages(4, 2, Click::Double, 10, 20);
        assert_eq!(m.iter().map(|(_, l)| (*l as u32) & 0xFFFF).collect::<Vec<_>>(), vec![WM_LBUTTONDBLCLK, WM_LBUTTONUP]);
        // Negative coordinates (a screen left of the main one) stay in their words.
        let m = click_messages(4, 1, Click::Left, -100, 5);
        assert_eq!(m[0].0 & 0xFFFF, (-100i16) as u16 as usize);
    }

    #[test]
    fn older_version_click_mapping() {
        // Version 0: wParam = uid, lParam = the mouse message; no NIN_SELECT.
        assert_eq!(click_messages(0, 5, Click::Left, 1, 1), vec![(5, WM_LBUTTONDOWN as isize), (5, WM_LBUTTONUP as isize)]);
        // Version 3: same encoding, plus NIN_SELECT / WM_CONTEXTMENU.
        assert_eq!(click_messages(3, 5, Click::Right, 1, 1).last(), Some(&(5, WM_CONTEXTMENU as isize)));
        assert_eq!(click_messages(3, 5, Click::Left, 1, 1).last(), Some(&(5, NIN_SELECT as isize)));
    }

    #[test]
    fn icon_rect_reply_packs_points() {
        assert_eq!(icon_rect_reply(1, (100, 200, 120, 220)), (100 | (200 << 16)) as isize);
        assert_eq!(icon_rect_reply(2, (100, 200, 120, 220)), (120 | (220 << 16)) as isize);
        assert_eq!(icon_rect_reply(9, (1, 2, 3, 4)), 0);
    }

    #[test]
    fn add_modify_delete_lifecycle() {
        let mut icons = Vec::new();
        let mut png = |h: u64| Some(vec![h as u8]);
        let add = parse(&msg32(NIM_ADD, 0x40, 1, NIF_MESSAGE | NIF_ICON | NIF_TIP, 0x8010, 0xA1, "OneDrive")).unwrap();
        assert!(apply(&mut icons, &add, &mut png));
        assert_eq!(icons.len(), 1);
        assert_eq!((icons[0].callback, icons[0].tip.as_str(), icons[0].id.as_str()), (0x8010, "OneDrive", "h40-1"));
        // Set version 4.
        let mut v = msg32(NIM_SETVERSION, 0x40, 1, 0, 0, 0, "");
        put(&mut v, 8 + 800, 4, 4);
        apply(&mut icons, &parse(&v).unwrap(), &mut png);
        assert_eq!(icons[0].version, 4);
        // New tooltip and icon.
        let m = parse(&msg32(NIM_MODIFY, 0x40, 1, NIF_ICON | NIF_TIP, 0, 0xA2, "OneDrive - Syncing")).unwrap();
        assert!(apply(&mut icons, &m, &mut png));
        assert_eq!((icons[0].tip.as_str(), icons[0].rev, icons[0].callback), ("OneDrive - Syncing", 2, 0x8010));
        // Same again: nothing changes.
        assert!(!apply(&mut icons, &m, &mut png));
        // Hidden via state.
        let mut h = msg32(NIM_MODIFY, 0x40, 1, NIF_STATE, 0, 0, "");
        put(&mut h, 8 + 280, 1, 4);
        put(&mut h, 8 + 284, 1, 4);
        assert!(apply(&mut icons, &parse(&h).unwrap(), &mut png));
        assert!(icons[0].hidden);
        // A modify for an unknown icon without an image is ignored; with one it's added.
        assert!(!apply(&mut icons, &parse(&msg32(NIM_MODIFY, 0x50, 2, NIF_TIP, 0, 0, "x")).unwrap(), &mut png));
        assert!(apply(&mut icons, &parse(&msg32(NIM_MODIFY, 0x50, 2, NIF_ICON, 0, 0xB0, "")).unwrap(), &mut png));
        assert_eq!(icons.len(), 2);
        // Delete.
        assert!(apply(&mut icons, &parse(&msg32(NIM_DELETE, 0x40, 1, 0, 0, 0, "")).unwrap(), &mut png));
        assert_eq!(icons.len(), 1);
        assert_eq!(icons[0].id, "h50-2");
    }

    #[test]
    fn guid_icons_match_by_guid() {
        let mut icons = Vec::new();
        let mut png = |_h: u64| Some(vec![1]);
        let mut a = msg32(NIM_ADD, 0x40, 1, NIF_ICON | NIF_GUID, 0, 0xA1, "");
        a[8 + 936] = 0xAB;
        assert!(apply(&mut icons, &parse(&a).unwrap(), &mut png));
        assert_eq!(icons[0].id, "g000000ab-0000-0000-0000-000000000000");
        // Delete by GUID, from another window handle.
        let mut d = msg32(NIM_DELETE, 0x99, 5, NIF_GUID, 0, 0, "");
        d[8 + 936] = 0xAB;
        assert!(apply(&mut icons, &parse(&d).unwrap(), &mut png));
        assert!(icons.is_empty());
    }

    #[test]
    fn names_conflicting_tools() {
        assert_eq!(conflict_name("C:\\Program Files\\YASB\\yasb.exe"), Some("YASB"));
        assert_eq!(conflict_name("TranslucentTB.exe"), Some("TranslucentTB"));
        assert_eq!(conflict_name("seelen-ui.exe"), Some("Seelen UI"));
        assert_eq!(conflict_name("zebar.exe"), Some("Zebar"));
        assert_eq!(conflict_name("explorer.exe"), None);
        assert_eq!(conflict_name("Discord.exe"), None);
    }

    #[test]
    fn first_line_of_tooltip() {
        assert_eq!(first_line("Discord\n3 unread"), "Discord");
        assert_eq!(first_line("\n  Steam "), "Steam");
        assert_eq!(first_line(""), "");
    }
}
