#!/bin/bash

echo "🚀 Starting optimization..."
echo "This may take 10-20 minutes on first run..."
echo ""

docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  -e RUSTFLAGS='-C link-arg=-s' \
  cosmwasm/workspace-optimizer:0.17.0

echo ""
echo "✅ Optimization complete! Check ./artifacts/ for .wasm files"
