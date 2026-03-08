#!/usr/bin/env bash
set -euo pipefail

REPO="femboi/query.rs"
BIN="query"
INSTALL_DIR="${HOME}/.local/bin"

ARCH=$(uname -m)
case $ARCH in
  x86_64)   TARGET="x86_64-unknown-linux-musl" ;;
  aarch64)  TARGET="aarch64-unknown-linux-musl" ;;
  *)        echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Detected architecture: $ARCH ($TARGET)"

# Fetch the latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | head -1 | cut -d'"' -f4)

echo "Installing ${BIN} ${LATEST}..."

URL="https://github.com/${REPO}/releases/download/${LATEST}/${BIN}-${TARGET}"

mkdir -p "${INSTALL_DIR}"
curl -fsSL "${URL}" -o "${INSTALL_DIR}/${BIN}"
chmod +x "${INSTALL_DIR}/${BIN}"

echo "Installed to ${INSTALL_DIR}/${BIN}"
echo "Make sure ${INSTALL_DIR} is in your PATH."
