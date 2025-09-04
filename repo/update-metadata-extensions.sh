#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Function to find leaf directories containing RPMs, only in extension directories
find_extension_rpm_dirs() {
    local dir="$1"
    
    # Find all extension directories (target/*-ext)
    find "$dir" -type d -name "*-ext" -path "*/target/*-ext" | while read -r ext_dir; do
        # Verify this is actually a directory and contains RPMs
        if [ -d "$ext_dir" ] && [ -n "$(find "$ext_dir" -maxdepth 1 -name "*.rpm" -type f -print -quit)" ]; then
            # Check if this is a leaf directory (no subdirectories with RPMs)
            has_subdirs_with_rpms=false
            while IFS= read -r -d '' subdir; do
                if [ -n "$(find "$subdir" -maxdepth 1 -name "*.rpm" -type f -print -quit)" ]; then
                    has_subdirs_with_rpms=true
                    break
                fi
            done < <(find "$ext_dir" -mindepth 1 -type d -not -path "*/repodata" -print0)
            
            if [ "$has_subdirs_with_rpms" = false ]; then
                echo "$ext_dir"
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
    echo "This script only processes extension directories (target/*-ext)."
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
echo "Processing only extension directories for metadata generation"

# Find and process all leaf directories containing RPMs (extensions only)
extension_dirs_found=false
while IFS= read -r rpm_dir; do
    extension_dirs_found=true
    echo "Processing extension repository: ${rpm_dir}"

    # Determine output directory for this repo
    if [ -n "$OUTPUTDIR" ]; then
        # Calculate relative path from TARGET_DEPLOY_DIR to rpm_dir
        rel_path="${rpm_dir#${TARGET_DEPLOY_DIR}/}"
        output_path="${OUTPUTDIR}/${rel_path}"
        mkdir -p "${output_path}"
    else
        output_path="${rpm_dir}"
    fi

    # Calculate relative path from output_path to rpm_dir for location prefix
    basedir_path=$(realpath --relative-to="${output_path}" "${rpm_dir}")
    echo "DEBUG: rpm_dir=${rpm_dir}"
    echo "DEBUG: output_path=${output_path}"
    echo "DEBUG: basedir_path=${basedir_path}"
    
    # Change to output directory and run createrepo_c with relative paths
    pushd "${output_path}" > /dev/null
    
    if [ -d "repodata" ]; then
        echo "Updating existing extension repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --update --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    else
        echo "Creating new extension repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    fi
    
    popd > /dev/null
done < <(find_extension_rpm_dirs "${TARGET_DEPLOY_DIR}")

if [ "$extension_dirs_found" = false ]; then
    echo "No extension directories found to process"
else
    echo "Extension repository metadata update complete!"
fi
