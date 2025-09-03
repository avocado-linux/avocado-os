#!/usr/bin/env bash

# Common configuration and functions for Avocado build scripts
# This file can be sourced by both development scripts and CI/CD workflows

# Default configuration values
export AVOCADO_DEFAULT_DISTRO_CODENAME="latest/apollo/edge"
export AVOCADO_DEFAULT_DISTRO_VERSION="0.1.0"
export AVOCADO_DEFAULT_REPO_BASE="https://repo.avocadolinux.org"

# All supported machine targets
export AVOCADO_ALL_MACHINES=(
    "imx8mp-evk"
    "imx91-frdm"
    "imx93-frdm"
    "imx93-evk"
    "qemuarm64"
    "qemux86-64"
    "reterminal"
    "reterminal-dm"
    "jetson-orin-nano-devkit-nvme"
    "raspberrypi4"
    "raspberrypi5"
)

# Function to log with timestamp
avocado_log() {
    echo "[$(date '+%Y-%m-%d %H:%M:%S')] $*" >&2
}

# Function to log error and exit
avocado_error() {
    avocado_log "ERROR: $*"
    exit 1
}

# Function to validate machine target
avocado_validate_machine() {
    local machine="$1"
    
    for valid_machine in "${AVOCADO_ALL_MACHINES[@]}"; do
        if [ "$machine" = "$valid_machine" ]; then
            return 0
        fi
    done
    
    return 1
}

# Function to generate release date timestamp
avocado_generate_release_date() {
    date -u '+%Y-%m-%d-%H%M%S'
}

# Function to generate development release ID
avocado_generate_dev_release_id() {
    echo "dev-$(date -u '+%Y%m%d-%H%M%S')"
}

# Function to validate build directory structure
avocado_validate_build_dir() {
    local target="$1"
    local build_dir="$2"
    
    if [ ! -d "$build_dir" ]; then
        avocado_log "Build directory '$build_dir' not found for target '$target'"
        return 1
    fi
    
    local deploy_dir="$build_dir/build/tmp/deploy/rpm"
    if [ ! -d "$deploy_dir" ]; then
        avocado_log "Deploy directory '$deploy_dir' not found for target '$target'"
        return 1
    fi
    
    local map_file="$deploy_dir/avocado-repo.map"
    if [ ! -f "$map_file" ]; then
        avocado_log "Map file '$map_file' not found for target '$target'"
        return 1
    fi
    
    return 0
}

# Function to discover extensions and their supported targets
avocado_discover_extensions() {
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

# Function to check if extension supports target
avocado_extension_supports_target() {
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

# Function to get extensions that support a target
avocado_get_extensions_for_target() {
    local target="$1"
    local extensions=()
    
    while IFS=':' read -r extension supported_targets; do
        if avocado_extension_supports_target "$extension" "$target" "$supported_targets"; then
            extensions+=("$extension")
        fi
    done < <(avocado_discover_extensions)
    
    printf '%s\n' "${extensions[@]}"
}

# Function to generate build matrix for GitHub Actions
avocado_generate_build_matrix() {
    local build_all="$1"
    local target_to_build="$2"
    
    local selected_machines=()
    
    if [ "$build_all" = "true" ]; then
        selected_machines=("${AVOCADO_ALL_MACHINES[@]}")
    else
        if avocado_validate_machine "$target_to_build"; then
            selected_machines=("$target_to_build")
        else
            avocado_log "Unknown target selected: $target_to_build"
            return 1
        fi
    fi
    
    # Generate JSON array
    local matrix_json="["
    for i in "${!selected_machines[@]}"; do
        if [ $i -gt 0 ]; then
            matrix_json+=","
        fi
        matrix_json+="\"${selected_machines[$i]}\""
    done
    matrix_json+="]"
    
    echo "$matrix_json"
}

# Function to generate extension build matrix for GitHub Actions
avocado_generate_extension_matrix() {
    local build_all="$1"
    local target_to_build="$2"
    
    # Determine filtered targets (same logic as build matrix)
    local filtered_targets=()
    if [ "$build_all" = "true" ]; then
        filtered_targets=("${AVOCADO_ALL_MACHINES[@]}")
    else
        if avocado_validate_machine "$target_to_build"; then
            filtered_targets=("$target_to_build")
        else
            avocado_log "Unknown target selected: $target_to_build"
            return 1
        fi
    fi
    
    # Generate matrix entries for extensions
    local matrix_includes=()
    
    while IFS=':' read -r extension supported_targets; do
        # Determine which targets this extension supports
        local extension_targets=()
        if [ "$supported_targets" = "*" ]; then
            # Extension supports all targets, use filtered targets
            extension_targets=("${filtered_targets[@]}")
        else
            # Parse the list of specific targets
            local targets_list=$(echo "$supported_targets" | sed 's/\[//g' | sed 's/\]//g' | sed 's/"//g' | sed "s/'//g" | tr ',' '\n')
            
            for target in $targets_list; do
                target=$(echo "$target" | xargs) # trim whitespace
                if [ -n "$target" ]; then
                    # Check if this target is in our filtered targets
                    for filtered_target in "${filtered_targets[@]}"; do
                        if [ "$target" = "$filtered_target" ]; then
                            extension_targets+=("$target")
                            break
                        fi
                    done
                fi
            done
        fi
        
        # Add matrix entries for this extension's supported targets
        for target in "${extension_targets[@]}"; do
            matrix_includes+=("{\"extension\": \"$extension\", \"target\": \"$target\"}")
        done
    done < <(avocado_discover_extensions)
    
    # Create JSON matrix
    local matrix_json="["
    for i in "${!matrix_includes[@]}"; do
        if [ $i -gt 0 ]; then
            matrix_json+=","
        fi
        matrix_json+="${matrix_includes[$i]}"
    done
    matrix_json+="]"
    
    echo "$matrix_json"
}

# Function to setup repository paths (for CI/CD)
avocado_setup_repo_paths() {
    local distro_codename="$1"
    local release_date="$2"
    local repo_base_path="${3:-/home/runner/_cache/repos}"
    
    local staging_base_path="$repo_base_path/staging/$release_date"
    local packages_path="$repo_base_path/packages/$distro_codename"
    local releases_path="$repo_base_path/releases/$distro_codename/$release_date"
    
    echo "staging_base_path=$staging_base_path"
    echo "packages_path=$packages_path"
    echo "releases_path=$releases_path"
}

# Function to check if Docker container exists
avocado_container_exists() {
    local container_name="$1"
    docker ps -a --filter "name=^${container_name}$" --format "{{.Names}}" | grep -q "^${container_name}$"
}

# Function to check if Docker container is running
avocado_container_running() {
    local container_name="$1"
    docker ps --filter "name=^${container_name}$" --filter "status=running" --format "{{.Names}}" | grep -q "^${container_name}$"
}

# Function to check if Docker network exists
avocado_network_exists() {
    local network_name="$1"
    docker network ls --filter "name=^${network_name}$" --format "{{.Name}}" | grep -q "^${network_name}$"
}

# Function to ensure Docker network exists
avocado_ensure_network() {
    local network_name="$1"
    
    if ! avocado_network_exists "$network_name"; then
        avocado_log "Creating Docker network: $network_name"
        docker network create "$network_name"
    fi
}

# Function to check if avocado CLI is available
avocado_check_cli() {
    if ! command -v avocado &> /dev/null; then
        avocado_error "avocado CLI not found. Please install it first:
  curl -fsSL https://github.com/avocadolinux/avocado/releases/latest/download/avocado-linux-amd64 -o /usr/local/bin/avocado
  chmod +x /usr/local/bin/avocado"
    fi
}

# Function to validate repository URL is accessible
avocado_check_repo_url() {
    local repo_url="$1"
    local timeout="${2:-5}"
    
    if ! curl -s --connect-timeout "$timeout" "$repo_url" > /dev/null; then
        return 1
    fi
    
    return 0
}
