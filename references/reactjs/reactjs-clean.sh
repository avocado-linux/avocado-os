#!/usr/bin/env bash

set -e

echo "Cleaning React.js build artifacts"
cd ref-reactjs

# Remove build output
rm -rf dist

# Remove node_modules
rm -rf node_modules

# Remove npm cache artifacts
rm -rf .npm
rm -rf package-lock.json

echo "React.js build artifacts cleaned successfully"
