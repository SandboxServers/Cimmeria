<#
.SYNOPSIS
    Tests for the lab ops commands: clients stop (Select-StopTargets) and the
    doctor checks (no Pester needed). Exits non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-ops.ps1

.DESCRIPTION
    Dot-sources clients.ps1 and doctor.ps1, which define their functions
    without running their commands. No client is stopped: the stop tests use a
    fake status, and the only command run is the -Force refusal, which exits
    before the daemon is asked. CIMMERIA_LAB_HOME and LOCALAPPDATA point at a
    temp folder, so the real labd.env, labd.pid and install are never read.
    The environment is restored afterwards.
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'clients.ps1')
. (Join-Path $PSScriptRoot 'doctor.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# A /status-shaped object: default is leased, p2 is free with a client, p3 is free without one.
function New-FakeStatus {
    [pscustomobject]@{
        daemon = [pscustomobject]@{ pid = 4242; version = '0.1.0'; uptime_s = 60 }
        instances = @(
            [pscustomobject]@{
                instance = 'default'; account = 'lab'; bridge_port = 8770; client_pid = 101
                lease = [pscustomobject]@{ held = $true; lease = [pscustomobject]@{ owner = 'lp07-lab'; purpose = 'UAT'; remaining_s = 600 } }
            },
            [pscustomobject]@{
                instance = 'p2'; account = 'lab2'; bridge_port = 8771; client_pid = 202
                lease = [pscustomobject]@{ held = $false }
            },
            [pscustomobject]@{
                instance = 'p3'; account = $null; bridge_port = 8772; client_pid = $null
                lease = [pscustomobject]@{ held = $false }
            }
        )
    }
}

function Names($Items) { @($Items | ForEach-Object { $_.instance }) -join ',' }

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-ops-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$saved = @{
    Home  = $env:CIMMERIA_LAB_HOME
    Local = $env:LOCALAPPDATA
    Root  = $env:CIMMERIA_LAB_PROFILE_ROOT
}
try {
    $env:LOCALAPPDATA = Join-Path $tmp 'local'
    $env:CIMMERIA_LAB_HOME = Join-Path $tmp 'home'
    $env:CIMMERIA_LAB_PROFILE_ROOT = $null
    New-Item -ItemType Directory $env:CIMMERIA_LAB_HOME | Out-Null

    # --- Select-StopTargets (pure) ---
    $fake = New-FakeStatus
    $sel = Select-StopTargets $fake 'all' $false
    Check ((Names $sel.stop) -eq 'p2') 'all, no -Force: only the free client with a client is stopped'
    Check ((Names $sel.refuse) -eq 'default') 'all, no -Force: the leased client is refused'
    Check ((Names $sel.none) -eq 'p3') 'all, no -Force: the instance without a client is none'
    Check (@($sel.forced).Count -eq 0) 'all, no -Force: nothing is forced'

    $sel = Select-StopTargets (New-FakeStatus) '' $false
    Check ((Names $sel.refuse) -eq 'default' -and (Names $sel.stop) -eq 'p2') 'no instance means all'

    $sel = Select-StopTargets (New-FakeStatus) 'default' $false
    Check ((Names $sel.refuse) -eq 'default' -and @($sel.stop).Count -eq 0) 'a leased instance by name, no -Force: refused'

    $sel = Select-StopTargets (New-FakeStatus) 'default' $true
    Check ((Names $sel.forced) -eq 'default' -and (Names $sel.stop) -eq 'default') 'a leased instance by name with -Force: forced and stopped'
    Check (@($sel.refuse).Count -eq 0) 'a leased instance by name with -Force: not refused'

    $sel = Select-StopTargets (New-FakeStatus) 'all' $true
    Check (@($sel.forced).Count -eq 0 -and (Names $sel.refuse) -eq 'default') '-Force never applies to all'

    $sel = Select-StopTargets (New-FakeStatus) 'p3' $false
    Check ((Names $sel.none) -eq 'p3' -and @($sel.stop).Count -eq 0) 'a named instance without a client is none'

    # A /status without client_pid or lease.held must not throw under strict mode.
    $bare = [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'p4' }) }
    $sel = Select-StopTargets $bare 'all' $false
    Check ((Names $sel.none) -eq 'p4') 'a /status without client_pid or lease reads as no client'

    # --- clients stop: the refusals exit before the daemon is asked ---
    # The temp labd.pid points at a closed port, so if a refusal ever reached
    # the daemon it would find none (exit 1), never the real one.
    [System.IO.File]::WriteAllText((Join-Path $env:CIMMERIA_LAB_HOME 'labd.pid'), '{"pid":1,"bind":"127.0.0.1:9"}')
    $clients = Join-Path $PSScriptRoot 'clients.ps1'
    $null = pwsh -NoProfile -File $clients stop all -Force 2>$null
    Check ($LASTEXITCODE -eq 2) "'clients stop all -Force' exits 2 (got $LASTEXITCODE)"
    $null = pwsh -NoProfile -File $clients stop -Force 2>$null
    Check ($LASTEXITCODE -eq 2) "'clients stop -Force' with no instance exits 2 (got $LASTEXITCODE)"
    $null = pwsh -NoProfile -File $clients start 2>$null
    Check ($LASTEXITCODE -eq 2) "a verb other than stop exits 2 (got $LASTEXITCODE)"

    # --- doctor: profile root inside and outside the install dir ---
    $install = Join-Path $tmp 'Stargate Worlds'
    Check ((Get-ProfileRootCheck (Join-Path $install 'instances') $install).Status -eq 'FAIL') 'a profile root inside the install dir FAILs'
    Check ((Get-ProfileRootCheck (Join-Path $tmp 'instances') $install).Status -eq 'PASS') 'a profile root outside the install dir PASSes'
    Check ((Get-ProfileRootCheck "$install`X\instances" $install).Status -eq 'PASS') 'a name that only starts with the install dir is outside it'
    Check ((Get-ProfileRootCheck 'instances' $install).Status -eq 'FAIL') 'a relative profile root FAILs'

    # --- doctor: the other checks, with fakes and temp files ---
    Check ((Get-TaskCheck $false).Status -eq 'FAIL') 'a missing scheduled task FAILs'
    Check ((Get-TaskCheck $true).Status -eq 'PASS') 'an installed scheduled task PASSes'
    Check ((Get-DaemonCheck $null).Status -eq 'FAIL') 'no /status FAILs the daemon check'
    Check ((Get-DaemonCheck $fake).Status -eq 'PASS') 'a /status PASSes the daemon check'
    Check ((Get-TokenCheck $false).Status -eq 'FAIL') 'an unset token FAILs'
    Check ((Get-TokenCheck $true).Status -eq 'PASS') 'a set token PASSes'
    Check ((Get-InstallCheck $null).Status -eq 'FAIL') 'no install dir FAILs'

    New-Item -ItemType Directory (Join-Path $install 'Binaries\sessions') | Out-Null
    Check ((Get-InstallCheck $install).Status -eq 'FAIL') 'an install dir without SGW.exe FAILs'
    Set-Content -LiteralPath (Join-Path $install 'Binaries\SGW.exe') -Value 'x'
    Check ((Get-InstallCheck $install).Status -eq 'PASS') 'an install dir with SGW.exe PASSes'

    Set-Content -LiteralPath (Join-Path $install 'Binaries\sessions\lab-account.json') -Value '{}'
    $profileRoot = Join-Path $tmp 'instances'
    New-Item -ItemType Directory (Join-Path $profileRoot 'default\profile\Documents\My Games\Firesky\SGWGame') | Out-Null
    $acct = Get-AccountCheck $install @('default', 'p2')
    Check ($acct.Status -eq 'WARN' -and $acct.Detail -match 'p2' -and $acct.Detail -notmatch 'default') 'a missing p2 account file WARNs, naming p2'
    Check ((Get-AccountCheck $install @('default')).Status -eq 'PASS') 'the default account alone PASSes'
    $seed = Get-SeedCheck $profileRoot @('default', 'p2')
    Check ($seed.Status -eq 'WARN' -and $seed.Detail -match 'p2' -and $seed.Detail -notmatch 'default') 'an unseeded p2 WARNs, naming p2'
    Check ((Get-SeedCheck $profileRoot @('default')).Status -eq 'PASS') 'a seeded default PASSes'

    $stray = Get-StraySgwCheck @(10, 20) @(10)
    Check ($stray.Status -eq 'WARN' -and $stray.Detail -match '20') 'an SGW.exe pid that /status does not list WARNs, naming it'
    Check ((Get-StraySgwCheck @(10) @(10)).Status -eq 'PASS') 'a listed SGW.exe pid PASSes'
    Check ((Get-StraySgwCheck @() @()).Status -eq 'PASS') 'no SGW.exe PASSes'

    $binA = Join-Path $tmp 'bin\cimmeria-lab.exe'
    $binB = Join-Path $tmp 'labd\cimmeria-lab.exe'
    New-Item -ItemType Directory (Split-Path $binA), (Split-Path $binB) | Out-Null
    Set-Content -LiteralPath $binA -Value 'one'
    Set-Content -LiteralPath $binB -Value 'two'
    Check ((Get-BinaryCheck $binA $binB).Status -eq 'WARN') 'differing bin and labd copies WARN'
    Set-Content -LiteralPath $binB -Value 'one'
    Check ((Get-BinaryCheck $binA $binB).Status -eq 'PASS') 'identical bin and labd copies PASS'
    Check ((Get-BinaryCheck $binA (Join-Path $tmp 'none.exe')).Status -eq 'WARN') 'a missing copy WARNs'

    $daemonDown = Get-StraySgwCheck @(30) @() $false
    Check ($daemonDown.Status -eq 'WARN' -and $daemonDown.Detail -match 'daemon down' -and $daemonDown.Detail -match '30') 'with the daemon down, a running SGW.exe WARNs as unmatched'

    # Version: content of tools/lab against origin/main, in this checkout.
    $repo = (git -C $PSScriptRoot rev-parse --show-toplevel).Trim()
    $mainSha = (git -C $repo rev-parse --short origin/main).Trim()
    $labTouch = (git -C $repo rev-list -1 origin/main -- tools/lab).Trim()
    $beforeLab = (git -C $repo rev-parse --short "$labTouch^").Trim()
    Check ((Get-VersionCheck $null $repo).Status -eq 'WARN') 'no VERSION inside a checkout WARNs'
    Check ((Get-VersionCheck 'abc1234' $null).Status -eq 'PASS') 'outside a checkout the VERSION check is skipped (PASS)'
    Check ((Get-VersionCheck $mainSha $repo).Status -eq 'PASS') "origin/main's own sha PASSes"
    Check ((Get-VersionCheck $beforeLab $repo).Status -eq 'WARN') 'a sha whose tools/lab differs from origin/main WARNs'
    $unknown = Get-VersionCheck 'deadbee' $repo
    Check ($unknown.Status -eq 'WARN' -and $unknown.Detail -match 'unknown') 'an unknown sha WARNs as unknown'

    # Stop-LabClient acts only on an SGW process: this test's own pid is left alone.
    $ok = Stop-LabClient 'self' $PID 6>$null
    Check ($ok -and -not (Get-Process -Id $PID).HasExited) 'a pid that is not SGW is not stopped'
}
finally {
    $env:CIMMERIA_LAB_HOME = $saved.Home
    $env:LOCALAPPDATA = $saved.Local
    $env:CIMMERIA_LAB_PROFILE_ROOT = $saved.Root
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed) {
    Write-Host "$($script:failed) check(s) failed"
    exit 1
}
Write-Host 'all ops checks passed'
exit 0
