<#
.SYNOPSIS
    Show, get, set or unset labd.env entries. Secret values and URL hosts are masked.

.DESCRIPTION
    lab env                  every labd.env line, values masked
    lab env get KEY          one value, masked
    lab env set KEY VALUE    set KEY in place, or append it; comments and order are kept
    lab env unset KEY        remove KEY's lines

    A key whose name contains TOKEN, SECRET, PASSWORD, KEY, AUTH or CREDENTIAL
    prints as <redacted>. A URL's user, password and host print as <host>, and a
    line that is not KEY=VALUE or a comment prints as <unparsed line>. All of it
    is display only: set and unset write the values back unchanged. Keys match
    ignoring case, as the daemon reads them. Before a set or unset, labd.env is
    copied to labd.env.bak-<yyyyMMdd-HHmmss>, and the new file replaces the old
    one in a single move. The daemon reads labd.env only at start, so a change
    applies after lab restart.

    A VALUE that starts with '-' must be written -Value:<value>, or PowerShell
    reads it as a parameter name. A VALUE with leading or trailing spaces is
    refused, because labd.env lines are trimmed when read.

.EXAMPLE
    pwsh tools/lab/lab.ps1 env
    pwsh tools/lab/lab.ps1 env set CIMMERIA_LAB_INSTANCES default,p2
    pwsh tools/lab/lab.ps1 env set SOME_FLAGS -Value:-Xmx512m
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

# The index of the last line that sets $Key, ignoring case, or -1. The last one
# wins in Read-LabdEnvFile too, and its map ignores case, so these must as well.
function Find-LabdEnvLine([string[]]$Lines, [string]$Key) {
    for ($i = $Lines.Count - 1; $i -ge 0; $i--) {
        if ((Get-LabdEnvLineKey $Lines[$i]) -ieq $Key) { return $i }
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

# $Lines without any line that sets $Key (ignoring case); comments and other keys are kept.
function Remove-LabdEnvLine([string[]]$Lines, [string]$Key) {
    $out = [System.Collections.Generic.List[string]]::new()
    foreach ($line in $Lines) {
        if ((Get-LabdEnvLineKey $line) -ine $Key) { $out.Add($line) }
    }
    return , $out.ToArray()
}

# KEY must be upper case: ^[A-Z][A-Z0-9_]*$ (-cmatch, because -match ignores case).
function Test-LabdEnvKey([string]$Key) {
    return [bool]($Key -cmatch '^[A-Z][A-Z0-9_]*$')
}

# Key names whose values are secrets.
$script:SecretKeyPattern = 'TOKEN|SECRET|PASSWORD|KEY|AUTH|CREDENTIAL'

# Masks secret-looking text for display: a secret-named assignment loses its
# value, a bearer token and a 64-hex token become <redacted>, and a URL's
# userinfo and host become <host> (the port and path stay). The userinfo match
# runs to the last '@' in the URL, so a password holding '/' is covered too.
function Hide-LabdEnvText([string]$Text) {
    $t = $Text -replace "(?i)((?:$script:SecretKeyPattern)\w*\s*=)\S*", '${1}<redacted>'
    $t = $t -replace '(?i)bearer\s+\S+', '<redacted>'
    $t = $t -replace '(?i)\b[0-9a-f]{64}\b', '<redacted>'
    $t = $t -replace '(?i)([a-z][a-z0-9+.-]*://)(?:\S*@)?[^/\s:?#@]+', '${1}<host>'
    return $t
}

# A value for display: <redacted> when its key is a secret, else the masked text.
function Get-MaskedEnvValue([string]$Key, [string]$Value) {
    if ($Key -imatch $script:SecretKeyPattern) { return '<redacted>' }
    return Hide-LabdEnvText $Value
}

# One labd.env line for display: KEY=<masked value>. Comments and blank lines
# are masked as text; any other line (a stray pasted value) is not shown.
function Get-MaskedEnvLine([string]$Line) {
    $key = Get-LabdEnvLineKey $Line
    if ($null -eq $key) {
        $l = $Line.Trim().TrimStart([char]0xFEFF)
        if (-not $l -or $l.StartsWith('#')) { return Hide-LabdEnvText $Line }
        return '<unparsed line>'
    }
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
    # Write beside it, then move over it, so a crash never leaves a half-written labd.env.
    $tmp = "$Path.tmp"
    Write-LabdEnvFile $tmp $Lines
    Move-Item -LiteralPath $tmp -Destination $Path -Force
}

# Prints the usage line to stderr and exits 2.
function Exit-EnvUsage([string]$Message) {
    [Console]::Error.WriteLine("$Message`nusage: lab env [get KEY | set KEY VALUE | unset KEY]  (a VALUE starting with '-': -Value:<value>)")
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
        if ($Value -ne $Value.Trim()) {
            [Console]::Error.WriteLine('VALUE must not start or end with spaces (labd.env lines are trimmed when read)')
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
