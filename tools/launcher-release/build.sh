#!/usr/bin/env bash
# Build the release launcher in stages. The release workflow
# (.github/workflows/launcher-release.yml) runs these; launcher-build.yml runs
# the same stages on every launcher PR as a dry run, so the release is
# exercised before it is needed.
#
# Usage: tools/launcher-release/build.sh <stage> [args]
#
#   i686             build the 32-bit artifacts: cimmeria-client-patches.dll,
#                    the player build of cimmeria-client-telemetry.dll (no
#                    lab-bridge feature) and the sgw-start32.exe helper
#   launcher         build the 64-bit launcher, embedding the i686 artifacts
#   verify           check the artifacts: bitness, the helper's manifest and
#                    version resource, the telemetry DLL is not a lab-bridge
#                    build, all embedded in the launcher
#   overlay <url>    pack crates/client-patches/overlay; <url> is the release
#                    download base the manifest entry will point at
#   paths            print the artifact paths
#
# PROFILE=release (default) or dev. Output goes to cargo's target dir
# (CARGO_TARGET_DIR if set, else target/).
set -euo pipefail

PROFILE="${PROFILE:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
I686=i686-pc-windows-msvc
case "$PROFILE" in
  release) PROFILE_FLAG=(--release); PROFILE_DIR=release ;;
  dev) PROFILE_FLAG=(); PROFILE_DIR=debug ;;
  *) echo "PROFILE must be release or dev, got $PROFILE" >&2; exit 2 ;;
esac

I686_DIR="$TARGET_DIR/$I686/$PROFILE_DIR"
DLL="$I686_DIR/cimmeria_client_patches.dll"
TELEMETRY_DLL="$I686_DIR/cimmeria_client_telemetry.dll"
HELPER="$I686_DIR/sgw-start32.exe"
LAUNCHER="$TARGET_DIR/$PROFILE_DIR/sgw-launcher.exe"
PACKER="$TARGET_DIR/$PROFILE_DIR/pack-client-overlay.exe"

# Text only a lab-bridge build of the telemetry DLL carries (it logs it at
# boot). Same literal as cimmeria_client_telemetry::LAB_BRIDGE_MARKER; a
# launcher test (client_telemetry_dll.rs) pins that every copy agrees.
LAB_BRIDGE_MARKER="cimmeria-client-telemetry build flavour: lab-bridge"

die() { echo "::error::$*" >&2; exit 1; }
need() { [ -s "$1" ] || die "$1 was not built"; }

# An absolute Windows path, for build-script variables and Windows tools
# (cargo, python, powershell) that do not read MSYS paths.
winpath() {
  local p="$1"
  case "$p" in /*|[A-Za-z]:*) ;; *) p="$PWD/$p" ;; esac
  if command -v cygpath >/dev/null; then cygpath -w "$p"; else echo "$p"; fi
}

stage_i686() {
  # Separate invocations: building them together would unify their
  # dependency features, and the helper must link only cimmeria-client-launch.
  # The telemetry DLL is built with its default features, never lab-bridge:
  # players get it when they opt in to telemetry, and the lab build opens an
  # inbound command port. `verify` checks the result.
  cargo build -p cimmeria-client-patches "${PROFILE_FLAG[@]}" --target "$I686"
  cargo build -p cimmeria-client-telemetry "${PROFILE_FLAG[@]}" --target "$I686"
  cargo build -p cimmeria-start32 "${PROFILE_FLAG[@]}" --target "$I686"
  need "$DLL"; need "$TELEMETRY_DLL"; need "$HELPER"
}

stage_launcher() {
  need "$DLL"; need "$TELEMETRY_DLL"; need "$HELPER"
  CIMMERIA_CLIENT_PATCHES_DLL="$(winpath "$DLL")" \
  CIMMERIA_CLIENT_TELEMETRY_DLL="$(winpath "$TELEMETRY_DLL")" \
  CIMMERIA_START32_EXE="$(winpath "$HELPER")" \
    cargo build -p sgw-launcher "${PROFILE_FLAG[@]}"
  need "$LAUNCHER"; need "$PACKER"
}

# PE machine field: 0x14c = i386, 0x8664 = x86-64.
pe_machine() {
  python - "$(winpath "$1")" <<'PY'
import struct, sys
d = open(sys.argv[1], "rb").read()
pe = struct.unpack_from("<I", d, 0x3C)[0]
assert d[pe:pe + 4] == b"PE\0\0", "not a PE image"
print(hex(struct.unpack_from("<H", d, pe + 4)[0]))
PY
}

contains() {
  python -c "import sys; sys.exit(0 if open(sys.argv[1],'rb').read().find(open(sys.argv[2],'rb').read()) >= 0 else 1)" \
    "$(winpath "$1")" "$(winpath "$2")"
}

# Exit 0 when the file contains the lab-bridge marker.
has_lab_marker() {
  python -c "import sys; sys.exit(0 if open(sys.argv[1],'rb').read().find(sys.argv[2].encode()) >= 0 else 1)" \
    "$(winpath "$1")" "$LAB_BRIDGE_MARKER"
}

stage_verify() {
  need "$DLL"; need "$TELEMETRY_DLL"; need "$HELPER"; need "$LAUNCHER"
  [ "$(pe_machine "$DLL")" = 0x14c ] || die "$DLL is not 32-bit"
  [ "$(pe_machine "$TELEMETRY_DLL")" = 0x14c ] || die "$TELEMETRY_DLL is not 32-bit"
  if has_lab_marker "$TELEMETRY_DLL"; then
    die "$TELEMETRY_DLL is a lab-bridge build; players must get the build without it"
  fi
  [ "$(pe_machine "$HELPER")" = 0x14c ] || die "$HELPER is not 32-bit"
  [ "$(pe_machine "$LAUNCHER")" = 0x8664 ] || die "$LAUNCHER is not 64-bit"
  grep -q asInvoker "$HELPER" || die "$HELPER has no asInvoker manifest"
  if command -v powershell >/dev/null; then
    product=$(powershell -NoProfile -Command \
      "(Get-Item '$(winpath "$HELPER")').VersionInfo.ProductName" | tr -d '\r')
    [ -n "$product" ] || die "$HELPER has no version resource"
    echo "helper version resource: $product"
  fi
  contains "$LAUNCHER" "$DLL" || die "the launcher does not embed $DLL"
  contains "$LAUNCHER" "$TELEMETRY_DLL" || die "the launcher does not embed $TELEMETRY_DLL"
  contains "$LAUNCHER" "$HELPER" || die "the launcher does not embed $HELPER"
  echo "verified: 32-bit DLLs (player telemetry build) + helper, all embedded in the 64-bit launcher"
}

stage_overlay() {
  local base="${1:?overlay needs the release download base URL}"
  need "$PACKER"
  mkdir -p overlay-out
  "$PACKER" --overlay crates/client-patches/overlay --out-dir overlay-out --blob-base-url "$base"
  ls -l overlay-out
}

case "${1:-}" in
  i686) stage_i686 ;;
  launcher) stage_launcher ;;
  verify) stage_verify ;;
  overlay) shift; stage_overlay "$@" ;;
  paths) printf '%s\n' "DLL=$DLL" "TELEMETRY_DLL=$TELEMETRY_DLL" "HELPER=$HELPER" "LAUNCHER=$LAUNCHER" ;;
  *) sed -n '2,20p' "$0" >&2; exit 2 ;;
esac
