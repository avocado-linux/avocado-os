#!/bin/bash
set -euo pipefail

RUST_TARGET=""

# Find the Rust target from RUST_TARGET_PATH
for json_file in "$RUST_TARGET_PATH"/*.json; do
    if [ -f "$json_file" ]; then
        json_name=$(basename "$json_file" .json)
        if [[ "$json_name" == "${OECORE_TARGET_ARCH}-"* ]]; then
            RUST_TARGET="$json_name"
            break
        fi
    fi
done

# Same guard cad-compile.sh has. Without it an unmatched loop leaves RUST_TARGET
# empty, BINARY_PATH collapses to "$AVOCADO_BUILD_DIR//release/...", and the -f
# check below reports a missing binary instead of the actual fault.
if [ -z "$RUST_TARGET" ]; then
    echo "Error: Could not find Rust target for $OECORE_TARGET_ARCH"
    exit 1
fi

BINARY_PATH="$AVOCADO_BUILD_DIR/$RUST_TARGET/release/avocado-container-agent-dev"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Installing avocado-container-agent-dev into extension"
install -D -m 755 "$BINARY_PATH" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/avocado-container-agent-dev"
echo "avocado-container-agent-dev installed successfully"
