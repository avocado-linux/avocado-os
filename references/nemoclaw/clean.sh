#!/bin/sh
set -e

echo "Cleaning NemoClaw build artifacts..."
rm -rf "$AVOCADO_BUILD_DIR/NemoClaw"
rm -rf "$AVOCADO_BUILD_DIR/bin"
rm -rf "$AVOCADO_BUILD_DIR/lib"
