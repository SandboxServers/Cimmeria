<#
.SYNOPSIS
    Tests for `lab record` (record-lib.ps1), no Pester and no daemon: every
    MCP call is stubbed. Exits non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-record.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'record-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$labHome = Join-Path ([IO.Path]::GetTempPath()) "lab-record-test-$PID"
New-Item -ItemType Directory -Force -Path $labHome | Out-Null

# The stub: each call is logged to $script:calls and answered by the next
# entry of $script:replies (an object to return, or a string to throw).
$script:calls = @()
$script:replies = @()
$stub = {
    param($Name, $Arguments, $TimeoutSec)
    $script:calls += , @{ Name = $Name; Args = $Arguments }
    $next = $script:replies[0]
    $script:replies = @($script:replies | Select-Object -Skip 1)
    if ($next -is [string]) { throw $next }
    return $next
}
function Ctx([bool]$Up = $true) {
    $probe = if ($Up) { { $true } } else { { $false } }
    return @{ LabHome = $labHome; UatRoot = 'C:\lab\uat-runs'; SpecsDir = 'C:\repo\docs\guides\uat-specs'; Probe = $probe; Call = $stub }
}
function Run([hashtable]$Opt, $Replies = @(), [bool]$Up = $true) {
    $script:calls = @()
    $script:replies = @($Replies)
    return Invoke-RecordAction $Opt (Ctx $Up)
}
function Reply([string]$Json) { return $Json | ConvertFrom-Json }
$id = '20261012-143000-p2'

try {
    # --- argument checks (no daemon) ---
    Check ((Test-RecordArguments @{ Action = '' }) -like 'usage: lab record*') 'no action prints the usage'
    Check ((Test-RecordArguments @{ Action = 'Start'; Target = 'first-session' }) -like 'usage:*') 'actions are lower case'
    Check ((Test-RecordArguments @{ Action = 'start' }) -like 'start needs a section*') 'start needs a section'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = 'First Session' }) -like 'section:*') 'a bad section is refused'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = ('a' * 49) }) -like 'section:*') 'a 49-character section is refused'
    Check ($null -eq (Test-RecordArguments @{ Action = 'start'; Target = 'first-session'; Instance = 'p2'; MaxMinutes = 30 })) 'a good start passes'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = 'fs'; MaxMinutes = 0 }) -like '-MaxMinutes must be 1 to 30') '-MaxMinutes 0 is refused'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = 'fs'; MaxMinutes = 31 }) -like '-MaxMinutes must be 1 to 30') '-MaxMinutes 31 is refused'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = 'fs'; MaxMinutes = 'abc' }) -like '-MaxMinutes must be 1 to 30') '-MaxMinutes abc is refused'
    Check ($null -eq (Test-RecordArguments @{ Action = 'start'; Target = 'fs'; MaxMinutes = '' })) 'an empty -MaxMinutes is not given'
    Check ((Test-RecordArguments @{ Action = 'status'; Instance = '..\x' }) -like '-Instance:*') 'a path-like instance is refused'
    Check ((Test-RecordArguments @{ Action = 'stop'; CharacterExpr = "x'; --" }) -like '-CharacterExpr:*') 'a quoting character_expr is refused'
    Check ($null -eq (Test-RecordArguments @{ Action = 'stop'; CharacterExpr = 'Px${run_id}' })) 'Px${run_id} passes'
    Check ((Test-RecordArguments @{ Action = 'start'; Target = 'fs'; NoDraft = $true }) -like '-NoDraft is for lab record stop') '-NoDraft on start is refused'
    Check ((Test-RecordArguments @{ Action = 'status'; Target = 'x' }) -like 'status takes no positional*') 'status takes no positional argument'
    Check ((Test-RecordArguments @{ Action = 'transform' }) -like 'transform needs a recording*') 'transform needs a recording'
    Check ((Test-RecordArguments @{ Action = 'transform'; Target = $id; Instance = 'p2' }) -like 'transform runs on no instance*') 'transform takes no -Instance'
    Check ((Test-RecordArguments @{ Action = 'transform'; Target = $id; Section = 'Bad_Section' }) -like '-Section:*') 'transform refuses a bad -Section'
    Check ((Test-RecordArguments @{ Action = 'status'; SpecsDir = 'C:\x' }) -like '-SpecsDir is for*') '-SpecsDir on status is refused'

    # --- state file path ---
    Check ((Get-RecordStatePath $labHome 'p2') -eq (Join-Path $labHome 'record-p2.json')) 'the state file is record-<instance>.json in the lab home'
    $threw = $false; try { Get-RecordStatePath $labHome '..\evil' | Out-Null } catch { $threw = $true }
    Check $threw 'a bad instance never becomes a path'
    Check ($null -eq (Read-RecordState (Join-Path $labHome 'record-none.json'))) 'no state file reads as nothing'
    Set-Content -LiteralPath (Join-Path $labHome 'record-junk.json') -Value 'not json'
    Check ($null -eq (Read-RecordState (Join-Path $labHome 'record-junk.json'))) 'an unreadable state file reads as nothing'

    # --- usage and no daemon: nothing is called ---
    $o = Run @{ Action = 'start'; Target = 'BAD' }
    Check ($o.Exit -eq 2 -and $script:calls.Count -eq 0) 'a usage error exits 2 before any call'
    $o = Run @{ Action = 'status' } -Up $false
    Check ($o.Exit -eq 3 -and $o.Body.error -like '*not answering*' -and $script:calls.Count -eq 0) 'no daemon exits 3'
    $o = Run @{ Action = 'transform'; Target = $id } -Up $false
    Check ($o.Exit -eq 3 -and $o.Body.error.Contains('runs in the lab daemon; start it with `lab start`')) 'transform without the daemon says to start it'

    # --- start writes the state file; stop reads it ---
    $state = Join-Path $labHome 'record-p2.json'
    $o = Run @{ Action = 'start'; Target = 'first-session'; Instance = 'p2'; MaxMinutes = 20 } @(
        (Reply "{`"ok`":true,`"recording_id`":`"$id`",`"instance`":`"p2`",`"section`":`"first-session`"}"))
    $sent = $script:calls[0].Args
    Check ($o.Exit -eq 0 -and $script:calls[0].Name -eq 'lab_record_start') 'start calls lab_record_start'
    Check ($sent.section -eq 'first-session' -and $sent.instance -eq 'p2' -and $sent.max_minutes -eq 20 -and $sent.specs_dir -like '*uat-specs') 'start sends section, instance, max_minutes and the spec folder'
    Check (-not $sent.ContainsKey('character')) 'start sends no character unless given'
    Check ((Read-RecordState $state).recording_id -eq $id -and (Read-RecordState $state).instance -eq 'p2') 'start saves {recording_id, instance}'
    Check ($o.Text -like "recording $id on p2*lab record stop -Instance p2") 'start says how to stop'

    # A failed stop (exit 1) keeps the file and prints the body as it came back.
    $body = '{"ok":false,"exit":1,"error":"C:\\lab\\records\\x; see transform-error.txt; transform: boom"}'
    $o = Run @{ Action = 'stop'; Instance = 'p2' } @("lab_record_stop failed: $body")
    Check ($o.Exit -eq 1 -and (ConvertTo-RecordCompactJson $o.Body) -eq $body) 'a failed stop prints the failure body unchanged'
    Check ($script:calls[0].Args.recording_id -eq $id -and $script:calls[0].Args.instance -eq 'p2') 'stop sends the saved id and instance'
    Check (Test-Path -LiteralPath $state) 'a failed stop keeps the state file'
    Check ($o.Text -like '*recording is kept; retry with lab record stop -Instance p2') 'a failed stop offers the retry'
    $o = Run @{ Action = 'stop'; Instance = 'p2' } @("lab_record_stop failed: {`"ok`":false,`"exit`":2,`"error`":`"character_expr: bad`"}")
    Check ($o.Exit -eq 2 -and (Test-Path -LiteralPath $state)) 'a refused stop exits 2 and keeps the state file'

    # The retry succeeds: the file goes, the answer names the draft and the raw log.
    $o = Run @{ Action = 'stop'; Instance = 'p2'; CharacterExpr = 'Px${run_id}'; Profile = 'fresh' } @(
        (Reply "{`"ok`":true,`"draft`":`"first-session-20261012-1430.toml`",`"rows`":3,`"steps`":9,`"todo`":1,`"dropped`":0,`"raw`":`"$id`"}"))
    Check ($o.Exit -eq 0 -and -not (Test-Path -LiteralPath $state)) 'a stop that succeeds deletes the state file'
    Check ($script:calls[0].Args.character_expr -eq 'Px${run_id}' -and $script:calls[0].Args.profile -eq 'fresh') 'stop passes -CharacterExpr and -Profile'
    Check ($o.Text -like '*uat-specs\drafts\first-session-20261012-1430.toml*' -and $o.Text -like "*records\$id\record.jsonl*") 'stop prints the draft and raw log paths'
    Check ($o.Text -like '*3 row(s), 9 step(s), 1 TODO(s), 0 dropped*') 'stop prints the counts'

    # No state file: nothing to stop, unless -RecordingId names one.
    $o = Run @{ Action = 'stop'; Instance = 'p2' }
    Check ($o.Exit -eq 2 -and $o.Body.error -like 'no recording started here for instance p2*' -and $script:calls.Count -eq 0) 'stop without a state file exits 2 and calls nothing'
    $o = Run @{ Action = 'stop'; Instance = 'p2'; RecordingId = $id; NoDraft = $true } @((Reply "{`"ok`":true,`"recording_id`":`"$id`",`"ticks`":4,`"raw`":`"$id`"}"))
    Check ($o.Exit -eq 0 -and $script:calls[0].Args.recording_id -eq $id -and $script:calls[0].Args.no_draft -eq $true) '-RecordingId stops without a state file'
    Check ($o.Text -like 'stopped; 4 tick(s), no draft*') '-NoDraft says no draft'

    # Unrouted start: the answer's instance names the state file.
    $o = Run @{ Action = 'start'; Target = 'first-session' } @((Reply "{`"ok`":true,`"recording_id`":`"x1`",`"instance`":`"default`",`"section`":`"first-session`"}"))
    Check (-not $script:calls[0].Args.ContainsKey('instance') -and (Read-RecordState (Join-Path $labHome 'record-default.json')).recording_id -eq 'x1') 'an unrouted start saves record-default.json'
    $o = Run @{ Action = 'start'; Target = 'first-session' } @((Reply '{"ok":true,"recording_id":"x2","instance":"..\\evil"}'))
    Check ($o.Exit -eq 1 -and -not (Test-Path -LiteralPath (Join-Path $labHome '..\evil'))) 'a bad instance in the answer writes no file'

    # A state file that cannot be written after a good start: exit 1, and the
    # error still names the recording and how to stop it.
    $realSave = ${function:Save-RecordState}
    ${function:Save-RecordState} = { throw 'disk full' }
    try {
        $o = Run @{ Action = 'start'; Target = 'first-session'; Instance = 'p4' } @(
            (Reply "{`"ok`":true,`"recording_id`":`"$id`",`"instance`":`"p4`",`"section`":`"first-session`"}"))
    } finally { ${function:Save-RecordState} = $realSave }
    Check ($o.Exit -eq 1 -and $o.Body.ok -eq $false -and $o.Body.exit -eq 1) 'a state save failure after start exits 1'
    Check ($o.Body.error -like "*$id*" -and $o.Body.error -like "*lab record stop -Instance p4 -RecordingId $id*" -and $o.Body.error -like '*disk full*') 'the save failure names the recording and the stop'

    # A state file without instance (or specs_dir) still stops, under StrictMode.
    $defaultState = Join-Path $labHome 'record-default.json'
    Set-Content -LiteralPath $defaultState -Value "{`"recording_id`":`"$id`"}"
    $o = Run @{ Action = 'stop' } @((Reply "{`"ok`":true,`"draft`":`"fs-20261012-1430.toml`",`"rows`":1,`"steps`":1,`"todo`":0,`"dropped`":0,`"raw`":`"$id`"}"))
    Check ($o.Exit -eq 0 -and $script:calls[0].Args.recording_id -eq $id -and -not $script:calls[0].Args.ContainsKey('instance')) 'a state file with no instance stops unrouted'
    Check ($o.Text -like 'draft <specs>/drafts/fs-20261012-1430.toml*') 'a spec folder unknown here prints <specs>, never a guessed path'

    # A daemon that stops answering mid-stop (exit 3) keeps the state and offers the retry.
    Save-RecordState $state $id 'p2' 'C:\repo\docs\guides\uat-specs'
    $o = Run @{ Action = 'stop'; Instance = 'p2' } @('No connection could be made')
    Check ($o.Exit -eq 3 -and (Test-Path -LiteralPath $state) -and $o.Text -like '*recording is kept; retry with lab record stop -Instance p2') 'exit 3 on stop offers the retry too'
    Remove-Item -LiteralPath $state -Force

    # --- exit-code mapping: the body's exit, straight through ---
    $leased = 'lab_record_start failed: {"ok":false,"exit":4,"error":"the lab is leased to lab_uat (lease-0123456789abcdef). Wait for it"}'
    $o = Run @{ Action = 'start'; Target = 'first-session'; Instance = 'p3' } @($leased)
    Check ($o.Exit -eq 4 -and $o.Body.exit -eq 4) 'a busy answer (exit 4) exits 4, and the body says so'
    Check ($o.Body.error -notmatch 'lease-[0-9a-f]{8,}' -and $o.Body.error -like '*lease-<redacted>*') 'lease ids are hidden'
    $o = Run @{ Action = 'start'; Target = 'first-session'; Instance = 'p3' } @('lab_record_start failed: {"ok":false,"exit":2,"error":"the lab is leased; already running"}')
    Check ($o.Exit -eq 2 -and $o.Body.exit -eq 2) 'an exit-2 body that mentions busy words stays 2 (no text rewrite)'
    $o = Run @{ Action = 'stop'; RecordingId = 'zz' } @('lab_record_stop failed: {"ok":false,"exit":2,"error":"that recording_id is not the running recording on instance default"}')
    Check ($o.Exit -eq 2 -and $o.Body.exit -eq 2) 'a wrong id exits 2'
    $o = Run @{ Action = 'status' } @('lab_record_status failed: the call needs a lease; the lab is leased to someone')
    Check ($o.Exit -eq 1 -and $o.Body.ok -eq $false) 'a plain tool error exits 1, whatever it says'
    $o = Run @{ Action = 'status' } @('lab_record_status failed: invalid params')
    Check ($o.Exit -eq 1) 'any other tool error exits 1'
    $o = Run @{ Action = 'status' } @('No connection could be made because the target machine actively refused it.')
    Check ($o.Exit -eq 3 -and $o.Body.error -like 'the lab daemon did not answer*') 'a transport error exits 3'

    # --- status and transform ---
    $o = Run @{ Action = 'status'; Instance = 'p2' } @((Reply "{`"ok`":true,`"recording`":false,`"recording_id`":`"$id`",`"instance`":`"p2`",`"section`":`"fs`",`"age_s`":75,`"ticks`":150,`"anchors`":2}"))
    Check ($o.Exit -eq 0 -and $o.Text -like "stopped by itself*$id on p2*150 ticks, 2 anchors") 'status shows a recording that stopped by itself'
    $o = Run @{ Action = 'status' } @((Reply '{"ok":true,"recording":false,"instance":"default"}'))
    Check ($o.Text -eq 'no recording on default') 'status with nothing armed'
    $o = Run @{ Action = 'transform'; Target = $id; Section = 'other-section'; SpecsDir = 'D:\specs' } @(
        (Reply "{`"ok`":true,`"draft`":`"other-section-20261012-1430.toml`",`"rows`":1,`"steps`":2,`"todo`":0,`"dropped`":0,`"raw`":`"$id`"}"))
    $sent = $script:calls[0].Args
    Check ($script:calls[0].Name -eq 'lab_record_transform' -and $sent.dir -eq $id -and $sent.section -eq 'other-section' -and $sent.specs_dir -eq 'D:\specs') 'transform sends a bare id, -Section and -SpecsDir'
    Check (-not $sent.ContainsKey('instance')) 'transform is not routed'
    Check ($o.Text -like 'draft D:\specs\drafts\other-section-20261012-1430.toml*') 'transform prints the draft path'
    $rel = Join-Path $labHome 'rel-rec'
    New-Item -ItemType Directory -Force -Path $rel | Out-Null
    Push-Location $labHome
    try { $o = Run @{ Action = 'transform'; Target = 'rel-rec' } @((Reply '{"ok":true,"draft":"d.toml","raw":"rel-rec"}')) } finally { Pop-Location }
    Check ($script:calls[0].Args.dir -eq (Resolve-Path -LiteralPath $rel).Path) 'a relative folder is sent in full'

    # --- compact JSON ---
    $line = ConvertTo-RecordCompactJson ([ordered]@{ ok = $true; draft = 'a.toml'; ended = $null; note = ''; rows = 0 })
    Check ($line -eq '{"ok":true,"draft":"a.toml","rows":0}') 'nulls and empty strings are dropped, zeros kept'
    $long = 'C:\' + (('deep\' * 120)) + 'x'
    $line = ConvertTo-RecordCompactJson (New-RecordFailure 1 $long)
    $back = $line | ConvertFrom-Json
    Check ($line.Length -lt 400 -and $back.ok -eq $false -and $back.exit -eq 1 -and $back.error.StartsWith('C:\deep\')) 'a long failure is cut under 400, shape kept'
    Check ($line -match '^\{"ok":false,"exit":1,"error":"') 'the failure shape is {ok, exit, error} in that order'
    $o = Run @{ Action = 'start'; Target = 'first-session'; Instance = 'p2' } @(
        (Reply "{`"ok`":true,`"recording_id`":`"$id`",`"instance`":`"p2`",`"section`":`"first-session`"}"))
    Check ((ConvertTo-RecordCompactJson $o.Body).Length -lt 400) 'a start answer stays under 400'
    Remove-Item -LiteralPath $state -Force

    # --- the script itself: a usage error under -Json is one line, exit 2 ---
    # (Exit 2 comes before the daemon is asked, so this child never reaches it.)
    $pwshExe = (Get-Process -Id $PID).Path
    $lines = @(& $pwshExe -NoProfile -File (Join-Path $PSScriptRoot 'record.ps1') 'start' 'Bad Section' '-Json')
    $code = $LASTEXITCODE
    Check ($code -eq 2 -and $lines.Count -eq 1 -and ($lines[0] | ConvertFrom-Json).exit -eq 2) 'record.ps1 -Json prints one failure line and exits 2'
    $null = & $pwshExe -NoProfile -File (Join-Path $PSScriptRoot 'record.ps1') 'bogus' 2>$null
    Check ($LASTEXITCODE -eq 2) 'an unknown action exits 2'
    $lines = @(& $pwshExe -NoProfile -File (Join-Path $PSScriptRoot 'record.ps1') 'start' 'first-session' '-MaxMinutes' 'abc' '-Json' 2>$null)
    $code = $LASTEXITCODE
    Check ($code -eq 2 -and $lines.Count -eq 1 -and ($lines[0] | ConvertFrom-Json).error -like '-MaxMinutes must be 1 to 30') '-MaxMinutes abc exits 2 with one JSON line'
} finally {
    Remove-Item -LiteralPath $labHome -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all record checks passed'
