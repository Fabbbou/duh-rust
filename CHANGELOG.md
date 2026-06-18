# Changelog

All notable changes to duh are documented here. Format follows
[Keep a Changelog](https://keepachangelog.com); duh uses semantic versioning.

## [0.11.0]

**BREAKING — decluttered command surface.** Machine-only commands are hidden,
low-value ones removed, and package lifecycle ops are grouped under `duh pkg`.

### Changed
- Package lifecycle verbs moved back under `duh pkg`:
  `duh pkg enable|disable|rename|sync|push|export|import|open`
  (`duh open` → `duh pkg open`). Package CRUD stays verb-noun
  (`create pkg` / `get pkg` / `delete pkg`); `duh use` stays top-level.

### Removed
- `duh inject` and `duh status` — these were the shell-hook engine, not things you
  type. They became hidden commands: `duh _internal emit` (write cache + print the
  eval script) and `duh _internal hook` (per-prompt staleness check). `duh init`
  now wires those; `duh-reload` re-evals `duh _internal emit`.
- `duh man` (and the `clap_mangen` dependency).
- The auto `help` subcommand (`-h` / `--help` still work).
- The `duh-cd-config` shell helper (low value); `duh-cd` stays.

### Notes
- A child process can't change the parent shell's cwd or environment, so `duh-cd`
  and `duh-reload` remain shell functions rather than subcommands by necessity.

## [0.10.0]

**BREAKING — new kubectl-style grammar.** Every command is now
`duh <verb> <resource> [name]`. The old verb-first/noun-first mix
(`add`, `rm`, `ls`, `pkg <op>`) is removed. Resources: `alias`, `export`, `fn`,
`pkg`, `gitalias` (with aliases like `aliases`, `package`, `func`, `git`).

### Changed
- CRUD verbs: `get` (list/show), `create`, `edit`, `delete`, `describe`.
- Package lifecycle ops are now flat top-level verbs (the `pkg` namespace is gone).
- `create`/`delete` take `-p/--package` to target a non-default package.

### Migration

| old (≤0.9) | new (0.10) |
|---|---|
| `duh add alias N V` / `add export N V` | `duh create alias N V` / `create export N V` |
| `duh add fn N` | `duh create fn N` |
| `duh add git alias N V` | `duh create gitalias N V` |
| `duh rm alias/export/fn N` | `duh delete alias/export/fn N` |
| `duh rm git alias N` | `duh delete gitalias N` |
| `duh ls [kind]` | `duh get [resource]` |
| `duh ls --fn N` | `duh describe fn N` |
| `duh pkg ls` | `duh get pkg` |
| `duh pkg create N` | `duh create pkg N` |
| `duh pkg add URL [N]` | `duh create pkg N --remote URL` |
| `duh pkg rm N` | `duh delete pkg N` |
| `duh pkg enable/disable N` | `duh enable/disable N` |
| `duh pkg rename O N` | `duh rename O N` |
| `duh pkg sync` / `pkg push N` | `duh sync` / `duh push N` |
| `duh pkg export N` / `pkg import F` | `duh export N` / `duh import F` |
| `duh use` / `edit` / `open` | unchanged (`edit` now takes a resource: `edit pkg` / `edit fn N`) |

## [0.9.0]

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
