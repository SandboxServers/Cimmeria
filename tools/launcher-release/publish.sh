#!/usr/bin/env bash
# The publishing steps of .github/workflows/launcher-release.yml. Only its
# publish jobs run this, from a checkout of the released commit: jobs that
# hold a write token and run no build code, no npm and no third-party action
# (crates/launcher/desktop/docs/release.md, "Jobs"). Every command is safe to
# re-run, in any order: uploads replace this release's own assets (--clobber),
# and the dated release's notes are always rewritten as its base text plus the
# macOS paragraph while the release holds the macOS zip, so any sequence of
# runs and re-runs ends with the same notes.
#
# Usage: tools/launcher-release/publish.sh <command> [args]
#
#   check <file> <sha256>
#                    the downloaded file has the SHA-256 its build job output,
#                    and so does its .sha256 beside it, when there is one
#   dated            create or update the dated release $TAG: its notes, the
#                    Windows zip, .sha256, .zip.sig (required), and the
#                    client-patches overlay patch when one was packed
#   current          refresh launcher-current (D-LX7) with the Windows zip,
#                    .sha256, .zip.sig and latest.json, the feed last
#   macos            attach the macOS zip and its .sha256 to $TAG
#   macos-current    replace the macOS zip and its .sha256 on launcher-current
#   current-notes    rewrite launcher-current's notes from its assets
#
# Environment: TAG; VERSION for dated and macos-current; TRIGGERING_PR (optional) for dated;
# GH_TOKEN, GH_REPO and the GITHUB_* variables Actions sets.
set -euo pipefail

ZIP_NAME=StargateWorlds-Launcher-windows-x64.zip
MAC_ZIP_NAME=StargateWorlds-Launcher-macos-arm64.zip
ZIP="artifacts/$ZIP_NAME"
MAC_ZIP="artifacts/$MAC_ZIP_NAME"
CURRENT=launcher-current
TAG="${TAG:?TAG is not set}"
STABLE="https://github.com/${GITHUB_REPOSITORY}/releases/download/$CURRENT"
# The macOS paragraph of the dated release's notes is one line starting with
# this marker; nothing else in the notes contains it.
MAC_MARKER='**macOS (Apple silicon):**'

die() { echo "::error::$*" >&2; exit 1; }

check() {
  local file="${1:?check needs a file}" want="${2:-}" got
  [ -s "$file" ] || die "$file was not downloaded"
  got="$(sha256sum "$file" | cut -d' ' -f1)"
  if [ -z "$want" ] || [ "$got" != "$want" ]; then
    die "the downloaded $file is $got, its build job made '$want'"
  fi
  if [ -f "$file.sha256" ]; then
    [ "$(cut -d' ' -f1 "$file.sha256")" = "$want" ] \
      || die "$file.sha256 does not name the build job's digest"
  fi
}

# One asset name per line.
assets_of() { gh release view "$1" --json assets --jq '.assets[].name'; }

# A release's notes without the macOS paragraph and the blank lines that
# trailed it.
base_of() {
  local body
  body="$(gh release view "$1" --json body --jq .body)"
  body="$(grep -vF -e "$MAC_MARKER" <<<"$body" || true)"
  printf '%s' "${body%"${body##*[![:space:]]}"}"
}

# Write the dated release's notes: the base, then the macOS paragraph only
# while the macOS zip is on the release.
dated_notes() {
  local base="$1" assets notes
  assets="$(assets_of "$TAG")"
  notes="$base"
  if grep -qxF "$MAC_ZIP_NAME" <<<"$assets"; then
    notes="$notes

$MAC_MARKER download \`${MAC_ZIP_NAME}\`, unzip it and drag \"Cimmeria Launcher\" to Applications in Finder before opening it. It is signed ad hoc, not by a registered Apple developer, so macOS blocks the first open: open System Settings > Privacy & Security and click Open Anyway, once. It needs Rosetta 2, and the first install downloads the Wine runtime (64 MB) from the WoWSilicon project. It does not update itself: download each new release from ${STABLE}/${MAC_ZIP_NAME}."
  fi
  gh release edit "$TAG" --notes "$notes"
}

# launcher-current's notes name the macOS zip only while it is there, so the
# link they give never 404s. The macOS zip there may be an earlier release's
# when this release's macOS build failed.
current_notes() {
  local assets notes
  assets="$(assets_of "$CURRENT")"
  notes="The newest Stargate Worlds launcher, currently ${TAG}. Windows: download ${ZIP_NAME}."
  if grep -qxF "$MAC_ZIP_NAME" <<<"$assets"; then
    notes="$notes macOS (Apple silicon): download ${MAC_ZIP_NAME}."
  fi
  notes="$notes The desktop launcher's updater reads latest.json from here."
  gh release edit "$CURRENT" --latest=false --notes "$notes"
}

dated() {
  local version="${VERSION:?dated needs VERSION}"
  local run_url="${GITHUB_SERVER_URL}/${GITHUB_REPOSITORY}/actions/runs/${GITHUB_RUN_ID}"
  # The run link is how the stamp step tells a re-run of this run from a
  # second dispatch of the same commit; keep it in the notes.
  local body="Stargate Worlds launcher ${version}, built from commit \`${GITHUB_SHA}\` by ${run_url}."
  if [ -n "${TRIGGERING_PR:-}" ]; then
    body="$body

Dispatched from PR #${TRIGGERING_PR}."
  fi
  body="$body

**Windows:** download \`${ZIP_NAME}\`, unzip it anywhere and run \`cimmeria-launcher-desktop.exe\`. The launcher includes its supporting files and prepares them automatically. It needs the WebView2 runtime (part of Windows 11). The newest release is always at ${STABLE}/${ZIP_NAME}.

**Players of the old Windows launcher (\`sgw-launcher.exe\`):** your old launcher will no longer update. Download the zip from ${STABLE}/${ZIP_NAME} and use \`cimmeria-launcher-desktop.exe\` instead.

Updater artifacts carry a Minisign signature. The Windows executable is not Authenticode signed; for Defender or SmartScreen see docs/client/launcher-guide.md."
  local assets=("$ZIP" "$ZIP.sha256")
  [ -s "$ZIP.sig" ] || die "signed updater artifact is required"
  assets+=("$ZIP.sig")
  shopt -s nullglob
  local overlay=(overlay-out/*.zip overlay-out/*.entry.json)
  shopt -u nullglob
  if [ ${#overlay[@]} -gt 0 ]; then
    body="$body

Includes the client-patches UI overlay patch. To publish it, add its \`.entry.json\` to the content manifest and re-sign (docs/client/launcher-distribution-setup.md)."
  fi
  if gh release view "$TAG" >/dev/null 2>&1; then
    # A re-run: the macOS paragraph comes back when the macOS zip is already
    # attached, whichever publish job ran last.
    echo "$TAG exists (a re-run): replacing its notes and assets"
    gh release edit "$TAG" --title "Launcher $TAG"
    gh release upload "$TAG" --clobber "${assets[@]}" ${overlay[@]+"${overlay[@]}"}
    dated_notes "$body"
  else
    # --target: the tag is made at the dispatched commit, not at whatever
    # main has become since.
    gh release create "$TAG" --target "$GITHUB_SHA" --title "Launcher $TAG" \
      --notes "$body" "${assets[@]}" ${overlay[@]+"${overlay[@]}"}
  fi
}

# `--clobber` replaces each asset (gh deletes and uploads it; any error fails
# the step). latest.json goes last, and it names the dated release's zip, so a
# launcher reading the feed mid-refresh is never pointed at a missing or
# mismatched file. The release is never marked latest, and its tag has no
# date, which the egui launcher's self-update skips.
current() {
  if [[ ! -s "$ZIP.sig" || ! -s artifacts/latest.json ]]; then
    die "signature and verified feed are required"
  fi
  if gh release view "$CURRENT" >/dev/null 2>&1; then
    mkdir -p artifacts/previous-feed
    gh release download "$CURRENT" --pattern latest.json --dir artifacts/previous-feed --clobber
    python "$(dirname "$0")/release_identity.py" current artifacts/latest.json artifacts/previous-feed/latest.json
  fi
  if ! gh release view "$CURRENT" >/dev/null 2>&1; then
    gh release create "$CURRENT" --latest=false --target "$GITHUB_SHA" \
      --title "Launcher (current)" --notes "The newest Stargate Worlds launcher."
  fi
  local file
  for file in "$ZIP" "$ZIP.sha256" "$ZIP.sig" artifacts/latest.json; do
    gh release upload "$CURRENT" "$file" --clobber
  done
  current_notes
}

macos() {
  gh release upload "$TAG" --clobber "$MAC_ZIP" "$MAC_ZIP.sha256"
  dated_notes "$(base_of "$TAG")"
}

macos_current() {
  local file status
  mkdir -p artifacts/previous-feed
  gh release download "$CURRENT" --pattern latest.json --dir artifacts/previous-feed --clobber
  status="$(python "$(dirname "$0")/release_identity.py" macos-current "${VERSION:?macos-current needs VERSION}" artifacts/previous-feed/latest.json)"
  if [ "$status" = skipped_older ]; then
    echo "launcher-current is newer than $VERSION; keeping its macOS assets and notes"
    if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "status=$status" >> "$GITHUB_OUTPUT"; fi
    return
  fi
  for file in "$MAC_ZIP" "$MAC_ZIP.sha256"; do
    gh release upload "$CURRENT" "$file" --clobber
  done
  current_notes
  if [ -n "${GITHUB_OUTPUT:-}" ]; then echo "status=$status" >> "$GITHUB_OUTPUT"; fi
}

case "${1:-}" in
  check) shift; check "$@" ;;
  dated) dated ;;
  current) current ;;
  macos) macos ;;
  macos-current) macos_current ;;
  current-notes) current_notes ;;
  *) sed -n '2,26p' "$0" >&2; exit 2 ;;
esac
