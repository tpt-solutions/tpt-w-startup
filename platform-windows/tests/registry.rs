// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Registry Run backend against a throwaway HKCU key. Never touches real
//! Run entries. Windows-only.

#![cfg(windows)]

use tpt_startup_core::{SourceError, StartupSource};
use tpt_startup_windows::registry::{Hive, RegistryRunSource, DISABLED_PREFIX};
use winreg::enums::HKEY_CURRENT_USER;
use winreg::RegKey;

const TEST_ROOT: &str = r"Software\tpt-startup-test";

/// Unique HKCU subkey, removed on drop.
struct TestKey {
    subkey: String,
}

impl TestKey {
    fn new(name: &str) -> Self {
        let subkey = format!(r"{TEST_ROOT}\{}\{name}", std::process::id());
        let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&subkey);
        RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(&subkey)
            .unwrap();
        TestKey { subkey }
    }

    fn set(&self, name: &str, value: &str) {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey(&self.subkey)
            .unwrap();
        key.set_value(name, &value).unwrap();
    }

    fn has(&self, name: &str) -> bool {
        RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(&self.subkey)
            .unwrap()
            .get_raw_value(name)
            .is_ok()
    }

    fn source(&self) -> RegistryRunSource {
        RegistryRunSource::with_subkey(Hive::CurrentUser, self.subkey.clone())
    }
}

impl Drop for TestKey {
    fn drop(&mut self) {
        let root = RegKey::predef(HKEY_CURRENT_USER);
        let _ = root.delete_subkey_all(&self.subkey);
        // Remove the per-process folder if nothing else is in it.
        if let Some((parent, _)) = self.subkey.rsplit_once('\\') {
            let _ = root.delete_subkey(parent);
        }
    }
}

#[test]
fn enumerates_values_with_commands() {
    let k = TestKey::new("enumerate");
    k.set("Helper", r"C:\Tools\helper.exe");
    let items = k.source().enumerate().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].display_name, "Helper");
    assert_eq!(items[0].command.as_deref(), Some(r"C:\Tools\helper.exe"));
    assert!(items[0].enabled);
    assert!(items[0].reversible_disable);
}

#[test]
fn disable_renames_and_enable_restores() {
    let k = TestKey::new("toggle");
    k.set("Helper", r"C:\Tools\helper.exe");
    let mut src = k.source();
    let id = src.enumerate().unwrap()[0].id.clone();

    src.set_enabled(&id, false).unwrap();
    assert!(!k.has("Helper"));
    assert!(k.has(&format!("{DISABLED_PREFIX}Helper")));

    let disabled = src.enumerate().unwrap();
    assert_eq!(disabled.len(), 1);
    assert!(!disabled[0].enabled);
    assert_eq!(disabled[0].display_name, "Helper");

    src.set_enabled(&disabled[0].id, true).unwrap();
    assert!(k.has("Helper"));
    assert!(!k.has(&format!("{DISABLED_PREFIX}Helper")));
}

#[test]
fn delete_removes_value() {
    let k = TestKey::new("delete");
    k.set("Helper", "x.exe");
    let mut src = k.source();
    let id = src.enumerate().unwrap()[0].id.clone();
    src.delete(&id).unwrap();
    assert!(!k.has("Helper"));
}

#[test]
fn rejects_ids_that_are_not_present() {
    let k = TestKey::new("reject");
    k.set("Helper", "x.exe");
    let mut src = k.source();
    let fake = format!(r"registry:HKCU\{}\Missing", k.subkey);
    assert!(matches!(
        src.set_enabled(&fake, false),
        Err(SourceError::NotFound(_))
    ));
    assert!(matches!(
        src.delete(r"registry:HKLM\Evil"),
        Err(SourceError::NotFound(_))
    ));
    assert!(k.has("Helper"));
}

#[test]
fn missing_key_enumerates_empty() {
    let src = RegistryRunSource::with_subkey(
        Hive::CurrentUser,
        format!(r"{TEST_ROOT}\does-not-exist-{}", std::process::id()),
    );
    assert!(src.enumerate().unwrap().is_empty());
}
