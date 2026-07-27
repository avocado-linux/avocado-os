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

if [ -z "$RUST_TARGET" ]; then
    echo "Error: Could not find Rust target for $OECORE_TARGET_ARCH"
    exit 1
fi

echo "Compiling avocado-container-agent-dev for target: $RUST_TARGET"

cd "$(dirname "$0")/agent"

# Clear any rustflags that might cause conflicts with our .cargo/config.toml
unset RUSTFLAGS
unset CARGO_BUILD_RUSTFLAGS
for var in $(env | grep -o 'CARGO_TARGET_[A-Z0-9_]*_RUSTFLAGS'); do
    unset "$var"
done

# Remove any existing config that might conflict
rm -rf .cargo

# Create config.toml with cross-compilation settings
mkdir -p .cargo
cat > .cargo/config.toml << EOF
[target.$RUST_TARGET]
rustflags = ["--sysroot=$SDKTARGETSYSROOT/usr", "-C", "link-arg=--sysroot=$SDKTARGETSYSROOT"]
EOF

# Use a persistent cargo registry cache to avoid re-downloading crates
export CARGO_HOME="${AVOCADO_BUILD_DIR}/.cargo-cache"

# Enforce the ring-only rustls crypto provider before building. TARGET is
# mandatory here so the guard resolves the same SDK triple the build ships;
# without it the guard would check the host triple instead. Runs under set -e,
# so a non-zero exit (aws-lc-rs present, or a guard error) fails the compile.
TARGET="$RUST_TARGET" scripts/check-provider.sh
cargo build --release --target "$RUST_TARGET" --target-dir "$AVOCADO_BUILD_DIR"

echo "avocado-container-agent-dev compiled successfully"
