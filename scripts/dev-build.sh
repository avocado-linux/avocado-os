#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Default configuration
DEFAULT_REPO_DIR="/tmp/avocado-dev-repo"
DEFAULT_DISTRO_CODENAME="latest/apollo/edge"
DEFAULT_RELEASE_ID="dev-$(date -u '+%Y%m%d-%H%M%S')"
DEFAULT_PORT="8080"

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] <target> [target2] [target3] ...

Main development build script that orchestrates the entire build process.

Arguments:
    target              Target name(s) (e.g., qemux86-64, raspberrypi4)

Options:
    -r, --repo-dir DIR      Repository directory (default: $DEFAULT_REPO_DIR)
    -d, --distro CODENAME   Distribution codename (default: $DEFAULT_DISTRO_CODENAME)
    -i, --release-id ID     Release identifier (default: auto-generated timestamp)
    -p, --port PORT         Repository server port (default: $DEFAULT_PORT)
    --sync-only             Only sync packages, don't start repo or build extensions
    --no-extensions         Skip building extensions
    --extensions EXT1,EXT2  Build only specific extensions (comma-separated)
    --keep-repo             Keep repository server running after build
    --build-dir DIR         Custom build directory pattern (default: build-<target>)
    -h, --help              Show this help message

Examples:
    $0 qemux86-64                           # Build everything for qemux86-64
    $0 qemux86-64 raspberrypi4              # Build for multiple targets
    $0 -i stable-v1.0 qemux86-64            # Use custom release ID
    $0 --sync-only qemux86-64               # Only sync packages
    $0 --no-extensions qemux86-64           # Skip extensions
    $0 --extensions docker,sshd qemux86-64  # Build specific extensions
    $0 --keep-repo qemux86-64               # Keep repo server running

This script performs the following steps for each target:
1. Sync packages from build-<target> to the repository
2. Start the package repository server (if not already running)
3. Build extensions for the target
4. Update extension repository metadata
5. Optionally stop the repository server

The repository will aggregate packages from all targets, allowing you to
build multiple targets and have them all available in a single repository.

EOF
}

# Function to log with timestamp
log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*"
}

# Function to check if target build directory exists
check_target_build() {
    local target="$1"
    local build_dir="$2"
    
    if [ ! -d "$build_dir" ]; then
        log "ERROR: Build directory '$build_dir' not found for target '$target'"
        log "Have you built the target '$target'? Expected directory: $build_dir"
        return 1
    fi
    
    local deploy_dir="$build_dir/build/tmp/deploy/rpm"
    if [ ! -d "$deploy_dir" ]; then
        log "ERROR: Deploy directory '$deploy_dir' not found for target '$target'"
        log "The build may not have completed successfully."
        return 1
    fi
    
    local map_file="$deploy_dir/avocado-repo.map"
    if [ ! -f "$map_file" ]; then
        log "ERROR: Map file '$map_file' not found for target '$target'"
        log "The build may not have completed successfully."
        return 1
    fi
    
    return 0
}

# Function to sync packages for a target
sync_target_packages() {
    local target="$1"
    local build_dir="$2"
    
    log "Syncing packages for target: $target"
    
    ./scripts/dev-sync-packages.sh \
        --repo-dir "$REPO_DIR" \
        --distro "$DISTRO_CODENAME" \
        --release-id "$RELEASE_ID" \
        --build-dir "$build_dir" \
        "$target"
    
    if [ $? -eq 0 ]; then
        log "✓ Package sync completed for target: $target"
        return 0
    else
        log "✗ Package sync failed for target: $target"
        return 1
    fi
}

# Function to start repository server
start_repo_server() {
    log "Starting repository server..."
    
    ./scripts/dev-start-repo.sh \
        --repo-dir "$REPO_DIR" \
        --port "$PORT"
    
    if [ $? -eq 0 ]; then
        log "✓ Repository server started successfully"
        return 0
    else
        log "✗ Repository server failed to start"
        return 1
    fi
}

# Function to build extensions for a target
build_target_extensions() {
    local target="$1"
    
    log "Building extensions for target: $target"
    
    local ext_args=(
        --target "$target"
        --repo-dir "$REPO_DIR"
        --distro "$DISTRO_CODENAME"
        --repo-url "http://localhost:$PORT"
    )
    
    if [ "$BUILD_ALL_EXTENSIONS" = true ]; then
        ext_args+=(--all)
    elif [ ${#SPECIFIC_EXTENSIONS[@]} -gt 0 ]; then
        ext_args+=("${SPECIFIC_EXTENSIONS[@]}")
    else
        ext_args+=(--all)
    fi
    
    ./scripts/dev-build-extensions.sh "${ext_args[@]}"
    
    if [ $? -eq 0 ]; then
        log "✓ Extension build completed for target: $target"
        return 0
    else
        log "✗ Extension build failed for target: $target"
        return 1
    fi
}

# Function to stop repository server
stop_repo_server() {
    log "Stopping repository server..."
    
    ./scripts/dev-start-repo.sh --stop
    
    if [ $? -eq 0 ]; then
        log "✓ Repository server stopped"
        return 0
    else
        log "✗ Failed to stop repository server"
        return 1
    fi
}

# Parse command line arguments
REPO_DIR="$DEFAULT_REPO_DIR"
DISTRO_CODENAME="$DEFAULT_DISTRO_CODENAME"
RELEASE_ID="$DEFAULT_RELEASE_ID"
PORT="$DEFAULT_PORT"
BUILD_DIR_PATTERN=""
TARGETS=()
SYNC_ONLY=false
NO_EXTENSIONS=false
SPECIFIC_EXTENSIONS=()
BUILD_ALL_EXTENSIONS=true
KEEP_REPO=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -r|--repo-dir)
            REPO_DIR="$2"
            shift 2
            ;;
        -d|--distro)
            DISTRO_CODENAME="$2"
            shift 2
            ;;
        -i|--release-id)
            RELEASE_ID="$2"
            shift 2
            ;;
        -p|--port)
            PORT="$2"
            shift 2
            ;;
        --build-dir)
            BUILD_DIR_PATTERN="$2"
            shift 2
            ;;
        --sync-only)
            SYNC_ONLY=true
            shift
            ;;
        --no-extensions)
            NO_EXTENSIONS=true
            shift
            ;;
        --extensions)
            IFS=',' read -ra SPECIFIC_EXTENSIONS <<< "$2"
            BUILD_ALL_EXTENSIONS=false
            shift 2
            ;;
        --keep-repo)
            KEEP_REPO=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        -*)
            echo "Error: Unknown option $1" >&2
            usage >&2
            exit 1
            ;;
        *)
            TARGETS+=("$1")
            shift
            ;;
    esac
done

# Validate required arguments
if [ ${#TARGETS[@]} -eq 0 ]; then
    echo "Error: At least one target is required" >&2
    usage >&2
    exit 1
fi

# Convert relative path to absolute path
REPO_DIR="$(realpath "$REPO_DIR")"

# Create repository directory if it doesn't exist
mkdir -p "$REPO_DIR"

log "=== Avocado Development Build ==="
log "Targets: ${TARGETS[*]}"
log "Repository directory: $REPO_DIR"
log "Distribution codename: $DISTRO_CODENAME"
log "Release ID: $RELEASE_ID"
log "Repository port: $PORT"
log "Sync only: $SYNC_ONLY"
log "No extensions: $NO_EXTENSIONS"
if [ "$BUILD_ALL_EXTENSIONS" = false ]; then
    log "Specific extensions: ${SPECIFIC_EXTENSIONS[*]}"
fi
log "Keep repository: $KEEP_REPO"
log ""

# Validate all targets before starting
log "Validating targets..."
failed_targets=()
for target in "${TARGETS[@]}"; do
    if [ -n "$BUILD_DIR_PATTERN" ]; then
        build_dir="$BUILD_DIR_PATTERN"
    else
        build_dir="build-$target"
    fi
    
    if ! check_target_build "$target" "$build_dir"; then
        failed_targets+=("$target")
    else
        log "✓ Target $target validation passed"
    fi
done

if [ ${#failed_targets[@]} -gt 0 ]; then
    log "ERROR: Validation failed for targets: ${failed_targets[*]}"
    exit 1
fi

log "✓ All targets validated successfully"
log ""

# Sync packages for all targets
log "=== Syncing Packages ==="
sync_failed_targets=()
for target in "${TARGETS[@]}"; do
    if [ -n "$BUILD_DIR_PATTERN" ]; then
        build_dir="$BUILD_DIR_PATTERN"
    else
        build_dir="build-$target"
    fi
    
    if ! sync_target_packages "$target" "$build_dir"; then
        sync_failed_targets+=("$target")
    fi
done

if [ ${#sync_failed_targets[@]} -gt 0 ]; then
    log "ERROR: Package sync failed for targets: ${sync_failed_targets[*]}"
    exit 1
fi

log "✓ Package sync completed for all targets"
log ""

# Exit early if sync-only mode
if [ "$SYNC_ONLY" = true ]; then
    log "=== Sync Complete (sync-only mode) ==="
    log "Packages synced to: $REPO_DIR/packages/$DISTRO_CODENAME"
    log "Metadata generated at: $REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_ID"
    log ""
    log "Next steps:"
    log "1. Start repository server: ./scripts/dev-start-repo.sh -r '$REPO_DIR' -p $PORT"
    log "2. Build extensions: ./scripts/dev-build-extensions.sh -t <target> -r '$REPO_DIR'"
    exit 0
fi

# Start repository server
log "=== Starting Repository Server ==="
if ! start_repo_server; then
    log "ERROR: Failed to start repository server"
    exit 1
fi
log ""

# Build extensions for all targets (unless disabled)
if [ "$NO_EXTENSIONS" = false ]; then
    log "=== Building Extensions ==="
    extension_failed_targets=()
    for target in "${TARGETS[@]}"; do
        if ! build_target_extensions "$target"; then
            extension_failed_targets+=("$target")
        fi
    done
    
    if [ ${#extension_failed_targets[@]} -gt 0 ]; then
        log "WARNING: Extension build failed for targets: ${extension_failed_targets[*]}"
        log "Distro packages are still available"
    else
        log "✓ Extension build completed for all targets"
    fi
    log ""
else
    log "=== Skipping Extensions (--no-extensions) ==="
    log ""
fi

# Stop repository server unless --keep-repo
if [ "$KEEP_REPO" = false ]; then
    log "=== Stopping Repository Server ==="
    stop_repo_server
    log ""
fi

# Final summary
log "=== Build Complete ==="
log "Repository directory: $REPO_DIR"
log "Packages: $REPO_DIR/packages/$DISTRO_CODENAME"
log "Metadata: $REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_ID"

if [ "$KEEP_REPO" = true ]; then
    log "Repository server: http://localhost:$PORT/"
    log ""
    log "To stop the repository server:"
    log "  ./scripts/dev-start-repo.sh --stop"
fi

if [ ${#sync_failed_targets[@]} -gt 0 ] || [ ${#extension_failed_targets[@]} -gt 0 ]; then
    log ""
    log "⚠ Some operations failed:"
    if [ ${#sync_failed_targets[@]} -gt 0 ]; then
        log "  Package sync failed: ${sync_failed_targets[*]}"
    fi
    if [ ${#extension_failed_targets[@]} -gt 0 ]; then
        log "  Extension build failed: ${extension_failed_targets[*]}"
    fi
    exit 1
fi

log "✓ All operations completed successfully"
