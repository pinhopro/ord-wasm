#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
proof_directory="$PWD"
source scripts/rust-build-environment.sh
if [ -x /opt/homebrew/opt/llvm/bin/llvm-ar ]; then
    export AR_wasm32_unknown_unknown=/opt/homebrew/opt/llvm/bin/llvm-ar
fi
mkdir -p artifacts
cargo build --locked --release --target wasm32-unknown-unknown --lib
cp "$CARGO_TARGET_DIR/wasm32-unknown-unknown/release/ord_core_poc.wasm" artifacts/ord_core_poc.wasm
cargo run --locked --release --bin native_vectors > artifacts/native-vectors.json
node scripts/test-node.mjs
python3 scripts/verify-artifact-privacy.py artifacts
