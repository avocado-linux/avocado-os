#!/usr/bin/env bash
set -e

echo "Installing React.js application into extension"

# Verify build artifacts exist
if [ ! -d "ref-reactjs/dist" ]; then
  echo "ERROR: ref-reactjs/dist/ not found — run compile first"
  exit 1
fi

if [ ! -d "ref-reactjs/node_modules" ]; then
  echo "ERROR: ref-reactjs/node_modules/ not found — run compile first"
  exit 1
fi

DEST="$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-reactjs"
mkdir -p "$DEST"

echo "  Copying dist/..."
cp -r ref-reactjs/dist "$DEST/"

echo "  Copying node_modules/..."
cp -r ref-reactjs/node_modules "$DEST/"

echo "  Copying package.json..."
cp ref-reactjs/package.json "$DEST/"

echo "  Copying server.js..."
cp ref-reactjs/server.js "$DEST/"

echo "React.js application installed successfully"
