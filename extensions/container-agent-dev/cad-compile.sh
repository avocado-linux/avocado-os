#!/bin/bash
set -euo pipefail

RUST_TARGET=""
match_count=0
matches=""

# Resolve the Rust target from RUST_TARGET_PATH. Enumerate every candidate
# instead of stopping at the first: an OECORE_TARGET_ARCH that prefix-matches
# two target JSONs used to take whichever the glob happened to return first and
# discard the rest without a word. That is the worst failure shape available
# here - a binary for the wrong architecture packages, publishes and installs
# successfully, and only fails when the device tries to exec it, several layers
# from the cause. Refusing to guess keeps the failure local and attributable.
#
# devtool-debt: this block is duplicated, still unfixed, in the three sibling
# extensions that have not yet moved to their own ext-* repo - cli, connect and
# tunnels - in both their compile and install scripts. jtop and microclaw carry
# a copy too, but theirs are superseded: both migrated to ext-* repos, and
# ext-microclaw resolves the architecture with a plain case statement that does
# not have this defect at all.
# Ceiling: those three are safe by GLOB ORDERING ALONE, not by construction.
# Every target whose architecture matches the SDK host architecture has two
# matching JSONs - the device triple and the SDK's own <arch>-avocadosdk-* - and
# first-match-and-break survives only because "avocado-" sorts ahead of
# "avocadosdk-" on the hyphen. Nothing enforces that ordering. This is not a
# theoretical ceiling: it is exactly what broke qemux86-64 here the first time
# the guard below ran.
# Upgrade trigger: a vendor string that reorders those two, a third triple
# sorting between them, or any of the three gaining a target whose architecture
# prefix-matches more than one device triple.
for json_file in "$RUST_TARGET_PATH"/*.json; do
    if [ -f "$json_file" ]; then
        json_name=$(basename "$json_file" .json)

        # Skip the SDK's own nativesdk triple. It shares the architecture prefix
        # with the device triple whenever the SDK host arch matches the target
        # arch, which on x86_64 means RUST_TARGET_PATH holds both
        # x86_64-avocado-linux-gnu and x86_64-avocadosdk-linux-gnu. The
        # avocadosdk vendor is host-side by construction - avocado-cli tags every
        # nativesdk artifact <arch>_avocadosdk - so it is never a candidate for a
        # device binary and must not count as a competing match.
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

if [ -z "$RUST_TARGET" ]; then
    echo "Error: Could not find Rust target for $OECORE_TARGET_ARCH" >&2
    exit 1
fi

if [ "$match_count" -gt 1 ]; then
    echo "Error: $OECORE_TARGET_ARCH matches $match_count Rust targets:$matches" >&2
    echo "Error: refusing to pick one - the wrong triple yields a binary that packages and installs but cannot exec on the device." >&2
    exit 1
fi

# Stable marker: the resolution fixture test greps for this to tell "resolution
# succeeded and a later step failed" from "resolution itself failed", which exit
# code alone cannot distinguish when the test runs without a real SDK. Load-
# bearing test surface, not a debug echo.
echo "resolved-target: $RUST_TARGET"

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
