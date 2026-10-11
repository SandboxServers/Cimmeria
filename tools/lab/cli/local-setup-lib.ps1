<#
.SYNOPSIS
    Pure helpers for local-setup.ps1, kept apart so test-local-setup.ps1 can
    test them against a temp folder and a fake environment. Dot-source it; it
    defines functions only. Nothing here prints or returns a secret's value.
#>

Set-StrictMode -Version Latest

# The lab endpoint's local bind. No default exists in the server (it is
# fail-closed, crates/lab-mcp/src/config.rs); 8451 is the one its docs use,
# and 8444 is taken by the dev supervisor (crates/supervisor, SUPERVISOR_PORT).
$script:LabLocalBind = '127.0.0.1:8451'
# The local server's login port (crates/common/src/config.rs, logon_port).
$script:LabLocalLoginUrl = 'http://127.0.0.1:8081'
# The login-server row and the seeded lab account (db/sgw/Accounts/Seed/account.sql:
# account 10, password SHA-1 of "test").
$script:LabLocalServerRow = 'Local'
$script:LabLocalUsername = 'lab'
$script:LabLocalPassword = 'test'
# Not seeded: lab_create_character (or lab_ensure_in_world with create) makes
# it. The name matches lab-account.p<n>.json's Labtwo..Labfive (instances.ps1).
$script:LabLocalCharacter = 'Labone'

# $Bytes random bytes from the OS CSPRNG, as lowercase hex (2 chars a byte).
# Create().GetBytes rather than ::Fill so Windows PowerShell 5.1 runs it too.
function New-LabSecret([int]$Bytes = 32) {
    $buf = New-Object byte[] $Bytes
    $rng = [System.Security.Cryptography.RandomNumberGenerator]::Create()
    try { $rng.GetBytes($buf) } finally { $rng.Dispose() }
    return -join ($buf | ForEach-Object { $_.ToString('x2') })
}

# How many key bytes the server reads from a secret: hex decodes to half its
# length, anything else is its UTF-8 bytes (admin-api dev_session/token.rs).
function Get-LabSecretByteCount([string]$Value) {
    if (-not $Value) { return 0 }
    $v = $Value.Trim()
    if ($v.Length % 2 -eq 0 -and $v -match '^[0-9a-fA-F]+$') { return $v.Length / 2 }
    return [System.Text.Encoding]::UTF8.GetByteCount($v)
}

# An environment store over a scope: Get(name) and Set(name, value).
# The real one is the user environment; the tests pass a hashtable-backed one.
function New-LabEnvStore([string]$Scope = 'User') {
    $s = $Scope
    return [pscustomobject]@{
        Get = { param($n) [Environment]::GetEnvironmentVariable($n, $s) }.GetNewClosure()
        Set = { param($n, $v) [Environment]::SetEnvironmentVariable($n, $v, $s) }.GetNewClosure()
    }
}

# Whether a host:port bind is loopback (127.x, localhost, ::1).
function Test-LabLoopbackBind([string]$Bind) {
    $hostPart = ($Bind -replace ':\d+$', '').Trim('[', ']')
    return ($hostPart -match '^127\.' -or $hostPart -ieq 'localhost' -or $hostPart -eq '::1')
}

# Records the game install in $Store's CIMMERIA_LAB_INSTALL_DIR. An unset
# variable is set. A different value is replaced when $Explicit (the user
# passed -InstallDir: the latest word wins, and the row names both paths),
# else kept. Returns one row like Set-LabLocalEnv's.
function Set-LabInstallDirEnv($Store, [string]$InstallDir, [bool]$Explicit) {
    $name = 'CIMMERIA_LAB_INSTALL_DIR'
    $current = & $Store.Get $name
    if (-not $current) {
        & $Store.Set $name $InstallDir
        return @{ name = $name; action = 'set'; note = $InstallDir }
    }
    $same = [string]::Equals($current.TrimEnd('\'), $InstallDir.TrimEnd('\'), [System.StringComparison]::OrdinalIgnoreCase)
    if ($same) { return @{ name = $name; action = 'kept'; note = 'already set' } }
    if ($Explicit) {
        & $Store.Set $name $InstallDir
        return @{ name = $name; action = 'replaced'; note = "was $current; now $InstallDir (-InstallDir)" }
    }
    return @{ name = $name; action = 'WARN'; note = "set to $current, not $InstallDir; left alone" }
}

# Warnings (strings) about an existing lab-account.json that will not reach
# the local server: another login row, or another account than the seeded
# one. Empty when it fits or is absent. Never mentions the password.
function Get-LabAccountWarnings([string]$InstallDir,
                                [string]$Server = $script:LabLocalServerRow) {
    $path = Join-Path $InstallDir 'Binaries\sessions\lab-account.json'
    if (-not (Test-Path -LiteralPath $path)) { return @() }
    try {
        $acct = Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    } catch {
        return @("$path is not valid JSON")
    }
    $names = $acct.PSObject.Properties.Name
    $warn = @()
    $have = if ($names -contains 'server') { [string]$acct.server } else { '' }
    if ($have -ne $Server) { $warn += "server is '$have', not '$Server': the lab logs in elsewhere unless a call passes server=$Server" }
    $user = if ($names -contains 'username') { [string]$acct.username } else { '' }
    if ($user -ne $script:LabLocalUsername) { $warn += "username is '$user', not the seeded '$($script:LabLocalUsername)'" }
    return $warn
}

# The checkout whose rust-toolchain.toml rustup should act on: $Given when
# passed (and nothing when it is not a checkout), else the git toplevel of
# the current directory if it is a Cimmeria checkout (not some other repo),
# else the checkout this script lives in. $null when none qualifies; never
# throws (git may be missing).
function Resolve-LabCheckout([string]$Given, [string]$ScriptRoot) {
    $isCheckout = { param($d) [bool]($d -and (Test-Path -LiteralPath (Join-Path $d 'rust-toolchain.toml'))) }
    if ($Given) {
        if (& $isCheckout $Given) { return (Resolve-Path -LiteralPath $Given).Path }
        return $null
    }
    $top = $null
    try { $top = git rev-parse --show-toplevel 2>$null } catch { $top = $null }
    if ($top -and (& $isCheckout $top)) { return $top }
    $own = Join-Path $ScriptRoot '..\..\..'
    if (& $isCheckout $own) { return (Resolve-Path -LiteralPath $own).Path }
    return $null
}

# Sets what a local server and the lab need in $Store, keeping every value
# already there that works. Returns one row per variable,
# @{ name; action; note }, action one of kept | generated | replaced | set.
# Never a value: the caller prints the rows.
function Set-LabLocalEnv($Store, [string]$Bind = $script:LabLocalBind) {
    $rows = @()
    $secrets = @(
        # name, bytes generated, minimum the reader accepts, measured as it
        # measures: the lab endpoint compares the string's length
        # (crates/lab-mcp/src/config.rs), the telemetry mint its key bytes.
        @('CIMMERIA_LAB_MCP_TOKEN', 32, 32, 'length'),
        @('CIMMERIA_TELEMETRY_HMAC_SECRET', 64, 32, 'bytes')
    )
    foreach ($s in $secrets) {
        $name = $s[0]
        $current = & $Store.Get $name
        $size = if ($s[3] -eq 'length') { "$current".Trim().Length } else { Get-LabSecretByteCount $current }
        if ($size -ge $s[2]) {
            $rows += @{ name = $name; action = 'kept'; note = 'already set' }
            continue
        }
        & $Store.Set $name (New-LabSecret $s[1])
        $action = if ($current) { 'replaced' } else { 'generated' }
        $note = if ($current) { "was under $($s[2]) bytes; $($s[1]) random bytes" } else { "$($s[1]) random bytes" }
        $rows += @{ name = $name; action = $action; note = $note }
    }

    $bindNow = & $Store.Get 'CIMMERIA_LAB_MCP_BIND'
    if ($bindNow -and -not (Test-LabLoopbackBind $bindNow)) {
        # The endpoint drives the server's console: on a non-loopback address
        # anyone who can reach it and has the token can.
        $rows += @{ name = 'CIMMERIA_LAB_MCP_BIND'; action = 'WARN'; note = "set to $bindNow, which is not loopback; left alone. Locally use $Bind" }
    } elseif ($bindNow) {
        $note = if ($bindNow -eq $Bind) { 'already set' } else { "already set to $bindNow (not $Bind); left alone" }
        $rows += @{ name = 'CIMMERIA_LAB_MCP_BIND'; action = 'kept'; note = $note }
    } else {
        & $Store.Set 'CIMMERIA_LAB_MCP_BIND' $Bind
        $bindNow = $Bind
        $rows += @{ name = 'CIMMERIA_LAB_MCP_BIND'; action = 'set'; note = $Bind }
    }

    # The URL clients dial: the bind, with a wildcard host made loopback.
    $url = 'http://{0}/mcp' -f ($bindNow -replace '^0\.0\.0\.0:', '127.0.0.1:' -replace '^\[::\]:', '[::1]:')
    $urlNow = & $Store.Get 'CIMMERIA_LAB_MCP_URL'
    if ($urlNow) {
        $note = if ($urlNow -eq $url) { 'already set' } else { "already set to $urlNow (the bind suggests $url); left alone" }
        $rows += @{ name = 'CIMMERIA_LAB_MCP_URL'; action = 'kept'; note = $note }
    } else {
        & $Store.Set 'CIMMERIA_LAB_MCP_URL' $url
        $rows += @{ name = 'CIMMERIA_LAB_MCP_URL'; action = 'set'; note = $url }
    }
    return $rows
}

# Writes <install>\Binaries\sessions\lab-account.json for the local server
# unless it exists. Returns 'kept' or 'written'. The fields are the ones
# crates/lab/src/supervisor/session_file.rs (LabAccount) reads.
function Write-LabAccountFile([string]$InstallDir,
                              [string]$Server = $script:LabLocalServerRow,
                              [string]$Character = $script:LabLocalCharacter) {
    $dir = Join-Path $InstallDir 'Binaries\sessions'
    $path = Join-Path $dir 'lab-account.json'
    if (Test-Path -LiteralPath $path) { return 'kept' }
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $account = [ordered]@{
        server    = $Server
        username  = $script:LabLocalUsername
        password  = $script:LabLocalPassword
        character = $Character
    }
    [System.IO.File]::WriteAllText($path, ($account | ConvertTo-Json), (New-Object System.Text.UTF8Encoding $false))
    return 'written'
}

# <install>\SGWGame\Content\UI\Startup\Login\LoginInternal.lua.
function Get-LoginInternalPath([string]$InstallDir) {
    return Join-Path $InstallDir 'SGWGame\Content\UI\Startup\Login\LoginInternal.lua'
}

# The Lua line for a login-server row, as the launcher writes it.
function Format-LoginRow([string]$Name, [string]$Url) {
    return '    LoginMod.servers["{0}"] = "{1}"' -f $Name, $Url
}

# LoginInternal.lua text with a LoginMod.servers["$Name"] row. Returns
# @{ status; text }: status 'present' (the row is there with $Url), 'conflict'
# (the row names another URL; text unchanged), 'added' (inserted after the last
# row, else after `LoginMod.servers = {}`), or 'unrecognised' (neither found;
# text unchanged). Keeps the file's line endings. Pure.
function Add-LoginInternalRow([string]$Text, [string]$Name, [string]$Url) {
    $nl = if ($Text -match "`r`n") { "`r`n" } else { "`n" }
    $lines = [System.Collections.Generic.List[string]]::new()
    foreach ($l in ($Text -split "`r?`n")) { $lines.Add($l) }
    $rowRe = '^\s*LoginMod\.servers\["([^"]*)"\]\s*=\s*"([^"]*)"'
    $last = -1
    $table = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        $line = $lines[$i]
        if ($line.TrimStart().StartsWith('--')) { continue }
        $m = [regex]::Match($line, $rowRe)
        if ($m.Success) {
            if ($m.Groups[1].Value -ieq $Name) {
                $status = if ($m.Groups[2].Value -eq $Url) { 'present' } else { 'conflict' }
                return @{ status = $status; text = $Text; url = $m.Groups[2].Value }
            }
            $last = $i
        } elseif ($line -match '^\s*LoginMod\.servers\s*=\s*\{\s*\}') {
            $table = $i
        }
    }
    $at = if ($last -ge 0) { $last } else { $table }
    if ($at -lt 0) { return @{ status = 'unrecognised'; text = $Text; url = $null } }
    $lines.Insert($at + 1, (Format-LoginRow $Name $Url))
    return @{ status = 'added'; text = ($lines -join $nl); url = $Url }
}
