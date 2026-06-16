# Changelog

All notable changes to duh are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com); duh uses semantic versioning.

## [0.9.0] — unreleased

Pre-1.0 milestone: fill the remaining CLI gaps and add stability/diagnostics.
(1.0 is gated on Windows support.)

### Added
- `duh use [<pkg>]` — show or set the default (write-target) package.
- `duh pkg create <name>` — create a new empty local package.
- `duh pkg rename <old> <new>`.
- `duh pkg export <name>` / `duh pkg import <file> [name]` — share a package as a
  `.tar.gz` without a git remote.
- `duh doctor` — diagnostics: shell wiring, default/enabled packages present,
  cache freshness, gitconfig includes, cross-package alias/export conflicts,
  function lint. Exits non-zero on hard errors.
- `duh edit [<pkg>]` — open a package's `db.toml` in `$EDITOR`.
- `duh man` — render the man page (roff) to stdout.
- On-disk **schema version** in `prefs.toml` and package `db.toml`; files without
  it load as v1; a newer schema warns. 0.9 establishes a stable on-disk format.
- `duh ls` marks shadowed alias/export entries `(shadowed by <pkg>)`.

### Notes
- Platforms: Linux, macOS (incl. WSL). Windows support is planned and gates 1.0.

## [0.7.2]
- Richer `duh ls --fn <name>` describe view (name, package, script, path, full doc).

## [0.7.1]
- Colored `ls` keyword labels (alias/export/git/script/fn) + per-package stat line.

## [0.7.0]
- Dynamic tab completion (packages, filters, function names, ssh hosts).
- gitconfig in `ls` + `duh add/rm git alias`; `duh ls git`.
- `ls` script/function labels.

## [0.6.0]
- Per-package `gitconfig` included into `~/.gitconfig` at inject time.

## [0.5.0]
- Restyled output: eza-style tree, colors, `--json`, `--plain`/`--no-color`;
  SIGPIPE-safe.

## [0.4.0]
- Real shell-parser function parsing in `ls`; shebang stripped on inject;
  friendly release artifact names (`duh-<os>-<arch>`).

## [0.3.x]
- `duh upgrade` self-update; install/checksum fixes.

## [0.2.0]
- "default package" UX, opt-in SSH allowlist, `duh-reload`/`duh-cd` helpers,
  `where`, `open`, function doc/lint.

## [0.1.x]
- Initial Rust rewrite: packages, inject, direnv-style hook, SSH injection,
  uninstall, migration from the Go `duh`, CI + release matrix + installer.
