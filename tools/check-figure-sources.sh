#!/usr/bin/env bash
# Verifies that every rendered SVG under docs/drafts/spec/figures/ is at-or-
# after its source DSL in docs/drafts/spec/figures/sources/. If a source is
# committed without its re-rendered SVG, this exits non-zero so CI fails.
#
# Pairing rule: `sources/<slug>.<ext>` pairs with `<slug>.svg` one directory up.
# README.md and dotfiles under sources/ are ignored.
#
# Companion: tools/check-figure-sources.ps1 (Windows / PowerShell).

set -uo pipefail

if ! git rev-parse --show-toplevel >/dev/null 2>&1; then
  echo "tools/check-figure-sources.sh: not inside a git work tree" >&2
  exit 2
fi

# Format a unix timestamp as ISO-ish UTC. Linux uses GNU date `-d @TS`; macOS
# (BSD date) uses `-r TS`. Fall back to the raw integer if neither works.
fmt_ts() {
  local ts="$1"
  date -u -d "@$ts" '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null \
    || date -u -r "$ts" '+%Y-%m-%d %H:%M:%S UTC' 2>/dev/null \
    || echo "$ts"
}

REPO_ROOT="$(git rev-parse --show-toplevel)"
SOURCES_DIR="$REPO_ROOT/docs/drafts/spec/figures/sources"
RENDERS_DIR="$REPO_ROOT/docs/drafts/spec/figures"

if [ ! -d "$SOURCES_DIR" ]; then
  echo "tools/check-figure-sources.sh: $SOURCES_DIR does not exist" >&2
  exit 2
fi

# Collect every source file. find excludes README.md and dotfiles by name.
mapfile -t sources < <(
  find "$SOURCES_DIR" -maxdepth 1 -type f \
    ! -name 'README.md' \
    ! -name '.*' \
    | sort
)

if [ "${#sources[@]}" -eq 0 ]; then
  echo "tools/check-figure-sources.sh: no source files found in $SOURCES_DIR" >&2
  exit 2
fi

failed=0
checked=0
for src in "${sources[@]}"; do
  rel_src="${src#"$REPO_ROOT/"}"
  base="$(basename "$src")"
  slug="${base%.*}"
  svg="$RENDERS_DIR/$slug.svg"
  rel_svg="${svg#"$REPO_ROOT/"}"
  checked=$((checked + 1))

  if [ ! -f "$svg" ]; then
    echo "MISSING SVG: $rel_svg (source: $rel_src)"
    failed=1
    continue
  fi

  # %ct = committer timestamp (unix seconds). 0 if path is untracked.
  src_ts="$(git log -1 --format=%ct -- "$src" 2>/dev/null || true)"
  svg_ts="$(git log -1 --format=%ct -- "$svg" 2>/dev/null || true)"
  src_ts="${src_ts:-0}"
  svg_ts="${svg_ts:-0}"

  # Source not yet committed — no comparison possible, skip (the developer
  # is mid-authoring; CI doesn't see uncommitted state anyway).
  if [ "$src_ts" -eq 0 ]; then
    continue
  fi

  if [ "$svg_ts" -lt "$src_ts" ]; then
    if [ "$svg_ts" -eq 0 ]; then
      svg_when="never committed"
    else
      svg_when="$(fmt_ts "$svg_ts")"
    fi
    src_when="$(fmt_ts "$src_ts")"
    echo "OUT OF DATE: $slug"
    echo "  source: $rel_src last modified $src_when"
    echo "  SVG:    $rel_svg last modified $svg_when"
    failed=1
  fi
done

echo ""
if [ "$failed" -eq 0 ]; then
  echo "All $checked figure sources are in sync with their rendered SVGs."
  exit 0
fi

echo "Re-render the out-of-date diagrams via Prixmaviz (or the local renderer)" >&2
echo "and commit the regenerated SVG(s) alongside the source change." >&2
echo "See docs/drafts/spec/figures/sources/README.md for renderer commands." >&2
exit 1
