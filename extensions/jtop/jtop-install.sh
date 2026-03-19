#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "============================================"
echo "Installing jetson-stats (jtop) into extension"
echo "============================================"

BUILD_DIR="${AVOCADO_BUILD_DIR}/jtop-build"

if [ ! -d "$BUILD_DIR" ]; then
    echo "Error: Build directory not found at $BUILD_DIR"
    exit 1
fi

# Install Python packages
install -d "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/python3/dist-packages"
cp -r "$BUILD_DIR"/* "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/python3/dist-packages/"

# Install jtop binary
install -d "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin"
install -m 755 "$BUILD_DIR/bin/jtop" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/jtop"

echo "jetson-stats installed successfully"
