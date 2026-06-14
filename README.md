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
(dynamic — `duh open <tab>` lists packages, `duh ls <tab>` lists filters, etc.).

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

```sh
duh add alias ll "ls -al"        # add an alias (to the default package)
duh add export EDITOR nvim        # add an export
duh add fn greet                  # create a function (opens $EDITOR)

duh rm alias ll                   # remove
duh ls                            # list everything (shows each package + path)
duh ls alias                      # list one kind
duh ls --package work             # list one package
duh ls fn                         # functions as a script → function tree, with docs
duh ls --fn greet                 # full documentation for one function
duh ls git                        # git aliases per package (from each gitconfig)

duh add git alias co checkout     # add a git alias to the package gitconfig
duh rm  git alias co              # remove it

duh where                         # print every path duh uses
duh open                          # open the default package folder in your editor
duh open work                     # open a specific package folder

duh status                        # show sync state + which package add/rm target
duh init                          # one-time rc wiring (run once)
duh inject                        # the script that wiring runs each shell start
```

Every `add`/`rm` prints which package it wrote to, so you always know the target.

Output is colorized when writing to a terminal and plain when piped (`NO_COLOR`
is respected). Add `--no-color` for plain colors, `--plain` for ASCII-only, or
`--json` (on `ls`/`status`) for machine-readable output.

### Reloading

The per-prompt hook auto-reloads your shell on the next prompt after any change.
To apply immediately in the current shell, the injected `duh-reload` function
re-evals on the spot. (There is no `duh reload` command — a child process can't
reload its parent shell; a shell function can.) `duh-cd` / `duh-cd-config` jump
to the packages and config folders.

### Packages

Share config via git repositories:

```sh
duh pkg add https://github.com/you/dotfiles   # clone + enable
duh pkg ls                                     # list packages
duh pkg sync                                   # pull updates for all enabled
duh pkg push dotfiles                          # commit + push your changes
duh pkg enable dotfiles / duh pkg disable …
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
duh add alias ll "ls -al" --ssh-safe   # flag an entry as ssh-safe
duh ssh user@host                       # ships ONLY flagged entries
duh ssh user@host --cleanup             # remove the injected snippet afterwards
```

Aliases/exports are never sent unless flagged, so secret exports never leak into
your SSH sessions by accident. (Functions are only shipped to a host that
explicitly sets `inject_functions=true`, and then unfiltered — see the doc.)
Full details — what's injected, how it works, per-host config, and the security
model — are in **[docs/ssh.md](docs/ssh.md)**.

## How the zero-latency check works

`duh inject` writes the generated script, a flat list of source files, and an
aggregate change stamp into the cache dir. The per-prompt hook
(`duh status --hook`) only `stat()`s those files and compares the stamp — no TOML
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

## Build from source

```sh
cargo build --release   # binary at target/release/duh
cargo test              # run the suite
```

## License

MIT © Fabbbou
