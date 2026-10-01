# Provenance: recomputer-mini-j5012

Every pinned input this extension is built from, each followed by the command
that produced the value. Re-run a command and compare its output with the value
recorded here; a different result means the input has drifted. All values were
resolved on 2026-09-29 and re-verified on 2026-10-01. Pinned sources are read
through the GitHub API at a fixed commit, never from a local checkout on another
branch.

This file is a build record. It is not listed in `package_files` and does not
ship in the extension package.

## Seeed carrier sources

- Repository: <https://github.com/Seeed-Studio/Linux_for_Tegra>
- Branch: `r36.5.0`
- Commit: `f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`

```sh
gh api repos/Seeed-Studio/Linux_for_Tegra/commits/r36.5.0 --jq .sha
```

The branch moves; the commit is the pin. Every Seeed file is fetched at
`?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`, never at `r36.5.0`.

### Board configuration this extension follows

- File: `recomputer-mini-agx-orin-j501x.conf` at the Seeed commit above
- sha256: `a45a06424c320019cbbaa70b2f01c9f5f49fc0d0cb4048183a00ce5e131fa506`

```sh
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/recomputer-mini-agx-orin-j501x.conf?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | sha256sum
```

## Context: the board's shipped image (not a pin)

The customer's board runs Seeed's own image: Seeed branch `r36.4.4`, L4T 36.4.4,
JetPack 6.2.1 (evidence: `EVIDENCE/inventory/summary.md` in the devspec change
directory). This extension builds against L4T 36.5.2 (below), so the shipped
image is a reference for observed behaviour and not an input to the build.

## meta-avocado (2024 feed)

- Repository: <https://github.com/avocado-linux/meta-avocado>
- Branch: `scarthgap`
- Commit: `e5b750c61c09b16deea4cd21cb01586e1d39d874` (committer date
  2026-09-29T14:21:16Z; the `scarthgap` head when resolved)

```sh
gh api repos/avocado-linux/meta-avocado/commits/scarthgap --jq .sha
gh api repos/avocado-linux/meta-avocado/commits/e5b750c61c09b16deea4cd21cb01586e1d39d874 --jq '.sha + " " + .commit.committer.date'
```

Provisioning script whose carrier block the override check runs:
`meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh`, git blob
`821bc7ce648fa7b0aca66f8f674145b8d8c62c38`, 26430 bytes.

```sh
gh api "repos/avocado-linux/meta-avocado/contents/meta-avocado-nvidia/stone/tegra/stone-provision-tegraflash.sh?ref=e5b750c61c09b16deea4cd21cb01586e1d39d874" --jq '.path + " blob=" + .sha + " size=" + (.size|tostring)'
```

## meta-tegra (2024 feed pin)

- Repository: <https://github.com/avocado-linux/vendor-meta-tegra>
- Branch: `scarthgap`
- Commit: `053a4e97d356499ab028a59c17a5ee36e2c5b8c2` (committer date
  2026-08-30T15:29:54Z)

Named by `kas/vendor/nvidia.yml` at the meta-avocado commit above, key
`repos.meta-tegra.commit`:

```sh
gh api "repos/avocado-linux/meta-avocado/contents/kas/vendor/nvidia.yml?ref=e5b750c61c09b16deea4cd21cb01586e1d39d874" --jq .content | base64 -d
gh api repos/avocado-linux/vendor-meta-tegra/commits/053a4e97d356499ab028a59c17a5ee36e2c5b8c2 --jq '.sha + " " + .commit.committer.date'
```

### L4T version

`L4T_VERSION ?= "36.5.2"` in `classes/l4t_version.bbclass`:

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/classes/l4t_version.bbclass?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
```

### URL variables

From `classes-recipe/l4t_bsp.bbclass` at the pin:

- `L4T_BSP_NAME ??= "releases"`
- `L4T_SRCS_NAME ??= "sources"`
- `L4T_BSP_PREFIX ??= "Jetson"`
- `L4T_URI_BASE ?= "https://developer.download.nvidia.com/embedded/L4T/${@l4t_release_dir(d)}/${L4T_BSP_NAME}"`
- `l4t_release_dir()` turns `36.5.2` into `r36_Release_v5.2`

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/classes-recipe/l4t_bsp.bbclass?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d | head -40
```

Neither `conf/layer.conf` nor `conf/machine/include/tegra-common.inc` at the
pin overrides `L4T_URI_BASE`, `L4T_BSP_PREFIX`, `L4T_BSP_NAME` or
`L4T_SRCS_NAME`.

### NVIDIA BSP archive

`recipes-bsp/tegra-binaries/tegra-binaries-36.5.2.inc` fetches
`${L4T_URI_BASE}/${L4T_BSP_PREFIX}_Linux_R${L4T_VERSION}_aarch64.tbz2` with
`L4T_BSP_NAME` left at `releases`, which resolves to:

- URL: <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/releases/Jetson_Linux_R36.5.2_aarch64.tbz2>
- sha256: `752326264c5e16826d3044a78e59ae06109467d37705143b94e664c91a471f47`
- Size: 749742254 bytes (HTTP `content-length`)

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-bsp/tegra-binaries/tegra-binaries-36.5.2.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
curl -sIL https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/releases/Jetson_Linux_R36.5.2_aarch64.tbz2
```

The sha256 is the recipe's `SRC_URI[sha256sum]`. Verify the downloaded archive
against it before extracting anything from it.

### NVIDIA device-tree sources

`recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot_36.5.2.bb` sets
`TEGRA_SRC_SUBARCHIVE` to
`Linux_for_Tegra/source/kernel_oot_modules_src.tbz2` and
`Linux_for_Tegra/source/nvidia_kernel_display_driver_source.tbz2`, then
requires `recipes-bsp/tegra-sources/tegra-sources-36.5.2.inc`. That include sets
`L4T_BSP_NAME = "${L4T_SRCS_NAME}"` (`sources`) and fetches
`${L4T_URI_BASE}/public_sources.tbz2`, extracting each subarchive from it.

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot_36.5.2.bb?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-bsp/tegra-sources/tegra-sources-36.5.2.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d
```

Outer archive, the unit the recipe fetches and checksums:

- URL: <https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2>
- sha256: `4347a718e828edebee0d776d2110870d02a1d5766665d06c0abd3f325b1801d5`
  (recipe `SRC_URI[sha256sum]`, and the sha256 of the downloaded file matched it)
- Size: 232402561 bytes

```sh
curl -sSfL -o public_sources.tbz2 https://developer.download.nvidia.com/embedded/L4T/r36_Release_v5.2/sources/public_sources.tbz2
sha256sum public_sources.tbz2
```

Device-tree subarchive, the `TEGRA_SRC_SUBARCHIVE` entry that carries the
device tree. It has no URL of its own; it is a member of the outer archive:

- Member: `Linux_for_Tegra/source/kernel_oot_modules_src.tbz2`
- sha256: `d5f334212b2c3bc4bb3047a4d2df9c339e0378730fa3d6c87d5050304aaf2812`

```sh
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_oot_modules_src.tbz2 > kernel_oot_modules_src.tbz2
sha256sum kernel_oot_modules_src.tbz2
```

The second subarchive, `nvidia_kernel_display_driver_source.tbz2`, holds the
display driver and no device tree.

Third member, used only by the DTB build and not named by the recipe's
`TEGRA_SRC_SUBARCHIVE`: the kernel source tarball, from which only
`kernel/kernel-jammy-src/include/dt-bindings` is extracted, for the
`dt-bindings` headers that `kernel_oot_modules_src.tbz2` does not carry.

- Member: `Linux_for_Tegra/source/kernel_src.tbz2`
- sha256: `2a26015d13a2c4551c1266c8ec7503fc27b487d1cf74ccf3965d0633dacced1d`

```sh
tar -xjOf public_sources.tbz2 Linux_for_Tegra/source/kernel_src.tbz2 > kernel_src.tbz2
sha256sum kernel_src.tbz2
```

Device-tree directory: `hardware/nvidia/` inside the subarchive, unpacked into
the recipe's `${S}`. `nvidia-kernel-oot.inc` installs it as the device-tree
source tree with `cp -R ${S}/hardware/nvidia/ ${D}/usr/src/device-tree`.

```sh
gh api "repos/avocado-linux/vendor-meta-tegra/contents/recipes-kernel/nvidia-kernel-oot/nvidia-kernel-oot.inc?ref=053a4e97d356499ab028a59c17a5ee36e2c5b8c2" --jq .content | base64 -d | grep -n 'hardware/nvidia'
tar -tjf kernel_oot_modules_src.tbz2 | grep -c '^hardware/nvidia/'
```

## Device-tree compiler

- Version: `DTC v1.8.1`

```sh
dtc --version
```

## Carrier files

Every file in `stone/carrier-bsp/`, with its origin. All Seeed paths are read at
commit `f9a68317fbe3d276efc25b14b8b72339a1cc5d5c`. For the three Seeed-copied
files the upstream and committed sha256 are equal (the copy is byte-exact).

This extension was derived by retargeting an earlier build of the same extension
for the sibling Seeed board configuration. No file name or value here comes from
that sibling; its differences from the Mini configuration are recorded in
`EVIDENCE/robo-vs-mini.diff` (sha256
`b04e1bb2d52f734f30716e93564262809af08f85d2dbb4b77f195eb69596c924`), so a reader
comparing the Mini files with Seeed's other configuration can see why they differ.

```sh
sha256sum "$EVIDENCE/robo-vs-mini.diff"
```

### Seeed-copied files

| Committed file (`stone/carrier-bsp/`) | Seeed path | Upstream sha256 | Committed sha256 |
|---|---|---|---|
| `recomputer-mini-agx-orin-j501x-gpio-default.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-gpio-default.dtsi` | `5efec361e3c01d0c185c4489ba7f1964fd16a8a6f625dedf01de4a8dfc7b0cdb` | `5efec361e3c01d0c185c4489ba7f1964fd16a8a6f625dedf01de4a8dfc7b0cdb` |
| `recomputer-mini-agx-orin-j501x-padvoltage-default.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-padvoltage-default.dtsi` | `aede16fea255ea7b87669eeece54fe5d2ee7e28388812597c199922609017ed5` | `aede16fea255ea7b87669eeece54fe5d2ee7e28388812597c199922609017ed5` |
| `recomputer-mini-agx-orin-j501x-pinmux.dtsi` | `bootloader/generic/BCT/recomputer-mini-agx-orin-j501x-pinmux.dtsi` | `18115da2682e03a50b893756958e3fb6c399c751bb8d91ab09f328977dbc7698` | `18115da2682e03a50b893756958e3fb6c399c751bb8d91ab09f328977dbc7698` |

```sh
# upstream sha256, per Seeed path in the table
gh api -H 'Accept: application/vnd.github.raw' 'repos/Seeed-Studio/Linux_for_Tegra/contents/<Seeed path>?ref=f9a68317fbe3d276efc25b14b8b72339a1cc5d5c' | sha256sum
# committed sha256
sha256sum bsp/recomputer-mini-j5012/stone/carrier-bsp/<committed file>
```

### MB2 BCT misc override (authored, builds on a stock file)

- Committed file: `tegra234-mb2-bct-misc-p3701-0000-seeed.dts`
- Committed sha256: `5611b92a6bceda9af9494611155287151972d474bb3c7809c3e5b5aa2c0727d0`
- Builds on the stock file `Linux_for_Tegra/bootloader/generic/BCT/tegra234-mb2-bct-misc-p3701-0000.dts`
  and its include `Linux_for_Tegra/bootloader/tegra234-mb2-bct-common.dtsi`, both in
  `Jetson_Linux_R36.5.2_aarch64.tbz2` (sha256
  `752326264c5e16826d3044a78e59ae06109467d37705143b94e664c91a471f47`, listed above;
  paths recorded in `EVIDENCE/mb2-common.notes.txt`).
- One delta against stock, taken from Seeed's `bootloader/tegra234-mb2-bct-common.dtsi`:
  `cvb_eeprom_read_size` from `0x100` to `0` (`EVIDENCE/mb2-common.diff`, the only hunk).

```sh
sha256sum bsp/recomputer-mini-j5012/stone/carrier-bsp/tegra234-mb2-bct-misc-p3701-0000-seeed.dts
grep '^[-+] ' "$EVIDENCE/mb2-common.diff"
```

### SCR

No SCR file ships. Seeed's `tegra234-mb2-bct-scr-p3701-0000.dts` (sha256
`8fe59ec54fd93bb63b4b22bd4d4dfb2b20b49efd93cb5e904bfbe2096b941eb0`) differs from stock
by one undocumented `reg@322` (`GPIO_M_SCR_00_0`) entry that is not carried, and
Seeed's Mini configuration never sets `SCR_CONFIG`, so Seeed's own flash applies
the stock file (`EVIDENCE/mb2-scr.md`).

```sh
grep -n -i scr "$EVIDENCE/recomputer-mini-agx-orin-j501x.conf"   # prints nothing, exit 1
```

### carrier.env (authored, not copied from Seeed)

- Committed sha256: `48fcfdb8b62b08942704a7894e327d920ff053599798bde311d70a1fd2948d79`
- Encodes Seeed `recomputer-mini-agx-orin-j501x.conf` at the commit above, sha256
  `a45a06424c320019cbbaa70b2f01c9f5f49fc0d0cb4048183a00ce5e131fa506`
  (`gh api` command in the "Board configuration" section).

| Knob | Seeed conf line |
|---|---|
| `CARRIER_ENV_DTBFILE` | 44 (`DTB_FILE`, SKU 0005 branch, lines 41-44) |
| `CARRIER_FV_PINMUX_CONFIG` | 61 (`PINMUX_CONFIG`) |
| `CARRIER_FV_PMC_CONFIG` | 62 (`PMC_CONFIG`) |
| `CARRIER_ENV_ODMDATA` | 60 (`ODMDATA`) |
| `CARRIER_FV_CHECK_BOARDID`, `CARRIER_FV_CHECK_BOARDSKU` | 37-48 (SKU branches; also in the stock flashvars) |
| `CARRIER_FV_MB2BCT_CFG` | none; names the authored MB2 file above |
| `TBCDTB_FILE` (deliberately unset) | 50 |

```sh
sha256sum "$EVIDENCE/recomputer-mini-agx-orin-j501x.conf"
cat -n "$EVIDENCE/recomputer-mini-agx-orin-j501x.conf" | sed -n 35,65p
```

### Kernel DTB

- Committed file: `tegra234-j501x-0000+p3701-0005-recomputer-mini.dtb`
- Output sha256 (enforced by `dtb/build-dtb.sh --verify`):
  `07d830c48c1a697d7e0baa05e8f9ea9d32de64dc7b56027721f4474bc800953d`
- Built with `DTC v1.8.1`; `public_sources.tbz2` sha256
  `4347a718e828edebee0d776d2110870d02a1d5766665d06c0abd3f325b1801d5` (see above).
- Seeed inputs, git blob sha at the commit above (paths under
  `source/hardware/nvidia/t23x/nv-public/`):

| Blob sha | Path |
|---|---|
| `ca325af7ef26132460504d59aa1b031f7f6eddd3` | `nv-platform/tegra234-j501x-0000+p3701-0000-recomputer-mini.dts` |
| `e01b6915d8cbf758fd6a9f01296b3c70b08f6eeb` | `nv-platform/tegra234-j501x-0000+p3701-0005-recomputer-mini.dts` |
| `72289c86ac0bb14350a7a3126f1077aa11b6f134` | `nv-platform/tegra234-p3737-0000+p3701-xxxx-nv-common.dtsi` |
| `b4013dab7f6cd34b948cf162eea677990bce5d72` | `tegra234-j501x-0000+p3701-0000.dts` |

```sh
sha256sum bsp/recomputer-mini-j5012/stone/carrier-bsp/tegra234-j501x-0000+p3701-0005-recomputer-mini.dtb
grep -n 'SEEED_FILES=' -A4 bsp/recomputer-mini-j5012/dtb/build-dtb.sh
dtc --version
```
