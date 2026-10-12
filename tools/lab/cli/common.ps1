<#
.SYNOPSIS
    Shared helpers for the PowerShell the lab CLI still runs (tools/lab/cli/):
    the lab home and labd.env. Dot-source it; it defines functions only.

.DESCRIPTION
    CIMMERIA_LAB_HOME overrides the lab home (%LOCALAPPDATA%\cimmeria-lab);
    the tests in test-common.ps1 use it to point at a temp folder. It is for
    tests only: daemon.ps1 (behind lab start, stop and restart) always uses
    %LOCALAPPDATA%\cimmeria-lab.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot '..\labd-lib.ps1')

# The lab home: %LOCALAPPDATA%\cimmeria-lab, or $env:CIMMERIA_LAB_HOME when set.
function Get-LabHome {
    if ($env:CIMMERIA_LAB_HOME) { return $env:CIMMERIA_LAB_HOME }
    return Join-Path $env:LOCALAPPDATA 'cimmeria-lab'
}

# The labd.env file under the lab home.
function Get-LabdEnvPath {
    return Join-Path (Get-LabHome) 'labd.env'
}

# labd.env as an ordered KEY -> VALUE map (labd-lib.ps1's Read-LabdEnvFile).
function Get-LabdEnv {
    return Read-LabdEnvFile (Get-LabdEnvPath)
}
