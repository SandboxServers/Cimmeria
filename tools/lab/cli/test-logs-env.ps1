<#
.SYNOPSIS
    Tests for tools/lab/cli/logs.ps1 (no Pester needed). Exits
    non-zero on the first failure:

        pwsh -NoProfile -File tools/lab/cli/test-logs-env.ps1

    The pure functions are dot-sourced. The command cases run the real
    scripts on a temp lab home (CIMMERIA_LAB_HOME), so the real labd.env and
    labd.log are never read or written. The environment is restored afterwards.
#>
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'logs.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$hex = 'ab' * 32
$logLines = @(
    '2026-10-10T20:00:00.000001Z  INFO serve: instance=Some("p2") started',
    '2026-10-10T20:00:01.000001Z  WARN serve: instance=None idle',
    '2026-10-10T20:00:02.000001Z DEBUG serve: instance="P2" bearer abc123def',
    "2026-10-10T20:00:03.000001Z ERROR serve: key $hex",
    '2026-10-10T20:00:04.000001Z  TRACE serve: instance="default" ping',
    'continuation line with no level'
)

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("lab-logs-env-test-" + [guid]::NewGuid())
New-Item -ItemType Directory $tmp | Out-Null
$savedHome = $env:CIMMERIA_LAB_HOME
try {
    # --- Select-LabLogLine: instance ---
    $byInstance = @($logLines | Select-LabLogLine -Instance 'p2')
    Check ($byInstance.Count -eq 2) 'instance p2 keeps instance=Some("p2") and instance="P2" (ignoring case)'
    Check (@($logLines | Select-LabLogLine -Instance 'p').Count -eq 0) 'instance matches the whole name, not a prefix'

    # --- Select-LabLogLine: level ---
    $warnUp = @($logLines | Select-LabLogLine -Level 'warn')
    Check ($warnUp.Count -eq 2 -and $warnUp[0] -match 'WARN' -and $warnUp[1] -match 'ERROR') 'level warn keeps WARN and ERROR only'
    Check (@($logLines | Select-LabLogLine -Level 'info').Count -eq 3) 'level info keeps INFO, WARN and ERROR'
    Check (@($logLines | Select-LabLogLine -Instance 'p2' -Level 'info').Count -eq 1) 'instance and level filters combine'
    Check (@($logLines | Select-LabLogLine).Count -eq 6) 'no filter keeps every line, including the continuation line'

    # --- Select-LabLogLine: continuation lines follow their entry ---
    $panic = @(
        '2026-10-10T20:00:05.000001Z ERROR serve: instance=Some("p2") panicked',
        '   at error handling here',
        '   stack frame 2',
        '2026-10-10T20:00:06.000001Z  INFO serve: instance=Some("p3") fine',
        '   at error handling there'
    )
    $errs = @($panic | Select-LabLogLine -Level 'error')
    Check ($errs.Count -eq 3 -and $errs[2] -match 'frame 2') 'a kept ERROR entry keeps its continuation lines'
    Check (@($errs | Where-Object { $_ -match 'there' }).Count -eq 0) "a continuation line whose second word is a level name is not ranked on its own"
    $p2 = @($panic | Select-LabLogLine -Instance 'p2')
    Check ($p2.Count -eq 3) 'an -Instance match keeps the entry continuation lines'

    # --- Select-LabLogLine: redaction ---
    $all = @($logLines | Select-LabLogLine)
    Check ($all[2] -notmatch 'abc123def' -and $all[2] -match '<redacted>') 'a bearer token is masked'
    Check ($all[3] -notmatch $hex -and $all[3] -match '<redacted>') 'a 64-hex token is masked'

    # --- logs.ps1 on a temp lab home ---
    $env:CIMMERIA_LAB_HOME = $tmp
    $logsPs1 = Join-Path $PSScriptRoot 'logs.ps1'
    [System.IO.File]::WriteAllLines((Join-Path $tmp 'labd.log'), [string[]]$logLines)

    $tail = pwsh -NoProfile -File $logsPs1 -Instance p2 -Lines 1
    Check ($LASTEXITCODE -eq 0 -and "$tail" -match 'DEBUG' -and "$tail" -notmatch 'abc123def') 'logs -Instance p2 -Lines 1 prints the last p2 line, masked'
} finally {
    $env:CIMMERIA_LAB_HOME = $savedHome
    Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) failed"; exit 1 }
Write-Host 'all passed'
