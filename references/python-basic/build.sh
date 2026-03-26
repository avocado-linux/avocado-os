#!/usr/bin/env bash
set -e

echo "=== Install ==="
avocado install

echo "=== Build ==="
avocado build

echo "=== Provision ==="
avocado provision -r dev
