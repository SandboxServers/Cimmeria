#Requires -Version 7.0
<#
.SYNOPSIS
    Compatibility shim - calls setup.ps1.

.DESCRIPTION
    This script has been refactored into the CimmeriaBootstrap PowerShell module.
    It now delegates to setup.ps1, which provides the full bootstrap pipeline.

    For the original phases (download, extract, patch, build libs), run:
        pwsh setup.ps1 -NoLaunch

    For the full pipeline (libs + solution + database + server launch), run:
        pwsh setup.ps1
#>
param(
    [switch]$SkipDownload,
    [switch]$SkipBuild,
    [switch]$InstallVS,
    [ValidateSet("x64", "x86", "both")]
    [string]$BuildArch = "x64"
)

Write-Host "NOTE: setup-dependencies.ps1 is now a compatibility shim." -ForegroundColor Yellow
Write-Host "      Calling setup.ps1 -NoLaunch (original behavior: download + build only)." -ForegroundColor Yellow
Write-Host "      For the full pipeline, use: pwsh setup.ps1" -ForegroundColor Yellow
Write-Host ""

$setupArgs = @{}
if ($SkipDownload) { $setupArgs['SkipDownload'] = $true }
if ($SkipBuild) { $setupArgs['SkipBuild'] = $true }
if ($InstallVS) { $setupArgs['InstallVS'] = $true }
$setupArgs['NoLaunch'] = $true

& (Join-Path $PSScriptRoot "setup.ps1") @setupArgs
