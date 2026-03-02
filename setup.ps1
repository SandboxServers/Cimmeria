#Requires -Version 7.0
<#
.SYNOPSIS
    Cimmeria full bootstrap: fresh clone to running server.

.DESCRIPTION
    Thin wrapper that imports the CimmeriaBootstrap module and runs the full pipeline.
    See bootstrap/CimmeriaBootstrap/README.md for details.

.EXAMPLE
    pwsh setup.ps1

.EXAMPLE
    pwsh setup.ps1 -SkipDownload -NoLaunch -Configuration Release
#>
param(
    [switch]$SkipDownload,
    [switch]$SkipBuild,
    [switch]$InstallVS,
    [switch]$NoLaunch,
    [ValidateSet("Debug","Release")][string]$Configuration = "Debug",
    [switch]$IncludeBigWorld,
    [switch]$ForceDatabase
)

Import-Module (Join-Path $PSScriptRoot "bootstrap/CimmeriaBootstrap") -Force
Invoke-CimmeriaBootstrap @PSBoundParameters
