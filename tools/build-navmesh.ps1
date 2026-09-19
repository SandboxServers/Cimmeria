<#
.SYNOPSIS
    Build a .nav for one cooked UE3 map: extract -> NavBuilder -> inspect.

.DESCRIPTION
    Windows counterpart of tools/build-navmesh.sh. See
    docs/engine/navmesh-build-pipeline.md for the axis convention, the
    NavBuilder input layout and the failure modes this script guards.

    NavBuilder exits 0 even when it writes nothing, so the output-file check
    is the real success test.

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

.EXAMPLE
    tools/build-navmesh.ps1 Castle_CellBlock -ProbeFile probes/castle.txt
#>
[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)][string]$Map,
    [string]$CookedPc = $env:CIMMERIA_COOKED_PC,
    [string]$OutDir,
    [string]$IndexFile,
    [string]$NavFile,
    [string]$ProbeFile,
    [switch]$SkipExtract
)

$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot

$navBuilder = if ($env:CIMMERIA_NAVBUILDER) { $env:CIMMERIA_NAVBUILDER }
              else { Join-Path $repoRoot 'bin64/NavBuilder_d.exe' }

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
& $navBuilder chunked $chunks $NavFile nav
# Deliberately not checking $LASTEXITCODE: builder.cpp::exportNavmesh logs
# FAULT and returns void on every failure path, so the process still exits 0.
if (-not (Test-Path -LiteralPath $NavFile) -or (Get-Item -LiteralPath $NavFile).Length -eq 0) {
    throw 'NavBuilder produced no .nav (it still exits 0 - check its log above)'
}

# --- 3. Inspect -----------------------------------------------------------
Write-Host '==> nav_inspect'
$inspectArgs = @($NavFile)
if ($ProbeFile) { $inspectArgs += @('--probes', $ProbeFile) }
& cargo run --release -p cimmeria-navmesh-extractor --bin nav_inspect -- @inspectArgs
if ($LASTEXITCODE -ne 0) { throw "nav_inspect reported a problem ($LASTEXITCODE)" }
