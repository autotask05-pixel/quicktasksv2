#!/usr/bin/env bash
set -e

echo "===== QuickTasks Installer ====="

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

OS="$(uname -s)"
case "$OS" in
  Linux) OS_TYPE="unknown-linux-gnu" ;;
  Darwin) OS_TYPE="apple-darwin" ;;
  *) echo "❌ Unsupported OS"; exit 1 ;;
esac

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_TYPE="x86_64" ;;
  aarch64|arm64) ARCH_TYPE="aarch64" ;;
  *) echo "❌ Unsupported ARCH"; exit 1 ;;
esac

TARGET="${ARCH_TYPE}-${OS_TYPE}"
FILE="${BINARY_NAME}-${VARIANT}-${TARGET}.tar.gz"

URL="https://github.com/${REPO}/releases/latest/download/${FILE}"
DATA_URL="https://raw.githubusercontent.com/${REPO}/main/data.json"

echo "⬇️ Downloading binary..."
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

curl -fL "$URL" -o pkg.tar.gz
tar -xzf pkg.tar.gz

chmod +x $BINARY_NAME

INSTALL_DIR="/usr/local/bin"

echo "⚙️ Installing binary → $INSTALL_DIR"

if [ -w "$INSTALL_DIR" ]; then
  mv $BINARY_NAME "$INSTALL_DIR/"
else
  sudo mv $BINARY_NAME "$INSTALL_DIR/"
fi

echo "📦 Installing data.json → $INSTALL_DIR/data.json"

if [ -w "$INSTALL_DIR" ]; then
  curl -fsSL "$DATA_URL" -o "$INSTALL_DIR/data.json"
else
  sudo curl -fsSL "$DATA_URL" -o "$INSTALL_DIR/data.json"
fi

cd ~
rm -rf "$TMP_DIR"

echo "✅ Installed successfully!"
echo "👉 Run: quicktasks"
