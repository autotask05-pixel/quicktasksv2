#!/usr/bin/env bash
set -e

REPO="autotask05-pixel/quicktasksv2"
BINARY_NAME="quicktasks"

# -------- Variant Mapping --------
INPUT_VARIANT="${1:-ui}"

case "$INPUT_VARIANT" in
  ui) VARIANT="ui" ;;
  noui) VARIANT="default" ;;
  fire-and-forget) VARIANT="fire-and-forget" ;;
  fire-and-forget-ui) VARIANT="fire-and-forget-ui" ;;
  *)
    echo " Invalid variant: $INPUT_VARIANT"
    echo "Valid: ui (default), noui, fire-and-forget, fire-and-forget-ui"
    exit 1
    ;;
esac

echo "🎯 Variant: $INPUT_VARIANT → $VARIANT"

# -------- OS --------
OS="$(uname -s)"
case "$OS" in
  Linux) OS_TYPE="unknown-linux-gnu" ;;
  Darwin) OS_TYPE="apple-darwin" ;;
  *)
    echo " Unsupported OS: $OS"
    exit 1
    ;;
esac

# -------- ARCH --------
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_TYPE="x86_64" ;;
  aarch64|arm64) ARCH_TYPE="aarch64" ;;
  *)
    echo " Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

TARGET="${ARCH_TYPE}-${OS_TYPE}"
EXT="tar.gz"

echo "📦 Target: $TARGET"

# -------- VERSION --------
VERSION=$(curl -s https://api.github.com/repos/$REPO/releases/latest | grep tag_name | cut -d '"' -f 4)

[ -z "$VERSION" ] && { echo "❌ Failed to fetch version"; exit 1; }

FILE="${BINARY_NAME}-${VARIANT}-${TARGET}.${EXT}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${FILE}"

echo " $URL"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

curl -L "$URL" -o pkg.tar.gz
tar -xzf pkg.tar.gz

chmod +x $BINARY_NAME

INSTALL_PATH="/usr/local/bin/$BINARY_NAME"

if [ -w "/usr/local/bin" ]; then
  mv $BINARY_NAME "$INSTALL_PATH"
else
  sudo mv $BINARY_NAME "$INSTALL_PATH"
fi

cd ~ && rm -rf "$TMP_DIR"

echo " Installed → $BINARY_NAME"
