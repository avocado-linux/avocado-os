#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Script to generate a target-specific JSON fragment for targets.json
# This script analyzes the avocado-repo.map file to determine which repositories
# a specific target uses and creates a JSON fragment for that target.

if [ $# -ne 4 ]; then
    echo "Usage: $0 <source-deploy-directory> <target-name> <output-directory> <releasever>"
    echo "Example: $0 /path/to/build/tmp/deploy/rpm qemux86-64 /path/to/staging latest/apollo/edge"
    exit 1
fi

SOURCE_DEPLOY_DIR=$1
TARGET_NAME=$2
OUTPUT_DIR=$3
releasever=$4

MAP_FILE="${SOURCE_DEPLOY_DIR}/avocado-repo.map"

if [ ! -f "${MAP_FILE}" ]; then
    echo "Error: Map file not found at ${MAP_FILE}" >&2
    exit 1
fi

echo "Generating target fragment for: ${TARGET_NAME}"
echo "Using map file: ${MAP_FILE}"
echo "Output directory: ${OUTPUT_DIR}"
echo "Release version: ${releasever}"

# Create output directory
mkdir -p "${OUTPUT_DIR}"

# Initialize the repositories array
repos=()

# Process mappings from the map file to collect repository paths
while IFS='=' read -r key value || [ -n "$key" ]; do   
    # Skip empty lines or lines without an equals sign
    if [ -z "$key" ] || [ -z "$value" ]; then
        continue
    fi
    
    # Expand variables in the value (like $releasever)
    expanded_value=$(eval "echo \"${value}\"")
    
    # Convert absolute path to relative path by removing the releasever prefix
    # This makes paths relative to the targets.json file location
    relative_path="${expanded_value#${releasever}/}"
    
    source_dir="${SOURCE_DEPLOY_DIR}/${key}"
    
    # Only include repositories that actually exist and have packages
    if [ -d "${source_dir}" ] && [ -n "$(find "${source_dir}" -name "*.rpm" -print -quit)" ]; then
        echo "Found packages in: ${source_dir} -> ${relative_path}"
        repos+=("\"${relative_path}\"")
    fi
done < "${MAP_FILE}"

# Always add the target-specific extension repository (relative path)
target_ext_repo="target/${TARGET_NAME}-ext"
repos+=("\"${target_ext_repo}\"")
echo "Added extension repository: ${target_ext_repo}"

# Generate JSON fragment (compact format)
fragment_file="${OUTPUT_DIR}/${TARGET_NAME}-fragment.json"

# Create the JSON structure (compact, no unnecessary whitespace)
printf '{"' > "${fragment_file}"
printf '%s":[' "${TARGET_NAME}" >> "${fragment_file}"

# Add repositories with minimal spacing
first_repo=true
for repo in "${repos[@]}"; do
    if [ "$first_repo" = false ]; then
        printf "," >> "${fragment_file}"
    fi
    first_repo=false
    printf '%s' "$repo" >> "${fragment_file}"
done

printf ']}' >> "${fragment_file}"

echo "Generated target fragment: ${fragment_file}"
echo "Repositories for ${TARGET_NAME}:"
for repo in "${repos[@]}"; do
    echo "  - $(echo "$repo" | tr -d '"')"
done

echo "Target fragment generation complete!"
