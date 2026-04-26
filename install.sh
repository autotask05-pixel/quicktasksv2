#!/usr/bin/env bash
set -e

REPO="autotask05-pixel/quicktasksv2"
BINARY_NAME="quicktasks"

echo "===== QuickTasks Installer ====="

# -------- Variant Mapping --------
INPUT_VARIANT="${1:-ui}"

case "$INPUT_VARIANT" in
  ui) VARIANT="ui" ;;
  noui) VARIANT="default" ;;
  fire-and-forget) VARIANT="fire-and-forget" ;;
  fire-and-forget-ui) VARIANT="fire-and-forget-ui" ;;
  *)
    echo "❌ Invalid variant: $INPUT_VARIANT"
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
    echo "❌ Unsupported OS: $OS"
    exit 1
    ;;
esac

# -------- ARCH --------
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ARCH_TYPE="x86_64" ;;
  aarch64|arm64) ARCH_TYPE="aarch64" ;;
  *)
    echo "❌ Unsupported architecture: $ARCH"
    exit 1
    ;;
esac

TARGET="${ARCH_TYPE}-${OS_TYPE}"
EXT="tar.gz"

FILE="${BINARY_NAME}-${VARIANT}-${TARGET}.${EXT}"
URL="https://github.com/${REPO}/releases/latest/download/${FILE}"

echo "⬇️ Downloading: $URL"

TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

curl -fL "$URL" -o package.tar.gz

echo "📦 Extracting..."
tar -xzf package.tar.gz

chmod +x $BINARY_NAME

# -------- Install binary --------
INSTALL_PATH="/usr/local/bin/$BINARY_NAME"

echo "⚙️ Installing binary → $INSTALL_PATH"

if [ -w "/usr/local/bin" ]; then
  mv $BINARY_NAME "$INSTALL_PATH"
else
  sudo mv $BINARY_NAME "$INSTALL_PATH"
fi

# -------- Setup data --------
DATA_DIR="$HOME/.quicktasks"
DATA_FILE="$DATA_DIR/data.json"

echo "📁 Setting up data directory → $DATA_DIR"
mkdir -p "$DATA_DIR"

if [ ! -f "$DATA_FILE" ]; then
  echo "⬇️ Downloading default data.json"
  curl -fsSL "https://raw.githubusercontent.com/${REPO}/main/data.json" -o "$DATA_FILE"
else
  echo "✅ Existing data.json found (not overwriting)"
fi

cd ~
rm -rf "$TMP_DIR"

echo ""
echo "✅ Installed successfully!"
echo "👉 Run: quicktasks"
