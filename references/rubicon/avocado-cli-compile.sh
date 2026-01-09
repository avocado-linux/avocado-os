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

if [ -z "$RUST_TARGET" ]; then
    echo "Error: Could not find Rust target for $OECORE_TARGET_ARCH"
    exit 1
fi

echo "Building avocado-cli for target: $RUST_TARGET"

cd avocado-cli

# Clear any rustflags that might cause conflicts
unset RUSTFLAGS
unset CARGO_TARGET_AARCH64_AVOCADO_LINUX_GNU_RUSTFLAGS
unset CARGO_BUILD_RUSTFLAGS

# Remove any existing config that might conflict
rm -rf .cargo

# Create config.toml with cross-compilation settings
mkdir -p .cargo
cat > .cargo/config.toml << EOF
[target.$RUST_TARGET]
rustflags = ["--sysroot=$SDKTARGETSYSROOT/usr", "-C", "link-arg=--sysroot=$SDKTARGETSYSROOT"]
EOF

cargo build --release --target "$RUST_TARGET"
