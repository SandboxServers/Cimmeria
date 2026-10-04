#!/usr/bin/env bash
# Throwaway, user-authorized native Mac packaging experiment. No game operations.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
proof="$root/crates/launcher/prototype-packaging"
cd "$root"
[ "$(uname -s)" = Darwin ] || { echo 'Run natively on macOS.' >&2; exit 2; }
(cd "$proof/tauri" && npm ci --ignore-scripts && npm run build)
bash tools/build-lane/lane.sh cargo build --locked --manifest-path "$proof/Cargo.toml" --target-dir "$root/target" --release -p cimmeria-egui-proof -p packaging-proof-model
# The lane controls all compilation, including the Cargo invocation inside Tauri CLI.
bash tools/build-lane/lane.sh bash "$proof/tauri-build.sh"
python3 "$proof/package-macos.py"
