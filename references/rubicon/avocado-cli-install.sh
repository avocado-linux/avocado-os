#!/bin/bash
echo "Installing avocado-cli into extension"
install -D -m 755 avocado-cli/target/*/release/avocado /usr/bin/avocado

