#!/bin/sh
set -e

echo "Installing NemoClaw CLI and Ollama into extension..."

# Install Ollama binary
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin"
install -m 0755 "$AVOCADO_BUILD_DIR/bin/ollama" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/ollama"

# Install Ollama CUDA libraries
if [ -d "$AVOCADO_BUILD_DIR/lib/ollama" ]; then
    mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ollama"
    cp -a "$AVOCADO_BUILD_DIR/lib/ollama/." "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ollama/"
fi

# Install NemoClaw CLI
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/nemoclaw"
cp -a "$AVOCADO_BUILD_DIR/NemoClaw/." "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/nemoclaw/"

# Symlink the CLI binary into PATH
ln -sf /usr/lib/nemoclaw/node_modules/.bin/nemoclaw "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/nemoclaw"

echo "NemoClaw CLI and Ollama installed successfully"
