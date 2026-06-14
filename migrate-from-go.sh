#!/bin/sh
# Migrate from the old Go `duh` to the new Rust `duh`.
#
# Steps:
#   1. Uninstall the old Go duh executable (detected via its `self` subcommand,
#      which the new Rust duh lacks — the new binary is never touched).
#   2. Migrate packages + preferences (see below).
#
# What changes between versions:
#   - Packages (db.toml + functions/*.sh) are the SAME format → copied as-is.
#   - Preferences move + rename:
#       old: <data>/user_preferences.toml  [repositories] activated_repos / default_repo_name
#       new: <config>/prefs.toml           [packages]     enabled        / default
#   - Per-package `gitconfig` files are NOT used by the new duh → left in place,
#     a warning is printed (re-add git aliases to your ~/.gitconfig manually).
#
# Safe by default: never overwrites an existing package or prefs.toml without
# asking, and makes a .bak of anything it replaces. Run it, read the summary.
#
#   sh migrate-from-go.sh            # interactive
#   DUH_MIGRATE_FORCE=1 sh migrate-from-go.sh   # overwrite without prompts

set -eu

info()  { printf '\033[1;34m=>\033[0m %s\n' "$1"; }
warn()  { printf '\033[1;33mwarning:\033[0m %s\n' "$1" >&2; }
err()   { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

FORCE="${DUH_MIGRATE_FORCE:-0}"

ask() { # ask "question" -> 0 (yes) / 1 (no); auto-yes under FORCE / non-tty
  [ "$FORCE" = "1" ] && return 0
  [ -t 0 ] || return 1
  printf '%s [y/N] ' "$1"; read -r a; case "$a" in [Yy]*) return 0;; *) return 1;; esac
}

# --- old (Go) layout: adrg/xdg → XDG data dir on every platform -------------
OLD_DATA="${XDG_DATA_HOME:-$HOME/.local/share}/duh"
OLD_PKGS="$OLD_DATA/packages"
OLD_PREFS="$OLD_DATA/user_preferences.toml"

# --- new (Rust) layout: `directories` crate (net.fabou.duh) -----------------
case "$(uname -s)" in
  Darwin)
    NEW_DATA="$HOME/Library/Application Support/net.fabou.duh"
    NEW_CONFIG="$HOME/Library/Application Support/net.fabou.duh"
    ;;
  *)
    NEW_DATA="${XDG_DATA_HOME:-$HOME/.local/share}/duh"
    NEW_CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/duh"
    ;;
esac
NEW_PKGS="$NEW_DATA/packages"
NEW_PREFS="$NEW_CONFIG/prefs.toml"

[ -d "$OLD_PKGS" ] || err "no old duh packages found at $OLD_PKGS"

# --- uninstall the old Go duh executable -----------------------------------
# The old Go duh has a `self` subcommand; the new Rust duh does not. Probe it
# so we only ever remove the OLD binary, never the new one.
remove_old_binary() {
  command -v duh >/dev/null 2>&1 || { info "no 'duh' on PATH — nothing to uninstall"; return; }
  bin="$(command -v duh)"
  if ! duh self version >/dev/null 2>&1 && ! duh self config-path >/dev/null 2>&1; then
    info "'duh' on PATH ($bin) is the new Rust version — leaving it in place"
    return
  fi
  if ask "Found old Go duh at $bin — remove it?"; then
    if rm -f "$bin" 2>/dev/null; then
      info "removed old duh binary ($bin)"
    else
      warn "could not remove $bin (try with sudo) — delete it manually"
    fi
  else
    warn "kept old duh binary at $bin — note the new install must shadow it on PATH"
  fi
}
remove_old_binary
echo

info "Old packages: $OLD_PKGS"
info "New packages: $NEW_PKGS"
info "New prefs:    $NEW_PREFS"
echo

mkdir -p "$NEW_PKGS" "$NEW_CONFIG"

# --- copy packages ---------------------------------------------------------
copied=0; skipped=0; gitcfg=0
for src in "$OLD_PKGS"/*/; do
  [ -d "$src" ] || continue
  name="$(basename "$src")"
  dest="$NEW_PKGS/$name"

  if [ "$src" = "$dest/" ] || [ "$(cd "$src" && pwd)" = "$(cd "$dest" 2>/dev/null && pwd || echo /nonexistent)" ]; then
    info "package '$name' already in place (same path) — skipping copy"
    skipped=$((skipped+1))
  elif [ -e "$dest" ] && ! ask "package '$name' exists at destination — overwrite?"; then
    info "package '$name' kept (not overwritten)"
    skipped=$((skipped+1))
  else
    [ -e "$dest" ] && mv "$dest" "$dest.bak.$(date +%s)"
    mkdir -p "$dest"
    [ -f "$src/db.toml" ] && cp "$src/db.toml" "$dest/db.toml"
    [ -d "$src/functions" ] && cp -R "$src/functions" "$dest/functions"
    info "migrated package '$name'"
    copied=$((copied+1))
  fi

  if [ -f "$src/gitconfig" ]; then
    warn "package '$name' has a gitconfig (git aliases) — not supported by the new duh; left at $src/gitconfig"
    gitcfg=$((gitcfg+1))
  fi
done

# --- convert preferences ---------------------------------------------------
enabled_line='enabled = ["default"]'
default_line='default = "default"'

if [ -f "$OLD_PREFS" ]; then
  # default_repo_name = "x"  ->  default = "x"
  d="$(sed -n 's/^[[:space:]]*default_repo_name[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p' "$OLD_PREFS" | head -n1)"
  [ -n "${d:-}" ] && default_line="default = \"$d\""

  # activated_repos = ["a", "b"]  ->  enabled = ["a", "b"]  (single-line arrays)
  arr="$(sed -n 's/^[[:space:]]*activated_repos[[:space:]]*=[[:space:]]*\(\[.*\]\).*/\1/p' "$OLD_PREFS" | head -n1)"
  if [ -n "${arr:-}" ]; then
    enabled_line="enabled = $arr"
  else
    warn "could not parse activated_repos (multi-line array?) — review $NEW_PREFS by hand"
  fi
else
  warn "no old user_preferences.toml; writing defaults (all/default enabled)"
fi

if [ -f "$NEW_PREFS" ] && ! ask "prefs.toml already exists — overwrite?"; then
  info "kept existing prefs.toml (review it manually)"
else
  [ -f "$NEW_PREFS" ] && cp "$NEW_PREFS" "$NEW_PREFS.bak.$(date +%s)"
  {
    echo "[packages]"
    echo "$enabled_line"
    echo "$default_line"
  } > "$NEW_PREFS"
  info "wrote $NEW_PREFS"
fi

# --- summary ---------------------------------------------------------------
echo
info "Done: $copied migrated, $skipped skipped, $gitcfg gitconfig file(s) flagged."
info "Verify with:  duh pkg ls  &&  duh ls"
[ "$gitcfg" -gt 0 ] && warn "Re-add any git aliases to your ~/.gitconfig manually."
echo "Then reload your shell (or run: duh reload)."
