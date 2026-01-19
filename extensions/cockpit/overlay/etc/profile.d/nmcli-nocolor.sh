#!/bin/sh
# Disable colors in nmcli output by default
# This improves readability in many terminal environments and logging contexts

# Set NO_COLOR environment variable to disable color output
export NO_COLOR=1

# Alternative: Create an alias that explicitly disables colors
# Uncomment if you prefer the alias approach instead
# alias nmcli='nmcli --color no'
