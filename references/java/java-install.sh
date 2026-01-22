#!/usr/bin/env bash

# AVOCADO_BUILD_EXT_SYSROOT: The sysroot of the extension being installed into

set -e

echo "Installing Java application into extension"

# Create the target directory
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-java"
mkdir -p "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin"

# Copy the built JAR file
cp ref-java/build/jar/hello-world.jar "$AVOCADO_BUILD_EXT_SYSROOT/usr/lib/ref-java/"

# Create a wrapper script to run the application
cat > "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/hello-java" << 'EOF'
#!/bin/sh
exec java -jar /usr/lib/ref-java/hello-world.jar "$@"
EOF

chmod +x "$AVOCADO_BUILD_EXT_SYSROOT/usr/bin/hello-java"

echo "Java application installed successfully"
echo "Run 'hello-java' on the target to execute"
