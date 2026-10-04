#!/usr/bin/env bash
# Build the development launcher for this host. Never launches the application.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root"
case "$(uname -s)" in
  Darwin|MINGW*|MSYS*) ;;
  *) echo 'Build the desktop launcher natively on macOS or Windows.' >&2; exit 2 ;;
esac
npm ci --ignore-scripts --prefix crates/launcher/desktop/frontend
npm run build --prefix crates/launcher/desktop/frontend
bash tools/build-lane/lane.sh cargo build --locked --manifest-path crates/launcher/desktop/Cargo.toml \
  -p cimmeria-launcher-desktop --target-dir "$root/target/desktop"
