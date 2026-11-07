// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Post-v1 Archon backend stub (Phase 9, design in `docs/archon-design.md`).
//!
//! Compiled only with `--features archon`. It is deliberately inert until the
//! `tpt-archon` capability/telemetry API exists: it reports itself unavailable
//! and every mutating call returns [`SourceError::Unsupported`].

use crate::model::{SourceKind, StartupItem};
use crate::source::{SourceError, StartupSource};

#[derive(Debug, Default)]
pub struct ArchonStartupSource;

impl ArchonStartupSource {
    pub fn new() -> Self {
        Self
    }

    /// Always `false` until `tpt-archon` exposes a public API.
    pub fn is_available() -> bool {
        false
    }
}

impl StartupSource for ArchonStartupSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Archon
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        Err(SourceError::Unsupported(
            "Archon backend is design-only; tpt-archon API not available".into(),
        ))
    }

    fn set_enabled(&mut self, _id: &str, _enabled: bool) -> Result<(), SourceError> {
        Err(SourceError::Unsupported(
            "Archon backend is design-only".into(),
        ))
    }

    fn delete(&mut self, _id: &str) -> Result<(), SourceError> {
        Err(SourceError::Unsupported(
            "Archon backend is design-only".into(),
        ))
    }
}
