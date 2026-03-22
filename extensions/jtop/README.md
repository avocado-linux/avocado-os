# jtop Extension

Avocado OS extension that provides [jetson-stats](https://github.com/rbonghi/jetson_stats) (jtop) for NVIDIA Jetson system monitoring.

## Supported Targets

- jetson-orin-nano-devkit
- jetson-agx-orin-devkit
- icam-540

## What's Included

- **jtop** — Terminal-based system monitoring tool for Jetson devices
- **jtop.service** — Systemd service that runs the jtop stats monitor in the background

## Build

The extension cross-compiles jetson-stats using `uv` to resolve and install Python packages for the target platform. Build scripts:

- `jtop-compile.sh` — Resolves dependencies and installs packages into a build directory
- `jtop-install.sh` — Copies built packages and the jtop entry point into the extension sysroot
- `jtop-clean.sh` — Removes build artifacts

## License

AGPL-3.0
