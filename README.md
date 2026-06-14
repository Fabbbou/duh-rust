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

Restart your shell. Done.

## Migrating from the old Go `duh`

Coming from the original Go version? This uninstalls the old `duh` binary and
copies your packages + preferences into the new layout:

```sh
curl -sSL https://raw.githubusercontent.com/Fabbbou/duh-rust/main/migrate-from-go.sh | sh
```

It only removes the *old* binary (detected via its `self` subcommand — the new
Rust binary is never touched), keeps your packages, and backs up anything it
overwrites. Per-package `gitconfig` files aren't used by the new duh and are left
in place with a warning. Run `DUH_MIGRATE_FORCE=1 … | sh` to skip prompts.

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
duh add alias ll "ls -al"        # add an alias
duh add export EDITOR nvim        # add an export
duh add fn greet                  # create a function (opens $EDITOR)

duh rm alias ll                   # remove
duh ls                            # list everything
duh ls alias                      # list one kind

duh inject                        # print the shell script (what eval runs)
duh status                        # show sync state
duh reload                        # force-regenerate now
```

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

### SSH injection

```sh
duh ssh user@host                 # ssh in with your aliases + exports loaded
duh ssh user@host --cleanup       # remove the injected snippet afterwards
```

By default only aliases and exports are shipped (portable). Opt a host into
functions in `~/.config/duh/ssh.toml`:

```toml
[hosts."user@host"]
packages = ["default", "work"]
inject_functions = true
```

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
