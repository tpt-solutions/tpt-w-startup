// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Linux startup backends: XDG autostart (file-based), user crontab
//! `@reboot` entries, and systemd user units. External programs go through
//! [`runner::CommandRunner`], so the logic is testable without systemd.

pub mod cron;
pub mod cron_dir;
pub mod cron_source;
pub mod runner;
pub mod systemd;
pub mod xdg;

pub use cron_dir::CronDirSource;
pub use cron_source::CronUserSource;
pub use runner::{CommandOutput, CommandRunner, SystemRunner};
pub use systemd::SystemdUserSource;
pub use xdg::XdgAutostartSource;
