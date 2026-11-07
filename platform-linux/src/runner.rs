// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Seam between the Linux sources and external programs (`systemctl`,
//! `systemd-analyze`, `crontab`). Production code uses [`SystemRunner`];
//! tests substitute a fake so parsing and state changes run on any OS.
//!
//! Programs are always invoked with an argument vector, never through a shell,
//! so unit names and file contents cannot inject commands.

use std::io::{self, Write};
use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub trait CommandRunner {
    /// Run `program` with `args`. If `stdin` is given it is written to the
    /// child's standard input. `Err` means the program could not be started.
    fn run(&self, program: &str, args: &[&str], stdin: Option<&str>) -> io::Result<CommandOutput>;
}

/// Runs real programs with `std::process::Command`.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[&str], stdin: Option<&str>) -> io::Result<CommandOutput> {
        let mut cmd = Command::new(program);
        cmd.args(args)
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = cmd.spawn()?;
        if let Some(input) = stdin {
            if let Some(mut pipe) = child.stdin.take() {
                pipe.write_all(input.as_bytes())?;
            }
        }
        let out = child.wait_with_output()?;
        Ok(CommandOutput {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        })
    }
}
