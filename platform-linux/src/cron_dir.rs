// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! System cron directory backend (`/etc/cron.d`). Each file holds entries in
//! the system format, which has a user column before the command.
//!
//! Ids are `cron:system:<file>:<line>`. The file name is checked against
//! cron's own naming rule (letters, digits, `-` and `_` only), and it must
//! match a file in the directory listing, so ids cannot reach other paths.
//!
//! Changes need root. Without it, writes fail with `PermissionDenied`.

use crate::cron::{self, CronEditError};
use std::fs;
use std::io;
use std::path::PathBuf;
use tpt_startup_core::{SourceError, SourceKind, StartupItem, StartupSource};

const PREFIX: &str = "cron:system:";

pub struct CronDirSource {
    dir: PathBuf,
}

impl CronDirSource {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// `/etc/cron.d`.
    pub fn system_default() -> Self {
        Self::new("/etc/cron.d")
    }

    /// Files cron itself would read: no dot in the name, not a backup (`~`).
    fn files(&self) -> Result<Vec<String>, SourceError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(map_io(e)),
        };
        let mut names: Vec<String> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_file())
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| valid_file_name(n))
            .collect();
        names.sort();
        Ok(names)
    }

    /// Split an id into (file name, line) after checking both against the listing.
    fn resolve(&self, id: &str) -> Result<(String, usize), SourceError> {
        let not_found = || SourceError::NotFound(id.to_string());
        let rest = id.strip_prefix(PREFIX).ok_or_else(not_found)?;
        let (file, line) = rest.rsplit_once(':').ok_or_else(not_found)?;
        let line: usize = line.parse().map_err(|_| not_found())?;
        if !self.files()?.iter().any(|f| f == file) {
            return Err(not_found());
        }
        Ok((file.to_string(), line))
    }

    fn path_of(&self, file: &str) -> PathBuf {
        self.dir.join(file)
    }

    fn edit_file(
        &self,
        id: &str,
        edit: impl FnOnce(&str, usize) -> Result<String, CronEditError>,
    ) -> Result<(), SourceError> {
        let (file, line) = self.resolve(id)?;
        let path = self.path_of(&file);
        let text = fs::read_to_string(&path).map_err(map_io)?;
        let updated = edit(&text, line).map_err(|e| SourceError::NotFound(e.to_string()))?;
        fs::write(&path, updated).map_err(map_io)
    }
}

/// Cron's file-name rule for `/etc/cron.d`: letters, digits, `-` and `_`.
pub fn valid_file_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

impl StartupSource for CronDirSource {
    fn kind(&self) -> SourceKind {
        SourceKind::Cron
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let mut out = Vec::new();
        for file in self.files()? {
            let path = self.path_of(&file);
            let text = match fs::read_to_string(&path) {
                Ok(t) => t,
                // An unreadable file is skipped rather than hiding the others.
                Err(_) => continue,
            };
            let entries = cron::parse_reboot_entries(&text, true);
            for mut item in cron::to_startup_items(&format!("system:{file}"), &entries) {
                item.location = path.to_string_lossy().into_owned();
                out.push(item);
            }
        }
        Ok(out)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        self.edit_file(id, |text, line| {
            cron::set_reboot_enabled(text, line, enabled)
        })
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        self.edit_file(id, cron::delete_reboot_line)
    }
}

fn map_io(e: io::Error) -> SourceError {
    match e.kind() {
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        io::ErrorKind::NotFound => SourceError::NotFound(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_name_rule_matches_cron() {
        assert!(valid_file_name("backup-job_1"));
        assert!(!valid_file_name("with.dot"));
        assert!(!valid_file_name("backup~"));
        assert!(!valid_file_name("../etc"));
        assert!(!valid_file_name(""));
    }
}
