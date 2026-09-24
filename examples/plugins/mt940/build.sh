#!/usr/bin/env bash
# Rebuilds the component from `src/` and puts it back in the package.
#
# `wasm32-wasip2` emits a component directly, so there is no second tool in the chain:
#   rustup target add wasm32-wasip2
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --manifest-path src/Cargo.toml --target wasm32-wasip2
cp src/target/wasm32-wasip2/release/stonqs_mt940_reader.wasm reader.wasm
echo "reader.wasm is $(wc -c < reader.wasm) bytes"
