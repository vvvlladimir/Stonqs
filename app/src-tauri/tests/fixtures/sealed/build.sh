#!/usr/bin/env bash
# Rebuilds the fixture component from `src/`:  rustup target add wasm32-wasip2
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --manifest-path src/Cargo.toml --target wasm32-wasip2
cp src/target/wasm32-wasip2/release/stonqs_sealed_reader.wasm reader.wasm
echo "reader.wasm is $(wc -c < reader.wasm) bytes"
