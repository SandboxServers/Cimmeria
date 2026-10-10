<#
.SYNOPSIS
    Stop the lab daemon (kills only the daemon's own process).

.DESCRIPTION
    Runs tools/lab/daemon.ps1 stop and passes its exit code on. daemon.ps1
    is found beside the cli folder, so the installed copy works too.

.EXAMPLE
    pwsh tools/lab/lab.ps1 stop
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

pwsh -NoProfile -File (Join-Path $PSScriptRoot '..\daemon.ps1') stop
exit $LASTEXITCODE
