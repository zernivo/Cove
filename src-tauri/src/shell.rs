//! Tell Explorer and apps that settings changed, so the taskbar
//! refreshes without a sign-out. Documented WM_SETTINGCHANGE broadcast.

#[cfg(windows)]
pub fn notify_settings_changed() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };
    for area in ["TraySettings", "Policy"] {
        let wide: Vec<u16> = area.encode_utf16().chain(std::iter::once(0)).collect();
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0,
                wide.as_ptr() as isize,
                SMTO_ABORTIFHUNG,
                1000,
                &mut result,
            );
        }
    }
}

#[cfg(not(windows))]
pub fn notify_settings_changed() {}

/// Open a Windows Settings page (used when Cove can't change something itself).
pub fn open_settings(uri: &str) -> std::io::Result<()> {
    if !uri.starts_with("ms-settings:") {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "not a settings uri"));
    }
    #[cfg(windows)]
    {
        std::process::Command::new("explorer.exe").arg(uri).spawn()?;
    }
    Ok(())
}
