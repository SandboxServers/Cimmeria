<#
.SYNOPSIS
    Shared library for the lab CLI (tools/lab/cli/): paths, the daemon token,
    labd.env, labd.pid and the /status probe. Dot-source it; it defines
    functions only. Never print a token or a lease id from here.

.DESCRIPTION
    CIMMERIA_LAB_HOME overrides the lab home (%LOCALAPPDATA%\cimmeria-lab);
    the tests in test-common.ps1 use it to point at a temp folder. It is for
    tests only: daemon.ps1 (behind start, stop and restart) always uses
    %LOCALAPPDATA%\cimmeria-lab, so with the override set, status would read a
    different labd.pid than the one those commands act on.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot '..\labd-lib.ps1')

# The lab home: %LOCALAPPDATA%\cimmeria-lab, or $env:CIMMERIA_LAB_HOME when set.
function Get-LabHome {
    if ($env:CIMMERIA_LAB_HOME) { return $env:CIMMERIA_LAB_HOME }
    return Join-Path $env:LOCALAPPDATA 'cimmeria-lab'
}

# The labd.env file under the lab home.
function Get-LabdEnvPath {
    return Join-Path (Get-LabHome) 'labd.env'
}

# labd.env as an ordered KEY -> VALUE map (labd-lib.ps1's Read-LabdEnvFile).
function Get-LabdEnv {
    return Read-LabdEnvFile (Get-LabdEnvPath)
}

# The bearer token: the user environment variable, else the process one; $null when unset.
function Get-LabToken {
    $token = [Environment]::GetEnvironmentVariable('CIMMERIA_LAB_DAEMON_TOKEN', 'User')
    if (-not $token) { $token = [Environment]::GetEnvironmentVariable('CIMMERIA_LAB_DAEMON_TOKEN', 'Process') }
    if (-not $token) { return $null }
    return $token
}

# labd.pid as an object (pid, bind, started_at, ...), or $null when absent or unreadable.
function Get-DaemonInfo {
    $path = Join-Path (Get-LabHome) 'labd.pid'
    if (-not (Test-Path -LiteralPath $path)) { return $null }
    try {
        return Get-Content -LiteralPath $path -Raw | ConvertFrom-Json
    } catch {
        return $null
    }
}

# The daemon's /status object, or $null when the daemon or its endpoint is down.
# Never throws, and never reports the token in an error.
function Get-LabStatus {
    $token = Get-LabToken
    if (-not $token) { return $null }

    # The bind comes from labd.pid; a pidfile without a bind uses the default.
    $bind = '127.0.0.1:8779'
    $info = Get-DaemonInfo
    if ($info -and $info.PSObject.Properties.Name -contains 'bind' -and $info.bind) { $bind = [string]$info.bind }

    try {
        $status = Invoke-RestMethod -Uri "http://$bind/status" -Headers @{ Authorization = "Bearer $token" } -TimeoutSec 5
    } catch {
        return $null
    }
    # Only the daemon's own shape: something else answering on the port, or an
    # older /status, reads as unavailable rather than failing later under StrictMode.
    if ($status -isnot [pscustomobject]) { return $null }
    $names = $status.PSObject.Properties.Name
    if ($names -notcontains 'daemon' -or $names -notcontains 'instances' -or $null -eq $status.daemon) { return $null }
    return $status
}

# The game's profile root: labd.env's CIMMERIA_LAB_PROFILE_ROOT, else the
# environment variable, else <LOCALAPPDATA>\cimmeria-lab\instances.
function Get-ProfileRoot {
    $map = Get-LabdEnv
    foreach ($raw in @($map['CIMMERIA_LAB_PROFILE_ROOT'], $env:CIMMERIA_LAB_PROFILE_ROOT)) {
        if ("$raw".Trim()) { return "$raw".Trim() }
    }
    return Join-Path $env:LOCALAPPDATA 'cimmeria-lab\instances'
}

# A duration as "12s", "5m" or "2h 3m".
function Format-Ago([int]$seconds) {
    if ($seconds -lt 60) { return "${seconds}s" }
    # Truncating division: [int](x / y) rounds (90 s would read 2m).
    if ($seconds -lt 3600) { return '{0}m' -f [math]::Floor($seconds / 60) }
    return '{0}h {1}m' -f [math]::Floor($seconds / 3600), [math]::Floor(($seconds % 3600) / 60)
}

# Rows (objects whose property names are the column names) as an aligned table string.
function Write-LabTable($rows, [string[]]$columns) {
    $rows | Format-Table -Property $columns -AutoSize | Out-String -Width 200
}
