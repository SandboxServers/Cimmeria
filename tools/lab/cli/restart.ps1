<#
.SYNOPSIS
    Restart the lab daemon, picking up a newer cimmeria-lab.exe build.

.DESCRIPTION
    Runs tools/lab/daemon.ps1 restart and passes its exit code on. daemon.ps1
    is found beside the cli folder, so the installed copy works too.

.EXAMPLE
    pwsh tools/lab/lab.ps1 restart
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

pwsh -NoProfile -File (Join-Path $PSScriptRoot '..\daemon.ps1') restart
exit $LASTEXITCODE
