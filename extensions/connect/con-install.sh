#!/bin/bash
set -e

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

BINARY_PATH="$AVOCADO_BUILD_DIR/$RUST_TARGET/release/avocado-conn"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Installing avocado-conn into extension"
install -D -m 755 "$BINARY_PATH" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/avocado-conn"
echo "avocado-conn installed successfully"
