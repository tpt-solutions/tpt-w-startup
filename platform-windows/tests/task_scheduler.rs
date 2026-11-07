// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Task Scheduler backend driven by a fake `schtasks`. Runs on any OS and
//! never changes a real task.

use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use tpt_startup_core::runner::{CommandOutput, CommandRunner};
use tpt_startup_core::{SourceError, StartupSource};
use tpt_startup_windows::TaskSchedulerSource;

const XML: &str = r#"<?xml version="1.0"?>
<Task><RegistrationInfo><URI>\Acme\Sync</URI></RegistrationInfo>
<Triggers><LogonTrigger /></Triggers>
<Settings><Enabled>true</Enabled></Settings>
<Actions><Exec><Command>sync.exe</Command></Exec></Actions></Task>
<?xml version="1.0"?>
<Task><RegistrationInfo><URI>\Nightly\Backup</URI></RegistrationInfo>
<Triggers><CalendarTrigger /></Triggers>
<Actions><Exec><Command>backup.exe</Command></Exec></Actions></Task>
<?xml version="1.0"?>
<Task><RegistrationInfo><URI>\Microsoft\Windows\Locked</URI></RegistrationInfo>
<Triggers><BootTrigger /></Triggers>
<Actions><Exec><Command>locked.exe</Command></Exec></Actions></Task>
"#;

#[derive(Clone, Default)]
struct FakeSchtasks {
    calls: Rc<RefCell<Vec<Vec<String>>>>,
    /// Stderr returned for change commands on this task path, if set.
    deny_path: Rc<RefCell<Option<String>>>,
}

impl CommandRunner for FakeSchtasks {
    fn run(&self, program: &str, args: &[&str], _stdin: Option<&str>) -> io::Result<CommandOutput> {
        assert_eq!(program, "schtasks");
        self.calls
            .borrow_mut()
            .push(args.iter().map(|s| s.to_string()).collect());
        if args == ["/query", "/xml"] {
            return Ok(CommandOutput {
                success: true,
                stdout: XML.into(),
                stderr: String::new(),
            });
        }
        if let Some(denied) = self.deny_path.borrow().as_ref() {
            if args.contains(&denied.as_str()) {
                return Ok(CommandOutput {
                    success: false,
                    stdout: String::new(),
                    stderr: "ERROR: Access is denied.".into(),
                });
            }
        }
        Ok(CommandOutput {
            success: true,
            stdout: String::new(),
            stderr: String::new(),
        })
    }
}

#[test]
fn lists_only_logon_and_boot_tasks() {
    let src = TaskSchedulerSource::new(FakeSchtasks::default());
    let items = src.enumerate().unwrap();
    let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
    assert_eq!(
        ids,
        vec![r"task:\Acme\Sync", r"task:\Microsoft\Windows\Locked"]
    );
    assert_eq!(items[0].command.as_deref(), Some("sync.exe"));
    assert!(items[0].reversible_disable);
}

#[test]
fn disable_and_enable_use_schtasks_change() {
    let fake = FakeSchtasks::default();
    let mut src = TaskSchedulerSource::new(fake.clone());
    src.set_enabled(r"task:\Acme\Sync", false).unwrap();
    src.set_enabled(r"task:\Acme\Sync", true).unwrap();
    let calls = fake.calls.borrow();
    assert!(calls.contains(&vec![
        "/change".to_string(),
        "/tn".into(),
        r"\Acme\Sync".into(),
        "/disable".into()
    ]));
    assert!(calls.contains(&vec![
        "/change".to_string(),
        "/tn".into(),
        r"\Acme\Sync".into(),
        "/enable".into()
    ]));
}

#[test]
fn delete_uses_force_flag() {
    let fake = FakeSchtasks::default();
    let mut src = TaskSchedulerSource::new(fake.clone());
    src.delete(r"task:\Acme\Sync").unwrap();
    assert!(fake.calls.borrow().contains(&vec![
        "/delete".to_string(),
        "/tn".into(),
        r"\Acme\Sync".into(),
        "/f".into()
    ]));
}

#[test]
fn ids_for_non_startup_tasks_are_rejected() {
    let fake = FakeSchtasks::default();
    let mut src = TaskSchedulerSource::new(fake.clone());
    assert!(matches!(
        src.set_enabled(r"task:\Nightly\Backup", false),
        Err(SourceError::NotFound(_))
    ));
    assert!(matches!(
        src.delete(r"task:\Acme\Sync; del *"),
        Err(SourceError::NotFound(_))
    ));
    assert!(!fake
        .calls
        .borrow()
        .iter()
        .any(|c| c.first().map(String::as_str) == Some("/change")
            || c.first().map(String::as_str) == Some("/delete")));
}

#[test]
fn access_denied_on_system_task_is_permission_denied() {
    let fake = FakeSchtasks::default();
    *fake.deny_path.borrow_mut() = Some(r"\Microsoft\Windows\Locked".into());
    let mut src = TaskSchedulerSource::new(fake);
    let err = src
        .set_enabled(r"task:\Microsoft\Windows\Locked", false)
        .unwrap_err();
    assert!(matches!(err, SourceError::PermissionDenied(_)));
}

#[test]
fn missing_schtasks_is_unsupported() {
    struct NoProgram;
    impl CommandRunner for NoProgram {
        fn run(&self, _: &str, _: &[&str], _: Option<&str>) -> io::Result<CommandOutput> {
            Err(io::Error::new(io::ErrorKind::NotFound, "not found"))
        }
    }
    let src = TaskSchedulerSource::new(NoProgram);
    assert!(matches!(src.enumerate(), Err(SourceError::Unsupported(_))));
}

/// Read-only check against the real Task Scheduler on Windows. It never changes
/// a task; it only confirms the live XML parses into tasks.
#[cfg(windows)]
#[test]
fn live_query_parses() {
    use tpt_startup_core::runner::SystemRunner;
    let src = TaskSchedulerSource::new(SystemRunner);
    // Any real Windows install has at least one task with a logon or boot trigger.
    assert!(!src.enumerate().unwrap().is_empty());
}
