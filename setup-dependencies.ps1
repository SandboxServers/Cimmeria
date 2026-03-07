#Requires -Version 7.0
<#
.SYNOPSIS
    Compatibility shim - calls setup.ps1.

.DESCRIPTION
    This script has been refactored into the CimmeriaBootstrap PowerShell module.
    It now delegates to setup.ps1 -NoLaunch -SkipApp (server dependencies only).

    For the full pipeline, run:
        pwsh setup.ps1
#>
param(
    [switch]$SkipDownload,
    [switch]$ForceDatabase
)

Write-Host "NOTE: setup-dependencies.ps1 is now a compatibility shim." -ForegroundColor Yellow
Write-Host "      Calling setup.ps1 -NoLaunch -SkipApp" -ForegroundColor Yellow
Write-Host ""

$setupArgs = @{ NoLaunch = $true; SkipApp = $true }
if ($SkipDownload) { $setupArgs['SkipDownload'] = $true }
if ($ForceDatabase) { $setupArgs['ForceDatabase'] = $true }

& (Join-Path $PSScriptRoot "setup.ps1") @setupArgs
