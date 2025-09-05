#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Default configuration
DEFAULT_REPO_DIR="/tmp/avocado-dev-repo"
DEFAULT_DISTRO_CODENAME="latest/apollo/edge"
DEFAULT_CONTAINER_NAME="avocado-dev-repo"
DEFAULT_NETWORK_NAME="avocado-dev-network"
DEFAULT_REPO_URL="http://$DEFAULT_CONTAINER_NAME"
DEFAULT_RELEASE_DIR=""  # Empty means auto-detect latest
DEFAULT_SKIP_CLEANUP=false

# Function to show usage
usage() {
    cat << EOF
Usage: $0 [OPTIONS] -t <target> [extension1] [extension2] ...

Build extensions for a specific target using the development repository.

Required:
    -t, --target TARGET     Target name (e.g., qemux86-64, raspberrypi4)

Options:
    -r, --repo-dir DIR      Repository directory (default: $DEFAULT_REPO_DIR)
    -d, --distro CODENAME   Distribution codename (default: $DEFAULT_DISTRO_CODENAME)
    -u, --repo-url URL      Repository URL (default: http://<container-name>)
    -n, --container-name NAME   Repository container name (default: $DEFAULT_CONTAINER_NAME)
    --network NETWORK       Docker network name (default: $DEFAULT_NETWORK_NAME)
    --release-dir DIR       Specific release directory name (default: auto-detect latest)
    --skip-cleanup          Skip cleaning up extension build artifacts after building
    --all                   Build all extensions that support the target
    --list                  List available extensions and their supported targets
    -h, --help              Show this help message

Examples:
    $0 -t qemux86-64 --all                     # Build all extensions for qemux86-64
    $0 -t raspberrypi4 docker sshd             # Build specific extensions
    $0 -t raspberrypi4 bsp-raspberrypi4        # Build BSP extension for raspberrypi4
    $0 --list                                   # List all extensions
    $0 -t qemux86-64 -u http://my-repo-container dev   # Use custom repo container
    $0 -t qemux86-64 --all --skip-cleanup      # Build all extensions but skip cleanup

This script:
1. Checks that the repository server is running
2. Discovers available extensions (both regular and BSP) and their supported targets
3. Builds the specified extensions for the target
4. Copies built packages to the repository
5. Updates extension repository metadata
6. Cleans up extension build artifacts (unless --skip-cleanup is used)

The built extension packages will be available in:
    <repo-dir>/packages/<distro-codename>/target/<target>-ext/

EOF
}

# Function to find the latest release directory
find_latest_release_dir() {
    local releases_base_dir="$REPO_DIR/releases/$DISTRO_CODENAME"
    
    if [ ! -d "$releases_base_dir" ]; then
        echo "Error: Releases directory not found: $releases_base_dir" >&2
        return 1
    fi
    
    # Find the latest timestamped directory (dev-YYYYMMDD-HHMMSS format)
    local latest_dir=$(find "$releases_base_dir" -maxdepth 1 -type d -name "dev-*" | sort -V | tail -n 1)
    
    if [ -z "$latest_dir" ]; then
        echo "Error: No release directories found in $releases_base_dir" >&2
        echo "Expected directories with format: dev-YYYYMMDD-HHMMSS" >&2
        return 1
    fi
    
    echo "$(basename "$latest_dir")"
}

# Function to check if avocado CLI is available
check_avocado_cli() {
    if ! command -v avocado &> /dev/null; then
        echo "Error: avocado CLI not found" >&2
        echo "Please install avocado CLI first:" >&2
        echo "  curl -fsSL https://github.com/avocadolinux/avocado/releases/latest/download/avocado-linux-amd64 -o /usr/local/bin/avocado" >&2
        echo "  chmod +x /usr/local/bin/avocado" >&2
        exit 1
    fi
}

# Function to check if repository server is running
check_repo_server() {
    if ! docker ps --filter "name=^${CONTAINER_NAME}$" --filter "status=running" --format "{{.Names}}" | grep -q "^${CONTAINER_NAME}$"; then
        echo "Error: Repository server container '$CONTAINER_NAME' is not running" >&2
        echo "Start it first with: ./scripts/dev-start-repo.sh -r '$REPO_DIR'" >&2
        exit 1
    fi
    
    # Test if the repository is accessible via localhost port mapping
    # (The avocado CLI will use the container name, but we test via localhost)
    LOCAL_REPO_URL="http://localhost:8080"
    if ! curl -s --connect-timeout 5 "$LOCAL_REPO_URL" > /dev/null; then
        echo "Error: Repository server is not accessible via $LOCAL_REPO_URL" >&2
        echo "Check that the container is running and port 8080 is mapped" >&2
        echo "The avocado CLI will connect to: $REPO_URL" >&2
        exit 1
    fi
    
    echo "✓ Repository server is accessible (avocado CLI will use: $REPO_URL)"
}

# Function to discover extensions and their supported targets
discover_extensions() {
    local extensions_info=()
    
    # Discover regular extensions
    for ext_dir in extensions/*/; do
        if [ -d "$ext_dir" ] && [ -f "$ext_dir/avocado.toml" ]; then
            local extension=$(basename "$ext_dir")
            
            # Read supported_targets from avocado.toml
            local supported_targets=""
            if grep -q '^supported_targets' "$ext_dir/avocado.toml"; then
                supported_targets=$(grep '^supported_targets' "$ext_dir/avocado.toml" | sed 's/supported_targets = //' | tr -d '"' | tr -d "'" | tr -d ' ')
            else
                # Default to all targets if not specified
                supported_targets="*"
            fi
            
            extensions_info+=("$extension:$supported_targets")
        fi
    done
    
    # Discover BSP extensions
    for bsp_dir in bsp/*/; do
        if [ -d "$bsp_dir" ] && [ -f "$bsp_dir/avocado.toml" ]; then
            local bsp_name=$(basename "$bsp_dir")
            local extension="bsp-$bsp_name"
            
            # Read supported_targets from avocado.toml
            local supported_targets=""
            if grep -q '^supported_targets' "$bsp_dir/avocado.toml"; then
                supported_targets=$(grep '^supported_targets' "$bsp_dir/avocado.toml" | sed 's/supported_targets = //' | tr -d '"' | tr -d "'" | tr -d ' ')
            else
                # Default to all targets if not specified
                supported_targets="*"
            fi
            
            extensions_info+=("$extension:$supported_targets")
        fi
    done
    
    printf '%s\n' "${extensions_info[@]}"
}

# Function to list extensions
list_extensions() {
    echo "Available extensions and their supported targets:"
    echo ""
    
    # Separate regular and BSP extensions for better display
    local regular_extensions=()
    local bsp_extensions=()
    
    while IFS=':' read -r extension supported_targets; do
        if [[ "$extension" == bsp-* ]]; then
            bsp_extensions+=("$extension:$supported_targets")
        else
            regular_extensions+=("$extension:$supported_targets")
        fi
    done < <(discover_extensions)
    
    # Display regular extensions
    if [ ${#regular_extensions[@]} -gt 0 ]; then
        echo "Regular Extensions:"
        for ext_info in "${regular_extensions[@]}"; do
            IFS=':' read -r extension supported_targets <<< "$ext_info"
            printf "  %-20s %s\n" "$extension" "$supported_targets"
        done
        echo ""
    fi
    
    # Display BSP extensions
    if [ ${#bsp_extensions[@]} -gt 0 ]; then
        echo "BSP Extensions:"
        for ext_info in "${bsp_extensions[@]}"; do
            IFS=':' read -r extension supported_targets <<< "$ext_info"
            printf "  %-20s %s\n" "$extension" "$supported_targets"
        done
    fi
}

# Function to check if extension supports target
extension_supports_target() {
    local extension="$1"
    local target="$2"
    local supported_targets="$3"
    
    if [ "$supported_targets" = "*" ]; then
        return 0
    fi
    
    # Parse TOML array format: ["target1", "target2"] or comma-separated
    local targets_list=$(echo "$supported_targets" | sed 's/\[//g' | sed 's/\]//g' | sed 's/"//g' | sed "s/'//g" | tr ',' '\n')
    
    for supported_target in $targets_list; do
        supported_target=$(echo "$supported_target" | xargs) # trim whitespace
        if [ "$supported_target" = "$target" ]; then
            return 0
        fi
    done
    
    return 1
}

# Function to get extensions for target
get_extensions_for_target() {
    local target="$1"
    local extensions=()
    
    while IFS=':' read -r extension supported_targets; do
        if extension_supports_target "$extension" "$target" "$supported_targets"; then
            extensions+=("$extension")
        fi
    done < <(discover_extensions)
    
    printf '%s\n' "${extensions[@]}"
}

# Function to clean up extension build artifacts
cleanup_extension() {
    local extension="$1"
    local target="$2"
    
    echo "Cleaning up extension: $extension for target: $target"
    
    # Determine extension directory and package name based on type
    local ext_dir=""
    local package_name=""
    
    if [[ "$extension" == bsp-* ]]; then
        # BSP extension
        local bsp_name="${extension#bsp-}"
        ext_dir="bsp/$bsp_name"
        package_name="avocado-bsp-$bsp_name"
    else
        # Regular extension
        ext_dir="extensions/$extension"
        package_name="avocado-ext-$extension"
    fi
    
    if [ ! -d "$ext_dir" ]; then
        echo "  ⚠ Extension directory '$ext_dir' not found, skipping cleanup" >&2
        return 0
    fi
    
    # Change to extension directory
    cd "$ext_dir"
    
    # Set up environment for avocado CLI (same as build)
    export AVOCADO_SDK_REPO_URL="$REPO_URL"
    export AVOCADO_CONTAINER_NETWORK="$NETWORK_NAME"
    export AVOCADO_SDK_REPO_RELEASE="$DISTRO_CODENAME"
    
    echo "  Cleaning extension environment..."
    if avocado clean --target "$target" 2>/dev/null; then
        echo "  ✓ Extension $extension cleaned successfully"
    else
        echo "  ⚠ Extension $extension cleanup had issues"
    fi
    
    # Return to original directory
    cd - > /dev/null
}

# Function to build extension
build_extension() {
    local extension="$1"
    local target="$2"
    
    echo "Building extension: $extension for target: $target"
    
    # Determine extension directory and package name based on type
    local ext_dir=""
    local package_name=""
    
    if [[ "$extension" == bsp-* ]]; then
        # BSP extension
        local bsp_name="${extension#bsp-}"
        ext_dir="bsp/$bsp_name"
        package_name="avocado-bsp-$bsp_name"
    else
        # Regular extension
        ext_dir="extensions/$extension"
        package_name="avocado-ext-$extension"
    fi
    
    if [ ! -d "$ext_dir" ]; then
        echo "Error: Extension directory '$ext_dir' not found" >&2
        return 1
    fi
    
    if [ ! -f "$ext_dir/avocado.toml" ]; then
        echo "Error: Extension configuration '$ext_dir/avocado.toml' not found" >&2
        return 1
    fi
    
    # Change to extension directory
    cd "$ext_dir"
    
    # Set up environment for avocado CLI
    export AVOCADO_SDK_REPO_URL="$REPO_URL"
    export AVOCADO_CONTAINER_NETWORK="$NETWORK_NAME"
    export AVOCADO_SDK_REPO_RELEASE="$DISTRO_CODENAME"
    
    local output_dir="$extension-$target"
    
    echo "  Installing extension environment..."
    avocado ext install -e "$package_name" -f --target "$target" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    echo "  Building extension..."
    avocado ext build -e "$package_name" --target "$target" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    echo "  Packaging extension..."
    avocado ext package -e "$package_name" --target "$target" --out-dir "$output_dir" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    # Copy packages to repository (both packages and releases directories)
    local packages_target_ext_dir="$REPO_DIR/packages/$DISTRO_CODENAME/target/$target-ext"
    local releases_target_ext_dir="$REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR/target/$target-ext"
    local packages_sdk_dir="$REPO_DIR/packages/$DISTRO_CODENAME/sdk/$target"
    local releases_sdk_dir="$REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR/sdk/$target"
    
    echo "  Copying packages to repository:"
    echo "    Extension packages: $packages_target_ext_dir"
    echo "    Extension releases: $releases_target_ext_dir"
    echo "    SDK packages: $packages_sdk_dir"
    echo "    SDK releases: $releases_sdk_dir"
    
    mkdir -p "$packages_target_ext_dir"
    mkdir -p "$releases_target_ext_dir"
    mkdir -p "$packages_sdk_dir"
    mkdir -p "$releases_sdk_dir"
    
    if [ -d "$output_dir" ]; then
        local regular_rpm_count=0
        local sdk_rpm_count=0
        
        # Process each RPM package individually to handle all_avocadosdk packages specially
        while IFS= read -r -d '' rpm_file; do
            local rpm_basename=$(basename "$rpm_file")
            
            if [[ "$rpm_basename" == *"all_avocadosdk"* ]]; then
                # Copy all_avocadosdk packages to SDK directories
                cp "$rpm_file" "$packages_sdk_dir/"
                cp "$rpm_file" "$releases_sdk_dir/"
                ((sdk_rpm_count++))
                echo "    SDK: $rpm_basename"
            else
                # Copy regular packages to extension directories
                cp "$rpm_file" "$packages_target_ext_dir/"
                cp "$rpm_file" "$releases_target_ext_dir/"
                ((regular_rpm_count++))
                echo "    EXT: $rpm_basename"
            fi
        done < <(find "$output_dir" -name "*.rpm" -print0)
        
        echo "  ✓ Copied $regular_rpm_count extension packages and $sdk_rpm_count SDK packages"
    else
        echo "  ⚠ No output directory found: $output_dir"
    fi
    
    # Return to original directory
    cd - > /dev/null
    
    echo "  ✓ Extension $extension built successfully for $target"
}

# Function to update extension metadata
update_extension_metadata() {
    echo "Updating extension repository metadata..."
    echo "Extension metadata will use relative paths to packages"
    
    # Update metadata for both packages and releases directories
    local packages_dir="$REPO_DIR/packages/$DISTRO_CODENAME"
    local releases_dir="$REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR"
    
    echo "Updating extension packages metadata..."
    ./repo/update-metadata-extensions.sh "$packages_dir" "" "$releases_dir"
    
    if [ $? -eq 0 ]; then
        echo "✓ Extension metadata updated successfully"
    else
        echo "✗ Extension metadata update failed" >&2
        return 1
    fi
    
    echo "Updating SDK packages metadata..."
    ./repo/update-metadata-sdk.sh "$packages_dir" "" "$releases_dir"
    
    if [ $? -eq 0 ]; then
        echo "✓ SDK metadata updated successfully"
    else
        echo "✗ SDK metadata update failed" >&2
        return 1
    fi
}

# Parse command line arguments
REPO_DIR="$DEFAULT_REPO_DIR"
DISTRO_CODENAME="$DEFAULT_DISTRO_CODENAME"
REPO_URL="$DEFAULT_REPO_URL"
CONTAINER_NAME="$DEFAULT_CONTAINER_NAME"
NETWORK_NAME="$DEFAULT_NETWORK_NAME"
RELEASE_DIR="$DEFAULT_RELEASE_DIR"
SKIP_CLEANUP="$DEFAULT_SKIP_CLEANUP"
TARGET=""
EXTENSIONS=()
BUILD_ALL=false
LIST_ONLY=false

while [[ $# -gt 0 ]]; do
    case $1 in
        -t|--target)
            TARGET="$2"
            shift 2
            ;;
        -r|--repo-dir)
            REPO_DIR="$2"
            shift 2
            ;;
        -d|--distro)
            DISTRO_CODENAME="$2"
            shift 2
            ;;
        -u|--repo-url)
            REPO_URL="$2"
            shift 2
            ;;
        -n|--container-name)
            CONTAINER_NAME="$2"
            shift 2
            ;;
        --network)
            NETWORK_NAME="$2"
            shift 2
            ;;
        --release-dir)
            RELEASE_DIR="$2"
            shift 2
            ;;
        --skip-cleanup)
            SKIP_CLEANUP=true
            shift
            ;;
        --all)
            BUILD_ALL=true
            shift
            ;;
        --list)
            LIST_ONLY=true
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
            EXTENSIONS+=("$1")
            shift
            ;;
    esac
done

# Convert relative path to absolute path
REPO_DIR="$(realpath "$REPO_DIR")"

echo "=== Avocado Development Extension Builder ==="

# Handle list-only mode
if [ "$LIST_ONLY" = true ]; then
    list_extensions
    exit 0
fi

# Validate required arguments
if [ -z "$TARGET" ]; then
    echo "Error: Target is required (use -t/--target)" >&2
    usage >&2
    exit 1
fi

# Check prerequisites
check_avocado_cli
check_repo_server

# Determine release directory
if [ -z "$RELEASE_DIR" ]; then
    echo "Auto-detecting latest release directory..."
    RELEASE_DIR=$(find_latest_release_dir)
    if [ $? -ne 0 ]; then
        exit 1
    fi
    echo "Using latest release directory: $RELEASE_DIR"
else
    echo "Using specified release directory: $RELEASE_DIR"
    # Validate that the specified release directory exists
    if [ ! -d "$REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR" ]; then
        echo "Error: Specified release directory does not exist: $REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR" >&2
        exit 1
    fi
fi

echo "Target: $TARGET"
echo "Repository directory: $REPO_DIR"
echo "Distribution codename: $DISTRO_CODENAME"
echo "Release directory: $RELEASE_DIR"
echo "Container name: $CONTAINER_NAME"
echo "Network name: $NETWORK_NAME"
echo "Skip cleanup: $SKIP_CLEANUP"
echo ""
echo "Docker networking configuration:"
echo "  Repository URL (for avocado CLI): $REPO_URL"
echo "  Avocado containers will connect via Docker network: $NETWORK_NAME"
echo "  All avocado commands will use --container-arg --network --container-arg $NETWORK_NAME"
echo ""

# Determine which extensions to build
if [ "$BUILD_ALL" = true ]; then
    echo "Building all extensions that support target: $TARGET"
    mapfile -t EXTENSIONS < <(get_extensions_for_target "$TARGET")
    
    if [ ${#EXTENSIONS[@]} -eq 0 ]; then
        echo "No extensions support target: $TARGET"
        exit 0
    fi
    
    echo "Extensions to build: ${EXTENSIONS[*]}"
elif [ ${#EXTENSIONS[@]} -eq 0 ]; then
    echo "Error: No extensions specified. Use --all or specify extension names" >&2
    usage >&2
    exit 1
fi

echo ""

# Validate that specified extensions support the target
if [ "$BUILD_ALL" = false ]; then
    echo "Validating extensions support target: $TARGET"
    for extension in "${EXTENSIONS[@]}"; do
        local extension_info=""
        while IFS=':' read -r ext_name supported_targets; do
            if [ "$ext_name" = "$extension" ]; then
                extension_info="$supported_targets"
                break
            fi
        done < <(discover_extensions)
        
        if [ -z "$extension_info" ]; then
            echo "Error: Extension '$extension' not found" >&2
            exit 1
        fi
        
        if ! extension_supports_target "$extension" "$TARGET" "$extension_info"; then
            echo "Error: Extension '$extension' does not support target '$TARGET'" >&2
            echo "Supported targets: $extension_info" >&2
            exit 1
        fi
        
        echo "  ✓ $extension supports $TARGET"
    done
    echo ""
fi

# Build extensions
echo "Building ${#EXTENSIONS[@]} extension(s)..."
failed_extensions=()
built_extensions=()

for extension in "${EXTENSIONS[@]}"; do
    echo ""
    echo "--- Building $extension ---"
    if build_extension "$extension" "$TARGET"; then
        built_extensions+=("$extension")
        echo "✓ Successfully built extension: $extension"
    else
        failed_extensions+=("$extension")
        echo "✗ Failed to build extension: $extension"
    fi
done

# Clean up built extensions unless skipped
if [ "$SKIP_CLEANUP" = false ] && [ ${#built_extensions[@]} -gt 0 ]; then
    echo ""
    echo "--- Cleaning up extension build artifacts ---"
    for extension in "${built_extensions[@]}"; do
        cleanup_extension "$extension" "$TARGET"
    done
fi

echo ""

# Update metadata if any extensions were built successfully
if [ ${#failed_extensions[@]} -lt ${#EXTENSIONS[@]} ]; then
    update_extension_metadata
fi

# Report results
echo ""
echo "=== Build Complete ==="
if [ ${#failed_extensions[@]} -eq 0 ]; then
    echo "✓ All ${#EXTENSIONS[@]} extension(s) built successfully"
else
    echo "✓ $((${#EXTENSIONS[@]} - ${#failed_extensions[@]})) extension(s) built successfully"
    echo "✗ ${#failed_extensions[@]} extension(s) failed: ${failed_extensions[*]}"
fi

echo ""
echo "Extension packages available at:"
echo "  Extension packages: $REPO_DIR/packages/$DISTRO_CODENAME/target/$TARGET-ext/"
echo "  Extension releases: $REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR/target/$TARGET-ext/"
echo "  SDK packages: $REPO_DIR/packages/$DISTRO_CODENAME/sdk/$TARGET/"
echo "  SDK releases: $REPO_DIR/releases/$DISTRO_CODENAME/$RELEASE_DIR/sdk/$TARGET/"
echo "Repository URL: $REPO_URL/"

if [ ${#failed_extensions[@]} -gt 0 ]; then
    exit 1
fi
