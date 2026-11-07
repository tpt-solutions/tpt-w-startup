// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Windows startup backends: Startup folder (file-based, compiles everywhere),
//! and on Windows only the registry Run / RunOnce keys and Authenticode
//! verification. Task Scheduler is tracked in todo.md (Phase 2).

pub mod command;
pub mod startup_folder;

#[cfg(windows)]
pub mod registry;
#[cfg(windows)]
pub mod signature;

pub use startup_folder::StartupFolderSource;

#[cfg(windows)]
pub use registry::{Hive, RegistryRunSource};
