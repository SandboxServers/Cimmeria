<#
.SYNOPSIS
    Close the game client of one lab instance, or of all of them: lab clients stop [<instance>|all] [-Force].

.DESCRIPTION
    Asks each client window to close (CloseMainWindow) and force-stops it when
    it is still alive after 8 s. Only a process named SGW is touched, and the
    pid comes from the daemon's GET /status.

    An instance a session holds (a lease) is not stopped, because its holder
    would lose the client. -Force overrides that, for one named instance only:
    `all` with -Force is refused (D-LC5). Exit codes: 0 when nothing was
    refused, 1 when the daemon is not running, 2 on a usage error, 3 when a
    leased client was refused, 4 when a client could not be stopped.

    Select-StopTargets is pure and dot-sourceable: tools/lab/cli/test-ops.ps1
    dot-sources this file, and the command runs only when it is not dot-sourced.

.EXAMPLE
    pwsh tools/lab/lab.ps1 clients stop p2
    pwsh tools/lab/lab.ps1 clients stop p2 -Force
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Verb,
    [Parameter(Position = 1)]
    [string]$Which,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

# A property of a JSON object, or $null when the object lacks it (strict mode throws on a missing one).
function Get-Field($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

# The instance's client pid as an int, or $null when it has no client (client_pid is null or absent).
function Get-ClientPid($Instance) {
    $clientPid = Get-Field $Instance 'client_pid'
    if ("$clientPid" -match '^\d+$') { return [int]$clientPid }
    return $null
}

# Whether a session holds the instance's lease (the status's lease.held).
function Get-LeaseHeld($Instance) {
    return [bool](Get-Field (Get-Field $Instance 'lease') 'held')
}

# 'owner (purpose)' of the instance's lease, '-' for a part the status leaves out. Never a lease id.
function Get-LeaseWho($Instance) {
    $held = Get-Field (Get-Field $Instance 'lease') 'lease'
    $owner = [string](Get-Field $held 'owner')
    $purpose = [string](Get-Field $held 'purpose')
    if (-not $owner) { $owner = '-' }
    if (-not $purpose) { $purpose = '-' }
    return "$owner ($purpose)"
}

# Sorts the instances a stop names into buckets. stop: to stop; forced: the leased
# ones stopped by -Force (also in stop); refuse: leased, not stopped; none: no client.
# -Force applies only to a named instance, never to all. Pure: no process is touched.
function Select-StopTargets($Status, [string]$Which, [bool]$Force) {
    $all = (-not $Which) -or ($Which -eq 'all')
    $result = @{ stop = @(); forced = @(); refuse = @(); none = @() }
    foreach ($inst in @($Status.instances)) {
        if (-not $all -and (Get-Field $inst 'instance') -ne $Which) { continue }
        if ($null -eq (Get-ClientPid $inst)) { $result.none += $inst; continue }
        if (-not (Get-LeaseHeld $inst)) { $result.stop += $inst; continue }
        if ($Force -and -not $all) {
            $result.forced += $inst
            $result.stop += $inst
        } else {
            $result.refuse += $inst
        }
    }
    return $result
}

# Closes one client: CloseMainWindow, then Stop-Process -Force after 8 s. Acts
# only on a process named SGW; the process object is held, so a reused pid
# cannot be swapped in between the checks.
# Returns $false only when an SGW was there and is still alive afterwards (for
# example an elevated client this shell may not stop).
function Stop-LabClient([string]$Label, [int]$ClientPid) {
    $proc = Get-Process -Id $ClientPid -ErrorAction SilentlyContinue
    if (-not $proc -or $proc.ProcessName -ne 'SGW') {
        Write-Host "${Label}: no client (pid $ClientPid is not a running SGW)"
        return $true
    }
    # HasExited throws on a process this shell cannot query: treat that as alive.
    $exited = { try { $proc.HasExited } catch { $false } }
    try { $null = $proc.CloseMainWindow() } catch { }
    $deadline = (Get-Date).AddSeconds(8)
    while (-not (& $exited) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 250 }
    if (-not (& $exited)) {
        Stop-Process -InputObject $proc -Force -ErrorAction SilentlyContinue
        $deadline = (Get-Date).AddSeconds(2)
        while (-not (& $exited) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 100 }
    }
    if (-not (& $exited)) {
        [Console]::Error.WriteLine("${Label}: could not stop pid $ClientPid")
        return $false
    }
    Write-Host "${Label}: stopped pid $ClientPid"
    return $true
}

# The stop command: prints one line per instance and returns the exit code.
function Invoke-ClientsStop([string]$Verb, [string]$Which, [bool]$Force) {
    if ($Verb -ne 'stop') {
        [Console]::Error.WriteLine('usage: lab clients stop [<instance>|all] [-Force]')
        return 2
    }
    $all = (-not $Which) -or ($Which -eq 'all')
    # Refused before the daemon is asked, so nothing is stopped.
    if ($Force -and $all) {
        [Console]::Error.WriteLine('-Force needs a named instance')
        return 2
    }
    $status = Get-LabStatus
    if (-not $status) {
        [Console]::Error.WriteLine('daemon not running (or status endpoint unavailable); nothing stopped')
        return 1
    }
    if (-not $all) {
        $names = @($status.instances | ForEach-Object { Get-Field $_ 'instance' })
        if ($names -notcontains $Which) {
            [Console]::Error.WriteLine("unknown instance '$Which'; the daemon hosts: $($names -join ', ')")
            return 2
        }
    }

    $sel = Select-StopTargets $status $Which $Force
    $forcedNames = @($sel.forced | ForEach-Object { Get-Field $_ 'instance' })
    foreach ($inst in $sel.none) { Write-Host "$(Get-Field $inst 'instance'): no client" }
    foreach ($inst in $sel.refuse) {
        Write-Host "$(Get-Field $inst 'instance'): leased to $(Get-LeaseWho $inst); not stopped (use -Force to override)"
    }
    $failed = 0
    foreach ($inst in $sel.stop) {
        $label = [string](Get-Field $inst 'instance')
        if ($forcedNames -contains $label) {
            Write-Host "${label}: leased to $(Get-LeaseWho $inst); stopping anyway (-Force). The holder loses the client and the watchdog may relaunch it."
        }
        if (-not (Stop-LabClient $label (Get-ClientPid $inst))) { $failed++ }
    }
    if ($failed) { return 4 }
    if ($sel.refuse.Count) { return 3 }
    return 0
}

if ($MyInvocation.InvocationName -ne '.') {
    exit (Invoke-ClientsStop $Verb $Which ([bool]$Force))
}
