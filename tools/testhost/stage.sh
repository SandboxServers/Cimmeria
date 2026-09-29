#!/usr/bin/env bash
# Build everything the sgw-testhost boot tests load, for i686, into one
# directory the tests find (crates/sgw-testhost/tests/).
#
# Usage: tools/testhost/stage.sh [out-dir]
#   out-dir defaults to <target dir>/testhost, where the tests look first.
#
# What it stages:
#   sgw-testhost.exe                 the 32-bit stand-in for SGW.exe
#   sgw-start32.exe                  the launcher's injection helper
#   cimmeria_client_patches.dll
#   cimmeria_client_telemetry.dll    default features
#   lab/cimmeria_client_telemetry.dll   built with --features lab-bridge
#
# The two telemetry builds need separate cargo invocations (features
# unify within one), so the default one is copied out before the
# lab-bridge build overwrites it.
#
# Agents run this through the build lane:
#   bash tools/build-lane/lane.sh bash tools/testhost/stage.sh
set -euo pipefail

TARGET=i686-pc-windows-msvc
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
OUT="${1:-$TARGET_DIR/testhost}"
BUILT="$TARGET_DIR/$TARGET/debug"

cd "$ROOT"
cargo build --target "$TARGET" \
  -p cimmeria-sgw-testhost -p cimmeria-start32 \
  -p cimmeria-client-patches -p cimmeria-client-telemetry

mkdir -p "$OUT/lab"
cp "$BUILT/sgw-testhost.exe" "$BUILT/sgw-start32.exe" \
   "$BUILT/cimmeria_client_patches.dll" "$BUILT/cimmeria_client_telemetry.dll" "$OUT/"

cargo build --target "$TARGET" -p cimmeria-client-telemetry --features lab-bridge
cp "$BUILT/cimmeria_client_telemetry.dll" "$OUT/lab/"

echo "staged into $OUT"
