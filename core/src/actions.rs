// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Item actions and the in-session audit trail (Phase 6).
//!
//! All mutations go through [`execute`] so the confirmation rule and the audit
//! log cannot be skipped by a caller.

use crate::model::StartupItem;
use crate::source::{SourceError, StartupSource};
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Enable,
    Disable,
    Delete,
}

impl Action {
    /// The action that reverses this one. Deletes have no inverse: the entry
    /// is gone and is not restored by the app.
    pub fn inverse(self) -> Option<Action> {
        match self {
            Action::Enable => Some(Action::Disable),
            Action::Disable => Some(Action::Enable),
            Action::Delete => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionError {
    /// The action needs explicit user confirmation and none was given.
    ConfirmationRequired,
    Source(SourceError),
}

impl fmt::Display for ActionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionError::ConfirmationRequired => write!(f, "this action requires confirmation"),
            ActionError::Source(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for ActionError {}

impl From<SourceError> for ActionError {
    fn from(e: SourceError) -> Self {
        ActionError::Source(e)
    }
}

/// Whether `action` on `item` needs confirmation.
///
/// Rules: any change to a Critical item needs confirmation, and deletion
/// always does (disabling is preferred and reversible).
pub fn requires_confirmation(item: &StartupItem, action: Action) -> bool {
    item.category.requires_confirmation() || action == Action::Delete
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct AuditEntry {
    pub item_id: String,
    pub display_name: String,
    pub action: Action,
    /// Seconds since the Unix epoch.
    pub timestamp: u64,
    /// `None` on success, otherwise the error message.
    pub error: Option<String>,
}

/// Session-scoped log of attempted actions. Not persisted.
#[derive(Debug, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    pub fn last(&self) -> Option<&AuditEntry> {
        self.entries.last()
    }

    fn record(&mut self, item: &StartupItem, action: Action, error: Option<String>) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        self.entries.push(AuditEntry {
            item_id: item.id.clone(),
            display_name: item.display_name.clone(),
            action,
            timestamp,
            error,
        });
    }
}

/// Apply `action` to `item` through `source`, enforcing confirmation and
/// recording the attempt (success or failure) in `log`.
///
/// Refused actions (missing confirmation) are not logged, since nothing was
/// attempted.
pub fn execute(
    source: &mut dyn StartupSource,
    item: &StartupItem,
    action: Action,
    confirmed: bool,
    log: &mut AuditLog,
) -> Result<(), ActionError> {
    if requires_confirmation(item, action) && !confirmed {
        return Err(ActionError::ConfirmationRequired);
    }

    let result = match action {
        Action::Enable => source.set_enabled(&item.id, true),
        Action::Disable => source.set_enabled(&item.id, false),
        Action::Delete => source.delete(&item.id),
    };

    log.record(item, action, result.as_ref().err().map(|e| e.to_string()));
    result.map_err(ActionError::from)
}
