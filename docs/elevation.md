# Elevation and scope

tpt-startup separates **user scope** (your account) from **system scope**
(every user on the machine). Only user scope is changed without extra rights.

## Windows

| Source | Scope | Changes need |
|--------|-------|--------------|
| Startup folder (`%APPDATA%\...\Startup`) | user | nothing |
| Registry Run / RunOnce, HKCU | user | nothing |
| Registry Run / RunOnce, HKLM | system | administrator |
| All-users Startup folder | system | administrator (not wired yet) |
| Scheduled tasks | varies | not implemented yet |

- The app manifest requests `asInvoker`. The app starts unelevated and never
  shows a UAC prompt at launch.
- System entries are still listed when unelevated. Trying to change one returns
  a permission error, and the UI explains how to reopen the app as
  administrator.
- Signature checks (`WinVerifyTrust`) need no elevation.

## Linux

| Source | Scope | Changes need |
|--------|-------|--------------|
| XDG autostart, `~/.config/autostart` | user | nothing |
| systemd user units (`systemctl --user`) | user | nothing |
| User crontab (`crontab -l` / `crontab -`) | user | nothing |
| `/etc/xdg/autostart`, `/etc/cron.d`, system units | system | root (not wired yet) |

- Disabling a systemd user unit runs `systemctl --user disable`. It does not
  delete the unit file.
- Disabling a cron entry comments the line out with a `#tpt-disabled` marker.
- System-scope sources are not listed yet. When they are added they will be
  read-only until the user chooses to run with root.

## Rules that apply to every source

- Changes to user scope never need elevation.
- If a change needs more rights than the process has, the change fails with a
  clear message. The app does not escalate on its own.
