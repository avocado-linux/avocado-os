#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing Python app into extension"

# Copy pip packages to a dedicated app directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/app/packages"
cp -r app/packages/* "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/app/packages/"

echo "Python app installed successfully"
