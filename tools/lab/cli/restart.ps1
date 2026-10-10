<#
.SYNOPSIS
    Restart the lab daemon, picking up a newer cimmeria-lab.exe build: lab restart [-Force].

.DESCRIPTION
    Runs tools/lab/daemon.ps1 restart and passes its exit code on. daemon.ps1
    is found beside the cli folder, so the installed copy works too.

    The new daemon does not adopt the old one's clients, so the restart
    closes every lab client once the old daemon is down. While a session
    holds a lease it refuses, naming the holder (exit 3): the lease ends
    with the daemon. -Force closes the leased clients too.

.EXAMPLE
    pwsh tools/lab/lab.ps1 restart
    pwsh tools/lab/lab.ps1 restart -Force
#>
[CmdletBinding()]
param(
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$daemonArgs = @('-NoProfile', '-File', (Join-Path $PSScriptRoot '..\daemon.ps1'), 'restart')
if ($Force) { $daemonArgs += '-Force' }
pwsh @daemonArgs
exit $LASTEXITCODE
