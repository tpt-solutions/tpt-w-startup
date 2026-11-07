# Versioning and changelog process

## Versions

The project uses [Semantic Versioning](https://semver.org/):

- **MAJOR** when a release breaks a stable interface: the `StartupSource` trait,
  the IPC command names or their argument shapes, or the on-disk formats the app
  writes.
- **MINOR** for new backends, new sources, or new UI features that keep
  existing behaviour.
- **PATCH** for fixes that change no interface.

Before v1.0.0 the project is in the `0.x` series. Breaking changes are still
recorded in the changelog, but MINOR may carry them.

The version in `Cargo.toml` (`[workspace.package]`) and `package.json` must
match. `src-tauri/tauri.conf.json` carries it too.

## Changelog

`CHANGELOG.md` follows [Keep a Changelog](https://keepachangelog.com/):

1. Add entries under `## [Unreleased]` as you work. Group them under Added,
   Changed, Deprecated, Removed, Fixed or Security.
2. For each entry, state the user-visible effect in one line. Put the internal
   detail in the commit message.
3. When cutting a release, rename `Unreleased` to the version and date, and open
   a new empty `Unreleased` section.
4. Security fixes always get their own entry, even when small.

## Release checklist

Publishing is a separate, deliberate step. These checks come before it:

- `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` pass.
- `npm run check` and `npm run build` pass.
- Versions match across the files listed above.
- `CHANGELOG.md` has the release section.
- `LICENSE-MIT` and `LICENSE-APACHE` are included in the artifacts (todo.md, Phase 12).
