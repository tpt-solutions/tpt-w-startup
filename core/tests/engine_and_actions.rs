// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Engine and action tests against a mocked `StartupSource` (Phase 1 and 4 tests).

use std::collections::HashMap;
use tpt_startup_core::{
    execute, Action, ActionError, AuditLog, Category, Engine, SignatureStatus, SourceError,
    SourceKind, StartupItem, StartupSource,
};

fn item(id: &str, name: &str, command: Option<&str>) -> StartupItem {
    let mut it = StartupItem::new(id, SourceKind::Registry, name, id);
    it.command = command.map(str::to_string);
    it.signature = SignatureStatus::Valid;
    it
}

fn classified(engine: &Engine, mut it: StartupItem) -> StartupItem {
    engine.classify(&mut it);
    it
}

// ---- Mock backend -------------------------------------------------------

#[derive(Default)]
struct MockSource {
    items: HashMap<String, StartupItem>,
    fail_with: Option<SourceError>,
}

impl StartupSource for MockSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Registry
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        Ok(self.items.values().cloned().collect())
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        if let Some(e) = self.fail_with.clone() {
            return Err(e);
        }
        let it = self
            .items
            .get_mut(id)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        it.enabled = enabled;
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        if let Some(e) = self.fail_with.clone() {
            return Err(e);
        }
        self.items
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| SourceError::NotFound(id.to_string()))
    }
}

// ---- Engine: categorization ---------------------------------------------

#[test]
fn bundled_database_parses() {
    assert_eq!(Engine::bundled().database_version(), 1);
}

#[test]
fn curated_item_gets_database_category_and_publisher() {
    let engine = Engine::bundled();
    let it = classified(
        &engine,
        item("a", "Spotify", Some("C:\\Program Files\\Spotify.exe")),
    );
    assert_eq!(it.category, Category::Safe);
    assert_eq!(it.publisher.as_deref(), Some("Spotify AB"));
    assert!(!it.explanation.is_empty());
}

#[test]
fn critical_rule_cannot_be_downgraded_by_known_item() {
    // "explorer.exe" is a critical rule; make sure a later curated entry
    // cannot reclassify it.
    let engine = Engine::bundled();
    let it = classified(&engine, item("b", "Windows Explorer", Some("explorer.exe")));
    assert_eq!(it.category, Category::Critical);
}

#[test]
fn critical_rule_wins_even_when_item_looks_safe() {
    let engine = Engine::bundled();
    let mut it = item("c", "dbus-broker", Some("/usr/bin/dbus-broker"));
    it.signature = SignatureStatus::Valid;
    it.reversible_disable = true;
    let it = classified(&engine, it);
    assert_eq!(it.category, Category::Critical);
}

#[test]
fn temp_location_is_critical_heuristic() {
    let engine = Engine::bundled();
    let it = classified(
        &engine,
        item(
            "d",
            "helper",
            Some("C:\\Users\\x\\AppData\\Local\\Temp\\helper.exe"),
        ),
    );
    assert_eq!(it.category, Category::Critical);
}

#[test]
fn invalid_signature_is_critical_heuristic() {
    let engine = Engine::bundled();
    let mut it = item("e", "unknownthing", Some("C:\\Tools\\unknownthing.exe"));
    it.signature = SignatureStatus::Invalid;
    assert_eq!(classified(&engine, it).category, Category::Critical);
}

#[test]
fn unknown_item_falls_back_to_recommended() {
    let engine = Engine::bundled();
    let it = classified(
        &engine,
        item("f", "someapp", Some("C:\\Tools\\someapp.exe")),
    );
    assert_eq!(it.category, Category::Recommended);
    assert!(it.explanation.contains("Not in the curated list"));
}

#[test]
fn engine_overwrites_backend_supplied_category() {
    let engine = Engine::bundled();
    let mut it = item("g", "Windows Explorer", Some("explorer.exe"));
    it.category = Category::Safe; // attempt to spoof a lower tier
    engine.classify(&mut it);
    assert_eq!(it.category, Category::Critical);
}

#[test]
fn bad_database_is_rejected() {
    assert!(Engine::from_json("{not json").is_err());
}

// ---- Actions: confirmation and audit ------------------------------------

#[test]
fn disable_safe_item_needs_no_confirmation() {
    let engine = Engine::bundled();
    let it = classified(&engine, item("s1", "Spotify", None));
    let mut src = MockSource::default();
    src.items.insert(it.id.clone(), it.clone());
    let mut log = AuditLog::new();

    execute(&mut src, &it, Action::Disable, false, &mut log).unwrap();
    assert!(!src.items["s1"].enabled);
    assert_eq!(log.entries().len(), 1);
    assert!(log.last().unwrap().error.is_none());
}

#[test]
fn critical_item_refused_without_confirmation_and_not_logged() {
    let engine = Engine::bundled();
    let it = classified(
        &engine,
        item("c1", "Windows Explorer", Some("explorer.exe")),
    );
    let mut src = MockSource::default();
    src.items.insert(it.id.clone(), it.clone());
    let mut log = AuditLog::new();

    let err = execute(&mut src, &it, Action::Disable, false, &mut log).unwrap_err();
    assert_eq!(err, ActionError::ConfirmationRequired);
    assert!(src.items["c1"].enabled, "item must be untouched");
    assert!(log.entries().is_empty(), "refused actions are not attempts");
}

#[test]
fn critical_item_allowed_with_confirmation() {
    let engine = Engine::bundled();
    let it = classified(
        &engine,
        item("c2", "Windows Explorer", Some("explorer.exe")),
    );
    let mut src = MockSource::default();
    src.items.insert(it.id.clone(), it.clone());
    let mut log = AuditLog::new();

    execute(&mut src, &it, Action::Disable, true, &mut log).unwrap();
    assert!(!src.items["c2"].enabled);
}

#[test]
fn delete_always_requires_confirmation() {
    let engine = Engine::bundled();
    let it = classified(&engine, item("d1", "Spotify", None));
    let mut src = MockSource::default();
    src.items.insert(it.id.clone(), it.clone());
    let mut log = AuditLog::new();

    assert_eq!(
        execute(&mut src, &it, Action::Delete, false, &mut log),
        Err(ActionError::ConfirmationRequired)
    );
    assert!(src.items.contains_key("d1"));

    execute(&mut src, &it, Action::Delete, true, &mut log).unwrap();
    assert!(!src.items.contains_key("d1"));
}

#[test]
fn failed_action_is_recorded_with_error() {
    let engine = Engine::bundled();
    let it = classified(&engine, item("x1", "Spotify", None));
    let mut src = MockSource {
        fail_with: Some(SourceError::PermissionDenied("HKLM is read-only".into())),
        ..Default::default()
    };
    src.items.insert(it.id.clone(), it.clone());
    let mut log = AuditLog::new();

    let err = execute(&mut src, &it, Action::Enable, false, &mut log).unwrap_err();
    assert!(matches!(
        err,
        ActionError::Source(SourceError::PermissionDenied(_))
    ));
    let last = log.last().unwrap();
    assert_eq!(last.action, Action::Enable);
    assert!(last.error.as_deref().unwrap().contains("permission denied"));
}

#[test]
fn unknown_id_reports_not_found() {
    let engine = Engine::bundled();
    let it = classified(&engine, item("ghost", "Spotify", None));
    let mut src = MockSource::default();
    let mut log = AuditLog::new();
    assert_eq!(
        execute(&mut src, &it, Action::Disable, false, &mut log),
        Err(ActionError::Source(SourceError::NotFound("ghost".into())))
    );
}
