# SSH injection

`duh ssh <host>` opens an SSH session with **only the config you explicitly
allowed** loaded on the remote. It is opt-in by design: nothing leaves your
machine unless you flag it.

## TL;DR

```sh
duh add alias ll "ls -al" --ssh-safe     # flag an alias as ssh-safe
duh add export EDITOR nvim --ssh-safe     # flag an export as ssh-safe
duh ssh user@host                          # ships ONLY the flagged entries
```

If nothing is flagged, `duh ssh` connects normally and injects nothing (it
prints a note telling you so).

## What gets injected

| Item | Shipped over SSH? |
|---|---|
| Aliases flagged `--ssh-safe` | ✅ yes |
| Exports flagged `--ssh-safe` | ✅ yes |
| Aliases / exports **not** flagged | ❌ never |
| Functions | ⚠️ only if a host sets `inject_functions=true` — and then **unfiltered** (see below) |
| Local shell helpers (`duh-reload`, `duh-cd`, …) | ❌ never (they assume a local duh) |

This is an **allowlist**: the default is to send nothing. Secrets you keep in
exports (API tokens, etc.) never reach a remote host unless you deliberately
flag them, so installing duh can't cause surprise leakage into your SSH sessions.

## How it works

1. `duh` generates a minimal POSIX snippet containing only your ssh-safe entries.
2. The snippet is written locally to a `0600` temp file, then `scp`'d to
   `~/.duh_session.sh` on the remote.
3. `duh` runs `ssh -t <host> "bash --rcfile ~/.duh_session.sh -i"`, so the
   snippet is sourced into your interactive remote shell.
4. With `--cleanup`, the remote snippet is `rm`'d when the session ends.

The host argument and any extra `ssh` args (after `--`) are passed as discrete
arguments — never interpolated into a shell string — so a hostname can't inject
commands.

```sh
duh ssh user@host --cleanup -- -p 2222 -i ~/.ssh/key
```

## Per-host configuration

Optional `~/.config/duh/ssh.toml` (see `duh where` for the exact path) lets you
scope which packages a host sees and whether functions are allowed:

```toml
[hosts."user@host"]
packages = ["default", "work"]   # restrict to these packages
inject_functions = true           # allow function bodies for this host (off by default)
```

Even with `packages` set, only aliases/exports flagged `--ssh-safe` within those
packages are shipped.

> ⚠️ **`inject_functions=true` ships ALL function bodies in the selected packages,
> unfiltered** — the ssh-safe allowlist does **not** apply to functions. Function
> files are arbitrary shell code, so only enable this for hosts and packages you
> fully trust. duh prints a warning when you connect with it on. Leave it off
> (the default) and your SSH sessions only ever receive flagged aliases/exports.

## Managing flags

- Flag while adding: `duh add alias g "git" --ssh-safe`
- See what's flagged: `duh ls` shows an `[ssh-safe]` marker next to each entry.
- Unflag: remove and re-add without `--ssh-safe`, or `duh rm alias g` (removing
  an entry also clears its ssh-safe flag).

## Security notes

- Allowlist, not blocklist: forgetting to flag something fails *safe* (it isn't sent).
- Values are single-quoted in the generated snippet, so no expansion happens on
  the remote.
- The local temp file is created `0600`; the remote file lands in your home dir
  and can be auto-removed with `--cleanup`.
