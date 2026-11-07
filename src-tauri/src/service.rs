// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tauri-independent command logic (Phase 7). The Tauri command layer in
//! `lib.rs` is a thin wrapper around this type so it can be unit-tested
//! without a window.
//!
//! Every mutation re-enumerates the owning backend and re-classifies the item
//! before acting. Category and confirmation rules are therefore decided here,
//! from fresh backend data, and never taken from the frontend.

use tpt_startup_core::{
    execute, Action, ActionError, AuditEntry, AuditLog, Engine, SourceError, StartupItem,
    StartupSource,
};

pub type BoxedSource = Box<dyn StartupSource + Send>;

/// Result of a scan: classified items plus per-backend warnings, so one
/// failing source (e.g. permission denied) does not hide the others.
#[derive(Debug, Default, serde::Serialize)]
pub struct ScanResult {
    pub items: Vec<StartupItem>,
    pub warnings: Vec<String>,
}

pub struct Service {
    engine: Engine,
    sources: Vec<BoxedSource>,
    audit: AuditLog,
}

impl Service {
    pub fn new(sources: Vec<BoxedSource>) -> Self {
        Self {
            engine: Engine::bundled(),
            sources,
            audit: AuditLog::new(),
        }
    }

    /// Enumerate and classify every item. Sorted Critical first, then by name,
    /// so the most sensitive entries are the first thing the user sees.
    pub fn scan(&self) -> ScanResult {
        let mut result = ScanResult::default();
        for source in &self.sources {
            match source.enumerate() {
                Ok(items) => result.items.extend(items),
                Err(e) => result.warnings.push(format!("{:?}: {e}", source.kind())),
            }
        }
        for item in &mut result.items {
            self.engine.classify(item);
        }
        result.items.sort_by(|a, b| {
            b.category.cmp(&a.category).then_with(|| {
                a.display_name
                    .to_lowercase()
                    .cmp(&b.display_name.to_lowercase())
            })
        });
        result
    }

    /// Apply an action to an item found by a fresh scan.
    ///
    /// The owning backend is the one whose enumeration contains `id`. Matching
    /// on `SourceKind` would be ambiguous: HKCU and HKLM Run are both `Registry`.
    pub fn apply(&mut self, id: &str, action: Action, confirmed: bool) -> Result<(), ActionError> {
        let mut found = None;
        for (idx, source) in self.sources.iter().enumerate() {
            let Ok(items) = source.enumerate() else {
                continue;
            };
            if let Some(mut item) = items.into_iter().find(|i| i.id == id) {
                self.engine.classify(&mut item);
                found = Some((idx, item));
                break;
            }
        }
        let (idx, item) =
            found.ok_or_else(|| ActionError::Source(SourceError::NotFound(id.to_string())))?;
        execute(
            self.sources[idx].as_mut(),
            &item,
            action,
            confirmed,
            &mut self.audit,
        )
    }

    pub fn audit_entries(&self) -> Vec<AuditEntry> {
        self.audit.entries().to_vec()
    }

    /// Reverse the most recent successful enable or disable.
    ///
    /// The reversal goes through [`Service::apply`], so it gets the same
    /// confirmation rule and audit entry as any other action. Deletes cannot
    /// be undone.
    pub fn undo_last(&mut self, confirmed: bool) -> Result<(), ActionError> {
        let last = self
            .audit
            .entries()
            .iter()
            .rev()
            .find(|e| e.error.is_none())
            .cloned()
            .ok_or_else(|| {
                ActionError::Source(SourceError::Unsupported("nothing to undo".into()))
            })?;
        let inverse = last.action.inverse().ok_or_else(|| {
            ActionError::Source(SourceError::Unsupported(
                "deleted entries cannot be restored from here".into(),
            ))
        })?;
        self.apply(&last.item_id, inverse, confirmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tpt_startup_core::{SignatureStatus, SourceKind};

    /// Backend whose state is shared so tests can inspect it after the call.
    #[derive(Clone, Default)]
    struct Shared(Arc<Mutex<Vec<StartupItem>>>);

    struct MockBackend {
        kind: SourceKind,
        state: Shared,
    }

    impl StartupSource for MockBackend {
        fn kind(&self) -> SourceKind {
            self.kind
        }
        fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
            Ok(self.state.0.lock().unwrap().clone())
        }
        fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
            let mut items = self.state.0.lock().unwrap();
            let it = items
                .iter_mut()
                .find(|i| i.id == id)
                .ok_or_else(|| SourceError::NotFound(id.into()))?;
            it.enabled = enabled;
            Ok(())
        }
        fn delete(&mut self, id: &str) -> Result<(), SourceError> {
            let mut items = self.state.0.lock().unwrap();
            let before = items.len();
            items.retain(|i| i.id != id);
            if items.len() == before {
                return Err(SourceError::NotFound(id.into()));
            }
            Ok(())
        }
    }

    struct FailingBackend;

    impl StartupSource for FailingBackend {
        fn kind(&self) -> SourceKind {
            SourceKind::Cron
        }
        fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
            Err(SourceError::PermissionDenied("crontab".into()))
        }
        fn set_enabled(&mut self, _: &str, _: bool) -> Result<(), SourceError> {
            Err(SourceError::Unsupported("x".into()))
        }
        fn delete(&mut self, _: &str) -> Result<(), SourceError> {
            Err(SourceError::Unsupported("x".into()))
        }
    }

    fn item(id: &str, name: &str, command: &str) -> StartupItem {
        let mut it = StartupItem::new(id, SourceKind::XdgAutostart, name, id);
        it.command = Some(command.into());
        it.signature = SignatureStatus::Valid;
        it
    }

    fn service_with(items: Vec<StartupItem>) -> (Service, Shared) {
        let shared = Shared(Arc::new(Mutex::new(items)));
        let svc = Service::new(vec![Box::new(MockBackend {
            kind: SourceKind::XdgAutostart,
            state: shared.clone(),
        })]);
        (svc, shared)
    }

    #[test]
    fn scan_classifies_and_sorts_critical_first() {
        let (svc, _) = service_with(vec![
            item("a", "Spotify", "spotify"),
            item("b", "Windows Explorer", "explorer.exe"),
        ]);
        let scan = svc.scan();
        assert_eq!(scan.items[0].display_name, "Windows Explorer");
        assert_eq!(scan.items[0].category, tpt_startup_core::Category::Critical);
        assert!(scan.warnings.is_empty());
    }

    #[test]
    fn one_failing_backend_becomes_a_warning() {
        let (mut svc, _) = service_with(vec![item("a", "Spotify", "spotify")]);
        svc.sources.push(Box::new(FailingBackend));
        let scan = svc.scan();
        assert_eq!(scan.items.len(), 1);
        assert_eq!(scan.warnings.len(), 1);
        assert!(scan.warnings[0].contains("permission denied"));
    }

    #[test]
    fn critical_action_needs_confirmation_even_when_frontend_asks_for_none() {
        let (mut svc, shared) = service_with(vec![item("b", "Windows Explorer", "explorer.exe")]);
        assert_eq!(
            svc.apply("b", Action::Disable, false),
            Err(ActionError::ConfirmationRequired)
        );
        assert!(shared.0.lock().unwrap()[0].enabled);

        svc.apply("b", Action::Disable, true).unwrap();
        assert!(!shared.0.lock().unwrap()[0].enabled);
        assert_eq!(svc.audit_entries().len(), 1);
    }

    #[test]
    fn unknown_id_is_rejected_without_touching_backends() {
        let (mut svc, shared) = service_with(vec![item("a", "Spotify", "spotify")]);
        assert_eq!(
            svc.apply("../../etc/passwd", Action::Delete, true),
            Err(ActionError::Source(SourceError::NotFound(
                "../../etc/passwd".into()
            )))
        );
        assert_eq!(shared.0.lock().unwrap().len(), 1);
        assert!(svc.audit_entries().is_empty());
    }

    #[test]
    fn delete_requires_confirmation_for_safe_item() {
        let (mut svc, shared) = service_with(vec![item("a", "Spotify", "spotify")]);
        assert_eq!(
            svc.apply("a", Action::Delete, false),
            Err(ActionError::ConfirmationRequired)
        );
        svc.apply("a", Action::Delete, true).unwrap();
        assert!(shared.0.lock().unwrap().is_empty());
    }

    #[test]
    fn undo_reverses_last_successful_disable() {
        let (mut svc, shared) = service_with(vec![item("a", "Spotify", "spotify")]);
        svc.apply("a", Action::Disable, false).unwrap();
        assert!(!shared.0.lock().unwrap()[0].enabled);

        svc.undo_last(false).unwrap();
        assert!(shared.0.lock().unwrap()[0].enabled);
        // The undo itself is recorded like any other action.
        assert_eq!(svc.audit_entries().last().unwrap().action, Action::Enable);
    }

    #[test]
    fn undo_refuses_to_restore_a_delete() {
        let (mut svc, _) = service_with(vec![item("a", "Spotify", "spotify")]);
        svc.apply("a", Action::Delete, true).unwrap();
        assert!(matches!(
            svc.undo_last(true),
            Err(ActionError::Source(SourceError::Unsupported(_)))
        ));
    }

    #[test]
    fn undo_with_empty_log_is_an_error() {
        let (mut svc, _) = service_with(vec![item("a", "Spotify", "spotify")]);
        assert!(matches!(
            svc.undo_last(false),
            Err(ActionError::Source(SourceError::Unsupported(_)))
        ));
    }
}
