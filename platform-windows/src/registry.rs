// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Registry Run / RunOnce backend (Phase 2).
//!
//! Each value under a Run key is one startup entry. Disable is reversible: the
//! value is renamed to `<DISABLED_PREFIX><name>`, which Windows does not treat
//! as a startup command, and enable renames it back. Nothing is deleted unless
//! the user asks for delete.
//!
//! Item ids are `registry:<hive>\<subkey>\<value name>`. Every action
//! re-reads the key and checks the value exists, so ids cannot point anywhere
//! the enumeration did not return.
//!
//! HKLM writes need elevation. Those errors surface as
//! `SourceError::PermissionDenied` rather than being hidden.

use crate::command::{exe_from_command, expand_env};
use crate::signature;
use std::io;
use tpt_startup_core::{SignatureStatus, SourceError, SourceKind, StartupItem, StartupSource};
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE};
use winreg::RegKey;

/// Prefix marking a disabled Run value. Keep in sync with nothing else: only
/// this backend reads it.
pub const DISABLED_PREFIX: &str = "tpt-disabled.";

pub const RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const RUNONCE_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\RunOnce";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

impl Hive {
    fn label(self) -> &'static str {
        match self {
            Hive::CurrentUser => "HKCU",
            Hive::LocalMachine => "HKLM",
        }
    }

    fn root(self) -> RegKey {
        match self {
            Hive::CurrentUser => RegKey::predef(HKEY_CURRENT_USER),
            Hive::LocalMachine => RegKey::predef(HKEY_LOCAL_MACHINE),
        }
    }
}

pub struct RegistryRunSource {
    hive: Hive,
    subkey: String,
}

impl RegistryRunSource {
    /// Standard Run key for `hive`.
    pub fn run(hive: Hive) -> Self {
        Self {
            hive,
            subkey: RUN_SUBKEY.to_string(),
        }
    }

    /// Standard RunOnce key for `hive`. RunOnce values are removed by Windows
    /// after they run once, so disabling one is rarely useful, but it is shown
    /// so the user can see it.
    pub fn run_once(hive: Hive) -> Self {
        Self {
            hive,
            subkey: RUNONCE_SUBKEY.to_string(),
        }
    }

    /// Arbitrary key under `hive`. Used by tests to avoid touching real entries.
    pub fn with_subkey(hive: Hive, subkey: impl Into<String>) -> Self {
        Self {
            hive,
            subkey: subkey.into(),
        }
    }

    fn id_for(&self, value: &str) -> String {
        format!("registry:{}\\{}\\{}", self.hive.label(), self.subkey, value)
    }

    fn open(&self, write: bool) -> Result<RegKey, SourceError> {
        let flags = if write {
            KEY_READ | KEY_WRITE
        } else {
            KEY_READ
        };
        self.hive
            .root()
            .open_subkey_with_flags(&self.subkey, flags)
            .map_err(map_io)
    }

    /// Resolve an id back to the value name, checking it belongs to this source.
    fn resolve(&self, id: &str) -> Result<String, SourceError> {
        let prefix = self.id_for("");
        let name = id
            .strip_prefix(&prefix)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        let key = self.open(false)?;
        if key.get_raw_value(name).is_ok() {
            Ok(name.to_string())
        } else {
            Err(SourceError::NotFound(id.to_string()))
        }
    }
}

impl StartupSource for RegistryRunSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Registry
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let key = match self.open(false) {
            Ok(k) => k,
            // A missing Run key just means no entries.
            Err(SourceError::NotFound(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let mut out = Vec::new();
        for entry in key.enum_values() {
            let (name, _raw) = entry.map_err(map_io)?;
            let command: String = key.get_value(&name).unwrap_or_default();
            let (display, enabled) = match name.strip_prefix(DISABLED_PREFIX) {
                Some(stripped) => (stripped.to_string(), false),
                None => (name.clone(), true),
            };
            let mut item = StartupItem::new(
                self.id_for(&name),
                SourceKind::Registry,
                display.clone(),
                format!("{}\\{}", self.hive.label(), self.subkey),
            );
            item.command = if command.is_empty() {
                None
            } else {
                Some(command)
            };
            item.enabled = enabled;
            item.reversible_disable = true;
            // Verify the executable the command launches. Values with no
            // parseable path, or whose file is missing, stay Unknown.
            item.signature = item
                .command
                .as_deref()
                .map(expand_env)
                .and_then(|c| exe_from_command(&c))
                .map_or(SignatureStatus::Unknown, |exe| signature::check(&exe));
            out.push(item);
        }
        Ok(out)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let name = self.resolve(id)?;
        let is_disabled = name.starts_with(DISABLED_PREFIX);
        if enabled != is_disabled {
            return Ok(()); // already in the requested state
        }
        let key = self.open(true)?;
        let (old, new) = if enabled {
            let stripped = name
                .strip_prefix(DISABLED_PREFIX)
                .unwrap_or(&name)
                .to_string();
            (name.clone(), stripped)
        } else {
            (name.clone(), format!("{DISABLED_PREFIX}{name}"))
        };
        rename_value(&key, &old, &new)
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        let name = self.resolve(id)?;
        let key = self.open(true)?;
        key.delete_value(&name).map_err(map_io)
    }
}

/// Copy a value to a new name, then delete the old one. Registry has no rename.
fn rename_value(key: &RegKey, old: &str, new: &str) -> Result<(), SourceError> {
    let raw = key.get_raw_value(old).map_err(map_io)?;
    key.set_raw_value(new, &raw).map_err(map_io)?;
    key.delete_value(old).map_err(map_io)
}

fn map_io(e: io::Error) -> SourceError {
    match e.kind() {
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        io::ErrorKind::NotFound => SourceError::NotFound(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}
