# Avocado Development Build Scripts

This directory contains development scripts for building and managing Avocado Linux packages and extensions locally. These scripts complement the existing Docker-based build system and GitHub Actions workflows.

## Overview

The development build system consists of several scripts that work together to:

1. **Sync packages** from Yocto build outputs to a local repository
2. **Start a package repository server** for serving packages to extension builds
3. **Build extensions** for specific targets using the local repository
4. **Orchestrate the entire process** with a single command

## Prerequisites

- Docker (for running the package repository server)
- `avocado` CLI tool (for building extensions)
- Completed Yocto builds in `build-<target>` directories

### Installing avocado CLI

```bash
curl -fsSL https://github.com/avocadolinux/avocado/releases/latest/download/avocado-linux-amd64 -o /usr/local/bin/avocado
chmod +x /usr/local/bin/avocado
```

## Scripts

### `dev-build.sh` - Main Orchestration Script

The main script that orchestrates the entire development build process.

```bash
# Build everything for a single target
./scripts/dev-build.sh qemux86-64

# Build for multiple targets
./scripts/dev-build.sh qemux86-64 raspberrypi4

# Use custom repository directory and release ID
./scripts/dev-build.sh -r /opt/avocado-repo -i stable-v1.0 qemux86-64

# Only sync packages, don't build extensions
./scripts/dev-build.sh --sync-only qemux86-64

# Build specific extensions only
./scripts/dev-build.sh --extensions docker,sshd qemux86-64

# Keep repository server running after build
./scripts/dev-build.sh --keep-repo qemux86-64
```

### `dev-sync-packages.sh` - Package Synchronization

Syncs packages from Yocto build outputs to a local repository structure.

```bash
# Sync packages for a target
./scripts/dev-sync-packages.sh qemux86-64

# Use custom repository directory and release ID
./scripts/dev-sync-packages.sh -r /opt/avocado-repo -i my-build qemux86-64

# Use custom build directory
./scripts/dev-sync-packages.sh -b my-custom-build qemux86-64
```

### `dev-start-repo.sh` - Repository Server Management

Manages the Docker-based package repository server.

```bash
# Start repository server (default port 8080)
./scripts/dev-start-repo.sh

# Use custom repository directory and port
./scripts/dev-start-repo.sh -r /opt/avocado-repo -p 9000

# Stop the repository server
./scripts/dev-start-repo.sh --stop

# Restart the repository server
./scripts/dev-start-repo.sh --restart

# View server logs
./scripts/dev-start-repo.sh --logs

# Check server status
./scripts/dev-start-repo.sh --status
```

### `dev-build-extensions.sh` - Extension Building

Builds extensions for specific targets using the local repository.

```bash
# Build all extensions for a target
./scripts/dev-build-extensions.sh -t qemux86-64 --all

# Build specific extensions
./scripts/dev-build-extensions.sh -t qemux86-64 docker sshd

# List available extensions
./scripts/dev-build-extensions.sh --list

# Use custom repository URL
./scripts/dev-build-extensions.sh -t qemux86-64 -u http://localhost:9000 --all
```

### `check-staging-checksums.py` - Staging Directory Analysis

Analyzes staging directories for duplicate packages and checksum mismatches. This Python-based script helps identify inconsistencies in package builds across different machines and architectures.

```bash
# Analyze default staging directory
./scripts/check-staging-checksums.py

# Analyze custom staging directory
./scripts/check-staging-checksums.py /path/to/staging

# Verbose output with JSON format
./scripts/check-staging-checksums.py -v --format json /path/to/staging

# CSV output for further analysis
./scripts/check-staging-checksums.py --format csv /path/to/staging > anomalies.csv

# Quiet mode (only errors)
./scripts/check-staging-checksums.py -q /path/to/staging

# Use 8 parallel workers for faster checksum calculation
./scripts/check-staging-checksums.py -j 8 /path/to/staging

# Single-threaded mode (useful for debugging)
./scripts/check-staging-checksums.py -j 1 /path/to/staging

# Disable progress display (useful for scripting)
./scripts/check-staging-checksums.py --no-progress /path/to/staging

# Combine options for automated processing
./scripts/check-staging-checksums.py -j 16 --no-progress --format json /path/to/staging

# CI environment with custom progress interval (every 10 seconds)
./scripts/check-staging-checksums.py --progress-interval 10.0 /path/to/staging

# High-performance CI mode
./scripts/check-staging-checksums.py -j 32 --progress-interval 2.0 --no-color /path/to/staging
```

The script expects a staging directory structure like:
```
staging/
├── 2025-01-15-123456/
│   ├── avocado-distro/
│   │   └── machine1/latest/apollo/edge/target/arch/
│   └── avocado-sdk/
│       ├── x86_64/machine1/latest/apollo/edge/target/arch/
│       └── aarch64/machine1/latest/apollo/edge/target/arch/
└── 2025-01-15-234567/
    └── ...
```

The script will:
- Find all `.rpm` packages across timestamped staging builds
- Calculate checksums in parallel using all available CPU cores (configurable)
- Group packages by their canonical location in the tree structure
- Compare checksums of packages with the same name and location
- Report any mismatches with detailed checksum information

**Performance Features:**
- Automatic detection of CPU cores for optimal parallel processing
- Configurable parallel job count for different system capabilities
- Efficient batch processing of checksum calculations
- Minimal memory footprint even with large package sets

**User Experience Features:**
- Real-time progress bars with percentage completion using tqdm (or fallback implementation)
- Phase-based progress tracking (discovery, checksums, grouping, analysis)
- Colored output with Unicode progress bars
- Automatic progress disabling for non-interactive environments
- Configurable progress display for scripting scenarios
- Python-based implementation for better performance and reliability
- Graceful interrupt handling with Ctrl-C (single press for graceful shutdown, double press for immediate exit)
- Proper cleanup of temporary files and database connections on interruption

## Workflow Examples

### Basic Development Workflow

1. **Build the distro** using the existing Docker-based system:
   ```bash
   source ./distro/scripts/init-build distro/kas/machine/qemux86-64.yml
   source .envrc
   kas build "$KAS_YML" --target avocado-distro
   ```

2. **Sync packages and build extensions**:
   ```bash
   ./scripts/dev-build.sh qemux86-64
   ```

3. **Access the repository** at `http://localhost:8080/`

### Multi-Target Development

Build packages for multiple targets and aggregate them in a single repository:

```bash
# Build distro for multiple targets (using existing Docker system)
for target in qemux86-64 raspberrypi4; do
    source ./distro/scripts/init-build distro/kas/machine/$target.yml
    source .envrc
    kas build "$KAS_YML" --target avocado-distro
done

# Sync all targets to the same repository
./scripts/dev-build.sh -i multi-target-build qemux86-64 raspberrypi4
```

### Incremental Development

For iterative development where you want to keep the repository server running:

```bash
# Initial build with persistent repository
./scripts/dev-build.sh --keep-repo qemux86-64

# Later, sync additional targets to the same repository
./scripts/dev-sync-packages.sh -i multi-target-build raspberrypi4
./scripts/dev-build-extensions.sh -t raspberrypi4 --all

# Repository server remains accessible throughout
```

### Extension-Only Development

If you only want to rebuild extensions without re-syncing distro packages:

```bash
# Start repository server with existing packages
./scripts/dev-start-repo.sh -r /opt/avocado-repo

# Build extensions for a target
./scripts/dev-build-extensions.sh -t qemux86-64 --all

# Stop server when done
./scripts/dev-start-repo.sh --stop
```

## Repository Structure

The development repository uses the following structure:

```
<repo-dir>/
├── packages/<distro-codename>/          # Aggregated packages from all targets
│   ├── target/                          # Target-specific packages
│   │   ├── <arch>/                      # Architecture-specific packages
│   │   └── <target>-ext/                # Extension packages for target
│   └── ...                              # Other package categories
└── releases/<distro-codename>/          # Repository metadata
    └── <release-id>/                    # Timestamped releases
        ├── target/                      # Target metadata
        └── ...                          # Other metadata
```

## Configuration

### Default Values

- **Repository Directory**: `/tmp/avocado-dev-repo`
- **Distribution Codename**: `latest/apollo/edge`
- **Repository Port**: `8080`
- **Container Name**: `avocado-dev-repo`
- **Docker Network**: `avocado-dev-network`

### Environment Variables

You can override defaults using environment variables:

```bash
export AVOCADO_DEFAULT_DISTRO_CODENAME="custom/release/name"
export AVOCADO_DEFAULT_REPO_BASE="https://my-repo.example.com"
```

## Integration with GitHub Actions

The development scripts share common logic with the GitHub Actions workflows through the `scripts/lib/common.sh` library. This ensures consistency between local development and CI/CD processes.

Key shared functions:
- Machine target validation
- Extension discovery and matrix generation
- Repository path management
- Docker container management

## Troubleshooting

### Repository Server Issues

1. **Container won't start**: Check Docker daemon and port availability
2. **Packages not found**: Ensure packages were synced first
3. **Permission issues**: Check file ownership in repository directory

### Extension Build Issues

1. **avocado CLI not found**: Install the CLI tool (see Prerequisites)
2. **Network connectivity**: Ensure repository server is accessible
3. **Extension not found**: Check extension exists and supports the target

### Build Directory Issues

1. **Build directory not found**: Ensure Yocto build completed successfully
2. **Map file missing**: Check that the build finished without errors
3. **Deploy directory empty**: Verify build target was correct

## Advanced Usage

### Custom Release Management

Use meaningful release IDs for tracking builds:

```bash
# Use semantic versioning
./scripts/dev-build.sh -i v1.2.3-rc1 qemux86-64

# Use feature branch names
./scripts/dev-build.sh -i feature-new-driver qemux86-64

# Use date-based releases
./scripts/dev-build.sh -i "$(date +%Y%m%d)-nightly" qemux86-64
```

### Repository Mirroring

The repository server can be used to mirror packages for offline development or testing environments.

### Integration Testing

The repository structure is compatible with the production repository format, making it suitable for integration testing of package management and deployment workflows.
