# Archon Support — Design (Post-v1, design only)

Status: **design only**. No Archon code runs in v1. The `archon` Cargo feature
in `tpt-startup-core` compiles an inert stub so the trait boundary is exercised
in CI.

Reference: https://github.com/tpt-solutions/tpt-archon

## Goal

On Archon, background services are lightweight user-space tasks. tpt-startup
should list them alongside other startup entries and let the user control when
they start, so that idle services use zero CPU until explicitly triggered.

## Mapping to `StartupSource`

| `StartupSource` method | Archon behaviour (proposed) |
|------------------------|-----------------------------|
| `enumerate()` | Ask `tpt-archon` for registered lazy-load entries and their trigger conditions. |
| `set_enabled(id, false)` | Revoke the entry's boot-time trigger, keeping its registration. Reversible. |
| `set_enabled(id, true)` | Restore the boot-time trigger. |
| `delete()` | Unregister the entry. Requires confirmation (see the core action rules). |

Boot impact on Archon is expected to be `Unknown` for lazy entries, because
they do not run at boot. The UI should say so rather than show a misleading
High/Medium/Low value.

## Stub behaviour

`ArchonStartupSource` (feature `archon`):

- `is_available()` returns `false`.
- `enumerate()`, `set_enabled()`, and `delete()` return `SourceError::Unsupported`.

The UI must check `is_available()` before showing an Archon section, so the
stub is never visible to users.

## Open questions (pending the tpt-archon team)

1. **Capability grant.** What capability does tpt-startup need to register or
   inspect a user-space startup entry, and can it be scoped to the current user?
2. **Trigger model.** How does Archon's lazy-load trigger map onto the existing
   enable/disable semantics? Is "disable" a revoked trigger, or a stopped
   service?
3. **Telemetry.** Can Archon report per-entry activation counts or CPU time,
   which would let us estimate impact without a boot trace?
4. **Delete semantics.** Does unregistering an entry also remove its
   provisioned capabilities, and can that be undone?

Until these are answered, no implementation work starts.
