<#
.SYNOPSIS
    Show the last lines of the lab daemon's log, filtered by instance or level, with tokens masked.

.DESCRIPTION
    Reads labd.log under the lab home (common.ps1's Get-LabHome).

    -Instance keeps lines that name the instance (instance="x" or
    instance=Some("x"), ignoring case). -Level keeps lines whose level token,
    the second whitespace field (INFO, WARN, ...), is at or above the level.
    A line that does not start with a timestamp (a panic body, a backtrace)
    is kept or dropped with the entry above it.

    A bearer token or a 64-hex token prints as <redacted>.

    -Follow streams the file (Get-Content -Tail -Wait) through the same
    filter, so it can show fewer than -Lines lines at first.

.EXAMPLE
    pwsh tools/lab/lab.ps1 logs -Lines 5
    pwsh tools/lab/lab.ps1 logs -Instance p2 -Level warn -Follow
#>
param(
    [int]$Lines = 50,
    [switch]$Follow,
    [string]$Instance,
    [ValidateSet('trace', 'debug', 'info', 'warn', 'error')][string]$Level
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

if ($MyInvocation.InvocationName -ne '.') {
    $path = Join-Path (Get-LabHome) 'labd.log'
    if (-not (Test-Path -LiteralPath $path)) {
        [Console]::Error.WriteLine("no daemon log yet: $path")
        exit 1
    }
    if ($Follow) {
        Get-Content -LiteralPath $path -Tail $Lines -Wait | Select-LabLogLine -Instance $Instance -Level $Level
    } else {
        Get-Content -LiteralPath $path | Select-LabLogLine -Instance $Instance -Level $Level | Select-Object -Last $Lines
    }
}
