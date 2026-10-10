<#
.SYNOPSIS
    Check the lab install, the daemon and each instance: one PASS, WARN or FAIL line per check.

.DESCRIPTION
    Read-only. Prints `PASS|WARN|FAIL  <check>  <detail>` for each check and
    exits 1 when any check fails (WARN does not fail). The token is only ever
    reported as set or not set.

    Each check is a function returning [pscustomobject]@{ Status; Check; Detail },
    fed with facts the caller gathered, so tools/lab/cli/test-ops.ps1 dot-sources
    this file and calls them with fakes. The command runs only when it is not
    dot-sourced.

.EXAMPLE
    pwsh tools/lab/lab.ps1 doctor
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

# A property of a JSON object, or $null when the object lacks it (strict mode throws on a missing one).
function Get-Field($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

function New-CheckResult([string]$Status, [string]$Check, [string]$Detail) {
    return [pscustomobject]@{ Status = $Status; Check = $Check; Detail = $Detail }
}

function Get-TaskCheck([bool]$Installed) {
    $name = 'scheduled task CimmeriaLabDaemon exists'
    if ($Installed) { return New-CheckResult 'PASS' $name 'installed' }
    return New-CheckResult 'FAIL' $name 'not installed; run lab install'
}

function Get-DaemonCheck($Status) {
    $name = 'daemon answers /status'
    if (-not $Status) { return New-CheckResult 'FAIL' $name 'not running, or the status endpoint is down; run lab start' }
    $daemon = Get-Field $Status 'daemon'
    return New-CheckResult 'PASS' $name ('pid {0}, version {1}' -f (Get-Field $daemon 'pid'), (Get-Field $daemon 'version'))
}

function Get-TokenCheck([bool]$Set) {
    $name = 'CIMMERIA_LAB_DAEMON_TOKEN is set'
    if ($Set) { return New-CheckResult 'PASS' $name 'set' }
    return New-CheckResult 'FAIL' $name 'not set; run lab install'
}

function Get-InstallCheck([string]$InstallDir) {
    $name = 'labd.env has CIMMERIA_LAB_INSTALL_DIR and SGW.exe is under it'
    if (-not $InstallDir) { return New-CheckResult 'FAIL' $name 'CIMMERIA_LAB_INSTALL_DIR is not set in labd.env' }
    $exe = Join-Path $InstallDir 'Binaries\SGW.exe'
    if (-not (Test-Path -LiteralPath $exe)) { return New-CheckResult 'FAIL' $name "no $exe" }
    return New-CheckResult 'PASS' $name $exe
}

# The profile root must be absolute and outside the install dir (a path prefix
# that is only a name prefix, like C:\Games\SGWX, is outside C:\Games\SGW).
function Get-ProfileRootCheck([string]$Root, [string]$InstallDir) {
    $name = 'profile root is absolute and not inside the install dir'
    if (-not $Root -or -not [System.IO.Path]::IsPathFullyQualified($Root)) {
        return New-CheckResult 'FAIL' $name "not an absolute path: $Root"
    }
    $full = [System.IO.Path]::GetFullPath($Root).TrimEnd('\')
    if ($InstallDir) {
        $install = [System.IO.Path]::GetFullPath($InstallDir).TrimEnd('\')
        if ($full -ieq $install -or $full.StartsWith("$install\", [System.StringComparison]::OrdinalIgnoreCase)) {
            return New-CheckResult 'FAIL' $name "inside the install dir: $full"
        }
    }
    return New-CheckResult 'PASS' $name $full
}

# The account file of an instance: lab-account.json for default, else lab-account.<label>.json (instances.ps1).
function Get-AccountPath([string]$InstallDir, [string]$Label) {
    $file = if ($Label -eq 'default') { 'lab-account.json' } else { "lab-account.$Label.json" }
    return Join-Path $InstallDir "Binaries\sessions\$file"
}

function Get-AccountCheck([string]$InstallDir, [string[]]$Labels) {
    $name = "each instance's account file exists"
    if (-not $InstallDir) { return New-CheckResult 'WARN' $name 'no install dir; account files not checked' }
    $missing = @($Labels | Where-Object { -not (Test-Path -LiteralPath (Get-AccountPath $InstallDir $_)) })
    if ($missing.Count) { return New-CheckResult 'WARN' $name ("missing: {0}; run lab instances init" -f ($missing -join ', ')) }
    return New-CheckResult 'PASS' $name ("{0} present" -f $Labels.Count)
}

function Get-SeedCheck([string]$ProfileRoot, [string[]]$Labels) {
    $name = "each instance's profile is seeded"
    $unseeded = @($Labels | Where-Object {
        -not (Test-Path -LiteralPath (Join-Path $ProfileRoot "$_\profile\Documents\My Games\Firesky\SGWGame"))
    })
    if ($unseeded.Count) { return New-CheckResult 'WARN' $name ("not seeded: {0}" -f ($unseeded -join ', ')) }
    return New-CheckResult 'PASS' $name ("{0} seeded" -f $Labels.Count)
}

# Running SGW.exe pids that the daemon's /status does not list as an instance's client.
function Get-StraySgwCheck([int[]]$Running, [int[]]$Listed) {
    $name = 'no SGW.exe runs that /status does not list'
    $stray = @($Running | Where-Object { $Listed -notcontains $_ })
    if ($stray.Count) { return New-CheckResult 'WARN' $name ("not listed by /status: pid {0}" -f ($stray -join ', ')) }
    return New-CheckResult 'PASS' $name 'none'
}

# The installed exe (bin\) and the daemon's copy (labd\) must be the same file.
function Get-BinaryCheck([string]$BinExe, [string]$DaemonExe) {
    $name = "installed cimmeria-lab.exe is the daemon's copy"
    foreach ($exe in $BinExe, $DaemonExe) {
        if (-not (Test-Path -LiteralPath $exe)) { return New-CheckResult 'WARN' $name "missing: $exe; run lab install" }
    }
    $a = (Get-FileHash -LiteralPath $BinExe -Algorithm SHA256).Hash
    $b = (Get-FileHash -LiteralPath $DaemonExe -Algorithm SHA256).Hash
    if ($a -ne $b) { return New-CheckResult 'WARN' $name 'bin and labd copies differ; run lab restart' }
    return New-CheckResult 'PASS' $name "same SHA-256 ($($a.Substring(0, 12))...)"
}

# The CLI copy's VERSION must be an ancestor of origin/main. Only checked
# inside a git checkout; elsewhere it passes as skipped.
function Get-VersionCheck([string]$Sha, [bool]$InCheckout) {
    $name = 'CLI VERSION is an ancestor of origin/main'
    if (-not $InCheckout) { return New-CheckResult 'PASS' $name 'not in a git checkout; skipped' }
    if (-not $Sha) { return New-CheckResult 'WARN' $name 'no cli\cli\VERSION; run lab setup' }
    git merge-base --is-ancestor $Sha origin/main 2>$null | Out-Null
    switch ($LASTEXITCODE) {
        0 { return New-CheckResult 'PASS' $name "$Sha is in origin/main" }
        1 { return New-CheckResult 'WARN' $name "$Sha is not in origin/main; run lab setup" }
        default { return New-CheckResult 'WARN' $name "cannot compare $Sha with origin/main" }
    }
}

# Gathers the facts, runs every check and prints them. Returns the exit code.
function Invoke-Doctor {
    $labHome = Get-LabHome
    $status = Get-LabStatus
    $map = Get-LabdEnv
    $installDir = "$($map['CIMMERIA_LAB_INSTALL_DIR'])".Trim()
    if (-not $installDir) { $installDir = $null }

    # The instances the daemon hosts; without a daemon, the labd.env list (status.ps1's rule).
    if ($status) {
        $labels = @($status.instances | ForEach-Object { Get-Field $_ 'instance' })
    } else {
        $labels = @("$($map['CIMMERIA_LAB_INSTANCES'])".Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ })
        if (-not $labels) { $labels = @('default') }
    }
    $listed = @()
    if ($status) {
        $listed = @($status.instances | ForEach-Object { Get-Field $_ 'client_pid' } | Where-Object { "$_" -match '^\d+$' } | ForEach-Object { [int]$_ })
    }
    $running = @(Get-Process -Name SGW -ErrorAction SilentlyContinue | ForEach-Object { [int]$_.Id })

    $versionFile = Join-Path $labHome 'cli\cli\VERSION'
    $sha = $null
    if (Test-Path -LiteralPath $versionFile) { $sha = "$(Get-Content -LiteralPath $versionFile -TotalCount 1)".Trim() }
    git rev-parse --is-inside-work-tree 2>$null | Out-Null
    $inCheckout = ($LASTEXITCODE -eq 0)

    $results = @(
        Get-TaskCheck ([bool](Get-ScheduledTask -TaskName 'CimmeriaLabDaemon' -ErrorAction SilentlyContinue))
        Get-DaemonCheck $status
        Get-TokenCheck ([bool](Get-LabToken))
        Get-InstallCheck $installDir
        Get-ProfileRootCheck (Get-ProfileRoot) $installDir
        Get-AccountCheck $installDir $labels
        Get-SeedCheck (Get-ProfileRoot) $labels
        Get-StraySgwCheck $running $listed
        Get-BinaryCheck (Join-Path $labHome 'bin\cimmeria-lab.exe') (Join-Path $labHome 'labd\cimmeria-lab.exe')
        Get-VersionCheck $sha $inCheckout
    )
    foreach ($r in $results) { Write-Host ('{0}  {1}  {2}' -f $r.Status, $r.Check, $r.Detail) }
    if ($results.Status -contains 'FAIL') { return 1 }
    return 0
}

if ($MyInvocation.InvocationName -ne '.') {
    exit (Invoke-Doctor)
}
