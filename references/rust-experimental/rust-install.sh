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

BINARY_PATH="ref-rust/target/$RUST_TARGET/release/ref_rust"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Installing rust application into extension"
install -D -m 755 "$BINARY_PATH" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/ref_rust"
echo "Installed: $(file "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/ref_rust")"
