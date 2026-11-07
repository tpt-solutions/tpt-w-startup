// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Startup-folder backend tests against a temporary directory (CI-safe, no
//! real startup entries are touched).

use std::fs;
use std::path::PathBuf;
use tpt_startup_core::{SourceError, StartupSource};
use tpt_startup_windows::StartupFolderSource;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p =
            std::env::temp_dir().join(format!("tpt-startup-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        TempDir(p)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn enumerates_enabled_and_disabled_entries() {
    let tmp = TempDir::new("enumerate");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    fs::create_dir_all(tmp.0.join("disabled")).unwrap();
    fs::write(tmp.0.join("disabled").join("Old.lnk"), b"x").unwrap();

    let src = StartupFolderSource::new(&tmp.0, "user");
    let items = src.enumerate().unwrap();
    assert_eq!(items.len(), 2);

    let helper = items.iter().find(|i| i.display_name == "Helper").unwrap();
    assert!(helper.enabled);
    assert_eq!(helper.id, "startup:user:enabled/Helper.lnk");

    let old = items.iter().find(|i| i.display_name == "Old").unwrap();
    assert!(!old.enabled);
    assert!(old.reversible_disable);
}

#[test]
fn disable_moves_into_disabled_folder_and_enable_restores() {
    let tmp = TempDir::new("toggle");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    let mut src = StartupFolderSource::new(&tmp.0, "user");

    src.set_enabled("startup:user:enabled/Helper.lnk", false)
        .unwrap();
    assert!(!tmp.0.join("Helper.lnk").exists());
    assert!(tmp.0.join("disabled").join("Helper.lnk").exists());

    src.set_enabled("startup:user:disabled/Helper.lnk", true)
        .unwrap();
    assert!(tmp.0.join("Helper.lnk").exists());
    assert!(!tmp.0.join("disabled").join("Helper.lnk").exists());
}

#[test]
fn setting_current_state_is_a_noop() {
    let tmp = TempDir::new("noop");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    let mut src = StartupFolderSource::new(&tmp.0, "user");
    src.set_enabled("startup:user:enabled/Helper.lnk", true)
        .unwrap();
    assert!(tmp.0.join("Helper.lnk").exists());
}

#[test]
fn delete_removes_entry() {
    let tmp = TempDir::new("delete");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    let mut src = StartupFolderSource::new(&tmp.0, "user");
    src.delete("startup:user:enabled/Helper.lnk").unwrap();
    assert!(!tmp.0.join("Helper.lnk").exists());
}

#[test]
fn rejects_ids_not_from_enumeration() {
    let tmp = TempDir::new("reject");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    let mut src = StartupFolderSource::new(&tmp.0, "user");

    // Path traversal and unknown names must not resolve to anything.
    for bad in [
        "startup:user:enabled/../secret.txt",
        "startup:user:enabled/Missing.lnk",
        "startup:other/Helper.lnk",
        "registry:HKCU/whatever",
        "",
    ] {
        assert!(
            matches!(src.set_enabled(bad, false), Err(SourceError::NotFound(_))),
            "id should be rejected: {bad}"
        );
    }
}

#[test]
fn missing_folder_enumerates_empty() {
    let tmp = TempDir::new("missing");
    let src = StartupFolderSource::new(tmp.0.join("does-not-exist"), "user");
    assert!(src.enumerate().unwrap().is_empty());
}

#[test]
fn ids_from_another_scope_are_rejected() {
    let tmp = TempDir::new("scope");
    fs::write(tmp.0.join("Helper.lnk"), b"x").unwrap();
    let mut src = StartupFolderSource::new(&tmp.0, "user");
    // An all-users id must not resolve against the user folder.
    assert!(matches!(
        src.set_enabled("startup:all-users:enabled/Helper.lnk", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(tmp.0.join("Helper.lnk").exists());
}

#[test]
fn same_file_name_in_two_scopes_gets_distinct_ids() {
    let user = TempDir::new("scope-user");
    let all = TempDir::new("scope-all");
    fs::write(user.0.join("Helper.lnk"), b"x").unwrap();
    fs::write(all.0.join("Helper.lnk"), b"x").unwrap();
    let a = StartupFolderSource::new(&user.0, "user")
        .enumerate()
        .unwrap();
    let b = StartupFolderSource::new(&all.0, "all-users")
        .enumerate()
        .unwrap();
    assert_ne!(a[0].id, b[0].id);
}
