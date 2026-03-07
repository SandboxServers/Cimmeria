#!/usr/bin/env bash
# Build the Cimmeria server and copy the exe to the project root.
set -e
cargo build -p cimmeria-server "$@"

# Copy from release or debug depending on build flags
if [[ " $* " == *" --release "* ]]; then
    cp target/x86_64-pc-windows-gnu/release/cimmeria-server.exe .
else
    cp target/x86_64-pc-windows-gnu/debug/cimmeria-server.exe .
fi
echo "Copied cimmeria-server.exe to project root."
