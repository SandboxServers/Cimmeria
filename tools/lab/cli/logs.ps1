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

# The log lines that pass the instance and level filters, with tokens masked.
# Pure: it reads lines from the pipeline and touches no file. An entry starts
# with a timestamp; a line without one (a panic body, a backtrace) belongs to
# the entry above it and is kept or dropped with it, so a level word inside a
# continuation line ("   at error handling") never ranks it on its own.
function Select-LabLogLine {
    param(
        [Parameter(ValueFromPipeline)][string]$Line,
        [string]$Instance,
        [string]$Level
    )
    begin {
        $ranks = @{ trace = 0; debug = 1; info = 2; warn = 3; error = 4 }
        # Before the first entry, continuation lines have no entry to follow.
        $keep = -not ($Instance -or $Level)
    }
    process {
        if ($Line -match '^\d{4}-\d\d-\d\dT') {
            $keep = $true
            if ($Instance -and $Line -inotmatch ('instance=(Some\()?"' + [regex]::Escape($Instance) + '"')) { $keep = $false }
            if ($keep -and $Level) {
                $parts = @($Line.Trim() -split '\s+')
                $token = if ($parts.Count -ge 2) { $parts[1].ToLowerInvariant() } else { '' }
                $rank = $ranks[$token]
                if ($null -eq $rank -or $rank -lt $ranks[$Level]) { $keep = $false }
            }
        }
        if (-not $keep) { return }

        $masked = $Line -replace '(?i)bearer\s+\S+', '<redacted>'
        $masked -replace '(?i)\b[0-9a-f]{64}\b', '<redacted>'
    }
}

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
