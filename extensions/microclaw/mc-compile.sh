#!/bin/bash
# Fetch and verify the upstream MicroClaw release tarball, then stage the
# extracted binary in $AVOCADO_BUILD_DIR for mc-install.sh to pick up.
#
# We ship a prebuilt linux-gnu glibc binary instead of building from source:
# microclaw's Cargo workspace has 7 crates, a build.rs that invokes npm to
# build the bundled web UI, and a mixed rustls + native-tls dep tree. Building
# it under the Yocto offline fetcher is significantly more work than pinning
# an upstream release artifact by sha256.

set -euo pipefail

MICROCLAW_VERSION="0.1.57"

case "${OECORE_TARGET_ARCH:-}" in
  aarch64)
    ASSET_ARCH="aarch64-linux-gnu"
    ASSET_SHA256="39dfdf56e2d822f129934afa7b3cea29c51b2656a00bad42235fb5ee3848b99f"
    ;;
  x86_64)
    ASSET_ARCH="x86_64-linux-gnu"
    ASSET_SHA256="aa12bbb3f923d6af5fe1d1d27673f0b0d437d5fc3054e9d8b3b9a8bea7522a4d"
    ;;
  *)
    echo "Error: avocado-ext-microclaw has no prebuilt artifact for OECORE_TARGET_ARCH=${OECORE_TARGET_ARCH:-unset}" >&2
    echo "Upstream ships aarch64-linux-gnu and x86_64-linux-gnu only." >&2
    exit 1
    ;;
esac

ASSET_NAME="microclaw-${MICROCLAW_VERSION}-${ASSET_ARCH}.tar.gz"
ASSET_URL="https://github.com/microclaw/microclaw/releases/download/v${MICROCLAW_VERSION}/${ASSET_NAME}"

STAGE_DIR="${AVOCADO_BUILD_DIR}/microclaw"
mkdir -p "${STAGE_DIR}"

cd "${STAGE_DIR}"

if [ ! -f "${ASSET_NAME}" ]; then
    echo "Fetching ${ASSET_URL}"
    curl -fSL --retry 3 --retry-delay 2 -o "${ASSET_NAME}.partial" "${ASSET_URL}"
    mv "${ASSET_NAME}.partial" "${ASSET_NAME}"
fi

echo "${ASSET_SHA256}  ${ASSET_NAME}" | sha256sum -c -

rm -f microclaw
tar -xzf "${ASSET_NAME}"

if [ ! -x microclaw ]; then
    echo "Error: extracted tarball did not produce an executable 'microclaw' at ${STAGE_DIR}/microclaw" >&2
    exit 1
fi

echo "microclaw ${MICROCLAW_VERSION} (${ASSET_ARCH}) staged at ${STAGE_DIR}/microclaw"
