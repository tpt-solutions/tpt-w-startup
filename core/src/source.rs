// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! The `StartupSource` trait every platform backend implements.

use crate::model::{SourceKind, StartupItem};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceError {
    /// The item id does not refer to an item this backend enumerated.
    NotFound(String),
    /// The operation needs elevated rights the process does not have.
    PermissionDenied(String),
    /// The backend cannot perform this operation (e.g. delete on a
    /// reversible-only source, or a stub backend).
    Unsupported(String),
    /// Any other backend failure, with a message suitable for logs.
    Other(String),
}

impl fmt::Display for SourceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SourceError::NotFound(id) => write!(f, "startup item not found: {id}"),
            SourceError::PermissionDenied(msg) => write!(f, "permission denied: {msg}"),
            SourceError::Unsupported(msg) => write!(f, "not supported: {msg}"),
            SourceError::Other(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for SourceError {}

/// One enumerable class of startup entries. Windows and Linux backends share
/// this interface so the engine and IPC layer never branch on platform.
///
/// Implementations must only act on items they enumerated: callers pass an
/// `id` obtained from [`StartupSource::enumerate`], and backends should reject
/// unknown ids with [`SourceError::NotFound`] rather than constructing paths
/// or keys from caller input.
pub trait StartupSource {
    fn kind(&self) -> SourceKind;

    /// Enumerate current entries. Category/explanation fields may be left as
    /// placeholders; the engine fills them in.
    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError>;

    /// Enable or disable an entry, preferring a reversible method.
    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError>;

    /// Permanently remove an entry.
    fn delete(&mut self, id: &str) -> Result<(), SourceError>;
}
