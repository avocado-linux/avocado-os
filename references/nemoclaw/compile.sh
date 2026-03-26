#!/bin/sh
set -e

OLLAMA_BASE_URL="https://github.com/ollama/ollama/releases/latest/download"

cd "$AVOCADO_BUILD_DIR"

# Download Ollama base binary + JetPack 6 CUDA libraries
if [ ! -f bin/ollama ]; then
    rm -rf bin lib
    echo "Downloading Ollama..."
    curl -fSL -o ollama-base.tar.zst "$OLLAMA_BASE_URL/ollama-linux-arm64.tar.zst"
    tar --zstd -xf ollama-base.tar.zst
    rm ollama-base.tar.zst

    echo "Downloading Ollama JetPack 6 CUDA libraries..."
    curl -fSL -o ollama-jp6.tar.zst "$OLLAMA_BASE_URL/ollama-linux-arm64-jetpack6.tar.zst"
    tar --zstd -xf ollama-jp6.tar.zst
    rm ollama-jp6.tar.zst

    ls -la bin/ollama
fi

# Clone and build NemoClaw CLI
if [ ! -d NemoClaw ]; then
    echo "Cloning NemoClaw..."
    git clone https://github.com/NVIDIA/NemoClaw.git
fi

cd NemoClaw
npm install --production
