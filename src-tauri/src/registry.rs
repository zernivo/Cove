//! Per-user registry access (HKCU only, never HKLM).
//!
//! Targets are written as `HKCU\Path\To\Key\ValueName`.
//! Writing `Value::Missing` deletes the value, which is how restore
//! removes settings that did not exist before Cove touched them.

use crate::journal::Value;
use std::io;

fn split(target: &str) -> io::Result<(&str, &str)> {
    let rest = target
        .strip_prefix("HKCU\\")
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Cove only writes HKCU"))?;
    rest.rsplit_once('\\')
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "target needs a value name"))
}

#[cfg(windows)]
pub fn read(target: &str) -> io::Result<Value> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ};
    use winreg::RegKey;

    let (path, name) = split(target)?;
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(path, KEY_READ) {
        Ok(k) => k,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Value::Missing),
        Err(e) => return Err(e),
    };
    if let Ok(v) = key.get_value::<u32, _>(name) {
        return Ok(Value::Dword(v));
    }
    match key.get_value::<String, _>(name) {
        Ok(s) => Ok(Value::Text(s)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(Value::Missing),
        Err(e) => Err(e),
    }
}

#[cfg(windows)]
pub fn write(target: &str, value: &Value) -> io::Result<()> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let (path, name) = split(target)?;
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    match value {
        Value::Missing => {
            let key = match hkcu.open_subkey_with_flags(path, winreg::enums::KEY_SET_VALUE) {
                Ok(k) => k,
                Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(e) => return Err(e),
            };
            match key.delete_value(name) {
                Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
                _ => Ok(()),
            }
        }
        Value::Dword(v) => {
            let (key, _) = hkcu.create_subkey(path)?;
            key.set_value(name, v)
        }
        Value::Text(s) => {
            let (key, _) = hkcu.create_subkey(path)?;
            key.set_value(name, s)
        }
    }
}

#[cfg(not(windows))]
pub fn read(target: &str) -> io::Result<Value> {
    split(target)?;
    Err(io::Error::new(io::ErrorKind::Unsupported, "Windows only"))
}

#[cfg(not(windows))]
pub fn write(target: &str, _value: &Value) -> io::Result<()> {
    split(target)?;
    Err(io::Error::new(io::ErrorKind::Unsupported, "Windows only"))
}

#[cfg(test)]
mod tests {
    use super::split;

    #[test]
    fn splits_targets_and_rejects_hklm() {
        let (p, n) = split("HKCU\\Software\\X\\Advanced\\ShowTaskViewButton").unwrap();
        assert_eq!(p, "Software\\X\\Advanced");
        assert_eq!(n, "ShowTaskViewButton");
        assert!(split("HKLM\\Software\\X\\Y").is_err());
    }
}
