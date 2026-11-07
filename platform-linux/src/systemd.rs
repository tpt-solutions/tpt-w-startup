// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! systemd user-unit backend (`systemctl --user`).
//!
//! Which units are shown: units whose state is `enabled`, plus units that are
//! `disabled` but whose preset is `enabled`. The second group keeps a unit the
//! app turned off visible, so it can be turned back on. Units that are
//! `static`, `indirect`, `generated`, or `masked` are not startup entries and
//! are not shown.
//!
//! Disable and enable are reversible `systemctl --user` calls. Delete is not
//! supported: removing a unit file is outside the reversible-action model, so
//! the backend returns `Unsupported`.
//!
//! Boot impact comes from `systemd-analyze --user blame` when it is available.

use crate::runner::CommandRunner;
use std::collections::HashMap;
use tpt_startup_core::impact::{impact_from_millis, parse_systemd_blame};
use tpt_startup_core::{
    BootImpact, SignatureStatus, SourceError, SourceKind, StartupItem, StartupSource,
};

const PREFIX: &str = "systemd:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitFile {
    pub name: String,
    pub state: String,
    pub preset: Option<String>,
}

/// Parse `systemctl --user list-unit-files --no-legend --plain` output.
/// Columns are `UNIT FILE`, `STATE`, and optionally `PRESET`.
pub fn parse_unit_files(output: &str) -> Vec<UnitFile> {
    output
        .lines()
        .filter_map(|line| {
            let mut cols = line.split_whitespace();
            let name = cols.next()?.to_string();
            let state = cols.next()?.to_string();
            let preset = cols.next().map(str::to_string);
            Some(UnitFile {
                name,
                state,
                preset,
            })
        })
        .collect()
}

/// Whether a unit should appear as a startup entry.
pub fn is_startup_unit(u: &UnitFile) -> bool {
    match u.state.as_str() {
        "enabled" => true,
        "disabled" => u.preset.as_deref() == Some("enabled"),
        _ => false,
    }
}

/// Unit names are passed as arguments, never through a shell. Still, reject
/// anything outside the systemd unit-name alphabet before it reaches a command.
fn valid_unit_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | ':'))
}

pub struct SystemdUserSource<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> SystemdUserSource<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    fn systemctl(&self, args: &[&str]) -> Result<String, SourceError> {
        let mut full = vec!["--user"];
        full.extend_from_slice(args);
        let out = self
            .runner
            .run("systemctl", &full, None)
            .map_err(map_spawn)?;
        if out.success {
            Ok(out.stdout)
        } else {
            Err(SourceError::Other(out.stderr.trim().to_string()))
        }
    }

    /// Per-unit boot cost in milliseconds. Empty if the tool is unavailable;
    /// units without a timing then report `Unknown`.
    fn blame(&self) -> HashMap<String, u64> {
        match self
            .runner
            .run("systemd-analyze", &["--user", "blame"], None)
        {
            Ok(out) if out.success => parse_systemd_blame(&out.stdout),
            _ => HashMap::new(),
        }
    }

    fn unit_name_from_id(id: &str) -> Result<&str, SourceError> {
        let name = id
            .strip_prefix(PREFIX)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        if valid_unit_name(name) {
            Ok(name)
        } else {
            Err(SourceError::NotFound(id.to_string()))
        }
    }
}

impl<R: CommandRunner> StartupSource for SystemdUserSource<R> {
    fn kind(&self) -> SourceKind {
        SourceKind::SystemdUnit
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let listing = self.systemctl(&[
            "list-unit-files",
            "--type=service",
            "--no-legend",
            "--no-pager",
            "--plain",
        ])?;
        let blame = self.blame();
        let mut out = Vec::new();
        for unit in parse_unit_files(&listing)
            .into_iter()
            .filter(is_startup_unit)
        {
            let display = unit.name.trim_end_matches(".service").to_string();
            let mut item = StartupItem::new(
                format!("{PREFIX}{}", unit.name),
                SourceKind::SystemdUnit,
                display,
                format!("systemctl --user {}", unit.name),
            );
            item.enabled = unit.state == "enabled";
            item.reversible_disable = true;
            item.signature = SignatureStatus::Unknown;
            item.boot_impact = match blame.get(&unit.name) {
                Some(ms) => impact_from_millis(*ms),
                None => BootImpact::Unknown,
            };
            out.push(item);
        }
        Ok(out)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let name = Self::unit_name_from_id(id)?.to_string();
        // Re-check against the listing: only units the app would show may change.
        let listing = self.systemctl(&[
            "list-unit-files",
            "--type=service",
            "--no-legend",
            "--no-pager",
            "--plain",
        ])?;
        let known = parse_unit_files(&listing)
            .into_iter()
            .filter(is_startup_unit)
            .any(|u| u.name == name);
        if !known {
            return Err(SourceError::NotFound(id.to_string()));
        }
        let verb = if enabled { "enable" } else { "disable" };
        self.systemctl(&[verb, &name]).map(|_| ())
    }

    fn delete(&mut self, _id: &str) -> Result<(), SourceError> {
        Err(SourceError::Unsupported(
            "systemd units are disabled, not deleted; remove the unit file manually".into(),
        ))
    }
}

fn map_spawn(e: std::io::Error) -> SourceError {
    match e.kind() {
        std::io::ErrorKind::NotFound => {
            SourceError::Unsupported("systemctl is not available (no systemd?)".into())
        }
        std::io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}
