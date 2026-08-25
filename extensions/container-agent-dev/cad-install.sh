#!/bin/bash
set -euo pipefail

RUST_TARGET=""
match_count=0
matches=""

# Resolution must stay byte-for-byte equivalent to cad-compile.sh's. The two run
# in separate invocations against the same SDK, so a divergence here silently
# installs a different triple's binary than the one that was compiled - the
# install would look successful and ship the wrong artifact. Enumerate every
# candidate rather than stopping at the first; see cad-compile.sh for why an
# ambiguous match must refuse rather than pick.
for json_file in "$RUST_TARGET_PATH"/*.json; do
    if [ -f "$json_file" ]; then
        json_name=$(basename "$json_file" .json)

        # Skip the SDK's own nativesdk triple - see cad-compile.sh for why. This
        # exclusion must stay identical in both scripts: if only one of them
        # skipped it, that script would refuse while the other resolved, and the
        # extension would compile but fail to install (or worse, install a
        # host-triple binary onto the device).
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

# Same guard cad-compile.sh has. Without it an unmatched loop leaves RUST_TARGET
# empty, BINARY_PATH collapses to "$AVOCADO_BUILD_DIR//release/...", and the -f
# check below reports a missing binary instead of the actual fault.
if [ -z "$RUST_TARGET" ]; then
    echo "Error: Could not find Rust target for $OECORE_TARGET_ARCH" >&2
    exit 1
fi

if [ "$match_count" -gt 1 ]; then
    echo "Error: $OECORE_TARGET_ARCH matches $match_count Rust targets:$matches" >&2
    echo "Error: refusing to pick one - installing a different triple's binary than was compiled would ship a wrong-architecture artifact that looks installed." >&2
    exit 1
fi

# Stable marker, same contract as cad-compile.sh's: the resolution fixture test
# greps for it to separate a successful resolution from a later failure.
echo "resolved-target: $RUST_TARGET"

BINARY_PATH="$AVOCADO_BUILD_DIR/$RUST_TARGET/release/avocado-container-agent-dev"

if [ ! -f "$BINARY_PATH" ]; then
    echo "Error: Binary not found at $BINARY_PATH"
    exit 1
fi

echo "Installing avocado-container-agent-dev into extension"
install -D -m 755 "$BINARY_PATH" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/avocado-container-agent-dev"
echo "avocado-container-agent-dev installed successfully"
