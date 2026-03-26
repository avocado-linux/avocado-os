#!/usr/bin/env bash

set -e

echo "Installing Python dependencies..."
uv pip install --target app/packages --python $(which python3) requests paho-mqtt

echo "Python dependencies installed successfully"
