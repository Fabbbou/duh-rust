#!/bin/sh
# duh uninstaller — remove the binary and (with confirmation) local packages.
#
#   curl -sSL https://raw.githubusercontent.com/Fabbbou/duh-rust/main/uninstall.sh | sh
#
# Env overrides:
#   DUH_PURGE=1   delete packages + config without prompting
#   DUH_KEEP=1    keep packages + config without prompting

set -eu

info() { printf '\033[1;34m=>\033[0m %s\n' "$1"; }

# --- locate binary ---------------------------------------------------------
if command -v duh >/dev/null 2>&1; then
  BIN="$(command -v duh)"
else
  for d in "${HOME}/.local/bin" /usr/local/bin; do
    [ -x "${d}/duh" ] && BIN="${d}/duh" && break
  done
fi

# --- resolve XDG data/config/cache dirs (matches `directories` net.fabou.duh) ---
DATA="${XDG_DATA_HOME:-$HOME/.local/share}/duh"
CONFIG="${XDG_CONFIG_HOME:-$HOME/.config}/duh"
CACHE="${XDG_CACHE_HOME:-$HOME/.cache}/duh"
case "$(uname -s)" in
  Darwin)
    DATA="$HOME/Library/Application Support/net.fabou.duh"
    CONFIG="$HOME/Library/Application Support/net.fabou.duh"
    CACHE="$HOME/Library/Caches/net.fabou.duh"
    ;;
esac

# --- decide whether to delete packages -------------------------------------
delete_data=0
if [ "${DUH_PURGE:-}" = "1" ]; then
  delete_data=1
elif [ "${DUH_KEEP:-}" = "1" ]; then
  delete_data=0
elif [ -t 0 ]; then
  printf 'Also delete all local packages and config?\n  %s\n  %s\n[y/N] ' "$DATA" "$CONFIG"
  read -r ans
  case "$ans" in [Yy]*) delete_data=1 ;; esac
else
  # Non-interactive (piped) and no override → keep data (safe default).
  info "Non-interactive: keeping packages (set DUH_PURGE=1 to delete them)."
fi

# --- remove ----------------------------------------------------------------
rm -rf "$CACHE" && info "removed cache ($CACHE)"

if [ "$delete_data" = "1" ]; then
  rm -rf "$DATA" "$CONFIG"
  info "removed packages + config"
else
  info "kept packages + config ($DATA)"
fi

if [ -n "${BIN:-}" ] && [ -e "$BIN" ]; then
  rm -f "$BIN" && info "removed binary ($BIN)"
else
  info "binary not found on PATH; nothing to remove"
fi

cat <<'EOF'

duh uninstalled. Remove the duh line from your shell rc:
  ~/.bashrc / ~/.zshrc:  eval "$(duh init ...)"
EOF
