#!/bin/bash
set -e

echo "Cleaning avocado-container-agent-dev build artifacts"

cd "$(dirname "$0")/agent"

cargo clean --target-dir "$AVOCADO_BUILD_DIR"

echo "avocado-container-agent-dev cleaned successfully"
