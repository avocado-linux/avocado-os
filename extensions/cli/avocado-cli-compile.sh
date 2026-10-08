#!/bin/bash
set -e

# Find the Rust target from RUST_TARGET_PATH
# Enumerate every candidate rather than stopping at the first. The SDK ships its
# own nativesdk triple beside the device one, so on any target whose architecture
# matches the SDK host's this prefix matches twice - x86_64-avocado-linux-gnu and
# x86_64-avocadosdk-linux-gnu. Taking the first match picked the device triple
# only because "avocado-" sorts ahead of "avocadosdk-" on the hyphen, which
# nothing enforces.
match_count=0
matches=""
for json_file in "$RUST_TARGET_PATH"/*.json; do
    if [ -f "$json_file" ]; then
        json_name=$(basename "$json_file" .json)

        # The avocadosdk vendor is host-side by construction - avocado-cli tags
        # every nativesdk artifact <arch>_avocadosdk - so it is never a device
        # target and must not count as a competing match.
        case "$json_name" in
        "${OECORE_TARGET_ARCH}-avocadosdk-"*) continue ;;
        esac

        if [[ "$json_name" == "${OECORE_TARGET_ARCH}-"* ]]; then
            RUST_TARGET="$json_name"
            match_count=$((match_count + 1))
            matches="$matches $json_name"
        fi
    fi
done

if [ "$match_count" -gt 1 ]; then
    echo "Error: $OECORE_TARGET_ARCH matches $match_count Rust targets:$matches" >&2
    echo "Error: refusing to pick one - the wrong triple yields a binary that packages and installs but cannot exec on the device." >&2
    exit 1
fi

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
