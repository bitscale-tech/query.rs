#!/usr/bin/env bash
set -euo pipefail

REPO="bitscale-tech/query.rs"
BIN="query-rs"
INSTALL_DIR="${HOME}/.local/bin"

ARCH=$(uname -m)
case $ARCH in
  x86_64)   TARGET="query-rs-x86_64-linux" ;;
  aarch64)  TARGET="query-rs-aarch64-linux" ;;
  *)        echo "Unsupported architecture: $ARCH"; exit 1 ;;
esac

echo "Detected architecture: $ARCH ($TARGET)"

# Fetch the latest release tag
LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | head -1 | cut -d'"' -f4)

echo "Installing ${BIN} ${LATEST}..."

URL="https://github.com/${REPO}/releases/download/${LATEST}/${TARGET}"

mkdir -p "${INSTALL_DIR}"
curl -fsSL "${URL}" -o "${INSTALL_DIR}/${BIN}"
chmod +x "${INSTALL_DIR}/${BIN}"

echo "Installed to ${INSTALL_DIR}/${BIN}"
echo "Make sure ${INSTALL_DIR} is in your PATH."
