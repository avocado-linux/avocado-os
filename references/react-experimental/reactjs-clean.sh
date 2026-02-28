#!/usr/bin/env bash
set -e

echo "Cleaning React.js build artifacts"
cd ref-reactjs

rm -rf dist
echo "  Removed dist/"

rm -rf node_modules
echo "  Removed node_modules/"

rm -rf .npm
echo "  Removed .npm/"

rm -rf package-lock.json
echo "  Removed package-lock.json"

echo "React.js build artifacts cleaned successfully"
