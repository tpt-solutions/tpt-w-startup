// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Startup-folder backend (`shell:startup`, per-user or all-users).
//!
//! Disable is reversible: the entry is moved into a `disabled` subfolder of
//! the startup folder and moved back to re-enable. Item ids are
//! `startup:<enabled|disabled>/<file name>`; each id is resolved against a
//! fresh listing, so caller input never becomes a path.
//!
//! Limitations (todo.md Phase 2): `.lnk` targets are not parsed yet and
//! signature status is reported as `Unknown` until `WinVerifyTrust` is wired.

use std::fs;
use std::io;
use std::path::PathBuf;
use tpt_startup_core::{SignatureStatus, SourceError, SourceKind, StartupItem, StartupSource};

const PREFIX: &str = "startup:";
const DISABLED_DIR: &str = "disabled";

pub struct StartupFolderSource {
    dir: PathBuf,
}

impl StartupFolderSource {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// Per-user startup folder: `%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup`.
    pub fn user_default() -> Option<Self> {
        let appdata = std::env::var_os("APPDATA")?;
        Some(Self::new(
            PathBuf::from(appdata)
                .join("Microsoft")
                .join("Windows")
                .join("Start Menu")
                .join("Programs")
                .join("Startup"),
        ))
    }

    fn list(&self, enabled: bool) -> Result<Vec<PathBuf>, SourceError> {
        let dir = if enabled {
            self.dir.clone()
        } else {
            self.dir.join(DISABLED_DIR)
        };
        let entries = match fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(map_io(e)),
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            // Skip the disabled folder itself when listing the enabled folder.
            .collect();
        files.sort();
        Ok(files)
    }

    fn id_for(enabled: bool, file_name: &str) -> String {
        let state = if enabled { "enabled" } else { "disabled" };
        format!("{PREFIX}{state}/{file_name}")
    }

    /// Resolve an id to (current path, enabled state) by re-listing both folders.
    fn resolve(&self, id: &str) -> Result<(PathBuf, bool), SourceError> {
        let rest = id
            .strip_prefix(PREFIX)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        let (state, name) = rest
            .split_once('/')
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        let enabled = match state {
            "enabled" => true,
            "disabled" => false,
            _ => return Err(SourceError::NotFound(id.to_string())),
        };
        self.list(enabled)?
            .into_iter()
            .find(|p| p.file_name().is_some_and(|n| n == name))
            .map(|p| (p, enabled))
            .ok_or_else(|| SourceError::NotFound(id.to_string()))
    }
}

impl StartupSource for StartupFolderSource {
    fn kind(&self) -> SourceKind {
        SourceKind::StartupFolder
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let mut out = Vec::new();
        for (enabled, paths) in [(true, self.list(true)?), (false, self.list(false)?)] {
            for path in paths {
                let file_name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let display = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let mut item = StartupItem::new(
                    Self::id_for(enabled, &file_name),
                    SourceKind::StartupFolder,
                    display,
                    path.to_string_lossy(),
                );
                item.enabled = enabled;
                item.reversible_disable = true;
                item.signature = SignatureStatus::Unknown;
                out.push(item);
            }
        }
        Ok(out)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let (path, currently_enabled) = self.resolve(id)?;
        if currently_enabled == enabled {
            return Ok(());
        }
        let disabled_dir = self.dir.join(DISABLED_DIR);
        let target_dir = if enabled { &self.dir } else { &disabled_dir };
        fs::create_dir_all(target_dir).map_err(map_io)?;
        let target = target_dir.join(path.file_name().unwrap_or_default());
        fs::rename(&path, &target).map_err(map_io)
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        let (path, _) = self.resolve(id)?;
        fs::remove_file(&path).map_err(map_io)
    }
}

fn map_io(e: io::Error) -> SourceError {
    match e.kind() {
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        io::ErrorKind::NotFound => SourceError::NotFound(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}
