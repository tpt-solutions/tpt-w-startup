// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! XDG autostart backend: `~/.config/autostart` and `/etc/xdg/autostart`.
//!
//! Disable is reversible: it sets `Hidden=true` in the `.desktop` file rather
//! than removing it. Item ids are `xdg:<file name>`; each id is resolved by
//! re-listing the directory, so caller input never becomes a path.

use std::fs;
use std::io;
use std::path::PathBuf;
use tpt_startup_core::{
    BootImpact, SignatureStatus, SourceError, SourceKind, StartupItem, StartupSource,
};

const PREFIX: &str = "xdg:";
const GROUP: &str = "[Desktop Entry]";

pub struct XdgAutostartSource {
    dir: PathBuf,
}

impl XdgAutostartSource {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    /// User scope: `$XDG_CONFIG_HOME/autostart`, falling back to `~/.config/autostart`.
    pub fn user_default() -> Option<Self> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
        Some(Self::new(base.join("autostart")))
    }

    fn desktop_files(&self) -> Result<Vec<PathBuf>, SourceError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(e) => e,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(map_io(e)),
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "desktop") && p.is_file())
            .collect();
        files.sort();
        Ok(files)
    }

    fn resolve(&self, id: &str) -> Result<PathBuf, SourceError> {
        let name = id
            .strip_prefix(PREFIX)
            .ok_or_else(|| SourceError::NotFound(id.to_string()))?;
        self.desktop_files()?
            .into_iter()
            .find(|p| p.file_name().is_some_and(|n| n == name))
            .ok_or_else(|| SourceError::NotFound(id.to_string()))
    }
}

impl StartupSource for XdgAutostartSource {
    fn kind(&self) -> SourceKind {
        SourceKind::XdgAutostart
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let mut out = Vec::new();
        for path in self.desktop_files()? {
            let text = fs::read_to_string(&path).map_err(map_io)?;
            let fields = parse_desktop_entry(&text);
            let file_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let display = fields
                .get("Name")
                .cloned()
                .unwrap_or_else(|| file_name.clone());
            let id = format!("{PREFIX}{file_name}");
            let mut item = StartupItem::new(
                id,
                SourceKind::XdgAutostart,
                display,
                path.to_string_lossy(),
            );
            item.command = fields.get("Exec").cloned();
            item.enabled = !is_hidden(&fields)
                && fields
                    .get("X-GNOME-Autostart-enabled")
                    .is_none_or(|v| v != "false");
            item.reversible_disable = true;
            item.signature = SignatureStatus::Unsigned; // no signature concept on XDG entries
            item.boot_impact = BootImpact::Unknown; // no timing signal for autostart entries
            out.push(item);
        }
        Ok(out)
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let path = self.resolve(id)?;
        let text = fs::read_to_string(&path).map_err(map_io)?;
        let updated = set_hidden(&text, !enabled);
        fs::write(&path, updated).map_err(map_io)
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        let path = self.resolve(id)?;
        fs::remove_file(&path).map_err(map_io)
    }
}

/// Read key/value pairs from the `[Desktop Entry]` group only.
pub fn parse_desktop_entry(text: &str) -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    let mut in_group = false;
    for raw in text.lines() {
        let line = raw.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_group = line == GROUP;
            continue;
        }
        if !in_group || line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.entry(k.trim().to_string())
                .or_insert_with(|| v.trim().to_string());
        }
    }
    map
}

fn is_hidden(fields: &std::collections::HashMap<String, String>) -> bool {
    fields.get("Hidden").is_some_and(|v| v == "true")
}

/// Return `text` with the `Hidden` key in `[Desktop Entry]` set to `hidden`.
/// All other lines are preserved byte-for-byte. Adds the key if missing.
pub fn set_hidden(text: &str, hidden: bool) -> String {
    let value = if hidden { "true" } else { "false" };
    let mut out = Vec::new();
    let mut in_group = false;
    let mut replaced = false;
    let mut group_seen = false;

    for raw in text.lines() {
        let trimmed = raw.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            if in_group && !replaced {
                out.push(format!("Hidden={value}"));
                replaced = true;
            }
            in_group = trimmed == GROUP;
            group_seen |= in_group;
            out.push(raw.to_string());
            continue;
        }
        if in_group && !replaced {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim() == "Hidden" {
                    out.push(format!("Hidden={value}"));
                    replaced = true;
                    continue;
                }
            }
        }
        out.push(raw.to_string());
    }
    if in_group && !replaced {
        out.push(format!("Hidden={value}"));
    }
    if !group_seen {
        // No [Desktop Entry] group at all: add one so the file stays valid.
        out.insert(0, GROUP.to_string());
        out.insert(1, format!("Hidden={value}"));
    }
    let mut s = out.join("\n");
    s.push('\n');
    s
}

fn map_io(e: io::Error) -> SourceError {
    match e.kind() {
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        io::ErrorKind::NotFound => SourceError::NotFound(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}
