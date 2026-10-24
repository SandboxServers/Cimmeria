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
#                    launcher is x86-64, carries no Windows Play pieces, and
#                    embeds the release stamps
#   package          copy it to
#                    artifacts/StargateWorlds-Launcher-linux-x86_64.AppImage,
#                    executable, and write a .sha256
#   paths            print the artifact paths
#
# The Linux launcher installs the game in-process (the seed's cabinet set is
# expanded by cimmeria-launcher-core's portable decoder). It has no Play,
# Repair, Update or prerequisite backend yet, so it carries none of the Windows
# pieces the Windows zip and the macOS app embed (crates/launcher/desktop/README.md,
# "Linux").
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
  local config=()
  if [ -n "${CIMMERIA_LAUNCHER_VERSION:-}" ]; then
    config=(--config "{\"version\":\"$CIMMERIA_LAUNCHER_VERSION\"}")
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
  # No Play backend yet, so no injected Windows code ships in the AppImage.
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
  app) stage_app ;;
  verify) stage_verify ;;
  package) stage_package ;;
  paths) stage_paths ;;
  *) sed -n '2,28p' "$0" >&2; exit 2 ;;
esac
