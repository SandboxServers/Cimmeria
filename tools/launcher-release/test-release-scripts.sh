#!/usr/bin/env bash
# Tests for stamp.sh and publish.sh against a stub `gh` that keeps releases in
# a temporary directory, so no test touches GitHub. launcher-desktop.yml runs
# this on every PR that touches the release scripts. Linux (GNU date).
#
# Usage: tools/launcher-release/test-release-scripts.sh
set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
FAILED=0

pass() { echo "ok   $*"; }
fail() { echo "FAIL $*"; FAILED=1; }

# The stub: STUB_STORE/<tag>/body and /assets (one name per line).
mkdir -p "$WORK/bin"
cat > "$WORK/bin/gh" <<'STUB'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = api ]; then
  if [[ "$*" == *releases* && "${STUB_FAIL_LISTING:-false}" == true ]]; then exit 1; fi
  if [[ "$*" == *actions/runs* ]]; then echo "2026-10-23T12:00:00Z"; else printf '%s\n' "$STUB_LISTING"; fi
  exit 0
fi
[ "$1" = release ] || { echo "stub gh: unsupported $*" >&2; exit 2; }
cmd="$2"; shift 2
case "$cmd" in
  download)
    tag="$1"; shift
    while [ $# -gt 0 ]; do
      case "$1" in --dir) mkdir -p "$2"; cp "$STUB_STORE/$tag/latest.json" "$2/latest.json"; shift 2 ;; *) shift ;; esac
    done ;;
  list) printf '%s\n' "$STUB_LISTING" ;;
  view)
    tag="$1"; shift
    [ -d "$STUB_STORE/$tag" ] || exit 1
    case "${*:-}" in
      "") ;;
      *"--json body"*) cat "$STUB_STORE/$tag/body" ;;
      *"--json assets"*) cat "$STUB_STORE/$tag/assets" ;;
      *) echo "stub gh: unsupported view $*" >&2; exit 2 ;;
    esac ;;
  create|edit|upload)
    tag="$1"; shift
    if [ "$cmd" = create ]; then
      mkdir -p "$STUB_STORE/$tag"; : > "$STUB_STORE/$tag/body"; : > "$STUB_STORE/$tag/assets"
    fi
    [ -d "$STUB_STORE/$tag" ] || { echo "stub gh: no release $tag" >&2; exit 1; }
    while [ $# -gt 0 ]; do
      case "$1" in
        --notes) printf '%s' "$2" > "$STUB_STORE/$tag/body"; shift 2 ;;
        --title|--target) shift 2 ;;
        --*) shift ;;
        *) name="$(basename "$1")"
           if [ "$name" = latest.json ]; then cp "$1" "$STUB_STORE/$tag/latest.json"; fi
           grep -qxF "$name" "$STUB_STORE/$tag/assets" || echo "$name" >> "$STUB_STORE/$tag/assets"
           shift ;;
      esac
    done ;;
  *) echo "stub gh: unsupported $cmd" >&2; exit 2 ;;
esac
STUB
chmod +x "$WORK/bin/gh"
export PATH="$WORK/bin:$PATH"
export GITHUB_REPOSITORY=SandboxServers/Cimmeria GITHUB_SERVER_URL=https://github.com
export GITHUB_RUN_ID=42 GITHUB_SHA=0123456789abcdef0123456789abcdef01234567

# --- stamp.sh: actual script dry run, no network or signing keys -------------
cd "$WORK"
export STUB_STORE="$WORK/stamp"
mkdir -p "$STUB_STORE"
export STUB_LISTING='[[]]'
# No API failure may become an empty release day.

export GITHUB_OUTPUT="$WORK/out"
"$HERE/stamp.sh"
BASE=2026.10.2301
TAG=launcher-20261023-0123456
if grep -qx "version=$BASE" "$GITHUB_OUTPUT"; then pass "stamp: stable first daily slot"; else fail "stamp: stable first daily slot"; fi
rm -rf stamp-reservation
export STUB_LISTING='[[{"tag_name":"launcher-20261023-aaaaaaa","created_at":"2026-10-23T00:00:00Z","body":"legacy release"}]]'
: > "$GITHUB_OUTPUT"
"$HERE/stamp.sh"
SECOND=2026.10.2302
if grep -qx "version=$SECOND" "$GITHUB_OUTPUT"; then pass "stamp: second signed stable daily slot"; else fail "stamp: second signed stable daily slot"; fi
export STUB_FAIL_LISTING=true
if "$HERE/stamp.sh" >/dev/null 2>&1; then fail "stamp: API failure refuses"; else pass "stamp: API failure refuses"; fi
unset STUB_FAIL_LISTING
# The helper tests cover repeat releases, migration and refusal paths.
python "$HERE/test_release_identity.py"

# --- publish.sh -------------------------------------------------------------
export TAG VERSION="$BASE"
cd "$WORK"
mkdir -p artifacts
for f in StargateWorlds-Launcher-windows-x64.zip StargateWorlds-Launcher-macos-arm64.zip; do
  echo "$f bytes" > "artifacts/$f"
  printf '%s  %s\n' "$(sha256sum "artifacts/$f" | cut -d' ' -f1)" "$f" > "artifacts/$f.sha256"
done
printf '{"version":"%s"}\n' "$BASE" > artifacts/latest.json
printf 'stub-signature\n' > artifacts/StargateWorlds-Launcher-windows-x64.zip.sig
ZIP_SHA="$(cut -d' ' -f1 artifacts/StargateWorlds-Launcher-windows-x64.zip.sha256)"

if "$HERE/publish.sh" check artifacts/StargateWorlds-Launcher-windows-x64.zip "$ZIP_SHA" >/dev/null 2>&1; then
  pass "check: the build job's digest"; else fail "check: the build job's digest"; fi
if "$HERE/publish.sh" check artifacts/StargateWorlds-Launcher-windows-x64.zip "$(printf '%064d' 0)" >/dev/null 2>&1 \
  || "$HERE/publish.sh" check artifacts/StargateWorlds-Launcher-windows-x64.zip "" >/dev/null 2>&1; then
  fail "check: refuses another or an empty digest"; else pass "check: refuses another or an empty digest"; fi

# Run publish commands against a fresh store; print the dated notes.
scenario() {
  export STUB_STORE="$WORK/store-$1"; shift
  rm -rf "$STUB_STORE"; mkdir -p "$STUB_STORE"
  local step
  for step in "$@"; do "$HERE/publish.sh" "$step" >/dev/null; done
  cat "$STUB_STORE/$TAG/body"
}
A="$(scenario a dated macos)"
B="$(scenario b dated macos dated)"
C="$(scenario c dated macos dated macos)"
if [ "$A" = "$B" ] && [ "$A" = "$C" ]; then pass "dated notes converge in any order"; else fail "dated notes converge in any order"; fi
if [ "$(grep -cF '**macOS (Apple silicon):**' <<<"$A")" = 1 ] && grep -qF '**Windows:**' <<<"$A"; then pass "dated notes: each paragraph once"; else fail "dated notes: each paragraph once"; fi
D="$(scenario d dated)"
if grep -qF 'macOS' <<<"$D"; then fail "dated notes: no macOS paragraph without its asset"; else pass "dated notes: no macOS paragraph without its asset"; fi

scenario e dated current >/dev/null
if grep -qF 'macos-arm64' "$STUB_STORE/launcher-current/body"; then
  fail "current notes: no macOS link before the macOS zip is there"; else pass "current notes: no macOS link before the macOS zip is there"; fi
"$HERE/publish.sh" macos-current >/dev/null
if grep -qF 'macOS (Apple silicon): download StargateWorlds-Launcher-macos-arm64.zip' "$STUB_STORE/launcher-current/body"; then
  pass "current notes: the macOS link once the zip is there"; else fail "current notes: the macOS link once the zip is there"; fi

mv artifacts/StargateWorlds-Launcher-windows-x64.zip.sig artifacts/saved.sig
if "$HERE/publish.sh" dated >/dev/null 2>&1 || "$HERE/publish.sh" current >/dev/null 2>&1; then
  fail "missing signature refuses dated and current publication"
else
  pass "missing signature refuses dated and current publication"
fi
mv artifacts/saved.sig artifacts/StargateWorlds-Launcher-windows-x64.zip.sig
if grep -qF 'This build is unsigned' <<<"$A"; then fail "unsigned fallback release notes removed"; fi
exit "$FAILED"
