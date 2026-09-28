#!/usr/bin/env bash
# Build the release launcher in stages. The release workflow
# (.github/workflows/launcher-release.yml) runs these; launcher-build.yml runs
# the same stages on every launcher PR as a dry run, so the release is
# exercised before it is needed.
#
# Usage: tools/launcher-release/build.sh <stage> [args]
#
#   i686             build the 32-bit artifacts: cimmeria-client-patches.dll
#                    and the sgw-start32.exe injection helper
#   launcher         build the 64-bit launcher, embedding both i686 artifacts
#   verify           check the artifacts: bitness, the helper's manifest and
#                    version resource, both embedded in the launcher
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
HELPER="$I686_DIR/sgw-start32.exe"
LAUNCHER="$TARGET_DIR/$PROFILE_DIR/sgw-launcher.exe"
PACKER="$TARGET_DIR/$PROFILE_DIR/pack-client-overlay.exe"

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
  # Separate invocations: building the two together would unify their
  # dependency features, and the helper must link only cimmeria-client-launch.
  cargo build -p cimmeria-client-patches "${PROFILE_FLAG[@]}" --target "$I686"
  cargo build -p cimmeria-start32 "${PROFILE_FLAG[@]}" --target "$I686"
  need "$DLL"; need "$HELPER"
}

stage_launcher() {
  need "$DLL"; need "$HELPER"
  CIMMERIA_CLIENT_PATCHES_DLL="$(winpath "$DLL")" \
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

stage_verify() {
  need "$DLL"; need "$HELPER"; need "$LAUNCHER"
  [ "$(pe_machine "$DLL")" = 0x14c ] || die "$DLL is not 32-bit"
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
  contains "$LAUNCHER" "$HELPER" || die "the launcher does not embed $HELPER"
  echo "verified: 32-bit DLL + helper, both embedded in the 64-bit launcher"
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
  paths) printf '%s\n' "DLL=$DLL" "HELPER=$HELPER" "LAUNCHER=$LAUNCHER" ;;
  *) sed -n '2,20p' "$0" >&2; exit 2 ;;
esac
