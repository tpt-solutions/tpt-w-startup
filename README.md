# tpt-startup

Smart Startup & Background Service Manager, by TPT Solutions.

A unified view of everything that runs at login or boot — registry Run keys,
Startup folder shortcuts, scheduled tasks, systemd user units, cron `@reboot`
jobs and XDG autostart entries — categorized as **Safe**, **Recommended** or
**Critical**, with a plain-English explanation for each item.

> Status: early development. The core engine and the Startup-folder (Windows)
> and XDG-autostart (Linux) backends are implemented and tested. The Tauri UI,
> registry/Task Scheduler/systemd backends, and packaging are not yet built.
> See [todo.md](todo.md) for the full roadmap.

## Layout

| Path | Purpose |
|------|---------|
| `core/` | Platform-independent data model, `StartupSource` trait, categorization engine, boot-impact normalization, audit-logged actions |
| `platform-windows/` | Windows backends (Startup folder so far) |
| `platform-linux/` | Linux backends (XDG autostart; cron `@reboot` parsing) |
| `src-tauri/` | Tauri shell: IPC commands in `lib.rs`, testable logic in `service.rs` |
| `src/` | Svelte 5 + Vite dashboard (`App.svelte`, `lib/`) |
| `docs/archon-design.md` | Design-only notes for the post-v1 Archon backend |

## Safety model

- **Categories are computed by the core engine**, never accepted from a
  backend or the UI. Critical rules in `core/data/known-items.json` cannot be
  downgraded by the curated list.
- **Disable is preferred over delete.** Disabling is reversible on every
  backend implemented so far (`Hidden=true`, moving a shortcut into `disabled/`).
- **Confirmation is required** for any change to a Critical item and for every
  delete.
- **Item ids are never turned into paths.** Each backend re-enumerates before
  acting, so an id that was not produced by enumeration is rejected.

## Development

Rust: the toolchain in `rust-toolchain.toml` (1.94.1; rustup installs it). The
Tauri dependency tree needs Rust 1.90 or newer.

Node: Node 20.19+ or 22.12+ is recommended. Vite and the Svelte plugin are pinned
to the 6.x line, which builds on Node 20.16 as well. Vite 8 requires Node 20.19+
and native Rolldown bindings.

```sh
npm install
npm run check        # svelte-check
npm run build        # frontend to dist/
npm run tauri dev    # run the desktop app

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Licensing

Dual-licensed under either of

- [MIT license](LICENSE-MIT)
- [Apache License, Version 2.0](LICENSE-APACHE)

at your option.
