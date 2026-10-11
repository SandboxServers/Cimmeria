#!/usr/bin/env bash
# Build the desktop launcher (crates/launcher/desktop) for Windows in stages.
# The release workflow (.github/workflows/launcher-release.yml) runs these;
# launcher-desktop.yml runs the same stages on every desktop-launcher PR as a
# dry run, so the release is exercised before it is needed. build.sh beside
# this file does the same for the original launcher (sgw-launcher).
#
# Usage: tools/launcher-release/build-desktop.sh <stage> [args]
#
#   helpers          build the 32-bit pieces the launcher starts the game
#                    with: cimmeria-launch-worker.exe, cimmeria_client_patches.dll
#                    and the player build of cimmeria_client_telemetry.dll
#                    (no lab-bridge feature)
#   stage            copy them into the launcher's bundled resources through
#                    crates/launcher/desktop/tools/stage-helper.py, which
#                    checks each one's architecture and digest
#   launcher         build the UI and the 64-bit launcher, pinned to the
#                    SHA-256 of each staged file; a release exports
#                    CIMMERIA_LAUNCHER_TAG, CIMMERIA_LAUNCHER_BUILD_EPOCH,
#                    CIMMERIA_LAUNCHER_VERSION, LAUNCHER_MANIFEST_PUBKEY_HEX
#                    and, once the updater key exists, LAUNCHER_UPDATER_PUBKEY
#   verify           check the result: bitness, the telemetry DLL is not a
#                    lab-bridge build, the files beside the launcher are the
#                    ones it was pinned to, and the release stamps reached it
#   package          zip the launcher and its resources into
#                    artifacts/StargateWorlds-Launcher-windows-x64.zip, the
#                    executable at the zip's root as the self-update requires,
#                    with a .sha256 beside it
#   feed-tool        build the release_feed example (always the release
#                    profile) that sign and verify-feed run; never with the
#                    signing key in the environment
#   sign <version> <url> <notes>
#                    Minisign-sign the zip with LAUNCHER_UPDATER_MINISIGN_KEY
#                    (and _PASSWORD) and write artifacts/<zip>.sig and
#                    artifacts/latest.json, the feed offering it from <url>;
#                    runs the built tool, no cargo
#   verify-feed <version>
#                    run the launcher engine's own feed, signature and zip
#                    checks on those files against LAUNCHER_UPDATER_PUBKEY
#   feed-dry-run <version>
#                    feed-tool, then sign and verify-feed with a throwaway key
#                    made here, for the PR dry run, which has no real key
#   paths            print the artifact paths
#
# PROFILE=release (default) or dev. Output goes under cargo's target dir
# (CARGO_TARGET_DIR if set, else target/). The desktop launcher is its own
# workspace, so it and the launch worker build into target/desktop; the two
# DLLs are root-workspace crates and build where build.sh builds them.
#
# The launcher is not one file. It reads the three 32-bit pieces from a
# windows/ directory beside the executable and refuses any whose SHA-256 is
# not the one compiled in, so the package is a zip of that layout, and the
# pins are taken from the staged files in the same run that compiles them in.
set -euo pipefail

PROFILE="${PROFILE:-release}"
TARGET_DIR="${CARGO_TARGET_DIR:-target}"
I686=i686-pc-windows-msvc
DESKTOP=crates/launcher/desktop
MANIFEST="$DESKTOP/Cargo.toml"
case "$PROFILE" in
  release) PROFILE_FLAG=(--release); PROFILE_DIR=release ;;
  dev) PROFILE_FLAG=(); PROFILE_DIR=debug ;;
  *) echo "PROFILE must be release or dev, got $PROFILE" >&2; exit 2 ;;
esac

DESKTOP_DIR="$TARGET_DIR/desktop"
WORKER="$DESKTOP_DIR/$I686/$PROFILE_DIR/cimmeria-launch-worker.exe"
PATCHES_DLL="$TARGET_DIR/$I686/$PROFILE_DIR/cimmeria_client_patches.dll"
TELEMETRY_DLL="$TARGET_DIR/$I686/$PROFILE_DIR/cimmeria_client_telemetry.dll"
STAGED="$DESKTOP/shell/resources/windows"
OUT="$DESKTOP_DIR/$PROFILE_DIR"
LAUNCHER="$OUT/cimmeria-launcher-desktop.exe"
# The release names (docs/analysis/launcher-consolidation/work-packets.md,
# "Release names"). The launcher's updater offers whatever latest.json names,
# but players and docs link to the zip by this exact name on launcher-current.
ZIP_NAME="StargateWorlds-Launcher-windows-x64.zip"
ZIP="artifacts/$ZIP_NAME"
FEED="artifacts/latest.json"

# The staged file name of each piece and the build variable that pins it.
PIECES=(
  "cimmeria-launch-worker.exe CIMMERIA_LAUNCH_HELPER_SHA256"
  "cimmeria_client_patches.dll CIMMERIA_CLIENT_PATCHES_SHA256"
  "cimmeria_client_telemetry.dll CIMMERIA_CLIENT_TELEMETRY_SHA256"
)

# Text only a lab-bridge build of the telemetry DLL carries. Same literal as
# build.sh and cimmeria_client_telemetry::LAB_BRIDGE_MARKER.
LAB_BRIDGE_MARKER="cimmeria-client-telemetry build flavour: lab-bridge"

die() { echo "::error::$*" >&2; exit 1; }
need() { [ -s "$1" ] || die "$1 was not built"; }
sha() { sha256sum "$1" | cut -d' ' -f1; }

# An absolute Windows path, for Windows tools (python) that do not read MSYS
# paths.
winpath() {
  local p="$1"
  case "$p" in /*|[A-Za-z]:*) ;; *) p="$PWD/$p" ;; esac
  if command -v cygpath >/dev/null; then cygpath -w "$p"; else echo "$p"; fi
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

# Exit 0 when the file contains the given text.
has_text() {
  python -c "import sys; sys.exit(0 if open(sys.argv[1],'rb').read().find(sys.argv[2].encode()) >= 0 else 1)" \
    "$(winpath "$1")" "$2"
}

stage_helpers() {
  # Separate invocations, as in build.sh: building the DLLs together would
  # unify their dependency features. The telemetry DLL is built with its
  # default features, never lab-bridge. The launch worker is a binary of the
  # desktop workspace's runtime-probe package.
  cargo build --locked --manifest-path "$MANIFEST" -p cimmeria-runtime-probe \
    --bin cimmeria-launch-worker "${PROFILE_FLAG[@]}" --target "$I686" --target-dir "$DESKTOP_DIR"
  cargo build --locked -p cimmeria-client-patches "${PROFILE_FLAG[@]}" --target "$I686"
  cargo build --locked -p cimmeria-client-telemetry "${PROFILE_FLAG[@]}" --target "$I686"
  need "$WORKER"; need "$PATCHES_DLL"; need "$TELEMETRY_DLL"
}

stage_stage() {
  need "$WORKER"; need "$PATCHES_DLL"; need "$TELEMETRY_DLL"
  local revision; revision="$(git rev-parse HEAD)"
  local kind file
  for pair in "launch $WORKER" "client-patches $PATCHES_DLL" "client-telemetry $TELEMETRY_DLL"; do
    read -r kind file <<<"$pair"
    python "$DESKTOP/tools/stage-helper.py" "$(winpath "$file")" \
      --kind "$kind" --sha256 "$(sha "$file")" --revision "$revision"
  done
}

# Print NAME=<sha256> for each staged piece.
pins() {
  local name var
  for pair in "${PIECES[@]}"; do
    read -r name var <<<"$pair"
    need "$STAGED/$name"
    echo "$var=$(sha "$STAGED/$name")"
  done
}

stage_launcher() {
  # A release carries its tag. Such a build with no manifest key cannot
  # verify any release, and says only "The release could not be verified".
  if [ -n "${CIMMERIA_LAUNCHER_TAG:-}" ] && [ -z "${LAUNCHER_MANIFEST_PUBKEY_HEX:-}" ]; then
    die "a tagged build needs LAUNCHER_MANIFEST_PUBKEY_HEX (the release manifest's public key)"
  fi
  npm ci --ignore-scripts --prefix "$DESKTOP/frontend"
  npm run build --prefix "$DESKTOP/frontend"
  local revision; revision="$(git rev-parse HEAD)"
  local pin
  while read -r pin; do
    echo "pinning $pin"
    export "${pin?}"
  done < <(pins)
  CIMMERIA_DESKTOP_SOURCE_REVISION="$revision" \
  CIMMERIA_BUILD_GIT_SHA="${CIMMERIA_BUILD_GIT_SHA:-$revision}" \
  CIMMERIA_BUILD_BRANCH="${CIMMERIA_BUILD_BRANCH:-$(git rev-parse --abbrev-ref HEAD)}" \
    cargo build --locked --manifest-path "$MANIFEST" -p cimmeria-launcher-desktop \
      "${PROFILE_FLAG[@]}" --target-dir "$DESKTOP_DIR"
  need "$LAUNCHER"
}

stage_verify() {
  need "$LAUNCHER"
  [ "$(pe_machine "$LAUNCHER")" = 0x8664 ] || die "$LAUNCHER is not 64-bit"
  local name var
  for pair in "${PIECES[@]}"; do
    read -r name var <<<"$pair"
    # tauri-build copies the bundled resources beside the executable, which
    # is where the launcher looks for them on Windows.
    need "$OUT/windows/$name"
    [ "$(pe_machine "$OUT/windows/$name")" = 0x14c ] || die "$OUT/windows/$name is not 32-bit"
    [ "$(sha "$OUT/windows/$name")" = "$(sha "$STAGED/$name")" ] \
      || die "$OUT/windows/$name is not the staged file the launcher was pinned to"
    # The digest is compiled in as text; a launcher built before the last
    # staging would refuse the file at Play.
    has_text "$LAUNCHER" "$(sha "$STAGED/$name")" \
      || die "the launcher is not pinned to this $name ($var)"
  done
  if has_text "$OUT/windows/cimmeria_client_telemetry.dll" "$LAB_BRIDGE_MARKER"; then
    die "the telemetry DLL is a lab-bridge build; players must get the build without it"
  fi
  if [ -n "${CIMMERIA_LAUNCHER_TAG:-}" ]; then
    has_text "$LAUNCHER" "$CIMMERIA_LAUNCHER_TAG" \
      || die "the launcher does not embed its release tag $CIMMERIA_LAUNCHER_TAG"
    echo "launcher embeds release tag $CIMMERIA_LAUNCHER_TAG"
  fi
  # A stamp that never reached the compile would leave the package version
  # (the updater compares against it) or a Disabled updater, silently.
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    has_text "$LAUNCHER" "$CIMMERIA_LAUNCHER_VERSION" \
      || die "the launcher does not embed its version $CIMMERIA_LAUNCHER_VERSION"
    echo "launcher embeds version $CIMMERIA_LAUNCHER_VERSION"
  fi
  if [ -n "${LAUNCHER_UPDATER_PUBKEY:-}" ]; then
    has_text "$LAUNCHER" "$LAUNCHER_UPDATER_PUBKEY" \
      || die "the launcher does not embed LAUNCHER_UPDATER_PUBKEY"
    echo "launcher embeds the updater public key"
  fi
  echo "verified: 64-bit launcher pinned to its 32-bit launch worker and DLLs (player telemetry build)"
}

stage_package() {
  need "$LAUNCHER"
  local root="artifacts/package-root"
  rm -rf "$root" "$ZIP" "$ZIP.sha256" "$ZIP.sig" "$FEED"
  mkdir -p "$root"
  cp "$LAUNCHER" "$root/"
  # Everything the bundle configuration ships: windows/ and graphics/.
  cp -R "$OUT/windows" "$OUT/graphics" "$root/"
  # The executable sits at the zip's root, not in a folder: the self-update
  # swap (engine updater/apply/zip_swap.rs) refuses a zip without exactly one
  # root .exe. Files only, sorted, `/` separators.
  python - "$(winpath "$root")" "$(winpath "$ZIP")" <<'PY'
import os, sys, zipfile
root, out = sys.argv[1], sys.argv[2]
with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
    for here, dirs, files in os.walk(root):
        dirs.sort()
        for name in sorted(files):
            path = os.path.join(here, name)
            z.write(path, os.path.relpath(path, root).replace(os.sep, "/"))
PY
  rm -rf "$root"
  need "$ZIP"
  printf '%s  %s\n' "$(sha "$ZIP")" "$ZIP_NAME" > "$ZIP.sha256"
  cat "$ZIP.sha256"
}

# The release-profile example that signs and checks the feed with the
# launcher engine's own code (engine/examples/release_feed.rs). It is always a
# release build, whatever PROFILE says.
FEED_TOOL="$DESKTOP_DIR/release/examples/release_feed"
case "$(uname -s)" in MINGW*|MSYS*|CYGWIN*) FEED_TOOL="$FEED_TOOL.exe" ;; esac

# Built in its own step, before any key is in the environment: compiling it
# runs build scripts and proc macros of its dependencies for the first time,
# and none of them may see the signing key.
stage_feed_tool() {
  cargo build --locked --release --manifest-path "$MANIFEST" -p cimmeria-launcher-engine \
    --example release_feed --target-dir "$DESKTOP_DIR"
  need "$FEED_TOOL"
}

# Runs the already-built tool. No cargo here, so `sign` compiles nothing while
# the key is set.
release_feed() {
  need "$FEED_TOOL"
  "$FEED_TOOL" "$@"
}

stage_sign() {
  local version="${1:?sign needs the version}" url="${2:?sign needs the zip URL}" notes="${3:-}"
  need "$ZIP"
  [ -n "${LAUNCHER_UPDATER_MINISIGN_KEY:-}" ] || die "sign needs LAUNCHER_UPDATER_MINISIGN_KEY"
  release_feed sign "$(winpath "$ZIP")" "$version" "$url" "$notes" "$(winpath artifacts)"
  need "$ZIP.sig"; need "$FEED"
}

stage_verify_feed() {
  local version="${1:?verify-feed needs the version}"
  need "$ZIP"; need "$ZIP.sig"; need "$FEED"
  [ -n "${LAUNCHER_UPDATER_PUBKEY:-}" ] || die "verify-feed needs LAUNCHER_UPDATER_PUBKEY"
  release_feed verify "$(winpath "$FEED")" "$(winpath "$ZIP")" "$(winpath "$ZIP.sig")" \
    "$LAUNCHER_UPDATER_PUBKEY" "$version"
}

stage_feed_dry_run() {
  local version="${1:?feed-dry-run needs the version}"
  stage_feed_tool
  local keys; keys="$(mktemp -d)"
  # Throwaway: made here, used once, deleted. Never a release key.
  export LAUNCHER_UPDATER_MINISIGN_PASSWORD="dry-run-$RANDOM$RANDOM"
  release_feed keygen "$(winpath "$keys")"
  LAUNCHER_UPDATER_MINISIGN_KEY="$(cat "$keys/secret.key")" \
    stage_sign "$version" \
    "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-dry-run/$ZIP_NAME" \
    "Dry run $version"
  LAUNCHER_UPDATER_PUBKEY="$(cat "$keys/public.key")" stage_verify_feed "$version"
  rm -rf "$keys"
}

case "${1:-}" in
  helpers) stage_helpers ;;
  stage) stage_stage ;;
  launcher) stage_launcher ;;
  verify) stage_verify ;;
  package) stage_package ;;
  feed-tool) stage_feed_tool ;;
  sign) shift; stage_sign "$@" ;;
  verify-feed) shift; stage_verify_feed "$@" ;;
  feed-dry-run) shift; stage_feed_dry_run "$@" ;;
  paths) printf '%s\n' "WORKER=$WORKER" "PATCHES_DLL=$PATCHES_DLL" "TELEMETRY_DLL=$TELEMETRY_DLL" "LAUNCHER=$LAUNCHER" "ZIP=$ZIP" "FEED=$FEED" "FEED_TOOL=$FEED_TOOL" ;;
  *) sed -n '2,43p' "$0" >&2; exit 2 ;;
esac
