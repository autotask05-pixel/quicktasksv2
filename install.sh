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

echo "⚙️ Installing to $INSTALL_BASE"

if [ -w "/usr/local/lib" ]; then
  rm -rf "$INSTALL_BASE"
  mkdir -p "$INSTALL_BASE"
  cp -r ./* "$INSTALL_BASE/"
else
  sudo rm -rf "$INSTALL_BASE"
  sudo mkdir -p "$INSTALL_BASE"
  sudo cp -r ./* "$INSTALL_BASE/"
fi

# Ensure binary executable
chmod +x "$INSTALL_BASE/quicktasks"

# ================= SYMLINK =================
echo "🔗 Linking binary to $BIN_LINK"

if [ -w "/usr/local/bin" ]; then
  ln -sf "$INSTALL_BASE/quicktasks" "$BIN_LINK"
else
  sudo ln -sf "$INSTALL_BASE/quicktasks" "$BIN_LINK"
fi

# ================= CLEANUP =================
cd ~
rm -rf "$TMP_DIR"

echo ""
echo "✅ Installation successful!"
echo "📂 Files installed at: $INSTALL_BASE"
echo "👉 Run: quicktasks"
