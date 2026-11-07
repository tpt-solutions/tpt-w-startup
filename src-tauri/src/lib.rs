// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Tauri command layer (Phase 7). All logic lives in [`service`]; these
//! functions only translate between IPC and the service.

pub mod service;

use service::{BoxedSource, ScanResult, Service};
use std::sync::Mutex;
use tauri::State;
use tpt_startup_core::{Action, AuditEntry};

type AppState = Mutex<Service>;

/// Backends for the current platform, user scope only.
/// System-wide sources (`/etc/xdg/autostart`, HKLM, all-users Startup) are
/// read-only in v1 and need elevation to change; they are not wired yet.
fn default_sources() -> Vec<BoxedSource> {
    let mut sources: Vec<BoxedSource> = Vec::new();
    #[cfg(target_os = "windows")]
    if let Some(src) = tpt_startup_windows::StartupFolderSource::user_default() {
        sources.push(Box::new(src));
    }
    #[cfg(target_os = "windows")]
    {
        use tpt_startup_windows::{Hive, RegistryRunSource};
        // HKLM sources are listed for visibility; their writes need elevation
        // and report PermissionDenied when the app is not elevated.
        for hive in [Hive::CurrentUser, Hive::LocalMachine] {
            sources.push(Box::new(RegistryRunSource::run(hive)));
            sources.push(Box::new(RegistryRunSource::run_once(hive)));
        }
    }
    #[cfg(target_os = "linux")]
    {
        use tpt_startup_linux::{CronUserSource, SystemRunner, SystemdUserSource};
        if let Some(src) = tpt_startup_linux::XdgAutostartSource::user_default() {
            sources.push(Box::new(src));
        }
        // Missing systemctl or crontab shows up as a per-source warning.
        sources.push(Box::new(SystemdUserSource::new(SystemRunner)));
        sources.push(Box::new(CronUserSource::new(SystemRunner)));
    }
    sources
}

#[tauri::command]
fn list_items(state: State<'_, AppState>) -> Result<ScanResult, String> {
    let svc = state
        .lock()
        .map_err(|_| "internal state poisoned".to_string())?;
    Ok(svc.scan())
}

#[tauri::command]
fn apply_action(
    state: State<'_, AppState>,
    id: String,
    action: Action,
    confirmed: bool,
) -> Result<(), String> {
    let mut svc = state
        .lock()
        .map_err(|_| "internal state poisoned".to_string())?;
    svc.apply(&id, action, confirmed).map_err(|e| e.to_string())
}

#[tauri::command]
fn audit_log(state: State<'_, AppState>) -> Result<Vec<AuditEntry>, String> {
    let svc = state
        .lock()
        .map_err(|_| "internal state poisoned".to_string())?;
    Ok(svc.audit_entries())
}

pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(Service::new(default_sources())))
        .invoke_handler(tauri::generate_handler![
            list_items,
            apply_action,
            audit_log
        ])
        .run(tauri::generate_context!())
        .expect("error while running tpt-startup");
}
