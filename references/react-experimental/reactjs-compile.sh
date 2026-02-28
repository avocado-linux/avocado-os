#!/usr/bin/env bash
set -e

echo "Compiling React.js application"
cd ref-reactjs

echo "Installing dependencies..."
npm install

echo "Building React frontend..."
npm run build

# Verify build output
if [ ! -d "dist" ]; then
  echo "ERROR: dist/ directory not found after build"
  exit 1
fi

if [ ! -f "dist/index.html" ]; then
  echo "ERROR: dist/index.html not found after build"
  exit 1
fi

echo "React.js application compiled successfully"
