<#
.SYNOPSIS
    Show, get, set or unset labd.env entries. Secret values and URL hosts are masked.

.DESCRIPTION
    lab env                  every labd.env line, values masked
    lab env get KEY          one value, masked
    lab env set KEY VALUE    set KEY in place, or append it; comments and order are kept
    lab env unset KEY        remove KEY's lines

    A key whose name contains TOKEN, SECRET or PASSWORD prints as <redacted>.
    A host in a URL prints as <host>. Both are display only: set and unset
    write the values back unchanged. Before a set or unset, labd.env is copied
    to labd.env.bak-<yyyyMMdd-HHmmss>. The daemon reads labd.env only at start,
    so a change applies after lab restart.

.EXAMPLE
    pwsh tools/lab/lab.ps1 env
    pwsh tools/lab/lab.ps1 env set CIMMERIA_LAB_INSTANCES default,p2
#>
param(
    [Parameter(Position = 0)][string]$Verb,
    [Parameter(Position = 1)][string]$Key,
    [Parameter(Position = 2)][string]$Value
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

# The key of a labd.env line (the text before its first '='), or $null for a
# blank line, a comment, or a line with no key. The same rules as Read-LabdEnvFile.
function Get-LabdEnvLineKey([string]$Line) {
    $l = $Line.Trim().TrimStart([char]0xFEFF)
    if (-not $l -or $l.StartsWith('#')) { return $null }
    $i = $l.IndexOf('=')
    if ($i -lt 1) { return $null }
    return $l.Substring(0, $i).Trim()
}

# The index of the last line that sets $Key, or -1. The last one wins in Read-LabdEnvFile too.
function Find-LabdEnvLine([string[]]$Lines, [string]$Key) {
    for ($i = $Lines.Count - 1; $i -ge 0; $i--) {
        if ((Get-LabdEnvLineKey $Lines[$i]) -ceq $Key) { return $i }
    }
    return -1
}

# $Lines with $Key set to $Value: its last line replaced in place, else appended.
function Set-LabdEnvLine([string[]]$Lines, [string]$Key, [string]$Value) {
    $out = [System.Collections.Generic.List[string]]::new([string[]]$Lines)
    $i = Find-LabdEnvLine $Lines $Key
    if ($i -ge 0) { $out[$i] = "$Key=$Value" } else { $out.Add("$Key=$Value") }
    return , $out.ToArray()
}

# $Lines without any line that sets $Key; comments and other keys are kept.
function Remove-LabdEnvLine([string[]]$Lines, [string]$Key) {
    $out = [System.Collections.Generic.List[string]]::new()
    foreach ($line in $Lines) {
        if ((Get-LabdEnvLineKey $line) -cne $Key) { $out.Add($line) }
    }
    return , $out.ToArray()
}

# KEY must be upper case: ^[A-Z][A-Z0-9_]*$ (-cmatch, because -match ignores case).
function Test-LabdEnvKey([string]$Key) {
    return [bool]($Key -cmatch '^[A-Z][A-Z0-9_]*$')
}

# Masks secret-looking text for display: a TOKEN=, SECRET= or PASSWORD=
# assignment loses its value, a bearer token and a 64-hex token become
# <redacted>, and a URL's host becomes <host> (the port stays).
function Hide-LabdEnvText([string]$Text) {
    $t = $Text -replace '(?i)((?:TOKEN|SECRET|PASSWORD)\w*\s*=)\S*', '${1}<redacted>'
    $t = $t -replace '(?i)bearer\s+\S+', '<redacted>'
    $t = $t -replace '(?i)\b[0-9a-f]{64}\b', '<redacted>'
    $t = $t -replace '(?i)([a-z][a-z0-9+.-]*://)(?:[^/\s@]*@)?[^/\s:?#]+', '${1}<host>'
    return $t
}

# A value for display: <redacted> when its key is a secret, else the masked text.
function Get-MaskedEnvValue([string]$Key, [string]$Value) {
    if ($Key -imatch 'TOKEN|SECRET|PASSWORD') { return '<redacted>' }
    return Hide-LabdEnvText $Value
}

# One labd.env line for display: KEY=<masked value>. Comments and blank lines are masked as text.
function Get-MaskedEnvLine([string]$Line) {
    $key = Get-LabdEnvLineKey $Line
    if ($null -eq $key) { return Hide-LabdEnvText $Line }
    $text = $Line.Trim().TrimStart([char]0xFEFF)
    $value = $text.Substring($text.IndexOf('=') + 1)
    return "$key=" + (Get-MaskedEnvValue $key $value)
}

# Copies labd.env to labd.env.bak-<stamp> (a -2, -3, ... suffix when that name is taken),
# then writes $Lines to it.
function Save-LabdEnv([string]$Path, [string[]]$Lines) {
    $dir = Split-Path -Parent $Path
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    if (Test-Path -LiteralPath $Path) {
        $stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
        $backup = Join-Path $dir "labd.env.bak-$stamp"
        for ($n = 2; Test-Path -LiteralPath $backup; $n++) { $backup = Join-Path $dir "labd.env.bak-$stamp-$n" }
        Copy-Item -LiteralPath $Path -Destination $backup
        Write-Host "backup: $backup"
    }
    Write-LabdEnvFile $Path $Lines
}

# Prints the usage line to stderr and exits 2.
function Exit-EnvUsage([string]$Message) {
    [Console]::Error.WriteLine("$Message`nusage: lab env [get KEY | set KEY VALUE | unset KEY]")
    exit 2
}

if ($MyInvocation.InvocationName -ne '.') {
    $path = Get-LabdEnvPath
    $lines = @(if (Test-Path -LiteralPath $path) { [System.IO.File]::ReadAllLines($path) })

    if (-not $Verb) {
        if (-not (Test-Path -LiteralPath $path)) {
            [Console]::Error.WriteLine("no labd.env at $path")
            exit 1
        }
        foreach ($line in $lines) { Get-MaskedEnvLine $line }
    } elseif ($Verb -notin 'get', 'set', 'unset') {
        Exit-EnvUsage "unknown verb '$Verb'"
    } elseif (-not (Test-LabdEnvKey $Key)) {
        Exit-EnvUsage 'KEY must match ^[A-Z][A-Z0-9_]*$'
    } elseif ($Verb -eq 'get') {
        $map = Get-LabdEnv
        if (-not $map.Contains($Key)) {
            [Console]::Error.WriteLine("$Key is not set")
            exit 1
        }
        Get-MaskedEnvValue $Key ($map[$Key])
    } elseif ($Verb -eq 'set') {
        if (-not $PSBoundParameters.ContainsKey('Value')) { Exit-EnvUsage 'set needs KEY and VALUE' }
        if ($Value -match "[`r`n]") {
            [Console]::Error.WriteLine('VALUE must be one line')
            exit 2
        }
        Save-LabdEnv $path (Set-LabdEnvLine $lines $Key $Value)
        Write-Host 'restart the daemon for this to apply: lab restart'
    } else {
        $map = Get-LabdEnv
        if (-not $map.Contains($Key)) {
            Write-Host "$Key is not set; labd.env unchanged"
        } else {
            Save-LabdEnv $path (Remove-LabdEnvLine $lines $Key)
            Write-Host 'restart the daemon for this to apply: lab restart'
        }
    }
}
