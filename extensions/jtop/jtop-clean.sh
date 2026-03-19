#!/usr/bin/env bash
set -e

echo "Cleaning jetson-stats build artifacts"

rm -rf "${AVOCADO_BUILD_DIR}/jtop-build"

echo "jetson-stats cleaned successfully"
