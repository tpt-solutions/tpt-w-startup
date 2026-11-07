import { invoke } from '@tauri-apps/api/core';
import type { Action, AuditEntry, ScanResult } from './types';

// Thin typed wrappers over the Rust commands in src-tauri/src/lib.rs.
// Errors arrive as plain strings from the backend.

export function listItems(): Promise<ScanResult> {
  return invoke<ScanResult>('list_items');
}

export function applyAction(id: string, action: Action, confirmed: boolean): Promise<void> {
  return invoke<void>('apply_action', { id, action, confirmed });
}

export function auditLog(): Promise<AuditEntry[]> {
  return invoke<AuditEntry[]>('audit_log');
}
