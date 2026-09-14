# tpt-startup — Project Checklist

TPT Solutions — dual-licensed under MIT OR Apache-2.0.
Smart Startup & Background Service Manager (Autoruns/Task Manager-Startup-tab
alternative). Rust core (registry/scheduler/systemd/cron scanning,
categorization + boot-impact engine) + Tauri UI (Svelte/Vite frontend,
categorized list dashboard). Targets Windows and Linux for v1; Archon
(lazy-loaded, capability-provisioned startup entries) is design-only until
`tpt-archon-kernel` exposes a public capability/telemetry API.

Reference: Archon platform — https://github.com/tpt-solutions/tpt-archon

Decisions locked in for this roadmap:
- Scope is **startup items, not services**: registry Run/RunOnce keys,
  Startup folder shortcuts, Task Scheduler tasks with logon/boot triggers,
  systemd user units, cron `@reboot` entries, and Linux XDG autostart
  `.desktop` files. No Windows Services (services.msc) or systemd
  system-level daemons in v1 — that's Task Manager's/systemctl's own turf —
  backlog.
- UI is **Rust core + Tauri** (Rust `src-tauri/` + Svelte/Vite frontend),
  matching tpt-monitor's stack, but as a **categorized list/tabs dashboard**
  (Safe / Recommended / Critical), not a rearrangeable widget grid — no
  gridstack.js needed here.
- **Actions are enable/disable/delete**, not view-only: v1 lets users toggle
  or remove any item, preferring a reversible disable (rename registry
  value, `.desktop` `Hidden=true`, `systemctl --user disable`) over deletion
  where the OS allows it, with a required confirmation step for anything
  flagged Critical.
- **Boot/login impact estimate is in v1** (High/Med/Low), shown alongside
  the category — Windows and Linux use different underlying signals (see
  Phase 5), normalized to the same three-tier scale.
- **Categorization is curated-list + heuristic fallback**: a bundled
  database of known publishers/apps maps to category + plain-English
  explanation; unrecognized items fall back to a heuristic (signature
  validity, install path, known-risky patterns) — enforced in the core
  engine, not just the UI, so it can't be bypassed.
- **Archon support is design-only** in v1: document the capability model for
  lazy-loaded, zero-idle-CPU startup entries; no implementation until
  `tpt-archon` exposes the relevant APIs.

---

## Status Summary

| Phase | Area | Progress |
|-------|------|----------|
| 0  | Project Setup & Licensing                    | ❌ Not Started |
| 1  | Core Data Model                              | ❌ Not Started |
| 2  | Windows Backend                              | ❌ Not Started |
| 3  | Linux Backend                                | ❌ Not Started |
| 4  | Categorization & Explanation Engine          | ❌ Not Started |
| 5  | Boot Impact Estimation                       | ❌ Not Started |
| 6  | Item Actions (Enable/Disable/Delete)         | ❌ Not Started |
| 7  | Tauri Shell & IPC Layer                       | ❌ Not Started |
| 8  | Frontend Dashboard                            | ❌ Not Started |
| 9  | Archon Support (Design Only)                  | ❌ Not Started |
| 10 | Performance & Polish                          | ❌ Not Started |
| 11 | Testing & QA                                  | ❌ Not Started |
| 12 | Release v1                                    | ❌ Not Started |

---

## Phase 0 — Project Setup & Licensing
- [ ] Init git repo, `.gitignore` (Rust + Node/Tauri targets + Windows build artifacts)
- [ ] Add `LICENSE-MIT` and `LICENSE-APACHE` (dual license, TPT Solutions copyright)
- [ ] Add SPDX dual-license header/notice convention for source files
- [ ] Cargo workspace: `core/` (data model, categorization engine, `StartupSource` trait), `platform-windows/`, `platform-linux/`, `src-tauri/` (Tauri backend wrapping core), `src/` (Svelte + Vite frontend); shared `[workspace.package]` (`license`, `authors = ["TPT Solutions"]`, `repository`)
- [ ] `rust-toolchain.toml` (MSRV pin), `rustfmt.toml`, `clippy.toml`
- [ ] Configure `tauri.conf.json` (app id, window defaults, bundle identifiers, elevation/manifest requirements for registry/Task Scheduler/systemd-unit edits)
- [ ] TypeScript config (tsconfig, Vite, Svelte)
- [ ] `README.md` (from spec.txt), build instructions, license badges
- [ ] Set up CI (GitHub Actions, build+lint+test on Windows + Linux runners)
- [ ] Repo hosting under `tpt-solutions` GitHub org

## Phase 1 — Core Data Model
- [ ] `StartupItem` model: source type (registry/startup-folder/task/systemd-unit/cron/xdg-autostart), display name, command/path, publisher/signature info, category (Safe/Recommended/Critical), boot-impact estimate, enabled/disabled state, reversible-disable capability flag
- [ ] Shared `trait StartupSource` (enumerate items, toggle enabled state, delete) so Windows/Linux backends share one interface
- [ ] Category enum + explanation-string association, kept independent of any one platform
- [ ] Core unit tests (mocked backend implementing `StartupSource`)

## Phase 2 — Windows Backend
- [ ] Enumerate registry Run/RunOnce (HKLM + HKCU, 32-bit and 64-bit registry views)
- [ ] Enumerate Startup folder shortcuts (per-user + all-users `shell:startup` paths)
- [ ] Enumerate Task Scheduler tasks filtered to logon/boot triggers (via `taskschd` COM API)
- [ ] Toggle/delete per source type: registry value delete/rename, shortcut move/delete, task disable/delete via COM
- [ ] Digital-signature verification (`WinVerifyTrust`) feeding the categorization engine
- [ ] UAC elevation manifest wired into the Tauri/MSVC build; verify unelevated-launch fallback messaging
- [ ] Unit/integration tests against a mockable Win32/COM boundary (CI-safe)

## Phase 3 — Linux Backend
- [ ] Enumerate systemd user units (`systemctl --user list-unit-files`, units/timers wanted by `default.target`)
- [ ] Enumerate cron `@reboot` entries (user crontab + `/etc/cron.d`)
- [ ] Enumerate XDG autostart `.desktop` files (`~/.config/autostart`, `/etc/xdg/autostart`)
- [ ] Toggle/delete per source type: `systemctl --user disable`, crontab line edit, `.desktop` `Hidden=true` or move/delete
- [ ] Document user-scope vs. root-scope elevation needs (system-wide `/etc/cron.d` and `/etc/xdg/autostart` entries need elevated access to modify)
- [ ] Verify behavior across at least one major distro (X11 + Wayland)

## Phase 4 — Categorization & Explanation Engine
- [ ] Curated known-item database: common apps/publishers → category + plain-English explanation (bundled, versioned data file)
- [ ] Heuristic fallback for unrecognized items: signature validity, install path (system vs. user vs. temp), known-risky naming/location patterns
- [ ] Safe/Recommended/Critical enforced in the engine itself, not just the UI (so the UI can't be tricked into mis-flagging a Critical item)
- [ ] Curated Critical list (never auto-suggest disable: core OS-signed startup entries) enforced same as tpt-monitor's critical-process-list approach
- [ ] Engine unit tests (fixture items → expected category + explanation, unknown-item fallback behavior)

## Phase 5 — Boot Impact Estimation
- [ ] Windows: investigate available signal (ETW boot trace vs. a lighter heuristic sampling process launch delay/CPU+disk during a monitored login) and document the chosen approach's accuracy tradeoffs
- [ ] Linux: parse `systemd-analyze blame`/`systemd-analyze --user blame` for systemd units
- [ ] Linux: mark cron/XDG-autostart entries "unknown impact" where no timing signal exists
- [ ] Normalize all platform signals to a common High/Medium/Low scale
- [ ] Unit tests for parsing (`systemd-analyze` output) and mocked Windows timing data

## Phase 6 — Item Actions (Enable/Disable/Delete)
- [ ] Wire enable/disable/delete from Phase 2/3 behind the `StartupSource` trait
- [ ] Confirmation-required flag enforced for any Critical-flagged item
- [ ] Prefer reversible disable over delete where the OS supports it (rename registry value instead of removing, `Hidden=true` instead of deleting the `.desktop` file, `systemctl --user disable` instead of removing the unit file)
- [ ] Action audit trail (in-memory/session log of what was toggled/deleted, for the confirmation UI and any "undo last action" affordance)

## Phase 7 — Tauri Shell & IPC Layer
- [ ] Tauri app scaffold wired to `core` scan/enumerate logic
- [ ] Command layer for list/toggle/delete actions
- [ ] IPC boundary validation (no arbitrary registry key/file path/unit name injection beyond the enumerated item set)
- [ ] On-demand scan model (scan triggered by UI open/refresh, not a background daemon by default)

## Phase 8 — Frontend Dashboard
- [ ] App shell (Svelte + Vite), light/dark theme
- [ ] Categorized list view (tabs or filter chips: Safe / Recommended / Critical), sortable/filterable/searchable
- [ ] Per-item detail panel: plain-English explanation, publisher, boot impact, source location/path, enabled state
- [ ] Enable/disable/delete controls with required confirmation dialog for Critical items
- [ ] Plain-English glossary/help panel (what "Recommended" vs. "Critical" means, what each source type is)
- [ ] Empty/loading/error states for scan failures (e.g. permission denied on an elevated-only source)

## Phase 9 — Archon Support (Design Only)
- [ ] `docs/archon-design.md`: design for lazy-loading + capability provisioning of startup entries at boot, ensuring idle services consume zero CPU cycles until explicitly triggered, per the spec's Archon section
- [ ] Open Questions (pending tpt-archon team): capability-grant model for registering/inspecting a user-space startup entry, mapping Archon's lazy-load/trigger model to the existing `StartupSource` trait, how enable/disable/delete map onto Archon's capability model
- [ ] Post-v1 `archon` Cargo feature flag plan (`ArchonStartupSource` stub, `is_available() -> false`, `Unsupported` errors until implemented)

## Phase 10 — Performance & Polish
- [ ] Profile scan time (must be fast and on-demand — the tool itself should not become a startup-impact contributor)
- [ ] App icon, branding assets (TPT Solutions)
- [ ] First-run onboarding (explain what Critical/Recommended/Safe mean, why elevation is requested)
- [ ] Crash reporting / local error logging (opt-in only)
- [ ] Accessibility pass (keyboard navigation, screen reader labels on the list + confirmation dialogs)

## Phase 11 — Testing & QA
- [ ] Unit tests: categorization engine, per-platform source parsing (mocked backends)
- [ ] Integration tests: enable/disable/delete against a disposable test registry key / systemd unit / cron line / `.desktop` file
- [ ] Manual QA: Windows 10/11 (elevated + unelevated launch)
- [ ] Manual QA: Linux (X11 + Wayland, at least one major distro; user-scope and root-scope entries)
- [ ] Security review: elevated attack surface (IPC command validation, no arbitrary registry/file/unit-name injection), Critical-item list cannot be bypassed to disable core OS startup entries

## Phase 12 — Release v1
- [ ] Finalize versioning scheme (SemVer) and changelog process
- [ ] Windows installer (MSI/NSIS via Tauri bundler) with UAC manifest embedded, code-signing decision
- [ ] Linux packaging (.deb/.rpm/AppImage via Tauri bundler), documented elevation path for root-scope sources
- [ ] Publish LICENSE files + notices in release artifacts
- [ ] User-facing docs (install guide, category/glossary explainer, safety notes on disabling Critical items)
- [ ] Tag v1.0.0 release, publish to GitHub (tpt-solutions org)
- [ ] Post-release triage process for bug reports

---

## Backlog / Future Phases (post-v1)

- [ ] Windows Services (services.msc) and systemd system-level daemon management
- [ ] macOS support (`launchd` LaunchAgents/LaunchDaemons)
- [ ] Community-sourced/crowd-updated categorization database (vs. static bundled list)
- [ ] CLI (scriptable item list/actions, e.g. `tpt-startup list`, `tpt-startup disable <id>`)
- [ ] Archon backend implementation (post design-only phase, pending public `tpt-archon` API)
- [ ] Historical change log ("this item was added on <date>, likely by <installer>")
- [ ] "Undo last action" persistent history beyond the current session
