// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Re-export of the shared command runner. It lives in `tpt-startup-core`
//! so the Windows and Linux backends use the same seam.

pub use tpt_startup_core::runner::{CommandOutput, CommandRunner, SystemRunner};
