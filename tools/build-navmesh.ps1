<#
.SYNOPSIS
    Build a .nav for one cooked UE3 map: extract -> NavBuilder -> inspect.

.DESCRIPTION
    Windows counterpart of tools/build-navmesh.sh. See
    docs/engine/navmesh-build-pipeline.md for the axis convention, the
    NavBuilder input layout and the failure modes this script guards.

    The rebuilt NavBuilder (tools/build-navbuilder.ps1) exits 1 usage,
    2 internal error, 3 Recast build failed (incl. the 16-bit vertex / edge
    caps), 4 output not writable. The 2026-03 reference binary
    NavBuilder_d.exe accepts exactly four arguments and exits 0 even when it
    writes nothing, so the output-file check stays as a second line of defence.

.PARAMETER Map
    Cooked map directory name under <CookedPC>/Maps, e.g. Castle_CellBlock.

.PARAMETER CookedPc
    CookedPC root. Defaults to $env:CIMMERIA_COOKED_PC.

.PARAMETER OutDir
    Working directory. Defaults to build/navmesh/<Map>.

.PARAMETER IndexFile
    PackageIndex cache. Defaults to build/navmesh/package_index.bin.

.PARAMETER NavFile
    Final .nav path. Defaults to <OutDir>/<map lowercased>.nav.

.PARAMETER ProbeFile
    Probe list passed through to nav_inspect --probes.

.PARAMETER SkipExtract
    Reuse the OBJs already in <OutDir>/chunks.

.PARAMETER NavParam
    Recast parameters passed through to NavBuilder as key=value, e.g.
    -NavParam 'agentHeight=1.8','minRegionSize=24'. Quote any value that
    contains a comma (bounds=...), or PowerShell splits it. Needs the rebuilt
    NavBuilder; run it with no arguments for the key list.

.PARAMETER Preset
    Named parameter set. 'castle' is the whole-map Castle (World 8) set from
    docs/engine/navmesh-build-pipeline.md section 6. -NavParam entries are
    appended after the preset, so they win.

.EXAMPLE
    tools/build-navmesh.ps1 Castle_CellBlock -ProbeFile probes/castle.txt

.EXAMPLE
    tools/build-navmesh.ps1 Castle -Preset castle -ProbeFile probes/castle.txt
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Map,
    [string]$CookedPc = $env:CIMMERIA_COOKED_PC,
    [string]$OutDir,
    [string]$IndexFile,
    [string]$NavFile,
    [string]$ProbeFile,
    [switch]$SkipExtract,
    [string[]]$NavParam = @(),
    [ValidateSet('castle')][string]$Preset
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

$navBuilder = if ($env:CIMMERIA_NAVBUILDER) { $env:CIMMERIA_NAVBUILDER }
              elseif (Test-Path (Join-Path $repoRoot 'bin64/NavBuilder.exe')) { Join-Path $repoRoot 'bin64/NavBuilder.exe' }
              else { Join-Path $repoRoot 'bin64/NavBuilder_d.exe' }

# Whole-map Castle (World 8): fits Recast's 16-bit vertex AND edge caps with
# ~7 % headroom and keeps the interior probes in one component.
$presets = @{
    castle = @('partition=watershed', 'agentHeight=1.8', 'agentClimb=0.6',
               'minRegionSize=24', 'maxSimplificationError=2.5')
}
$navParams = @()
if ($Preset) { $navParams += $presets[$Preset] }
$navParams += $NavParam

if (-not $CookedPc) { throw 'Set -CookedPc or $env:CIMMERIA_COOKED_PC' }
$mapDir = Join-Path $CookedPc "Maps/$Map"
if (-not (Test-Path -LiteralPath $mapDir -PathType Container)) { throw "No such map directory: $mapDir" }

if (-not $OutDir)    { $OutDir    = Join-Path $repoRoot "build/navmesh/$Map" }
if (-not $IndexFile) { $IndexFile = Join-Path $repoRoot 'build/navmesh/package_index.bin' }
if (-not $NavFile)   { $NavFile   = Join-Path $OutDir ("{0}.nav" -f $Map.ToLowerInvariant()) }
$chunks = Join-Path $OutDir 'chunks'
New-Item -ItemType Directory -Force -Path $chunks | Out-Null

# --- 1. Extract OBJ -------------------------------------------------------
# NavBuilder's `chunked` mode globs *.obj and needs every basename to be
# `<8 hex digits>o`. A stray combined `<map>.obj` in the same directory
# leaves MapChunk's position fields uninitialised and the build dies with
# "Failed to create heightfield", so chunk OBJs get their own subdirectory.
#
# NOTE FOR THE COORDINATOR: this is the single call to the extractor CLI
# (`extract_map`, owned by worker nav-extract). Adjust the argument order
# here if its surface differs; nothing else in this script depends on it.
if (-not $SkipExtract) {
    Write-Host "==> extracting $Map"
    & cargo run --release -p cimmeria-navmesh-extractor --bin extract_map -- `
        $CookedPc $Map $chunks $IndexFile
    if ($LASTEXITCODE -ne 0) { throw "extract_map failed ($LASTEXITCODE)" }
}

$chunkObjs = @(Get-ChildItem -LiteralPath $chunks -Filter '*.obj' -File)
if ($chunkObjs.Count -eq 0) { throw "No chunk OBJs in $chunks - extraction produced nothing" }
foreach ($f in $chunkObjs) {
    if ($f.BaseName -notmatch '^[0-9a-fA-F]{8}o$') {
        throw ("Refusing to run NavBuilder: '{0}' is not a <hex8>o.obj chunk file; " +
               'NavBuilder would read it with uninitialised chunk bounds.' -f $f.Name)
    }
}

# --- 2. NavBuilder --------------------------------------------------------
if (-not (Test-Path -LiteralPath $navBuilder -PathType Leaf)) {
    throw "NavBuilder not found at $navBuilder (set `$env:CIMMERIA_NAVBUILDER)"
}
Write-Host "==> NavBuilder chunked ($($chunkObjs.Count) chunk OBJs)"
if (Test-Path -LiteralPath $NavFile) { Remove-Item -LiteralPath $NavFile -Force }
& $navBuilder chunked $chunks $NavFile nav @navParams
$navRc = $LASTEXITCODE
if ($navRc -ne 0) {
    if (Test-Path -LiteralPath $NavFile) { Remove-Item -LiteralPath $NavFile -Force }
    $hint = ''
    if ($navRc -eq 1 -and $navParams.Count -gt 0) {
        $hint = ' Exit 1 with -NavParam/-Preset usually means the old 4-argument NavBuilder_d.exe;' +
                ' build the tunable one with tools/build-navbuilder.ps1 and set $env:CIMMERIA_NAVBUILDER.'
    }
    throw "NavBuilder failed with exit code $navRc - see its ERROR lines above.$hint"
}
if (-not (Test-Path -LiteralPath $NavFile) -or (Get-Item -LiteralPath $NavFile).Length -eq 0) {
    throw 'NavBuilder exited 0 but produced no .nav (old reference binary?) - check its log above'
}

# --- 3. Inspect -----------------------------------------------------------
Write-Host '==> nav_inspect'
$inspectArgs = @($NavFile)
if ($ProbeFile) { $inspectArgs += @('--probes', $ProbeFile) }
& cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- @inspectArgs
if ($LASTEXITCODE -ne 0) { throw "nav_inspect reported a problem ($LASTEXITCODE)" }
