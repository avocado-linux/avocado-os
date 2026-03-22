#!/usr/bin/env bash
set -e

echo "============================================"
echo "Building jetson-stats (jtop)"
echo "============================================"

# Determine target platform from SDK environment variables
TARGET_ARCH="${OECORE_TARGET_ARCH:-aarch64}"
TARGET_OS="${OECORE_TARGET_OS:-linux}"

# Get Python version from target sysroot without executing the binary
PYTHON_VERSION=""

# Method 1: Check python3 symlink target in the sysroot
if [ -L "${SDKTARGETSYSROOT}/usr/bin/python3" ]; then
    LINK_TARGET=$(readlink "${SDKTARGETSYSROOT}/usr/bin/python3")
    PYTHON_VERSION=$(echo "$LINK_TARGET" | grep -oP 'python\K3\.\d+' || true)
fi

# Method 2: Look for python3.X directories in lib
if [ -z "$PYTHON_VERSION" ] && [ -d "${SDKTARGETSYSROOT}/usr/lib" ]; then
    PYTHON_DIR=$(ls -d "${SDKTARGETSYSROOT}/usr/lib/python3"* 2>/dev/null | head -1 || true)
    if [ -n "$PYTHON_DIR" ]; then
        PYTHON_VERSION=$(basename "$PYTHON_DIR" | grep -oP 'python\K3\.\d+' || true)
    fi
fi

if [ -z "$PYTHON_VERSION" ]; then
    echo "Warning: Could not detect Python version from target sysroot, defaulting to 3.12"
    PYTHON_VERSION="3.12"
fi

echo "Target platform: ${TARGET_OS} ${TARGET_ARCH}"
echo "Target Python version: ${PYTHON_VERSION}"

BUILD_DIR="${AVOCADO_BUILD_DIR}/jtop-build"
SITE_PACKAGES="${BUILD_DIR}/lib/python${PYTHON_VERSION}/site-packages"

# Clean up any previous build artifacts
rm -rf "$BUILD_DIR" requirements.lock

mkdir -p "$SITE_PACKAGES"
mkdir -p "${BUILD_DIR}/bin"

# Step 1: Resolve dependencies for the target platform using uv
echo "Resolving dependencies for target platform..."
uv pip compile \
    --python-platform "${TARGET_OS}" \
    --python-version "${PYTHON_VERSION}" \
    --output-file requirements.lock \
    <(echo "jetson-stats")

# Step 2: Install packages for the target using uv
echo "Installing packages to build directory..."
export PYTHONDONTWRITEBYTECODE=1

uv pip install \
    --python-platform "${TARGET_OS}" \
    --python-version "${PYTHON_VERSION}" \
    --target "$SITE_PACKAGES" \
    --no-deps \
    -r requirements.lock

# Clean up
rm -f requirements.lock

echo ""
echo "jetson-stats compiled successfully"
echo "Packages installed to: ${SITE_PACKAGES}"
