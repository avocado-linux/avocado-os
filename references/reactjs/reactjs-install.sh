#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing React.js application into extension"

# Create the target directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs"

# Copy the built application files
cp -r ref-reactjs/dist "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs/"
cp -r ref-reactjs/node_modules "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs/"
cp ref-reactjs/package.json "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs/"
cp ref-reactjs/server.js "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs/"

echo "React.js application installed successfully"
