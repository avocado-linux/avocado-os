#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "============================================"
echo "Installing jetson-stats (jtop) into extension"
echo "============================================"

BUILD_DIR="${AVOCADO_BUILD_DIR}/jtop-build"

if [ ! -d "$BUILD_DIR" ]; then
    echo "Error: Build directory not found at $BUILD_DIR"
    exit 1
fi

# Find the site-packages directory
SITE_PACKAGES=$(find "$BUILD_DIR/lib" -type d -name "site-packages" | head -1)

if [ -z "$SITE_PACKAGES" ]; then
    echo "Error: site-packages not found in $BUILD_DIR"
    exit 1
fi

# Detect Python version from the build layout
PYTHON_VERSION=$(echo "$SITE_PACKAGES" | grep -oP 'python\K3\.\d+')

# Install Python packages
TARGET_SITE_PACKAGES="$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/python${PYTHON_VERSION}/site-packages"
install -d "$TARGET_SITE_PACKAGES"
cp -r "$SITE_PACKAGES"/* "$TARGET_SITE_PACKAGES/"

# Install jtop binary
install -d "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin"

# Find jtop entry point from the installed package
if [ -f "$SITE_PACKAGES/../../../bin/jtop" ]; then
    install -m 755 "$SITE_PACKAGES/../../../bin/jtop" "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/jtop"
else
    # Generate entry point wrapper
    cat > "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/jtop" << 'EOF'
#!/usr/bin/python3
from jtop.__main__ import main
main()
EOF
    chmod 755 "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/jtop"
fi

echo "jetson-stats installed successfully"
