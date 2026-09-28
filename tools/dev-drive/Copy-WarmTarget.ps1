<#
.SYNOPSIS
  Seeds a worktree's target dir on the Dev Drive from the warmest existing one.

.DESCRIPTION
  On a ReFS Dev Drive, Windows 11 24H2+ copies files within the volume by block cloning:
  the copy shares the source's blocks until either side changes, so a 10 GB target dir
  "copies" in seconds and costs almost no space. Cargo then sees an up-to-date target
  dir and the worktree's first build only recompiles what the branch changed.

  Picks the most recently modified target dir under -TargetRoot (excluding -Name) as the
  source. Copy-Item goes through CopyFile, which is what performs the block clone.

.EXAMPLE
  pwsh tools/dev-drive/Copy-WarmTarget.ps1 -TargetRoot B:\targets -Name my-worktree
#>
param(
    [Parameter(Mandatory)] [string] $TargetRoot,
    [Parameter(Mandatory)] [string] $Name,
    [string] $From
)
$ErrorActionPreference = 'Stop'
$dest = Join-Path $TargetRoot $Name
if (Test-Path $dest) { Write-Host "target dir already exists: $dest"; return }
if (-not $From) {
    $src = Get-ChildItem $TargetRoot -Directory | Where-Object Name -ne $Name |
        Sort-Object LastWriteTime -Descending | Select-Object -First 1
    if (-not $src) { Write-Host "no warm target dir under $TargetRoot yet; first build will be cold"; return }
    $From = $src.FullName
}
$fs = (Get-Volume -FilePath $TargetRoot).FileSystemType
if ($fs -ne 'ReFS') { Write-Warning "$TargetRoot is $fs, not ReFS: this is a full copy, not a block clone." }
$sw = [Diagnostics.Stopwatch]::StartNew()
Copy-Item -Path $From -Destination $dest -Recurse
# Incremental caches are tied to their original path; drop them so rustc rebuilds them cleanly.
Get-ChildItem $dest -Directory -Recurse -Filter incremental -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force

# Build-script output that names the source target dir by absolute path would keep
# pointing there: at the other worktree's files, or at nothing once that worktree is
# retired (utoipa-swagger-ui bakes its OUT_DIR into generated code; #962). Cargo rewrites
# a unit's own OUT_DIR in the `cargo:` lines of its `output` file when the target dir
# moves, so that prefix is fine. Any other mention of the source dir, in a `cargo:` line or
# in generated Rust under out/, drops the unit's build dir and fingerprint, and cargo
# reruns that build script on the next build. (Object files and other noise in the build
# dir name it too, but nothing reads them as a path.) Paths are compared in one separator
# style, because the lane's target dirs mix them (B:\targets/<name>\debug).
function ConvertTo-OneSeparator([string] $t) { $t.Replace('/', '\').Replace('\\', '\') }
$srcRoot = ConvertTo-OneSeparator ((Resolve-Path $From).Path.TrimEnd('\'))
$dropped = @()
foreach ($build in Get-ChildItem $dest -Directory -Recurse -Depth 2 -Filter build -ErrorAction SilentlyContinue) {
    $fingerprints = Join-Path $build.Parent.FullName '.fingerprint'
    if (-not (Test-Path $fingerprints)) { continue }
    $rel = $build.FullName.Substring($dest.Length)   # \<profile>\build (or \<triple>\<profile>\build)
    foreach ($unit in Get-ChildItem $build.FullName -Directory) {
        $stale = $false
        $output = Join-Path $unit.FullName 'output'
        if (Test-Path $output) {
            $lines = @(Get-Content $output -ErrorAction SilentlyContinue) -match '^cargo:'
            $text = (ConvertTo-OneSeparator ($lines -join "`n")) -ireplace [regex]::Escape("$srcRoot$rel\$($unit.Name)\out"), ''
            if ($text.IndexOf($srcRoot, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $stale = $true }
        }
        $outDir = Join-Path $unit.FullName 'out'
        if (-not $stale -and (Test-Path $outDir)) {
            foreach ($rs in Get-ChildItem $outDir -File -Recurse -Filter *.rs) {
                $code = ConvertTo-OneSeparator ([IO.File]::ReadAllText($rs.FullName))
                if ($code.IndexOf($srcRoot, [StringComparison]::OrdinalIgnoreCase) -ge 0) { $stale = $true; break }
            }
        }
        if ($stale) {
            Remove-Item $unit.FullName -Recurse -Force
            Remove-Item (Join-Path $fingerprints $unit.Name) -Recurse -Force -ErrorAction SilentlyContinue
            $dropped += $unit.Name
        }
    }
}
if ($dropped) { Write-Host "dropped $($dropped.Count) build-script output(s) that named $srcRoot`: $($dropped -join ', ')" }
Write-Host ("seeded {0} from {1} in {2:N1}s" -f $dest, $From, $sw.Elapsed.TotalSeconds)
