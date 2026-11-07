# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses
[Semantic Versioning](docs/versioning.md).

## [Unreleased]

### Added

- Core model, `StartupSource` trait, categorization engine, boot-impact
  normalization, and audit-logged actions with a confirmation rule.
- Windows: Startup folder backend (reversible disable by moving to `disabled/`).
- Windows: registry Run and RunOnce backend, HKCU and HKLM (reversible disable
  by renaming the value).
- Windows: Authenticode signature checks, embedded and catalog signatures.
- Linux: XDG autostart backend (reversible disable via `Hidden=true`).
- Linux: systemd user-unit backend (enable and disable through `systemctl --user`).
- Linux: user crontab `@reboot` backend (reversible disable by comment marker).
- Linux: boot-impact estimates for systemd units from `systemd-analyze --user blame`.
- Tauri desktop shell with list, action and audit commands, and a Svelte
  dashboard with category tabs, search, a details panel, confirmation dialogs,
  a glossary and a first-run notice.
- Windows app manifest requesting `asInvoker`, so the app never prompts for
  elevation at launch.
- CI workflow for Windows and Ubuntu (fmt, clippy, tests).
- Documentation: README, user guide, elevation and scope, boot-impact decision,
  Archon design (design only), versioning process.

### Notes

- Windows boot impact is Unknown in this version.
- Installers are not built yet.
