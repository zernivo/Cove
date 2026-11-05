//! Cove Dock: pinned apps plus whatever is running, built from the same
//! "real window" rules Alt+Tab uses. Taskbar auto-hide goes only through
//! its documented switch (SHAppBarMessage) and is put back afterwards.

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct DockItem {
    /// "pin" | "running" | "trash"
    pub kind: &'static str,
    pub name: String,
    /// AppID for pins, exe path for running apps.
    pub target: String,
    /// Window handles (front to back) when running.
    pub windows: Vec<isize>,
    pub active: bool,
    /// One of its windows asked for attention (FlashWindow) and hasn't
    /// been brought to the front yet.
    pub attention: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,
    /// Every icon source has been tried and none gave a picture: the dock
    /// draws a letter tile instead of waiting for ever.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub no_icon: bool,
    /// Recycle Bin only: whether it holds anything (the icon follows).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DockState {
    pub items: Vec<DockItem>,
}

pub struct Win {
    pub hwnd: isize,
    /// Full exe path, or "" when Windows won't say (an app running as admin).
    pub exe: String,
    pub title: String,
    /// The window's AppUserModelID when it has one (Store apps, Chrome,
    /// Teams...): the same ID the taskbar groups and draws icons by.
    pub aumid: Option<String>,
    /// The icon the app asked the taskbar to show ("file,index"), e.g. a
    /// Chrome profile's icon with its picture.
    pub icon_res: Option<String>,
}

impl Win {
    pub fn is_uwp_frame(&self) -> bool {
        stem(&self.exe) == "applicationframehost"
    }
    /// What windows of the same app share.
    pub fn app_key(&self) -> String {
        if is_explorer_win(self) {
            return EXPLORER_ID.to_lowercase();
        }
        match (&self.aumid, self.exe.is_empty() || self.is_uwp_frame()) {
            (Some(a), _) => a.to_lowercase(),
            (None, false) => self.exe.to_lowercase(),
            (None, true) => format!("hwnd:{}", self.hwnd),
        }
    }
    /// Icon sources, best first: the app's Start entry, its exe, the
    /// window's own icon. Never a letter.
    pub fn icon_keys(&self) -> Vec<String> {
        let mut k = Vec::new();
        if let Some(r) = &self.icon_res {
            k.push(format!("res:{r}"));
        }
        if let Some(a) = &self.aumid {
            k.push(format!("shell:AppsFolder\\{a}"));
        }
        if !self.exe.is_empty() && !self.is_uwp_frame() {
            k.push(self.exe.clone());
            // The exe's own icon resource (SHGetFileInfo / PrivateExtractIcons),
            // for apps whose shell item gives no picture.
            k.push(format!("exeicon:{}", self.exe));
        }
        k.push(format!("hwnd:{}", self.hwnd));
        k
    }
}

/// The first ready icon among `keys`; queues the first untried one.
pub fn icon_from(keys: &[String], jobs: &mut Vec<String>) -> Option<String> {
    icon_or_none(keys, jobs).0
}

/// As `icon_from`, plus whether every key has been tried without a picture
/// (then nothing more will come: show a letter tile).
pub fn icon_or_none(keys: &[String], jobs: &mut Vec<String>) -> (Option<String>, bool) {
    for k in keys {
        if let Some(u) = crate::icons::get(k) {
            return (Some(u), false);
        }
        if !crate::icons::known(k) {
            jobs.push(k.clone());
            return (None, false);
        }
    }
    (None, true)
}

pub fn stem(exe: &str) -> String {
    std::path::Path::new(exe)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

/// Does a running exe belong to a pinned AppID / name? A heuristic: Start
/// AppIDs usually carry the exe or product name ("Chrome", "MSEdge",
/// "...\\Code.exe", "Microsoft.Windows.Explorer").
/// Does this window belong to this pin? The taskbar's rule: same
/// AppUserModelID when either side has one (so two Chrome profiles stay two
/// icons), otherwise the same program.
pub fn matches_win(pin: &crate::settings::Pin, w: &Win) -> bool {
    // File Explorer: its windows carry the AppID Microsoft.Windows.Explorer
    // (or none), while the pin is often a shortcut or explorer.exe. The
    // taskbar treats them as one app, and so does the dock.
    if is_explorer_win(w) {
        return is_explorer_pin(pin);
    }
    let pin_id = pin.aumid.as_deref().or((!is_exe_pin(&pin.target)).then_some(pin.target.as_str()));
    if let (Some(p), Some(a)) = (pin_id, &w.aumid) {
        if p.eq_ignore_ascii_case(a) {
            return true;
        }
    }
    if pin.aumid.is_some() || w.aumid.is_some() && is_exe_pin(&pin.target) {
        return false;
    }
    if let Some(e) = &pin.exe {
        return !w.exe.is_empty() && e.eq_ignore_ascii_case(&w.exe);
    }
    matches(&pin.target, &pin.name, &w.exe)
}

pub const EXPLORER_ID: &str = "Microsoft.Windows.Explorer";

/// A File Explorer window (folder windows of explorer.exe).
pub fn is_explorer_win(w: &Win) -> bool {
    w.aumid.as_deref().is_some_and(|a| a.eq_ignore_ascii_case(EXPLORER_ID)) || stem(&w.exe) == "explorer"
}

/// A pin that opens File Explorer: its AppID, explorer.exe, or the
/// "File Explorer" shortcut the taskbar keeps.
pub fn is_explorer_pin(pin: &crate::settings::Pin) -> bool {
    let id = |s: &str| s.eq_ignore_ascii_case(EXPLORER_ID);
    if pin.aumid.as_deref().is_some_and(id) || id(&pin.target) {
        return true;
    }
    if pin.aumid.is_some() {
        return false; // some other app's shortcut
    }
    let exe_is = |p: &str| stem(p) == "explorer" && p.to_lowercase().ends_with(".exe");
    if pin.exe.as_deref().is_some_and(exe_is) || exe_is(&pin.target) {
        return true;
    }
    let t = pin.target.to_lowercase();
    t.ends_with(".lnk") && matches!(stem(&t).as_str(), "file explorer" | "explorer" | "windows explorer")
}

pub fn matches(pin_target: &str, pin_name: &str, exe: &str) -> bool {
    let s = stem(exe);
    if s.len() < 3 || s == "applicationframehost" {
        return false;
    }
    let t = pin_target.to_lowercase();
    let n = pin_name.to_lowercase().replace(' ', "");
    t.contains(&s) || n.contains(&s) || (s == "code" && t.contains("visualstudiocode"))
}

/// Pins are Start AppIDs, or a path (an exe kept from the running list, or
/// a shortcut brought over from the taskbar).
pub fn is_exe_pin(target: &str) -> bool {
    let t = target.to_lowercase();
    (t.ends_with(".exe") || t.ends_with(".lnk")) && std::path::Path::new(target).is_absolute()
}

/// Packaged (Store / MSIX / system) apps are named by "Family!AppId": their
/// best picture is the package's own logo PNG (icons.rs reads the manifest).
pub fn package_logo_key(target: &str) -> Option<String> {
    let (family, app) = target.split_once('!')?;
    (!family.is_empty() && !app.is_empty() && family.contains('_') && !is_exe_pin(target)).then(|| format!("pkglogo:{target}"))
}

fn pin_icon_path(target: &str) -> String {
    if is_exe_pin(target) {
        target.to_string()
    } else {
        format!("shell:AppsFolder\\{target}")
    }
}

pub fn launch(target: &str) -> Result<(), String> {
    if is_exe_pin(target) {
        crate::search::open("file", target)
    } else {
        crate::search::open("app", target)
    }
}

/// Windows that asked for attention, learned from the documented shell
/// hook (HSHELL_FLASH). Read-only: Cove never flashes or changes them.
static ATTN: std::sync::Mutex<Vec<isize>> = std::sync::Mutex::new(Vec::new());
static ATTN_WATCH: std::sync::Once = std::sync::Once::new();

pub const HSHELL_WINDOWDESTROYED: usize = 2;
pub const HSHELL_WINDOWACTIVATED: usize = 4;
pub const HSHELL_RUDEAPPACTIVATED: usize = 0x8004;
pub const HSHELL_FLASH: usize = 0x8006;

/// Apply one shell-hook event to the attention list.
pub fn shell_event(list: &mut Vec<isize>, code: usize, hwnd: isize) {
    match code {
        HSHELL_FLASH => {
            if !list.contains(&hwnd) {
                list.push(hwnd);
            }
        }
        HSHELL_WINDOWACTIVATED | HSHELL_RUDEAPPACTIVATED | HSHELL_WINDOWDESTROYED => list.retain(|&h| h != hwnd),
        _ => {}
    }
}

/// Keep only windows that still exist and aren't in front.
fn prune_attention(list: &mut Vec<isize>, alive: &[isize], fg: isize) {
    list.retain(|h| *h != fg && alive.contains(h));
}

pub fn state(pins: &[crate::settings::Pin], stacks: &[String]) -> DockState {
    ATTN_WATCH.call_once(imp::attention_watch);
    let wins = imp::windows();
    let fg = imp::foreground();
    let attn: Vec<isize> = match ATTN.lock() {
        Ok(mut a) => {
            let alive: Vec<isize> = wins.iter().map(|w| w.hwnd).collect();
            prune_attention(&mut a, &alive, fg);
            a.clone()
        }
        Err(_) => Vec::new(),
    };
    let wants = |hw: &[isize]| hw.iter().any(|h| attn.contains(h)) && !hw.contains(&fg);
    let mut used = vec![false; wins.len()];
    let mut items = Vec::new();
    let mut icon_jobs = Vec::new();

    for p in pins {
        let mut hw = Vec::new();
        for (i, w) in wins.iter().enumerate() {
            if !used[i] && matches_win(p, w) {
                used[i] = true;
                hw.push(w.hwnd);
            }
        }
        // The pin's own icon, then its program, then (when it's running)
        // whatever its window offers: exe, then the window's own icon.
        let mut keys = package_logo_key(&p.target).into_iter().collect::<Vec<_>>();
        keys.push(pin_icon_path(&p.target));
        if let Some(e) = &p.exe {
            keys.push(e.clone());
        }
        if let Some(w) = hw.first().and_then(|h| wins.iter().find(|w| w.hwnd == *h)) {
            for k in w.icon_keys() {
                if !keys.contains(&k) {
                    keys.push(k);
                }
            }
        }
        let (icon, no_icon) = icon_or_none(&keys, &mut icon_jobs);
        items.push(DockItem {
            kind: "pin",
            name: p.name.clone(),
            target: p.target.clone(),
            active: hw.contains(&fg),
            attention: wants(&hw),
            windows: hw,
            icon,
            no_icon,
            full: None,
        });
    }

    // Running apps that aren't pinned, grouped by exe (Store apps by window).
    let mut groups: Vec<DockItem> = Vec::new();
    let mut keys: Vec<String> = Vec::new();
    for (i, w) in wins.iter().enumerate() {
        if used[i] {
            continue;
        }
        let key = w.app_key();
        if let Some(gi) = keys.iter().position(|k| *k == key) {
            let g = &mut groups[gi];
            g.windows.push(w.hwnd);
            g.active |= w.hwnd == fg;
            continue;
        }
        keys.push(key);
        let mut wkeys: Vec<String> = w.aumid.as_deref().and_then(package_logo_key).into_iter().collect();
        wkeys.extend(w.icon_keys());
        let (icon, no_icon) = icon_or_none(&wkeys, &mut icon_jobs);
        groups.push(DockItem {
            kind: "running",
            name: crate::appnames::friendly(&w.exe, w.aumid.as_deref(), &w.title),
            // Store apps relaunch through their AppID; others through the exe.
            target: match (&w.aumid, w.is_uwp_frame()) {
                (Some(a), true) => a.clone(),
                _ => w.exe.clone(),
            },
            windows: vec![w.hwnd],
            active: w.hwnd == fg,
            attention: false,
            icon,
            no_icon,
            full: None,
        });
    }
    for g in groups.iter_mut() {
        g.attention = wants(&g.windows);
    }
    items.extend(groups);

    // Folder stacks sit between the apps and the Recycle Bin.
    for key in stacks {
        let (Some(label), Some(dir)) = (crate::stacks::label(key), crate::stacks::folder(key)) else { continue };
        let path = dir.to_string_lossy().to_string();
        let (icon, no_icon) = icon_or_none(&[path], &mut icon_jobs);
        items.push(DockItem { kind: "stack", name: label.into(), target: key.clone(), windows: vec![], active: false, attention: false, icon, no_icon, full: None });
    }

    let trash = "::{645FF040-5081-101B-9F08-08002B2CF60E}".to_string();
    // The bin's shell item, then the same through its shell: name, then the
    // stock Recycle Bin icon Windows draws on the desktop.
    // Empty and full are Windows' own stock icons (SIID_RECYCLER /
    // SIID_RECYCLERFULL), so the picture follows the bin's state.
    let full = imp::bin_full();
    let stock = if full == Some(true) { "stock:recyclerfull" } else { "stock:recycler" };
    let bin_keys = [stock.to_string(), trash.clone(), "shell:RecycleBinFolder".to_string()];
    let (icon, no_icon) = icon_or_none(&bin_keys, &mut icon_jobs);
    items.push(DockItem { kind: "trash", name: "Recycle Bin".into(), target: trash, windows: vec![], active: false, attention: false, icon, no_icon, full });

    if !icon_jobs.is_empty() {
        std::thread::spawn(move || crate::icons::fill(icon_jobs));
    }
    DockState { items }
}

/// Cove's own Settings window, while it's open: Cove sits in the dock like
/// any other app (the window list skips Cove's own process).
pub fn cove_item(hwnd: isize) -> DockItem {
    let exe = std::env::current_exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
    let mut jobs = Vec::new();
    let (icon, no_icon) = icon_or_none(&[exe, format!("hwnd:{hwnd}")], &mut jobs);
    if !jobs.is_empty() {
        std::thread::spawn(move || crate::icons::fill(jobs));
    }
    DockItem {
        kind: "cove",
        name: "Cove".into(),
        target: "cove".into(),
        windows: vec![hwnd],
        active: imp::foreground() == hwnd,
        attention: false,
        icon,
        no_icon,
        full: None,
    }
}

/// Where Cove goes: after the running apps, before folder stacks and the bin.
pub fn insert_cove(items: &mut Vec<DockItem>, cove: DockItem) {
    let at = items.iter().position(|i| i.kind == "stack" || i.kind == "trash").unwrap_or(items.len());
    items.insert(at, cove);
}

pub use imp::{
    activate, bin_empty, close_windows, cursor, foreground, minimize, monitor_at, overlaps, taskbar_autohide, taskbar_is_visible,
    taskbar_visible, windows, tile, work_area, zoom,
};

/// While Cove desktop is on, the Windows taskbar stays hidden (the dock
/// replaces it). Checked by the main loop so a restarted Explorer, or a
/// Win key press that brings it back, is hidden again.
pub static TASKBAR_GONE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Watchdog side of the takeover: after Cove's process `pid` ends, for any
/// reason, the taskbar comes back and Cove's reserved strips are released. Runs as `cove --watchdog <pid> <settings>`.
pub fn watchdog(pid: u32, settings: &std::path::Path) {
    imp::wait_for_exit(pid);
    // The space the menu bar and dock reserved goes back to the apps.
    crate::appbar::release_recorded(settings);
    taskbar_visible(true);
    let mut s = crate::settings::Store::load(settings.to_path_buf());
    if let Some(prev) = s.data.taskbar_prev_autohide.take() {
        taskbar_autohide(Some(prev));
        let _ = s.save();
    }
}

/// One display: its full rectangle and its work area (physical px,
/// left/top/right/bottom).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mon {
    pub monitor: (i32, i32, i32, i32),
    pub work: (i32, i32, i32, i32),
}

impl Mon {
    pub fn centre(&self) -> (i32, i32) {
        ((self.monitor.0 + self.monitor.2) / 2, (self.monitor.1 + self.monitor.3) / 2)
    }
}

/// The main display (the point 0,0 always lies on it).
pub fn primary() -> Option<Mon> {
    monitor_at(0, 0)
}

/// The display the dock sits on, and its scale (x100) when it was placed.
static DOCK_MON: std::sync::Mutex<Option<(Mon, i32)>> = std::sync::Mutex::new(None);

pub fn dock_monitor() -> Option<Mon> {
    DOCK_MON.lock().ok().and_then(|g| g.map(|(m, _)| m))
}

/// Record where the dock should be; true when that changed (move or re-size it).
pub fn set_dock_monitor(m: Mon, scale100: i32) -> bool {
    let mut g = DOCK_MON.lock().unwrap();
    let changed = *g != Some((m, scale100));
    *g = Some((m, scale100));
    changed
}

/// Pointer pushed against the bottom edge of a display for long enough.
pub struct EdgeWatch {
    ticks: u32,
    mon: Option<Mon>,
}

impl EdgeWatch {
    pub const fn new() -> Self {
        EdgeWatch { ticks: 0, mon: None }
    }
    /// Feed one pointer sample; returns the display once the pointer has
    /// rested at its bottom edge for `need` samples in a row.
    pub fn feed(&mut self, y: i32, mon: Option<Mon>, need: u32) -> Option<Mon> {
        match mon {
            Some(m) if y >= m.monitor.3 - 3 => {
                if self.mon.map(|p| p.monitor) == Some(m.monitor) {
                    self.ticks += 1;
                } else {
                    self.ticks = 1;
                    self.mon = Some(m);
                }
            }
            _ => {
                self.ticks = 0;
                self.mon = None;
            }
        }
        if self.ticks >= need {
            self.mon
        } else {
            None
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::Win;
    use std::ffi::c_void;
    use windows_sys::Win32::Foundation::{CloseHandle, BOOL, HWND, LPARAM, RECT};
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows_sys::Win32::System::Threading::{
        GetCurrentProcessId, OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
        PROCESS_QUERY_LIMITED_INFORMATION,
    };
    use windows_sys::Win32::UI::Shell::{SHAppBarMessage, APPBARDATA};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetClassNameW, GetCursorPos, GetForegroundWindow, GetWindow, GetWindowLongW,
        GetWindowRect, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic,
        IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, SystemParametersInfoW,
        GWL_EXSTYLE, GW_OWNER, SPI_GETWORKAREA, SW_HIDE, SW_MINIMIZE, SW_RESTORE, SW_SHOWNOACTIVATE, WM_CLOSE, WS_EX_TOOLWINDOW,
    };
    use windows_sys::Win32::Foundation::{LRESULT, WPARAM};
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW, RegisterClassW,
        RegisterShellHookWindow, RegisterWindowMessageW, TranslateMessage, MSG, WNDCLASSW,
    };
    use std::sync::atomic::{AtomicU32, Ordering};

    static SHELLHOOK: AtomicU32 = AtomicU32::new(0);

    unsafe extern "system" fn shell_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
        let id = SHELLHOOK.load(Ordering::Relaxed);
        if id != 0 && msg == id {
            if let Ok(mut a) = super::ATTN.lock() {
                super::shell_event(&mut a, wp, lp);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, wp, lp)
    }

    /// A hidden window registered for shell-hook messages (documented
    /// RegisterShellHookWindow), so the dock can see FlashWindow requests.
    pub fn attention_watch() {
        std::thread::spawn(|| unsafe {
            let name: Vec<u16> = "CoveShellHook\0".encode_utf16().collect();
            let inst = GetModuleHandleW(std::ptr::null());
            let mut wc: WNDCLASSW = std::mem::zeroed();
            wc.lpfnWndProc = Some(shell_proc);
            wc.hInstance = inst;
            wc.lpszClassName = name.as_ptr();
            RegisterClassW(&wc);
            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW,
                name.as_ptr(),
                name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                inst,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return;
            }
            let hook: Vec<u16> = "SHELLHOOK\0".encode_utf16().collect();
            SHELLHOOK.store(RegisterWindowMessageW(hook.as_ptr()), Ordering::Relaxed);
            if RegisterShellHookWindow(hwnd) == 0 {
                DestroyWindow(hwnd);
                return;
            }
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        });
    }

    fn class(hwnd: HWND) -> String {
        let mut buf = [0u16; 64];
        let n = unsafe { GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32) };
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }

    unsafe fn is_real(hwnd: HWND) -> bool {
        if IsWindowVisible(hwnd) == 0 || !GetWindow(hwnd, GW_OWNER).is_null() {
            return false;
        }
        if (GetWindowLongW(hwnd, GWL_EXSTYLE) as u32) & WS_EX_TOOLWINDOW != 0 {
            return false;
        }
        let mut cloaked: u32 = 0;
        let hr = DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as _, &mut cloaked as *mut u32 as *mut c_void, 4);
        if hr == 0 && cloaked != 0 {
            return false;
        }
        if GetWindowTextLengthW(hwnd) == 0 {
            return false;
        }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut r) != 0 && (r.right - r.left <= 1 || r.bottom - r.top <= 1) {
            return IsIconic(hwnd) != 0;
        }
        !matches!(class(hwnd).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd")
    }

    fn exe_of(pid: u32) -> Option<String> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() {
                return None;
            }
            let mut buf = [0u16; 1024];
            let mut len = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(h, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len);
            CloseHandle(h);
            (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
        }
    }

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = &mut *(lparam as *mut Vec<HWND>);
        if is_real(hwnd) {
            out.push(hwnd);
        }
        1
    }

    pub fn windows() -> Vec<Win> {
        let mut hs: Vec<HWND> = Vec::new();
        unsafe { EnumWindows(Some(cb), &mut hs as *mut Vec<HWND> as LPARAM) };
        let me = unsafe { GetCurrentProcessId() };
        hs.into_iter()
            .filter_map(|h| {
                let mut pid = 0u32;
                unsafe { GetWindowThreadProcessId(h, &mut pid) };
                if pid == me {
                    return None;
                }
                let exe = exe_of(pid).unwrap_or_default();
                let n = unsafe { GetWindowTextLengthW(h) };
                let mut buf = vec![0u16; n as usize + 1];
                let got = unsafe { GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32) };
                let aumid = crate::shellprops::window_aumid(h as isize);
                let icon_res = crate::shellprops::window_icon_resource(h as isize);
                Some(Win { hwnd: h as isize, exe, title: String::from_utf16_lossy(&buf[..got.max(0) as usize]), aumid, icon_res })
            })
            .collect()
    }

    pub fn foreground() -> isize {
        unsafe { GetForegroundWindow() as isize }
    }

    fn valid(h: isize) -> Option<HWND> {
        let h = h as HWND;
        (unsafe { is_real(h) }).then_some(h)
    }

    pub fn activate(h: isize) {
        if let Some(h) = valid(h) {
            unsafe {
                if IsIconic(h) != 0 {
                    ShowWindow(h, SW_RESTORE);
                }
                SetForegroundWindow(h);
            }
        }
    }

    pub fn minimize(h: isize) {
        if let Some(h) = valid(h) {
            unsafe { ShowWindow(h, SW_MINIMIZE) };
        }
    }

    /// "Zoom" in the app menu: maximise, or restore when already maximised.
    /// Does the Recycle Bin hold anything (all drives)? None if unknown.
    pub fn bin_full() -> Option<bool> {
        use windows_sys::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
        unsafe {
            let mut info: SHQUERYRBINFO = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<SHQUERYRBINFO>() as u32;
            (SHQueryRecycleBinW(std::ptr::null(), &mut info) >= 0).then_some(info.i64NumItems > 0)
        }
    }

    /// Empty the Recycle Bin. Windows asks for confirmation itself.
    pub fn bin_empty() -> Result<(), String> {
        use windows_sys::Win32::UI::Shell::SHEmptyRecycleBinW;
        let hr = unsafe { SHEmptyRecycleBinW(std::ptr::null_mut(), std::ptr::null(), 0) };
        // S_OK, or the user said no / the bin was already empty.
        if hr >= 0 || hr == 0x8000_FFFFu32 as i32 || hr == 0x8007_04C7u32 as i32 { Ok(()) } else { Err(format!("Couldn't empty the Recycle Bin ({hr:#x}).")) }
    }

    /// Put a window on the left or right half of its display's work area.
    pub fn tile(h: isize, left: bool) {
        use windows_sys::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST as NEAREST};
        use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_NOZORDER};
        let Some(h) = valid(h) else { return };
        unsafe {
            let m = MonitorFromWindow(h, NEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(m, &mut mi) == 0 {
                return;
            }
            let w = mi.rcWork;
            let half = (w.right - w.left) / 2;
            ShowWindow(h, SW_RESTORE);
            let x = if left { w.left } else { w.left + half };
            let width = if left { half } else { w.right - w.left - half };
            SetWindowPos(h, std::ptr::null_mut(), x, w.top, width, w.bottom - w.top, SWP_NOZORDER | SWP_NOACTIVATE);
            SetForegroundWindow(h);
        }
    }

    pub fn zoom(h: isize) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{IsZoomed, SW_MAXIMIZE};
        if let Some(h) = valid(h) {
            unsafe {
                if IsZoomed(h) != 0 {
                    ShowWindow(h, SW_RESTORE);
                } else {
                    ShowWindow(h, SW_MAXIMIZE);
                }
            }
        }
    }

    /// Does any real, visible, non-minimised window (not Cove's bars) cover
    /// any part of `rc` (physical px)? The dock hides itself while one does.
    /// Cove's own Settings window ("Cove") counts like any app.
    pub fn overlaps(rc: (i32, i32, i32, i32)) -> bool {
        use windows_sys::Win32::Graphics::Dwm::DWMWA_EXTENDED_FRAME_BOUNDS;
        unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let out = &mut *(lparam as *mut Vec<HWND>);
            if is_real(hwnd) && IsIconic(hwnd) == 0 {
                out.push(hwnd);
            }
            1
        }
        let mut hs: Vec<HWND> = Vec::new();
        unsafe { EnumWindows(Some(each), &mut hs as *mut Vec<HWND> as LPARAM) };
        let me = unsafe { GetCurrentProcessId() };
        hs.into_iter().any(|h| unsafe {
            let mut pid = 0u32;
            GetWindowThreadProcessId(h, &mut pid);
            if pid == me {
                let n = GetWindowTextLengthW(h);
                let mut buf = vec![0u16; n as usize + 1];
                let got = GetWindowTextW(h, buf.as_mut_ptr(), buf.len() as i32);
                if String::from_utf16_lossy(&buf[..got.max(0) as usize]) != "Cove" {
                    return false;
                }
            }
            // The visible frame, without the invisible resize borders.
            let mut r: RECT = std::mem::zeroed();
            let ok = DwmGetWindowAttribute(
                h,
                DWMWA_EXTENDED_FRAME_BOUNDS as _,
                &mut r as *mut RECT as *mut c_void,
                std::mem::size_of::<RECT>() as u32,
            ) == 0
                || GetWindowRect(h, &mut r) != 0;
            ok && r.left < rc.2 && r.right > rc.0 && r.top < rc.3 && r.bottom > rc.1
        })
    }

    pub fn close_windows(hs: &[isize]) {
        for &h in hs {
            if let Some(h) = valid(h) {
                unsafe { PostMessageW(h, WM_CLOSE, 0, 0) };
            }
        }
    }

    pub fn cursor() -> Option<(i32, i32)> {
        let mut p = POINT { x: 0, y: 0 };
        (unsafe { GetCursorPos(&mut p) } != 0).then_some((p.x, p.y))
    }

    /// The display containing (or nearest to) a point.
    pub fn monitor_at(x: i32, y: i32) -> Option<super::Mon> {
        unsafe {
            let hm = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(hm, &mut mi) == 0 {
                return None;
            }
            let r = |r: RECT| (r.left, r.top, r.right, r.bottom);
            Some(super::Mon { monitor: r(mi.rcMonitor), work: r(mi.rcWork) })
        }
    }

    /// Primary monitor work area (physical px): left, top, right, bottom.
    pub fn work_area() -> Option<(i32, i32, i32, i32)> {
        let mut r: RECT = unsafe { std::mem::zeroed() };
        let ok = unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut RECT as *mut c_void, 0) };
        (ok != 0).then_some((r.left, r.top, r.right, r.bottom))
    }

    const ABM_GETSTATE: u32 = 4;
    const ABM_SETSTATE: u32 = 10;
    const ABS_AUTOHIDE: isize = 1;
    const ABS_ALWAYSONTOP: isize = 2;

    fn trays() -> Vec<HWND> {
        unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
            let out = &mut *(lparam as *mut Vec<HWND>);
            // Cove's own stand-in tray (tray.rs) shares the class: skip it.
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, &mut pid);
            if matches!(class(hwnd).as_str(), "Shell_TrayWnd" | "Shell_SecondaryTrayWnd") && pid != std::process::id() {
                out.push(hwnd);
            }
            1
        }
        let mut v: Vec<HWND> = Vec::new();
        unsafe { EnumWindows(Some(each), &mut v as *mut Vec<HWND> as LPARAM) };
        v
    }

    /// Show or hide the taskbar on every display. Plain window visibility:
    /// nothing in Explorer is patched, and showing it again undoes it.
    pub fn taskbar_visible(show: bool) {
        for h in trays() {
            unsafe { ShowWindow(h, if show { SW_SHOWNOACTIVATE } else { SW_HIDE }) };
        }
    }

    pub fn taskbar_is_visible() -> bool {
        trays().into_iter().any(|h| unsafe { IsWindowVisible(h) } != 0)
    }

    pub fn wait_for_exit(pid: u32) {
        use windows_sys::Win32::System::Threading::{WaitForSingleObject, INFINITE, PROCESS_SYNCHRONIZE};
        unsafe {
            let h = OpenProcess(PROCESS_SYNCHRONIZE, 0, pid);
            if !h.is_null() {
                WaitForSingleObject(h, INFINITE);
                CloseHandle(h);
            }
        }
    }

    /// Read (None) or set the taskbar's own "Automatically hide" switch.
    /// Returns the state before the call.
    pub fn taskbar_autohide(set: Option<bool>) -> bool {
        unsafe {
            // Explorer's taskbar, not Cove's stand-in tray (tray.rs).
            let tray = trays().into_iter().find(|&h| class(h) == "Shell_TrayWnd").unwrap_or(std::ptr::null_mut());
            let mut d: APPBARDATA = std::mem::zeroed();
            d.cbSize = std::mem::size_of::<APPBARDATA>() as u32;
            d.hWnd = tray;
            let prev = SHAppBarMessage(ABM_GETSTATE, &mut d) as isize & ABS_AUTOHIDE != 0;
            if let Some(on) = set {
                d.lParam = if on { ABS_AUTOHIDE } else { ABS_ALWAYSONTOP };
                SHAppBarMessage(ABM_SETSTATE, &mut d);
            }
            prev
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Win;
    pub fn attention_watch() {}
    pub fn taskbar_visible(_show: bool) {}
    pub fn taskbar_is_visible() -> bool {
        false
    }
    pub fn wait_for_exit(_pid: u32) {}
    pub fn cursor() -> Option<(i32, i32)> {
        None
    }
    pub fn monitor_at(_x: i32, _y: i32) -> Option<super::Mon> {
        None
    }
    pub fn windows() -> Vec<Win> {
        Vec::new()
    }
    pub fn foreground() -> isize {
        0
    }
    pub fn activate(_h: isize) {}
    pub fn minimize(_h: isize) {}
    pub fn zoom(_h: isize) {}
    pub fn bin_full() -> Option<bool> {
        None
    }
    pub fn bin_empty() -> Result<(), String> {
        Err("Not on Windows.".into())
    }
    pub fn tile(_h: isize, _left: bool) {}
    pub fn overlaps(_rc: (i32, i32, i32, i32)) -> bool {
        false
    }
    pub fn close_windows(_h: &[isize]) {}
    pub fn work_area() -> Option<(i32, i32, i32, i32)> {
        None
    }
    pub fn taskbar_autohide(_set: Option<bool>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::matches;
    #[test]
    fn attention_follows_shell_events() {
        use super::*;
        let mut a = Vec::new();
        shell_event(&mut a, HSHELL_FLASH, 7);
        shell_event(&mut a, HSHELL_FLASH, 7);
        shell_event(&mut a, HSHELL_FLASH, 9);
        assert_eq!(a, vec![7, 9]);
        shell_event(&mut a, HSHELL_WINDOWACTIVATED, 7);
        assert_eq!(a, vec![9]);
        shell_event(&mut a, HSHELL_RUDEAPPACTIVATED, 9);
        assert!(a.is_empty());
        let mut b = vec![1, 2, 3];
        prune_attention(&mut b, &[1, 2], 2);
        assert_eq!(b, vec![1]);
    }

    #[test]
    fn cove_sits_after_running_apps() {
        use super::*;
        let it = |kind: &'static str| DockItem { kind, name: kind.into(), target: kind.into(), windows: vec![], active: false, attention: false, icon: None, no_icon: false, full: None };
        let mut v = vec![it("pin"), it("running"), it("stack"), it("trash")];
        insert_cove(&mut v, it("cove"));
        assert_eq!(v.iter().map(|i| i.kind).collect::<Vec<_>>(), ["pin", "running", "cove", "stack", "trash"]);
    }

    #[test]
    fn edge_watch_needs_a_rest() {
        let m = super::Mon { monitor: (0, 0, 1920, 1080), work: (0, 0, 1920, 1040) };
        let mut e = super::EdgeWatch::new();
        assert!(e.feed(1079, Some(m), 2).is_none());
        assert_eq!(e.feed(1079, Some(m), 2), Some(m));
        assert!(e.feed(500, Some(m), 2).is_none());
    }

    #[test]
    fn explorer_windows_join_the_explorer_pin() {
        use super::*;
        use crate::settings::Pin;
        let win = |exe: &str, aumid: Option<&str>| Win { hwnd: 1, exe: exe.into(), title: "Downloads".into(), aumid: aumid.map(Into::into), icon_res: None };
        let ex = win(r"C:\Windows\explorer.exe", Some("Microsoft.Windows.Explorer"));
        let ex_noid = win(r"C:\Windows\explorer.exe", None);
        let pin = |target: &str, exe: Option<&str>, aumid: Option<&str>| Pin { name: "File Explorer".into(), target: target.into(), exe: exe.map(Into::into), aumid: aumid.map(Into::into) };
        let lnk = pin(r"C:\Users\k\AppData\Roaming\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar\File Explorer.lnk", None, None);
        let exe = pin(r"C:\Windows\explorer.exe", Some(r"C:\Windows\explorer.exe"), None);
        let id = pin("Microsoft.Windows.Explorer", None, None);
        let lnk_id = pin(r"C:\x\File Explorer.lnk", Some(r"C:\Windows\explorer.exe"), Some("Microsoft.Windows.Explorer"));
        for p in [&lnk, &exe, &id, &lnk_id] {
            assert!(matches_win(p, &ex), "{}", p.target);
            assert!(matches_win(p, &ex_noid), "{}", p.target);
        }
        assert_eq!(ex.app_key(), ex_noid.app_key());
        // Chrome profiles still stay apart, and Chrome never joins Explorer.
        let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        let work = pin(r"C:\x\Work - Chrome.lnk", Some(chrome), Some("Chrome.AB12"));
        let me = pin(r"C:\x\Google Chrome.lnk", Some(chrome), Some("Chrome"));
        let w_work = win(chrome, Some("Chrome.AB12"));
        let w_me = win(chrome, Some("Chrome"));
        assert!(matches_win(&work, &w_work) && !matches_win(&work, &w_me));
        assert!(matches_win(&me, &w_me) && !matches_win(&me, &w_work));
        assert!(!matches_win(&lnk, &w_me) && !matches_win(&me, &ex));
    }

    #[test]
    fn matches_pins_to_running_exes() {
        assert!(matches("Chrome", "Google Chrome", r"C:\Program Files\Google\Chrome\Application\chrome.exe"));
        assert!(matches("MSEdge", "Microsoft Edge", r"C:\x\msedge.exe"));
        assert!(matches("Microsoft.Windows.Explorer", "File Explorer", r"C:\Windows\explorer.exe"));
        assert!(matches("Microsoft.VisualStudioCode", "Visual Studio Code", r"C:\x\Code.exe"));
        assert!(!matches("MSEdge", "Microsoft Edge", r"C:\x\notepad.exe"));
    }
}
