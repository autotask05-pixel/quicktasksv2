#!/usr/bin/env bash
set -e

REPO="autotask05-pixel/quicktasksv2"
BINARY_NAME="quicktasks"

# ================= VARIANT =================
INPUT_VARIANT="${1:-ui}"

case "$INPUT_VARIANT" in
  ui) VARIANT="ui" ;;
  noui) VARIANT="default" ;;
  fire-and-forget) VARIANT="fire-and-forget" ;;
  fire-and-forget-ui) VARIANT="fire-and-forget-ui" ;;
  *)
    echo "❌ Invalid variant: $INPUT_VARIANT"
    echo "Valid: ui (default), noui, fire-and-forget, fire-and-forget-ui"
    exit 1
    ;;
esac

echo "🎯 Variant: $INPUT_VARIANT → $VARIANT"

# ================= OS =================
OS="$(uname -s)"
case "$OS" in
  Linux) OS_TYPE="unknown-linux-gnu" ;;
  Darwin) OS_TYPE="apple-darwin" ;;
  *)
    echo "❌ Unsupported OS: $OS"
    exit 1
    ;;
esac

# ================= ARCH =================
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

echo "📦 Target: $TARGET"
echo "⬇️ Downloading: $URL"

# ================= DOWNLOAD =================
TMP_DIR=$(mktemp -d)
cd "$TMP_DIR"

curl -fL "$URL" -o package.tar.gz

echo "📦 Extracting..."
tar -xzf package.tar.gz

# ================= INSTALL =================

INSTALL_BASE="/usr/local/lib/quicktasks"
BIN_LINK="/usr/local/bin/quicktasks"

# Decide if we need sudo ONCE
if [ -w "/usr/local/lib" ] && [ -w "/usr/local/bin" ]; then
  SUDO=""
else
  SUDO="sudo"
fi

echo "⚙️ Installing to $INSTALL_BASE"

# Clean + create
$SUDO rm -rf "$INSTALL_BASE"
$SUDO mkdir -p "$INSTALL_BASE"

# Copy everything (binary + static + data.json)
$SUDO cp -r ./* "$INSTALL_BASE/"

# Ensure binary executable (WITH sudo if needed)
$SUDO chmod +x "$INSTALL_BASE/quicktasks"

# ================= SYMLINK =================
echo "🔗 Linking binary to $BIN_LINK"

$SUDO ln -sf "$INSTALL_BASE/quicktasks" "$BIN_LINK"

echo ""
echo "✅ Installation successful!"
echo "📂 Installed at: $INSTALL_BASE"
echo "👉 Run: quicktasks"
