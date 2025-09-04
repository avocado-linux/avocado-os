#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Function to find leaf directories containing RPMs, excluding extension directories
find_rpm_dirs_exclude_extensions() {
    local dir="$1"
    find "$dir" -type d -not -path "*/repodata/*" -not -path "*/target/*-ext" -not -path "*/target/*-ext/*" | while read -r subdir; do
        # Skip extension directories
        if [[ "$subdir" =~ /target/[^/]+-ext(/.*)?$ ]]; then
            continue
        fi
        
        # Check if this directory contains RPMs
        if [ -n "$(find "$subdir" -maxdepth 1 -name "*.rpm" -print -quit)" ]; then
            # Check if this is a leaf directory (no subdirectories with RPMs)
            if [ -z "$(find "$subdir" -mindepth 1 -type d -not -path "*/repodata/*" -exec sh -c '[ -n "$(find \"$0\" -maxdepth 1 -name \"*.rpm\" -print -quit)" ]' {} \; -print -quit)" ]; then
                echo "$subdir"
            fi
        fi
    done
}

# Main script
if [ $# -lt 1 ] || [ $# -gt 3 ]; then
    echo "Usage: $0 <target-deploy-directory> [baseurl] [outputdir]"
    echo "Example: $0 /path/to/target/repo"
    echo "Example: $0 /path/to/target/repo https://repo.example.com/packages/apollo/edge"
    echo "Example: $0 /path/to/target/repo https://repo.example.com/packages/apollo/edge /path/to/metadata/output"
    echo ""
    echo "If baseurl is provided, the repository metadata will reference packages at that URL"
    echo "instead of the local paths. This is useful when packages and metadata are stored separately."
    echo "If outputdir is provided, metadata will be written there instead of alongside the packages."
    echo "This script excludes extension directories (target/*-ext) from metadata generation."
    exit 1
fi

TARGET_DEPLOY_DIR="$1"
BASEURL="$2"
OUTPUTDIR="$3"

if [ ! -d "${TARGET_DEPLOY_DIR}" ]; then
    echo "Error: Target directory ${TARGET_DEPLOY_DIR} not found" >&2
    exit 1
fi

echo "Target deploy directory: ${TARGET_DEPLOY_DIR}"
if [ -n "$BASEURL" ]; then
    echo "Base URL for packages: ${BASEURL}"
fi
if [ -n "$OUTPUTDIR" ]; then
    echo "Output directory for metadata: ${OUTPUTDIR}"
fi
echo "Excluding extension directories from metadata generation"

# Find and process all leaf directories containing RPMs (excluding extensions)
while IFS= read -r rpm_dir; do
    echo "Processing repository: ${rpm_dir}"

    # Determine output directory for this repo
    if [ -n "$OUTPUTDIR" ]; then
        # Calculate relative path from TARGET_DEPLOY_DIR to rpm_dir
        rel_path="${rpm_dir#${TARGET_DEPLOY_DIR}/}"
        output_path="${OUTPUTDIR}/${rel_path}"
        mkdir -p "${output_path}"
    else
        output_path="${rpm_dir}"
    fi

    # Build createrepo_c command with optional baseurl and outputdir
    cmd_args=()

    # if [ -n "$BASEURL" ]; then
    #     # Calculate relative path from TARGET_DEPLOY_DIR to rpm_dir
    #     rel_path="${rpm_dir#${TARGET_DEPLOY_DIR}/}"
    #     # Construct full baseurl for this specific repo directory
    #     full_baseurl="${BASEURL}/${rel_path}"
    #     cmd_args+=(--baseurl "${full_baseurl}")
    # fi

    # Calculate relative path from output_path to rpm_dir for location prefix
    basedir_path=$(realpath --relative-to="${output_path}" "${rpm_dir}")
    echo "DEBUG: rpm_dir=${rpm_dir}"
    echo "DEBUG: output_path=${output_path}"
    echo "DEBUG: basedir_path=${basedir_path}"
    
    # Change to output directory and run createrepo_c with relative paths
    pushd "${output_path}" > /dev/null
    
    if [ -d "repodata" ]; then
        echo "Updating existing repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --update --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    else
        echo "Creating new repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    fi
    
    popd > /dev/null
done < <(find_rpm_dirs_exclude_extensions "${TARGET_DEPLOY_DIR}")

echo "Base repository metadata update complete (extensions excluded)!"
