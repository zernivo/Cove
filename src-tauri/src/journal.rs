//! Change journal.
//!
//! Before Cove changes any setting it records the previous value here.
//! Restore replays the journal in reverse, so Windows ends up exactly
//! as it was, including settings that did not exist before.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// A setting's value as Cove saw it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Value {
    /// The setting did not exist; restore deletes it.
    Missing,
    Dword(u32),
    Text(String),
}

/// One recorded change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Which tweak made the change (e.g. "calm.hide-widgets").
    pub tweak: String,
    /// What was changed (e.g. a documented HKCU value path).
    pub target: String,
    pub previous: Value,
    pub applied: Value,
    /// Unix seconds.
    pub at: u64,
}

#[derive(Debug, Default)]
pub struct Journal {
    path: PathBuf,
    entries: Vec<Change>,
}

impl Journal {
    /// Load the journal from disk, or start an empty one.
    pub fn load(path: PathBuf) -> io::Result<Self> {
        let entries = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        Ok(Self { path, entries })
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn entries(&self) -> &[Change] {
        &self.entries
    }

    /// Record a change. Call this BEFORE applying it, so a crash
    /// mid-change can still be restored.
    pub fn record(
        &mut self,
        tweak: &str,
        target: &str,
        previous: Value,
        applied: Value,
    ) -> io::Result<()> {
        let at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.entries.push(Change {
            tweak: tweak.to_string(),
            target: target.to_string(),
            previous,
            applied,
            at,
        });
        self.save()
    }

    /// Drop the newest entry (used when a write failed after recording).
    pub fn pop_last(&mut self) -> io::Result<()> {
        self.entries.pop();
        self.save()
    }

    /// True if Cove has already changed this target.
    pub fn touched(&self, target: &str) -> bool {
        self.entries.iter().any(|c| c.target == target)
    }

    /// Take every change, newest first, ready to be reverted.
    pub fn take_for_restore(&mut self) -> io::Result<Vec<Change>> {
        let mut changes = std::mem::take(&mut self.entries);
        changes.reverse();
        self.save()?;
        Ok(changes)
    }

    /// Atomic write: temp file, then rename.
    fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, text)?;
        fs::rename(&tmp, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("cove-test-{}-{}", name, std::process::id()));
        p.push("journal.json");
        p
    }

    #[test]
    fn records_persist_and_restore_in_reverse() {
        let path = temp_path("roundtrip");
        let _ = fs::remove_file(&path);

        let mut j = Journal::load(path.clone()).unwrap();
        assert!(j.is_empty());
        j.record("a", "t1", Value::Dword(1), Value::Dword(0)).unwrap();
        j.record("b", "t2", Value::Missing, Value::Text("x".into())).unwrap();

        let mut reloaded = Journal::load(path.clone()).unwrap();
        assert_eq!(reloaded.len(), 2);

        let changes = reloaded.take_for_restore().unwrap();
        assert_eq!(changes[0].tweak, "b");
        assert_eq!(changes[1].tweak, "a");
        assert_eq!(changes[0].previous, Value::Missing);

        assert!(Journal::load(path).unwrap().is_empty());
    }
}
