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

BINARY_PATH="avocado-cli/target/$RUST_TARGET/release/avocado"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

install -D -m 755 "$BINARY_PATH" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/avocado"
echo "Installed: $(file "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/avocado")"
