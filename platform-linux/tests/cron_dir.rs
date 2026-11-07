// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! `/etc/cron.d` backend tests against a temporary directory. Nothing under
//! the real /etc is read or written.

use std::fs;
use std::path::PathBuf;
use tpt_startup_core::{BootImpact, SourceError, StartupSource};
use tpt_startup_linux::cron::DISABLED_MARKER;
use tpt_startup_linux::CronDirSource;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p =
            std::env::temp_dir().join(format!("tpt-startup-crond-{}-{name}", std::process::id()));
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
fn lists_reboot_entries_from_valid_file_names_only() {
    let tmp = TempDir::new("list");
    fs::write(
        tmp.0.join("sync-job"),
        "@reboot root /usr/local/bin/sync\n0 2 * * * root nightly\n",
    )
    .unwrap();
    fs::write(tmp.0.join("ignored.conf"), "@reboot root /nope\n").unwrap();
    fs::write(tmp.0.join("backup~"), "@reboot root /nope\n").unwrap();

    let items = CronDirSource::new(&tmp.0).enumerate().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "cron:system:sync-job:1");
    assert_eq!(items[0].command.as_deref(), Some("/usr/local/bin/sync"));
    assert_eq!(items[0].boot_impact, BootImpact::Unknown);
}

#[test]
fn disable_then_enable_rewrites_only_that_line() {
    let tmp = TempDir::new("toggle");
    let file = tmp.0.join("sync-job");
    fs::write(
        &file,
        "0 2 * * * root nightly\n@reboot root /usr/local/bin/sync\n",
    )
    .unwrap();
    let mut src = CronDirSource::new(&tmp.0);

    src.set_enabled("cron:system:sync-job:2", false).unwrap();
    let after = fs::read_to_string(&file).unwrap();
    assert_eq!(
        after,
        format!("0 2 * * * root nightly\n{DISABLED_MARKER}@reboot root /usr/local/bin/sync\n")
    );
    assert!(!src.enumerate().unwrap()[0].enabled);

    src.set_enabled("cron:system:sync-job:2", true).unwrap();
    assert_eq!(
        fs::read_to_string(&file).unwrap(),
        "0 2 * * * root nightly\n@reboot root /usr/local/bin/sync\n"
    );
}

#[test]
fn delete_removes_one_entry() {
    let tmp = TempDir::new("delete");
    let file = tmp.0.join("sync-job");
    fs::write(&file, "@reboot root /a\n@reboot root /b\n").unwrap();
    CronDirSource::new(&tmp.0)
        .delete("cron:system:sync-job:1")
        .unwrap();
    assert_eq!(fs::read_to_string(&file).unwrap(), "@reboot root /b\n");
}

#[test]
fn rejects_ids_that_do_not_name_a_listed_entry() {
    let tmp = TempDir::new("reject");
    fs::write(tmp.0.join("sync-job"), "@reboot root /a\n").unwrap();
    fs::write(tmp.0.join("secret"), "@reboot root /x\n").unwrap();
    let mut src = CronDirSource::new(&tmp.0);
    for bad in [
        "cron:system:../sync-job:1",
        "cron:system:sync-job.bak:1",
        "cron:system:missing:1",
        "cron:system:sync-job:99",
        "cron:system:sync-job:notanumber",
        "cron:user:1",
    ] {
        assert!(
            matches!(src.set_enabled(bad, false), Err(SourceError::NotFound(_))),
            "should reject {bad}"
        );
    }
}

#[test]
fn missing_directory_enumerates_empty() {
    let tmp = TempDir::new("missing");
    let src = CronDirSource::new(tmp.0.join("nope"));
    assert!(src.enumerate().unwrap().is_empty());
}
