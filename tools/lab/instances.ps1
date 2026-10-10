<#
.SYNOPSIS
    Write the account files for the extra lab clients (p2..p5) and report
    each lab instance's account, profile seed and client.

.DESCRIPTION
    Every lab client logs in with its own Stargate Worlds account, so
    several clients can be in the world at once. The default account is
    <InstallDir>\Binaries\sessions\lab-account.json. `init` writes
    lab-account.p<n>.json beside it for n = 2..Count: a copy with
    username `lab<n>` and character `Lab` + the ordinal word (Labtwo,
    Labthree, Labfour, Labfive). Every other field, the password included,
    is copied unchanged and never printed.

    `status` prints one line for the default instance and p2..p<Count>:
    the account's username and character, whether the instance's profile
    is seeded under <root>\<label>\profile, and whether that instance's
    lab-instance.json (under Binaries\sessions\instances\<label>) names a
    live client process. The root is the CIMMERIA_LAB_PROFILE_ROOT line of
    labd.env, else $env:CIMMERIA_LAB_PROFILE_ROOT, else
    %LOCALAPPDATA%\cimmeria-lab\instances. It prints "missing" rather than
    failing when the install or a file is absent.

    InstallDir defaults to $env:CIMMERIA_LAB_INSTALL_DIR, else the
    CIMMERIA_LAB_INSTALL_DIR line of %LOCALAPPDATA%\cimmeria-lab\labd.env.
    The runbook is docs/guides/live-research-lab.md.

.PARAMETER Command
    init | status

.PARAMETER InstallDir
    The SGW install folder, the one holding Binaries\SGW.exe.

.PARAMETER Count
    The highest instance number, 1..5. Default 5: the default instance
    plus p2..p5.

.PARAMETER Force
    init: rewrite account files that already exist.

.EXAMPLE
    pwsh tools/lab/instances.ps1 status
    pwsh tools/lab/instances.ps1 init
    pwsh tools/lab/instances.ps1 init -InstallDir 'D:\Games\Stargate Worlds' -Force
#>
[CmdletBinding()]
param(
    [Parameter(Position = 0, Mandatory = $true)]
    [ValidateSet('init', 'status')]
    [string]$Command,
    [string]$InstallDir,
    [ValidateRange(1, 5)]
    [int]$Count = 5,
    [switch]$Force
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'labd-lib.ps1')

$LabdEnv = Join-Path $env:LOCALAPPDATA 'cimmeria-lab\labd.env'
$Ordinals = @{ 2 = 'two'; 3 = 'three'; 4 = 'four'; 5 = 'five' }

function Get-InstallRoot {
    if ($InstallDir) { return $InstallDir }
    if ($env:CIMMERIA_LAB_INSTALL_DIR) { return $env:CIMMERIA_LAB_INSTALL_DIR }
    # Parsed the way the daemon parses it (last duplicate wins).
    $map = Read-LabdEnvFile $LabdEnv
    if ($map.Contains('CIMMERIA_LAB_INSTALL_DIR')) { return $map['CIMMERIA_LAB_INSTALL_DIR'] }
    return $null
}

function Get-SessionsDir {
    $root = Get-InstallRoot
    if (-not $root) { return $null }
    return Join-Path $root 'Binaries\sessions'
}

function Get-AccountPath([string]$File) {
    $sessions = Get-SessionsDir
    if (-not $sessions) { return $null }
    return Join-Path $sessions $File
}

function Get-InstancePath([string]$Label, [string]$Leaf) {
    $sessions = Get-SessionsDir
    if (-not $sessions) { return $null }
    return Join-Path $sessions "instances\$Label\$Leaf"
}

# Where the game's profiles live. The daemon applies labd.env over the
# process environment, so the file's value wins, then the variable, then
# the default. Whitespace-only values count as unset.
function Get-ProfileRoot {
    $map = Read-LabdEnvFile $LabdEnv
    foreach ($raw in @($map['CIMMERIA_LAB_PROFILE_ROOT'], $env:CIMMERIA_LAB_PROFILE_ROOT)) {
        if ("$raw".Trim()) { return "$raw".Trim() }
    }
    return Join-Path $env:LOCALAPPDATA 'cimmeria-lab\instances'
}

# The instances in order: the default account, then p2..p<Count>.
function Get-Instances {
    $list = @([pscustomobject]@{ Number = 1; Label = 'default'; Account = 'lab-account.json' })
    for ($n = 2; $n -le $Count; $n++) {
        $list += [pscustomobject]@{ Number = $n; Label = "p$n"; Account = "lab-account.p$n.json" }
    }
    return $list
}

function Test-Present([string]$Path) {
    return [bool]($Path -and (Test-Path -LiteralPath $Path))
}

function Read-Json([string]$Path) {
    if (-not (Test-Present $Path)) { return $null }
    try {
        return Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
    } catch {
        Write-Warning "unreadable JSON: $Path"
        return $null
    }
}

# Strict mode throws on a missing property, so status probes for it first.
function Get-OptionalProperty($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

function Invoke-Init {
    $source = Get-AccountPath 'lab-account.json'
    if (-not $source) { throw 'no SGW install folder: pass -InstallDir or set CIMMERIA_LAB_INSTALL_DIR' }
    if (-not (Test-Path -LiteralPath $source)) { throw "missing the default account file: $source" }

    $account = Get-Content -LiteralPath $source -Raw -Encoding UTF8 | ConvertFrom-Json
    foreach ($field in 'username', 'password') {
        if ($account.PSObject.Properties.Name -notcontains $field) { throw "$source has no '$field' field" }
    }
    $utf8 = [Text.UTF8Encoding]::new($false)
    foreach ($inst in Get-Instances) {
        if ($inst.Number -eq 1) { continue }
        $target = Get-AccountPath $inst.Account
        if ((Test-Path -LiteralPath $target) -and -not $Force) {
            Write-Host "kept     $target"
            continue
        }
        $copy = $account | ConvertTo-Json -Depth 32 | ConvertFrom-Json
        # character is optional in the Rust reader, so add it when absent.
        $copy | Add-Member -NotePropertyName username -NotePropertyValue "lab$($inst.Number)" -Force
        $copy | Add-Member -NotePropertyName character -NotePropertyValue ('Lab' + $Ordinals[$inst.Number]) -Force
        [IO.File]::WriteAllText($target, ($copy | ConvertTo-Json -Depth 32), $utf8)
        Write-Host "written  $target"
    }
}

function Invoke-Status {
    foreach ($inst in Get-Instances) {
        $account = Read-Json (Get-AccountPath $inst.Account)
        $username = Get-OptionalProperty $account 'username'
        $character = Get-OptionalProperty $account 'character'
        if (-not $username) { $username = 'missing' }
        if (-not $character) { $character = 'missing' }

        $seed = Join-Path (Get-ProfileRoot) "$($inst.Label)\profile\Documents\My Games\Firesky\SGWGame"
        $profile = if (Test-Present $seed) { 'seeded' } else { 'not seeded' }

        $info = Read-Json (Get-InstancePath $inst.Label 'lab-instance.json')
        $clientPid = Get-OptionalProperty $info 'pid'
        # A stale file plus pid reuse must not report a client: require SGW.exe.
        $proc = $null
        if ("$clientPid" -match '^\d+$') {
            $proc = Get-Process -Id ([int]$clientPid) -ErrorAction SilentlyContinue |
                Where-Object { $_.ProcessName -eq 'SGW' }
        }
        $client = if ($proc) { "client running pid $clientPid" } else { 'no client' }

        Write-Host ("{0,-8} {1,-10} {2,-12} {3,-11} {4}" -f $inst.Label, $username, $character, $profile, $client)
    }
}

switch ($Command) {
    'init' { Invoke-Init }
    'status' { Invoke-Status }
}
