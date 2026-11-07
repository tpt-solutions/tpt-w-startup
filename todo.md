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
| 0  | Project Setup & Licensing                    | 🟡 Partial — workspace, licensing, CI, Tauri + frontend scaffold done; elevation manifest and repo hosting outstanding |
| 1  | Core Data Model                              | ✅ Done |
| 2  | Windows Backend                              | ✅ Done — Startup folders (user + all-users), registry Run/RunOnce (native + WOW6432Node), Task Scheduler, signatures; HKCU WOW6432Node not read |
| 3  | Linux Backend                                | 🟡 Partial — XDG autostart, systemd user units, user crontab; system scope and distro QA outstanding |
| 4  | Categorization & Explanation Engine          | ✅ Done |
| 5  | Boot Impact Estimation                       | 🟡 Partial — normalization, systemd timings, Windows decision documented (not measured) |
| 6  | Item Actions (Enable/Disable/Delete)         | ✅ Done — wired to every implemented source; Undo reverses the last enable/disable |
| 7  | Tauri Shell & IPC Layer                       | ✅ Done — commands wired to core; ids validated by re-enumeration |
| 8  | Frontend Dashboard                            | 🟡 Built and type-checked; not yet exercised by hand in the running app |
| 9  | Archon Support (Design Only)                  | ✅ Done (design-only, as specified) |
| 10 | Performance & Polish                          | 🟡 Partial — first-run notice only |
| 11 | Testing & QA                                  | 🟡 Partial — 86 automated tests (Windows registry and file tests on real disposable data; read-only live Task Scheduler query); no manual QA |
| 12 | Release v1                                    | 🟡 Partial — versioning, changelog, user docs and a local NSIS installer done; code-signing decision, Linux packages and the release itself outstanding |

Test counts at last run: 86 automated tests, all passing; `cargo clippy -D warnings` and `cargo fmt --check` clean.

## Phase 0 — Project Setup & Licensing
- [x] Init git repo, `.gitignore` (Rust + Node/Tauri targets + Windows build artifacts)
- [x] Add `LICENSE-MIT` and `LICENSE-APACHE` (dual license, TPT Solutions copyright)
- [x] Add SPDX dual-license header/notice convention for source files (`// SPDX-License-Identifier: MIT OR Apache-2.0` + copyright line on every Rust source file)
- [x] Cargo workspace: `core/`, `platform-windows/`, `platform-linux/`, `src-tauri/`, and `src/` (Svelte + Vite frontend); shared `[workspace.package]` (`license`, `authors = ["TPT Solutions"]`, `repository`)
- [x] `rust-toolchain.toml` (MSRV pin), `rustfmt.toml`, `clippy.toml`
- [x] Configure `tauri.conf.json` (app id, window defaults, bundle identifiers; `asInvoker` app manifest embedded on Windows). Task Scheduler and systemd edit rights are not needed yet.
- [x] TypeScript config (tsconfig, Vite, Svelte)
- [x] `README.md` (from spec.txt), build instructions, license badges — partial: no license badges yet
- [x] Set up CI (GitHub Actions, build+lint+test on Windows + Linux runners) — workflow written in `.github/workflows/ci.yml`, not yet run on GitHub
- [ ] Repo hosting under `tpt-solutions` GitHub org

## Phase 1 — Core Data Model
- [x] `StartupItem` model: source type (registry/startup-folder/task/systemd-unit/cron/xdg-autostart), display name, command/path, publisher/signature info, category (Safe/Recommended/Critical), boot-impact estimate, enabled/disabled state, reversible-disable capability flag
- [x] Shared `trait StartupSource` (enumerate items, toggle enabled state, delete) so Windows/Linux backends share one interface
- [x] Category enum + explanation-string association, kept independent of any one platform
- [x] Core unit tests (mocked backend implementing `StartupSource`)

## Phase 2 — Windows Backend
- [x] Enumerate registry Run/RunOnce (HKCU + HKLM, Run + RunOnce, plus HKLM WOW6432Node for 32-bit entries; HKCU WOW6432Node not read)
- [x] Enumerate Startup folder shortcuts (per-user + all-users `shell:startup` paths); all-users changes need administrator rights
- [x] Enumerate Task Scheduler tasks filtered to logon/boot triggers (via one `schtasks /query /xml` call; COM API not used)
- [x] Toggle/delete per source type: registry value rename/delete, shortcut move/delete (disable = move to `disabled/`), task enable/disable/delete via `schtasks`
- [x] Digital-signature verification (`WinVerifyTrust`, embedded and catalog signatures) feeding the categorization engine
- [x] UAC elevation manifest wired into the Tauri/MSVC build (`asInvoker`); unelevated-launch fallback messaging shown when a change needs administrator rights. Manual QA still pending.
- [x] Unit/integration tests against a mockable Win32/COM boundary (CI-safe) — Startup folder tests use temp dirs; registry tests use a throwaway HKCU key; no COM boundary yet

## Phase 3 — Linux Backend
- [x] Enumerate systemd user units (`systemctl --user list-unit-files`; enabled units plus disabled units whose preset is enabled)
- [x] Enumerate cron `@reboot` entries (user crontab). `/etc/cron.d` is not read yet (system scope)
- [x] Enumerate XDG autostart `.desktop` files (`~/.config/autostart`, `/etc/xdg/autostart`) — both scopes listed; system scope is read-only without root
- [x] Toggle/delete per source type: `systemctl --user enable/disable` (delete is unsupported for systemd units by design), crontab line comment/uncomment and delete, `.desktop` `Hidden=true` and delete
- [x] Document user-scope vs. root-scope elevation needs (`docs/elevation.md`)
- [ ] Verify behavior across at least one major distro (X11 + Wayland)

## Phase 4 — Categorization & Explanation Engine
- [x] Curated known-item database: common apps/publishers → category + plain-English explanation (bundled, versioned data file) — `core/data/known-items.json`, version 1; small seed list, needs expanding
- [x] Heuristic fallback for unrecognized items: signature validity, install path (system vs. user vs. temp), known-risky naming/location patterns — temp path and invalid signature implemented; "system install path" is not used as a signal
- [x] Safe/Recommended/Critical enforced in the engine itself, not just the UI (so the UI can't be tricked into mis-flagging a Critical item)
- [x] Curated Critical list (never auto-suggest disable: core OS-signed startup entries) enforced same as tpt-monitor's critical-process-list approach — critical rules always win over the curated list
- [x] Engine unit tests (fixture items → expected category + explanation, unknown-item fallback behavior)

## Phase 5 — Boot Impact Estimation
- [x] Windows: investigate available signal and document the chosen approach (`docs/boot-impact.md`: Unknown in v1; launch-time sampling planned as an explicit user action). Measurement itself is not implemented.
- [x] Linux: parse `systemd-analyze blame`/`systemd-analyze --user blame` for systemd units — parser done (`core/src/impact.rs`); the command is not yet run
- [x] Linux: mark cron/XDG-autostart entries "unknown impact" where no timing signal exists
- [x] Normalize all platform signals to a common High/Medium/Low scale (thresholds: ≥3 s High, ≥0.5 s Medium)
- [x] Unit tests for parsing (`systemd-analyze` output) — mocked Windows timing data not applicable until the Windows signal exists

## Phase 6 — Item Actions (Enable/Disable/Delete)
- [x] Wire enable/disable/delete from Phase 2/3 behind the `StartupSource` trait — every implemented source; the Tauri command layer uses the owning backend
- [x] Confirmation-required flag enforced for any Critical-flagged item (delete always requires confirmation too)
- [x] Prefer reversible disable over delete where the OS supports it (XDG `Hidden=true`, startup-folder move to `disabled/`, registry value rename, `systemctl --user disable`, cron comment marker)
- [x] Action audit trail (in-memory/session log of what was toggled/deleted, for the confirmation UI and any "undo last action" affordance) — `core::AuditLog`; Undo button reverses the last enable/disable this session

## Phase 7 — Tauri Shell & IPC Layer
- [x] Tauri app scaffold wired to `core` scan/enumerate logic (`src-tauri/src/service.rs`, Tauri-free and unit-tested)
- [x] Command layer for list/toggle/delete actions (`list_items`, `apply_action`, `audit_log`)
- [x] IPC boundary validation (no arbitrary registry key/file path/unit name injection beyond the enumerated item set) — every action re-enumerates and rejects ids not found; category and confirmation are decided server-side
- [x] On-demand scan model (scan triggered by UI open/refresh, not a background daemon by default)

## Phase 8 — Frontend Dashboard
- [x] App shell (Svelte + Vite), light/dark theme (follows the OS setting; no manual toggle yet)
- [x] Categorized list view (tabs or filter chips: Safe / Recommended / Critical), filterable/searchable (Critical first; no column sorting yet)
- [x] Per-item detail panel: plain-English explanation, publisher, boot impact, source location/path, enabled state
- [x] Enable/disable/delete controls with required confirmation dialog for Critical items (and for every delete)
- [x] Plain-English glossary/help panel (what "Recommended" vs. "Critical" means; source-type names shown per item)
- [x] Empty/loading/error states for scan failures (per-backend warnings shown, so one failing source does not hide the rest)

## Phase 9 — Archon Support (Design Only)
- [x] `docs/archon-design.md`: design for lazy-loading + capability provisioning of startup entries at boot, ensuring idle services consume zero CPU cycles until explicitly triggered, per the spec's Archon section
- [x] Open Questions (pending tpt-archon team): capability-grant model for registering/inspecting a user-space startup entry, mapping Archon's lazy-load/trigger model to the existing `StartupSource` trait, how enable/disable/delete map onto Archon's capability model
- [x] Post-v1 `archon` Cargo feature flag plan (`ArchonStartupSource` stub, `is_available() -> false`, `Unsupported` errors until implemented) — `core` feature `archon`, built in CI

## Phase 10 — Performance & Polish
- [ ] Profile scan time (must be fast and on-demand — the tool itself should not become a startup-impact contributor)
- [ ] App icon, branding assets (TPT Solutions)
- [x] First-run onboarding (dismissible notice explaining labels, reversibility and admin rights; dismissal stored per viewer)
- [ ] Crash reporting / local error logging (opt-in only)
- [ ] Accessibility pass (keyboard navigation, screen reader labels on the list + confirmation dialogs)

## Phase 11 — Testing & QA
- [x] Unit tests: categorization engine, per-platform source parsing (mocked backends)
- [ ] Integration tests: enable/disable/delete against a disposable test registry key / systemd unit / cron line / `.desktop` file — registry, startup-folder and `.desktop` tests run against real disposable files/keys; systemd and cron tests use a fake command runner, not real systemd/crontab
- [ ] Manual QA: Windows 10/11 (elevated + unelevated launch)
- [ ] Manual QA: Linux (X11 + Wayland, at least one major distro; user-scope and root-scope entries)
- [ ] Security review: elevated attack surface (IPC command validation, no arbitrary registry/file/unit-name injection), Critical-item list cannot be bypassed to disable core OS startup entries

## Phase 12 — Release v1
- [x] Finalize versioning scheme (SemVer) and changelog process (`docs/versioning.md`, `CHANGELOG.md`)
- [x] Windows installer (NSIS via Tauri bundler) builds locally with the `asInvoker` manifest embedded. Code signing is not decided yet: the installer is unsigned.
- [ ] Linux packaging (.deb/.rpm/AppImage via Tauri bundler), documented elevation path for root-scope sources
- [ ] Publish LICENSE files + notices in release artifacts
- [ ] User-facing docs (install guide, category/glossary explainer, safety notes on disabling Critical items) — user guide with labels, glossary and safety notes done (`docs/user-guide.md`); install guide waits for installers
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
