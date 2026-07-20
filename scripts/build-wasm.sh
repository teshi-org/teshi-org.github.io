#!/usr/bin/env bash
set -euo pipefail

project_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
profile="debug"
release_flag=()

if [[ "${1:-}" == "--release" ]]; then
  profile="release"
  release_flag=(--release)
fi

cd "$project_root"
cargo build --target wasm32-unknown-unknown "${release_flag[@]}"
wasm-bindgen \
  "target/wasm32-unknown-unknown/$profile/teshi_web.wasm" \
  --out-dir web/src/wasm \
  --target web \
  --no-typescript
