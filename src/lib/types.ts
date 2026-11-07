// Mirrors the serde output of tpt-startup-core (kebab-case / lowercase enums,
// snake_case fields). Keep in sync with core/src/model.rs and actions.rs.

export type SourceKind =
  | 'registry'
  | 'startup-folder'
  | 'task-scheduler'
  | 'systemd-unit'
  | 'cron'
  | 'xdg-autostart'
  | 'archon';

export type Category = 'safe' | 'recommended' | 'critical';
export type BootImpact = 'high' | 'medium' | 'low' | 'unknown';
export type SignatureStatus = 'valid' | 'invalid' | 'unsigned' | 'unknown';
export type Action = 'enable' | 'disable' | 'delete';

export interface StartupItem {
  id: string;
  source: SourceKind;
  display_name: string;
  command: string | null;
  location: string;
  publisher: string | null;
  signature: SignatureStatus;
  enabled: boolean;
  reversible_disable: boolean;
  category: Category;
  explanation: string;
  boot_impact: BootImpact;
}

export interface ScanResult {
  items: StartupItem[];
  warnings: string[];
}

export interface AuditEntry {
  item_id: string;
  display_name: string;
  action: Action;
  timestamp: number;
  error: string | null;
}
