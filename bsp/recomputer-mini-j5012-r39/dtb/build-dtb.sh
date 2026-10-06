#!/usr/bin/env bash
# devtool-debt: one committed DTB per L4T release. Ceiling: the 2026 feed's L4T 39.2.x device-tree ABI. Upgrade trigger: this board is needed on the 2026 feed, or Seeed changes its J501 DTS - then build it in the Yocto device-tree recipe instead.
#
# Reproducible build of the reComputer Mini J5012 kernel DTB.
#
#   build-dtb.sh            fetch, verify, build, write the DTB into stone/carrier-bsp/
#   build-dtb.sh --verify   fetch, verify, rebuild, compare the sha256 with the committed DTB
#
# Pinned inputs (values and the commands that produced them: ../PROVENANCE.md):
#   - Seeed Linux_for_Tegra commit df17ed28... (commit and each fetched file's git blob sha)
#   - NVIDIA public_sources.tbz2 (L4T 39.2.0) and its member kernel_oot_modules_src.tbz2
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
# The cpp and dtc invocation mirrors what the nvidia-kernel-oot-dtb recipe drives at
# vendor-meta-tegra 727633deca9b72643112d1aebadafb736dad60c0 (classes-recipe/
# tegra-devicetree.bbclass): DT_INCLUDE gives the include search order, DTC_PPFLAGS the
# cpp defines (-DLINUX_VERSION=600 -DTEGRA_HOST1X_DT_VERSION=2 -DOS_LINUX), and
# DT_FILES_PATH is hardware/nvidia/t23x/nv-public/nv-platform. The bbclass takes
# dt-bindings from the kernel build (KERNEL_INCLUDE); this script takes them from the
# kernel_src.tbz2 member of the same NVIDIA archive (kernel/kernel-noble, the tree
# the 6.8 l4t kernel recipe is built from). `-@` comes from the nv-platform Makefile.

set -euo pipefail

SEEED_REPO="Seeed-Studio/Linux_for_Tegra"
SEEED_COMMIT="df17ed28201645fcaeb6c8e6fb82a0b4592faefb"
SEEED_BASE="source/hardware/nvidia/t23x/nv-public"

OUTER_URL="https://developer.download.nvidia.com/embedded/L4T/r39_Release_v2.0/sources/public_sources.tbz2"
OUTER_SHA256="87d2e31ff55beaf2373e2f288538585995b231fd5745ec21f39a668e36efab2f"
OOT_MEMBER="Linux_for_Tegra/source/kernel_oot_modules_src.tbz2"
OOT_SHA256="c1db05b32d7429c27b15bd4fd8d826bd56ee03690a337e19f03097b48aa03206"
KSRC_MEMBER="Linux_for_Tegra/source/kernel_src.tbz2"
KSRC_SHA256="2bc287ba8193e8b686f23b6e14ac25fbe8fcbfac457a3432d634c46f217a350e"
KSRC_BINDINGS="kernel/kernel-noble/include/dt-bindings"

DTS_NAME="tegra234-j501x-0000+p3701-0005-recomputer-mini"

# Seeed files overlaid onto NVIDIA's hardware/nvidia/t23x/nv-public: the include closure of
# the DTS that Seeed carries. Format: <git blob sha>  <path under nv-public>
SEEED_FILES="ca325af7ef26132460504d59aa1b031f7f6eddd3  nv-platform/tegra234-j501x-0000+p3701-0000-recomputer-mini.dts
e01b6915d8cbf758fd6a9f01296b3c70b08f6eeb  nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-mini.dts
16639d55a3fc83f8163ee7db4ed857098b5b44ca  nv-platform/tegra234-p3737-0000+p3701-xxxx-nv-common.dtsi
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

# Order is DT_INCLUDE:tegra234 from tegra-devicetree.bbclass: nv-public/ dtsi files must
# precede nv-platform/ ones of the same name (they carry the mmc aliases), then the
# kernel dt-bindings last.
NVTEGRA="$TREE/hardware/nvidia/tegra/nv-public"
DTC_INCLUDE=(
  "$NVTEGRA/include/kernel"
  "$NVTEGRA/include/nvidia-oot"
  "$NVTEGRA"
  "$NVPUB/include/nvidia-oot"
  "$NVPUB/include/platforms"
  "$NVPUB"
  "$NVPUB/nv-platform"
  "$WORK/$(dirname "$KSRC_BINDINGS")"
)

CPP_FLAGS=(-nostdinc -undef -D__DTS__ -DLINUX_VERSION=600 -DTEGRA_HOST1X_DT_VERSION=2 -DOS_LINUX)
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
