#!/usr/bin/env bash

set -e

echo "Compiling React.js application"
cd ref-reactjs

echo "Installing dependencies..."
npm install

echo "Building React frontend..."
npm run build

echo "React.js application compiled successfully"
