<#
.SYNOPSIS
    Build the lab from a worktree through the PowerShell lane, install it, and refresh the CLI copy.

.DESCRIPTION
    Runs <From>\tools\lab\install.ps1 -Worktree <From>, with -SkipBuild when given
    and -InstallDir from labd.env's CIMMERIA_LAB_INSTALL_DIR when it is set. Then
    Install-LabCli copies the CLI from <From>, so the installed CLI matches the build.
    A lab build must come from a worktree you choose, so -From is required.

.PARAMETER From
    The worktree to build from, e.g. C:\src\Cimmeria\.claude\worktrees\lab-fix.

.PARAMETER SkipBuild
    Install what the target dir already holds; build nothing.

.EXAMPLE
    lab install -From C:\src\Cimmeria\.claude\worktrees\lab-fix
.EXAMPLE
    lab install -From C:\src\Cimmeria\.claude\worktrees\lab-fix -SkipBuild
#>
[CmdletBinding()]
param(
    [string]$From,
    [switch]$SkipBuild
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'install-lib.ps1')

if (-not $From) {
    throw 'lab install needs -From <worktree>: a lab build must come from a worktree you choose (e.g. lab install -From C:\src\Cimmeria\.claude\worktrees\lab-fix)'
}
$From = (Resolve-Path -LiteralPath $From).Path
$installer = Join-Path $From 'tools\lab\install.ps1'
if (-not (Test-Path -LiteralPath $installer)) { throw "$From has no tools\lab\install.ps1" }

$installArgs = @('-NoProfile', '-File', $installer, '-Worktree', $From)
if ($SkipBuild) { $installArgs += '-SkipBuild' }
$installDir = (Get-LabdEnv)['CIMMERIA_LAB_INSTALL_DIR']
if ($installDir) { $installArgs += @('-InstallDir', $installDir) }

& pwsh @installArgs
if ($LASTEXITCODE -ne 0) { throw "install.ps1 failed (exit $LASTEXITCODE); the CLI copy was not refreshed" }

Install-LabCli $From
Write-Host "CLI copy refreshed from $From"
