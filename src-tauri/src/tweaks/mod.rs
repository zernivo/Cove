//! Tweaks: small, independent, reversible changes.
//!
//! Every tweak writes through the journal, uses documented settings only,
//! and never needs admin rights.
#![allow(dead_code)]

pub mod calm;

use serde::Serialize;
use std::io;

use crate::journal::{Change, Journal};

/// What the UI shows for a tweak.
#[derive(Debug, Clone, Serialize)]
pub struct TweakInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    /// Where the documented setting lives (shown in Cove for transparency).
    pub setting: &'static str,
}

pub trait Tweak {
    fn info(&self) -> TweakInfo;
    /// Record the previous value in the journal, then apply.
    fn apply(&self, journal: &mut Journal) -> io::Result<()>;
    /// Put back one recorded change.
    fn revert(&self, change: &Change) -> io::Result<()>;
}
