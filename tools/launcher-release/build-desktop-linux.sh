#!/usr/bin/env bash
# Build the desktop launcher (crates/launcher/desktop) for Linux x86-64 in
# stages, as an AppImage. The release workflow
# (.github/workflows/launcher-release.yml, job desktop-linux) runs these on an
# Ubuntu runner through launcher-desktop-linux.yml; launcher-desktop.yml runs the
# same stages on every desktop-launcher PR as a dry run.
#
# Usage: tools/launcher-release/build-desktop-linux.sh <stage>
#
#   tools            put the three AppImage tools the Tauri bundler would
#                    download into its cache, each checked against its pinned
#                    SHA-256 (app runs this first)
#   app              build the UI and the AppImage (release profile); a release
#                    exports CIMMERIA_LAUNCHER_TAG, CIMMERIA_LAUNCHER_BUILD_EPOCH,
#                    CIMMERIA_LAUNCHER_VERSION and LAUNCHER_MANIFEST_PUBKEY_HEX
#   verify           check the result: an x86-64 ELF AppImage whose extracted
#                    launcher is x86-64, embeds the four pinned Windows pieces (when
#                    staged) and runs --verify-embedded-payload, carries no loose
#                    Windows executable or DLL, embeds the release stamps, and
#                    unpacks a RAR (--self-test-rar)
#   package          copy it to
#                    artifacts/StargateWorlds-Launcher-linux-x86_64.AppImage,
#                    executable, and write a .sha256
#   pieces <dir> <sha256>
#                    check <dir>/mac-pieces.sha256 against <sha256> (the
#                    Windows job's output, a channel separate from the
#                    artifact), then stage the four Windows pieces it lists
#                    through crates/launcher/desktop/tools/stage-helper.py
#   paths            print the artifact paths
#
# The Linux launcher embeds the four Windows pieces the Windows zip and the
# macOS app carry (D-PP8), pinned by SHA-256 and staged from the Windows job's
# artifact, so they exist only inside the launcher binary. A run with no
# artifact (a developer build) builds without the payload and says so; a tagged
# build without the pieces fails in shell/build_payload.rs.
set -euo pipefail

[ "$(uname -s)" = Linux ] || { echo "Run natively on Linux." >&2; exit 2; }

TARGET_DIR="${CARGO_TARGET_DIR:-target}"
case "$TARGET_DIR" in /*) ;; *) TARGET_DIR="$PWD/$TARGET_DIR" ;; esac
DESKTOP=crates/launcher/desktop
DESKTOP_DIR="$TARGET_DIR/desktop"
BUNDLE_DIR="$DESKTOP_DIR/release/bundle/appimage"
# The release name (docs/analysis/launcher-consolidation/work-packets.md,
# "Release names", has the others); players and docs link to this exact name
# on launcher-current.
APPIMAGE_NAME="StargateWorlds-Launcher-linux-x86_64.AppImage"
APPIMAGE="artifacts/$APPIMAGE_NAME"
# The shell's Cargo package and binary.
EXE_NAME=cimmeria-launcher-desktop
STAGED="$DESKTOP/shell/resources/windows"

# The four Windows pieces the launcher embeds: staged file name, stage-helper.py
# kind, and the build variable that pins it. The archive worker and the mac
# helpers are not Linux's; the macOS app's list is in build-desktop-macos.sh.
PIECES=(
  "cimmeria-launch-worker.exe launch CIMMERIA_LAUNCH_HELPER_SHA256"
  "cimmeria-prerequisite-worker.exe prerequisite CIMMERIA_PREREQUISITE_HELPER_SHA256"
  "cimmeria_client_patches.dll client-patches CIMMERIA_CLIENT_PATCHES_SHA256"
  "cimmeria_client_telemetry.dll client-telemetry CIMMERIA_CLIENT_TELEMETRY_SHA256"
)

# The tools tauri-bundler 2.10.1 (tauri-cli 2.12.1) fetches to build an
# AppImage: cache file name, URL, SHA-256. It skips a download when the file is
# already in its cache, so staging checked copies first pins them. The
# appimage plugin comes from upstream's moving `continuous` tag and provides
# the runtime every shipped AppImage starts with; when upstream moves it, this
# check fails and the pin is updated on purpose. Bump all three with the CLI.
TAURI_TOOLS="${XDG_CACHE_HOME:-$HOME/.cache}/tauri"
APPIMAGE_TOOLS=(
  "AppRun-x86_64 https://github.com/tauri-apps/binary-releases/releases/download/apprun-old/AppRun-x86_64 f30140a43a0a59e46db21bdefdf749b9e9f2c6946e92afabbacf98b8ae73fb4f"
  "linuxdeploy-07333c6-x86_64.AppImage https://github.com/tauri-apps/binary-releases/releases/download/linuxdeploy-07333c6/linuxdeploy-x86_64.AppImage 36a2d7e274d12e1050d0e9ecfe11d339ed54720b2bec464c286d53f8b07f5c62"
  "linuxdeploy-plugin-appimage.AppImage https://github.com/linuxdeploy/linuxdeploy-plugin-appimage/releases/download/continuous/linuxdeploy-plugin-appimage-x86_64.AppImage 49d6a17160675a6bd1781699aae6bdf7692d98552e02a3671d2183d10547842e"
)

die() { echo "::error::$*" >&2; exit 1; }
need() { [ -e "$1" ] || die "$1 is missing"; }
sha() { sha256sum "$1" | cut -d' ' -f1; }
has_text() { LC_ALL=C grep -aqF -- "$2" "$1"; }

# The one AppImage the bundler wrote; its file name carries the version.
built() {
  shopt -s nullglob
  local found=("$BUNDLE_DIR"/*.AppImage)
  shopt -u nullglob
  [ ${#found[@]} -eq 1 ] || die "expected one AppImage in $BUNDLE_DIR, found ${#found[@]}"
  printf '%s\n' "${found[0]}"
}

stage_tools() {
  mkdir -p "$TAURI_TOOLS"
  local entry name url want got tmp
  for entry in "${APPIMAGE_TOOLS[@]}"; do
    read -r name url want <<<"$entry"
    if [ -f "$TAURI_TOOLS/$name" ] && [ "$(sha "$TAURI_TOOLS/$name")" = "$want" ]; then
      echo "pinned $name (cached)"
      continue
    fi
    tmp="$(mktemp "$TAURI_TOOLS/.$name.XXXXXX")"
    curl -sSfL --retry 3 -o "$tmp" "$url" || { rm -f "$tmp"; die "cannot download $url"; }
    got="$(sha "$tmp")"
    if [ "$got" != "$want" ]; then
      rm -f "$tmp"
      die "$name from $url is $got, not the pinned $want"
    fi
    chmod 755 "$tmp"
    mv -f "$tmp" "$TAURI_TOOLS/$name"
    echo "pinned $name"
  done
}

staged_pieces() {
  local name kind var
  for entry in "${PIECES[@]}"; do
    read -r name kind var <<<"$entry"
    [ -s "$STAGED/$name" ] || return 1
  done
}

stage_pieces() {
  local dir="${1:?pieces needs the artifact directory}" expected="${2:-}"
  [[ "$expected" =~ ^[0-9a-f]{64}$ ]] \
    || die "pieces needs the SHA-256 of mac-pieces.sha256 from the Windows job, got '$expected'"
  local manifest="$dir/mac-pieces.sha256"
  need "$manifest"
  [ "$(sha "$manifest")" = "$expected" ] \
    || die "$manifest is not the one the Windows job wrote"
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
  stage_tools
  npm ci --ignore-scripts --prefix "$DESKTOP/frontend"
  npm run build --prefix "$DESKTOP/frontend"
  local revision; revision="$(git rev-parse HEAD)"
  export CIMMERIA_DESKTOP_SOURCE_REVISION="$revision"
  export CIMMERIA_BUILD_GIT_SHA="${CIMMERIA_BUILD_GIT_SHA:-$revision}"
  export CIMMERIA_BUILD_BRANCH="${CIMMERIA_BUILD_BRANCH:-$(git rev-parse --abbrev-ref HEAD)}"
  # shell/build.rs puts the version in the binary; the CLI names and stamps the
  # AppImage from its own reading of the config, so it gets the version too.
  # Staged pieces are pinned and embedded (CIMMERIA_EMBED_PAYLOAD=1); none
  # staged is a build without the payload, as a developer build has.
  local name kind var embed=0
  if staged_pieces; then
    for entry in "${PIECES[@]}"; do
      read -r name kind var <<<"$entry"
      echo "pinning $var"
      export "$var=$(sha "$STAGED/$name")"
    done
    export CIMMERIA_EMBED_PAYLOAD=1
    embed=1
  else
    echo "no Windows pieces staged: building without the payload"
  fi
  local parts=()
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    parts+=("\"version\":\"$CIMMERIA_LAUNCHER_VERSION\"")
  fi
  # The pieces live in the binary: the bundler must not copy them beside it,
  # where the AppImage would carry them loose (D-PP8).
  if [ "$embed" = 1 ]; then
    parts+=('"bundle":{"resources":{"resources/windows/":null}}')
  fi
  local config=()
  if [ ${#parts[@]} -gt 0 ]; then
    config=(--config "{$(IFS=,; echo "${parts[*]}")}")
  fi
  rm -rf "$BUNDLE_DIR"
  # linuxdeploy is itself an AppImage; extract-and-run needs no FUSE, which
  # containers and some runners lack.
  (cd "$DESKTOP/shell" && CARGO_TARGET_DIR="$DESKTOP_DIR" APPIMAGE_EXTRACT_AND_RUN=1 \
    cargo tauri build --bundles appimage ${config[@]+"${config[@]}"} -- --locked)
  built >/dev/null
}

stage_verify() {
  local image; image="$(built)"
  file -b "$image" | grep -q 'ELF 64-bit LSB .*x86-64' || die "$image is not an x86-64 ELF: $(file -b "$image")"
  local check; check="$(mktemp -d)"
  # --appimage-extract runs the AppImage's own runtime: it unpacks
  # squashfs-root/ into the current directory and starts nothing else.
  (cd "$check" && APPIMAGE_EXTRACT_AND_RUN=1 "$image" --appimage-extract >/dev/null) \
    || die "$image does not extract"
  local root="$check/squashfs-root" exe
  exe="$root/usr/bin/$EXE_NAME"
  need "$exe"
  file -b "$exe" | grep -q 'ELF 64-bit LSB .*x86-64' || die "$exe is not x86-64: $(file -b "$exe")"
  need "$root/AppRun"
  local name kind var
  if staged_pieces; then
    for entry in "${PIECES[@]}"; do
      read -r name kind var <<<"$entry"
      has_text "$exe" "$(sha "$STAGED/$name")" || die "the launcher is not pinned to this $name ($var)"
    done
    # The launcher checks its own payload: the pieces extract and match their pins.
    "$exe" --verify-embedded-payload "$check/payloads" \
      || die "the launcher does not verify its embedded Windows pieces"
    echo "launcher embeds the four Windows pieces"
  else
    echo "no Windows pieces staged: the AppImage carries no payload"
  fi
  # Windows pieces are embedded in the launcher binary, never loose (D-PP8).
  if find "$root" \( -iname '*.exe' -o -iname '*.dll' \) -print -quit | grep -q .; then
    die "the AppImage carries Windows executables or DLLs: $(find "$root" \( -iname '*.exe' -o -iname '*.dll' \) | head -3)"
  fi
  if [ -n "${CIMMERIA_LAUNCHER_TAG:-}" ]; then
    has_text "$exe" "$CIMMERIA_LAUNCHER_TAG" \
      || die "the launcher does not embed its release tag $CIMMERIA_LAUNCHER_TAG"
    echo "launcher embeds release tag $CIMMERIA_LAUNCHER_TAG"
  fi
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    has_text "$exe" "$CIMMERIA_LAUNCHER_VERSION" \
      || die "the launcher does not embed its version $CIMMERIA_LAUNCHER_VERSION"
    echo "launcher embeds version $CIMMERIA_LAUNCHER_VERSION"
  fi
  # The bundled UnRAR must unpack a RAR archive (PP-U-FIX). The hidden self-test
  # exits non-zero, with the UnRAR error, when it cannot.
  "$exe" --self-test-rar || die "$exe cannot unpack a RAR archive"
  echo "verified: x86-64 AppImage, launcher $(sha "$exe")"
  rm -rf "$check"
}

stage_package() {
  local image; image="$(built)"
  mkdir -p artifacts
  rm -f "$APPIMAGE" "$APPIMAGE.sha256"
  cp "$image" "$APPIMAGE"
  chmod 755 "$APPIMAGE"
  printf '%s  %s\n' "$(sha "$APPIMAGE")" "$APPIMAGE_NAME" > "$APPIMAGE.sha256"
  cat "$APPIMAGE.sha256"
}

stage_paths() {
  echo "$APPIMAGE"
  echo "$APPIMAGE.sha256"
}

case "${1:-}" in
  tools) stage_tools ;;
  pieces) stage_pieces "${2:-}" "${3:-}" ;;
  app) stage_app ;;
  verify) stage_verify ;;
  package) stage_package ;;
  paths) stage_paths ;;
  *) sed -n '2,/^set -euo pipefail/{/^set -euo/d;p}' "$0" >&2; exit 2 ;;
esac
