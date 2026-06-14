#!/bin/sh
# duh installer — download the right prebuilt binary for your OS/arch.
#
#   curl -sSL https://raw.githubusercontent.com/Fabbbou/duh-rust/main/install.sh | sh
#
# Env overrides:
#   DUH_INSTALL_DIR   install location (default: ~/.local/bin, or /usr/local/bin as root)
#   DUH_VERSION       specific tag to install (default: latest release)

set -eu

REPO="Fabbbou/duh-rust"
BIN="duh"

info() { printf '\033[1;34m=>\033[0m %s\n' "$1"; }
err()  { printf '\033[1;31merror:\033[0m %s\n' "$1" >&2; exit 1; }

# --- detect platform -------------------------------------------------------
os="$(uname -s)"
arch="$(uname -m)"

case "$os" in
  Linux)  os_target="unknown-linux-gnu" ;;
  Darwin) os_target="apple-darwin" ;;
  *) err "unsupported OS: $os (Linux, macOS, and WSL are supported)" ;;
esac

case "$arch" in
  x86_64|amd64)  arch_target="x86_64" ;;
  aarch64|arm64) arch_target="aarch64" ;;
  *) err "unsupported architecture: $arch" ;;
esac

target="${arch_target}-${os_target}"

# --- resolve version -------------------------------------------------------
if [ "${DUH_VERSION:-}" = "" ]; then
  info "Resolving latest release..."
  # GitHub returns single-line minified JSON, so extract just the tag_name token
  # (grep -o) rather than splitting the whole line on quotes.
  DUH_VERSION="$(
    curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
      | grep -o '"tag_name"[ ]*:[ ]*"[^"]*"' | head -n1 \
      | sed -E 's/.*"tag_name"[ ]*:[ ]*"([^"]+)".*/\1/'
  )"
  [ -n "$DUH_VERSION" ] || err "could not determine latest version"
fi
info "Installing duh ${DUH_VERSION} for ${target}"

# --- download + verify -----------------------------------------------------
asset="duh-${target}.tar.gz"
base="https://github.com/${REPO}/releases/download/${DUH_VERSION}"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

info "Downloading ${asset}..."
curl -fsSL "${base}/${asset}" -o "${tmp}/${asset}" \
  || err "download failed: ${base}/${asset}"

# Checksum is published as `duh-<target>.sha256` (not `<asset>.sha256`).
sha="duh-${target}.sha256"
info "Verifying checksum..."
curl -fsSL "${base}/${sha}" -o "${tmp}/${sha}" \
  || err "checksum download failed: ${base}/${sha}"
expected="$(cut -d' ' -f1 < "${tmp}/${sha}")"
if command -v sha256sum >/dev/null 2>&1; then
  actual="$(sha256sum "${tmp}/${asset}" | cut -d' ' -f1)"
else
  actual="$(shasum -a 256 "${tmp}/${asset}" | cut -d' ' -f1)"
fi
[ "$expected" = "$actual" ] || err "checksum mismatch (expected $expected, got $actual)"

# --- install ---------------------------------------------------------------
tar -xzf "${tmp}/${asset}" -C "$tmp"

if [ "${DUH_INSTALL_DIR:-}" = "" ]; then
  if [ "$(id -u)" = "0" ]; then
    DUH_INSTALL_DIR="/usr/local/bin"
  else
    DUH_INSTALL_DIR="${HOME}/.local/bin"
  fi
fi
mkdir -p "$DUH_INSTALL_DIR"
install -m 0755 "${tmp}/${BIN}" "${DUH_INSTALL_DIR}/${BIN}" 2>/dev/null \
  || { cp "${tmp}/${BIN}" "${DUH_INSTALL_DIR}/${BIN}" && chmod 0755 "${DUH_INSTALL_DIR}/${BIN}"; }

info "Installed ${BIN} to ${DUH_INSTALL_DIR}/${BIN}"

# --- post-install hints ----------------------------------------------------
case ":${PATH}:" in
  *":${DUH_INSTALL_DIR}:"*) ;;
  *) printf '\n\033[1;33mnote:\033[0m %s is not on your PATH. Add:\n  export PATH="%s:$PATH"\n' \
        "$DUH_INSTALL_DIR" "$DUH_INSTALL_DIR" ;;
esac

cat <<'EOF'

Next steps — wire duh into your shell:

  bash:  echo 'eval "$(duh init --shell bash)"' >> ~/.bashrc
  zsh:   echo 'eval "$(duh init --shell zsh)"'  >> ~/.zshrc

Then restart your shell and try:

  duh add alias ll "ls -al"
  duh ls

EOF
