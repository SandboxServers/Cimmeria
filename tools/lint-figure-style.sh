#!/usr/bin/env bash
# Verifies the style and format rules for figure work that have caused real
# review-feedback churn on PR #284 and follow-ups. Catches deterministic
# violations (missing init directive, htmlLabels mismatch, theme-aware
# backdrop missing, non-sequential Figure numbering, dangling image refs).
#
# Rules
#   M1  Mermaid `flowchart` must carry the htmlLabels:false init directive.
#   M2  Mermaid `sequenceDiagram` must carry the htmlLabels:false init directive.
#   M4  Sources with htmlLabels:false set must not use <br/> in labels — use \n.
#   S1  Every rendered SVG must contain the cimmeria-bg-injected marker.
#   S3  Graphviz outputs must not retain the intrinsic fill="white" backdrop polygon.
#   S4  Svgbob outputs must override rect.backdrop with fill: transparent !important.
#   C1  Chapter *Figure N:* captions must be sequential and gap-free in document order.
#   C2  Markdown image alt text must be descriptive (>=10 chars, not "diagram"/"figure"/"image").
#   C3  Every markdown image ref must resolve to either an existing SVG or an
#       existing source DSL (a missing SVG with an existing source is the
#       sync-check's concern, not ours).
#
# Companion: tools/lint-figure-style.ps1 (Windows / PowerShell).

set -uo pipefail

if ! REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null)"; then
  echo "tools/lint-figure-style.sh: not inside a git work tree" >&2
  exit 2
fi

SOURCES_DIR="$REPO_ROOT/docs/drafts/spec/figures/sources"
RENDERS_DIR="$REPO_ROOT/docs/drafts/spec/figures"
SPEC_DIR="$REPO_ROOT/docs/drafts/spec"

failures=0

emit() {
  local level="$1" file="$2" line="$3" rule="$4" msg="$5"
  local rel="${file#"$REPO_ROOT/"}"
  if [ -n "${GITHUB_ACTIONS:-}" ]; then
    printf '::%s file=%s,line=%s,title=lint-figure-style %s::%s\n' "$level" "$rel" "$line" "$rule" "$msg"
  fi
  printf '%s: %s:%s: [%s] %s\n' "$level" "$rel" "$line" "$rule" "$msg" >&2
  failures=$((failures + 1))
}

# Resolve a slug (e.g. "mercury-26-foo") to its source DSL path if one exists.
find_source_for_slug() {
  local slug="$1"
  find "$SOURCES_DIR" -maxdepth 1 -type f -name "${slug}.*" \
       ! -name '*.md' ! -name '.*' 2>/dev/null | head -1
}

# ===== Mermaid source rules =====

lint_mermaid_source() {
  local f="$1"
  local kind
  kind="$(grep -E -m 1 '^(flowchart|sequenceDiagram|stateDiagram-v2|stateDiagram|classDiagram|erDiagram|gantt|pie|journey)' "$f" 2>/dev/null | awk '{print $1}')"

  case "$kind" in
    flowchart)
      if ! grep -q -E '%%\{init:.*"flowchart".*"htmlLabels".*false' "$f"; then
        emit error "$f" 1 M1 'Mermaid flowchart missing htmlLabels:false init directive. Prepend: %%{init: {"flowchart": {"htmlLabels": false}}}%%'
      fi
      ;;
    sequenceDiagram)
      if ! grep -q -E '%%\{init:.*"sequence".*"htmlLabels".*false' "$f"; then
        emit error "$f" 1 M2 'Mermaid sequenceDiagram missing htmlLabels:false init directive. Prepend: %%{init: {"sequence": {"htmlLabels": false}}}%%'
      fi
      ;;
  esac

  # M4: <br/> while htmlLabels:false is set (anywhere outside the init line)
  if grep -q -E '%%\{init:.*"htmlLabels".*false' "$f"; then
    local hit
    hit=$(grep -n -F '<br/>' "$f" 2>/dev/null | grep -v -E '^[0-9]+:%%' | head -1)
    if [ -n "$hit" ]; then
      local lineno="${hit%%:*}"
      emit error "$f" "$lineno" M4 'Mermaid source uses <br/> while htmlLabels:false is set — use \n for line breaks instead'
    fi
  fi
}

# ===== Rendered SVG rules =====

lint_rendered_svg() {
  local svg="$1"

  if ! grep -q -F -- '<!-- cimmeria-bg-injected -->' "$svg"; then
    emit error "$svg" 1 S1 'SVG missing theme-aware backdrop marker — run the cimmeria-bg post-processor (see commit 18889d0)'
  fi

  if grep -q -F -- '<polygon fill="white" stroke="none" points="-4,4' "$svg"; then
    emit error "$svg" 1 S3 'Graphviz intrinsic backdrop polygon retains fill="white" — strip the attribute and assign class="cimmeria-bg"'
  fi

  if grep -q -F -- 'rect class="backdrop"' "$svg"; then
    if ! grep -q -F -- 'fill: transparent !important' "$svg"; then
      emit error "$svg" 1 S4 'Svgbob SVG has rect.backdrop but no theme-aware !important override'
    fi
  fi
}

# ===== Chapter rules =====

lint_chapter() {
  local chapter="$1"

  # C1: Figure-N captions must be sequential and gap-free
  local expected=1
  while IFS=: read -r lineno content; do
    [ -z "$lineno" ] && continue
    local n
    n=$(printf '%s' "$content" | sed -E 's/^\*Figure ([0-9]+):.*$/\1/')
    if [ "$n" -ne "$expected" ]; then
      emit error "$chapter" "$lineno" C1 "Figure caption number is $n; expected $expected (must be sequential and gap-free in document order)"
    fi
    expected=$((expected + 1))
  done < <(grep -n -E '^\*Figure [0-9]+:' "$chapter" 2>/dev/null)

  # C2/C3: image refs to figures/*.svg
  while IFS=: read -r lineno content; do
    [ -z "$lineno" ] && continue
    local alt path
    alt=$(printf '%s' "$content" | sed -nE 's/.*!\[([^]]*)\]\(figures\/[^)]+\.svg\).*/\1/p')
    path=$(printf '%s' "$content" | sed -nE 's/.*!\[[^]]*\]\((figures\/[^)]+\.svg)\).*/\1/p')
    [ -z "$path" ] && continue

    # C2: alt text quality
    local alt_lower
    alt_lower=$(printf '%s' "$alt" | tr '[:upper:]' '[:lower:]')
    if [ "${#alt}" -lt 10 ] || \
       [ "$alt_lower" = "diagram" ] || [ "$alt_lower" = "figure" ] || \
       [ "$alt_lower" = "image" ]   || [ "$alt_lower" = "svg" ]; then
      emit error "$chapter" "$lineno" C2 "Image alt text is too short or generic (\"$alt\") — use a descriptive sentence (>=10 chars)"
    fi

    # C3: must resolve to either existing SVG or existing source DSL
    local abs_path="$REPO_ROOT/docs/drafts/spec/$path"
    local slug
    slug="$(basename "$path" .svg)"
    if [ ! -f "$abs_path" ] && [ -z "$(find_source_for_slug "$slug")" ]; then
      emit error "$chapter" "$lineno" C3 "Image reference is dangling — neither $path nor a source DSL for $slug exists"
    fi
  done < <(grep -n -E '!\[[^]]*\]\(figures/[^)]+\.svg\)' "$chapter" 2>/dev/null)
}

# ===== Run =====

source_count=0
if [ -d "$SOURCES_DIR" ]; then
  while IFS= read -r f; do
    [ -z "$f" ] && continue
    lint_mermaid_source "$f"
    source_count=$((source_count + 1))
  done < <(find "$SOURCES_DIR" -maxdepth 1 -type f -name '*.mmd' 2>/dev/null | sort)
fi

svg_count=0
if [ -d "$RENDERS_DIR" ]; then
  while IFS= read -r svg; do
    [ -z "$svg" ] && continue
    lint_rendered_svg "$svg"
    svg_count=$((svg_count + 1))
  done < <(find "$RENDERS_DIR" -maxdepth 1 -type f -name '*.svg' 2>/dev/null | sort)
fi

chapter_count=0
if [ -d "$SPEC_DIR" ]; then
  while IFS= read -r chapter; do
    [ -z "$chapter" ] && continue
    lint_chapter "$chapter"
    chapter_count=$((chapter_count + 1))
  done < <(find "$SPEC_DIR" -maxdepth 1 -type f -name '*.md' 2>/dev/null | sort)
fi

echo ""
if [ "$failures" -eq 0 ]; then
  printf 'OK: %d Mermaid source(s), %d SVG(s), %d chapter(s) lint clean.\n' "$source_count" "$svg_count" "$chapter_count"
  exit 0
fi

printf 'FAILED: %d style violation(s) across %d Mermaid source(s), %d SVG(s), %d chapter(s).\n' \
  "$failures" "$source_count" "$svg_count" "$chapter_count" >&2
echo 'Fix the rule violations above. The sync check (tools/check-figure-sources) covers the separate "is the SVG present and fresh?" question.' >&2
exit 1
