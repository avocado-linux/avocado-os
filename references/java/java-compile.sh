#!/usr/bin/env bash

set -e

echo "Compiling Java application"

cd ref-java

# Create output directories
mkdir -p build/classes
mkdir -p build/jar

# Compile Java source files
echo "Compiling Java sources..."
javac -d build/classes \
    src/main/java/com/avocado/hello/HelloWorld.java

# Create a manifest file for the JAR
echo "Main-Class: com.avocado.hello.HelloWorld" > build/MANIFEST.MF

# Package into a JAR file
echo "Creating JAR file..."
jar cfm build/jar/hello-world.jar build/MANIFEST.MF -C build/classes .

echo "Build complete: ref-java/build/jar/hello-world.jar"
