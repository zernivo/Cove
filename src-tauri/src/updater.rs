//! "Update Cove" button: checks this repo's latest GitHub release,
//! downloads its installer with progress, and installs it silently in
//! update mode (no wizard, no uninstall step), then reopens Cove.
//!
//! Safety: only release assets of github.com/zernivo/Cove over HTTPS,
//! size checked, SHA-256 checked against the digest GitHub publishes, and
//! (since v0.8.1) a minisign signature (Tauri updater format) that must
//! match the public key built into Cove. An installer without a valid
//! signature is deleted and never run, even if GitHub itself were tampered
//! with, because the private key never touches GitHub.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::PathBuf;

const REPO: &str = "zernivo/Cove";
/// Cove's update-signing public key (base64 of a minisign public key file).
const UPDATE_PUBKEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDM4NzY1RDgzRTlBMDk2NkEKUldScWxxRHBnMTEyT0lkYTdWZGFHOHEvUmpWMitNNzBQenFuY3NkblZ5K09kM2F2RTBBSjRlSHgK";
/// Cove is relaunched with this after a silent update.
pub const UPDATED_FLAG: &str = "--updated";

#[derive(Debug, Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    body: Option<String>,
    assets: Vec<Asset>,
}

#[derive(Debug, Deserialize, Clone)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub page: String,
}

pub fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let core = v.split(['-', '+']).next()?;
    let mut it = core.split('.').map(|p| p.parse::<u64>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

fn latest_release() -> Result<Release, String> {
    ureq::get(&format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .set("User-Agent", concat!("Cove/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .timeout(std::time::Duration::from_secs(15))
        .call()
        .map_err(|e| format!("Couldn't reach GitHub: {e}"))?
        .into_json::<Release>()
        .map_err(|e| format!("Unexpected reply from GitHub: {e}"))
}

pub fn check() -> Result<UpdateInfo, String> {
    let r = latest_release()?;
    let current = env!("CARGO_PKG_VERSION").to_string();
    let available = match (parse_version(&r.tag_name), parse_version(&current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    };
    Ok(UpdateInfo {
        current,
        latest: r.tag_name.trim_start_matches('v').to_string(),
        available,
        notes: r.body.unwrap_or_default(),
        page: r.html_url,
    })
}

/// Download the latest setup.exe, verify it, and return its path.
pub fn download(mut progress: impl FnMut(u64, u64)) -> Result<PathBuf, String> {
    let r = latest_release()?;
    let asset = r
        .assets
        .iter()
        .find(|a| a.name.ends_with("-setup.exe"))
        .cloned()
        .ok_or("This release has no installer yet.")?;
    let prefix = format!("https://github.com/{REPO}/releases/download/");
    if !asset.browser_download_url.starts_with(&prefix) {
        return Err("Refusing an installer from outside Cove's releases.".into());
    }

    let resp = ureq::get(&asset.browser_download_url)
        .set("User-Agent", "Cove-updater")
        .timeout(std::time::Duration::from_secs(300))
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;
    let path = std::env::temp_dir().join(&asset.name);
    let mut file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
    let mut reader = resp.into_reader().take(asset.size + 1);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| format!("Download failed: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
        file.write_all(&buf[..n]).map_err(|e| e.to_string())?;
        total += n as u64;
        progress(total, asset.size);
    }
    drop(file);

    let fail = |msg: &str| {
        let _ = std::fs::remove_file(&path);
        Err(msg.to_string())
    };
    if total != asset.size {
        return fail("Download was incomplete. Try again.");
    }
    if let Some(d) = asset.digest.as_deref().and_then(|d| d.strip_prefix("sha256:")) {
        let got: String = hasher.finalize().iter().map(|b| format!("{b:02x}")).collect();
        if !got.eq_ignore_ascii_case(d) {
            return fail("Installer failed its integrity check, so it was deleted.");
        }
    }
    // Signatures are published in the repo (signatures/), signed offline;
    // the private key is never on GitHub.
    let sig_url = format!("https://raw.githubusercontent.com/{REPO}/main/signatures/{}.sig", asset.name);
    let sig_text = match ureq::get(&sig_url)
        .set("User-Agent", "Cove-updater")
        .timeout(std::time::Duration::from_secs(30))
        .call()
        .ok()
        .and_then(|r| r.into_string().ok())
    {
        Some(t) if t.len() < 4096 => t,
        _ => return fail("This release isn't signed yet, so Cove won't install it. Try again in a few minutes."),
    };
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => return fail(&e.to_string()),
    };
    if let Err(e) = verify_signature(&bytes, &sig_text) {
        return fail(&format!("Installer failed its signature check, so it was deleted ({e})."));
    }
    Ok(path)
}

fn b64_text(s: &str) -> Result<String, String> {
    use base64::Engine;
    let raw = base64::engine::general_purpose::STANDARD
        .decode(s.trim())
        .map_err(|e| format!("bad base64: {e}"))?;
    String::from_utf8(raw).map_err(|e| format!("bad text: {e}"))
}

/// Check a Tauri-format signature (base64 of a minisign .sig file).
pub fn verify_signature(data: &[u8], sig_b64: &str) -> Result<(), String> {
    let pk = minisign_verify::PublicKey::decode(&b64_text(UPDATE_PUBKEY)?).map_err(|e| e.to_string())?;
    let sig = minisign_verify::Signature::decode(&b64_text(sig_b64)?).map_err(|e| e.to_string())?;
    pk.verify(data, &sig, true).map_err(|e| e.to_string())
}

/// Start the installer. The caller quits Cove right after so files aren't locked.
pub fn launch(path: &PathBuf) -> Result<(), String> {
    #[cfg(windows)]
    {
        // Tauri NSIS flags: /S silent, /UPDATE keep the install (no uninstall
        // step, no new shortcuts), /R relaunch Cove after, /ARGS passed to it.
        std::process::Command::new(path)
            .args(["/S", "/UPDATE", "/R", "/ARGS", UPDATED_FLAG])
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        Err("Updating works on Windows only.".into())
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_version, verify_signature};

    #[test]
    fn accepts_a_real_signature_and_rejects_tampering() {
        let sig = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVScWxxRHBnMTEyT0pzYkRhZVpiMkJNNCsyeFRGUzRmYmJwYUxVMGt4M0oxZVNyMG5WZlpRTCtLckRhODdrTVg5NCtpY2FUWGZjU3Y1Sm9sajQzYzdYRDBQUkljalY5UVFzPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkzNDQzNTE3CWZpbGU6Zml4dHVyZS5iaW4KejVhRlpWRXMzVk9UNEMvUmFnNXM3WUE0Mm12UllTNEJrTmVkaU05dldnOVFVUVptUnc2U2VGTDE5ckNkdytYbWFhYkl5VnJ2amFjKzhNME4vV01CQ1E9PQo=";
        assert!(verify_signature(b"cove signed update test", sig).is_ok());
        assert!(verify_signature(b"cove signed update test!", sig).is_err());
    }

    #[test]
    fn rejects_garbage_signatures() {
        assert!(verify_signature(b"hello", "bm90IGEgc2lnbmF0dXJl").is_err());
        assert!(verify_signature(b"hello", "!!!").is_err());
    }

    #[test]
    fn compares_versions() {
        assert!(parse_version("v0.1.10") > parse_version("0.1.9"));
        assert!(parse_version("v1.0.0") > parse_version("0.9.9"));
        assert_eq!(parse_version("v0.2.0-beta"), Some((0, 2, 0)));
        assert_eq!(parse_version("nope"), None);
    }
}
