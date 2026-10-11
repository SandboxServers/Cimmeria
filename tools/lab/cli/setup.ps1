<#
.SYNOPSIS
    Refresh the installed lab CLI, write the lab.cmd shim (unless lab.exe is installed), and add <LabHome>\bin to the user PATH.

.DESCRIPTION
    Run it from a Cimmeria checkout, or pass -From. It copies the CLI into
    <LabHome>\cli (Install-LabCli), writes <LabHome>\bin\lab.cmd unless <LabHome>\bin\lab.exe exists (then it deletes the shim), and adds
    <LabHome>\bin to the user PATH when it is missing, then broadcasts the change
    so a new terminal sees it. LabHome is %LOCALAPPDATA%\cimmeria-lab, or
    CIMMERIA_LAB_HOME when set.

.PARAMETER From
    The checkout to install from. Needed when setup runs from the installed copy.

.PARAMETER NoPath
    Leave the user PATH alone; add <LabHome>\bin to it yourself.

.PARAMETER Yes
    Add <LabHome>\bin to the user PATH without asking.

.EXAMPLE
    pwsh tools/lab/cli/setup.ps1 -NoPath
.EXAMPLE
    lab setup -From C:\src\Cimmeria
#>
[CmdletBinding()]
param(
    [string]$From,
    [switch]$NoPath,
    [switch]$Yes
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'install-lib.ps1')

$root = Resolve-SetupRoot $From $PSScriptRoot
Install-LabCli $root

$labHome = Get-InstallLabHome
$bin = Join-Path $labHome 'bin'
New-Item -ItemType Directory -Force -Path $bin | Out-Null
# Install-LabCli already deleted bin\lab.cmd and printed the notice when bin\lab.exe exists.
if (-not (Test-LabExeInstalled $labHome)) {
    $shim = Write-LabShim $labHome
    Write-Host "shim:  $shim"
}

$manual = "Add $bin to the user PATH to run 'lab' anywhere."
if ($NoPath) {
    Write-Host "PATH:  left alone (-NoPath). $manual"
} elseif ($null -eq (Add-PathEntryText (Get-UserPathRaw) $bin)) {
    Write-Host "PATH:  $bin is already on the user PATH."
} else {
    # D-LC3: the PATH edit is asked for, every time it would change something.
    $answer = $null
    if (-not $Yes) {
        try { $answer = Read-Host "Add $bin to your user PATH? [y/N]" } catch { $answer = $null }
    }
    if (-not $Yes -and "$answer".Trim() -notmatch '^(?i)y(es)?$') {
        Write-Host "PATH:  left alone. $manual"
    } elseif (Add-UserPathEntry $bin) {
        Write-Host "PATH:  added $bin to the user PATH. A new terminal sees it."
    } else {
        Write-Host "PATH:  $bin is already on the user PATH."
    }
}
