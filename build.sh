#!/usr/bin/env bash
# Build the Cimmeria server and copy the exe to the project root.
# Native Windows build (docs/building.md); cargo flags pass through, e.g. --release.
set -e
cargo build -p cimmeria-server "$@"

# The output folder follows the profile: --release -> release, --profile <p> -> <p>.
profile=debug
prev=""
for arg in "$@"; do
    case "$arg" in
        --release) profile=release ;;
        --profile=*) profile="${arg#--profile=}" ;;
    esac
    [ "$prev" = "--profile" ] && profile="$arg"
    prev="$arg"
done
[ "$profile" = dev ] && profile=debug

# The build lane (tools/build-lane/lane.sh) may point CARGO_TARGET_DIR at a Dev Drive.
cp "${CARGO_TARGET_DIR:-target}/$profile/cimmeria-server.exe" .
echo "Copied cimmeria-server.exe to project root."
