#!/usr/bin/env bash
# devtool-debt: one committed DTB per L4T release. Ceiling: the 2024 feed's L4T 36.5.x device-tree ABI. Upgrade trigger: this board is needed on the 2026 feed, or Seeed changes its J501 DTS - then build it in the Yocto device-tree recipe instead.
#
# Reproducible build of the reComputer Mini J5012 kernel DTB.
#
#   build-dtb.sh            fetch, verify, build, write the DTB into stone/carrier-bsp/
#   build-dtb.sh --verify   fetch, verify, rebuild, compare the sha256 with the committed DTB
#
# Pinned inputs (values and the commands that produced them: ../PROVENANCE.md):
#   - Seeed Linux_for_Tegra commit f9a68317... (commit and each fetched file's git blob sha)
#   - NVIDIA public_sources.tbz2 (L4T 36.5.2) and its member kernel_oot_modules_src.tbz2
#   - kernel_src.tbz2, another member of public_sources.tbz2, only for the kernel's
#     dt-bindings headers (Seeed's DTS includes dt-bindings/net/ti-dp83867.h)
# Any mismatch exits non-zero and leaves the committed DTB untouched.
#
# --verify compares against the committed file and is expected to fail on a dtc other than the
# one pinned here (DTC v1.8.1), because dtc output differs across versions.
#
# Environment overrides (all optional):
#   J5012_DTB_CACHE_DIR    download cache for public_sources.tbz2
#                          (default: ${XDG_CACHE_HOME:-$HOME/.cache}/avocado-os/recomputer-mini-j5012-dtb)
#   J5012_DTB_WORK_PARENT  directory the scratch dir is created in (default: ${TMPDIR:-/tmp})
#   COMMITTED_DTB          DTB path to write, or to compare against in --verify
#
# The cpp and dtc invocation mirrors what nvidia-kernel-oot.inc drives at
# vendor-meta-tegra 053a4e97d356499ab028a59c17a5ee36e2c5b8c2: `oe_runmake dtbs` runs the
# Makefile inside kernel_oot_modules_src, whose kernel-devicetree/generic-dts/Makefile
# builds DTC_INCLUDE and DTC_CPP_FLAGS and whose scripts/Makefile.lib adds the dtc
# warning suppressions. `-@` comes from hardware/nvidia/t23x/nv-public/nv-platform/Makefile.

set -euo pipefail

SEEED_REPO="Seeed-Studio/Linux_for_Tegra"
SEEED_COMMIT="f9a68317fbe3d276efc25b14b8b72339a1cc5d5c"
SEEED_BASE="source/hardware/nvidia/t23x/nv-public"

OUTER_URL="https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2"
OUTER_SHA256="4347a718e828edebee0d776d2110870d02a1d5766665d06c0abd3f325b1801d5"
OOT_MEMBER="Linux_for_Tegra/source/kernel_oot_modules_src.tbz2"
OOT_SHA256="d5f334212b2c3bc4bb3047a4d2df9c339e0378730fa3d6c87d5050304aaf2812"
KSRC_MEMBER="Linux_for_Tegra/source/kernel_src.tbz2"
KSRC_SHA256="2a26015d13a2c4551c1266c8ec7503fc27b487d1cf74ccf3965d0633dacced1d"
KSRC_BINDINGS="kernel/kernel-jammy-src/include/dt-bindings"

DTS_NAME="tegra234-j501x-0000+p3701-0005-recomputer-mini"

# Seeed files overlaid onto NVIDIA's hardware/nvidia/t23x/nv-public: the include closure of
# the DTS that Seeed carries. Format: <git blob sha>  <path under nv-public>
SEEED_FILES="ca325af7ef26132460504d59aa1b031f7f6eddd3  nv-platform/tegra234-j501x-0000+p3701-0000-recomputer-mini.dts
e01b6915d8cbf758fd6a9f01296b3c70b08f6eeb  nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-mini.dts
72289c86ac0bb14350a7a3126f1077aa11b6f134  nv-platform/tegra234-p3737-0000+p3701-xxxx-nv-common.dtsi
b4013dab7f6cd34b948cf162eea677990bce5d72  tegra234-j501x-0000+p3701-0000.dts"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
EXT_DIR="$(dirname "$SCRIPT_DIR")"
COMMITTED_DTB="${COMMITTED_DTB:-$EXT_DIR/stone/carrier-bsp/$DTS_NAME.dtb}"
CACHE_DIR="${J5012_DTB_CACHE_DIR:-${XDG_CACHE_HOME:-$HOME/.cache}/avocado-os/recomputer-mini-j5012-dtb}"
WORK_PARENT="${J5012_DTB_WORK_PARENT:-${TMPDIR:-/tmp}}"

MODE="build"
case "${1:-}" in
  "") ;;
  --verify) MODE="verify" ;;
  *)
    echo "usage: $0 [--verify]" >&2
    exit 2
    ;;
esac

die() {
  echo "build-dtb: REFUSED: $*" >&2
  exit 1
}

sha256_of() { sha256sum "$1" | cut -d' ' -f1; }

# git blob sha1 of a file, to compare against the tree the pinned commit names
blob_sha() {
  {
    printf 'blob %d\0' "$(stat -c %s "$1")"
    cat "$1"
  } | sha1sum | cut -d' ' -f1
}

for tool in cpp dtc sha256sum sha1sum curl gh tar; do
  command -v "$tool" >/dev/null || die "required tool missing: $tool"
done

mkdir -p "$WORK_PARENT" "$CACHE_DIR"
WORK="$(mktemp -d "$WORK_PARENT/j5012-dtb-work.XXXXXX")"
PART=""
cleanup() {
  rm -rf "$WORK"
  [ -z "$PART" ] || rm -f "$PART"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

echo "build-dtb: $(dtc --version)"
echo "build-dtb: cpp $(cpp --version | head -1)"

# 1. Seeed commit and files ---------------------------------------------------------
got_commit="$(gh api "repos/$SEEED_REPO/commits/$SEEED_COMMIT" --jq .sha)" \
  || die "cannot resolve Seeed commit $SEEED_COMMIT"
[ "$got_commit" = "$SEEED_COMMIT" ] || die "Seeed commit is $got_commit, pinned $SEEED_COMMIT"

SEEED_DIR="$WORK/seeed"
while read -r want path; do
  [ -n "$path" ] || continue
  mkdir -p "$SEEED_DIR/$(dirname "$path")"
  gh api -H 'Accept: application/vnd.github.raw' \
    "repos/$SEEED_REPO/contents/$SEEED_BASE/$path?ref=$SEEED_COMMIT" >"$SEEED_DIR/$path" \
    || die "cannot fetch Seeed file $path"
  have="$(blob_sha "$SEEED_DIR/$path")"
  [ "$have" = "$want" ] || die "Seeed file $path blob sha is $have, pinned $want"
done <<<"$SEEED_FILES"
echo "build-dtb: Seeed commit and 4 files verified"

# 2. NVIDIA archives ----------------------------------------------------------------
OUTER="$CACHE_DIR/public_sources-$OUTER_SHA256.tbz2"
if [ ! -f "$OUTER" ]; then
  PART="$OUTER.part.$$"
  curl -sSfL -o "$PART" "$OUTER_URL" || die "download failed: $OUTER_URL"
  mv "$PART" "$OUTER"
  PART=""
fi
# the cache is never trusted: re-hash on every run
got="$(sha256_of "$OUTER")"
[ "$got" = "$OUTER_SHA256" ] || die "public_sources.tbz2 sha256 is $got, pinned $OUTER_SHA256 (delete $OUTER to re-download)"

# one pass over the bzip2 stream for both members; --strip-components=2 drops Linux_for_Tegra/source/
mkdir -p "$WORK/members"
tar -xjf "$OUTER" -C "$WORK/members" --no-same-owner --strip-components=2 "$OOT_MEMBER" "$KSRC_MEMBER" \
  || die "cannot extract $OOT_MEMBER and $KSRC_MEMBER"
mv "$WORK/members/$(basename "$OOT_MEMBER")" "$WORK/kernel_oot_modules_src.tbz2"
mv "$WORK/members/$(basename "$KSRC_MEMBER")" "$WORK/kernel_src.tbz2"
got="$(sha256_of "$WORK/kernel_oot_modules_src.tbz2")"
[ "$got" = "$OOT_SHA256" ] || die "kernel_oot_modules_src.tbz2 sha256 is $got, pinned $OOT_SHA256"
got="$(sha256_of "$WORK/kernel_src.tbz2")"
[ "$got" = "$KSRC_SHA256" ] || die "kernel_src.tbz2 sha256 is $got, pinned $KSRC_SHA256"
echo "build-dtb: NVIDIA archives verified"

# 3. Assemble the source tree -------------------------------------------------------
TREE="$WORK/tree"
mkdir -p "$TREE"
tar -xjf "$WORK/kernel_oot_modules_src.tbz2" -C "$TREE" --no-same-owner hardware/nvidia
tar -xjf "$WORK/kernel_src.tbz2" -C "$WORK" --no-same-owner "$KSRC_BINDINGS"

NVPUB="$TREE/hardware/nvidia/t23x/nv-public"
while read -r _ path; do
  [ -n "$path" ] || continue
  cp "$SEEED_DIR/$path" "$NVPUB/$path"
done <<<"$SEEED_FILES"

# 4. Preprocess and compile ---------------------------------------------------------
DTS="$NVPUB/nv-platform/$DTS_NAME.dts"
[ -f "$DTS" ] || die "DTS missing from assembled tree: $DTS"

DTC_INCLUDE=(
  "$TREE/hardware/nvidia/tegra/nv-public"
  "$NVPUB/include/kernel"
  "$NVPUB/include/nvidia-oot"
  "$NVPUB/include/platforms"
  "$NVPUB"
  "$WORK/$(dirname "$KSRC_BINDINGS")"
)

CPP_FLAGS=(-nostdinc -undef -D__DTS__ -DLINUX_VERSION=600 -DTEGRA_HOST1X_DT_VERSION=2)
for d in "${DTC_INCLUDE[@]}"; do CPP_FLAGS+=("-I$d"); done

# -@ changes the output. The -Wno-* flags only silence checks, so a check this dtc does not
# know (older or newer than the kernel's bundled dtc) is dropped rather than fatal.
DTC_FLAGS=(-@)
printf '/dts-v1/;\n/ { };\n' >"$WORK/probe.dts"
for w in interrupt_provider unit_address_vs_reg unit_address_format avoid_unnecessary_addr_size \
  alias_paths graph_child_address simple_bus_reg unique_unit_address; do
  if dtc -o /dev/null -b 0 "-Wno-$w" "$WORK/probe.dts" >/dev/null 2>&1; then
    DTC_FLAGS+=("-Wno-$w")
  fi
done
DTC_I=("-i$(dirname "$DTS")")
for d in "${DTC_INCLUDE[@]}"; do DTC_I+=("-i$d"); done

PRE="$WORK/$DTS_NAME.dts.pre"
OUT="$WORK/$DTS_NAME.dtb"
cpp "${CPP_FLAGS[@]}" -x assembler-with-cpp -o "$PRE" "$DTS" || die "cpp failed on $DTS"
dtc -o "$OUT" -b 0 "${DTC_I[@]}" "${DTC_FLAGS[@]}" "$PRE" || die "dtc failed on $DTS"

built="$(sha256_of "$OUT")"
echo "build-dtb: built $DTS_NAME.dtb sha256=$built"

# 5. Write or compare ---------------------------------------------------------------
if [ "$MODE" = "verify" ]; then
  [ -f "$COMMITTED_DTB" ] || die "no committed DTB at $COMMITTED_DTB"
  have="$(sha256_of "$COMMITTED_DTB")"
  if [ "$have" != "$built" ]; then
    die "rebuilt sha256 $built differs from committed $have ($COMMITTED_DTB)"
  fi
  echo "build-dtb: VERIFIED committed DTB matches rebuild ($built)"
else
  mkdir -p "$(dirname "$COMMITTED_DTB")"
  cp "$OUT" "$COMMITTED_DTB.part.$$"
  mv "$COMMITTED_DTB.part.$$" "$COMMITTED_DTB"
  echo "build-dtb: wrote $COMMITTED_DTB"
fi
