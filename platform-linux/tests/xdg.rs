// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! XDG autostart backend tests against a temporary directory.

use std::fs;
use std::path::PathBuf;
use tpt_startup_core::{SourceError, StartupSource};
use tpt_startup_linux::xdg::{parse_desktop_entry, set_hidden};
use tpt_startup_linux::XdgAutostartSource;

struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!("tpt-startup-xdg-{}-{name}", std::process::id()));
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

const SAMPLE: &str = "\
[Desktop Entry]
Type=Application
Name=Example Sync
Exec=/usr/bin/example-sync --background
X-GNOME-Autostart-enabled=true
";

#[test]
fn parses_only_desktop_entry_group() {
    let text = "[Other]\nName=wrong\n[Desktop Entry]\nName=Right\n# c\nExec=x\n";
    let f = parse_desktop_entry(text);
    assert_eq!(f.get("Name").map(String::as_str), Some("Right"));
    assert_eq!(f.get("Exec").map(String::as_str), Some("x"));
}

#[test]
fn set_hidden_replaces_existing_key_and_preserves_rest() {
    let text = "[Desktop Entry]\nName=A\nHidden=false\nExec=y\n";
    assert_eq!(
        set_hidden(text, true),
        "[Desktop Entry]\nName=A\nHidden=true\nExec=y\n"
    );
}

#[test]
fn set_hidden_adds_key_when_missing() {
    let out = set_hidden(SAMPLE, true);
    assert!(out.contains("Hidden=true"));
    assert!(out.contains("Exec=/usr/bin/example-sync --background"));
    assert_eq!(out.matches("Hidden=").count(), 1);
}

#[test]
fn set_hidden_does_not_touch_other_groups() {
    let text = "[Desktop Entry]\nName=A\n[Desktop Action X]\nExec=z\n";
    let out = set_hidden(text, true);
    let action_section = out.split("[Desktop Action X]").nth(1).unwrap();
    assert!(!action_section.contains("Hidden="));
}

#[test]
fn enumerate_disable_enable_roundtrip() {
    let tmp = TempDir::new("roundtrip");
    let file = tmp.0.join("example-sync.desktop");
    fs::write(&file, SAMPLE).unwrap();
    let mut src = XdgAutostartSource::new(&tmp.0);

    let items = src.enumerate().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].display_name, "Example Sync");
    assert!(items[0].enabled);
    assert!(items[0].reversible_disable);

    src.set_enabled("xdg:example-sync.desktop", false).unwrap();
    let after = src.enumerate().unwrap();
    assert!(!after[0].enabled);
    assert!(file.exists(), "disable must not delete the file");

    src.set_enabled("xdg:example-sync.desktop", true).unwrap();
    assert!(src.enumerate().unwrap()[0].enabled);
}

#[test]
fn delete_removes_file() {
    let tmp = TempDir::new("delete");
    let file = tmp.0.join("example-sync.desktop");
    fs::write(&file, SAMPLE).unwrap();
    let mut src = XdgAutostartSource::new(&tmp.0);
    src.delete("xdg:example-sync.desktop").unwrap();
    assert!(!file.exists());
}

#[test]
fn ignores_non_desktop_files_and_rejects_unknown_ids() {
    let tmp = TempDir::new("reject");
    fs::write(tmp.0.join("notes.txt"), "hi").unwrap();
    fs::write(tmp.0.join("example-sync.desktop"), SAMPLE).unwrap();
    let mut src = XdgAutostartSource::new(&tmp.0);
    assert_eq!(src.enumerate().unwrap().len(), 1);

    for bad in [
        "xdg:notes.txt",
        "xdg:../etc/passwd",
        "xdg:missing.desktop",
        "cron:user:1",
    ] {
        assert!(
            matches!(src.set_enabled(bad, false), Err(SourceError::NotFound(_))),
            "should reject {bad}"
        );
    }
}
