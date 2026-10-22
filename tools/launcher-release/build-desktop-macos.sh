#!/usr/bin/env bash
# Build the desktop launcher (crates/launcher/desktop) for macOS in stages, as
# an ad-hoc signed app (D-LX14). The release workflow
# (.github/workflows/launcher-release.yml, job desktop-macos) runs these on an
# arm64 macOS runner; launcher-desktop.yml runs the same stages on every
# desktop-launcher PR as a dry run. build-desktop.sh beside this file builds the
# Windows zip, and its mac-helpers and mac-pieces stages build the Windows
# pieces this app carries.
#
# Usage: tools/launcher-release/build-desktop-macos.sh <stage> [args]
#
#   pieces <dir> <sha256>
#                    check <dir>/mac-pieces.sha256 against <sha256> (the
#                    Windows job's output, a channel separate from the
#                    artifact), then stage each of the five Windows pieces it
#                    lists through crates/launcher/desktop/tools/stage-helper.py,
#                    which checks each one's digest, architecture and role
#   d3d9             download the pinned D9VK d3d9.dll into the bundled
#                    graphics/ resources and check its size and SHA-256
#   app              build the UI and the .app (release profile), pinned to
#                    the SHA-256 of each staged file; a release exports
#                    CIMMERIA_LAUNCHER_TAG, CIMMERIA_LAUNCHER_BUILD_EPOCH,
#                    CIMMERIA_LAUNCHER_VERSION and LAUNCHER_MANIFEST_PUBKEY_HEX
#   sign             ad-hoc sign the app: codesign --force --deep --sign -,
#                    no hardened runtime, no entitlements
#   verify           check the result: the seal, an ad-hoc signature without
#                    the hardened runtime, an arm64 executable, the bundled
#                    files are the ones it was pinned to, the telemetry DLL is
#                    the player build, and the release stamps reached it
#   package          zip the app with ditto into
#                    artifacts/StargateWorlds-Launcher-macos-arm64.zip, check
#                    the seal again on an unzipped copy, and write a .sha256
#   paths            print the artifact paths
#
# The Wine runtime is not in the app: the launcher downloads it at first
# install with a pinned size and SHA-256 (engine/src/mac_runtime/). The
# optional rosettax87 accelerator is left out, so its two pins stay unset.
# docs/analysis/launcher-consolidation/worknotes/lx-28-redistribution.md has
# the reasons.
set -euo pipefail

[ "$(uname -s)" = Darwin ] || { echo "Run natively on macOS." >&2; exit 2; }

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
case "$TARGET_DIR" in /*) ;; *) TARGET_DIR="$PWD/$TARGET_DIR" ;; esac
DESKTOP=crates/launcher/desktop
DESKTOP_DIR="$TARGET_DIR/desktop"
STAGED="$DESKTOP/shell/resources/windows"
GRAPHICS="$DESKTOP/shell/resources/graphics"
# tauri.conf.json's productName.
APP_NAME="Stargate Worlds Launcher.app"
APP="$DESKTOP_DIR/release/bundle/macos/$APP_NAME"
# The release names (docs/analysis/launcher-consolidation/work-packets.md,
# "Release names"); players and docs link to this exact name on
# launcher-current.
ZIP_NAME="StargateWorlds-Launcher-macos-arm64.zip"
ZIP="artifacts/$ZIP_NAME"

# The five Windows pieces: staged file name, stage-helper.py kind, and the
# build variable that pins it. The archive and prerequisite workers run under
# Wine for Install; the other three are the Play pieces the Windows zip has.
PIECES=(
  "cimmeria-archive-worker.exe archive CIMMERIA_WINDOWS_HELPER_SHA256"
  "cimmeria-prerequisite-worker.exe prerequisite CIMMERIA_PREREQUISITE_HELPER_SHA256"
  "cimmeria-launch-worker.exe launch CIMMERIA_LAUNCH_HELPER_SHA256"
  "cimmeria_client_patches.dll client-patches CIMMERIA_CLIENT_PATCHES_SHA256"
  "cimmeria_client_telemetry.dll client-telemetry CIMMERIA_CLIENT_TELEMETRY_SHA256"
)

# Unmodified D9VK from WoWSilicon v3.2.2 (shell/resources/graphics/README.md).
# zlib-licensed; D9VK-LICENSE.txt ships beside it.
D3D9_URL="https://raw.githubusercontent.com/WoWSilicon/WoWSilicon/5276d92627f26334f6580270eabadf22361d3717/Sources/WoWSiliconSwift/Resources/Patching/d9vk/d3d9.dll"
D3D9_SIZE=3399680
D3D9_SHA256=033cef34aafc409075c764684cd97751903f5850b4f8b9d9c09eb5b6e882045e

# Text only a lab-bridge build of the telemetry DLL carries. Same literal as
# build-desktop.sh and cimmeria_client_telemetry::LAB_BRIDGE_MARKER.
LAB_BRIDGE_MARKER="cimmeria-client-telemetry build flavour: lab-bridge"

die() { echo "::error::$*" >&2; exit 1; }
need() { [ -s "$1" ] || die "$1 was not built"; }
sha() { shasum -a 256 "$1" | cut -d' ' -f1; }

# Return 0 when the file contains the given text, 1 when it does not. Anything
# else (an unreadable file, an empty needle, python failing) ends the build, so
# a "does not contain" check can never pass because the search did not run.
has_text() {
  [ -n "$2" ] || die "has_text: empty search text for $1"
  local rc=0
  python3 -c '
import sys
try:
    data = open(sys.argv[1], "rb").read()
except OSError as error:
    print(f"cannot read {sys.argv[1]}: {error}", file=sys.stderr)
    sys.exit(2)
sys.exit(0 if data.find(sys.argv[2].encode()) >= 0 else 1)
' "$1" "$2" || rc=$?
  [ "$rc" -le 1 ] || die "could not search $1 (exit $rc)"
  return "$rc"
}

plist() { /usr/libexec/PlistBuddy -c "Print :$1" "$APP/Contents/Info.plist"; }

stage_pieces() {
  local dir="${1:?pieces needs the artifact directory}" expected="${2:-}"
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] \
    || die "pieces needs the SHA-256 of mac-pieces.sha256 from the Windows job, got '$expected'"
  local manifest="$dir/mac-pieces.sha256"
  need "$manifest"
  [ "$(sha "$manifest")" = "$expected" ] \
    || die "$manifest is not the one the Windows job wrote"
  [ "$(wc -l < "$manifest" | tr -d ' ')" -eq "${#PIECES[@]}" ] \
    || die "$manifest must list exactly ${#PIECES[@]} pieces"
  # The pieces were built from this commit: both jobs check out the same one.
  local revision; revision="$(git rev-parse HEAD)"
  local name kind var digest
  for entry in "${PIECES[@]}"; do
    read -r name kind var <<<"$entry"
    digest="$(awk -v n="$name" '$2 == n { print $1 }' "$manifest")"
    [[ "$digest" =~ ^[0-9a-f]{64}$ ]] || die "$manifest has no single digest for $name"
    # stage-helper.py refuses a file whose digest is not this one, a wrong
    # architecture or role, and a lab-bridge telemetry DLL.
    python3 "$DESKTOP/tools/stage-helper.py" "$dir/$name" \
      --kind "$kind" --sha256 "$digest" --revision "$revision"
  done
}

stage_d3d9() {
  local out="$GRAPHICS/d3d9.dll"
  curl -fsSL --retry 3 -o "$out" "$D3D9_URL"
  local size; size="$(stat -f %z "$out")"
  [ "$size" -eq "$D3D9_SIZE" ] || die "d3d9.dll is $size bytes, expected $D3D9_SIZE"
  [ "$(sha "$out")" = "$D3D9_SHA256" ] || die "d3d9.dll is not the pinned D9VK build"
  need "$GRAPHICS/D9VK-LICENSE.txt"
  echo "fetched the pinned D9VK d3d9.dll"
}

stage_app() {
  # A workflow passes an unset input or secret as an empty string; the build
  # must see "not stamped", exactly as a local build does.
  local stamp
  for stamp in CIMMERIA_LAUNCHER_TAG CIMMERIA_LAUNCHER_BUILD_EPOCH CIMMERIA_LAUNCHER_VERSION \
    CIMMERIA_BUILD_BRANCH LAUNCHER_MANIFEST_PUBKEY_HEX; do
    [ -n "${!stamp:-}" ] || unset "$stamp"
  done
  # A release carries its tag. Such a build with no manifest key cannot
  # verify any release, and says only "The release could not be verified".
  if [ -n "${CIMMERIA_LAUNCHER_TAG:-}" ] && [ -z "${LAUNCHER_MANIFEST_PUBKEY_HEX:-}" ]; then
    die "a tagged build needs LAUNCHER_MANIFEST_PUBKEY_HEX (the release manifest's public key)"
  fi
  npm ci --ignore-scripts --prefix "$DESKTOP/frontend"
  npm run build --prefix "$DESKTOP/frontend"
  local name kind var
  for entry in "${PIECES[@]}"; do
    read -r name kind var <<<"$entry"
    need "$STAGED/$name"
    echo "pinning $var"
    export "$var=$(sha "$STAGED/$name")"
  done
  need "$GRAPHICS/d3d9.dll"
  export CIMMERIA_D3D9_SHA256="$D3D9_SHA256"
  unset CIMMERIA_ROSETTA_X87_SHA256 CIMMERIA_ROSETTA_X87_LIBRARY_SHA256
  local revision; revision="$(git rev-parse HEAD)"
  export CIMMERIA_DESKTOP_SOURCE_REVISION="$revision"
  export CIMMERIA_BUILD_GIT_SHA="${CIMMERIA_BUILD_GIT_SHA:-$revision}"
  export CIMMERIA_BUILD_BRANCH="${CIMMERIA_BUILD_BRANCH:-$(git rev-parse --abbrev-ref HEAD)}"
  # shell/build.rs puts the version in the binary; the CLI writes Info.plist
  # from its own reading of the config, so it gets the version too.
  local config=()
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    config=(--config "{\"version\":\"$CIMMERIA_LAUNCHER_VERSION\"}")
  fi
  rm -rf "$APP"
  # ${config[@]+...}: an empty array is "unbound" to macOS's bash 3.2 under set -u.
  (cd "$DESKTOP/shell" && CARGO_TARGET_DIR="$DESKTOP_DIR" \
    cargo tauri build --bundles app ${config[@]+"${config[@]}"} -- --locked)
  [ -d "$APP" ] || die "$APP was not built"
}

stage_sign() {
  [ -d "$APP" ] || die "$APP was not built"
  # Ad hoc: no identity, no hardened runtime, no entitlements (D-LX14). The
  # only Mach-O is the launcher executable; the Windows pieces and d3d9.dll
  # are resources the seal covers.
  codesign --force --deep --sign - "$APP"
}

stage_verify() {
  [ -d "$APP" ] || die "$APP was not built"
  codesign --verify --deep --strict --verbose=2 "$APP"
  local details; details="$(codesign -dv "$APP" 2>&1)"
  grep -q '^Signature=adhoc$' <<<"$details" || die "the app is not ad-hoc signed: $details"
  if grep -Eq '^CodeDirectory .*flags=[^ ]*runtime' <<<"$details"; then
    die "the app has the hardened runtime on; D-LX14 leaves it off"
  fi
  local exe; exe="$APP/Contents/MacOS/$(plist CFBundleExecutable)"
  need "$exe"
  [ "$(lipo -archs "$exe")" = arm64 ] || die "$exe is not arm64 only: $(lipo -archs "$exe")"
  local resources="$APP/Contents/Resources" name kind var
  for entry in "${PIECES[@]}"; do
    read -r name kind var <<<"$entry"
    need "$resources/windows/$name"
    [ "$(sha "$resources/windows/$name")" = "$(sha "$STAGED/$name")" ] \
      || die "windows/$name in the app is not the staged file the launcher was pinned to"
    # The digest is compiled in as text; a launcher built before the last
    # staging would refuse the file.
    has_text "$exe" "$(sha "$STAGED/$name")" || die "the launcher is not pinned to this $name ($var)"
  done
  need "$resources/graphics/d3d9.dll"
  [ "$(sha "$resources/graphics/d3d9.dll")" = "$D3D9_SHA256" ] \
    || die "graphics/d3d9.dll in the app is not the pinned D9VK build"
  has_text "$exe" "$D3D9_SHA256" || die "the launcher is not pinned to d3d9.dll (CIMMERIA_D3D9_SHA256)"
  need "$resources/graphics/D9VK-LICENSE.txt"
  [ ! -e "$resources/graphics/rosettax87" ] || die "the app carries rosettax87, which LX-28 leaves out"
  if has_text "$resources/windows/cimmeria_client_telemetry.dll" "$LAB_BRIDGE_MARKER"; then
    die "the telemetry DLL is a lab-bridge build; players must get the build without it"
  fi
  if [ -n "${CIMMERIA_LAUNCHER_TAG:-}" ]; then
    has_text "$exe" "$CIMMERIA_LAUNCHER_TAG" \
      || die "the launcher does not embed its release tag $CIMMERIA_LAUNCHER_TAG"
    echo "launcher embeds release tag $CIMMERIA_LAUNCHER_TAG"
  fi
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    has_text "$exe" "$CIMMERIA_LAUNCHER_VERSION" \
      || die "the launcher does not embed its version $CIMMERIA_LAUNCHER_VERSION"
    # The bundler may drop a SemVer prerelease (a same-day build's -N) from
    # the Finder version, so the base is accepted there; the executable
    # carries the full version, which is what the launcher reports.
    local shown; shown="$(plist CFBundleShortVersionString)"
    [ "$shown" = "$CIMMERIA_LAUNCHER_VERSION" ] || [ "$shown" = "${CIMMERIA_LAUNCHER_VERSION%%-*}" ] \
      || die "Info.plist CFBundleShortVersionString is $shown, not $CIMMERIA_LAUNCHER_VERSION"
    echo "launcher embeds version $CIMMERIA_LAUNCHER_VERSION (Info.plist: $shown)"
  fi
  echo "verified: ad-hoc signed arm64 app pinned to its five Windows pieces and D9VK (player telemetry build)"
}

stage_package() {
  [ -d "$APP" ] || die "$APP was not built"
  mkdir -p artifacts
  rm -f "$ZIP" "$ZIP.sha256"
  # ditto keeps the bundle exactly as signed; a zip that changes any file
  # breaks the seal and macOS reports the app as damaged.
  ditto -c -k --sequesterRsrc --keepParent "$APP" "$ZIP"
  local check; check="$(mktemp -d)"
  ditto -x -k "$ZIP" "$check"
  codesign --verify --deep --strict --verbose=2 "$check/$APP_NAME" \
    || die "the app unzipped from $ZIP fails its seal"
  rm -rf "$check"
  printf '%s  %s\n' "$(sha "$ZIP")" "$ZIP_NAME" > "$ZIP.sha256"
  cat "$ZIP.sha256"
}

case "${1:-}" in
  pieces) shift; stage_pieces "$@" ;;
  d3d9) stage_d3d9 ;;
  app) stage_app ;;
  sign) stage_sign ;;
  verify) stage_verify ;;
  package) stage_package ;;
  paths) printf '%s\n' "APP=$APP" "ZIP=$ZIP" ;;
  *) sed -n '2,39p' "$0" >&2; exit 2 ;;
esac
