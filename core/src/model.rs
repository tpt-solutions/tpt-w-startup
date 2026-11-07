// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Core data model shared by every platform backend.

use serde::{Deserialize, Serialize};

/// Where a startup item was discovered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceKind {
    Registry,
    StartupFolder,
    TaskScheduler,
    SystemdUnit,
    Cron,
    XdgAutostart,
    /// Post-v1 Archon lazy-loaded entries (design-only; see docs/archon-design.md).
    Archon,
}

/// Plain-English safety tier shown to the user.
///
/// Categories are computed by [`crate::engine`], never supplied by a backend
/// or the UI, so they cannot be overridden from outside the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Safe,
    Recommended,
    Critical,
}

impl Category {
    /// Critical items require explicit confirmation before any change.
    pub fn requires_confirmation(self) -> bool {
        matches!(self, Category::Critical)
    }
}

/// Normalized boot/login impact on a shared High/Medium/Low scale.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootImpact {
    High,
    Medium,
    Low,
    /// No timing signal exists for this source (e.g. cron, XDG autostart).
    Unknown,
}

/// Publisher signature state as reported by the platform backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignatureStatus {
    Valid,
    Invalid,
    Unsigned,
    Unknown,
}

/// A single startup entry, regardless of where it lives.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StartupItem {
    /// Stable identifier, unique within a backend (e.g. `registry:HKCU\...\Run\Foo`).
    pub id: String,
    pub source: SourceKind,
    pub display_name: String,
    /// Command line or target path, when known.
    pub command: Option<String>,
    /// Where the entry is stored (registry key, file path, unit name, ...).
    pub location: String,
    pub publisher: Option<String>,
    pub signature: SignatureStatus,
    pub enabled: bool,
    /// Whether disabling can be undone without deleting anything.
    pub reversible_disable: bool,
    /// Filled in by the engine; any value supplied by a backend is overwritten.
    pub category: Category,
    /// Filled in by the engine; any value supplied by a backend is overwritten.
    pub explanation: String,
    pub boot_impact: BootImpact,
}

impl StartupItem {
    /// Convenience constructor for a backend. Category and explanation are
    /// placeholders until the engine classifies the item.
    pub fn new(
        id: impl Into<String>,
        source: SourceKind,
        display_name: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source,
            display_name: display_name.into(),
            command: None,
            location: location.into(),
            publisher: None,
            signature: SignatureStatus::Unknown,
            enabled: true,
            reversible_disable: false,
            category: Category::Recommended,
            explanation: String::new(),
            boot_impact: BootImpact::Unknown,
        }
    }
}
