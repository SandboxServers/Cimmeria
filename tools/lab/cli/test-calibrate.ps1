<#
.SYNOPSIS
    Tests for `lab calibrate` (calibrate-lib.ps1, calibrate.ps1), no Pester
    and no daemon. Exits non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-calibrate.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'calibrate-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}
$pwshExe = (Get-Process -Id $PID).Path

# --- argument checks ---
Check ((Test-CalibrateArguments 'first-session' '' $null) -like 'usage: lab calibrate*') 'no row is a usage error'
Check ((Test-CalibrateArguments '' '' $null) -like 'usage: lab calibrate*') 'no section is a usage error'
Check ((Test-CalibrateArguments 'first-session' 'FS-P3' 0) -ceq '-BudgetMinutes must be 1 to 30') 'a budget of 0 is refused'
Check ((Test-CalibrateArguments 'first-session' 'FS-P3' 31) -ceq '-BudgetMinutes must be 1 to 30') 'a budget of 31 is refused'
Check ($null -eq (Test-CalibrateArguments 'first-session' 'FS-P3' 1) -and $null -eq (Test-CalibrateArguments 'first-session' 'FS-P3' 30)) 'budgets 1 and 30 are allowed'
Check ($null -eq (Test-CalibrateArguments 'first-session' 'FS-P3' $null)) 'no budget is the tool default'

# --- one line per pose ---
$frost = [pscustomobject]@{ ok = $true; section = 'first-session'; row = 'FS-P3'; pose = 'frost'; score = 8312; confirmed = 3
    changed = [pscustomobject]@{ bearing_deg = @(322.5, 330.0) }
    diff_path = 'C:\lab\uat-runs\2026-10-11-1489a731-kqzxwbt\first-session.diff'; probes = 41 }
$line = Format-CalibrateLine $frost
Check ($line -ceq 'frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff C:\lab\uat-runs\2026-10-11-1489a731-kqzxwbt\first-session.diff') "a changed pose's line ($line)"
$same = [pscustomobject]@{ ok = $true; section = 'first-session'; row = 'FS-P3'; pose = 'supply'; score = 7400; confirmed = 3; probes = 30 }
$line = Format-CalibrateLine $same
Check ($line -ceq 'supply: score 7400, confirmed 3/3, unchanged') "an unchanged pose's line ($line)"
$zoom = [pscustomobject]@{ ok = $true; pose = 'guard'; score = 6000; confirmed = 3; probes = 9
    changed = [pscustomobject]@{ zoom = @('none', 250.0); yaw_offset_deg = @(0, -12.5) } }
$line = Format-CalibrateLine $zoom
Check ($line -ceq 'guard: score 6000, confirmed 3/3, zoom none -> 250.0, yaw 0.0 -> -12.5') "an unset field reads none, every changed field is listed ($line)"
$miss = [pscustomobject]@{ ok = $false; pose = 'frost'; confirmed = 0; probes = 120; reason = 'no candidate hovered the target' }
$line = Format-CalibrateLine $miss
Check ($line -ceq 'frost: no proposal: no candidate hovered the target') "a pose with no candidate ($line)"
$lease = [pscustomobject]@{ ok = $false; pose = 'frost'; score = 5100; confirmed = 1; probes = 60; reason = 'lease revoked: lease-0123456789abcdef by x' }
Check ((Format-CalibrateLine $lease) -ceq 'frost: no proposal: lease revoked: lease-<redacted> by x (best score 5100, confirmed 1/3)') 'a failed pose with a best score; lease ids hidden'
$budget = [pscustomobject]@{ ok = $true; pose = 'frost'; score = 8000; confirmed = 3; probes = 99; reason = 'budget: swept 40 of 108 candidates' }
Check ((Format-CalibrateLine $budget) -ceq 'frost: score 8000, confirmed 3/3, unchanged (budget: swept 40 of 108 candidates)') 'an early stop says why'

# --- exit codes ---
Check ((Get-CalibrateExitCode @($frost, $same)) -eq 0) 'every pose proposed or unchanged exits 0'
Check ((Get-CalibrateExitCode @($frost, $miss)) -eq 1) 'a pose with no candidate exits 1'
Check ((Get-CalibrateExitCode @()) -eq 1) 'no outcomes exits 1'

# --- -Json (D-SV12) ---
$long = [pscustomobject]@{ ok = $true; section = 'first-session'; row = 'FS-P3'; pose = 'frost'; score = 8312; confirmed = 3
    changed = [pscustomobject]@{ bearing_deg = @(322.5, 330.0); radius_m = @(3.75, 4.5) }
    diff_path = 'C:\Users\lab-operator\AppData\Local\cimmeria-lab\uat-runs\2026-10-11-1489a731-kqzxwbt\first-session.diff'; probes = 41 }
$failLong = [pscustomobject]@{ ok = $false; section = 'first-session'; row = 'FS-P3'; pose = 'supply'; confirmed = 0; probes = 300
    reason = ('budget: not started ' + ('x' * 300)) }
$j = ConvertTo-CalibrateCompactJson 'first-session' 'FS-P3' @($long, $failLong)
Check ($j.Length -lt 400) "two outcomes stay under 400 characters ($($j.Length))"
Check ($j -notmatch 'null' -and $j -notmatch '\[\]') 'no nulls and no empty arrays'
$o = $j | ConvertFrom-Json
Check (-not $o.ok -and $o.section -ceq 'first-session' -and $o.row -ceq 'FS-P3' -and @($o.outcomes).Count -eq 2) 'the object names the section and row, ok false when a pose failed'
Check ($o.outcomes[0].changed.bearing -ceq '322.5 -> 330.0' -and $o.outcomes[0].changed.radius -ceq '3.75 -> 4.5' -and $o.outcomes[0].diff -like '*first-session.diff') 'changed fields and the diff path'
Check (-not ($o.outcomes[1].PSObject.Properties.Name -contains 'score') -and -not ($o.outcomes[1].PSObject.Properties.Name -contains 'diff') -and -not ($o.outcomes[1].PSObject.Properties.Name -contains 'changed')) 'a failed pose has no score, diff or changed'
Check ($o.outcomes[1].reason.Length -le 40) 'a reason is cut to 40 characters'
Check (-not ($o.outcomes[0].PSObject.Properties.Name -contains 'probes')) 'probes and candidates stay out'
$j = ConvertTo-CalibrateCompactJson 'first-session' 'FS-P3' @($same) $true | ConvertFrom-Json
Check ($j.ok -and $j.applied -and -not ($j.outcomes[0].PSObject.Properties.Name -contains 'changed')) 'an unchanged pose has no changed key; applied is shown'
$j = ConvertTo-CalibrateCompactJson 'first-session' 'FS-P3' @()
Check ($j -ceq '{"ok":false,"section":"first-session","row":"FS-P3"}') "no outcomes: no empty array ($j)"

# --- diff target ---
$diffText = "--- a/docs/guides/uat-specs/first-session.toml`n+++ b/docs/guides/uat-specs/first-session.toml`n@@ -1,1 +1,1 @@`n-a`n+b`n"
Check ((Get-CalibrateDiffTarget $diffText) -ceq 'docs/guides/uat-specs/first-session.toml') 'the diff names its spec file'
Check ($null -eq (Get-CalibrateDiffTarget 'no header')) 'a diff with no +++ line names nothing'
Check ((Get-CalibrateTimeoutSec $null) -eq 1200 -and (Get-CalibrateTimeoutSec 30) -eq 2100) 'the call timeout is the budget plus 5 minutes'

# --- -Apply in a scratch checkout: never clobbers, never commits ---
$scratch = Join-Path ([System.IO.Path]::GetTempPath()) ("calibrate-apply-" + [guid]::NewGuid())
$repo = Join-Path $scratch 'repo'
New-Item -ItemType Directory -Path (Join-Path $repo 'docs\guides\uat-specs') -Force | Out-Null
try {
    function G { & git -C $repo @args 2>&1 | Out-Null }
    G init -q .
    G config user.email test@example.invalid
    G config user.name test
    G config core.autocrlf false
    $spec = Join-Path $repo 'docs\guides\uat-specs\x.toml'
    [IO.File]::WriteAllText($spec, "a`nb`nc`n")
    G add -A; G commit -q -m init
    $head = (& git -C $repo rev-parse HEAD)
    # The diffs live outside the checkout: git add -A and reset --hard below would take them.
    $diff = Join-Path $scratch 'x.diff'
    [IO.File]::WriteAllText($diff, "--- a/docs/guides/uat-specs/x.toml`n+++ b/docs/guides/uat-specs/x.toml`n@@ -1,3 +1,3 @@`n a`n-b`n+B`n c`n")

    # A local edit: refused, the file left as it was.
    [IO.File]::WriteAllText($spec, "a`nb`nc`nlocal`n")
    $r = Invoke-CalibrateApply $repo $diff
    Check (-not $r.Ok -and $r.Error -like '*uncommitted changes*' -and [IO.File]::ReadAllText($spec) -ceq "a`nb`nc`nlocal`n") 'a spec with local edits is refused and left alone'
    # A staged edit counts too.
    G add -A
    Check (-not (Invoke-CalibrateApply $repo $diff).Ok) 'a staged edit is refused too'
    G reset -q --hard

    $r = Invoke-CalibrateApply $repo $diff
    $staged = @(& git -C $repo diff --cached --name-only)
    Check ($r.Ok -and [IO.File]::ReadAllText($spec) -ceq "a`nB`nc`n") 'a clean spec gets the diff'
    Check ((& git -C $repo rev-parse HEAD) -ceq $head -and -not $staged.Count) 'nothing is staged or committed'

    $other = Join-Path $scratch 'other.diff'
    [IO.File]::WriteAllText($other, "--- a/elsewhere/x.toml`n+++ b/elsewhere/x.toml`n@@ -1,1 +1,1 @@`n-a`n+b`n")
    Check ((Invoke-CalibrateApply $repo $other).Error -like '*not this checkout*') "a diff outside the spec folder is not applied"
    Check (-not (Invoke-CalibrateApply $repo (Join-Path $repo 'missing.diff')).Ok) 'no diff, nothing applied'
} finally { Remove-Item -LiteralPath $scratch -Recurse -Force -ErrorAction SilentlyContinue }

# --- through a real process: usage errors exit 2 before the daemon is
# asked, and -Apply with -Json passes the argument checks. The lab home
# points at a temp folder whose labd.pid binds a closed port, so no daemon
# can ever be reached and nothing is driven. ---
$cal = Join-Path $PSScriptRoot 'calibrate.ps1'
$home0 = $env:CIMMERIA_LAB_HOME
$fakeHome = Join-Path ([System.IO.Path]::GetTempPath()) ("calibrate-home-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $fakeHome | Out-Null
Set-Content -LiteralPath (Join-Path $fakeHome 'labd.pid') -Value '{"pid":1,"bind":"127.0.0.1:9"}'
try {
    $env:CIMMERIA_LAB_HOME = $fakeHome
    $out = @(& $pwshExe -NoProfile -File $cal 'first-session' -Json 2>$null)
    $code = $LASTEXITCODE
    $obj = ($out | Select-Object -Last 1) | ConvertFrom-Json
    Check ($code -eq 2 -and -not $obj.ok -and $obj.exit -eq 2 -and $obj.error -like 'usage: lab calibrate*') "no row exits 2 with one JSON object ($code)"
    $out = @(& $pwshExe -NoProfile -File $cal 'first-session' 'FS-P3' -BudgetMinutes 31 -Json 2>$null)
    $code = $LASTEXITCODE
    Check ($code -eq 2 -and (($out | Select-Object -Last 1) | ConvertFrom-Json).error -ceq '-BudgetMinutes must be 1 to 30') "-BudgetMinutes 31 exits 2 ($code)"
    $out = @(& $pwshExe -NoProfile -File $cal 'no-such-section' 'FS-P3' -Apply -Json 2>$null)
    $code = $LASTEXITCODE
    $last = [string]($out | Select-Object -Last 1)
    $obj = $last | ConvertFrom-Json
    Check ($code -eq 3 -and $obj.exit -eq 3 -and $obj.error -like '*daemon is not answering*' -and $last.Length -lt 400) "-Apply -Json passes the argument checks and stops at the pre-flight ($code)"
} finally {
    $env:CIMMERIA_LAB_HOME = $home0
    Remove-Item -LiteralPath $fakeHome -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
