// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! cron and systemd backends driven by a fake command runner. These run on any
//! OS and never touch the real crontab or systemd.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io;
use std::rc::Rc;
use tpt_startup_core::{BootImpact, SourceError, StartupSource};
use tpt_startup_linux::cron::DISABLED_MARKER;
use tpt_startup_linux::{CommandOutput, CommandRunner, CronUserSource, SystemdUserSource};

/// Records calls and answers from a script keyed by "program arg arg".
/// The crontab is modelled as state, so edits show up on the next read.
#[derive(Clone, Default)]
struct FakeRunner {
    replies: Rc<RefCell<HashMap<String, CommandOutput>>>,
    calls: Rc<RefCell<Vec<String>>>,
    crontab: Rc<RefCell<String>>,
}

impl FakeRunner {
    fn reply(&self, key: &str, success: bool, stdout: &str, stderr: &str) {
        self.replies.borrow_mut().insert(
            key.to_string(),
            CommandOutput {
                success,
                stdout: stdout.into(),
                stderr: stderr.into(),
            },
        );
    }

    fn called(&self, prefix: &str) -> bool {
        self.calls.borrow().iter().any(|c| c.starts_with(prefix))
    }
}

impl CommandRunner for FakeRunner {
    fn run(&self, program: &str, args: &[&str], stdin: Option<&str>) -> io::Result<CommandOutput> {
        let key = std::iter::once(program)
            .chain(args.iter().copied())
            .collect::<Vec<_>>()
            .join(" ");
        self.calls.borrow_mut().push(key.clone());

        if program == "crontab" {
            if args == ["-l"] {
                let text = self.crontab.borrow().clone();
                return Ok(if text.is_empty() {
                    CommandOutput {
                        success: false,
                        stdout: String::new(),
                        stderr: "no crontab for tester".into(),
                    }
                } else {
                    CommandOutput {
                        success: true,
                        stdout: text,
                        stderr: String::new(),
                    }
                });
            }
            if args == ["-"] {
                *self.crontab.borrow_mut() = stdin.unwrap_or_default().to_string();
                return Ok(CommandOutput {
                    success: true,
                    stdout: String::new(),
                    stderr: String::new(),
                });
            }
        }

        match self.replies.borrow().get(&key) {
            Some(out) => Ok(out.clone()),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "no such program")),
        }
    }
}

// ---- cron ----------------------------------------------------------------

#[test]
fn cron_enumerates_reboot_lines() {
    let r = FakeRunner::default();
    *r.crontab.borrow_mut() = "@reboot /usr/bin/foo\n0 1 * * * nightly\n".into();
    let src = CronUserSource::new(r);
    let items = src.enumerate().unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, "cron:user:1");
    assert!(items[0].enabled);
    assert_eq!(items[0].boot_impact, BootImpact::Unknown);
}

#[test]
fn cron_missing_crontab_is_empty_not_error() {
    let r = FakeRunner::default();
    let src = CronUserSource::new(r);
    assert!(src.enumerate().unwrap().is_empty());
}

#[test]
fn cron_disable_enable_roundtrip_through_crontab() {
    let r = FakeRunner::default();
    *r.crontab.borrow_mut() = "@reboot /usr/bin/foo\n".into();
    let mut src = CronUserSource::new(r.clone());

    src.set_enabled("cron:user:1", false).unwrap();
    assert_eq!(
        *r.crontab.borrow(),
        format!("{DISABLED_MARKER}@reboot /usr/bin/foo\n")
    );
    assert!(!src.enumerate().unwrap()[0].enabled);

    src.set_enabled("cron:user:1", true).unwrap();
    assert_eq!(*r.crontab.borrow(), "@reboot /usr/bin/foo\n");
}

#[test]
fn cron_delete_removes_line() {
    let r = FakeRunner::default();
    *r.crontab.borrow_mut() = "0 1 * * * nightly\n@reboot /usr/bin/foo\n".into();
    let mut src = CronUserSource::new(r.clone());
    src.delete("cron:user:2").unwrap();
    assert_eq!(*r.crontab.borrow(), "0 1 * * * nightly\n");
}

#[test]
fn cron_rejects_ids_not_pointing_at_an_entry() {
    let r = FakeRunner::default();
    *r.crontab.borrow_mut() = "0 1 * * * nightly\n".into();
    let mut src = CronUserSource::new(r.clone());
    assert!(matches!(
        src.set_enabled("cron:user:1", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(matches!(
        src.set_enabled("cron:user:../../etc", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(matches!(
        src.delete("systemd:foo.service"),
        Err(SourceError::NotFound(_))
    ));
    assert_eq!(*r.crontab.borrow(), "0 1 * * * nightly\n");
    assert!(r.calls.borrow().iter().all(|c| c != "crontab -"));
}

#[test]
fn cron_without_crontab_program_is_unsupported() {
    struct NoProgram;
    impl CommandRunner for NoProgram {
        fn run(&self, _: &str, _: &[&str], _: Option<&str>) -> io::Result<CommandOutput> {
            Err(io::Error::new(io::ErrorKind::NotFound, "not found"))
        }
    }
    let src = CronUserSource::new(NoProgram);
    assert!(matches!(src.enumerate(), Err(SourceError::Unsupported(_))));
}

// ---- systemd --------------------------------------------------------------

const LISTING: &str = "\
pipewire.service          enabled enabled
backup.service            enabled -
old-thing.service         disabled enabled
dbus-broker.service       static  -
masked.service            masked  -
";

const LIST_CMD: &str =
    "systemctl --user list-unit-files --type=service --no-legend --no-pager --plain";

fn systemd_runner() -> FakeRunner {
    let r = FakeRunner::default();
    r.reply(LIST_CMD, true, LISTING, "");
    r.reply(
        "systemd-analyze --user blame",
        true,
        "   4.200s backup.service\n    120ms pipewire.service\n",
        "",
    );
    r
}

#[test]
fn systemd_lists_startup_units_only() {
    let src = SystemdUserSource::new(systemd_runner());
    let items = src.enumerate().unwrap();
    let mut ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
    ids.sort();
    assert_eq!(
        ids,
        vec![
            "systemd:backup.service",
            "systemd:old-thing.service",
            "systemd:pipewire.service"
        ]
    );
}

#[test]
fn systemd_disabled_but_preset_enabled_stays_visible() {
    let src = SystemdUserSource::new(systemd_runner());
    let items = src.enumerate().unwrap();
    let old = items
        .iter()
        .find(|i| i.display_name == "old-thing")
        .unwrap();
    assert!(!old.enabled);
    assert!(old.reversible_disable);
}

#[test]
fn systemd_boot_impact_from_blame() {
    let src = SystemdUserSource::new(systemd_runner());
    let items = src.enumerate().unwrap();
    let backup = items.iter().find(|i| i.display_name == "backup").unwrap();
    assert_eq!(backup.boot_impact, BootImpact::High);
    let pw = items.iter().find(|i| i.display_name == "pipewire").unwrap();
    assert_eq!(pw.boot_impact, BootImpact::Low);
    let old = items
        .iter()
        .find(|i| i.display_name == "old-thing")
        .unwrap();
    assert_eq!(old.boot_impact, BootImpact::Unknown);
}

#[test]
fn systemd_blame_unavailable_leaves_impact_unknown() {
    let r = FakeRunner::default();
    r.reply(LIST_CMD, true, LISTING, "");
    let items = SystemdUserSource::new(r).enumerate().unwrap();
    assert!(items.iter().all(|i| i.boot_impact == BootImpact::Unknown));
}

#[test]
fn systemd_disable_calls_systemctl_with_unit_name() {
    let r = systemd_runner();
    r.reply("systemctl --user disable backup.service", true, "", "");
    let mut src = SystemdUserSource::new(r.clone());
    src.set_enabled("systemd:backup.service", false).unwrap();
    assert!(r.called("systemctl --user disable backup.service"));
}

#[test]
fn systemd_refuses_units_outside_the_startup_listing() {
    let r = systemd_runner();
    let mut src = SystemdUserSource::new(r.clone());
    assert!(matches!(
        src.set_enabled("systemd:dbus-broker.service", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(matches!(
        src.set_enabled("systemd:x; rm -rf ~", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(!r.called("systemctl --user disable"));
}

#[test]
fn systemd_delete_is_unsupported() {
    let mut src = SystemdUserSource::new(systemd_runner());
    assert!(matches!(
        src.delete("systemd:backup.service"),
        Err(SourceError::Unsupported(_))
    ));
}

#[test]
fn systemd_failed_systemctl_reports_stderr() {
    let r = systemd_runner();
    r.reply(
        "systemctl --user enable backup.service",
        false,
        "",
        "Failed to enable unit",
    );
    let mut src = SystemdUserSource::new(r);
    let err = src.set_enabled("systemd:backup.service", true).unwrap_err();
    assert_eq!(err, SourceError::Other("Failed to enable unit".into()));
}
