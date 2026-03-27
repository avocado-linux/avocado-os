#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing gpio-toggle into extension"

DESTDIR="$AVOCADO_BUILD_EXT_SYSROOT" ninja -C app/src/build install

echo "gpio-toggle installed successfully"
