# NemoClaw Reference

This reference deploys [NVIDIA NemoClaw](https://github.com/NVIDIA/NemoClaw) on Avocado OS with local GPU-accelerated AI agent execution on Jetson hardware using Ollama for inference.

NemoClaw is NVIDIA's secure AI agent runtime built on OpenClaw. It provides containerized agent execution with embedded K3s orchestration, network egress control, filesystem isolation, and inference routing.

## Supported Targets

- `jetson-orin-nano-devkit`
- `jetson-agx-orin-devkit`

## Architecture

- **NemoClaw CLI** — Pre-installed Node.js application that orchestrates agent sandboxes via Docker
- **Ollama** — Local inference server with GPU acceleration, serves models like `nemotron-3-nano:30b`
- **Docker** — Container runtime for NemoClaw sandbox environments (provided by `nvidia-docker` extension)

The NemoClaw CLI is cross-compiled at build time via npm in the SDK container. Ollama and Node.js are included as distro packages.

## Data Persistence

All mutable state is stored on the `/var` partition (R/W):

| Path | Purpose |
|---|---|
| `/var/lib/nemoclaw/` | NemoClaw working directory (onboard state, sessions, agents) |
| `/var/lib/ollama/models/` | Downloaded LLM model weights |
| `/var/lib/docker/` | Docker images, containers, volumes |

## First Boot

On first boot, `nemoclaw.service` will:

1. Wait for Ollama to be ready
2. Pull the `nemotron-3-nano:30b` model (~30 GB download)
3. Run `nemoclaw onboard` with Ollama as the inference provider
4. Start the NemoClaw agent runtime

Subsequent boots skip the onboard step (state persists in `/var/lib/nemoclaw/.onboarded`).

## Services

| Service | Type | Purpose |
|---|---|---|
| `ollama.service` | long-running | Local inference server on port 11434 |
| `nemoclaw.service` | long-running | NemoClaw agent runtime |
| `docker.service` | long-running | Container runtime for sandboxes |

## Viewing Logs

```sh
journalctl -u ollama -f
journalctl -u nemoclaw -f
```

## Interacting with NemoClaw

Once the first-boot setup completes:

```sh
# Check status
nemoclaw status

# Connect to an agent
nemoclaw <agent-name> connect

# View logs
nemoclaw <agent-name> logs --follow
```

## Dependencies

This reference requires the following packages to be available in the Avocado package feed:

- `nodejs` (22+) — available
- `nodejs-npm` — available
- `nvidia-docker` — available
- `curl`, `git` — available

Ollama and the NemoClaw CLI are downloaded and bundled into the extension at build time (no distro recipe needed).
