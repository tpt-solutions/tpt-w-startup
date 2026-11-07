# Elevation and scope

tpt-startup separates **user scope** (your account) from **system scope**
(every user on the machine). Only user scope is changed without extra rights.

## Windows

| Source | Scope | Changes need |
|--------|-------|--------------|
| Startup folder (`%APPDATA%\...\Startup`) | user | nothing |
| Registry Run / RunOnce, HKCU | user | nothing |
| Registry Run / RunOnce, HKLM | system | administrator |
| All-users Startup folder | system | administrator |
| Scheduled tasks with logon or boot triggers | varies | administrator for system tasks (under `\Microsoft\`); nothing for your own tasks |

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
| `/etc/xdg/autostart` | system | root (listed; read-only without root) |
| `/etc/cron.d`, system units | system | root (not wired yet) |

- Disabling a systemd user unit runs `systemctl --user disable`. It does not
  delete the unit file.
- Disabling a cron entry comments the line out with a `#tpt-disabled` marker.
- `/etc/xdg/autostart` is listed. Without root, changes fail with a permission
  error. `/etc/cron.d` and system-wide systemd units are not listed yet.

## Rules that apply to every source

- Changes to user scope never need elevation.
- If a change needs more rights than the process has, the change fails with a
  clear message. The app does not escalate on its own.
