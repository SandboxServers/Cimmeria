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
Write-Host ("seeded {0} from {1} in {2:N1}s" -f $dest, $From, $sw.Elapsed.TotalSeconds)
