# Boot Impact: how it is estimated

Status: the Linux signal is implemented. The Windows signal is **not measured
in v1**; Windows items show "Unknown". This document records why, and what
would change that.

## Shared scale

Every platform signal is turned into a per-item duration in milliseconds, then
mapped by `core/src/impact.rs`:

| Tier | Threshold |
|------|-----------|
| High | 3 s or more |
| Medium | 0.5 s up to 3 s |
| Low | under 0.5 s |
| Unknown | no timing signal exists |

The thresholds are a judgement call, not a benchmark. Change them in one place
if real data suggests otherwise.

## Linux (implemented)

- **systemd user units:** `systemd-analyze --user blame` gives each unit's
  startup time. The parser is `core::impact::parse_systemd_blame`. The
  systemd backend calls the command and maps the result. If the command is
  missing or fails, units are shown as Unknown rather than as an error.
- **cron `@reboot` and XDG autostart:** no timing signal exists, so they are
  always Unknown.

## Windows (decision: Unknown in v1)

Options considered:

1. **ETW boot trace.** Most accurate. It needs administrator rights, a trace
   session, and a parser for the event stream. That is too heavy for an
   on-demand scan, and it cannot be used from an unelevated launch.
2. **Launch-time sampling.** Measure how long each startup process takes to
   reach its first idle point after login. It needs a monitored login and
   reliable process attribution. It is possible, but the results depend on
   load at the time and would need a separate "measure now" step.
3. **Static estimate** (executable size, publisher, known heavy frameworks).
   It is cheap, but the output would look like a measurement when it is only a
   guess, which is misleading.

**Chosen for v1:** show Unknown for Windows and say so in the UI. Option 2 is
the planned next step, as an explicit "Measure startup impact" action that the
user runs on purpose. It is not part of the default scan. Option 3 is rejected.

Until option 2 exists, the High/Medium/Low column means something only on
Linux systemd units.
