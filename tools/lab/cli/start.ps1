<#
.SYNOPSIS
    Start the lab daemon (the CimmeriaLabDaemon scheduled task).

.DESCRIPTION
    Runs tools/lab/daemon.ps1 start and passes its exit code on. daemon.ps1
    is found beside the cli folder, so the installed copy works too.

.EXAMPLE
    pwsh tools/lab/lab.ps1 start
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

pwsh -NoProfile -File (Join-Path $PSScriptRoot '..\daemon.ps1') start
exit $LASTEXITCODE
