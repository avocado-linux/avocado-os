#!/bin/bash
set -e

echo "Cleaning avocado-conn build artifacts"

cd "$(dirname "$0")/avocado-conn"

cargo clean --target-dir "$AVOCADO_BUILD_DIR"

echo "avocado-conn cleaned successfully"
