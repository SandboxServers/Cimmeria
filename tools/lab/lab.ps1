<#
.SYNOPSIS
    The lab CLI dispatcher: lab <command> [args...]. Commands are the files
    in tools/lab/cli/ (run `lab help` for the list).

.DESCRIPTION
    A command is a file tools/lab/cli/<command>.ps1, run with the remaining
    arguments. Its first .SYNOPSIS line is its help line. common.ps1,
    test-*.ps1 and *-lib.ps1 (dot-sourced libraries) are not commands. Adding a command never edits this file.

    The first argument is the command (help when there is none); the rest go to
    the command unchanged. There is no param() block on purpose: splatting the
    automatic $args keeps named parameters (-Lines 5) named, where a
    ValueFromRemainingArguments [string[]] turns them into positional strings.

.EXAMPLE
    pwsh tools/lab/lab.ps1 help
    pwsh tools/lab/lab.ps1 status
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Command = if ($args.Count) { [string]$args[0] } else { 'help' }
# Assigned directly, not through an if-expression: an expression's output
# unrolls a one-element array to a bare string, which @Rest then splats one
# character at a time (lab instances status bound 's', 't', ...).
$Rest = @()
if ($args.Count -gt 1) { $Rest = @($args[1..($args.Count - 1)]) }

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

# A command that ends without `exit` and runs nothing native leaves
# $LASTEXITCODE as it was, so reset it first: falling off the end is success.
$global:LASTEXITCODE = 0
& (Join-Path $CliDir "$Command.ps1") @Rest
exit $global:LASTEXITCODE
