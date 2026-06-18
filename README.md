# duh

> Inject your shell config — aliases, exports, functions — everywhere. Fast, direnv-style.

`duh` keeps your shell aliases, environment exports, and functions in simple TOML
**packages** and injects them into your shell with one `eval`. A per-prompt hook
checks for changes the way [direnv](https://direnv.net) does — stat-only, so it
adds no noticeable latency — and reloads only when something actually changed.
It can also inject your config into a remote host over SSH.

This is a Rust rewrite of the original Go `duh`, with a smaller, simpler CLI.

## Install

```sh
curl -sSL https://raw.githubusercontent.com/Fabbbou/duh-rust/main/install.sh | sh
```

Then wire it into your shell:

```sh
# bash
echo 'eval "$(duh init --shell bash)"' >> ~/.bashrc
# zsh
echo 'eval "$(duh init --shell zsh)"'  >> ~/.zshrc
```

Restart your shell. Done. The `duh init` snippet also enables **tab completion**
(dynamic — `duh use <tab>` lists packages, `duh get <tab>` lists resources, etc.).

## Updating

```sh
duh upgrade          # update to the latest release (verifies checksum, swaps binary)
duh upgrade --check  # just report whether a newer version exists
```

`duh upgrade` is a no-op if you're already on the latest version. (Re-running the
install one-liner also works.)

## Uninstall

```sh
duh uninstall          # prompts whether to also delete local packages
duh uninstall --purge  # delete binary, cache, packages, and config (no prompt)
```

Or without the binary on hand, the one-liner:

```sh
curl -sSL https://raw.githubusercontent.com/Fabbbou/duh-rust/main/uninstall.sh | sh
```

Both keep your packages by default and ask before deleting them (the script keeps
them when piped non-interactively — set `DUH_PURGE=1` to delete, `DUH_KEEP=1` to
keep). Remember to remove the `eval "$(duh init ...)"` line from your shell rc.

## Usage

The CLI is **kubectl-style**: `duh <verb> <resource> [name]`. Resources are
`alias`, `export`, `fn`, `pkg`, `gitalias` (aliases like `aliases`, `package`,
`func`, `git` also work).

```sh
duh create alias ll "ls -al"      # create an alias (in the default package)
duh create export EDITOR nvim     # create an export
duh create fn greet               # create a function (opens $EDITOR)
duh create gitalias co checkout   # create a git alias in the package gitconfig

duh delete alias ll               # remove an entry
duh delete gitalias co            # remove a git alias

duh get                           # list everything (each package + path)
duh get alias                     # list one kind
duh get --package work            # list one package (-p for short)
duh get fn                        # functions as a script → function tree, with docs
duh get gitalias                  # git aliases per package (from each gitconfig)
duh get pkg                       # list packages and their enabled state

duh describe fn greet             # full documentation for one function
duh describe pkg work             # detail for one package

duh use                           # show the default (write-target) package
duh use work                      # switch the default package
duh edit pkg                      # edit the default package's db.toml in $EDITOR
duh edit fn greet                 # edit a function in $EDITOR

duh where                         # print every path duh uses
duh doctor                        # diagnose your setup (wiring, conflicts, …)
duh init                          # one-time rc wiring (run once)
```

Every `create`/`delete` prints which package it wrote to, so you always know the
target. Use `-p/--package` to act on a package other than the default.

Output is colorized when writing to a terminal and plain when piped (`NO_COLOR`
is respected). Add `--no-color` for plain colors, `--plain` for ASCII-only, or
`--json` (on `get`/`describe`) for machine-readable output.

The shell integration calls two hidden commands (`duh _internal emit` at shell
start, `duh _internal hook` each prompt). You never type them — they're omitted
from `--help` on purpose.

> **Migrating from ≤0.10?** In 0.11 the package lifecycle verbs moved back under
> `duh pkg` (`duh pkg enable/disable/rename/sync/push/export/import/open`), and
> `duh inject` / `duh status` / `duh man` were removed (the first two became
> `duh _internal emit` / `duh _internal hook`; diagnostics live in `duh doctor`).

### Reloading

The per-prompt hook auto-reloads your shell on the next prompt after any change.
To apply immediately in the current shell, the injected `duh-reload` function
re-evals on the spot. (There is no `duh reload` command — a child process can't
reload its parent shell; a shell function can.) `duh-cd` jumps to the packages
folder.

### Packages

Share config via git repositories:

```sh
duh create pkg work                                    # new empty local package
duh create pkg dotfiles --remote https://github.com/you/dotfiles  # clone + enable
duh get pkg                                            # list packages
duh pkg sync                                           # pull updates for all enabled
duh pkg push dotfiles                                  # commit + push your changes
duh pkg enable dotfiles / duh pkg disable …
duh pkg rename old new                                 # rename a local package
duh pkg export work / duh pkg import work.tar.gz       # share without git
duh pkg open work                                      # open a package folder in your editor
```

Later-enabled packages override earlier ones, so a personal package can shadow a
shared one.

### Git config per package

Drop a `gitconfig` file in a package directory (next to its `db.toml`) and duh
wires it into your `~/.gitconfig` via git's `[include]` mechanism on the next
inject — so a package can ship git aliases/settings:

```ini
# ~/.gitconfig (after inject)
[include]
        path = /home/you/.local/share/duh/packages/default/gitconfig
        path = /home/you/.local/share/duh/packages/work/gitconfig
```

Add-only and idempotent (never duplicates a line, never removes your own
includes); enabled packages only; never shipped over SSH.

### SSH injection

SSH injection is **opt-in**: only entries you flag are ever shipped to a remote.

```sh
duh create alias ll "ls -al" --ssh-safe # flag an entry as ssh-safe
duh ssh user@host                       # ships ONLY flagged entries
duh ssh user@host --cleanup             # remove the injected snippet afterwards
```

Aliases/exports are never sent unless flagged, so secret exports never leak into
your SSH sessions by accident. (Functions are only shipped to a host that
explicitly sets `inject_functions=true`, and then unfiltered — see the doc.)
Full details — what's injected, how it works, per-host config, and the security
model — are in **[docs/ssh.md](docs/ssh.md)**.

## How the zero-latency check works

`duh _internal emit` writes the generated script, a flat list of source files, and
an aggregate change stamp into the cache dir. The per-prompt hook
(`duh _internal hook`) only `stat()`s those files and compares the stamp — no TOML
parsing on the hot path. It prints a reload command **only** when something
changed, so a steady-state prompt pays just a few `stat` syscalls.

## Security

All alias/export **values** are emitted single-quoted, which disables every form
of shell expansion (`$()`, backticks, `$VAR`, globbing); values with control
characters are rejected. Alias/export **names** must match
`[A-Za-z_][A-Za-z0-9_]*`; package names are validated to block path traversal.
Remote package URLs are restricted to `https://`, `ssh://`, `git://`, and
`git@host:path`. The generated cache (`inject.sh`) is written `0600` because it
can contain secret exports. See `src/inject/escape.rs` and `src/config/paths.rs`.

> **Function bodies run as code.** Functions are injected into your shell
> verbatim (that's the point), so **adding and enabling an untrusted package is
> equivalent to running its code.** `duh` warns when a package ships function
> files; review them before trusting a package. SSH injection ships aliases and
> exports only — functions are sent only when a host opts in via `ssh.toml`.

## Platforms

Linux and macOS (including WSL). On Linux duh follows XDG
(`~/.local/share/duh`, `~/.config/duh`, `~/.cache/duh`, honoring `XDG_*`); on
macOS it uses the Apple dirs (`~/Library/Application Support/net.fabou.duh`,
`~/Library/Caches/net.fabou.duh`). Windows support is planned and is the gate
for 1.0.

## On-disk format

`prefs.toml` and each package `db.toml` carry a `schema` version. As of 0.9 the
format is stable and forward-migratable; files written by a newer duh are
detected and warned about. See [CHANGELOG.md](CHANGELOG.md).

## Build from source

```sh
cargo build --release   # binary at target/release/duh
cargo test              # run the suite
```

## License

MIT © Fabbbou
