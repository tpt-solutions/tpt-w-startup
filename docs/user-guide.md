# tpt-startup user guide

tpt-startup shows the programs that start when you sign in or boot your
computer. For each one it gives a plain-English explanation and a safety label.

> Windows: run `tpt-startup_<version>_x64-setup.exe` from the installer build.
> Linux packages are not built yet. To run from source, see the README.

## The three labels

**Safe.** Well-known software that is fine to turn off at login. Turning it off
only stops it from opening automatically.

**Recommended.** Not in our curated list, or known to be useful. Check where it
comes from before you change it. This is the default for anything we do not
recognise.

**Critical.** Part of the operating system, or suspicious. Turning it off can
break something or hide a real problem. You are asked to confirm before any
change. Items are marked Critical when:

- they are on our list of core system entries (for example the Windows shell or
  the Linux service manager), or
- they run from a temporary folder, or
- their digital signature is present but does not verify.

The label is set by the app's own rules and cannot be changed from the list.

## Boot impact

How much an item slows startup, when we can measure it. On Windows this is
**Unknown** in this version. On Linux it comes from systemd's own timing data.
"Unknown" means we do not know, not that the item is harmless.

## Turning things off and deleting them

- **Disable** keeps the entry so you can turn it back on from the same screen.
  This is the normal choice.
- **Delete** removes the entry permanently. You are always asked first.
- systemd services can be turned off and on, but not deleted here.
- Some entries are system-wide and need administrator rights to change. If the
  app is not running as administrator, it says so and does not change anything.

## Recent changes

The "Recent changes" panel lists what you changed in this session. It is cleared
when you close the app. Nothing is stored on disk.

## Where the entries come from

| Name shown | What it is | Windows | Linux |
|------------|------------|---------|-------|
| Startup folder | Shortcuts in your Startup folder | yes | |
| Registry Run key | Commands stored in the registry that run at sign-in | yes | |
| Autostart (.desktop) | Desktop-session autostart entries | | yes |
| systemd user unit | Background services enabled for your account | | yes |
| Cron @reboot job | Commands scheduled to run at boot | | yes |
| Scheduled task | Task Scheduler jobs that start at sign-in or boot | yes | |

## Safety notes

- Do not disable Critical items unless you know what they do.
- If an item you do not recognise is Critical, look up its location in the
  details panel before changing anything.
- Disable first. Delete only when you are sure.
