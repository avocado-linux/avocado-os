#!/usr/bin/env bash

set -e

echo "Compiling Elixir application"
cd ref-elixir
export MIX_ENV=prod
mix deps.get
mix assets.setup
mix compile
mix assets.deploy
mix release --overwrite
