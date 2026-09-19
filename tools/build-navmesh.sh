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
#   --param K=V      Recast parameter passed through to NavBuilder (repeatable);
#                    run NavBuilder with no arguments for the key list
#   --preset NAME    named parameter set; `castle` = the whole-map Castle set
#                    from docs/engine/navmesh-build-pipeline.md section 6
#
# --param / --preset need the rebuilt NavBuilder (tools/build-navbuilder.ps1).
# The 2026-03 reference binary NavBuilder_d.exe accepts exactly four
# arguments and exits 0 even when it fails.
#
# Environment:
#   CIMMERIA_NAVBUILDER   path to NavBuilder (default: bin64/NavBuilder.exe if
#                         present, else bin64/NavBuilder_d.exe)
#   CIMMERIA_COOKED_PC    CookedPC root
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DEFAULT_NAVBUILDER="$REPO_ROOT/bin64/NavBuilder_d.exe"
[[ -x "$REPO_ROOT/bin64/NavBuilder.exe" ]] && DEFAULT_NAVBUILDER="$REPO_ROOT/bin64/NavBuilder.exe"
NAVBUILDER="${CIMMERIA_NAVBUILDER:-$DEFAULT_NAVBUILDER}"
COOKED="${CIMMERIA_COOKED_PC:-}"

MAP=""
OUT=""
INDEX=""
NAV=""
PROBES=""
SKIP_EXTRACT=0
NAV_PARAMS=()

# Whole-map Castle (World 8): fits Recast's 16-bit vertex AND edge caps with
# ~7 % headroom and keeps the interior probes in one component.
PRESET_CASTLE=(partition=watershed agentHeight=1.8 agentClimb=0.6
  minRegionSize=24 maxSimplificationError=2.5)

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cooked) COOKED="$2"; shift 2 ;;
    --out) OUT="$2"; shift 2 ;;
    --index) INDEX="$2"; shift 2 ;;
    --nav) NAV="$2"; shift 2 ;;
    --probes) PROBES="$2"; shift 2 ;;
    --skip-extract) SKIP_EXTRACT=1; shift ;;
    --param) NAV_PARAMS+=("$2"); shift 2 ;;
    --preset)
      case "$2" in
        castle) NAV_PARAMS+=("${PRESET_CASTLE[@]}") ;;
        *) echo "unknown preset: $2 (known: castle)" >&2; exit 2 ;;
      esac
      shift 2 ;;
    -h|--help) sed -n '2,29p' "${BASH_SOURCE[0]}"; exit 0 ;;
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
# The rebuilt NavBuilder exits 1 usage, 2 internal error, 3 Recast build
# failed (incl. the 16-bit vertex / edge caps), 4 output not writable. The
# 2026-03 reference binary exits 0 on every failure, so the file check stays
# as a second line of defence.
NAV_RC=0
"$NAVBUILDER" chunked "$CHUNKS" "$NAV" nav ${NAV_PARAMS[@]+"${NAV_PARAMS[@]}"} || NAV_RC=$?
if [[ "$NAV_RC" -ne 0 ]]; then
  rm -f "$NAV"
  echo "NavBuilder failed with exit code $NAV_RC — see its ERROR lines above." >&2
  if [[ "$NAV_RC" -eq 1 && ${#NAV_PARAMS[@]} -gt 0 ]]; then
    echo "exit 1 with --param/--preset usually means the old 4-argument NavBuilder_d.exe;" >&2
    echo "build the tunable one with tools/build-navbuilder.ps1 and set CIMMERIA_NAVBUILDER." >&2
  fi
  exit 5
fi
if [[ ! -s "$NAV" ]]; then
  echo "NavBuilder exited 0 but produced no .nav (old reference binary?) — check its log above" >&2
  exit 5
fi

# ── 3. Inspect ────────────────────────────────────────────────────────────
echo "==> nav_inspect"
INSPECT_ARGS=("$NAV")
[[ -n "$PROBES" ]] && INSPECT_ARGS+=(--probes "$PROBES")
cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- "${INSPECT_ARGS[@]}"
