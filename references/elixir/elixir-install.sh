#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing elixir application into extension"

# Create the target directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-elixir"

# Copy the built Elixir application files
cp -r ref-elixir/_build/prod/rel/ref_elixir/* "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-elixir/"

echo "Elixir application installed successfully"
