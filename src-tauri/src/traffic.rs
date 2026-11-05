//! Mac window buttons ("traffic lights") on the focused window.
//!
//! 2026-11-05 (user's test of b44d0c1: "the three traffic signals do not
//! appear and the windows style app header was there"): the dots are now
//! always shown on the focused window's title bar, and they REPLACE the
//! native Windows buttons: one small window of Cove's own covers the
//! native minimise / maximise / close buttons exactly, painted in the title
//! bar's own colour (read from the screen beside the buttons), with red /
//! yellow / green at its right end. A click becomes the window's own
//! `WM_SYSCOMMAND`; resting on green (or Alt+click) opens Snap Layouts with
//! Win+Z. Background windows keep their native buttons.
//!
//! How (see notes/traffic-lights-research.md):
//! - Watching uses out-of-context WinEvents only (`SetWinEventHook` with
//!   `WINEVENT_OUTOFCONTEXT`): nothing is loaded into other processes, no
//!   `SetWindowsHookEx`, no drivers, nothing written to the system. The
//!   native buttons are only covered, never changed: Narrator, Alt+F4 and
//!   the app itself see them as before.
//! - Standard frames are measured with DWM (`DWMWA_CAPTION_BUTTON_BOUNDS` +
//!   `DWMWA_EXTENDED_FRAME_BOUNDS`). Custom title bars (File Explorer,
//!   Chrome, Edge, VS Code first, then any app) are measured with UI
//!   Automation once when the window gains focus (and after a resize
//!   settles), never during a drag: the app's own caption buttons at the
//!   top-right corner, or the window is skipped.
//! - Anything uncertain is skipped: elevated windows, fullscreen, cloaked,
//!   tool windows, dialogs without min/max, hung windows, High Contrast,
//!   Cove's own windows. The patch hides while the window moves, and
//!   whenever another window lies over the buttons.
//! - The overlay (`traffic.html`) is no-activate and topmost; turning the
//!   setting off or quitting Cove destroys it.

use serde::Serialize;

/// Pure placement geometry (no Windows calls), unit-tested on every OS.
pub mod geom {
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct Rect {
        pub l: i32,
        pub t: i32,
        pub r: i32,
        pub b: i32,
    }

    impl Rect {
        pub const fn new(l: i32, t: i32, r: i32, b: i32) -> Self {
            Rect { l, t, r, b }
        }
        pub fn w(&self) -> i32 {
            self.r - self.l
        }
        pub fn h(&self) -> i32 {
            self.b - self.t
        }
        pub fn is_empty(&self) -> bool {
            self.r <= self.l || self.b <= self.t
        }
        pub fn contains(&self, x: i32, y: i32) -> bool {
            x >= self.l && x < self.r && y >= self.t && y < self.b
        }
        pub fn intersects(&self, o: &Rect) -> bool {
            self.l < o.r && o.l < self.r && self.t < o.b && o.t < self.b
        }
        pub fn inflate(&self, dx: i32, dy: i32) -> Rect {
            Rect { l: self.l - dx, t: self.t - dy, r: self.r + dx, b: self.b + dy }
        }
        pub fn offset(&self, dx: i32, dy: i32) -> Rect {
            Rect { l: self.l + dx, t: self.t + dy, r: self.r + dx, b: self.b + dy }
        }
        pub fn center(&self) -> (i32, i32) {
            ((self.l + self.r) / 2, (self.t + self.b) / 2)
        }
    }

    /// Dot diameter, gap between dots, inset from the frame's left edge to
    /// the first dot, and the transparent padding around the dots inside
    /// the overlay window (room for the ring and the pressed shade). DIP.
    pub const DOT: f64 = 12.0;
    pub const GAP: f64 = 8.0;
    pub const INSET: f64 = 13.0;
    pub const PAD: f64 = 2.0;
    /// Clear space required between the dots and anything clickable.
    pub const CLEAR: f64 = 8.0;
    /// The hover zone reaches this far from the frame's left edge.
    pub const ZONE_W: f64 = 140.0;

    pub fn dip(v: f64, scale: f64) -> i32 {
        (v * scale).round() as i32
    }

    /// Overlay window size in physical pixels: three dots, two gaps, padding.
    pub fn cluster_size(scale: f64) -> (i32, i32) {
        (dip(3.0 * DOT + 2.0 * GAP + 2.0 * PAD, scale), dip(DOT + 2.0 * PAD, scale))
    }

    /// Centre and radius of each dot inside the overlay window (physical px),
    /// for the window region that makes the rest of the window click-through.
    pub fn dot_circles(scale: f64) -> [(i32, i32, i32); 3] {
        let r = dip(DOT / 2.0, scale);
        let cy = dip(PAD + DOT / 2.0, scale);
        let c = |i: f64| dip(PAD + DOT / 2.0 + i * (DOT + GAP), scale);
        [(c(0.0), cy, r), (c(1.0), cy, r), (c(2.0), cy, r)]
    }

    /// The caption band (screen px) from `DWMWA_CAPTION_BUTTON_BOUNDS`, which
    /// is relative to the window rect. `None` unless the rect is one we can
    /// trust: a sensible height, at least one caption button wide, flush with
    /// the visible frame's right edge and at its top.
    pub fn caption_from_dwm(window: Rect, frame: Rect, buttons: Rect, scale: f64) -> Option<Rect> {
        if buttons.is_empty() || frame.is_empty() {
            return None;
        }
        let abs = buttons.offset(window.l, window.t);
        let h = abs.h();
        if h < dip(16.0, scale) || h > dip(64.0, scale) || abs.w() < dip(28.0, scale) {
            return None;
        }
        let slack = dip(16.0, scale);
        if (abs.r - frame.r).abs() > slack || abs.t < frame.t - slack || abs.t > frame.t + slack {
            return None;
        }
        let t = abs.t.max(frame.t);
        if abs.b <= t || abs.l <= frame.l {
            return None;
        }
        Some(Rect { l: frame.l, t, r: abs.l, b: abs.b })
    }

    /// The caption band from the native close button found by UI Automation
    /// in a custom title bar: the band spans the frame's left edge to the
    /// leftmost caption button and has the close button's height.
    pub fn caption_from_close(frame: Rect, close: Rect, leftmost_button: i32, scale: f64) -> Option<Rect> {
        if close.is_empty() {
            return None;
        }
        let slack = dip(12.0, scale);
        if (close.r - frame.r).abs() > slack || (close.t - frame.t).abs() > slack {
            return None;
        }
        if close.h() < dip(20.0, scale) || close.h() > dip(64.0, scale) {
            return None;
        }
        let t = close.t.max(frame.t);
        let r = leftmost_button.min(close.l);
        if r <= frame.l + dip(120.0, scale) {
            return None;
        }
        Some(Rect { l: frame.l, t, r, b: close.b })
    }

    /// The native caption buttons (screen px) from
    /// `DWMWA_CAPTION_BUTTON_BOUNDS`, validated like `caption_from_dwm`.
    pub fn buttons_from_dwm(window: Rect, frame: Rect, buttons: Rect, scale: f64) -> Option<Rect> {
        let band = caption_from_dwm(window, frame, buttons, scale)?;
        let abs = buttons.offset(window.l, window.t);
        let r = Rect { l: band.r, t: band.t, r: abs.r.min(frame.r), b: band.b };
        (r.w() >= dip(28.0, scale)).then_some(r)
    }

    /// The native caption buttons from the close button UI Automation
    /// found (and the leftmost caption button beside it).
    pub fn buttons_from_close(frame: Rect, close: Rect, leftmost_button: i32, scale: f64) -> Option<Rect> {
        let band = caption_from_close(frame, close, leftmost_button, scale)?;
        Some(Rect { l: band.r, t: band.t, r: close.r.min(frame.r), b: band.b })
    }

    /// The overlay that covers the native buttons (screen px): their
    /// rectangle, 1 px in from the window's top and right border so the
    /// frame's own edge stays visible.
    pub fn cover(buttons: Rect) -> Rect {
        Rect { l: buttons.l, t: buttons.t + 1, r: buttons.r - 1, b: buttons.b }
    }

    /// Right inset from the overlay's right edge to the zoom dot (DIP):
    /// a Mac's 13 DIP lead, mirrored into the corner.
    pub const RIGHT_INSET: f64 = 14.0;

    /// Centre and radius of each dot inside a cover overlay `w` x `h`
    /// physical px: right-aligned, vertically centred, close / minimise /
    /// zoom left to right.
    pub fn cover_dots(w: i32, h: i32, scale: f64) -> [(i32, i32, i32); 3] {
        let r = dip(DOT / 2.0, scale);
        let cy = h / 2;
        let last = w - dip(RIGHT_INSET + DOT / 2.0, scale);
        let step = dip(DOT + GAP, scale);
        [(last - 2 * step, cy, r), (last - step, cy, r), (last, cy, r)]
    }

    /// A standard (system-drawn) frame: the client area starts at or below
    /// the caption band, so nothing of the app's own is in the title bar.
    pub fn is_standard(band: Rect, client_top: i32) -> bool {
        client_top >= band.b - 2
    }

    /// Where the overlay window goes (screen px), or `None` when there is no
    /// safe space. The dots sit at the left of the caption band, centred on
    /// it vertically, `INSET` from the frame's left edge. Tab strips and
    /// custom title bars pass their clickable elements as `obstacles`: the
    /// spot plus `CLEAR` of space on each side must touch none of them, and
    /// it must stay inside the band and clear of the native caption buttons
    /// (`band.r`). Nothing is ever placed over a tab, a menu or a button.
    pub fn place(frame: Rect, band: Rect, obstacles: &[Rect], scale: f64) -> Option<Rect> {
        let (cw, ch) = cluster_size(scale);
        if band.is_empty() || band.h() < ch {
            return None;
        }
        let l = frame.l + dip(INSET - PAD, scale);
        let t = band.t + (band.h() - ch) / 2;
        let spot = Rect { l, t, r: l + cw, b: t + ch };
        let clear = dip(CLEAR, scale);
        if spot.r + clear > band.r || spot.t < frame.t || spot.b > frame.b {
            return None;
        }
        let guard = Rect { l: frame.l, t: spot.t, r: spot.r + clear, b: spot.b };
        if obstacles.iter().any(|o| !o.is_empty() && o.intersects(&guard)) {
            return None;
        }
        Some(spot)
    }

    /// Where the pointer makes the dots appear: the top-left corner of the
    /// focused window over the caption band (about 140 × caption DIP), never
    /// reaching into the first obstacle (a tab) or the native buttons.
    pub fn hover_zone(frame: Rect, band: Rect, spot: Rect, obstacles: &[Rect], scale: f64) -> Rect {
        let mut r = (frame.l + dip(ZONE_W, scale)).min(band.r);
        for o in obstacles {
            if !o.is_empty() && o.t < band.b && o.b > band.t && o.l >= spot.r {
                r = r.min(o.l);
            }
        }
        let r = r.max(spot.r + dip(4.0, scale));
        let edge = dip(6.0, scale);
        Rect { l: frame.l - edge, t: frame.t - edge, r, b: band.b.max(spot.b + dip(4.0, scale)) }
    }

    /// The frame covers its whole monitor: fullscreen (games, video,
    /// presentations, F11).
    pub fn covers_monitor(frame: Rect, monitor: Rect) -> bool {
        frame.l <= monitor.l + 1 && frame.t <= monitor.t + 1 && frame.r >= monitor.r - 1 && frame.b >= monitor.b - 1
    }

    /// The apps whose custom title bars Cove measures with UI Automation.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub enum Kind {
        Explorer,
        Chrome,
        Edge,
        VsCode,
        Other,
    }

    impl Kind {
        pub fn allowed(self) -> bool {
            self != Kind::Other
        }
    }

    /// Classify by program file name and window class.
    pub fn kind(exe: &str, class: &str) -> Kind {
        let name = exe_key(exe);
        match (name.as_str(), class) {
            ("explorer.exe", "CabinetWClass") => Kind::Explorer,
            ("chrome.exe", c) if c.starts_with("Chrome_WidgetWin") => Kind::Chrome,
            ("msedge.exe", c) if c.starts_with("Chrome_WidgetWin") => Kind::Edge,
            ("code.exe", c) if c.starts_with("Chrome_WidgetWin") => Kind::VsCode,
            _ => Kind::Other,
        }
    }

    /// The per-app key: the lower-case file name of the program.
    pub fn exe_key(exe: &str) -> String {
        exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_ascii_lowercase()
    }

    /// The user asked for no buttons on this app ("Don't show on this app").
    pub fn skipped(exe: &str, list: &[String]) -> bool {
        let k = exe_key(exe);
        !k.is_empty() && list.iter().any(|s| exe_key(s) == k)
    }

    /// Integrity check: Cove can only post to windows of equal or lower
    /// integrity (UIPI), so a higher one, or one it can't read, is skipped.
    pub fn can_act(mine: u32, theirs: Option<u32>) -> bool {
        matches!(theirs, Some(t) if t <= mine)
    }
}

/// What the overlay window should do (handled on the main thread).
#[derive(Debug, Clone)]
pub enum Ui {
    /// Put the overlay at this physical rect and fade it in.
    Show(Look),
    /// Fade out over `ms`, then hide the window.
    Hide { ms: u32 },
    /// The title bar changed colour (focus, theme): repaint the patch.
    Color(String),
    /// Destroy the overlay (setting off, safe mode, quit).
    Remove,
}

/// Where and how the overlay is drawn.
#[derive(Debug, Clone, Serialize)]
pub struct Look {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub scale: f64,
    /// Minimise and zoom are offered by the window (grey and inert if not).
    pub min: bool,
    pub max: bool,
    /// The window is maximised (zoom restores it).
    pub zoomed: bool,
    /// The title bar's colour next to the buttons ("#rrggbb"): the patch
    /// that hides the native buttons.
    pub color: String,
    /// The window's own corner radius at the top right (px; 0 when square).
    pub radius: i32,
}

/// Why Cove is holding the buttons back for now.
#[derive(Clone, Copy, Debug)]
pub enum Hold {
    Intro = 1,
    Expose = 2,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppEntry {
    pub exe: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrafficInfo {
    pub enabled: bool,
    pub skip: Vec<AppEntry>,
    /// Apps that had a window focused this session (to add to the skip list).
    pub seen: Vec<AppEntry>,
}

pub const FADE_IN: u32 = 150;
pub const FADE_OUT: u32 = 150;
pub const FADE_FAST: u32 = 80;
/// Stillness after a move/resize before the dots may come back.
pub const SETTLE_MS: u64 = 100;

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Emitter, Manager};

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOLDS: AtomicU32 = AtomicU32::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);
/// The page reports the pointer is over the dots.
static OVER: AtomicBool = AtomicBool::new(false);
static GEN: AtomicU64 = AtomicU64::new(0);
static SKIP: Mutex<Vec<String>> = Mutex::new(Vec::new());
/// exe key -> friendly name, for Settings' per-app list.
static SEEN: Mutex<Vec<(String, String)>> = Mutex::new(Vec::new());
/// Skip reasons already logged (one line per app and reason).
static LOGGED: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn log_once(exe: &str, why: &str) {
    let key = format!("{exe}:{why}");
    let Ok(mut l) = LOGGED.lock() else { return };
    if l.contains(&key) || l.len() > 400 {
        return;
    }
    l.push(key);
    if let Some(app) = APP.get() {
        crate::cove_log(app, &format!("traffic: skip {exe} ({why})"));
    }
}

fn note_seen(exe: &str, title: &str) {
    let key = geom::exe_key(exe);
    if key.is_empty() {
        return;
    }
    let Ok(mut s) = SEEN.lock() else { return };
    if s.iter().any(|(k, _)| *k == key) {
        return;
    }
    let name = crate::appnames::friendly(exe, None, title);
    s.push((key, name));
}

/// Hook up the overlay to Cove (once, in setup) and start it if it is on.
pub fn init(app: &AppHandle) {
    let _ = APP.set(app.clone());
    // Held until the startup intro ends (main.rs releases it).
    refresh();
}

/// Re-read the setting (and safe mode) and start or stop the overlay.
pub fn refresh() {
    let Some(app) = APP.get() else { return };
    let Some(state) = app.try_state::<crate::AppState>() else { return };
    let (on, skip) = state.settings.lock().map(|s| (s.data.traffic, s.data.traffic_skip.clone())).unwrap_or((false, vec![]));
    if let Ok(mut s) = SKIP.lock() {
        *s = skip;
    }
    set_enabled(on && !crate::safe());
}

fn set_enabled(on: bool) {
    let was = ENABLED.swap(on, Ordering::SeqCst);
    if on && !was {
        // Load the page now, so the first hover shows at once.
        if let Some(app) = APP.get() {
            let a = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = overlay(&a);
            });
        }
        imp::start();
    } else if !on && was {
        imp::stop();
        ui(Ui::Remove);
    } else if on {
        imp::poke();
    }
}

/// Quit: remove everything.
pub fn stop() {
    ENABLED.store(false, Ordering::SeqCst);
    imp::stop();
}

/// Hold the buttons back (intro, Exposé) or release that hold.
pub fn hold(h: Hold, on: bool) {
    let bit = h as u32;
    if on {
        HOLDS.fetch_or(bit, Ordering::SeqCst);
    } else {
        HOLDS.fetch_and(!bit, Ordering::SeqCst);
    }
    imp::poke();
}

fn held() -> bool {
    HOLDS.load(Ordering::SeqCst) != 0
}

fn ui(u: Ui) {
    let Some(app) = APP.get() else { return };
    let a = app.clone();
    let _ = app.run_on_main_thread(move || apply_ui(&a, u));
}

/// The last state sent to the page (for a page that loads after it).
static LAST: Mutex<Option<PagePayload>> = Mutex::new(None);

fn to_page(app: &AppHandle, p: PagePayload) {
    if let Ok(mut l) = LAST.lock() {
        *l = Some(p.clone());
    }
    let _ = app.emit_to("traffic", "traffic", p);
}

#[derive(Clone, Serialize)]
pub struct PagePayload {
    show: bool,
    fade: u32,
    min: bool,
    max: bool,
    zoomed: bool,
    color: String,
    radius: f64,
    scale: f64,
}

/// The overlay window: created when the feature turns on, destroyed when
/// it turns off. Transparent, no title bar, never in the taskbar.
fn overlay(app: &AppHandle) -> Option<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window("traffic") {
        return Some(w);
    }
    let w = tauri::WebviewWindowBuilder::new(app, "traffic", tauri::WebviewUrl::App("traffic.html".into()))
        .title("Cove window buttons")
        .inner_size(60.0, 16.0)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .visible(false)
        .build()
        .ok()?;
    imp::style_overlay(&w);
    Some(w)
}

fn apply_ui(app: &AppHandle, u: Ui) {
    match u {
        Ui::Show(look) => {
            if !ENABLED.load(Ordering::SeqCst) {
                return;
            }
            let Some(w) = overlay(app) else { return };
            GEN.fetch_add(1, Ordering::SeqCst);
            imp::show_overlay(&w, &look);
            to_page(app, PagePayload { show: true, fade: FADE_IN, min: look.min, max: look.max, zoomed: look.zoomed, color: look.color.clone(), radius: look.radius as f64 / look.scale, scale: look.scale });
        }
        Ui::Hide { ms } => {
            let Some(_) = app.get_webview_window("traffic") else { return };
            // A hidden page never sees pointerleave: forget the hover so a
            // stale "over" can't bring the dots back by itself.
            OVER.store(false, Ordering::SeqCst);
            let g = GEN.fetch_add(1, Ordering::SeqCst) + 1;
            let last = LAST.lock().ok().and_then(|l| l.clone());
            let (min, max, color, radius, scale) = last.map(|p| (p.min, p.max, p.color, p.radius, p.scale)).unwrap_or((true, true, String::new(), 0.0, 1.0));
            to_page(app, PagePayload { show: false, fade: ms, min, max, zoomed: false, color, radius, scale });
            let a = app.clone();
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(ms as u64 + 30));
                let a2 = a.clone();
                let _ = a.run_on_main_thread(move || {
                    if GEN.load(Ordering::SeqCst) == g {
                        if let Some(w) = a2.get_webview_window("traffic") {
                            imp::hide_overlay(&w);
                        }
                    }
                });
            });
        }
        Ui::Color(c) => {
            let next = LAST.lock().ok().and_then(|l| l.clone()).filter(|p| p.show && p.color != c);
            if let Some(mut p) = next {
                p.color = c;
                p.fade = 0;
                to_page(app, p);
            }
        }
        Ui::Remove => {
            GEN.fetch_add(1, Ordering::SeqCst);
            OVER.store(false, Ordering::SeqCst);
            if let Some(w) = app.get_webview_window("traffic") {
                let _ = w.destroy();
            }
        }
    }
}

// ---- Commands ----

fn info(app: &AppHandle) -> TrafficInfo {
    let (enabled, skip) = app
        .try_state::<crate::AppState>()
        .and_then(|state| state.settings.lock().ok().map(|s| (s.data.traffic, s.data.traffic_skip.clone())))
        .unwrap_or((true, vec![]));
    let seen = SEEN.lock().map(|s| s.clone()).unwrap_or_default();
    let name_of = |k: &str| seen.iter().find(|(e, _)| e == k).map(|(_, n)| n.clone()).unwrap_or_else(|| k.trim_end_matches(".exe").to_string());
    let skip_e: Vec<AppEntry> = skip.iter().map(|k| AppEntry { exe: k.clone(), name: name_of(k) }).collect();
    let mut seen_e: Vec<AppEntry> = seen
        .iter()
        .filter(|(k, _)| !geom::skipped(k, &skip))
        .map(|(k, n)| AppEntry { exe: k.clone(), name: n.clone() })
        .collect();
    seen_e.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    TrafficInfo { enabled, skip: skip_e, seen: seen_e }
}

#[tauri::command]
pub fn traffic_status(app: AppHandle) -> TrafficInfo {
    info(&app)
}

#[tauri::command]
pub fn traffic_set(enabled: bool, app: AppHandle) -> TrafficInfo {
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut s) = state.settings.lock() {
            s.data.traffic = enabled;
            let _ = s.save();
        }
    }
    refresh();
    info(&app)
}

#[tauri::command]
pub fn traffic_skip_set(apps: Vec<String>, app: AppHandle) -> TrafficInfo {
    let mut list: Vec<String> = Vec::new();
    for a in apps {
        let k = geom::exe_key(&a);
        if !k.is_empty() && !list.contains(&k) {
            list.push(k);
        }
    }
    if let Some(state) = app.try_state::<crate::AppState>() {
        if let Ok(mut s) = state.settings.lock() {
            s.data.traffic_skip = list;
            let _ = s.save();
        }
    }
    refresh();
    info(&app)
}

/// A dot was clicked: "close", "min" or "zoom" (Alt+click on zoom opens
/// Snap Layouts with Win+Z).
#[tauri::command]
pub fn traffic_click(action: String, alt: bool) {
    imp::click(&action, alt);
}

/// The overlay page has loaded: what it should be showing.
#[tauri::command]
pub fn traffic_hello() -> Option<PagePayload> {
    LAST.lock().ok().and_then(|l| l.clone())
}

/// The pointer entered or left the dots.
#[tauri::command]
pub fn traffic_hover(over: bool) {
    OVER.store(over, Ordering::SeqCst);
    imp::poke();
}

// ---- Windows ----

#[cfg(windows)]
mod imp {
    use super::geom::{self, Rect};
    use super::{held, log_once, note_seen, ui, Look, Ui, ENABLED, FADE_FAST, FADE_OUT, OVER, SETTLE_MS, SKIP};
    use std::cell::RefCell;
    use std::ffi::c_void;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    use windows_sys::Win32::Foundation::{CloseHandle, HWND, POINT, RECT};
    use windows_sys::Win32::Graphics::Dwm::{
        DwmGetWindowAttribute, DWMWA_CAPTION_BUTTON_BOUNDS, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
    };
    use windows_sys::Win32::Graphics::Gdi::{
        ClientToScreen, CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, GetDC, GetMonitorInfoW, GetPixel,
        MonitorFromWindow, ReleaseDC, SetWindowRgn, MONITORINFO, MONITOR_DEFAULTTONEAREST, RGN_OR,
    };
    use windows_sys::Win32::Security::{GetSidSubAuthority, GetSidSubAuthorityCount, GetTokenInformation, TokenIntegrityLevel, TOKEN_MANDATORY_LABEL, TOKEN_QUERY};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcess, GetCurrentProcessId, GetCurrentThreadId, OpenProcess, OpenProcessToken, QueryFullProcessImageNameW,
        PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Accessibility::{SetWinEventHook, UnhookWinEvent, HCF_HIGHCONTRASTON, HIGHCONTRASTW, HWINEVENTHOOK};
    use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_LWIN, VK_MENU,
    };
    use windows_sys::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        DispatchMessageW, GetAncestor, GetClassNameW, GetWindow, GW_HWNDPREV, GetClientRect, GetCursorPos, GetForegroundWindow, GetWindowLongPtrW,
        GetWindowLongW, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsHungAppWindow, IsIconic, IsWindow,
        IsWindowVisible, IsZoomed, MsgWaitForMultipleObjectsEx, PeekMessageW, PostMessageW, PostThreadMessageW,
        SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage,
        WindowFromPoint, CHILDID_SELF, EVENT_OBJECT_DESTROY, EVENT_OBJECT_LOCATIONCHANGE, EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MOVESIZEEND, EVENT_SYSTEM_MOVESIZESTART, GA_ROOT,
        GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, MSG, MWMO_INPUTAVAILABLE, OBJID_WINDOW, PM_REMOVE, QS_ALLINPUT, SC_CLOSE,
        SC_MAXIMIZE, SC_MINIMIZE, SC_RESTORE, SPI_GETHIGHCONTRAST, SWP_HIDEWINDOW, SWP_NOACTIVATE, SWP_NOMOVE,
        SWP_NOOWNERZORDER, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, WINEVENT_OUTOFCONTEXT, WINEVENT_SKIPOWNPROCESS,
        WM_APP, WM_QUIT, WM_SYSCOMMAND, WS_CAPTION, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
        WS_MAXIMIZEBOX, WS_MINIMIZEBOX,
    };

    /// The watcher thread (0 when not running).
    static TID: AtomicU32 = AtomicU32::new(0);
    /// The overlay window's handle (so its own clicks never count as a
    /// foreground change).
    static OVERLAY: Mutex<isize> = Mutex::new(0);
    /// The window the dots act on, and whether Cove may send it input.
    static CURRENT: Mutex<Option<(isize, bool)>> = Mutex::new(None);

    const WM_POKE: u32 = WM_APP + 41;

    fn r(rc: RECT) -> Rect {
        Rect::new(rc.left, rc.top, rc.right, rc.bottom)
    }

    // ---- Lifecycle ----

    pub fn start() {
        if TID.load(Ordering::SeqCst) != 0 {
            return;
        }
        std::thread::Builder::new()
            .name("cove-traffic".into())
            .spawn(run)
            .ok();
    }

    pub fn stop() {
        let tid = TID.swap(0, Ordering::SeqCst);
        if tid != 0 {
            unsafe { PostThreadMessageW(tid, WM_QUIT, 0, 0) };
        }
        if let Ok(mut c) = CURRENT.lock() {
            *c = None;
        }
    }

    /// Wake the watcher to re-check (setting, holds, hover).
    pub fn poke() {
        let tid = TID.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe { PostThreadMessageW(tid, WM_POKE, 0, 0) };
        }
    }

    // ---- Overlay window (main thread) ----

    pub fn style_overlay(w: &tauri::WebviewWindow) {
        let Ok(h) = w.hwnd() else { return };
        let h = h.0 as HWND;
        if let Ok(mut o) = OVERLAY.lock() {
            *o = h as isize;
        }
        unsafe {
            let ex = GetWindowLongPtrW(h, GWL_EXSTYLE);
            let want = (ex | (WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST) as isize) & !(WS_EX_APPWINDOW as isize);
            SetWindowLongPtrW(h, GWL_EXSTYLE, want);
        }
    }

    pub fn show_overlay(w: &tauri::WebviewWindow, look: &Look) {
        let Ok(h) = w.hwnd() else { return };
        let h = h.0 as HWND;
        unsafe {
            let flags = SWP_NOACTIVATE | SWP_NOOWNERZORDER | SWP_SHOWWINDOW;
            SetWindowPos(h, HWND_TOPMOST, look.x, look.y, look.w, look.h, flags);
            // Crossing to a display with another DPI can resize the window
            // once (WM_DPICHANGED); set it again so it is exact.
            SetWindowPos(h, HWND_TOPMOST, look.x, look.y, look.w, look.h, flags);
            // The whole patch takes clicks (it hides the native buttons);
            // the window's own rounded top-right corner stays the window's.
            let (w_, h_, r) = (look.w, look.h, look.radius);
            let rgn = if r > 0 {
                let a = CreateRectRgn(0, 0, w_ - r, h_);
                let b = CreateRectRgn(0, r, w_, h_);
                let c = CreateRoundRectRgn(w_ - 2 * r, 0, w_ + 1, 2 * r + 1, 2 * r, 2 * r);
                CombineRgn(a, a, b, RGN_OR);
                CombineRgn(a, a, c, RGN_OR);
                DeleteObject(b);
                DeleteObject(c);
                a
            } else {
                CreateRectRgn(0, 0, w_, h_)
            };
            if !rgn.is_null() && SetWindowRgn(h, rgn, 1) == 0 {
                DeleteObject(rgn);
            }
        }
    }

    pub fn hide_overlay(w: &tauri::WebviewWindow) {
        let Ok(h) = w.hwnd() else { return };
        unsafe {
            SetWindowPos(h.0 as HWND, std::ptr::null_mut(), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_HIDEWINDOW);
            ShowWindow(h.0 as HWND, SW_HIDE);
        }
    }

    // ---- Clicks ----

    pub fn click(action: &str, alt: bool) {
        let Some((h, same)) = CURRENT.lock().ok().and_then(|c| *c) else { return };
        let hwnd = h as HWND;
        unsafe {
            if IsWindow(hwnd) == 0 || IsHungAppWindow(hwnd) != 0 {
                return;
            }
            // If the click activated the overlay after all, hand the
            // foreground straight back (Cove is foreground, so it may).
            let overlay = OVERLAY.lock().map(|o| *o).unwrap_or(0);
            if overlay != 0 && GetForegroundWindow() as isize == overlay {
                SetForegroundWindow(hwnd);
            }
            let cmd = match action {
                // A rest on green: Windows' Snap Layouts, as hovering the
                // native maximise button would show.
                "snap" if same => {
                    snap_layouts();
                    return;
                }
                "close" => SC_CLOSE,
                "min" => SC_MINIMIZE,
                "zoom" if alt && same => {
                    snap_layouts();
                    ui(Ui::Hide { ms: FADE_FAST });
                    return;
                }
                "zoom" if IsZoomed(hwnd) != 0 => SC_RESTORE,
                "zoom" => SC_MAXIMIZE,
                _ => return,
            };
            // Post, never send: a busy app can't block Cove.
            PostMessageW(hwnd, WM_SYSCOMMAND, cmd as usize, 0);
        }
        ui(Ui::Hide { ms: FADE_FAST });
    }

    /// Win+Z: Windows' own Snap Layouts for the foreground window. Alt is
    /// held for the click, so release it first behind a masking key (0xE8,
    /// unassigned) so the app doesn't open its menu bar.
    fn snap_layouts() {
        fn key(vk: VIRTUAL_KEY, up: bool) -> INPUT {
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: vk, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 } },
            }
        }
        const MASK: VIRTUAL_KEY = 0xE8;
        const VK_Z: VIRTUAL_KEY = 0x5A;
        let seq = [
            key(MASK, false),
            key(MASK, true),
            key(VK_MENU, true),
            key(VK_LWIN, false),
            key(VK_Z, false),
            key(VK_Z, true),
            key(VK_LWIN, true),
        ];
        unsafe { SendInput(seq.len() as u32, seq.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    // ---- Facts about a window ----

    fn frame(h: HWND) -> Rect {
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_EXTENDED_FRAME_BOUNDS as _, &mut rc as *mut RECT as *mut c_void, std::mem::size_of::<RECT>() as u32) };
        if hr == 0 {
            r(rc)
        } else {
            window_rect(h)
        }
    }

    fn window_rect(h: HWND) -> Rect {
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetWindowRect(h, &mut rc) };
        r(rc)
    }

    fn caption_buttons(h: HWND) -> Rect {
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_CAPTION_BUTTON_BOUNDS as _, &mut rc as *mut RECT as *mut c_void, std::mem::size_of::<RECT>() as u32) };
        if hr == 0 {
            r(rc)
        } else {
            Rect::default()
        }
    }

    fn cloaked(h: HWND) -> bool {
        let mut v: u32 = 0;
        let hr = unsafe { DwmGetWindowAttribute(h, DWMWA_CLOAKED as _, &mut v as *mut u32 as *mut c_void, 4) };
        hr == 0 && v != 0
    }

    fn client_top(h: HWND) -> i32 {
        let mut p = POINT { x: 0, y: 0 };
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        unsafe {
            GetClientRect(h, &mut rc);
            ClientToScreen(h, &mut p);
        }
        p.y
    }

    fn monitor(h: HWND) -> Rect {
        let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
        mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        unsafe { GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) };
        r(mi.rcMonitor)
    }

    fn high_contrast() -> bool {
        let mut hc: HIGHCONTRASTW = unsafe { std::mem::zeroed() };
        hc.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
        let ok = unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, &mut hc as *mut _ as *mut c_void, 0) };
        ok != 0 && hc.dwFlags & HCF_HIGHCONTRASTON != 0
    }

    fn fullscreen_app() -> bool {
        let mut s = 0;
        let hr = unsafe { SHQueryUserNotificationState(&mut s) };
        hr == 0 && (s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE)
    }

    fn class(h: HWND) -> String {
        let mut buf = [0u16; 128];
        let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn title(h: HWND) -> String {
        let mut buf = [0u16; 256];
        let n = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    fn pid_of(h: HWND) -> u32 {
        let mut pid = 0u32;
        unsafe { GetWindowThreadProcessId(h, &mut pid) };
        pid
    }

    fn exe_of(pid: u32) -> String {
        unsafe {
            let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if p.is_null() {
                return String::new();
            }
            let mut buf = [0u16; 520];
            let mut n = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(p, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut n);
            CloseHandle(p);
            if ok == 0 {
                return String::new();
            }
            String::from_utf16_lossy(&buf[..n as usize])
        }
    }

    /// The mandatory integrity level (RID) of a process token.
    fn token_integrity(process: windows_sys::Win32::Foundation::HANDLE) -> Option<u32> {
        unsafe {
            let mut tok = std::ptr::null_mut();
            if OpenProcessToken(process, TOKEN_QUERY, &mut tok) == 0 {
                return None;
            }
            let mut need = 0u32;
            GetTokenInformation(tok, TokenIntegrityLevel, std::ptr::null_mut(), 0, &mut need);
            let mut buf = vec![0u8; need.max(64) as usize];
            let ok = GetTokenInformation(tok, TokenIntegrityLevel, buf.as_mut_ptr() as *mut c_void, buf.len() as u32, &mut need);
            CloseHandle(tok);
            if ok == 0 {
                return None;
            }
            let label = &*(buf.as_ptr() as *const TOKEN_MANDATORY_LABEL);
            let sid = label.Label.Sid;
            let count = *GetSidSubAuthorityCount(sid) as u32;
            if count == 0 {
                return None;
            }
            Some(*GetSidSubAuthority(sid, count - 1))
        }
    }

    fn my_integrity() -> u32 {
        static MINE: std::sync::OnceLock<u32> = std::sync::OnceLock::new();
        *MINE.get_or_init(|| token_integrity(unsafe { GetCurrentProcess() }).unwrap_or(0x2000))
    }

    fn integrity_of(pid: u32) -> Option<u32> {
        unsafe {
            let p = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if p.is_null() {
                return None;
            }
            let il = token_integrity(p);
            CloseHandle(p);
            il
        }
    }

    fn root(h: HWND) -> HWND {
        let a = unsafe { GetAncestor(h, GA_ROOT) };
        if a.is_null() {
            h
        } else {
            a
        }
    }

    // ---- The watched window ----

    #[derive(Clone)]
    struct Target {
        hwnd: HWND,
        pid: u32,
        exe: String,
        same_il: bool,
        /// The overlay rectangle: the native caption buttons it covers.
        spot: Rect,
        scale: f64,
        min: bool,
        max: bool,
        /// Top-right corner radius of the window (px).
        radius: i32,
        /// Title bar colour next to the buttons, and when it was read.
        color: String,
        sampled: Option<Instant>,
        /// Custom title bar measured by UI Automation: (frame size, buttons
        /// relative to the frame's top-left), reused while the window only
        /// moves.
        uia: Option<(i32, i32, Rect)>,
    }

    #[derive(Default)]
    struct Watch {
        target: Option<Target>,
        loc_hooks: Vec<HWINEVENTHOOK>,
        dragging: bool,
        settle_at: Option<Instant>,
        shown: bool,
    }

    thread_local! {
        static W: RefCell<Watch> = RefCell::new(Watch::default());
    }

    fn hide(w: &mut Watch, ms: u32) {
        if w.shown {
            w.shown = false;
            ui(Ui::Hide { ms });
        }
    }

    fn set_current(t: Option<&Target>) {
        if let Ok(mut c) = CURRENT.lock() {
            *c = t.map(|t| (t.hwnd as isize, t.same_il));
        }
    }

    fn unhook_loc(w: &mut Watch) {
        for h in w.loc_hooks.drain(..) {
            unsafe { UnhookWinEvent(h) };
        }
    }

    /// Everything that decides whether (and where) a window gets the dots.
    /// `Err` carries the reason it is skipped. `prev` lets a moved window
    /// reuse its UI Automation measurements.
    fn inspect(h: HWND, prev: Option<&Target>) -> Result<Target, (String, &'static str)> {
        unsafe {
            let pid = pid_of(h);
            if pid == 0 || pid == GetCurrentProcessId() {
                return Err((String::new(), "Cove"));
            }
            let exe = exe_of(pid);
            let fail = |why: &'static str| Err((exe.clone(), why));
            if IsWindowVisible(h) == 0 || IsIconic(h) != 0 {
                return fail("hidden or minimised");
            }
            if cloaked(h) {
                return fail("cloaked");
            }
            if IsHungAppWindow(h) != 0 {
                return fail("not responding");
            }
            if high_contrast() {
                return fail("High Contrast");
            }
            let style = GetWindowLongW(h, GWL_STYLE) as u32;
            let ex = GetWindowLongW(h, GWL_EXSTYLE) as u32;
            if ex & WS_EX_TOOLWINDOW != 0 {
                return fail("tool window");
            }
            let (min, max) = (style & WS_MINIMIZEBOX != 0, style & WS_MAXIMIZEBOX != 0);
            if !min && !max {
                return fail("dialog without minimise or maximise");
            }
            let cls = class(h);
            let kind = geom::kind(&exe, &cls);
            if exe.is_empty() {
                return fail("program unknown");
            }
            note_seen(&exe, &title(h));
            if SKIP.lock().map(|s| geom::skipped(&exe, &s)).unwrap_or(false) {
                return fail("turned off for this app");
            }
            let theirs = integrity_of(pid);
            if !geom::can_act(my_integrity(), theirs) {
                return fail("elevated or unreadable integrity");
            }
            let same_il = theirs == Some(my_integrity());
            let frame = frame(h);
            if geom::covers_monitor(frame, monitor(h)) || fullscreen_app() {
                return fail("fullscreen");
            }
            let dpi = GetDpiForWindow(h);
            let scale = if dpi == 0 { 1.0 } else { dpi as f64 / 96.0 };
            // The native caption buttons, which the overlay covers.
            // 1. A standard frame: DWM says where they are. Allow-listed
            //    custom title bars (Explorer, Chrome, Edge, VS Code) draw
            //    their own, so UI Automation is asked first for those.
            let dwm = geom::buttons_from_dwm(window_rect(h), frame, caption_buttons(h), scale);
            let standard = dwm.filter(|b| geom::is_standard(Rect { l: frame.l, t: b.t, r: b.l, b: b.b }, client_top(h)));
            let cached = prev
                .filter(|p| p.hwnd == h)
                .and_then(|p| p.uia)
                .filter(|(w, hh, _)| *w == frame.w() && *hh == frame.h());
            let by_uia = |cached: Option<(i32, i32, Rect)>| -> Option<(i32, i32, Rect)> {
                if cached.is_some() {
                    return cached;
                }
                // UI Automation looks at what is on screen: our own patch
                // must not be over the buttons it is looking for.
                hide_now();
                cached.or_else(|| super::uia::buttons(h as isize, frame, scale, pid).map(|b| (frame.w(), frame.h(), b.offset(-frame.l, -frame.t))))
            };
            let (btns, uia) = if kind.allowed() {
                match by_uia(cached) {
                    Some(u) => (u.2.offset(frame.l, frame.t), Some(u)),
                    None => match standard {
                        Some(b) => (b, None),
                        None => return fail("custom title bar not measurable"),
                    },
                }
            } else if let Some(b) = standard {
                (b, None)
            } else {
                // 2. Any other custom title bar: only if UI Automation finds
                //    the app's own caption buttons at the top-right corner.
                match by_uia(cached) {
                    Some(u) => (u.2.offset(frame.l, frame.t), Some(u)),
                    None => return fail(if style & WS_CAPTION != WS_CAPTION { "no title bar" } else { "custom title bar" }),
                }
            };
            let spot = geom::cover(btns);
            if spot.is_empty() || spot.h() < geom::dip(16.0, scale) {
                return fail("caption buttons too small");
            }
            let zoomed = IsZoomed(h) != 0;
            let radius = if win11() && !zoomed { geom::dip(8.0, scale) } else { 0 };
            Ok(Target { hwnd: h, pid, exe, same_il, spot, scale, min, max, radius, color: String::new(), sampled: None, uia })
        }
    }

    fn set_target(w: &mut Watch, fg: HWND) {
        let overlay = OVERLAY.lock().map(|o| *o).unwrap_or(0);
        if fg.is_null() || fg as isize == overlay {
            return;
        }
        let h = root(fg);
        if w.target.as_ref().map(|t| t.hwnd) == Some(h) {
            return;
        }
        hide(w, FADE_FAST);
        unhook_loc(w);
        w.target = None;
        w.dragging = false;
        w.settle_at = None;
        set_current(None);
        if !ENABLED.load(Ordering::SeqCst) || held() {
            return;
        }
        match inspect(h, None) {
            Ok(t) => {
                // Moves and the window closing, for this process only.
                unsafe {
                    for ev in [EVENT_OBJECT_LOCATIONCHANGE, EVENT_OBJECT_DESTROY] {
                        let hk = SetWinEventHook(ev, ev, std::ptr::null_mut(), Some(on_event), t.pid, 0, WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS);
                        if !hk.is_null() {
                            w.loc_hooks.push(hk);
                        }
                    }
                }
                set_current(Some(&t));
                w.target = Some(t);
            }
            Err((exe, why)) => {
                if !exe.is_empty() {
                    log_once(&geom::exe_key(&exe), why);
                }
            }
        }
    }

    fn drop_target(w: &mut Watch) {
        hide(w, 0);
        unhook_loc(w);
        w.target = None;
        set_current(None);
    }

    unsafe extern "system" fn on_event(_: HWINEVENTHOOK, event: u32, hwnd: HWND, id_object: i32, id_child: i32, _: u32, _: u32) {
        W.with(|cell| {
            let Ok(mut w) = cell.try_borrow_mut() else { return };
            let w = &mut *w;
            if event == EVENT_SYSTEM_FOREGROUND {
                set_target(w, hwnd);
                return;
            }
            let Some(t) = w.target.as_ref() else { return };
            let ours = hwnd == t.hwnd && id_object == OBJID_WINDOW && id_child == CHILDID_SELF as i32;
            match event {
                EVENT_SYSTEM_MOVESIZESTART if hwnd == t.hwnd => {
                    w.dragging = true;
                    w.settle_at = None;
                    hide(w, FADE_FAST);
                }
                EVENT_SYSTEM_MOVESIZEEND if hwnd == t.hwnd => {
                    w.dragging = false;
                    w.settle_at = Some(Instant::now() + Duration::from_millis(SETTLE_MS));
                }
                EVENT_SYSTEM_MINIMIZESTART if hwnd == t.hwnd => {
                    w.settle_at = None;
                    hide(w, FADE_FAST);
                }
                EVENT_SYSTEM_MINIMIZEEND if hwnd == t.hwnd => {
                    w.settle_at = Some(Instant::now() + Duration::from_millis(SETTLE_MS));
                }
                EVENT_OBJECT_LOCATIONCHANGE if ours && !w.dragging => {
                    // Snap, Win+arrow, maximise: hide, re-place once still.
                    hide(w, FADE_FAST);
                    w.settle_at = Some(Instant::now() + Duration::from_millis(SETTLE_MS));
                }
                EVENT_OBJECT_DESTROY if ours => drop_target(w),
                _ => {}
            }
        });
    }

    /// The window has been still for SETTLE_MS: measure it again.
    fn settle(w: &mut Watch) {
        w.settle_at = None;
        let Some(prev) = w.target.clone() else { return };
        match inspect(prev.hwnd, Some(&prev)) {
            Ok(mut t) => {
                t.pid = prev.pid;
                if t.spot == prev.spot {
                    t.color = prev.color.clone();
                    t.sampled = prev.sampled;
                }
                set_current(Some(&t));
                w.target = Some(t);
            }
            Err(_) => {
                // Skipped for now (fullscreen, minimised…): keep watching it,
                // with nothing to show until it qualifies again.
                hide(w, FADE_FAST);
                let mut t = prev;
                t.spot = Rect::default();
                w.target = Some(t);
            }
        }
    }

    /// The focused window qualifies: the dots stay on its title bar,
    /// covering the native buttons, unless another window lies over them.
    fn check_pointer(w: &mut Watch) {
        let Some(t) = w.target.as_mut() else { return };
        if w.dragging || w.settle_at.is_some() || held() || !ENABLED.load(Ordering::SeqCst) || t.spot.is_empty() {
            hide(w, FADE_FAST);
            return;
        }
        if !still_ok(t) || occluded(t) {
            hide(w, FADE_FAST);
            return;
        }
        // Read the title bar colour beside the buttons (focus and theme
        // change it), at most every 400 ms.
        let now = Instant::now();
        if t.sampled.is_none_or(|at| now.duration_since(at) >= Duration::from_millis(400)) {
            t.sampled = Some(now);
            if let Some(c) = title_color(t) {
                if c != t.color {
                    t.color = c.clone();
                    if w.shown {
                        ui(Ui::Color(c));
                    }
                }
            }
        }
        let t = t.clone();
        if !w.shown {
            w.shown = true;
            let zoomed = unsafe { IsZoomed(t.hwnd) } != 0;
            let color = if t.color.is_empty() { "#202020".to_string() } else { t.color.clone() };
            ui(Ui::Show(Look { x: t.spot.l, y: t.spot.t, w: t.spot.w(), h: t.spot.h(), scale: t.scale, min: t.min, max: t.max, zoomed, color, radius: t.radius }));
        }
    }

    /// Another visible window above the target overlaps the buttons.
    fn occluded(t: &Target) -> bool {
        let overlay = OVERLAY.lock().map(|o| *o).unwrap_or(0);
        let mut h = unsafe { GetWindow(t.hwnd, GW_HWNDPREV) };
        let mut n = 0;
        while !h.is_null() && n < 400 {
            n += 1;
            if h as isize != overlay && unsafe { IsWindowVisible(h) } != 0 && !cloaked(h) {
                let ex = unsafe { GetWindowLongW(h, GWL_EXSTYLE) } as u32;
                // Click-through overlays (other apps' HUDs) don't count.
                if ex & WS_EX_TRANSPARENT == 0 && window_rect(h).intersects(&t.spot) {
                    return true;
                }
            }
            h = unsafe { GetWindow(h, GW_HWNDPREV) };
        }
        false
    }

    /// The title bar's colour just left of the buttons: the median of three
    /// screen pixels down the band.
    fn title_color(t: &Target) -> Option<String> {
        let x = t.spot.l - geom::dip(6.0, t.scale);
        let ys = [t.spot.t + geom::dip(4.0, t.scale), (t.spot.t + t.spot.b) / 2, t.spot.b - geom::dip(4.0, t.scale)];
        let mut px: Vec<u32> = Vec::new();
        unsafe {
            let dc = GetDC(std::ptr::null_mut());
            if dc.is_null() {
                return None;
            }
            for y in ys {
                let c = GetPixel(dc, x, y);
                if c != 0xFFFF_FFFF {
                    px.push(c);
                }
            }
            ReleaseDC(std::ptr::null_mut(), dc);
        }
        if px.is_empty() {
            return None;
        }
        let lum = |c: u32| (c & 0xff) * 3 + ((c >> 8) & 0xff) * 6 + ((c >> 16) & 0xff);
        px.sort_by_key(|&c| lum(c));
        let c = px[px.len() / 2];
        Some(format!("#{:02x}{:02x}{:02x}", c & 0xff, (c >> 8) & 0xff, (c >> 16) & 0xff))
    }

    /// Hide the overlay window right away (before UI Automation looks).
    fn hide_now() {
        let o = OVERLAY.lock().map(|o| *o).unwrap_or(0);
        if o != 0 && unsafe { IsWindowVisible(o as HWND) } != 0 {
            unsafe { ShowWindow(o as HWND, SW_HIDE) };
            std::thread::sleep(Duration::from_millis(30));
        }
    }

    /// Windows 11 rounds window corners (8 DIP); Windows 10 doesn't.
    fn win11() -> bool {
        static V: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        *V.get_or_init(|| {
            winreg::RegKey::predef(winreg::enums::HKEY_LOCAL_MACHINE)
                .open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion")
                .and_then(|k| k.get_value::<String, _>("CurrentBuildNumber"))
                .ok()
                .and_then(|b| b.trim().parse::<u32>().ok())
                .is_some_and(|b| b >= 22000)
        })
    }

    /// Cheap re-checks right before showing.
    fn still_ok(t: &Target) -> bool {
        unsafe {
            IsWindow(t.hwnd) != 0
                && root(GetForegroundWindow()) == t.hwnd
                && IsIconic(t.hwnd) == 0
                && IsHungAppWindow(t.hwnd) == 0
                && !cloaked(t.hwnd)
                && !high_contrast()
        }
    }

    /// How long the loop may sleep: no target means no polling at all.
    fn next_wait(w: &Watch, now: Instant) -> u32 {
        let mut ms: u64 = u32::MAX as u64; // INFINITE
        if let Some(at) = w.settle_at {
            ms = ms.min(at.saturating_duration_since(now).as_millis() as u64);
        }
        if let Some(t) = &w.target {
            if !w.dragging && !t.spot.is_empty() && ENABLED.load(Ordering::SeqCst) && !held() {
                // Shown or waiting to show: keep an eye on overlap and colour.
                ms = ms.min(if w.shown { 120 } else { 160 });
            }
        }
        ms.min(u32::MAX as u64) as u32
    }

    fn run() {
        TID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
        let mut hooks: Vec<HWINEVENTHOOK> = Vec::new();
        unsafe {
            for (a, b) in [
                (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
                (EVENT_SYSTEM_MOVESIZESTART, EVENT_SYSTEM_MOVESIZEEND),
                (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
            ] {
                // Out of context: queued to this thread, nothing injected.
                let h = SetWinEventHook(a, b, std::ptr::null_mut(), Some(on_event), 0, 0, WINEVENT_OUTOFCONTEXT);
                if !h.is_null() {
                    hooks.push(h);
                }
            }
        }
        W.with(|c| set_target(&mut c.borrow_mut(), unsafe { GetForegroundWindow() }));
        let mut msg: MSG = unsafe { std::mem::zeroed() };
        'outer: loop {
            let wait = W.with(|c| next_wait(&c.borrow(), Instant::now()));
            unsafe { MsgWaitForMultipleObjectsEx(0, std::ptr::null(), wait, QS_ALLINPUT, MWMO_INPUTAVAILABLE) };
            while unsafe { PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_REMOVE) } != 0 {
                if msg.message == WM_QUIT {
                    break 'outer;
                }
                if msg.message == WM_POKE {
                    W.with(|c| {
                        let mut w = c.borrow_mut();
                        if held() || !ENABLED.load(Ordering::SeqCst) {
                            hide(&mut w, FADE_FAST);
                        } else if w.target.is_none() {
                            set_target(&mut w, unsafe { GetForegroundWindow() });
                        } else if SKIP.lock().map(|s| w.target.as_ref().is_some_and(|t| geom::skipped(&t.exe, &s))).unwrap_or(false) {
                            drop_target(&mut w);
                        }
                    });
                    continue;
                }
                unsafe {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            W.with(|c| {
                let mut w = c.borrow_mut();
                if w.settle_at.is_some_and(|at| Instant::now() >= at) {
                    settle(&mut w);
                }
                check_pointer(&mut w);
            });
        }
        W.with(|c| {
            let mut w = c.borrow_mut();
            unhook_loc(&mut w);
            w.target = None;
            w.shown = false;
        });
        for h in hooks {
            unsafe { UnhookWinEvent(h) };
        }
        set_current(None);
    }
}

/// UI Automation for the allow-listed custom title bars, on its own MTA
/// thread (UIA calls are cross-process COM). Asked once at focus time (or
/// after a resize settles), never during a drag, with short timeouts.
#[cfg(windows)]
mod uia {
    use super::geom::{self, Rect};
    use std::sync::mpsc::{channel, Sender};
    use std::sync::{Mutex, OnceLock};
    use std::time::Duration;
    use windows::Win32::Foundation::POINT;
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::*;
    use windows::core::Interface;

    type Reply = Option<(Rect, Vec<Rect>)>;
    struct Ask {
        hwnd: isize,
        /// The caption band from DWM, when it gave a trustworthy one.
        band: Option<Rect>,
        frame: Rect,
        scale: f64,
        pid: u32,
        reply: Sender<Reply>,
    }

    static TX: OnceLock<Mutex<Sender<Ask>>> = OnceLock::new();

    fn sender() -> Option<Sender<Ask>> {
        let m = TX.get_or_init(|| {
            let (tx, rx) = channel::<Ask>();
            std::thread::Builder::new()
                .name("cove-traffic-uia".into())
                .spawn(move || {
                    unsafe {
                        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                    }
                    let auto: Option<IUIAutomation> = unsafe {
                        CoCreateInstance::<_, IUIAutomation>(&CUIAutomation8, None, CLSCTX_INPROC_SERVER)
                            .or_else(|_| CoCreateInstance::<_, IUIAutomation>(&CUIAutomation, None, CLSCTX_INPROC_SERVER))
                            .ok()
                    };
                    if let Some(a2) = auto.as_ref().and_then(|a| a.cast::<IUIAutomation2>().ok()) {
                        unsafe {
                            let _ = a2.SetConnectionTimeout(400);
                            let _ = a2.SetTransactionTimeout(250);
                        }
                    }
                    for ask in rx {
                        let r = auto.as_ref().and_then(|a| measure_now(a, &ask));
                        let _ = ask.reply.send(r);
                    }
                })
                .ok();
            Mutex::new(tx)
        });
        m.lock().ok().map(|s| s.clone())
    }

    /// The app's own caption buttons (minimise to close, screen px) at
    /// the frame's top-right corner, or `None` if UI Automation can't say
    /// confidently (then Cove skips the window).
    pub fn buttons(hwnd: isize, frame: Rect, scale: f64, pid: u32) -> Option<Rect> {
        let tx = sender()?;
        let (rtx, rrx) = channel();
        tx.send(Ask { hwnd, band: None, frame, scale, pid, reply: rtx }).ok()?;
        rrx.recv_timeout(Duration::from_millis(600)).ok().flatten().map(|(b, _)| b)
    }

    fn rect_of(e: &IUIAutomationElement) -> Option<Rect> {
        let rc = unsafe { e.CurrentBoundingRectangle() }.ok()?;
        Some(Rect::new(rc.left, rc.top, rc.right, rc.bottom))
    }

    fn at(a: &IUIAutomation, x: i32, y: i32) -> Option<IUIAutomationElement> {
        unsafe { a.ElementFromPoint(POINT { x, y }) }.ok()
    }

    fn measure_now(a: &IUIAutomation, ask: &Ask) -> Reply {
        let _ = (ask.hwnd, ask.band);
        Some((caption_by_uia(a, ask)?, Vec::new()))
    }

    /// The caption band from the native caption buttons UI Automation finds
    /// at the frame's top-right corner (locale-free: by type and position).
    fn caption_by_uia(a: &IUIAutomation, ask: &Ask) -> Option<Rect> {
        let f = ask.frame;
        let d = |v: f64| geom::dip(v, ask.scale);
        let close = at(a, f.r - d(12.0), f.t + d(8.0))?;
        if unsafe { close.CurrentProcessId() }.ok()? as u32 != ask.pid {
            return None;
        }
        if unsafe { close.CurrentControlType() }.ok()? != UIA_ButtonControlTypeId {
            return None;
        }
        let close_r = rect_of(&close)?;
        // Walk left over the other caption buttons (minimise, maximise).
        let mut leftmost = close_r.l;
        for _ in 0..3 {
            let Some(e) = at(a, leftmost - d(4.0), close_r.t + close_r.h() / 2) else { break };
            let (Ok(t), Some(rc)) = (unsafe { e.CurrentControlType() }, rect_of(&e)) else { break };
            if t != UIA_ButtonControlTypeId || rc.h() != close_r.h() || rc.r > leftmost + 1 || rc.w() > d(80.0) {
                break;
            }
            leftmost = rc.l;
        }
        geom::buttons_from_close(f, close_r, leftmost, ask.scale)
    }

}

#[cfg(not(windows))]
mod imp {
    pub fn start() {}
    pub fn stop() {}
    pub fn poke() {}
    pub fn click(_: &str, _: bool) {}
    pub fn style_overlay(_: &tauri::WebviewWindow) {}
    pub fn show_overlay(_: &tauri::WebviewWindow, _: &super::Look) {}
    pub fn hide_overlay(_: &tauri::WebviewWindow) {}
}

#[cfg(test)]
mod tests {
    use super::geom::*;

    // 150 % scale, like the user's screen.
    const S: f64 = 1.5;

    fn std_window() -> (Rect, Rect) {
        // Window rect includes 7 px invisible borders on left/right/bottom.
        let window = Rect::new(93, 100, 1307, 907);
        let frame = Rect::new(100, 100, 1300, 900);
        (window, frame)
    }

    #[test]
    fn cluster_is_three_dots_and_two_gaps() {
        assert_eq!(cluster_size(1.0), (56, 16));
        assert_eq!(cluster_size(1.5), (84, 24));
        assert_eq!(cluster_size(2.0), (112, 32));
        let c = dot_circles(1.0);
        assert_eq!(c[0], (8, 8, 6));
        assert_eq!(c[1].0 - c[0].0, 20); // 12 dot + 8 gap
        assert_eq!(c[2].0 - c[1].0, 20);
    }

    #[test]
    fn dwm_caption_band_is_validated() {
        let (window, frame) = std_window();
        // Three 46 DIP buttons at the right, 32 DIP tall (window-relative).
        let buttons = Rect::new(1207 - 207, 0, 1207, 48);
        let band = caption_from_dwm(window, frame, buttons, S).unwrap();
        assert_eq!(band, Rect::new(100, 100, 1093 - 0, 148));
        // Empty, too short, too tall or not at the right edge: not trusted.
        assert!(caption_from_dwm(window, frame, Rect::default(), S).is_none());
        assert!(caption_from_dwm(window, frame, Rect::new(1000, 0, 1207, 10), S).is_none());
        assert!(caption_from_dwm(window, frame, Rect::new(1000, 0, 1207, 200), S).is_none());
        assert!(caption_from_dwm(window, frame, Rect::new(500, 0, 700, 48), S).is_none());
        assert!(caption_from_dwm(window, frame, Rect::new(1000, 300, 1207, 348), S).is_none());
    }

    #[test]
    fn standard_frame_means_client_below_caption() {
        let band = Rect::new(100, 100, 1093, 148);
        assert!(is_standard(band, 148));
        assert!(is_standard(band, 150));
        // Chrome/Explorer draw into the title bar: client starts at the top.
        assert!(!is_standard(band, 101));
    }

    #[test]
    fn places_at_the_left_centred_in_the_band() {
        let (_, frame) = std_window();
        let band = Rect::new(100, 100, 1093, 148);
        let spot = place(frame, band, &[], S).unwrap();
        // 13 DIP inset to the first dot, minus the 2 DIP padding.
        assert_eq!(spot.l, 100 + dip(11.0, S));
        assert_eq!((spot.w(), spot.h()), cluster_size(S));
        let (_, cy) = spot.center();
        assert!((cy - 124).abs() <= 1, "centred on the 48 px band, got {cy}");
        assert!(spot.r < band.r, "never reaches the native buttons");
    }

    #[test]
    fn never_covers_a_tab_and_skips_when_no_space() {
        let (_, frame) = std_window();
        let band = Rect::new(100, 100, 1093, 148);
        // Chrome-like: the first tab starts 12 px in. No safe space: skip.
        let tab = Rect::new(112, 108, 400, 148);
        assert!(place(frame, band, &[tab], S).is_none());
        // A tab-search button at the far left: skip too.
        let btn = Rect::new(104, 110, 140, 140);
        assert!(place(frame, band, &[btn], S).is_none());
        // Edge with vertical tabs / a wide empty lead: the dots fit.
        let far_tab = Rect::new(260, 108, 500, 148);
        let spot = place(frame, band, &[far_tab], S).unwrap();
        assert!(spot.r + dip(CLEAR, S) <= far_tab.l);
        // An obstacle just inside the clear margin still blocks.
        let near = Rect::new(spot.r + dip(CLEAR, S) - 1, 108, 600, 148);
        assert!(place(frame, band, &[near], S).is_none());
        // Obstacles below the band (the toolbar row) don't matter.
        let toolbar = Rect::new(100, 150, 400, 190);
        assert!(place(frame, band, &[toolbar], S).is_some());
    }

    #[test]
    fn skips_narrow_or_flat_title_bars() {
        let frame = Rect::new(0, 0, 90, 300);
        assert!(place(frame, Rect::new(0, 0, 60, 48), &[], S).is_none());
        let frame = Rect::new(0, 0, 900, 300);
        assert!(place(frame, Rect::new(0, 0, 800, 10), &[], S).is_none());
        assert!(place(frame, Rect::default(), &[], S).is_none());
    }

    #[test]
    fn hover_zone_is_the_top_left_corner() {
        let (_, frame) = std_window();
        let band = Rect::new(100, 100, 1093, 148);
        let spot = place(frame, band, &[], S).unwrap();
        let z = hover_zone(frame, band, spot, &[], S);
        assert_eq!(z.r, 100 + dip(ZONE_W, S));
        assert!(z.contains(spot.l, spot.t) && z.contains(spot.r - 1, spot.b - 1));
        assert!(z.b >= band.b);
        // Does not reach into the first tab.
        let tab = Rect::new(260, 108, 500, 148);
        let spot = place(frame, band, &[tab], S).unwrap();
        let z = hover_zone(frame, band, spot, &[tab], S);
        assert!(z.r <= tab.l);
        assert!(!z.contains(300, 120));
    }

    #[test]
    fn close_button_band_for_custom_title_bars() {
        let frame = Rect::new(0, 0, 1600, 1000);
        let close = Rect::new(1531, 0, 1600, 51);
        let band = caption_from_close(frame, close, 1393, S).unwrap();
        assert_eq!(band, Rect::new(0, 0, 1393, 51));
        // Not at the corner, or too tall: not confident.
        assert!(caption_from_close(frame, Rect::new(800, 0, 869, 51), 800, S).is_none());
        assert!(caption_from_close(frame, Rect::new(1531, 0, 1600, 300), 1393, S).is_none());
    }

    #[test]
    fn covers_exactly_the_native_buttons() {
        let (window, frame) = std_window();
        let buttons = Rect::new(1207 - 207, 0, 1207, 48);
        let b = buttons_from_dwm(window, frame, buttons, S).unwrap();
        assert_eq!(b, Rect::new(1093, 100, 1300, 148));
        let c = cover(b);
        assert_eq!(c, Rect::new(1093, 101, 1299, 148));
        // Custom title bar: from the close button UI Automation found.
        let frame = Rect::new(0, 0, 1600, 1000);
        let b = buttons_from_close(frame, Rect::new(1531, 0, 1600, 51), 1393, S).unwrap();
        assert_eq!(b, Rect::new(1393, 0, 1600, 51));
    }

    #[test]
    fn dots_sit_at_the_right_end_of_the_patch() {
        let d = cover_dots(207, 47, S);
        // Zoom dot centre: 14 + 6 DIP from the right edge.
        assert_eq!(d[2].0, 207 - dip(20.0, S));
        assert_eq!(d[2].0 - d[1].0, dip(20.0, S));
        assert_eq!(d[1].0 - d[0].0, dip(20.0, S));
        assert!(d[0].0 - d[0].2 > 0, "all three fit inside the patch");
        assert_eq!(d[0].1, 23);
    }

    #[test]
    fn fullscreen_detection() {
        let mon = Rect::new(0, 0, 2560, 1600);
        assert!(covers_monitor(Rect::new(0, 0, 2560, 1600), mon));
        assert!(covers_monitor(Rect::new(-8, -8, 2568, 1608), mon));
        assert!(!covers_monitor(Rect::new(0, 38, 2560, 1550), mon));
    }

    #[test]
    fn allow_list_and_skip_list() {
        assert_eq!(kind(r"C:\Windows\explorer.exe", "CabinetWClass"), Kind::Explorer);
        assert_eq!(kind(r"C:\Windows\explorer.exe", "Shell_TrayWnd"), Kind::Other);
        assert_eq!(kind(r"C:\Program Files\Google\Chrome\Application\chrome.exe", "Chrome_WidgetWin_1"), Kind::Chrome);
        assert_eq!(kind(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe", "Chrome_WidgetWin_1"), Kind::Edge);
        assert_eq!(kind(r"C:\Users\k\AppData\Local\Programs\Microsoft VS Code\Code.exe", "Chrome_WidgetWin_1"), Kind::VsCode);
        assert_eq!(kind(r"C:\Windows\notepad.exe", "Notepad"), Kind::Other);
        let list = vec!["chrome.exe".to_string()];
        assert!(skipped(r"C:\X\Chrome.EXE", &list));
        assert!(!skipped(r"C:\X\msedge.exe", &list));
        assert!(!skipped("", &list));
        assert_eq!(exe_key(r"C:\A\B\Code.exe"), "code.exe");
    }

    #[test]
    fn integrity_rules() {
        let medium = 0x2000;
        assert!(can_act(medium, Some(0x2000)));
        assert!(can_act(medium, Some(0x1000))); // low (sandboxed) is fine
        assert!(!can_act(medium, Some(0x3000))); // elevated: skip
        assert!(!can_act(medium, None)); // unknown: skip
    }
}
