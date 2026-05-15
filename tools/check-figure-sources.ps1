#!/usr/bin/env pwsh
# Verifies that every rendered SVG under docs/drafts/spec/figures/ is at-or-
# after its source DSL in docs/drafts/spec/figures/sources/. If a source is
# committed without its re-rendered SVG, this exits non-zero so CI fails.
#
# Pairing rule: `sources/<slug>.<ext>` pairs with `<slug>.svg` one directory up.
# README.md and dotfiles under sources/ are ignored.
#
# Companion: tools/check-figure-sources.sh (Linux / macOS / WSL).

$ErrorActionPreference = 'Stop'

try {
  $repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
} catch {
  Write-Error "tools/check-figure-sources.ps1: not inside a git work tree"
  exit 2
}
if (-not $repoRoot) {
  Write-Error "tools/check-figure-sources.ps1: not inside a git work tree"
  exit 2
}

$sourcesDir = Join-Path $repoRoot "docs/drafts/spec/figures/sources"
$rendersDir = Join-Path $repoRoot "docs/drafts/spec/figures"

if (-not (Test-Path $sourcesDir)) {
  Write-Error "tools/check-figure-sources.ps1: $sourcesDir does not exist"
  exit 2
}

$sources = Get-ChildItem $sourcesDir -File |
  Where-Object { $_.Name -ne 'README.md' -and -not $_.Name.StartsWith('.') } |
  Sort-Object Name

if ($sources.Count -eq 0) {
  Write-Error "tools/check-figure-sources.ps1: no source files found in $sourcesDir"
  exit 2
}

$failed = $false
$checked = 0

foreach ($src in $sources) {
  $checked++
  $slug = $src.BaseName
  $svg = Join-Path $rendersDir "$slug.svg"
  $relSrc = $src.FullName.Substring($repoRoot.Length + 1).Replace('\', '/')
  $relSvg = $svg.Substring($repoRoot.Length + 1).Replace('\', '/')

  if (-not (Test-Path $svg)) {
    Write-Host "MISSING SVG: $relSvg (source: $relSrc)"
    $failed = $true
    continue
  }

  $srcTsRaw = (& git log -1 --format=%ct -- $src.FullName 2>$null)
  $svgTsRaw = (& git log -1 --format=%ct -- $svg 2>$null)
  $srcTs = if ($srcTsRaw) { [int]$srcTsRaw } else { 0 }
  $svgTs = if ($svgTsRaw) { [int]$svgTsRaw } else { 0 }

  # Source not yet committed — skip (CI doesn't see uncommitted state anyway).
  if ($srcTs -eq 0) { continue }

  if ($svgTs -lt $srcTs) {
    $epoch = [datetime]::new(1970, 1, 1, 0, 0, 0, [DateTimeKind]::Utc)
    $srcWhen = $epoch.AddSeconds($srcTs).ToString("yyyy-MM-dd HH:mm:ss 'UTC'")
    $svgWhen = if ($svgTs -eq 0) { "never committed" } else { $epoch.AddSeconds($svgTs).ToString("yyyy-MM-dd HH:mm:ss 'UTC'") }
    Write-Host "OUT OF DATE: $slug"
    Write-Host "  source: $relSrc last modified $srcWhen"
    Write-Host "  SVG:    $relSvg last modified $svgWhen"
    $failed = $true
  }
}

Write-Host ""
if (-not $failed) {
  Write-Host "All $checked figure sources are in sync with their rendered SVGs."
  exit 0
}

Write-Host "Re-render the out-of-date diagrams via Prixmaviz (or the local renderer)"
Write-Host "and commit the regenerated SVG(s) alongside the source change."
Write-Host "See docs/drafts/spec/figures/sources/README.md for renderer commands."
exit 1
