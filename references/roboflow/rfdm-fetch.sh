#!/bin/bash

set -e

# Get the directory where this script is located
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Get the latest version
VERSION=$(curl -fsSL https://repo.roboflow.com/rfdm/latest/LATEST_VERSION.txt)

if [ -z "$VERSION" ]; then
    echo "Error: Failed to retrieve latest version"
    exit 1
fi

echo "Downloading rfdm version: $VERSION"

# Create build directory if it doesn't exist
mkdir -p "${SCRIPT_DIR}/_build"

# Download the binary
curl -fsSL -o "${SCRIPT_DIR}/_build/rfdm" "https://repo.roboflow.com/rfdm/${VERSION}/rfdm/linux-arm64"

# Make it executable
chmod +x "${SCRIPT_DIR}/_build/rfdm"

echo "Successfully downloaded rfdm binary to ${SCRIPT_DIR}/_build/rfdm"

