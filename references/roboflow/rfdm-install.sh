#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "Installing rfdm binary into extension"

# Create the target directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/opt/roboflow"

# Copy the rfdm binary
cp "${SCRIPT_DIR}/_build/rfdm" "$AVOCADO_BUILD_EXT_SYSROOT/opt/roboflow/rfdm"

echo "rfdm binary installed successfully"

