<#
.SYNOPSIS
    Stop the lab daemon and close its lab clients: lab stop [-Force].

.DESCRIPTION
    Runs tools/lab/daemon.ps1 stop and passes its exit code on. daemon.ps1
    is found beside the cli folder, so the installed copy works too.

    A later daemon does not adopt the clients this one launched, so the stop
    closes them once the daemon is down. While a session holds a lease it
    refuses, naming the holder (exit 3). -Force closes the leased clients too.

.EXAMPLE
    pwsh tools/lab/lab.ps1 stop
    pwsh tools/lab/lab.ps1 stop -Force
#>
[CmdletBinding()]
param(
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$daemonArgs = @('-NoProfile', '-File', (Join-Path $PSScriptRoot '..\daemon.ps1'), 'stop')
if ($Force) { $daemonArgs += '-Force' }
pwsh @daemonArgs
exit $LASTEXITCODE
