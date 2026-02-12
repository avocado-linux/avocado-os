#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing rust application into extension"

# Create the target directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin"

# Copy the built Rust binary
cp ref-rust/target/release/ref_rust "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/ref_rust"

echo "Rust application installed successfully"
