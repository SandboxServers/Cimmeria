<#
.SYNOPSIS
    Tests for tools/lab/cli/common.ps1 (no Pester needed). Exits non-zero on
    the first failure:

        pwsh -NoProfile -File tools/lab/cli/test-common.ps1

    It points CIMMERIA_LAB_HOME (the lab home override) and LOCALAPPDATA at a
    temp folder, so the real labd.env, labd.pid and daemon are never read or
    written. The environment is restored afterwards.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-cli-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$saved = @{
    Home  = $env:CIMMERIA_LAB_HOME
    Local = $env:LOCALAPPDATA
    Root  = $env:CIMMERIA_LAB_PROFILE_ROOT
    Token = $env:CIMMERIA_LAB_DAEMON_TOKEN
}
try {
    # --- Format-Ago ---
    Check ((Format-Ago 0) -eq '0s') 'Format-Ago 0 is 0s'
    Check ((Format-Ago 59) -eq '59s') 'Format-Ago 59 is 59s'
    Check ((Format-Ago 61) -eq '1m') 'Format-Ago 61 is 1m'
    Check ((Format-Ago 7380) -eq '2h 3m') 'Format-Ago 7380 is 2h 3m'
    Check ((Format-Ago 90) -eq '1m') 'Format-Ago 90 is 1m (truncates, not rounds)'
    Check ((Format-Ago 3599) -eq '59m') 'Format-Ago 3599 is 59m'
    Check ((Format-Ago 5400) -eq '1h 30m') 'Format-Ago 5400 is 1h 30m'

    # --- lab.ps1: named arguments reach the command, exit codes pass through ---
    # A copy of the dispatcher with two fake commands, run the way lab.cmd runs it.
    $disp = Join-Path $tmp 'dispatch'
    New-Item -ItemType Directory (Join-Path $disp 'cli') | Out-Null
    Copy-Item (Join-Path $PSScriptRoot '..\lab.ps1') $disp
    Set-Content (Join-Path $disp 'cli\echoargs.ps1') -Value @(
        'param([int]$Lines = 10, [switch]$Follow, [string]$Instance)',
        '"$Lines|$Follow|$Instance"')
    Set-Content (Join-Path $disp 'cli\seven.ps1') -Value 'exit 7'
    $labPs1 = Join-Path $disp 'lab.ps1'
    $out = pwsh -NoProfile -File $labPs1 echoargs -Instance p2 -Lines 5 -Follow
    Check ($LASTEXITCODE -eq 0 -and "$out" -eq '5|True|p2') "named arguments reach the command (got '$out', exit $LASTEXITCODE)"
    $null = pwsh -NoProfile -File $labPs1 seven
    Check ($LASTEXITCODE -eq 7) "a command's exit code passes through (got $LASTEXITCODE)"
    # One argument after the command (lab instances status) must stay one
    # argument, not be splatted a character at a time.
    Set-Content (Join-Path $disp 'cli\verb.ps1') -Value @(
        'param([Parameter(Position = 0)][ValidateSet(''init'', ''status'')][string]$Verb)',
        '"verb=$Verb"')
    $out = pwsh -NoProfile -File $labPs1 verb status 2>&1
    Check ($LASTEXITCODE -eq 0 -and "$out" -eq 'verb=status') "a single argument reaches the command whole (got '$out', exit $LASTEXITCODE)"
    $out = pwsh -NoProfile -File $labPs1 echoargs -Follow 2>&1
    Check ($LASTEXITCODE -eq 0 -and "$out" -eq '10|True|') "a single switch reaches the command (got '$out', exit $LASTEXITCODE)"
    $null = pwsh -NoProfile -File $labPs1 nope 2>$null
    Check ($LASTEXITCODE -eq 2) 'an unknown command exits 2'

    # --- lab home: CIMMERIA_LAB_HOME moves labd.env and labd.pid ---
    $labHome = Join-Path $tmp 'home'
    New-Item -ItemType Directory $labHome | Out-Null
    $env:LOCALAPPDATA = Join-Path $tmp 'local'
    $env:CIMMERIA_LAB_HOME = $labHome
    Check ((Get-LabHome) -eq $labHome) 'CIMMERIA_LAB_HOME overrides the lab home'
    Check ((Get-LabdEnvPath) -eq (Join-Path $labHome 'labd.env')) 'labd.env lives under the lab home'
    $env:CIMMERIA_LAB_HOME = $null
    Check ((Get-LabHome) -eq "$tmp\local\cimmeria-lab") 'the default lab home is %LOCALAPPDATA%\cimmeria-lab'
    $env:CIMMERIA_LAB_HOME = $labHome

    # --- Get-ProfileRoot: labd.env, then the environment, then the default ---
    $envFile = Join-Path $labHome 'labd.env'
    $env:CIMMERIA_LAB_PROFILE_ROOT = 'D:\from-env'
    Write-LabdEnvFile $envFile @('CIMMERIA_LAB_PROFILE_ROOT=D:\from-file')
    Check ((Get-ProfileRoot) -eq 'D:\from-file') 'labd.env wins over the environment'
    Remove-Item -Force $envFile
    Check ((Get-ProfileRoot) -eq 'D:\from-env') 'the environment is used without labd.env'
    $env:CIMMERIA_LAB_PROFILE_ROOT = $null
    Check ((Get-ProfileRoot) -eq "$tmp\local\cimmeria-lab\instances") 'the default profile root is under LOCALAPPDATA'

    # --- Get-LabStatus: null, not a throw, when nothing listens ---
    # The pidfile names bind 127.0.0.1:9 (discard, closed here).
    $env:CIMMERIA_LAB_DAEMON_TOKEN = 'test-token-not-real'
    [System.IO.File]::WriteAllText((Join-Path $labHome 'labd.pid'), '{"pid":4242,"bind":"127.0.0.1:9"}')
    Check ((Get-DaemonInfo).bind -eq '127.0.0.1:9') 'Get-DaemonInfo reads labd.pid'
    Check ($null -eq (Get-LabStatus)) 'Get-LabStatus is null when nothing listens'
} finally {
    $env:CIMMERIA_LAB_HOME = $saved.Home
    $env:LOCALAPPDATA = $saved.Local
    $env:CIMMERIA_LAB_PROFILE_ROOT = $saved.Root
    $env:CIMMERIA_LAB_DAEMON_TOKEN = $saved.Token
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
