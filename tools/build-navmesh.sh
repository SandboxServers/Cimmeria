#!/usr/bin/env bash
# Build a `.nav` for one cooked UE3 map: extract → NavBuilder → inspect.
#
# See docs/engine/navmesh-build-pipeline.md for the axis convention, the
# NavBuilder input layout and the failure modes this script guards.
#
# Usage:
#   tools/build-navmesh.sh <MapName> [options]
#
#   --cooked DIR     CookedPC root (default: $CIMMERIA_COOKED_PC)
#   --out DIR        output dir (default: build/navmesh/<MapName>)
#   --index FILE     PackageIndex cache (default: <out>/../package_index.bin)
#   --nav FILE       final .nav path (default: <out>/<mapname>.nav)
#   --probes FILE    probe list handed to nav_inspect
#   --skip-extract   reuse the OBJs already in <out>/chunks
#
# Environment:
#   CIMMERIA_NAVBUILDER   path to NavBuilder_d.exe (default: bin64/NavBuilder_d.exe)
#   CIMMERIA_COOKED_PC    CookedPC root
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NAVBUILDER="${CIMMERIA_NAVBUILDER:-$REPO_ROOT/bin64/NavBuilder_d.exe}"
COOKED="${CIMMERIA_COOKED_PC:-}"

MAP=""
OUT=""
INDEX=""
NAV=""
PROBES=""
SKIP_EXTRACT=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cooked) COOKED="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --index) INDEX="$2"; shift 2 ;;
    --nav) NAV="$2"; shift 2 ;;
    --probes) PROBES="$2"; shift 2 ;;
    --skip-extract) SKIP_EXTRACT=1; shift ;;
    -h|--help) sed -n '2,20p' "${BASH_SOURCE[0]}"; exit 0 ;;
    -*) echo "unknown flag: $1" >&2; exit 2 ;;
    *)
      if [[ -n "$MAP" ]]; then echo "unexpected argument: $1" >&2; exit 2; fi
      MAP="$1"; shift ;;
  esac
done

[[ -n "$MAP" ]] || { echo "usage: tools/build-navmesh.sh <MapName> [options]" >&2; exit 2; }
[[ -n "$COOKED" ]] || { echo "set --cooked or CIMMERIA_COOKED_PC" >&2; exit 2; }

MAP_DIR="$COOKED/Maps/$MAP"
[[ -d "$MAP_DIR" ]] || { echo "no such map directory: $MAP_DIR" >&2; exit 2; }

OUT="${OUT:-$REPO_ROOT/build/navmesh/$MAP}"
INDEX="${INDEX:-$REPO_ROOT/build/navmesh/package_index.bin}"
LOWER="$(echo "$MAP" | tr '[:upper:]' '[:lower:]')"
NAV="${NAV:-$OUT/$LOWER.nav}"
CHUNKS="$OUT/chunks"

mkdir -p "$CHUNKS"

# ── 1. Extract OBJ ────────────────────────────────────────────────────────
# NavBuilder's `chunked` mode globs *.obj and requires every basename to be
# `<8 hex digits>o`; a stray combined `<map>.obj` in the same directory
# leaves MapChunk's position fields uninitialised and the whole build dies
# with "Failed to create heightfield". Chunk OBJs therefore get their own
# subdirectory.
#
# NOTE FOR THE COORDINATOR: this is the single call to the extractor CLI
# (`extract_map`, owned by worker nav-extract). Adjust the argument order
# here if its surface differs; nothing else in this script depends on it.
if [[ "$SKIP_EXTRACT" -eq 0 ]]; then
  echo "==> extracting $MAP"
  cargo run --release -p cimmeria-navmesh-extractor --bin extract_map -- \
    "$COOKED" "$MAP" "$CHUNKS" "$INDEX"
fi

shopt -s nullglob
CHUNK_OBJS=("$CHUNKS"/*.obj)
shopt -u nullglob
if [[ ${#CHUNK_OBJS[@]} -eq 0 ]]; then
  echo "no chunk OBJs in $CHUNKS — extraction produced nothing" >&2
  exit 3
fi
for f in "${CHUNK_OBJS[@]}"; do
  base="$(basename "$f" .obj)"
  if [[ ! "$base" =~ ^[0-9a-fA-F]{8}o$ ]]; then
    echo "refusing to run NavBuilder: '$base.obj' is not a <hex8>o.obj chunk file." >&2
    echo "NavBuilder would read it with uninitialised chunk bounds." >&2
    exit 3
  fi
done

# ── 2. NavBuilder ─────────────────────────────────────────────────────────
[[ -x "$NAVBUILDER" ]] || { echo "NavBuilder not found at $NAVBUILDER (set CIMMERIA_NAVBUILDER)" >&2; exit 4; }
echo "==> NavBuilder chunked ($((${#CHUNK_OBJS[@]})) chunk OBJs)"
rm -f "$NAV"
# NavBuilder exits 0 even when it writes nothing, so the file check below is
# the real success test — see builder.cpp::exportNavmesh (returns void on
# every failure path).
"$NAVBUILDER" chunked "$CHUNKS" "$NAV" nav
if [[ ! -s "$NAV" ]]; then
  echo "NavBuilder produced no .nav (it still exits 0 — check its log above)" >&2
  exit 5
fi

# ── 3. Inspect ────────────────────────────────────────────────────────────
echo "==> nav_inspect"
INSPECT_ARGS=("$NAV")
[[ -n "$PROBES" ]] && INSPECT_ARGS+=(--probes "$PROBES")
cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- "${INSPECT_ARGS[@]}"
