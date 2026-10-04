#!/usr/bin/env bash
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
export CARGO_TARGET_DIR="$root/target"
cd "$root/crates/launcher/prototype-packaging/tauri"
cargo tauri build --bundles app -- --locked
