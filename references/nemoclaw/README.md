# NemoClaw Reference

This reference deploys [NVIDIA NemoClaw](https://github.com/NVIDIA/NemoClaw) on Avocado OS with GPU-accelerated AI agent execution on Jetson hardware.

NemoClaw is NVIDIA's secure AI agent runtime built on OpenClaw. It provides containerized agent execution with embedded K3s orchestration, network egress control, filesystem isolation, and GPU-accelerated inference routing.

## Supported Targets

- `jetson-orin-nano-devkit`
- `jetson-agx-orin-devkit`

## Architecture

The reference uses a fully containerized approach:

- **Host**: Runs Docker with NVIDIA Container Toolkit (provided by BSP)
- **Gateway Container**: NemoClaw gateway with embedded K3s, Node.js, and Python
- **Sandbox Containers**: Agent execution environments orchestrated by the gateway

No host-level K3s, Node.js, or Python installation is required.

## Data Persistence

All mutable state is stored on the `/var` partition (R/W):

| Path | Purpose |
|---|---|
| `/var/lib/nemoclaw/state/` | OpenClaw state (sessions, agents, credentials, config) |
| `/var/lib/nemoclaw/blueprints/` | NemoClaw blueprint storage |
| `/var/lib/docker/` | Docker images, containers, volumes |
| `/var/log/nemoclaw/` | Log directory |

## Configuration

Edit `/etc/nemoclaw/gateway.env` to configure:

- `NEMOCLAW_IMAGE` — Gateway container image (default: `nvcr.io/nvidia/nemoclaw-gateway:latest`)
- `NEMOCLAW_API_PORT` — Gateway API port (default: `3000`)

## Services

- `nemoclaw-setup.service` — One-shot service that pulls the gateway image and creates the Docker network on first boot
- `nemoclaw-gateway.service` — Runs the NemoClaw gateway container with GPU passthrough

## Viewing Logs

```sh
journalctl -u nemoclaw-gateway -f
journalctl -u nemoclaw-setup
```

## Air-Gapped Deployments

By default, `nemoclaw-setup.service` pulls the gateway container image on first boot, which requires internet access. For air-gapped deployments, pre-load the image:

### Save the image on a networked machine

```sh
docker pull nvcr.io/nvidia/nemoclaw-gateway:latest
docker save nvcr.io/nvidia/nemoclaw-gateway:latest | gzip > nemoclaw-gateway.tar.gz
```

### Load on the target device

Transfer `nemoclaw-gateway.tar.gz` to the device (e.g., via USB), then:

```sh
gunzip -c nemoclaw-gateway.tar.gz | docker load
```

The setup service will skip the pull if the image is already present.
