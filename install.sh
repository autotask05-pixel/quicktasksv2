#!/usr/bin/env bash
set -euo pipefail

if ! command -v curl >/dev/null 2>&1; then
  echo "curl is required" >&2
  exit 1
fi

if ! command -v tar >/dev/null 2>&1; then
  echo "tar is required" >&2
  exit 1
fi

REPO="${1:-${EFFICEINTNLP_REPO:-}}"
VERSION="${2:-${EFFICEINTNLP_VERSION:-latest}}"
VARIANT="${3:-${EFFICEINTNLP_VARIANT:-default}}"

if [ -z "$REPO" ]; then
  echo "Usage: curl -fsSL https://raw.githubusercontent.com/OWNER/REPO/main/install.sh | bash -s -- OWNER/REPO [version] [variant]" >&2
  exit 1
fi

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) OS_TAG="unknown-linux-gnu" ;;
  Darwin) OS_TAG="apple-darwin" ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64) ARCH_TAG="x86_64" ;;
  arm64|aarch64) ARCH_TAG="aarch64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

case "$VARIANT" in
  default|fire-and-forget|default-ui|fire-and-forget-ui) ;;
  *)
    echo "Unsupported variant: $VARIANT" >&2
    echo "Supported variants: default, fire-and-forget, default-ui, fire-and-forget-ui" >&2
    exit 1
    ;;
esac

ARTIFACT="quicktasks-${VARIANT}-${ARCH_TAG}-${OS_TAG}.tar.gz"
if [ "$OS_TAG" = "unknown-linux-gnu" ] && [ "$ARCH_TAG" = "aarch64" ]; then
  echo "No published Linux aarch64 archive is configured in the current workflow." >&2
  exit 1
fi

if [ "$VERSION" = "latest" ]; then
  DOWNLOAD_URL="https://github.com/${REPO}/releases/latest/download/${ARTIFACT}"
else
  DOWNLOAD_URL="https://github.com/${REPO}/releases/download/${VERSION}/${ARTIFACT}"
fi

INSTALL_ROOT="${EFFICEINTNLP_INSTALL_DIR:-$HOME/.local/share/quicktasks}"
BIN_DIR="${EFFICEINTNLP_BIN_DIR:-$HOME/.local/bin}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

mkdir -p "$INSTALL_ROOT" "$BIN_DIR"

echo "Downloading ${DOWNLOAD_URL}"
if ! curl -fL "$DOWNLOAD_URL" -o "$TMP_DIR/archive.tar.gz"; then
  echo "Failed to download ${DOWNLOAD_URL}" >&2
  echo "This installer only works with published GitHub release assets." >&2
  echo "GitHub Actions run artifacts are not available at releases/latest/download/..." >&2
  echo "Create or use a tagged release that includes ${ARTIFACT}, then retry." >&2
  exit 1
fi

rm -rf "$INSTALL_ROOT"/*
tar -xzf "$TMP_DIR/archive.tar.gz" -C "$INSTALL_ROOT"

chmod +x "$INSTALL_ROOT/quicktasks"
ln -sf "$INSTALL_ROOT/quicktasks" "$BIN_DIR/quicktasks"

cat <<EOF
Installed to: $INSTALL_ROOT
Binary link:   $BIN_DIR/quicktasks
Variant:       $VARIANT

Next:
  1. Ensure $BIN_DIR is in your PATH
  2. Run: quicktasks

The first startup downloads default models if they are missing.
EOF
