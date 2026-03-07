#!/usr/bin/env bash
# Pixicode CLI install script (curl | bash).
# Usage: curl -fsSL https://raw.githubusercontent.com/.../install.sh | bash -s -- [--version TAG] [--no-modify-path]
set -e

REPO="${PIXICODE_REPO:-pixibox/pixicode}"
VERSION="${PIXICODE_VERSION:-latest}"
# Install dir: PIXICODE_INSTALL_DIR > XDG_BIN_DIR > $HOME/.pixicode/bin
INSTALL_DIR="${PIXICODE_INSTALL_DIR:-${XDG_BIN_DIR:-$HOME/.pixicode/bin}}"
MODIFY_PATH=true

while [[ $# -gt 0 ]]; do
  case $1 in
    --version) VERSION="$2"; shift 2 ;;
    --no-modify-path) MODIFY_PATH=false; shift ;;
    *) shift ;;
  esac
done

mkdir -p "$INSTALL_DIR"
# Target triple to match release workflow (x86_64-apple-darwin, aarch64-unknown-linux-gnu, etc.)
ARCH=$(uname -m)
OS=$(uname -s)
case "$OS" in
  Darwin)
    case "$ARCH" in x86_64) TARGET=x86_64-apple-darwin ;; aarch64|arm64) TARGET=aarch64-apple-darwin ;; *) echo "Unsupported: $ARCH"; exit 1 ;; esac
    ;;
  Linux)
    case "$ARCH" in x86_64|amd64) TARGET=x86_64-unknown-linux-gnu ;; aarch64|arm64) TARGET=aarch64-unknown-linux-gnu ;; *) echo "Unsupported: $ARCH"; exit 1 ;; esac
    ;;
  MINGW*|MSYS*)
    TARGET=x86_64-pc-windows-msvc
    ;;
  *)
    echo "Unsupported OS: $OS"; exit 1
    ;;
esac

if [[ "$TARGET" == *windows* ]]; then
  ASSET="pixicode-${VERSION}-${TARGET}.zip"
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
  TMP=$(mktemp -d)
  trap "rm -rf $TMP" EXIT
  if command -v curl &>/dev/null; then
    curl -fsSL -o "$TMP/pkg.zip" "$URL"
  else
    wget -q -O "$TMP/pkg.zip" "$URL"
  fi
  (cd "$TMP" && unzip -o pkg.zip && mv pixicode.exe "$INSTALL_DIR/")
else
  ASSET="pixicode-${VERSION}-${TARGET}.tar.gz"
  URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
  TMP=$(mktemp -d)
  trap "rm -rf $TMP" EXIT
  if command -v curl &>/dev/null; then
    curl -fsSL -o "$TMP/pkg.tar.gz" "$URL"
  else
    wget -q -O "$TMP/pkg.tar.gz" "$URL"
  fi
  tar xzf "$TMP/pkg.tar.gz" -C "$TMP"
  mv "$TMP/pixicode" "$INSTALL_DIR/pixicode"
  chmod +x "$INSTALL_DIR/pixicode"
fi

echo "Installed to $INSTALL_DIR"
if [[ "$MODIFY_PATH" == true ]] && [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
  echo "Add to PATH: export PATH=\"$INSTALL_DIR:\$PATH\""
fi
