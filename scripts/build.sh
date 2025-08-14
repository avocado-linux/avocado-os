#!/usr/bin/env bash
set -e

if [ "$1" = "--clean" ]; then
    sudo rm -rf _avocado
    shift
fi
avocado install --force
avocado build
avocado provision --runtime dev
