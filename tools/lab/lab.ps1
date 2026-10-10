<#
.SYNOPSIS
    The lab CLI dispatcher: lab <command> [args...]. Commands are the files
    in tools/lab/cli/ (run `lab help` for the list).

.DESCRIPTION
    A command is a file tools/lab/cli/<command>.ps1, run with the remaining
    arguments. Its first .SYNOPSIS line is its help line. common.ps1,
    test-*.ps1 and *-lib.ps1 (dot-sourced libraries) are not commands. Adding a command never edits this file.

.PARAMETER Command
    The command to run, or help (the default).

.PARAMETER Rest
    The arguments passed to the command.

.EXAMPLE
    pwsh tools/lab/lab.ps1 help
    pwsh tools/lab/lab.ps1 status
#>
param(
    [Parameter(Position = 0)]
    [string]$Command = 'help',
    [Parameter(ValueFromRemainingArguments)]
    [string[]]$Rest
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$CliDir = Join-Path $PSScriptRoot 'cli'
$Commands = @(Get-ChildItem -LiteralPath $CliDir -Filter '*.ps1' -File |
    Where-Object { $_.BaseName -ne 'common' -and $_.BaseName -notlike 'test-*' -and $_.BaseName -notlike '*-lib' } |
    Sort-Object BaseName)

# The first non-blank line after a file's .SYNOPSIS line, or '' when it has none.
function Get-Synopsis([string]$Path) {
    $found = $false
    foreach ($line in Get-Content -LiteralPath $Path) {
        if ($found) {
            if ($line.Trim()) { return $line.Trim() }
        } elseif ($line.Trim() -eq '.SYNOPSIS') {
            $found = $true
        }
    }
    return ''
}

if ($Command -eq 'help') {
    foreach ($c in $Commands) {
        '{0}  {1}' -f $c.BaseName, (Get-Synopsis $c.FullName)
    }
    exit 0
}

$match = $Commands | Where-Object { $_.BaseName -eq $Command }
if (-not $match) {
    [Console]::Error.WriteLine("unknown command '$Command'; run: lab help")
    exit 2
}

& (Join-Path $CliDir "$Command.ps1") @Rest
$code = $LASTEXITCODE
if ($null -eq $code) { $code = 0 }
exit $code
