//! Cove Control Centre: sound, display brightness, Wi-Fi status and light /
//! dark appearance, all through documented Windows interfaces:
//! Core Audio (IAudioEndpointVolume), the WMI monitor-brightness class,
//! `netsh wlan`, the volume-mute media key, and the Personalize theme
//! values Windows Settings itself writes.

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct Controls {
    /// 0-100, None when there's no output device.
    pub volume: Option<u8>,
    pub muted: bool,
    /// 0-100, None when the screen can't be dimmed by software (most desktops).
    pub brightness: Option<u8>,
    /// Connected Wi-Fi name; None when not on Wi-Fi.
    pub wifi: Option<String>,
    pub dark: bool,
    /// Bluetooth radio on / off; None when there's no Bluetooth radio.
    pub bluetooth: Option<bool>,
    /// Do not disturb (Focus) is on.
    pub focus: Option<bool>,
    /// The sound output's friendly name ("Speakers (Realtek(R) Audio)").
    pub device: Option<String>,
    /// What's playing (Windows' media session), if anything.
    pub media: Option<Media>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Media {
    pub title: String,
    pub artist: String,
    /// The player's friendly name ("Spotify").
    pub app: String,
    pub playing: bool,
    /// The player's real icon (coveicon URL), once rendered.
    pub icon: Option<String>,
}

pub fn get() -> Controls {
    let (volume, muted) = imp::volume();
    Controls {
        volume,
        muted,
        brightness: imp::brightness(),
        wifi: imp::wifi(),
        dark: imp::dark(),
        bluetooth: imp::bluetooth(),
        focus: imp::focus(),
        device: imp::device().map(|d| tidy_device(&d)),
        media: imp::media(),
    }
}

/// "Speakers (Realtek(R) Audio)" -> "Speakers (Realtek)".
pub fn tidy_device(name: &str) -> String {
    name.replace("(R)", "").replace(" Audio)", ")").replace("  ", " ").trim().to_string()
}

/// A media app id ("Spotify.exe", "Chrome", "Family!App") as a friendly name.
pub fn player_name(app_id: &str) -> String {
    let base = app_id.rsplit(['\\', '!']).next().unwrap_or(app_id);
    let base = base.strip_suffix(".exe").or_else(|| base.strip_suffix(".EXE")).unwrap_or(base);
    let mut c = base.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn media_control(what: &str) -> Result<(), String> {
    imp::media_control(what)
}

/// Quick state for the bar icons (no PowerShell / netsh): volume and mute.
pub fn sound() -> (Option<u8>, bool) {
    imp::volume()
}

/// The connected Wi-Fi network's name (starts netsh: call off the UI thread).
pub fn get_wifi() -> Option<String> {
    imp::wifi()
}

pub use imp::{set_brightness, set_dark, set_volume, toggle_mute};

/// Pull the SSID out of `netsh wlan show interfaces` output.
pub fn parse_ssid(text: &str) -> Option<String> {
    let mut connected = false;
    let mut ssid = None;
    for line in text.lines() {
        let Some((k, v)) = line.split_once(':') else { continue };
        let (k, v) = (k.trim(), v.trim());
        if k.eq_ignore_ascii_case("State") {
            connected = v.eq_ignore_ascii_case("connected");
        } else if k.eq_ignore_ascii_case("SSID") && !v.is_empty() {
            ssid = Some(v.to_string());
        }
    }
    if connected { ssid } else { None }
}

#[cfg(windows)]
mod imp {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn ps(cmd: &str) -> Option<String> {
        let out = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn endpoint() -> Option<windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume> {
        use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL};
        unsafe {
            let en: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
            let dev = en.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
            dev.Activate(CLSCTX_ALL, None).ok()
        }
    }

    pub fn volume() -> (Option<u8>, bool) {
        let mut res = (None, false);
        crate::icons::with_com(|| {
            if let Some(v) = endpoint() {
                let lvl = unsafe { v.GetMasterVolumeLevelScalar() }.ok();
                let muted: bool = unsafe { v.GetMute() }.map(|b| b.into()).unwrap_or(false);
                res = (lvl.map(|l| (l * 100.0).round().clamp(0.0, 100.0) as u8), muted);
            }
        });
        res
    }

    pub fn set_volume(pct: u8) -> Result<(), String> {
        let mut r = Err("No sound output device.".to_string());
        crate::icons::with_com(|| {
            if let Some(v) = endpoint() {
                let lvl = pct.min(100) as f32 / 100.0;
                r = unsafe { v.SetMasterVolumeLevelScalar(lvl, std::ptr::null()) }.map_err(|e| e.to_string());
            }
        });
        r
    }

    /// Press the keyboard's mute key: Windows shows its own volume overlay.
    pub fn toggle_mute() {
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
            SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_VOLUME_MUTE,
        };
        let key = |up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VK_VOLUME_MUTE, wScan: 0, dwFlags: if up { KEYEVENTF_KEYUP } else { 0 }, time: 0, dwExtraInfo: 0 },
            },
        };
        let inputs = [key(false), key(true)];
        unsafe { SendInput(2, inputs.as_ptr(), std::mem::size_of::<INPUT>() as i32) };
    }

    pub fn brightness() -> Option<u8> {
        let out = ps("(Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightness -ErrorAction SilentlyContinue | Select-Object -First 1).CurrentBrightness")?;
        out.lines().next()?.trim().parse::<u8>().ok()
    }

    pub fn set_brightness(pct: u8) -> Result<(), String> {
        let cmd = format!(
            "Get-CimInstance -Namespace root/WMI -ClassName WmiMonitorBrightnessMethods | Invoke-CimMethod -MethodName WmiSetBrightness -Arguments @{{Timeout=1;Brightness=[byte]{}}} | Out-Null",
            pct.min(100)
        );
        ps(&cmd).map(|_| ()).ok_or_else(|| "Couldn't change brightness.".into())
    }

    pub fn wifi() -> Option<String> {
        let out = std::process::Command::new("netsh.exe")
            .args(["wlan", "show", "interfaces"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        super::parse_ssid(&String::from_utf8_lossy(&out.stdout))
    }

    const PERSONALIZE: &str = r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize";

    pub fn dark() -> bool {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(PERSONALIZE)
            .and_then(|k| k.get_value::<u32, _>("AppsUseLightTheme"))
            .map(|v| v == 0)
            .unwrap_or(false)
    }

    /// The same two values the Settings app's "Choose your mode" writes.
    pub fn set_dark(on: bool) -> Result<(), String> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let (k, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(PERSONALIZE).map_err(|e| e.to_string())?;
        let light: u32 = if on { 0 } else { 1 };
        k.set_value("AppsUseLightTheme", &light).map_err(|e| e.to_string())?;
        k.set_value("SystemUsesLightTheme", &light).map_err(|e| e.to_string())?;
        broadcast_theme();
        Ok(())
    }

    fn broadcast_theme() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SendMessageTimeoutW, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
        };
        let wide: Vec<u16> = "ImmersiveColorSet".encode_utf16().chain(std::iter::once(0)).collect();
        let mut result: usize = 0;
        unsafe {
            SendMessageTimeoutW(HWND_BROADCAST, WM_SETTINGCHANGE, 0, wide.as_ptr() as isize, SMTO_ABORTIFHUNG, 1000, &mut result);
        }
    }

    pub fn bluetooth() -> Option<bool> {
        use windows::Devices::Radios::{Radio, RadioKind, RadioState};
        let radios = Radio::GetRadiosAsync().ok()?.get().ok()?;
        for r in radios {
            if r.Kind().ok()? == RadioKind::Bluetooth {
                return Some(r.State().ok()? == RadioState::On);
            }
        }
        None
    }

    /// Windows 11's Do not disturb: the global toast switch Settings writes.
    pub fn focus() -> Option<bool> {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        let v = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Microsoft\Windows\CurrentVersion\Notifications\Settings")
            .and_then(|k| k.get_value::<u32, _>("NOC_GLOBAL_SETTING_TOASTS_ENABLED"));
        Some(matches!(v, Ok(0)))
    }

    pub fn device() -> Option<String> {
        use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
        use windows::Win32::Media::Audio::{eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator};
        use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};
        let mut out = None;
        crate::icons::with_com(|| unsafe {
            let Ok(en) = CoCreateInstance::<_, IMMDeviceEnumerator>(&MMDeviceEnumerator, None, CLSCTX_ALL) else { return };
            let Ok(dev) = en.GetDefaultAudioEndpoint(eRender, eConsole) else { return };
            let Ok(store) = dev.OpenPropertyStore(STGM_READ) else { return };
            if let Ok(v) = store.GetValue(&PKEY_Device_FriendlyName) {
                let s = v.to_string();
                if !s.is_empty() {
                    out = Some(s);
                }
            }
        });
        out
    }

    fn session() -> Option<windows::Media::Control::GlobalSystemMediaTransportControlsSession> {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as Manager;
        Manager::RequestAsync().ok()?.get().ok()?.GetCurrentSession().ok()
    }

    pub fn media() -> Option<super::Media> {
        use windows::Media::Control::GlobalSystemMediaTransportControlsSessionPlaybackStatus as Status;
        let s = session()?;
        let props = s.TryGetMediaPropertiesAsync().ok()?.get().ok()?;
        let title = props.Title().map(|t| t.to_string()).unwrap_or_default();
        if title.is_empty() {
            return None;
        }
        let app_id = s.SourceAppUserModelId().map(|t| t.to_string()).unwrap_or_default();
        let playing = s.GetPlaybackInfo().ok().and_then(|i| i.PlaybackStatus().ok()) == Some(Status::Playing);
        // The player's real icon: its package logo, its Start entry, its exe.
        let mut keys: Vec<String> = crate::dock::package_logo_key(&app_id).into_iter().collect();
        if !app_id.is_empty() {
            keys.push(format!(r"shell:AppsFolder\{app_id}"));
        }
        let mut jobs = Vec::new();
        let icon = crate::dock::icon_from(&keys, &mut jobs);
        if !jobs.is_empty() {
            std::thread::spawn(move || crate::icons::fill(jobs));
        }
        let app = crate::shellprops::app_name(&app_id).unwrap_or_else(|| super::player_name(&app_id));
        Some(super::Media { title, artist: props.Artist().map(|t| t.to_string()).unwrap_or_default(), app, playing, icon })
    }

    pub fn media_control(what: &str) -> Result<(), String> {
        let s = session().ok_or("Nothing is playing.")?;
        let op = match what {
            "toggle" => s.TryTogglePlayPauseAsync(),
            "next" => s.TrySkipNextAsync(),
            "prev" => s.TrySkipPreviousAsync(),
            _ => return Err("Unknown control.".into()),
        };
        op.and_then(|o| o.get()).map(|_| ()).map_err(|e| e.to_string())
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn volume() -> (Option<u8>, bool) {
        (None, false)
    }
    pub fn set_volume(_p: u8) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn toggle_mute() {}
    pub fn brightness() -> Option<u8> {
        None
    }
    pub fn set_brightness(_p: u8) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn wifi() -> Option<String> {
        None
    }
    pub fn dark() -> bool {
        false
    }
    pub fn set_dark(_on: bool) -> Result<(), String> {
        Err("Windows only".into())
    }
    pub fn bluetooth() -> Option<bool> {
        None
    }
    pub fn focus() -> Option<bool> {
        None
    }
    pub fn device() -> Option<String> {
        None
    }
    pub fn media() -> Option<super::Media> {
        None
    }
    pub fn media_control(_what: &str) -> Result<(), String> {
        Err("Windows only".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn tidies_names() {
        assert_eq!(super::tidy_device("Speakers (Realtek(R) Audio)"), "Speakers (Realtek)");
        assert_eq!(super::player_name("Spotify.exe"), "Spotify");
        assert_eq!(super::player_name("chrome"), "Chrome");
    }
    #[test]
    fn ssid_parsing() {
        let t = "    Name                   : Wi-Fi\n    State                  : connected\n    SSID                   : HomeNet 5G\n    BSSID                  : aa:bb\n";
        assert_eq!(super::parse_ssid(t).as_deref(), Some("HomeNet 5G"));
        let off = "    State                  : disconnected\n";
        assert_eq!(super::parse_ssid(off), None);
    }
}
