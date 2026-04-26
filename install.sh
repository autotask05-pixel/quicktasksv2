#!/usr/bin/env bash
set -e

REPO="autotask05-pixel/quicktasksv2"
BINARY_NAME="quicktasks"

INPUT_VARIANT="${1:-ui}"

case "$INPUT_VARIANT" in
  ui) VARIANT="ui" ;;
  noui) VARIANT="default" ;;
  fire-and-forget) VARIANT="fire-and-forget" ;;
  fire-and-forget-ui) VARIANT="fire-and-forget-ui" ;;
  *) echo "❌ Invalid variant"; exit 1 ;;
esac

echo "🎯 Variant: $INPUT_VARIANT → $VARIANT"

# -------- OS --------
OS="$(uname -s)"
case "$OS" in
  Linux) OS_TYPE="unknown-linux-gnu" ;;
  Darwin) OS_TYPE="apple-darwin" ;;
  *) echo "❌ Unsupported OS"; exit 1 ;;
esac

# -------- ARCH --------
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_TYPE="x86_64" ;;
  aarch64|arm64) ARCH_TYPE="aarch64" ;;
  *) echo "❌ Unsupported ARCH"; exit 1 ;;
esac

TARGET="${ARCH_TYPE}-${OS_TYPE}"
FILE="${BINARY_NAME}-${VARIANT}-${TARGET}.tar.gz"

URL="https://github.com/${REPO}/releases/latest/download/${FILE}"

echo "⬇️ $URL"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

curl -fL "$URL" -o pkg.tar.gz
tar -xzf pkg.tar.gz

chmod +x $BINARY_NAME

# -------- INSTALL BINARY --------
INSTALL_PATH="/usr/local/bin/$BINARY_NAME"

if [ -w "/usr/local/bin" ]; then
  mv $BINARY_NAME "$INSTALL_PATH"
else
  sudo mv $BINARY_NAME "$INSTALL_PATH"
fi

echo "⚙️ Installed binary → $INSTALL_PATH"

# -------- INSTALL data.json (repo version) --------
DATA_URL="https://raw.githubusercontent.com/${REPO}/main/data.json"

# 1. Try placing next to binary
BIN_DIR="/usr/local/bin"
DATA_TARGET="$BIN_DIR/data.json"

echo "📦 Installing data.json → $DATA_TARGET"

if [ -w "$BIN_DIR" ]; then
  curl -fsSL "$DATA_URL" -o "$DATA_TARGET"
else
  sudo curl -fsSL "$DATA_URL" -o "$DATA_TARGET"
fi

# 2. Also place user copy (fallback)
USER_DATA_DIR="$HOME/.quicktasks"
mkdir -p "$USER_DATA_DIR"

if [ ! -f "$USER_DATA_DIR/data.json" ]; then
  echo "📦 Installing fallback data.json → $USER_DATA_DIR"
  curl -fsSL "$DATA_URL" -o "$USER_DATA_DIR/data.json"
fi

cd ~
rm -rf "$TMP_DIR"

echo "✅ Install complete"
echo "👉 Run: quicktasks"
