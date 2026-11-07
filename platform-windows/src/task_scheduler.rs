// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Task Scheduler backend (Phase 2). Only tasks with a logon or boot trigger
//! are startup entries; everything else is out of scope.
//!
//! Reads use one `schtasks /query /xml` call, which prints every task as a
//! separate XML document. Changes use `schtasks /change` (enable/disable,
//! reversible) and `schtasks /delete`. Task paths are passed as arguments, not
//! through a shell.
//!
//! Tasks under `\Microsoft\Windows\` and other system folders usually need
//! administrator rights to change; those failures are reported as
//! `PermissionDenied`.

use tpt_startup_core::runner::CommandRunner;
use tpt_startup_core::{
    BootImpact, SignatureStatus, SourceError, SourceKind, StartupItem, StartupSource,
};

const PREFIX: &str = "task:";

/// One parsed task document, reduced to what the backend needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskInfo {
    /// Full task path, e.g. `\Folder\Name`.
    pub path: String,
    pub enabled: bool,
    pub has_logon_or_boot_trigger: bool,
    pub command: Option<String>,
    pub arguments: Option<String>,
}

/// Text between the first `open` and the following `close`, if both exist.
fn between<'a>(haystack: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let start = haystack.find(open)? + open.len();
    let end = haystack[start..].find(close)? + start;
    Some(&haystack[start..end])
}

/// Decode the XML entities `schtasks` emits in text content.
fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

fn text_of(doc: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    between(doc, &open, &close)
        .map(|s| unescape(s.trim()))
        .filter(|s| !s.is_empty())
}

/// Parse the output of `schtasks /query /xml`. Documents without a URI
/// (which cannot be addressed) are skipped.
pub fn parse_tasks_xml(xml: &str) -> Vec<TaskInfo> {
    // Each task starts with its own XML declaration.
    xml.split("<?xml")
        .skip(1)
        .filter_map(|doc| {
            let path = text_of(doc, "URI")?;
            let triggers = between(doc, "<Triggers>", "</Triggers>").unwrap_or("");
            let has_logon_or_boot_trigger =
                triggers.contains("<LogonTrigger") || triggers.contains("<BootTrigger");
            let settings = between(doc, "<Settings>", "</Settings>").unwrap_or("");
            let enabled = !settings.contains("<Enabled>false</Enabled>");
            let exec = between(doc, "<Exec>", "</Exec>").unwrap_or("");
            Some(TaskInfo {
                path,
                enabled,
                has_logon_or_boot_trigger,
                command: text_of(exec, "Command"),
                arguments: text_of(exec, "Arguments"),
            })
        })
        .collect()
}

pub struct TaskSchedulerSource<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> TaskSchedulerSource<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn startup_tasks(&self) -> Result<Vec<TaskInfo>, SourceError> {
        let out = self
            .runner
            .run("schtasks", &["/query", "/xml"], None)
            .map_err(map_spawn)?;
        if !out.success {
            return Err(map_failure(&out.stderr));
        }
        Ok(parse_tasks_xml(&out.stdout)
            .into_iter()
            .filter(|t| t.has_logon_or_boot_trigger)
            .collect())
    }

    /// Resolve an id to its task path, only if the task is a startup task now.
    fn resolve(&self, id: &str) -> Result<String, SourceError> {
        let path = id
            .strip_prefix(PREFIX)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        self.startup_tasks()?
            .into_iter()
            .find(|t| t.path == path)
            .map(|t| t.path)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))
    }

    fn run_change(&self, args: &[&str]) -> Result<(), SourceError> {
        let out = self.runner.run("schtasks", args, None).map_err(map_spawn)?;
        if out.success {
            Ok(())
        } else {
            Err(map_failure(&out.stderr))
        }
    }
}

impl<R: CommandRunner> StartupSource for TaskSchedulerSource<R> {
    fn kind(&self) -> SourceKind {
        SourceKind::TaskScheduler
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let tasks = self.startup_tasks()?;
        Ok(tasks
            .into_iter()
            .map(|t| {
                let name = t.path.rsplit('\\').next().unwrap_or(&t.path).to_string();
                let location = t.path.clone();
                let mut item = StartupItem::new(
                    format!("{PREFIX}{}", t.path),
                    SourceKind::TaskScheduler,
                    name,
                    location,
                );
                item.command = match (t.command, t.arguments) {
                    (Some(c), Some(a)) => Some(format!("{c} {a}")),
                    (Some(c), None) => Some(c),
                    _ => None,
                };
                item.enabled = t.enabled;
                item.reversible_disable = true;
                item.signature = SignatureStatus::Unknown;
                item.boot_impact = BootImpact::Unknown;
                item
            })
            .collect())
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let path = self.resolve(id)?;
        let flag = if enabled { "/enable" } else { "/disable" };
        self.run_change(&["/change", "/tn", &path, flag])
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        let path = self.resolve(id)?;
        self.run_change(&["/delete", "/tn", &path, "/f"])
    }
}

/// Map `schtasks` stderr to a source error. The text is English-only on
/// English Windows; other languages fall back to `Other` with the message.
fn map_failure(stderr: &str) -> SourceError {
    let lower = stderr.to_lowercase();
    let msg = stderr.trim().to_string();
    if lower.contains("access is denied") {
        SourceError::PermissionDenied(msg)
    } else if lower.contains("cannot find") || lower.contains("does not exist") {
        SourceError::NotFound(msg)
    } else {
        SourceError::Other(msg)
    }
}

fn map_spawn(e: std::io::Error) -> SourceError {
    match e.kind() {
        std::io::ErrorKind::NotFound => {
            SourceError::Unsupported("schtasks is not available".into())
        }
        _ => SourceError::Other(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2">
  <RegistrationInfo>
    <URI>\Acme\Sync &amp; Go</URI>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
    </LogonTrigger>
  </Triggers>
  <Settings>
    <Enabled>false</Enabled>
  </Settings>
  <Actions>
    <Exec>
      <Command>C:\Program Files\Acme\sync.exe</Command>
      <Arguments>--quiet &quot;x&quot;</Arguments>
    </Exec>
  </Actions>
</Task>
<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2">
  <RegistrationInfo>
    <URI>\Nightly\Backup</URI>
  </RegistrationInfo>
  <Triggers>
    <CalendarTrigger />
  </Triggers>
  <Actions>
    <Exec>
      <Command>backup.exe</Command>
    </Exec>
  </Actions>
</Task>
<?xml version="1.0"?>
<Task>
  <Triggers>
    <BootTrigger />
  </Triggers>
</Task>
"#;

    #[test]
    fn parses_uri_triggers_state_and_command() {
        let tasks = parse_tasks_xml(DOC);
        assert_eq!(tasks.len(), 2, "document without URI is skipped");
        let sync = &tasks[0];
        assert_eq!(sync.path, r"\Acme\Sync & Go");
        assert!(sync.has_logon_or_boot_trigger);
        assert!(!sync.enabled);
        assert_eq!(
            sync.command.as_deref(),
            Some(r"C:\Program Files\Acme\sync.exe")
        );
        assert_eq!(sync.arguments.as_deref(), Some(r#"--quiet "x""#));
    }

    #[test]
    fn calendar_only_task_is_not_a_startup_task() {
        let tasks = parse_tasks_xml(DOC);
        let backup = &tasks[1];
        assert!(!backup.has_logon_or_boot_trigger);
        assert!(backup.enabled);
    }

    #[test]
    fn empty_output_yields_no_tasks() {
        assert!(parse_tasks_xml("").is_empty());
    }

    #[test]
    fn failure_text_maps_to_error_kinds() {
        assert!(matches!(
            map_failure("ERROR: Access is denied."),
            SourceError::PermissionDenied(_)
        ));
        assert!(matches!(
            map_failure("ERROR: The system cannot find the file specified."),
            SourceError::NotFound(_)
        ));
        assert!(matches!(
            map_failure("something else"),
            SourceError::Other(_)
        ));
    }
}
