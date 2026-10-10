<#
.SYNOPSIS
    Show the lab daemon and each lab instance: account, client, port, lease, seed.

.DESCRIPTION
    Reads the daemon's GET /status. When the daemon or its endpoint is down,
    prints the instances named by labd.env's CIMMERIA_LAB_INSTANCES with the
    SEED column only, and exits 1.

    SEED is 'seeded' when the instance's profile folder exists under the
    profile root (Get-ProfileRoot), else '-'.

.EXAMPLE
    pwsh tools/lab/lab.ps1 status
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$Columns = @('INSTANCE', 'ACCOUNT', 'CLIENT', 'PORT', 'LEASE', 'SEED')

# A property of a JSON object, or $null when the object lacks it (strict mode throws on a missing one).
function Get-Field($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

# A table cell: the value, or '-' when it is unknown.
function Get-Cell($Value) {
    if ($null -eq $Value -or "$Value" -eq '') { return '-' }
    return "$Value"
}

# LEASE: '<owner> (<purpose>, <remaining> left)' while held, else 'free'.
function Get-LeaseText($Lease) {
    if (-not (Get-Field $Lease 'held')) { return 'free' }
    $held = Get-Field $Lease 'lease'
    if (-not $held) { return 'held' }
    return '{0} ({1}, {2} left)' -f (Get-Cell (Get-Field $held 'owner')), (Get-Cell (Get-Field $held 'purpose')), (Format-Ago ([int](Get-Field $held 'remaining_s')))
}

# SEED: 'seeded' when the instance's profile folder exists, else '-'.
function Get-SeedText([string]$Label) {
    $seed = Join-Path (Get-ProfileRoot) "$Label\profile\Documents\My Games\Firesky\SGWGame"
    if (Test-Path -LiteralPath $seed) { return 'seeded' }
    return '-'
}

$status = Get-LabStatus
if (-not $status) {
    Write-Host 'daemon: not running (or status endpoint unavailable)'
    $raw = (Get-LabdEnv)['CIMMERIA_LAB_INSTANCES']
    $labels = @("$raw".Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
    if (-not $labels) { $labels = @('default') }
    $rows = @($labels | ForEach-Object {
        [pscustomobject]@{ INSTANCE = $_; ACCOUNT = '-'; CLIENT = '-'; PORT = '-'; LEASE = '-'; SEED = Get-SeedText $_ }
    })
    Write-LabTable $rows $Columns
    exit 1
}

$daemon = $status.daemon
Write-Host ('daemon: pid {0}, up {1}, version {2}' -f $daemon.pid, (Format-Ago ([int]$daemon.uptime_s)), $daemon.version)

$rows = @(foreach ($inst in @($status.instances)) {
    [pscustomobject]@{
        INSTANCE = $inst.instance
        ACCOUNT  = Get-Cell (Get-Field $inst 'account')
        CLIENT   = Get-Cell (Get-Field $inst 'client_pid')
        PORT     = Get-Cell (Get-Field $inst 'bridge_port')
        LEASE    = Get-LeaseText (Get-Field $inst 'lease')
        SEED     = Get-SeedText $inst.instance
    }
})
Write-LabTable $rows $Columns
exit 0
