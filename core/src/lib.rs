// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `tpt-startup-core`: platform-independent model, categorization, boot-impact
//! normalization and action engine. Platform backends implement
//! [`source::StartupSource`]; nothing in this crate touches the OS directly.

pub mod actions;
pub mod engine;
pub mod impact;
pub mod model;
pub mod runner;
pub mod source;

#[cfg(feature = "archon")]
pub mod archon;

pub use actions::{execute, Action, ActionError, AuditEntry, AuditLog};
pub use engine::Engine;
pub use model::{BootImpact, Category, SignatureStatus, SourceKind, StartupItem};
pub use source::{SourceError, StartupSource};

/// Classify every item (category and explanation) with the engine.
/// Boot impact is set separately via [`impact`] from platform timing data.
pub fn classify_all(engine: &Engine, items: &mut [StartupItem]) {
    for item in items.iter_mut() {
        engine.classify(item);
    }
}
