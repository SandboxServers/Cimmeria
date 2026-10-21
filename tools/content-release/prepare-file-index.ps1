# Prepares a file index for the content release: verifies the current manifest,
# builds file-index.json, writes the candidate manifest (the current one plus
# its file_index entry) and proves the candidate by verify-file-index.
# It never signs, uploads or reads a signing key: the owner runs the printed steps.
#
# pwsh tools/content-release/prepare-file-index.ps1 -Seed seed.zip -Patch <id=zip>... `
#   -Current manifest.json -Out <dir> -BlobPath <blob value> -PubKey <hex> `
#   -BuildExe <build-file-index.exe> -VerifyExe <verify-file-index.exe> [-PatchsetExe <cimmeria-patchset.exe>]
#
# The exes come from a lane build (LANE_VERBOSE=1 prints target=<dir>; they sit in <dir>\debug).
# -PubKey is the release public key in hex. Never pass the private key here.

[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Seed,
    [string[]]$Patch = @(),
    [Parameter(Mandatory)][string]$Current,
    [Parameter(Mandatory)][string]$Out,
    [Parameter(Mandatory)][string]$BlobPath,
    [Parameter(Mandatory)][string]$PubKey,
    [Parameter(Mandatory)][string]$BuildExe,
    [Parameter(Mandatory)][string]$VerifyExe,
    [string]$PatchsetExe = ''
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Runs a native tool and stops the script on any non-zero exit.
function Invoke-Tool([string]$Exe, [string[]]$ToolArgs) {
    & $Exe @ToolArgs
    if ($LASTEXITCODE -ne 0) { throw "$(Split-Path -Leaf $Exe) exited with $LASTEXITCODE" }
}

# The patchset tool defaults to the lane build's sibling of build-file-index.
if (-not $PatchsetExe) { $PatchsetExe = Join-Path (Split-Path -Parent $BuildExe) 'cimmeria-patchset.exe' }

foreach ($p in @($Seed, $Current, $BuildExe, $VerifyExe, $PatchsetExe)) {
    if (-not (Test-Path -LiteralPath $p -PathType Leaf)) { throw "missing file: $p" }
}
if ($PubKey -notmatch '^[0-9a-fA-F]+$') { throw '-PubKey must be hex (the public key)' }
foreach ($p in $Patch) {
    if ($p -notmatch '^[^=]+=.+$') { throw "-Patch must be <id>=<zip>: $p" }
}

$Seed = (Resolve-Path -LiteralPath $Seed).Path
$Current = (Resolve-Path -LiteralPath $Current).Path
New-Item -ItemType Directory -Force -Path $Out | Out-Null
$Out = (Resolve-Path -LiteralPath $Out).Path

# Step 1: the current manifest must verify against the release public key.
Write-Host '[1/5] verify the current manifest'
Invoke-Tool $PatchsetExe @('verify', $Current, '--pubkey', $PubKey)

# Step 2: build the file index exactly as the release does.
Write-Host '[2/5] build file-index.json'
$patchArgs = @(foreach ($p in $Patch) { '--patch'; $p })
$IndexPath = Join-Path $Out 'file-index.json'
Invoke-Tool $BuildExe (@('--manifest', $Current, '--seed', $Seed) + $patchArgs + @('--out', $IndexPath))

# Step 3: the candidate is the current manifest plus file_index, nothing else changed.
Write-Host '[3/5] write manifest.candidate.json'
$index = Get-Item -LiteralPath $IndexPath
$indexSha = (Get-FileHash -LiteralPath $IndexPath -Algorithm SHA256).Hash.ToLowerInvariant()
$manifest = Get-Content -LiteralPath $Current -Raw | ConvertFrom-Json -Depth 20
$entry = [ordered]@{ blob = $BlobPath; size = $index.Length; sha256 = $indexSha }
$manifest | Add-Member -NotePropertyName file_index -NotePropertyValue $entry -Force
$CandidatePath = Join-Path $Out 'manifest.candidate.json'
$manifest | ConvertTo-Json -Depth 20 | Set-Content -LiteralPath $CandidatePath -Encoding utf8NoBOM

# Step 4: the chain and parity check. Stop on any FAIL.
Write-Host '[4/5] verify the candidate against the current manifest, the seed and the patches'
Invoke-Tool $VerifyExe (@('--manifest', $CandidatePath, '--file-index', $IndexPath, '--previous', $Current, '--seed', $Seed) + $patchArgs)

# Step 5: print the owner's remaining steps. Nothing below is run here.
Write-Host '[5/5] owner steps (printed, not run; the release key stays with the owner)'
Write-Host "  candidate : $CandidatePath"
Write-Host "  index     : $IndexPath  (size $($index.Length), sha256 $indexSha)"
Write-Host '  a. Copy the candidate to manifest.json in a clean folder. Sign it offline:'
Write-Host '       cimmeria-patchset sign manifest.json --key <offline-private-key-file>'
Write-Host '     That writes manifest.json.sig beside it.'
Write-Host "  b. cimmeria-patchset verify manifest.json --pubkey $PubKey"
Write-Host '  c. Upload file-index.json to the immutable per-publication release (the tag D-LT15 names):'
Write-Host '       gh release upload <per-publication-tag> file-index.json'
Write-Host '  d. Upload the signed manifest.json and manifest.json.sig to content-current (overwrite):'
Write-Host '       gh release upload content-current --clobber manifest.json manifest.json.sig'
Write-Host '  e. Run LT-27 U7 on the laptop that would not adopt; record the result in the ledger.'
Write-Host 'prepare-file-index: PASS (candidate written, verify-file-index passed)'
