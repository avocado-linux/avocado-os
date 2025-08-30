#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Default configuration
DEFAULT_REPO_DIR="/tmp/avocado-dev-repo"
DEFAULT_DISTRO_CODENAME="latest/apollo/edge"
DEFAULT_RELEASE_ID="dev-$(date -u '+%Y%m%d-%H%M%S')"

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] <target>

Sync packages from a build target to a development repository.

Arguments:
    target              Target name (e.g., qemux86-64, raspberrypi4)

Options:
    -r, --repo-dir DIR      Repository directory (default: $DEFAULT_REPO_DIR)
    -d, --distro CODENAME   Distribution codename (default: $DEFAULT_DISTRO_CODENAME)
    -i, --release-id ID     Release identifier (default: auto-generated timestamp)
    -b, --build-dir DIR     Build directory (default: build-<target>)
    -h, --help              Show this help message

Examples:
    $0 qemux86-64
    $0 -r /opt/avocado-repo -d latest/apollo/edge -i my-dev-build qemux86-64
    $0 --repo-dir /opt/avocado-repo --release-id stable-build raspberrypi4

This script:
1. Finds the build directory for the target (build-<target>)
2. Reads the avocado-repo.map file from build-<target>/build/tmp/deploy/rpm
3. Syncs packages to the specified repository directory
4. Updates repository metadata for distro packages (excludes extensions)

The repository structure will be:
    <repo-dir>/
    ├── packages/<distro-codename>/     # Aggregated packages
    └── releases/<distro-codename>/     # Repository metadata

EOF
}

# Parse command line arguments
REPO_DIR="$DEFAULT_REPO_DIR"
DISTRO_CODENAME="$DEFAULT_DISTRO_CODENAME"
RELEASE_ID="$DEFAULT_RELEASE_ID"
BUILD_DIR=""
TARGET=""

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
        -b|--build-dir)
            BUILD_DIR="$2"
            shift 2
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
            if [ -z "$TARGET" ]; then
                TARGET="$1"
            else
                echo "Error: Multiple targets specified" >&2
                usage >&2
                exit 1
            fi
            shift
            ;;
    esac
done

# Validate required arguments
if [ -z "$TARGET" ]; then
    echo "Error: Target is required" >&2
    usage >&2
    exit 1
fi

# Set default build directory if not specified
if [ -z "$BUILD_DIR" ]; then
    BUILD_DIR="build-$TARGET"
fi

# Validate build directory exists
if [ ! -d "$BUILD_DIR" ]; then
    echo "Error: Build directory '$BUILD_DIR' not found" >&2
    echo "Have you built the target '$TARGET'? Expected directory: $BUILD_DIR" >&2
    exit 1
fi

# Set up paths
SOURCE_DEPLOY_DIR="$BUILD_DIR/build/tmp/deploy/rpm"
PACKAGES_PATH="$REPO_DIR/packages/$DISTRO_CODENAME"
RELEASES_PATH="$REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_ID"

# Validate source directory exists
if [ ! -d "$SOURCE_DEPLOY_DIR" ]; then
    echo "Error: Source deploy directory '$SOURCE_DEPLOY_DIR' not found" >&2
    echo "Have you completed the build for target '$TARGET'?" >&2
    exit 1
fi

# Validate map file exists
MAP_FILE="$SOURCE_DEPLOY_DIR/avocado-repo.map"
if [ ! -f "$MAP_FILE" ]; then
    echo "Error: Map file not found at '$MAP_FILE'" >&2
    echo "The build may not have completed successfully." >&2
    exit 1
fi

echo "=== Avocado Development Package Sync ==="
echo "Target: $TARGET"
echo "Build directory: $BUILD_DIR"
echo "Source deploy directory: $SOURCE_DEPLOY_DIR"
echo "Repository directory: $REPO_DIR"
echo "Packages path: $PACKAGES_PATH"
echo "Releases path: $RELEASES_PATH"
echo "Distribution codename: $DISTRO_CODENAME"
echo "Release ID: $RELEASE_ID"
echo ""

# Create target directories
echo "Creating target directories..."
mkdir -p "$PACKAGES_PATH"
mkdir -p "$RELEASES_PATH"

# Stage packages using the existing script
echo "Staging packages from build to repository..."
./repo/stage-rpms.sh "$SOURCE_DEPLOY_DIR" "$PACKAGES_PATH" "$DISTRO_CODENAME"

if [ $? -eq 0 ]; then
    echo "✓ Package staging completed successfully"
else
    echo "✗ Package staging failed" >&2
    exit 1
fi

# Update repository metadata (distro packages only, excluding extensions)
echo ""
echo "Updating repository metadata..."
./repo/update-metadata-distro.sh "$PACKAGES_PATH" "" "$RELEASES_PATH"

if [ $? -eq 0 ]; then
    echo "✓ Repository metadata updated successfully"
else
    echo "✗ Repository metadata update failed" >&2
    exit 1
fi

echo ""
echo "=== Sync Complete ==="
echo "Packages synced to: $PACKAGES_PATH"
echo "Metadata generated at: $RELEASES_PATH"
echo ""
echo "Next steps:"
echo "1. Start package repository server: ./scripts/dev-start-repo.sh -r '$REPO_DIR'"
echo "2. Build extensions: ./scripts/dev-build-extensions.sh -r '$REPO_DIR' -t '$TARGET'"
echo ""
