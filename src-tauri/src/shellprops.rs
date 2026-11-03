//! Read-only Shell lookups through documented APIs: a window's
//! AppUserModelID, where a shortcut points, and the apps pinned to the
//! Windows taskbar (so the dock starts with exactly those).

use crate::settings::Pin;

/// Taskbar pins, in taskbar order, as dock pins (shortcut path + the exe it
/// opens). Empty when there are none or Windows hides them from us.
pub fn taskbar_pins() -> Vec<Pin> {
    let Some(appdata) = std::env::var_os("APPDATA") else { return Vec::new() };
    let dir = std::path::Path::new(&appdata).join(r"Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut files: Vec<std::path::PathBuf> = rd
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("lnk")).unwrap_or(false))
        .collect();
    files.sort();
    let order = imp::taskband_order();
    files.sort_by_key(|p| {
        let n = p.file_name().map(|n| n.to_string_lossy().to_lowercase()).unwrap_or_default();
        order.iter().position(|o| *o == n).unwrap_or(usize::MAX)
    });
    files
        .into_iter()
        .map(|p| {
            let name = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            let target = p.to_string_lossy().to_string();
            let exe = link_target(&target).filter(|t| t.to_lowercase().ends_with(".exe"));
            Pin { name, target, exe }
        })
        .collect()
}

/// Shortcut names in the order the taskbar shows them, read from the blob
/// Explorer keeps (HKCU ...\Taskband\Favorites): the names appear in it as
/// UTF-16 strings ending in ".lnk".
pub fn order_from_blob(blob: &[u8]) -> Vec<String> {
    let units: Vec<u16> = blob.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    let text: String = units.iter().map(|&u| char::from_u32(u as u32).filter(|c| !c.is_control()).unwrap_or('\0')).collect();
    let mut out = Vec::new();
    for part in text.split('\0') {
        let low = part.to_lowercase();
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

pub use imp::{link_target, window_aumid};

#[cfg(windows)]
mod imp {
    use windows::core::{GUID, HSTRING};
    use windows::Win32::Foundation::{HWND, PROPERTYKEY};
    use windows::Win32::System::Com::{CoInitializeEx, CoTaskMemFree, CoUninitialize, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow};
    use windows::Win32::UI::Shell::{IShellItem2, SHCreateItemFromParsingName};

    const PKEY_APPUSERMODEL_ID: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0x9F4C2855_9F79_4B39_A8D0_E1D42DE1D5F3), pid: 5 };
    const PKEY_LINK_TARGETPARSINGPATH: PROPERTYKEY =
        PROPERTYKEY { fmtid: GUID::from_u128(0xB9B4B3FC_2B51_4A42_B5D8_324146AFCF25), pid: 2 };

    pub fn window_aumid(hwnd: isize) -> Option<String> {
        unsafe {
            let store: IPropertyStore = SHGetPropertyStoreForWindow(HWND(hwnd as *mut _)).ok()?;
            let v = store.GetValue(&PKEY_APPUSERMODEL_ID).ok()?;
            let s = v.to_string();
            (!s.trim().is_empty()).then(|| s)
        }
    }

    pub fn link_target(lnk: &str) -> Option<String> {
        unsafe {
            let ok = CoInitializeEx(None, COINIT_APARTMENTTHREADED).is_ok();
            let out = (|| {
                let item: IShellItem2 = SHCreateItemFromParsingName(&HSTRING::from(lnk), None).ok()?;
                let p = item.GetString(&PKEY_LINK_TARGETPARSINGPATH).ok()?;
                let s = p.to_string().ok();
                CoTaskMemFree(Some(p.0 as *const _));
                s
            })();
            if ok {
                CoUninitialize();
            }
            out.filter(|s| !s.is_empty())
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
        assert_eq!(super::order_from_blob(&blob), vec!["google chrome.lnk", "visual studio code.lnk"]);
    }
}
