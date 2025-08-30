#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Default configuration
DEFAULT_REPO_DIR="/tmp/avocado-dev-repo"
DEFAULT_DISTRO_CODENAME="latest/apollo/edge"
DEFAULT_CONTAINER_NAME="avocado-dev-repo"
DEFAULT_NETWORK_NAME="avocado-dev-network"
DEFAULT_REPO_URL="http://localhost:8080"

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
    -u, --repo-url URL      Repository URL (default: $DEFAULT_REPO_URL)
    -n, --container-name NAME   Repository container name (default: $DEFAULT_CONTAINER_NAME)
    --network NETWORK       Docker network name (default: $DEFAULT_NETWORK_NAME)
    --all                   Build all extensions that support the target
    --list                  List available extensions and their supported targets
    -h, --help              Show this help message

Examples:
    $0 -t qemux86-64 --all                     # Build all extensions for qemux86-64
    $0 -t raspberrypi4 docker sshd             # Build specific extensions
    $0 --list                                   # List all extensions
    $0 -t qemux86-64 -u http://repo:8080 dev   # Use custom repo URL

This script:
1. Checks that the repository server is running
2. Discovers available extensions and their supported targets
3. Builds the specified extensions for the target
4. Copies built packages to the repository
5. Updates extension repository metadata

The built extension packages will be available in:
    <repo-dir>/packages/<distro-codename>/target/<target>-ext/

EOF
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
    
    # Test if the repository is accessible
    if ! curl -s --connect-timeout 5 "$REPO_URL" > /dev/null; then
        echo "Error: Repository server at '$REPO_URL' is not accessible" >&2
        echo "Check that the container is running and the URL is correct" >&2
        exit 1
    fi
}

# Function to discover extensions and their supported targets
discover_extensions() {
    local extensions_info=()
    
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
    
    printf '%s\n' "${extensions_info[@]}"
}

# Function to list extensions
list_extensions() {
    echo "Available extensions and their supported targets:"
    echo ""
    
    while IFS=':' read -r extension supported_targets; do
        printf "  %-20s %s\n" "$extension" "$supported_targets"
    done < <(discover_extensions)
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

# Function to build extension
build_extension() {
    local extension="$1"
    local target="$2"
    
    echo "Building extension: $extension for target: $target"
    
    local ext_dir="extensions/$extension"
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
    
    local package_name="avocado-ext-$extension"
    local output_dir="$extension-$target"
    
    echo "  Installing extension environment..."
    avocado ext install -e "$package_name" -f --target "$target" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    echo "  Building extension..."
    avocado ext build -e "$package_name" --target "$target" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    echo "  Packaging extension..."
    avocado ext package -e "$package_name" --target "$target" --out-dir "$output_dir" --container-arg "--network" --container-arg "$NETWORK_NAME"
    
    # Copy packages to repository
    local target_ext_dir="$REPO_DIR/packages/$DISTRO_CODENAME/target/$target-ext"
    echo "  Copying packages to repository: $target_ext_dir"
    mkdir -p "$target_ext_dir"
    
    if [ -d "$output_dir" ]; then
        find "$output_dir" -name "*.rpm" -exec cp {} "$target_ext_dir/" \;
        local rpm_count=$(find "$output_dir" -name "*.rpm" | wc -l)
        echo "  ✓ Copied $rpm_count RPM packages"
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
    ./repo/update-metadata-extensions.sh "$REPO_DIR/packages/$DISTRO_CODENAME" "" "$REPO_DIR/releases/$DISTRO_CODENAME"
    
    if [ $? -eq 0 ]; then
        echo "✓ Extension metadata updated successfully"
    else
        echo "✗ Extension metadata update failed" >&2
        return 1
    fi
}

# Parse command line arguments
REPO_DIR="$DEFAULT_REPO_DIR"
DISTRO_CODENAME="$DEFAULT_DISTRO_CODENAME"
REPO_URL="$DEFAULT_REPO_URL"
CONTAINER_NAME="$DEFAULT_CONTAINER_NAME"
NETWORK_NAME="$DEFAULT_NETWORK_NAME"
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

echo "Target: $TARGET"
echo "Repository directory: $REPO_DIR"
echo "Repository URL: $REPO_URL"
echo "Distribution codename: $DISTRO_CODENAME"
echo "Container name: $CONTAINER_NAME"
echo "Network name: $NETWORK_NAME"
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

for extension in "${EXTENSIONS[@]}"; do
    echo ""
    echo "--- Building $extension ---"
    if ! build_extension "$extension" "$TARGET"; then
        failed_extensions+=("$extension")
        echo "✗ Failed to build extension: $extension"
    fi
done

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
echo "Extension packages available at: $REPO_DIR/packages/$DISTRO_CODENAME/target/$TARGET-ext/"
echo "Repository URL: $REPO_URL/"

if [ ${#failed_extensions[@]} -gt 0 ]; then
    exit 1
fi
