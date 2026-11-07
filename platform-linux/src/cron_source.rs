// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! User crontab backend. Reads with `crontab -l` and writes with `crontab -`.
//! Item ids are `cron:user:<line>`. Every change re-reads the crontab and
//! checks the line is still an `@reboot` entry before editing it.

use crate::cron::{self, CronEditError};
use crate::runner::CommandRunner;
use std::io;
use tpt_startup_core::{SourceError, SourceKind, StartupItem, StartupSource};

const LABEL: &str = "user";

pub struct CronUserSource<R: CommandRunner> {
    runner: R,
}

impl<R: CommandRunner> CronUserSource<R> {
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Current crontab text. A user with no crontab yields an empty string.
    fn read(&self) -> Result<String, SourceError> {
        let out = self
            .runner
            .run("crontab", &["-l"], None)
            .map_err(map_spawn)?;
        if out.success {
            return Ok(out.stdout);
        }
        // Vixie cron reports "no crontab for <user>" on stderr with exit 1.
        if out.stderr.to_lowercase().contains("no crontab") {
            return Ok(String::new());
        }
        Err(SourceError::Other(out.stderr.trim().to_string()))
    }

    fn write(&self, text: &str) -> Result<(), SourceError> {
        let out = self
            .runner
            .run("crontab", &["-"], Some(text))
            .map_err(map_spawn)?;
        if out.success {
            Ok(())
        } else {
            Err(SourceError::Other(out.stderr.trim().to_string()))
        }
    }

    fn line_from_id(id: &str) -> Result<usize, SourceError> {
        let prefix = format!("cron:{LABEL}:");
        id.strip_prefix(&prefix)
            .and_then(|n| n.parse::<usize>().ok())
            .ok_or_else(|| SourceError::NotFound(id.to_string()))
    }

    fn map_edit(err: CronEditError) -> SourceError {
        SourceError::NotFound(err.to_string())
    }
}

impl<R: CommandRunner> StartupSource for CronUserSource<R> {
    fn kind(&self) -> SourceKind {
        SourceKind::Cron
    }

    fn enumerate(&self) -> Result<Vec<StartupItem>, SourceError> {
        let text = self.read()?;
        let entries = cron::parse_reboot_entries(&text, false);
        Ok(cron::to_startup_items(LABEL, &entries))
    }

    fn set_enabled(&mut self, id: &str, enabled: bool) -> Result<(), SourceError> {
        let line = Self::line_from_id(id)?;
        let text = self.read()?;
        let updated = cron::set_reboot_enabled(&text, line, enabled).map_err(Self::map_edit)?;
        self.write(&updated)
    }

    fn delete(&mut self, id: &str) -> Result<(), SourceError> {
        let line = Self::line_from_id(id)?;
        let text = self.read()?;
        let updated = cron::delete_reboot_line(&text, line).map_err(Self::map_edit)?;
        self.write(&updated)
    }
}

fn map_spawn(e: io::Error) -> SourceError {
    match e.kind() {
        io::ErrorKind::NotFound => SourceError::Unsupported("crontab is not installed".into()),
        io::ErrorKind::PermissionDenied => SourceError::PermissionDenied(e.to_string()),
        _ => SourceError::Other(e.to_string()),
    }
}
