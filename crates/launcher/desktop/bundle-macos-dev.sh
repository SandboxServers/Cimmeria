#!/usr/bin/env bash
# Invoked ONLY from inside the build lane. Creates a development .app; never opens it.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
[ "$(uname -s)" = Darwin ] || { echo 'Run natively on macOS.' >&2; exit 2; }
export CARGO_TARGET_DIR="$root/target/desktop"
cd "$root/crates/launcher/desktop/shell"
cargo tauri build --debug --bundles app -- --locked
