import type { BootImpact, Category, SourceKind } from './types';

export const CATEGORY_LABEL: Record<Category, string> = {
  safe: 'Safe',
  recommended: 'Recommended',
  critical: 'Critical',
};

export const IMPACT_LABEL: Record<BootImpact, string> = {
  high: 'High',
  medium: 'Medium',
  low: 'Low',
  unknown: 'Unknown',
};

export const SOURCE_LABEL: Record<SourceKind, string> = {
  registry: 'Registry Run key',
  'startup-folder': 'Startup folder',
  'task-scheduler': 'Scheduled task',
  'systemd-unit': 'systemd user unit',
  cron: 'Cron @reboot job',
  'xdg-autostart': 'Autostart (.desktop)',
  archon: 'Archon service',
};

/** Same rule as core::actions::requires_confirmation. Display only: the backend enforces it. */
export function needsConfirmation(category: Category, action: 'enable' | 'disable' | 'delete'): boolean {
  return category === 'critical' || action === 'delete';
}
