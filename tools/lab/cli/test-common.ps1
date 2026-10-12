<#
.SYNOPSIS
    Tests for tools/lab/cli/common.ps1 (no Pester needed). Exits non-zero on
    the first failure:

        pwsh -NoProfile -File tools/lab/cli/test-common.ps1

    It points CIMMERIA_LAB_HOME (the lab home override) and LOCALAPPDATA at a
    temp folder, so the real labd.env is never read or written. The
    environment is restored afterwards.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-cli-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$saved = @{
    Home  = $env:CIMMERIA_LAB_HOME
    Local = $env:LOCALAPPDATA
}
try {
    # The deleted functions stay deleted (nothing runs them any more).
    foreach ($gone in 'Get-LabStatus', 'Get-LabToken', 'Select-LabLogLine', 'Format-Ago') {
        Check ($null -eq (Get-Command $gone -CommandType Function -ErrorAction SilentlyContinue)) "$gone is not defined by common.ps1"
    }

    # --- lab home: CIMMERIA_LAB_HOME moves labd.env ---
    $labHome = Join-Path $tmp 'home'
    New-Item -ItemType Directory $labHome | Out-Null
    $env:LOCALAPPDATA = Join-Path $tmp 'local'
    $env:CIMMERIA_LAB_HOME = $labHome
    Check ((Get-LabHome) -eq $labHome) 'CIMMERIA_LAB_HOME overrides the lab home'
    Check ((Get-LabdEnvPath) -eq (Join-Path $labHome 'labd.env')) 'labd.env lives under the lab home'
    $env:CIMMERIA_LAB_HOME = $null
    Check ((Get-LabHome) -eq "$tmp\local\cimmeria-lab") 'the default lab home is %LOCALAPPDATA%\cimmeria-lab'
    $env:CIMMERIA_LAB_HOME = $labHome

    # --- Get-LabdEnv reads labd.env ---
    Write-LabdEnvFile (Join-Path $labHome 'labd.env') @('CIMMERIA_LAB_INSTALL_DIR=D:\Games\SGW')
    Check ((Get-LabdEnv)['CIMMERIA_LAB_INSTALL_DIR'] -eq 'D:\Games\SGW') 'Get-LabdEnv reads labd.env'
} finally {
    $env:CIMMERIA_LAB_HOME = $saved.Home
    $env:LOCALAPPDATA = $saved.Local
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'