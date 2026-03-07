#!/bin/bash
set -e

echo "Cleaning avocado-rat build artifacts"

cd "$(dirname "$0")/avocado-rat"

cargo clean --target-dir "$AVOCADO_BUILD_DIR"

echo "avocado-rat cleaned successfully"
