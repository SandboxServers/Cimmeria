<#
.SYNOPSIS
    Refresh the installed lab CLI, write the lab.cmd shim, and add it to the user PATH.

.DESCRIPTION
    Run it from a Cimmeria checkout. It copies the CLI into <LabHome>\cli
    (Install-LabCli), writes <LabHome>\bin\lab.cmd, and adds <LabHome>\bin to the
    user PATH when it is missing. A new terminal sees the PATH change.
    LabHome is %LOCALAPPDATA%\cimmeria-lab, or CIMMERIA_LAB_HOME when set.

.PARAMETER NoPath
    Leave the user PATH alone; add <LabHome>\bin to it yourself.

.EXAMPLE
    pwsh tools/lab/cli/setup.ps1 -NoPath
.EXAMPLE
    lab setup
#>
[CmdletBinding()]
param(
    [switch]$NoPath
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-lib.ps1')

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..\..')).Path
Install-LabCli $root

$bin = Join-Path (Get-InstallLabHome) 'bin'
New-Item -ItemType Directory -Force -Path $bin | Out-Null
$shim = Join-Path $bin 'lab.cmd'
Set-Content -LiteralPath $shim -Encoding ascii -Value @(
    '@echo off',
    'pwsh -NoProfile -File "%LOCALAPPDATA%\cimmeria-lab\cli\lab.ps1" %*')
Write-Host "shim:  $shim"

if ($NoPath) {
    Write-Host "PATH:  left alone (-NoPath). Add $bin to the user PATH to run 'lab' anywhere."
} elseif (Add-UserPathEntry $bin) {
    Write-Host "PATH:  added $bin to the user PATH. A new terminal sees it."
} else {
    Write-Host "PATH:  $bin is already on the user PATH."
}
