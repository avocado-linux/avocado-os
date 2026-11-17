# avocado-os

This repository used to hold the shared Avocado OS extensions and the board support packages (BSPs). Each of them now lives in its own repository under the avocado-linux organization.

- Avocado OS: https://avocadolinux.org
- Avocado CLI documentation: https://docs.peridio.com/developer-reference/avocado-cli/overview

## Where things moved

| Former path | Now |
|-------------|-----|
| bsp/alif-e8-devkit | [bsp-alif-e8-devkit](https://github.com/avocado-linux/bsp-alif-e8-devkit) |
| bsp/fr202 | [bsp-fr202](https://github.com/avocado-linux/bsp-fr202) |
| bsp/grinn-astra-1680-sbc | [bsp-grinn-astra-1680-sbc](https://github.com/avocado-linux/bsp-grinn-astra-1680-sbc) |
| bsp/icam-540 | [bsp-icam-540](https://github.com/avocado-linux/bsp-icam-540) |
| bsp/imx8mp-evk | [bsp-imx8mp-evk](https://github.com/avocado-linux/bsp-imx8mp-evk) |
| bsp/imx91-frdm | [bsp-imx91-frdm](https://github.com/avocado-linux/bsp-imx91-frdm) |
| bsp/imx93-evk | [bsp-imx93-evk](https://github.com/avocado-linux/bsp-imx93-evk) |
| bsp/imx93-frdm | [bsp-imx93-frdm](https://github.com/avocado-linux/bsp-imx93-frdm) |
| bsp/intel-x86-64-v2 | [bsp-intel-x86-64-v2](https://github.com/avocado-linux/bsp-intel-x86-64-v2) |
| bsp/intel-x86-64-v3 | [bsp-intel-x86-64-v3](https://github.com/avocado-linux/bsp-intel-x86-64-v3) |
| bsp/intel-x86-64-v4 | [bsp-intel-x86-64-v4](https://github.com/avocado-linux/bsp-intel-x86-64-v4) |
| bsp/jetson-agx-orin-devkit | [bsp-jetson-agx-orin](https://github.com/avocado-linux/bsp-jetson-agx-orin) |
| bsp/jetson-orin-nano-devkit | [bsp-jetson-orin-nano](https://github.com/avocado-linux/bsp-jetson-orin-nano) |
| bsp/jetson-orin-nx | [bsp-jetson-orin-nx](https://github.com/avocado-linux/bsp-jetson-orin-nx) |
| bsp/mic-733-ao5a1 | [bsp-mic-733-ao5a1](https://github.com/avocado-linux/bsp-mic-733-ao5a1) |
| bsp/mic-733-ao6a1 | [bsp-mic-733-ao6a1](https://github.com/avocado-linux/bsp-mic-733-ao6a1) |
| bsp/orangepi-5-plus | [bsp-orangepi-5-plus](https://github.com/avocado-linux/bsp-orangepi-5-plus) |
| bsp/qemuarm64 | [bsp-qemuarm64](https://github.com/avocado-linux/bsp-qemuarm64) |
| bsp/qemux86-64 | [bsp-qemux86-64](https://github.com/avocado-linux/bsp-qemux86-64) |
| bsp/raspberrypi0-2w | [bsp-raspberrypi0-2w](https://github.com/avocado-linux/bsp-raspberrypi0-2w) |
| bsp/raspberrypi4 | [bsp-raspberrypi4](https://github.com/avocado-linux/bsp-raspberrypi4) |
| bsp/raspberrypi5 | [bsp-raspberrypi5](https://github.com/avocado-linux/bsp-raspberrypi5) |
| bsp/recomputer-j301x | [bsp-recomputer-j301x](https://github.com/avocado-linux/bsp-recomputer-j301x) |
| bsp/recomputer-j401x | [bsp-recomputer-j401x](https://github.com/avocado-linux/bsp-recomputer-j401x) |
| bsp/recomputer-mini-j5012-r39 | [bsp-recomputer-mini-j5012-r39](https://github.com/avocado-linux/bsp-recomputer-mini-j5012-r39) |
| bsp/recomputer-mini-j5012 | not moved, see [the tree at 41c9c39e65](https://github.com/avocado-linux/avocado-os/tree/41c9c39e65/bsp/recomputer-mini-j5012) |
| bsp/reterminal-dm | [bsp-reterminal-dm](https://github.com/avocado-linux/bsp-reterminal-dm) |
| bsp/reterminal | [bsp-reterminal](https://github.com/avocado-linux/bsp-reterminal) |
| bsp/rubikpi3 | [bsp-rubikpi3](https://github.com/avocado-linux/bsp-rubikpi3) |
| bsp/rzv2n-sr-som | [bsp-rzv2n-sr-som](https://github.com/avocado-linux/bsp-rzv2n-sr-som) |
| bsp/stm32mp25-dk | [bsp-stm32mp25-dk](https://github.com/avocado-linux/bsp-stm32mp25-dk) |
| extensions/ca-certificates | [ext-ca-certificates](https://github.com/avocado-linux/ext-ca-certificates) |
| extensions/cli | [avocado-cli](https://github.com/avocado-linux/avocado-cli) |
| extensions/cockpit | [ext-cockpit](https://github.com/avocado-linux/ext-cockpit) |
| extensions/connect | [avocado-conn](https://github.com/avocado-linux/avocado-conn) |
| extensions/container-agent-dev | [ext-container-agent-dev](https://github.com/avocado-linux/ext-container-agent-dev) |
| extensions/coreutils | [ext-coreutils](https://github.com/avocado-linux/ext-coreutils) |
| extensions/dev | [ext-dev](https://github.com/avocado-linux/ext-dev) |
| extensions/docker | [ext-docker](https://github.com/avocado-linux/ext-docker) |
| extensions/hello | [references/container-dev-loop](https://github.com/avocado-linux/references/tree/main/container-dev-loop) |
| extensions/jtop | [ext-jtop](https://github.com/avocado-linux/ext-jtop) |
| extensions/kmod-v4l2loopback | [ext-kmod-v4l2loopback](https://github.com/avocado-linux/ext-kmod-v4l2loopback) |
| extensions/microclaw | [ext-microclaw](https://github.com/avocado-linux/ext-microclaw) |
| extensions/podman | [ext-podman](https://github.com/avocado-linux/ext-podman) |
| extensions/sshd-dev | [ext-sshd-dev](https://github.com/avocado-linux/ext-sshd-dev) |
| extensions/sshd | [ext-sshd](https://github.com/avocado-linux/ext-sshd) |
| extensions/tunnels | [avocado-rat](https://github.com/avocado-linux/avocado-rat) |
| extensions/webkit | [ext-webkit](https://github.com/avocado-linux/ext-webkit) |

ext-docker 0.2.0 carries the CA bundle overlay. The raw netfilter table module is not packaged for every 2024 target, so it stays only in the history at 41c9c39e65.

Every removed path is still in the git history at commit 41c9c39e65. Long-lived branches were left in place.
