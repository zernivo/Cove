//! Read-only Shell lookups through documented APIs: a window's
//! AppUserModelID, where a shortcut points, and the apps pinned to the
//! Windows taskbar (so the dock starts with exactly those).

use crate::settings::Pin;

/// Taskbar pins, in taskbar order, as dock pins (shortcut path + the exe it
/// opens). Empty when there are none or Windows hides them from us.
pub fn taskbar_pins() -> Vec<Pin> {
    let Some(appdata) = std::env::var_os("APPDATA") else { return Vec::new() };
    let dir = std::path::Path::new(&appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false))
                .collect()
        })
        .unwrap_or_default();
    files.sort();
    let lnk_pin = |p: &std::path::Path| {
        let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let target = p.to_string_lossy().to_string();
        let exe = link_target(&target).filter(|t| t.to_lowercase().ends_with(".exe"));
        let aumid = link_aumid(&target);
        Pin { name, target, exe, aumid }
    };
    let mut out: Vec<Pin> = Vec::new();
    // Taskbar order first: shortcuts by name, Store apps by their AppID.
    for entry in imp::taskband_order() {
        if entry.ends_with(".lnk") {
            if let Some(i) = files.iter().position(|f| f.file_name().map(|n| n.to_string_lossy().to_lowercase()) == Some(entry.clone())) {
                out.push(lnk_pin(&files.remove(i)));
            }
        } else if let Some(name) = app_name(&entry) {
            if !out.iter().any(|p| p.target.eq_ignore_ascii_case(&entry)) {
                out.push(Pin { name, target: entry, exe: None, aumid: None });
            }
        }
    }
    out.extend(files.iter().map(|f| lnk_pin(f)));
    out
}

/// A Store app's AppID ("Family_hash!App") as it appears in the taskbar blob.
fn looks_like_appid(s: &str) -> bool {
    let Some((fam, app)) = s.split_once('!') else { return false };
    fam.contains('_') && !app.is_empty() && s.len() < 200 && !s.contains([' ', '\\', '/', ':'])
}

/// Shortcut names in the order the taskbar shows them, read from the blob
/// Explorer keeps (HKCU ...\Taskband\Favorites): the names appear in it as
/// UTF-16 strings ending in ".lnk".
pub fn order_from_blob(blob: &[u8]) -> Vec<String> {
    // Entries are shortcut names ("chrome.lnk") or Store AppIDs, in order.
    let units: Vec<u16> = blob.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let text: String = units.iter().map(|&u| char::from_u32(u as u32).filter(|c| !c.is_control()).unwrap_or('\0')).collect();
    let mut out = Vec::new();
    for part in text.split('\0') {
        let low = part.to_lowercase();
        let tok = part.trim();
        if looks_like_appid(tok) {
            if !out.iter().any(|o: &String| o.eq_ignore_ascii_case(tok)) {
                out.push(tok.to_string());
            }
            continue;
        }
        if let Some(end) = low.find(".lnk") {
            let name = &low[..end + 4];
            let name = name.rsplit(['\\', '/']).next().unwrap_or(name).trim().to_string();
            if name.len() > 4 && !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out
}

pub use imp::{app_name, link_aumid, link_target, window_aumid, window_icon_resource};

#[cfg(windows)]
mod imp {
    use windows::core::{GUID, HSTRING};
    use windows::Win32::Foundation::{HWND, PROPERTYKEY};
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
    use windows::Win32::UI::Shell::{IShellItem2, SHCreateItemFromParsingName};

    const PKEY_APPUSERMODEL_ID: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3), pid: 5 };
    const PKEY_APPUSERMODEL_RELAUNCHICON: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3), pid: 3 };
    const PKEY_LINK_TARGETPARSINGPATH: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0xB9B4B3FC_2B51_4A42_B5D8_324146AFCF25), pid: 2 };

    fn window_prop(hwnd: isize, key: &PROPERTYKEY) -> Option<String> {
        unsafe {
            let store: IPropertyStore = SHGetPropertyStoreForWindow(HWND(hwnd as *mut _)).ok()?;
            let v = store.GetValue(key).ok()?;
            let s = v.to_string();
            (!s.trim().is_empty()).then(|| s)
        }
    }

    pub fn window_aumid(hwnd: isize) -> Option<String> {
        window_prop(hwnd, &PKEY_APPUSERMODEL_ID)
    }

    /// The icon the taskbar shows for this window when the app sets one
    /// (Chrome: each profile's icon with its picture), as "file,index".
    pub fn window_icon_resource(hwnd: isize) -> Option<String> {
        window_prop(hwnd, &PKEY_APPUSERMODEL_RELAUNCHICON)
    }

    fn item_string(path: &str, key: &PROPERTYKEY) -> Option<String> {
        unsafe {
            let ok = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let out = (|| {
                let item: IShellItem2 = SHCreateItemFromParsingName(&HSTRING::from(path), None).ok()?;
                let p = item.GetString(key).ok()?;
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const _));
                s
            })();
            if ok {
                CoUninitialize();
            }
            out.filter(|s| !s.trim().is_empty())
        }
    }

    pub fn link_target(lnk: &str) -> Option<String> {
        item_string(lnk, &PKEY_LINK_TARGETPARSINGPATH)
    }

    pub fn link_aumid(lnk: &str) -> Option<String> {
        item_string(lnk, &PKEY_APPUSERMODEL_ID)
    }

    /// A Start app's display name, from its AppID.
    pub fn app_name(appid: &str) -> Option<String> {
        use windows::Win32::UI::Shell::{IShellItem, SIGDN_NORMALDISPLAY};
        unsafe {
            let ok = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let out = (|| {
                let item: IShellItem = SHCreateItemFromParsingName(&HSTRING::from(format!("shell:AppsFolder\\{appid}")), None).ok()?;
                let p = item.GetDisplayName(SIGDN_NORMALDISPLAY).ok()?;
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const _));
                s
            })();
            if ok {
                CoUninitialize();
            }
            out.filter(|s| !s.trim().is_empty())
        }
    }

    pub fn taskband_order() -> Vec<String> {
        let hk = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
        hk.open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Explorer\Taskband")
            .and_then(|k| k.get_raw_value("Favorites"))
            .map(|v| super::order_from_blob(&v.bytes))
            .unwrap_or_default()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn window_aumid(_hwnd: isize) -> Option<String> {
        None
    }
    pub fn link_target(_lnk: &str) -> Option<String> {
        None
    }
    pub fn link_aumid(_lnk: &str) -> Option<String> {
        None
    }
    pub fn app_name(_appid: &str) -> Option<String> {
        None
    }
    pub fn window_icon_resource(_hwnd: isize) -> Option<String> {
        None
    }
    pub fn taskband_order() -> Vec<String> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn reads_taskband_order() {
        let mut blob = vec![0u8; 6];
        for n in ["Google Chrome.lnk", "Visual Studio Code.lnk"] {
            blob.extend(n.encode_utf16().flat_map(|u| u.to_le_bytes()));
            blob.extend([0, 0, 7, 0]);
        }
        blob.extend("OpenAI.ChatGPT-Desktop_2p2nqsd0c76g0!ChatGPT".encode_utf16().flat_map(|u| u.to_le_bytes()));
        blob.extend([0, 0]);
        assert_eq!(
            super::order_from_blob(&blob),
            vec!["google chrome.lnk", "visual studio code.lnk", "OpenAI.ChatGPT-Desktop_2p2nqsd0c76g0!ChatGPT"]
        );
    }
}
