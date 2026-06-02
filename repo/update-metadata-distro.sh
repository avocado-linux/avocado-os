#!/usr/bin/env bash

set -e # Exit immediately if a command exits with a non-zero status.

# Function to find leaf directories containing RPMs, excluding extension directories
# (legacy fallback, used only when no map file is supplied).
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

# Read the repository roots declared in the map file. Each `repo=<root>` line names
# a directory that gets ONE repomd; for target repos the packages nest in arch
# subdirs UNDER this root (W1: one `[<machine>-target]` repo @ target/<machine>),
# so createrepo_c runs once at the root and recurses into the subdirs. Extension
# repos (target/*-ext) are handled by update-metadata-extensions.sh and skipped here.
repo_roots_from_map() {
    local map_file="$1"
    while IFS='=' read -r key value || [ -n "$key" ]; do
        [ "$key" = "repo" ] || continue
        # Strip the literal "$releasever/" prefix the bbclass writes, leaving the
        # path relative to TARGET_DEPLOY_DIR (which already includes the codename).
        local rel="${value#\$releasever/}"
        case "$rel" in
            target/*-ext) continue ;;
        esac
        echo "$rel"
    done < "$map_file"
}

# createrepo_c for a single repo root (recurses into any arch subdirs), writing
# metadata to the OUTPUTDIR mirror when provided, with location_href pointing back
# at the package tree.
create_repo_at() {
    local rpm_dir="$1"
    echo "Processing repository: ${rpm_dir}"

    if [ ! -d "${rpm_dir}" ]; then
        echo "Warning: repo root ${rpm_dir} not found, skipping." >&2
        return 0
    fi

    # Determine output directory for this repo
    local output_path
    if [ -n "$OUTPUTDIR" ]; then
        local rel_path="${rpm_dir#${TARGET_DEPLOY_DIR}/}"
        output_path="${OUTPUTDIR}/${rel_path}"
        mkdir -p "${output_path}"
    else
        output_path="${rpm_dir}"
    fi

    # Relative path from the metadata dir back to the package tree, used as the
    # location prefix so repomd references resolve to the packages.
    local basedir_path
    basedir_path=$(realpath --relative-to="${output_path}" "${rpm_dir}")
    echo "DEBUG: rpm_dir=${rpm_dir}"
    echo "DEBUG: output_path=${output_path}"
    echo "DEBUG: basedir_path=${basedir_path}"

    pushd "${output_path}" > /dev/null
    if [ -d "repodata" ]; then
        echo "Updating existing repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --update --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    else
        echo "Creating new repository: packages in ${rpm_dir}, metadata in ${output_path}"
        createrepo_c --outputdir . --location-prefix "${basedir_path}/" "${basedir_path}"
    fi
    popd > /dev/null
}

# Main script
if [ $# -lt 1 ] || [ $# -gt 4 ]; then
    echo "Usage: $0 <target-deploy-directory> [baseurl] [outputdir] [map-file]"
    echo "Example: $0 /path/to/target/repo"
    echo "Example: $0 /path/to/target/repo https://repo.example.com/packages/apollo/edge"
    echo "Example: $0 /path/to/target/repo https://repo.example.com/packages/apollo/edge /path/to/metadata/output"
    echo "Example: $0 /path/to/target/repo \"\" /path/to/metadata/output /path/to/avocado-repo.map"
    echo ""
    echo "If baseurl is provided, the repository metadata will reference packages at that URL"
    echo "instead of the local paths. This is useful when packages and metadata are stored separately."
    echo "If outputdir is provided, metadata will be written there instead of alongside the packages."
    echo "If map-file is provided, one repomd is created per 'repo=<root>' entry (recursing into"
    echo "arch subdirs) — the W1 per-machine layout. Without it, the legacy per-leaf-dir scan runs."
    echo "Extension directories (target/*-ext) are always excluded."
    exit 1
fi

TARGET_DEPLOY_DIR="$1"
BASEURL="$2"
OUTPUTDIR="$3"
MAP_FILE="$4"

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

if [ -n "$MAP_FILE" ]; then
    if [ ! -f "$MAP_FILE" ]; then
        echo "Error: map file ${MAP_FILE} not found" >&2
        exit 1
    fi
    echo "Map-driven: one repomd per repo root declared in ${MAP_FILE}"
    while IFS= read -r rel_root; do
        [ -n "$rel_root" ] || continue
        create_repo_at "${TARGET_DEPLOY_DIR}/${rel_root}"
    done < <(repo_roots_from_map "$MAP_FILE")
else
    echo "No map file supplied; falling back to per-leaf-dir scan"
    while IFS= read -r rpm_dir; do
        create_repo_at "${rpm_dir}"
    done < <(find_rpm_dirs_exclude_extensions "${TARGET_DEPLOY_DIR}")
fi

echo "Base repository metadata update complete (extensions excluded)!"
