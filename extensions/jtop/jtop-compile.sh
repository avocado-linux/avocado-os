#!/usr/bin/env bash
set -e

echo "============================================"
echo "Building jetson-stats (jtop)"
echo "============================================"

pip3 install --target="${AVOCADO_BUILD_DIR}/jtop-build" jetson-stats

echo "jetson-stats compiled successfully"
