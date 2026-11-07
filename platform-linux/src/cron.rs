// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Cron `@reboot` entries: parsing and line-level edits.
//!
//! Disable is reversible: the line is prefixed with [`DISABLED_MARKER`] and
//! stays in place. Enable strips the marker. Edits are pure text transforms;
//! [`crate::cron_source`] is what reads and writes the real crontab.

use tpt_startup_core::{BootImpact, SignatureStatus, SourceKind, StartupItem};

/// Comment prefix that marks a disabled `@reboot` line.
pub const DISABLED_MARKER: &str = "#tpt-disabled ";

/// One `@reboot` line from a crontab or cron.d file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebootEntry {
    /// 1-based line number in the source text, used to build a stable id.
    pub line: usize,
    /// Command to run. For `/etc/cron.d` lines the user field is removed.
    pub command: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CronEditError {
    /// The line number is out of range or does not hold an `@reboot` entry.
    NotAnEntry(usize),
}

impl std::fmt::Display for CronEditError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CronEditError::NotAnEntry(n) => write!(f, "line {n} is not an @reboot entry"),
        }
    }
}

impl std::error::Error for CronEditError {}

/// Split a line into (enabled, body) where body starts at `@reboot`.
fn classify_line(raw: &str) -> Option<(bool, &str)> {
    let line = raw.trim();
    let (enabled, body) = match line.strip_prefix(DISABLED_MARKER) {
        Some(rest) => (false, rest.trim_start()),
        None if line.starts_with('#') => return None,
        None => (true, line),
    };
    let rest = body.strip_prefix("@reboot")?;
    if !rest.starts_with(char::is_whitespace) {
        return None; // e.g. "@rebooted" is not a match
    }
    Some((enabled, body))
}

/// Parse `@reboot` lines, enabled and disabled. Set `system_format` to true
/// for `/etc/cron.d` files, which carry a user field between the directive and
/// the command. Ordinary comments and other schedules are ignored.
pub fn parse_reboot_entries(text: &str, system_format: bool) -> Vec<RebootEntry> {
    text.lines()
        .enumerate()
        .filter_map(|(idx, raw)| {
            let (enabled, body) = classify_line(raw)?;
            let mut rest = body.strip_prefix("@reboot")?.trim_start();
            if system_format {
                let (_user, after) = rest.split_once(char::is_whitespace)?;
                rest = after.trim_start();
            }
            if rest.is_empty() {
                return None;
            }
            Some(RebootEntry {
                line: idx + 1,
                command: rest.to_string(),
                enabled,
            })
        })
        .collect()
}

/// Return `text` with line `line` (1-based) enabled or disabled.
pub fn set_reboot_enabled(text: &str, line: usize, enabled: bool) -> Result<String, CronEditError> {
    edit_line(text, line, |body| {
        if enabled {
            body.to_string()
        } else {
            format!("{DISABLED_MARKER}{body}")
        }
    })
}

/// Return `text` with the `@reboot` line `line` (1-based) removed.
pub fn delete_reboot_line(text: &str, line: usize) -> Result<String, CronEditError> {
    let mut out = Vec::new();
    let mut found = false;
    for (idx, raw) in text.lines().enumerate() {
        if idx + 1 == line {
            if classify_line(raw).is_none() {
                return Err(CronEditError::NotAnEntry(line));
            }
            found = true;
            continue;
        }
        out.push(raw);
    }
    if !found {
        return Err(CronEditError::NotAnEntry(line));
    }
    Ok(join_lines(&out))
}

fn edit_line(text: &str, line: usize, f: impl Fn(&str) -> String) -> Result<String, CronEditError> {
    let mut out = Vec::new();
    let mut found = false;
    for (idx, raw) in text.lines().enumerate() {
        if idx + 1 == line {
            let (_, body) = classify_line(raw).ok_or(CronEditError::NotAnEntry(line))?;
            out.push(f(body));
            found = true;
        } else {
            out.push(raw.to_string());
        }
    }
    if !found {
        return Err(CronEditError::NotAnEntry(line));
    }
    let refs: Vec<&str> = out.iter().map(String::as_str).collect();
    Ok(join_lines(&refs))
}

fn join_lines(lines: &[&str]) -> String {
    let mut s = lines.join("\n");
    s.push('\n');
    s
}

/// Convert parsed entries to startup items for a given crontab source label
/// (e.g. `"user"` or `"/etc/cron.d/foo"`). Boot impact is `Unknown`: cron has
/// no timing signal.
pub fn to_startup_items(source_label: &str, entries: &[RebootEntry]) -> Vec<StartupItem> {
    entries
        .iter()
        .map(|e| {
            let id = format!("cron:{source_label}:{}", e.line);
            let name = e
                .command
                .split_whitespace()
                .next()
                .unwrap_or("cron job")
                .rsplit('/')
                .next()
                .unwrap_or("cron job")
                .to_string();
            let mut item = StartupItem::new(id, SourceKind::Cron, name, source_label);
            item.command = Some(e.command.clone());
            item.enabled = e.enabled;
            item.reversible_disable = true;
            item.signature = SignatureStatus::Unknown;
            item.boot_impact = BootImpact::Unknown;
            item
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
# comment
@reboot /usr/bin/foo --bar
0 3 * * * /usr/bin/nightly
@reboot   sleep 10 && /opt/bar
@rebooted /nope
@reboot
";

    #[test]
    fn user_crontab_reboot_lines() {
        let entries = parse_reboot_entries(SAMPLE, false);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].line, 2);
        assert_eq!(entries[0].command, "/usr/bin/foo --bar");
        assert!(entries[0].enabled);
        assert_eq!(entries[1].command, "sleep 10 && /opt/bar");
    }

    #[test]
    fn system_cron_d_drops_user_field() {
        let entries = parse_reboot_entries("@reboot root /usr/local/bin/startup-thing\n", true);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].command, "/usr/local/bin/startup-thing");
    }

    #[test]
    fn disabled_lines_are_recognised() {
        let text = format!("{DISABLED_MARKER}@reboot /usr/bin/foo\n");
        let entries = parse_reboot_entries(&text, false);
        assert_eq!(entries.len(), 1);
        assert!(!entries[0].enabled);
        assert_eq!(entries[0].command, "/usr/bin/foo");
    }

    #[test]
    fn disable_then_enable_round_trips_text() {
        let original = "0 1 * * * a\n@reboot /usr/bin/foo\n";
        let disabled = set_reboot_enabled(original, 2, false).unwrap();
        assert_eq!(
            disabled,
            "0 1 * * * a\n#tpt-disabled @reboot /usr/bin/foo\n"
        );
        let restored = set_reboot_enabled(&disabled, 2, true).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn edit_rejects_lines_that_are_not_entries() {
        assert_eq!(
            set_reboot_enabled(SAMPLE, 3, false),
            Err(CronEditError::NotAnEntry(3))
        );
        assert_eq!(
            set_reboot_enabled(SAMPLE, 99, false),
            Err(CronEditError::NotAnEntry(99))
        );
    }

    #[test]
    fn delete_removes_only_that_line() {
        let out = delete_reboot_line(SAMPLE, 2).unwrap();
        assert!(!out.contains("/usr/bin/foo"));
        assert!(out.contains("0 3 * * * /usr/bin/nightly"));
        assert!(out.contains("sleep 10"));
        assert_eq!(
            delete_reboot_line(SAMPLE, 3),
            Err(CronEditError::NotAnEntry(3))
        );
    }

    #[test]
    fn items_have_unknown_impact_name_and_state() {
        let text = format!("{DISABLED_MARKER}@reboot /usr/bin/foo --x\n");
        let items = to_startup_items("user", &parse_reboot_entries(&text, false));
        assert_eq!(items[0].display_name, "foo");
        assert!(!items[0].enabled);
        assert_eq!(items[0].boot_impact, BootImpact::Unknown);
        assert_eq!(items[0].id, "cron:user:1");
    }
}
