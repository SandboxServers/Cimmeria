#!/usr/bin/env pwsh
# Verifies the style and format rules for figure work that have caused real
# review-feedback churn on PR #284 and follow-ups. See tools/lint-figure-style.sh
# for the full rule catalog (M1, M2, M4, S1, S3, S4, C1, C2, C3). This script
# implements the same rules with identical exit semantics.

$ErrorActionPreference = 'Stop'

try {
  $repoRoot = (& git rev-parse --show-toplevel 2>$null).Trim()
} catch {
  Write-Error "tools/lint-figure-style.ps1: not inside a git work tree"
  exit 2
}
if (-not $repoRoot) {
  Write-Error "tools/lint-figure-style.ps1: not inside a git work tree"
  exit 2
}

$sourcesDir = Join-Path $repoRoot "docs/drafts/spec/figures/sources"
$rendersDir = Join-Path $repoRoot "docs/drafts/spec/figures"
$specDir    = Join-Path $repoRoot "docs/drafts/spec"

$script:failures = 0

function Emit {
  param([string]$Level, [string]$FilePath, [int]$Line, [string]$Rule, [string]$Msg)
  $rel = $FilePath.Substring($repoRoot.Length + 1).Replace('\', '/')
  if ($env:GITHUB_ACTIONS) {
    Write-Output "::$Level file=$rel,line=$Line,title=lint-figure-style $Rule::$Msg"
  }
  Write-Host "$($Level.ToUpper()): ${rel}:${Line}: [$Rule] $Msg"
  $script:failures++
}

function Find-SourceForSlug {
  param([string]$Slug)
  if (-not (Test-Path $sourcesDir)) { return $null }
  # Exclude *.md entirely (parity with the bash version's `! -name '*.md'`)
  # so a stray slug-named markdown file can't be treated as a source DSL.
  Get-ChildItem $sourcesDir -File |
    Where-Object {
      $_.BaseName -eq $Slug -and
      $_.Extension -ne '.md' -and
      -not $_.Name.StartsWith('.')
    } |
    Select-Object -First 1
}

function Test-MermaidSource {
  param([System.IO.FileInfo]$File)
  $content = Get-Content -Raw $File.FullName
  $lines = Get-Content $File.FullName

  # Detect type from first matching line
  $kind = $null
  foreach ($l in $lines) {
    if ($l -match '^(flowchart|sequenceDiagram|stateDiagram-v2|stateDiagram|classDiagram|erDiagram|gantt|pie|journey)\b') {
      $kind = $Matches[1]
      break
    }
  }

  switch ($kind) {
    'flowchart' {
      if ($content -notmatch '%%\{init:.*"flowchart".*"htmlLabels".*false') {
        Emit -Level error -FilePath $File.FullName -Line 1 -Rule M1 `
          -Msg 'Mermaid flowchart missing htmlLabels:false init directive. Prepend: %%{init: {"flowchart": {"htmlLabels": false}}}%%'
      }
    }
    'sequenceDiagram' {
      if ($content -notmatch '%%\{init:.*"sequence".*"htmlLabels".*false') {
        Emit -Level error -FilePath $File.FullName -Line 1 -Rule M2 `
          -Msg 'Mermaid sequenceDiagram missing htmlLabels:false init directive. Prepend: %%{init: {"sequence": {"htmlLabels": false}}}%%'
      }
    }
  }

  # M4
  if ($content -match '%%\{init:.*"htmlLabels".*false') {
    for ($i = 0; $i -lt $lines.Count; $i++) {
      $l = $lines[$i]
      if ($l -match '<br/>' -and $l -notmatch '^%%') {
        Emit -Level error -FilePath $File.FullName -Line ($i + 1) -Rule M4 `
          -Msg 'Mermaid source uses <br/> while htmlLabels:false is set — use \n for line breaks instead'
        break
      }
    }
  }
}

function Test-RenderedSvg {
  param([System.IO.FileInfo]$File)
  $content = Get-Content -Raw $File.FullName

  if ($content -notmatch '<!-- cimmeria-bg-injected -->') {
    Emit -Level error -FilePath $File.FullName -Line 1 -Rule S1 `
      -Msg 'SVG missing theme-aware backdrop marker — run the cimmeria-bg post-processor (see commit 18889d0)'
  }

  if ($content -match '<polygon fill="white" stroke="none" points="-4,4') {
    Emit -Level error -FilePath $File.FullName -Line 1 -Rule S3 `
      -Msg 'Graphviz intrinsic backdrop polygon retains fill="white" — strip the attribute and assign class="cimmeria-bg"'
  }

  if ($content -match 'rect class="backdrop"' -and $content -notmatch 'fill: transparent !important') {
    Emit -Level error -FilePath $File.FullName -Line 1 -Rule S4 `
      -Msg 'Svgbob SVG has rect.backdrop but no theme-aware !important override'
  }
}

function Test-Chapter {
  param([System.IO.FileInfo]$File)
  $lines = Get-Content $File.FullName

  # C1
  $expected = 1
  for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '^\*Figure (\d+):') {
      $n = [int]$Matches[1]
      if ($n -ne $expected) {
        Emit -Level error -FilePath $File.FullName -Line ($i + 1) -Rule C1 `
          -Msg "Figure caption number is $n; expected $expected (must be sequential and gap-free in document order)"
      }
      $expected++
    }
  }

  # C2 / C3
  for ($i = 0; $i -lt $lines.Count; $i++) {
    if ($lines[$i] -match '!\[([^\]]*)\]\((figures/[^)]+\.svg)\)') {
      $alt = $Matches[1]
      $path = $Matches[2]
      $altLower = $alt.ToLower()

      if ($alt.Length -lt 10 -or $altLower -in @('diagram', 'figure', 'image', 'svg')) {
        Emit -Level error -FilePath $File.FullName -Line ($i + 1) -Rule C2 `
          -Msg "Image alt text is too short or generic (`"$alt`") — use a descriptive sentence (>=10 chars)"
      }

      $absPath = Join-Path $specDir $path
      $slug = [System.IO.Path]::GetFileNameWithoutExtension($path)
      $hasSvg = Test-Path $absPath
      $hasSource = (Find-SourceForSlug -Slug $slug) -ne $null
      if (-not $hasSvg -and -not $hasSource) {
        Emit -Level error -FilePath $File.FullName -Line ($i + 1) -Rule C3 `
          -Msg "Image reference is dangling — neither $path nor a source DSL for $slug exists"
      }
    }
  }
}

# Run
$sourceCount = 0
if (Test-Path $sourcesDir) {
  Get-ChildItem $sourcesDir -Filter '*.mmd' -File | Sort-Object Name | ForEach-Object {
    Test-MermaidSource $_
    $sourceCount++
  }
}

$svgCount = 0
if (Test-Path $rendersDir) {
  Get-ChildItem $rendersDir -Filter '*.svg' -File | Sort-Object Name | ForEach-Object {
    Test-RenderedSvg $_
    $svgCount++
  }
}

$chapterCount = 0
if (Test-Path $specDir) {
  Get-ChildItem $specDir -Filter '*.md' -File | Sort-Object Name | ForEach-Object {
    Test-Chapter $_
    $chapterCount++
  }
}

Write-Host ""
if ($script:failures -eq 0) {
  Write-Host "OK: $sourceCount Mermaid source(s), $svgCount SVG(s), $chapterCount chapter(s) lint clean."
  exit 0
}

Write-Host "FAILED: $($script:failures) style violation(s) across $sourceCount Mermaid source(s), $svgCount SVG(s), $chapterCount chapter(s)."
Write-Host 'Fix the rule violations above. The sync check (tools/check-figure-sources) covers the separate "is the SVG present and fresh?" question.'
exit 1
