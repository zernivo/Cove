//! Tweaks: small, independent, reversible changes.
//!
//! Every tweak writes through the journal, uses documented per-user
//! settings only, and never needs admin rights. Reverting is generic:
//! restore writes each journaled target back to its previous value.

pub mod calm;

use serde::Serialize;
use std::io;

use crate::journal::{Journal, Value};
use crate::registry;

/// What the UI shows for a tweak.
#[derive(Debug, Clone, Serialize)]
pub struct TweakInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// The matching Windows Settings page (fallback if a write is blocked).
    pub settings_uri: &'static str,
    /// None = unknown (not on Windows or unreadable).
    pub applied: Option<bool>,
}

/// A tweak made of one or more HKCU DWORD writes.
pub struct RegTweak {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub settings_uri: &'static str,
    pub writes: &'static [(&'static str, u32)],
}

impl RegTweak {
    pub fn info(&self) -> TweakInfo {
        TweakInfo {
            id: self.id,
            title: self.title,
            description: self.description,
            settings_uri: self.settings_uri,
            applied: self.is_applied(),
        }
    }

    pub fn is_applied(&self) -> Option<bool> {
        let mut all = true;
        for (target, v) in self.writes {
            match registry::read(target) {
                Ok(Value::Dword(cur)) => all &= cur == *v,
                Ok(_) => all = false,
                Err(_) => return None,
            }
        }
        Some(all)
    }

    /// Record each previous value, then write. On a blocked write, the
    /// journal entry is dropped and earlier writes of this tweak are undone.
    pub fn apply(&self, journal: &mut Journal) -> io::Result<()> {
        let mut done: Vec<(&str, Value)> = Vec::new();
        for (target, v) in self.writes {
            let previous = registry::read(target)?;
            if previous == Value::Dword(*v) {
                continue; // already set; nothing to record
            }
            journal.record(self.id, target, previous.clone(), Value::Dword(*v))?;
            if let Err(e) = registry::write(target, &Value::Dword(*v)) {
                journal.pop_last()?;
                for (t, prev) in done.iter().rev() {
                    let _ = registry::write(t, prev);
                    let _ = journal.pop_last();
                }
                return Err(e);
            }
            done.push((target, previous));
        }
        Ok(())
    }
}
