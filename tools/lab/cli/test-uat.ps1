<#
.SYNOPSIS
    Tests for `lab uat` (uat-lib.ps1), no Pester and no daemon. Exits non-zero
    when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-uat.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'calibrate-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$hosted = @('default', 'p2', 'p3', 'p4', 'p5')

# --- planning ---
$p = Get-UatPlan 1 1 $hosted @() 0 ''
Check ($null -eq $p.Error -and $p.Lanes.Count -eq 1 -and $p.Lanes[0].Instance -eq 'default') 'one lease runs on the first instance'
$p = Get-UatPlan 3 20 $hosted @('p2') 0 ''
Check ((@($p.Lanes.Instance) -join ',') -eq 'default,p3,p4') 'lanes skip an instance another session leased'
Check ((Get-UatPlan 0 1 $hosted @() 0 '').Error -like '-Leases must be 1 to 5') '0 leases is refused'
Check ((Get-UatPlan 6 1 $hosted @() 0 '').Error -like '-Leases must be 1 to 5') '6 leases is refused'
Check ((Get-UatPlan 1 21 $hosted @() 0 '').Error -like '-RunsPerLease must be 1 to 20') '21 runs per lease is refused'
Check ((Get-UatPlan 3 1 @('default', 'p2') @() 0 '').Error -like '*more than the lab allows*') 'more leases than hosted instances is refused'
Check ((Get-UatPlan 3 1 $hosted @() 2 '').Error -like '*max clients 2*') 'the client cap limits leases'
Check ((Get-UatPlan 2 1 @('default', 'p2') @('p2') 0 '').Error -like 'only 1 free instance*') 'too few free instances is refused, naming the busy ones'
$p = Get-UatPlan 1 5 $hosted @() 0 'p3'
Check ($p.Lanes.Count -eq 1 -and $p.Lanes[0].Instance -eq 'p3') '-Instance picks that lane'
Check ((Get-UatPlan 2 1 $hosted @() 0 'p3').Error -like '-Instance runs one lane*') '-Instance with 2 leases is refused'
Check ((Get-UatPlan 1 1 $hosted @() 0 'p9').Error -like "unknown instance 'p9'*") 'an unknown instance is refused'
Check ((Get-UatPlan 1 1 $hosted @('p3') 0 'p3').Error -like '*leased by another session*') 'a leased -Instance is refused'

# --- MCP body parsing ---
Check ($null -eq (Read-McpBody '')) 'an empty body (a notification) reads as nothing'
Check ((Read-McpBody '{"id":2,"result":{"x":1}}').result.x -eq 1) 'plain JSON body'
Check ((Read-McpBody "event: message`ndata: {`"id`":2,`"result`":{`"x`":3}}`n`n").result.x -eq 3) 'SSE body: the data line'

# --- verdicts ---
function Row($id, $result, $reason) { [pscustomobject]@{ row = $id; result = $result; reasons = @($reason) } }
$v = Get-UatRunVerdict @((Row 'FS-01' 'PASS' ''), (Row 'FS-P1' 'PASS' ''))
Check ($v.Ok) 'all PASS is ok'
$v = Get-UatRunVerdict @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'FAIL' 'clause movie-over failed: client_entity_find/count gte 1'),
    (Row 'FS-P3' 'FAIL' 'step action 1 failed'))
Check (-not $v.Ok -and $v.Row -eq 'FS-P2' -and $v.Issue -eq '#1341') 'the first failing row decides; FS-P2 movie-over is known #1341'
$v = Get-UatRunVerdict @((Row 'FS-P3' 'FAIL' 'clause aim failed'))
Check (-not $v.Ok -and $null -eq $v.Issue) 'any other failure is new'
$v = Get-UatRunVerdict @((Row 'FS-P1' 'BLOCKED' 'setup failed'))
Check (-not $v.Ok -and $v.Result -eq 'BLOCKED') 'BLOCKED fails a run'
$v = Get-UatRunVerdict @((Row 'FS-01' 'BLOCKED' 'Login_PasswordEdit holds 0 characters after typing, expected 4'))
Check ($v.Issue -eq '#1343') 'the empty login password box is known #1343'
Check ($null -eq (Get-UatKnownIssue 'FS-P3' 'clause aim failed')) 'a row and reason no known issue covers have no issue'

# --- brake ---
Check (-not (Test-UatBrake @($false, $false, $true, $false) 3)) 'a pass resets the failure streak'
Check (Test-UatBrake @($true, $false, $false, $false) 3) 'three failures in a row brake the lane'
Check (-not (Test-UatBrake @($false, $false, $false, $false) 0)) 'a brake of 0 never stops'

# --- aggregation ---
function Rec($inst, $i, [bool]$ok, $rows, $err = $null) {
    @{ Lane = 1; Instance = $inst; Index = $i; Ok = $ok; Rows = $rows; RunDir = "d\$inst-$i"; Error = $err; Seconds = 90
       Verdict = $(if ($err) { @{ Ok = $false; Row = $null; Result = $null; Reason = $err; Issue = $null } } else { Get-UatRunVerdict $rows }) }
}
$good = @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'PASS' ''))
$frost = @((Row 'FS-P1' 'PASS' ''), (Row 'FS-P2' 'FAIL' 'clause movie-over failed'))
$runs = @((Rec 'default' 1 $true $good), (Rec 'default' 2 $false $frost), (Rec 'p2' 1 $false $frost),
          (Rec 'p2' 2 $false @() 'lab_uat_run failed: bridge down'))
$lanes = @(@{ Lane = 1; Instance = 'default' }, @{ Lane = 2; Instance = 'p2' })
$s = Merge-UatResults $runs $lanes @('p2')
Check ($s.runs -eq 4 -and $s.passed -eq 1 -and $s.failed -eq 3) 'totals'
Check ($s.failed_known -eq 2 -and $s.failed_new -eq 1) 'known (#1341) and new failures are counted apart'
Check ($s.rows['FS-P2'].pass -eq 1 -and $s.rows['FS-P2'].total -eq 3) 'per-row pass counts (an errored run has no rows)'
$frostGroup = @($s.failures | Where-Object { $_.issue -eq '#1341' })[0]
Check ($frostGroup.count -eq 2 -and (@($frostGroup.runs) -join ',') -eq 'default#2,p2#1') 'identical failures group with their runs'
Check ((@($s.lanes | Where-Object { $_.instance -eq 'p2' })[0]).braked) 'a braked lane is marked'
Check ((Get-UatExitCode $s) -eq 4) 'a braked lane exits 4'
$s2 = Merge-UatResults @((Rec 'default' 1 $false $frost)) @($lanes[0]) @()
Check ((Get-UatExitCode $s2) -eq 1) 'a failed run exits 1'
$s3 = Merge-UatResults @((Rec 'default' 1 $true $good)) @($lanes[0]) @()
Check ((Get-UatExitCode $s3) -eq 0 -and $s3.ok) 'all passed exits 0'

# --- compact JSON for agents ---
$j = ConvertTo-UatCompactJson $s 'C:\b' 1 | ConvertFrom-Json
Check ($j.rows.'FS-P2' -eq '1/3') 'rows read as pass/total'
Check (@($j.failures).Count -eq 1 -and $j.more_failure_groups -eq 1) 'failure groups are capped, with a count of the rest'
Check ($j.failures[0].issue -eq '#1341' -and $j.failures[0].n -eq 2) 'the biggest group comes first, tagged'
$jsonText = ConvertTo-UatCompactJson $s3 'C:\b'
Check ($jsonText -notmatch 'null' -and $jsonText -notmatch 'failures') 'a clean batch has no nulls and no failure list'
# A realistic clean batch: the 7 Praxis rows, 2 lanes x 5 runs, a real batch path.
$praxis = 'FS-01', 'FS-02', 'FS-P1', 'FS-P2', 'FS-P3', 'FS-P4', 'FS-P5'
$realRuns = foreach ($inst in 'default', 'p2') { foreach ($i in 1..5) { Rec $inst $i $true @($praxis | ForEach-Object { Row $_ 'PASS' '' }) } }
$real = Merge-UatResults $realRuns $lanes @()
$realJson = ConvertTo-UatCompactJson $real 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832'
Check ($realJson.Length -lt 500) "a realistic clean batch stays a few hundred characters ($($realJson.Length))"
$longWhy = Merge-UatResults @((Rec 'default' 1 $false @((Row 'FS-P3' 'FAIL' ('x' * 600))))) @($lanes[0]) @()
Check (((ConvertTo-UatCompactJson $longWhy 'C:\b' | ConvertFrom-Json).failures[0].why).Length -le 200) 'a long failure reason is capped at 200 characters'

# --- review fixes ---
Check ($null -eq (Test-UatArguments 1 1 '')) 'valid arguments pass without a daemon'
Check ((Test-UatArguments 9 1 '') -like '-Leases must be*') 'argument checks need no daemon'
Check ((Test-UatArguments 2 1 'p2') -like '-Instance runs one lane*') '-Instance with leases is a usage error'
Check ((Test-UatArguments 1 1 '' -5) -like '-StaggerSeconds must be*') 'a negative stagger is a usage error'
Check ((Test-UatArguments 1 1 '' 20 0) -like '-RunTimeoutMinutes must be*') 'a zero timeout (no timeout at all) is a usage error'
Check ((Test-UatArguments 1 1 '' 20 15 -1) -like '-MaxConsecutiveFailures must be*') 'a negative brake is a usage error'
Check ($null -eq (Test-UatArguments 1 1 '' 20 15 3 'always')) '-Fixtures always is valid'
Check ($null -eq (Test-UatArguments 1 1 '' 20 15 3 '')) 'no -Fixtures leaves the daemon default'
Check ((Test-UatArguments 1 1 '' 20 15 3 'Always') -like '-Fixtures must be*') '-Fixtures is case-sensitive (the daemon takes snake_case)'
Check ((Test-UatArguments 1 1 '' 20 15 3 'sometimes') -like '-Fixtures must be*') 'an unknown -Fixtures mode is a usage error'
Check (((ConvertTo-UatRowIds @('FS-01, FS-02', 'FS-02', ' FS-P1 ')) -join '|') -eq 'FS-01|FS-02|FS-P1') 'row ids are trimmed and de-duplicated'
Check ((Hide-LeaseIds 'lease lease-0a1b2c3d4e5f is not the current lease') -eq 'lease lease-<redacted> is not the current lease') 'lease ids are redacted'
$v = Get-UatRunVerdict @((Row 'FS-01' 'PASS' '')) @('FS-01', 'FS-02')
Check (-not $v.Ok -and $v.Result -eq 'MISSING' -and $v.Row -eq 'FS-02') 'a run that reports fewer rows than planned fails'
$v = Get-UatRunVerdict @((Row 'FS-P3' 'FAIL' 'lease-0123456789abcdef is not the current lease'))
Check ($v.Reason -notmatch '0123456789abcdef') 'a failing reason has its lease id redacted'
Check (-not (Test-UatStopLane 3 3 @($false, $false, $false) 3)) 'a lane that ran every run did not stop early (no brake on the last run)'
Check (Test-UatStopLane 3 5 @($false, $false, $false) 3) 'a lane with runs left stops on the brake'
$empty = Merge-UatResults $null @($lanes[0]) @()
Check ($empty.runs -eq 0 -and -not $empty.ok -and (Get-UatExitCode $empty) -eq 1) 'no finished runs is not a pass'
Check ((Read-McpBody "event: message`r`ndata: {`"id`":2,`"result`":{`"x`":5}}`r`n`r`n").result.x -eq 5) 'SSE body with CRLF line ends'
Check ((Read-McpBody "data: {`"x`":1}`n`ndata: {`"id`":2,`"result`":{`"x`":6}}`n`n").result.x -eq 6) 'SSE body with several events: the last one'

# The JSONL record round trip: what a Ctrl+C-interrupted batch is summarised from.
$jl = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-test-" + [guid]::NewGuid() + '.jsonl')
try {
    Add-UatRecord $jl ([ordered]@{ Lane = 1; Instance = 'default'; Index = 1; Ok = $false; Rows = $frost; RunDir = 'd'; Error = $null
        Seconds = 90; Braked = $false; Verdict = (Get-UatRunVerdict $frost) })
    Add-UatRecord $jl ([ordered]@{ Lane = 1; Instance = 'default'; Index = 0; Ok = $false; Rows = @(); RunDir = $null; Error = 'lane: boom'
        Seconds = 0; Braked = $true; Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = 'boom'; Issue = $null } })
    $back = @(Read-UatRecords $jl)
    $rt = Merge-UatResults $back @($lanes[0]) @('default')
    Check ($back.Count -eq 2 -and $rt.failed_known -eq 1 -and $rt.failed_new -eq 1) 'records read back from JSONL aggregate like live ones'
    Check (@(Read-UatRecords (Join-Path $env:TEMP 'no-such-batch.jsonl')).Count -eq 0) 'a missing JSONL file reads as no runs'
} finally { Remove-Item -LiteralPath $jl -ErrorAction SilentlyContinue }

# --- batch files: plan.json, record times, the shared progress line ---
$frostRun = [pscustomobject]@{ row = 'FS-P2'; result = 'FAIL'; reasons = @('clause movie-over failed: client_entity_find/count gte 1') }
$frostVerdict = Get-UatRunVerdict @($frostRun)
$line = Format-UatProgressLine ([ordered]@{ Instance = 'default'; Index = 1; Ok = $true; Verdict = $null; Error = $null; Seconds = 114 }) 3
Check ($line -ceq '[default 1/3] PASS (114s)') "the progress line for a PASS run ($line)"
$line = Format-UatProgressLine @{ Instance = 'p2'; Index = 1; Ok = $false; Verdict = $frostVerdict; Error = $null; Seconds = 130 } 3
Check ($line -ceq '[p2 1/3] FAIL FS-P2: clause movie-over failed: client_entity_find/count gte 1 [known #1341] (130s)') "the progress line for a known FAIL ($line)"
$line = Format-UatProgressLine @{ Instance = 'p2'; Index = 2; Ok = $false; Verdict = @{ Ok = $false; Row = $null; Result = $null; Reason = 'bridge down'; Issue = $null }
    Error = 'lab_uat_run failed: bridge down'; Seconds = 3 } 3
Check ($line -ceq '[p2 2/3] ERROR lab_uat_run failed: bridge down (3s)') "the progress line for an ERROR run ($line)"

# --- where progress goes (-ProgressToStderr is lab golden's live progress) ---
Check ((Get-UatProgressMode $false $false $false) -eq 'host') 'progress goes to the console by default'
Check ((Get-UatProgressMode $true $false $false) -eq 'none') '-Json alone silences progress'
Check ((Get-UatProgressMode $false $true $false) -eq 'none') '-Quiet silences progress'
Check ((Get-UatProgressMode $true $false $true) -eq 'stderr') '-ProgressToStderr keeps progress under -Json, on stderr'
Check ((Get-UatProgressMode $true $true $true) -eq 'stderr') '-ProgressToStderr wins over -Quiet'
# Through a real process: stderr carries the line and stdout stays empty, so a
# caller still reads one JSON line from stdout.
$pwshExe = (Get-Process -Id $PID).Path
$lib = Join-Path $PSScriptRoot 'uat-lib.ps1'
$errFile = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-progress-" + [guid]::NewGuid() + '.txt')
try {
    foreach ($m in 'stderr', 'none') {
        $stdout = @(& $pwshExe -NoProfile -Command ". '$lib'; Write-UatProgress '$m' '[default 1/5] PASS (90s)'" 2> $errFile)
        $stderr = (Get-Content -LiteralPath $errFile -Raw) ?? ''
        if ($m -eq 'stderr') { Check ($stdout.Count -eq 0 -and $stderr -match '\[default 1/5\] PASS') 'stderr mode writes the line to stderr, nothing to stdout' }
        else { Check ($stdout.Count -eq 0 -and $stderr -notmatch 'PASS') 'none mode writes nothing' }
    }
} finally { Remove-Item -LiteralPath $errFile -ErrorAction SilentlyContinue }

$planFile = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-plan-" + [guid]::NewGuid() + '.json')
try {
    $planOut = [ordered]@{ schema = 1; section = 'first-session'; rows = @('FS-01', 'FS-P1')
        lanes = @([ordered]@{ lane = 1; instance = 'default' }, [ordered]@{ lane = 2; instance = 'p2' })
        runs_per_lease = 3; started = '2026-10-11T01:39:06.0000000Z'; pid = 41236 }
    Write-UatPlan $planFile $planOut
    $planBack = Read-UatPlan $planFile
    Check ($planBack.section -eq 'first-session' -and @($planBack.lanes).Count -eq 2 -and $planBack.pid -eq 41236) 'plan.json round-trips its section, two lanes and pid'
    Check (@(@($planBack.rows)).Count -eq 2 -and $planBack.lanes[1].instance -eq 'p2') 'plan.json keeps the rows and each lane'
    $head = [System.IO.File]::ReadAllBytes($planFile)[0..2]
    Check (-not ($head[0] -eq 0xEF -and $head[1] -eq 0xBB -and $head[2] -eq 0xBF)) 'plan.json has no byte order mark'
    Check ($null -eq (Read-UatPlan (Join-Path $env:TEMP 'no-such-plan.json'))) 'a missing plan.json reads as nothing'
    [System.IO.File]::WriteAllText($planFile, '{', [System.Text.UTF8Encoding]::new($false))
    Check ($null -eq (Read-UatPlan $planFile)) 'a plan.json that is not JSON reads as nothing'
} finally { Remove-Item -LiteralPath $planFile -ErrorAction SilentlyContinue }

# A record with Started and Ended aggregates exactly as one without them.
$plainRun = Rec 'default' 1 $false $frost
$timedRun = Rec 'default' 1 $false $frost
$timedRun.Started = '2026-10-11T01:39:06.0000000Z'; $timedRun.Ended = '2026-10-11T01:41:00.0000000Z'
$plainSum = Merge-UatResults @($plainRun) @($lanes[0]) @()
$timedSum = Merge-UatResults @($timedRun) @($lanes[0]) @()
Check ((ConvertTo-Json $timedSum -Depth 8) -ceq (ConvertTo-Json $plainSum -Depth 8)) 'Started and Ended do not change the aggregate'

$md = Format-UatSummary $s 'lab uat first-session' 'C:\b'
Check ($md -match 'FAIL \(1/4 runs passed\)' -and $md -match 'known: #1341') 'the Markdown summary names the verdict and the known issue'

# --- -DiffGolden (GD-09, README D-GD7) ---
$divA = 'row FS-P3: event 4 expected dialogDisplay(dialog=3995), got dialogDisplay(dialog=2982); after mission(688).step(2)=active'
$divB = 'row FS-P5: expected windows opened {MissionLog}, got {}'
# The value stored per run, from each lab_golden_diff reply shape.
Check ((Get-UatGoldenValue ([pscustomobject]@{ ok = $true; match = $true }) '') -ceq 'match') 'a matching diff stores match'
Check ((Get-UatGoldenValue ([pscustomobject]@{ ok = $true; match = $false; first = $divA; diverged = 2 }) '') -ceq $divA) 'a divergence stores its first line'
$gv = Get-UatGoldenValue ([pscustomobject]@{ ok = $false; code = 'cannot_diff'; error = 'run C:\r\x: no fingerprint.json' }) ''
Check ($gv -ceq 'error: cannot_diff: run C:\r\x: no fingerprint.json') "a cannot_diff refusal is an error, not a match ($gv)"
Check ((Get-UatGoldenValue $null 'lab_golden_diff failed: connection refused') -ceq 'error: lab_golden_diff failed: connection refused') 'a diff call that threw is an error'
Check ((Get-UatGoldenValue ([pscustomobject]@{ ok = $true }) '') -like 'error: *') 'a reply with no verdict is an error, not a match'
Check ((Get-UatGoldenValue ([pscustomobject]@{ ok = $true; match = $false }) '') -like 'error: *') 'a divergence without a line is an error'
$gv = Get-UatGoldenValue ([pscustomobject]@{ ok = $true; match = $false; first = ('row FS-P3: lease-0123456789abcdef ' + ('y' * 400)) }) ''
Check ($gv.Length -le 300 -and $gv -notmatch '0123456789abcdef') 'a stored line is cut to 300 characters with lease ids hidden'

function GRec($inst, $i, [bool]$ok, $golden) {
    $r = Rec $inst $i $ok $(if ($ok) { $good } else { $frost })
    if ($null -ne $golden) { $r.Golden = $golden }
    $r
}
# golden_match_and_divergence_are_counted
$gRuns = @((GRec 'default' 1 $true 'match'), (GRec 'default' 2 $true $divA), (GRec 'default' 3 $true 'match'),
           (GRec 'default' 4 $true $divA), (GRec 'default' 5 $true $divB))
$gs = Merge-UatResults $gRuns @($lanes[0]) @()
Check ($gs.golden.match -eq 2 -and $gs.golden.diverged -eq 3 -and $gs.golden.errors -eq 0) 'golden_match_and_divergence_are_counted'
Check (@($gs.golden.groups).Count -eq 2 -and $gs.golden.groups[0].line -ceq $divA -and $gs.golden.groups[0].count -eq 2 -and
       (@($gs.golden.groups[0].runs) -join ',') -eq 'default#2,default#4') 'divergence lines group with counts and run refs, most common first'
$many = Merge-UatResults @(1..7 | ForEach-Object { GRec 'default' $_ $true $divB }) @($lanes[0]) @()
Check (@($many.golden.groups[0].runs).Count -eq 5 -and $many.golden.groups[0].count -eq 7) 'a group keeps its full count but at most 5 run refs'
# a_divergence_on_passing_runs_exits_5: verdicts and ok stay as they are.
Check ($gs.ok -and $gs.passed -eq 5 -and $gs.failed -eq 0) 'a divergence leaves every run passed and the batch ok'
Check ((Get-UatExitCode $gs) -eq 5) 'a_divergence_on_passing_runs_exits_5'
$allMatch = Merge-UatResults @(1..5 | ForEach-Object { GRec 'default' $_ $true 'match' }) @($lanes[0]) @()
Check ((Get-UatExitCode $allMatch) -eq 0 -and $allMatch.golden.match -eq 5) 'every run matching the golden exits 0'
$withErr = Merge-UatResults @((GRec 'default' 1 $true 'match'), (GRec 'default' 2 $true 'error: cannot_diff: run d: no fingerprint')) @($lanes[0]) @()
Check ($withErr.golden.errors -eq 1 -and $withErr.golden.match -eq 1 -and (Get-UatExitCode $withErr) -eq 5) 'a run the diff could not compare is counted as an error and is not a match (exit 5)'
$failAndDiv = Merge-UatResults @((GRec 'default' 1 $false $divA), (GRec 'default' 2 $true $divB)) @($lanes[0]) @()
Check ((Get-UatExitCode $failAndDiv) -eq 1 -and $failAndDiv.failed -eq 1) 'a failed run exits 1 even when another run diverged'
$brakeAndDiv = Merge-UatResults @((GRec 'default' 1 $false $divA)) @($lanes[0]) @('default')
Check ((Get-UatExitCode $brakeAndDiv) -eq 4) 'a braked lane exits 4 even with a divergence'
# Exit 8 (lab-spec-vocab SV-12's heal proposal) wins over 5 and every other code.
$healDiv = Merge-UatResults $gRuns @($lanes[0]) @(); $healDiv.heal = @([ordered]@{ row = 'FS-P3'; pose = 'aim'; diff = 'C:\b\heal-FS-P3-aim.diff' })
Check ((Get-UatExitCode $healDiv) -eq 8) 'a heal proposal (8) wins over a golden divergence (5)'
$healFail = Merge-UatResults @((GRec 'default' 1 $false $null)) @($lanes[0]) @(); $healFail.heal = @([ordered]@{ row = 'FS-P3'; pose = 'aim'; diff = 'd' })
Check ((Get-UatExitCode $healFail) -eq 8) 'a heal proposal (8) wins over a failed run (1)'
$healNone = Merge-UatResults $gRuns @($lanes[0]) @(); $healNone.heal = @()
Check ((Get-UatExitCode $healNone) -eq 5) 'an empty heal list is no proposal'
# compact_json_with_golden_stays_under_500_chars: 5 runs, two distinct divergences.
$fiveRuns = foreach ($i in 1..5) {
    $r = Rec 'default' $i $true @($praxis | ForEach-Object { Row $_ 'PASS' '' })
    $r.Golden = @('match', $divA, 'match', $divB, 'match')[$i - 1]; $r
}
$fiveJson = ConvertTo-UatCompactJson (Merge-UatResults $fiveRuns @($lanes[0]) @()) 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832'
$fiveObj = $fiveJson | ConvertFrom-Json
Check ($fiveJson.Length -lt 500 -and $fiveJson -notmatch 'null') "compact_json_with_golden_stays_under_500_chars ($($fiveJson.Length))"
Check ($fiveObj.golden.match -eq 3 -and $fiveObj.golden.diverged -eq 2 -and $fiveObj.golden.first -ceq (Limit-UatText $divA 120) -and
       -not ($fiveObj.golden.PSObject.Properties.Name -contains 'errors')) 'compact golden is {match, diverged, first}, errors only when there are some'
$longDiv = Merge-UatResults @((GRec 'default' 1 $true ('row FS-P3: ' + ('z' * 290)))) @($lanes[0]) @()
Check (((ConvertTo-UatCompactJson $longDiv 'C:\b' | ConvertFrom-Json).golden.first).Length -le 120) 'the compact first line is capped at 120 characters'
# compact_first_is_a_divergence_not_an_error: 3 errors outnumber 2 identical divergences.
$cannot = 'error: cannot_diff: run d: no fingerprint.json'
$mixed = Merge-UatResults @((GRec 'default' 1 $true $cannot), (GRec 'default' 2 $true $cannot), (GRec 'default' 3 $true $cannot),
                            (GRec 'default' 4 $true $divA), (GRec 'default' 5 $true $divA)) @($lanes[0]) @()
$mixedObj = ConvertTo-UatCompactJson $mixed 'C:\b' | ConvertFrom-Json
Check ($mixed.golden.groups[0].line -ceq $cannot -and $mixedObj.golden.first -ceq (Limit-UatText $divA 120) -and $mixedObj.golden.errors -eq 3 -and
       -not ($mixedObj.golden.PSObject.Properties.Name -contains 'error')) 'compact_first_is_a_divergence_not_an_error'
$onlyErr = ConvertTo-UatCompactJson (Merge-UatResults @((GRec 'default' 1 $true 'match'), (GRec 'default' 2 $true $cannot)) @($lanes[0]) @()) 'C:\b' | ConvertFrom-Json
Check ($onlyErr.golden.error -ceq $cannot -and -not ($onlyErr.golden.PSObject.Properties.Name -contains 'first')) 'with no divergence the compact golden names the first error, not a first line'
$longErr = ConvertTo-UatCompactJson (Merge-UatResults @((GRec 'default' 1 $true ('error: ' + ('e' * 300)))) @($lanes[0]) @()) 'C:\b' | ConvertFrom-Json
Check ($longErr.golden.error.Length -le 120) 'the compact error line is capped at 120 characters'
# compact_json_with_200_char_divergences_stays_under_500_chars
$long200 = 'row FS-P3: ' + ('w' * 189)
$long200b = 'row FS-P5: ' + ('v' * 189)
$longRuns = foreach ($i in 1..5) {
    $r = Rec 'default' $i $true @($praxis | ForEach-Object { Row $_ 'PASS' '' })
    $r.Golden = @('match', $long200, 'match', $long200b, 'match')[$i - 1]; $r
}
$longJson = ConvertTo-UatCompactJson (Merge-UatResults $longRuns @($lanes[0]) @()) 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832'
Check ($long200.Length -eq 200 -and $longJson.Length -lt 500) "compact_json_with_200_char_divergences_stays_under_500_chars ($($longJson.Length))"

# --- the per-run diff in the lane loop (Get-UatRunGolden) ---
$diffArgs = @{ section = 'first-session'; specs_dir = 'C:\specs' }
$script:seen = $null
$replyMatch = { param($a) $script:seen = $a; [pscustomobject]@{ ok = $true; match = $true } }
$passRec = [ordered]@{ Instance = 'default'; Index = 1; Ok = $true; RunDir = 'C:\runs\r1' }
Check ((Get-UatRunGolden $passRec $diffArgs $replyMatch) -ceq 'match') 'run_golden: a passing run that matches is match'
Check ($script:seen.run_dir -ceq 'C:\runs\r1' -and $script:seen.section -ceq 'first-session' -and -not $diffArgs.Contains('run_dir')) 'run_golden: the call gets the run folder; the shared arguments are not changed'
$replyDiv = { param($a) [pscustomobject]@{ ok = $true; match = $false; first = $divA } }
Check ((Get-UatRunGolden $passRec $diffArgs $replyDiv) -ceq $divA) 'run_golden: a divergence is its first line'
$replyThrow = { param($a) throw "lab_golden_diff failed: connection refused`nmore" }
Check ((Get-UatRunGolden $passRec $diffArgs $replyThrow) -ceq 'error: lab_golden_diff failed: connection refused') 'run_golden: a call that throws is an error'
$script:calls = 0
$counting = { param($a) $script:calls++; [pscustomobject]@{ ok = $true; match = $true } }
$noDir = [ordered]@{ Instance = 'default'; Index = 2; Ok = $true; RunDir = $null }
Check ((Get-UatRunGolden $noDir $diffArgs $counting) -ceq 'error: the run named no run folder' -and $script:calls -eq 0) 'run_golden: a passing run with no folder is an error, with no call'
$failRec = [ordered]@{ Instance = 'default'; Index = 3; Ok = $false; RunDir = 'C:\runs\r3' }
Check ($null -eq (Get-UatRunGolden $failRec $diffArgs $counting) -and $script:calls -eq 0) 'run_golden: a failed run is not compared and gets no golden'
Check ($null -eq (Get-UatRunGolden $passRec $null $counting) -and $script:calls -eq 0) 'run_golden: without -DiffGolden there is no golden and no call'
# As the lane loop calls it: in a ForEach-Object -Parallel runspace that
# dot-sources the lib, with a call block that reads the lane's own variables.
$libPath = Join-Path $PSScriptRoot 'uat-lib.ps1'
$par = @(1 | ForEach-Object -Parallel {
    . $using:libPath
    $urlL = 'http://lane'
    $rec = [ordered]@{ Ok = $true; RunDir = 'C:\runs\p1' }
    Get-UatRunGolden $rec ($using:diffArgs) { param($ga) [pscustomobject]@{ ok = $true; match = $false; first = "row X: $urlL $($ga.run_dir)" } }
})
Check ($par.Count -eq 1 -and $par[0] -ceq 'row X: http://lane C:\runs\p1') "run_golden: works in a parallel lane runspace ($($par -join '|'))"

# --- the golden file checked up front (Test-UatGoldenFile) ---
$gDir = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-golden-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $gDir | Out-Null
try {
    $gf = Join-Path $gDir 'g.json'
    Set-Content -LiteralPath $gf -Value '{"schema":1,"section":"s","fingerprint":{"rows":[]}}'
    Check ($null -eq (Test-UatGoldenFile $gf)) 'golden_file: schema, section and fingerprint read fine'
    Set-Content -LiteralPath $gf -Value '{"schema":1,"section":"s"}'
    Check ((Test-UatGoldenFile $gf) -ceq 'it has no fingerprint') 'golden_file: a missing fingerprint is named'
    Set-Content -LiteralPath $gf -Value '{ not json'
    Check ([bool](Test-UatGoldenFile $gf)) 'golden_file: a file that is not JSON cannot be read'
    Set-Content -LiteralPath $gf -Value '[1,2]'
    Check ([bool](Test-UatGoldenFile $gf)) 'golden_file: a JSON array cannot be read'
} finally { Remove-Item -LiteralPath $gDir -Recurse -ErrorAction SilentlyContinue }
$matchJson = ConvertTo-UatCompactJson $allMatch 'C:\b' | ConvertFrom-Json
Check ($matchJson.golden.match -eq 5 -and -not ($matchJson.golden.PSObject.Properties.Name -contains 'first')) 'an all-match batch has no first line'
# A batch without Golden fields is unchanged.
Check (-not $s.Contains('golden') -and -not $s3.Contains('golden')) 'a batch without Golden fields has no golden summary'
Check ((ConvertTo-UatCompactJson $s3 'C:\b') -notmatch 'golden' -and (Format-UatSummary $s3 't' 'C:\b') -notmatch 'Golden') 'a batch without Golden fields prints no golden'
$md = Format-UatSummary $gs 'lab uat first-session' 'C:\b'
Check ($md -match 'Golden: 2 match, 3 diverged' -and $md.Contains("- 2x $divA; runs: default#2, default#4") -and $md.Contains("- 1x $divB")) 'the Markdown summary has a Golden block with each line and its count'
Check ((Format-UatSummary $withErr 't' 'C:\b') -match 'Golden: 1 match, 0 diverged, 1 could not be compared') 'the Golden block counts runs that could not be compared'
$line = Format-UatProgressLine @{ Instance = 'default'; Index = 2; Ok = $true; Verdict = $null; Error = $null; Seconds = 114; Golden = $divB } 5
Check ($line -ceq "[default 2/5] PASS (114s); golden: $divB") "the progress line ends with the golden value ($line)"
$jl = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-test-" + [guid]::NewGuid() + '.jsonl')
try {
    foreach ($r in $gRuns) { Add-UatRecord $jl $r }
    $back = Merge-UatResults @(Read-UatRecords $jl) @($lanes[0]) @()
    Check ($back.golden.match -eq 2 -and $back.golden.diverged -eq 3 -and (Get-UatExitCode $back) -eq 5) 'Golden fields read back from JSONL aggregate like live ones'
} finally { Remove-Item -LiteralPath $jl -ErrorAction SilentlyContinue }

# Through a real process: -DiffGolden without a golden stops with exit 2
# before the daemon is asked (an empty spec folder; no daemon is touched).
$emptySpecs = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-specs-" + [guid]::NewGuid())
New-Item -ItemType Directory -Path $emptySpecs | Out-Null
try {
    $uat = Join-Path $PSScriptRoot 'uat.ps1'
    $out = @(& $pwshExe -NoProfile -File $uat 'no-such-section' -DiffGolden -SpecsDir $emptySpecs -Json 2>$null)
    $code = $LASTEXITCODE
    $obj = ($out | Select-Object -Last 1) | ConvertFrom-Json
    Check ($code -eq 2 -and -not $obj.ok -and $obj.exit -eq 2 -and $obj.error -ceq 'no golden for no-such-section; record one with lab golden record') "-DiffGolden without a golden exits 2 up front ($code)"
    # A golden that exists but cannot be read stops the same way, before the daemon is asked.
    New-Item -ItemType Directory -Path (Join-Path $emptySpecs 'golden') | Out-Null
    Set-Content -LiteralPath (Join-Path $emptySpecs 'golden\broken-section.json') -Value '{"schema":1,"section":"broken-section"}'
    $out = @(& $pwshExe -NoProfile -File $uat 'broken-section' -DiffGolden -SpecsDir $emptySpecs -Json 2>$null)
    $code = $LASTEXITCODE
    $obj = ($out | Select-Object -Last 1) | ConvertFrom-Json
    Check ($code -eq 2 -and $obj.exit -eq 2 -and $obj.error -ceq 'golden for broken-section cannot be read: it has no fingerprint') "-DiffGolden with an unreadable golden exits 2 up front ($code)"
} finally { Remove-Item -LiteralPath $emptySpecs -Recurse -ErrorAction SilentlyContinue }

# --- -Heal (lab-spec-vocab SV-12, D-SV8 (a)) ---
Check ((Test-UatArguments 2 1 '' 20 15 3 '' $true) -like '-Heal needs -Leases 1*') '-Heal -Leases 2 is refused'
Check ($null -eq (Test-UatArguments 1 5 '' 20 15 3 '' $true)) '-Heal with one lease is allowed'
Check ($null -eq (Test-UatArguments 2 1 '' 20 15 3 '')) 'two leases without -Heal are allowed'

function HealRow($id, $result, $pose) {
    $r = [pscustomobject]@{ row = $id; result = $result; reasons = @('step action 2 failed') }
    if ($pose) { $r | Add-Member -NotePropertyName heal -NotePropertyValue ([pscustomobject]@{ pose = $pose; step = 'hover' }) }
    $r
}
$healRuns = @(
    (Rec 'default' 1 $false @((HealRow 'FS-P1' 'PASS' $null), (HealRow 'FS-P3' 'FAIL' 'frost'), (HealRow 'FS-P4' 'FAIL' $null)))
    (Rec 'default' 2 $false @((HealRow 'FS-P3' 'FAIL' 'frost'), (HealRow 'FS-P5' 'BLOCKED' 'supply')))
    # A PASS row never carries a hint; one that somehow does is ignored.
    (Rec 'default' 3 $true @((HealRow 'FS-P3' 'PASS' 'guard')))
    (Rec 'default' 4 $false @() 'lab_uat_run failed: bridge down')
)
$hints = @(Get-UatHealHints $healRuns)
Check ($hints.Count -eq 2 -and $hints[0].row -ceq 'FS-P3' -and $hints[0].pose -ceq 'frost' -and $hints[1].row -ceq 'FS-P5' -and $hints[1].pose -ceq 'supply') 'heal hints: two runs'' identical hint once, rows without one and PASS rows ignored'
Check (@(Get-UatHealHints @((Rec 'default' 1 $false @((HealRow 'FS-P3' 'FAIL' $null))))).Count -eq 0) 'a failed batch without hints has none'
$jl = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-heal-" + [guid]::NewGuid() + '.jsonl')
try {
    foreach ($r in $healRuns) { Add-UatRecord $jl $r }
    $back = @(Get-UatHealHints @(Read-UatRecords $jl))
    Check ($back.Count -eq 2 -and $back[0].pose -ceq 'frost') 'heal hints read back from batch.jsonl like live ones'
} finally { Remove-Item -LiteralPath $jl -ErrorAction SilentlyContinue }
Check ((Get-UatHealFileName 'FS-P3' 'frost') -ceq 'heal-FS-P3-frost.diff' -and (Get-UatHealFileName 'a/b' 'c d') -ceq 'heal-a_b-c_d.diff') 'heal file names are safe'

# The heal pass with a stubbed lab_uat_calibrate: one call per hint on the
# batch's instance, the proposal copied into the batch folder, no file under
# the spec folder written.
$healTmp = Join-Path ([System.IO.Path]::GetTempPath()) ("uat-heal-" + [guid]::NewGuid())
$healBatch = Join-Path $healTmp 'batch'; $runDir = Join-Path $healTmp 'run'; $healSpecs = Join-Path $healTmp 'specs'
New-Item -ItemType Directory -Path $healBatch, $runDir, $healSpecs, (Join-Path $healSpecs 'golden') | Out-Null
# Every file under the spec folder with its hash: a write, a new file or a
# deleted one changes it.
function Get-SpecTreeHashes([string]$Dir) {
    (@(Get-ChildItem -LiteralPath $Dir -Recurse -File | Sort-Object FullName |
        ForEach-Object { "$($_.FullName)=$((Get-FileHash -LiteralPath $_.FullName).Hash)" })) -join "`n"
}
try {
    $specFile = Join-Path $healSpecs 'first-session.toml'
    Set-Content -LiteralPath $specFile -Value 'pose frost = { tag = "frost", radius_m = 3.75, bearing_deg = 322.5 }'
    Set-Content -LiteralPath (Join-Path $healSpecs 'abilities.toml') -Value '[section]'
    Set-Content -LiteralPath (Join-Path $healSpecs 'golden\first-session.json') -Value '{"schema":1}'
    $specsBefore = Get-SpecTreeHashes $healSpecs
    $diffText = "--- a/docs/guides/uat-specs/first-session.toml`n+++ b/docs/guides/uat-specs/first-session.toml`n@@ -1 +1 @@`n-pose frost = { bearing_deg = 322.5 }`n+pose frost = { bearing_deg = 330.0 }`n"
    $script:calls = @()
    $stub = {
        param($a)
        $script:calls += , $a
        if ($a.pose -ceq 'frost') {
            $d = Join-Path $runDir 'first-session.diff'
            [IO.File]::WriteAllText($d, $diffText)
            return [pscustomobject]@{ run_dir = $runDir; outcomes = @([pscustomobject]@{ ok = $true; section = $a.section; row = $a.row; pose = 'frost'
                score = 8312; confirmed = 3; changed = [pscustomobject]@{ bearing_deg = @(322.5, 330.0) }; diff_path = $d; probes = 41 }) }
        }
        return [pscustomobject]@{ run_dir = $runDir; outcomes = @([pscustomobject]@{ ok = $false; section = $a.section; row = $a.row; pose = $a.pose
            confirmed = 0; probes = 108; reason = 'no candidate hovered the target' }) }
    }
    $healed = @(Invoke-UatHeal $hints $healBatch 'first-session' $healSpecs 'p2' $stub)
    Check ($script:calls.Count -eq 2) 'one lab_uat_calibrate call per hint'
    $c = $script:calls[0]
    Check ($c.section -ceq 'first-session' -and $c.row -ceq 'FS-P3' -and $c.pose -ceq 'frost' -and $c.instance -ceq 'p2' -and $c.specs_dir -ceq $healSpecs) 'the call names the section, row, pose, instance and spec folder'
    $copy = Join-Path $healBatch 'heal-FS-P3-frost.diff'
    Check ($healed[0].Diff -ceq $copy -and (Test-Path -LiteralPath $copy) -and [IO.File]::ReadAllText($copy) -ceq $diffText) 'the proposal is saved as heal-FS-P3-frost.diff in the batch folder'
    Check ($healed[0].Line -ceq "heal FS-P3 frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff $copy" -and $healed[0].Text -ceq $diffText) "the heal line names the batch copy, and the diff text is kept to print ($($healed[0].Line))"
    Check ($null -eq $healed[1].Diff -and $healed[1].Line -ceq 'heal FS-P5 supply: no proposal: no candidate hovered the target' -and -not (Test-Path -LiteralPath (Join-Path $healBatch 'heal-FS-P5-supply.diff'))) 'a pose with no proposal writes no file'
    Check ((Get-SpecTreeHashes $healSpecs) -ceq $specsBefore) 'no file under the spec folder is written, added or removed'

    # The summary with the proposal: exit 8, over a failed batch's 1 and a golden 5.
    $healSummary = Merge-UatResults $healRuns @($lanes[0]) @()
    Check ((Get-UatExitCode $healSummary) -eq 1) 'without a proposal the failed batch exits 1'
    $healSummary.heal = @($healed | Where-Object { $_.Diff } | ForEach-Object { [ordered]@{ row = $_.Row; pose = $_.Pose; diff = $_.Diff } })
    Check ((Get-UatExitCode $healSummary) -eq 8) 'a heal proposal exits 8'
    $hj = ConvertTo-UatCompactJson $healSummary $healBatch | ConvertFrom-Json
    Check (@($hj.heal).Count -eq 1 -and $hj.heal[0].row -ceq 'FS-P3' -and $hj.heal[0].pose -ceq 'frost' -and $hj.heal[0].diff -ceq 'heal-FS-P3-frost.diff' -and $hj.batch -ceq $healBatch) '-Json lists heal as {row, pose, diff}, diff named in the batch folder'
    Check (-not ($hj.PSObject.Properties.Name -contains 'more_heal')) 'one proposal has no more_heal'
    $hmd = Format-UatSummary $healSummary 'lab uat first-session' $healBatch
    Check ($hmd.Contains("- FS-P3 frost: $copy") -and $hmd -match 'nothing was applied') 'summary.md lists the heal proposal'
    $both = Merge-UatResults @((GRec 'default' 1 $true $divA)) @($lanes[0]) @()
    Check ((Get-UatExitCode $both) -eq 5) 'a golden divergence alone exits 5'
    $both.heal = @([ordered]@{ row = 'FS-P3'; pose = 'frost'; diff = $copy })
    Check ((Get-UatExitCode $both) -eq 8) '8 wins over 5 when both apply'

    # A call that throws, and a reply without the pose, propose nothing.
    $bad = @(Invoke-UatHeal @([ordered]@{ row = 'FS-P3'; pose = 'frost' }) $healBatch 'first-session' '' 'default' { param($a) throw 'lab_uat_calibrate failed: lease-0123456789abcdef held' })
    Check ($null -eq $bad[0].Diff -and $bad[0].Line -ceq 'heal FS-P3 frost: lab_uat_calibrate failed: lease-<redacted> held') "a failed call proposes nothing ($($bad[0].Line))"
    $none = @(Invoke-UatHeal @([ordered]@{ row = 'FS-P3'; pose = 'frost' }) $healBatch 'first-session' '' 'default' { param($a) [pscustomobject]@{ run_dir = 'x'; outcomes = @() } })
    Check ($null -eq $none[0].Diff -and $none[0].Line -like '*no outcome for the pose') 'a reply without the pose proposes nothing'

    # After a failed call: wait for the lease before the next hint (not
    # after the last one), and stop healing, naming the rest, when it stays held.
    $two = @([ordered]@{ row = 'FS-P3'; pose = 'frost' }, [ordered]@{ row = 'FS-P5'; pose = 'supply' })
    $throwing = { param($a) $script:calls += , $a; throw 'lab_uat_calibrate timed out' }
    $script:calls = @(); $script:waits = @()
    $w = @(Invoke-UatHeal $two $healBatch 'first-session' '' 'p2' $throwing -WaitLeaseFree { param($i) $script:waits += $i; $true })
    Check ($script:calls.Count -eq 2 -and $w.Count -eq 2 -and @($script:waits).Count -eq 1 -and $script:waits[0] -ceq 'p2') 'a failed call waits for the lease once before the next hint, not after the last'
    $script:calls = @(); $script:waits = @()
    $w = @(Invoke-UatHeal $two $healBatch 'first-session' '' 'p2' $throwing -WaitLeaseFree { param($i) $script:waits += $i; $false })
    Check ($script:calls.Count -eq 1 -and $w.Count -eq 2 -and $null -eq $w[1].Row -and $w[1].Line -ceq 'heal stopped: the lease on p2 did not clear after the failed call; not recalibrated: FS-P5 supply') "a lease that stays held stops healing ($($w[1].Line))"
    $script:calls = @()
    $w = @(Invoke-UatHeal $two $healBatch 'first-session' '' 'p2' $throwing -WaitLeaseFree { param($i) throw 'status failed' })
    Check ($script:calls.Count -eq 1 -and $w[1].Line -like 'heal stopped:*') 'a wait that throws counts as a held lease'
    $script:calls = @(); $script:waits = @()
    $w = @(Invoke-UatHeal $two $healBatch 'first-session' '' 'p2' $stub -WaitLeaseFree { param($i) $script:waits += $i; $true })
    Check ($script:calls.Count -eq 2 -and @($script:waits).Count -eq 0) 'calls that return need no wait'

    # Invoke-UatHealPass, the wiring uat.ps1 runs after the batch: the hints
    # lead to calls, summary.md and batch.json are rewritten with heal, and
    # the batch exits 8.
    $passBatch = Join-Path $healTmp 'pass-batch'
    New-Item -ItemType Directory -Path $passBatch | Out-Null
    $passSummary = Merge-UatResults $healRuns @($lanes[0]) @()
    $batchCtx = @{ Dir = $passBatch; Title = 'lab uat first-session'; Section = 'first-session'; RowIds = @('FS-P1', 'FS-P3', 'FS-P4', 'FS-P5')
                   LaneCount = 1; RunsPerLease = 3; Started = (Get-Date) }
    $md0 = Write-UatBatchSummary $passSummary $batchCtx
    Check (-not $md0.Contains('Heal proposals') -and (Get-UatExitCode $passSummary) -eq 1) 'before the heal: no heal in the summary, the failed batch exits 1'
    $script:calls = @(); $script:said = @(); $script:waits = @()
    $passMd = Invoke-UatHealPass $healRuns $passSummary $batchCtx $healSpecs 'default' $stub { param($m) $script:said += $m } { param($i) $script:waits += $i; $true }
    Check ($script:calls.Count -eq 2 -and $script:calls[0].pose -ceq 'frost' -and $script:calls[1].pose -ceq 'supply' -and $script:calls[0].instance -ceq 'default' -and $script:calls[0].specs_dir -ceq $healSpecs) 'the pass calls lab_uat_calibrate once per hint, on the batch instance'
    Check (@($script:said).Count -eq 3 -and $script:said[0] -ceq 'heal: recalibrating 2 pose(s) on default: FS-P3 frost, FS-P5 supply' -and $script:said[1] -like 'heal FS-P3 frost: score 8312*' -and $script:said[2] -like 'heal FS-P5 supply: no proposal*') 'the pass says what it heals, then one line per hint'
    $passCopy = Join-Path $passBatch 'heal-FS-P3-frost.diff'
    $smd = Get-Content -LiteralPath (Join-Path $passBatch 'summary.md') -Raw
    $bj = Get-Content -LiteralPath (Join-Path $passBatch 'batch.json') -Raw | ConvertFrom-Json
    Check ((Test-Path -LiteralPath $passCopy) -and $smd.Contains("- FS-P3 frost: $passCopy") -and $passMd.Contains("- FS-P3 frost: $passCopy")) 'summary.md is rewritten with the proposal, and the pass returns it'
    Check (@($bj.summary.heal).Count -eq 1 -and $bj.summary.heal[0].row -ceq 'FS-P3' -and $bj.summary.heal[0].pose -ceq 'frost' -and $bj.summary.heal[0].diff -ceq $passCopy -and $bj.section -ceq 'first-session' -and $bj.runs_per_lease -eq 3) 'batch.json is rewritten with heal'
    Check ((Get-UatExitCode $passSummary) -eq 8) 'the healed batch exits 8'
    Check ((Get-SpecTreeHashes $healSpecs) -ceq $specsBefore) 'the pass writes nothing under the spec folder'

    # No hints: nothing called, the files stay as the batch wrote them.
    $plainBatch = Join-Path $healTmp 'plain-batch'
    New-Item -ItemType Directory -Path $plainBatch | Out-Null
    $plainSummary = Merge-UatResults @((Rec 'default' 1 $false $frost)) @($lanes[0]) @()
    $plainCtx = $batchCtx.Clone(); $plainCtx.Dir = $plainBatch
    Write-UatBatchSummary $plainSummary $plainCtx | Out-Null
    $plainBefore = Get-SpecTreeHashes $plainBatch
    $script:calls = @(); $script:said = @()
    $r = Invoke-UatHealPass @((Rec 'default' 1 $false $frost)) $plainSummary $plainCtx $healSpecs 'default' $stub { param($m) $script:said += $m }
    Check ($null -eq $r -and $script:calls.Count -eq 0 -and $script:said[0] -like 'heal: no failed row carries a heal hint*' -and (Get-SpecTreeHashes $plainBatch) -ceq $plainBefore -and (Get-UatExitCode $plainSummary) -eq 1) 'no hints: no call, the summary files untouched, the batch code kept'

    # Ctrl+C mid-heal (the stub stops the pipeline on the second hint, as
    # Ctrl+C does): in a child process, since a pipeline stop ends the whole
    # script. The diff of the first hint is still in batch.json and summary.md.
    $intBatch = Join-Path $healTmp 'int-batch'
    New-Item -ItemType Directory -Path $intBatch | Out-Null
    $child = Join-Path $healTmp 'interrupt.ps1'
    Set-Content -LiteralPath $child -Encoding utf8 -Value @'
param($Lib, $CalLib, $Batch, $RunDir)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. $Lib
. $CalLib
$hint = { param($row, $pose) [pscustomobject]@{ row = $row; result = 'FAIL'; reasons = @('hover missed'); heal = [pscustomobject]@{ pose = $pose; step = 'hover' } } }
$runs = @(@{ Lane = 1; Instance = 'default'; Index = 1; Ok = $false; Rows = @((& $hint 'FS-P3' 'frost'), (& $hint 'FS-P4' 'guard')); RunDir = 'd'
    Error = $null; Seconds = 1; Verdict = @{ Ok = $false; Row = 'FS-P3'; Result = 'FAIL'; Reason = 'hover missed'; Issue = $null } })
$summary = Merge-UatResults $runs @(@{ Lane = 1; Instance = 'default' }) @()
$batch = @{ Dir = $Batch; Title = 'lab uat first-session'; Section = 'first-session'; RowIds = @('FS-P3', 'FS-P4'); LaneCount = 1; RunsPerLease = 1; Started = (Get-Date) }
Write-UatBatchSummary $summary $batch | Out-Null
$script:n = 0
Invoke-UatHealPass $runs $summary $batch '' 'default' {
    param($a)
    $script:n++
    if ($script:n -gt 1) { throw [System.Management.Automation.PipelineStoppedException]::new() }
    $d = Join-Path $RunDir 'first-session.diff'
    [IO.File]::WriteAllText($d, "--- a/x`n+++ b/x`n")
    [pscustomobject]@{ run_dir = $RunDir; outcomes = @([pscustomobject]@{ ok = $true; pose = $a.pose; score = 9000; confirmed = 3; diff_path = $d }) }
} { param($m) } | Out-Null
Set-Content -LiteralPath (Join-Path $Batch 'not-interrupted') -Value 'x'
'@
    & $pwshExe -NoProfile -File $child (Join-Path $PSScriptRoot 'uat-lib.ps1') (Join-Path $PSScriptRoot 'calibrate-lib.ps1') $intBatch $runDir 2>&1 | Out-Null
    $ibj = Get-Content -LiteralPath (Join-Path $intBatch 'batch.json') -Raw | ConvertFrom-Json
    $imd = Get-Content -LiteralPath (Join-Path $intBatch 'summary.md') -Raw
    $intCopy = Join-Path $intBatch 'heal-FS-P3-frost.diff'
    Check (-not (Test-Path -LiteralPath (Join-Path $intBatch 'not-interrupted'))) 'the stub stopped the child mid-heal'
    Check (@(Get-UatField $ibj.summary 'heal' | Where-Object { $null -ne $_ }).Count -eq 1 -and $ibj.summary.heal[0].pose -ceq 'frost' -and $ibj.summary.heal[0].diff -ceq $intCopy -and $imd.Contains("- FS-P3 frost: $intCopy") -and -not (Test-Path -LiteralPath (Join-Path $intBatch 'heal-FS-P4-guard.diff'))) 'an interrupted heal still records the diff already written in batch.json and summary.md'
} finally { Remove-Item -LiteralPath $healTmp -Recurse -Force -ErrorAction SilentlyContinue }

# -Json bounds heal: at most 3 entries, more_heal for the rest, diff the
# file name in the batch folder, within the -Json budget.
$manyRuns = foreach ($i in 1..5) {
    Rec 'default' $i $false @(($praxis | Select-Object -First 4 | ForEach-Object { Row $_ 'PASS' '' }) + @(Row 'FS-P3' 'FAIL' 'clause hover failed: frost not under the cursor'))
}
$many = Merge-UatResults $manyRuns @($lanes[0]) @()
$manyBatch = 'C:\Users\someone\AppData\Local\cimmeria-lab\uat-runs\batch-20261010-183832'
$many.heal = @(foreach ($p in 'frost', 'supply', 'guard', 'crate', 'console') {
    [ordered]@{ row = 'FS-P3'; pose = $p; diff = (Join-Path $manyBatch (Get-UatHealFileName 'FS-P3' $p)) }
})
$manyJson = ConvertTo-UatCompactJson $many $manyBatch
$mj = $manyJson | ConvertFrom-Json
Check (@($mj.heal).Count -eq 3 -and $mj.more_heal -eq 2 -and $mj.heal[2].diff -ceq 'heal-FS-P3-guard.diff') 'five proposals: three listed by file name, more_heal 2'
# The batch without heal is the 500-character budget's (above); heal adds a
# bounded block on top, whatever the number of proposals.
$noHealJson = ConvertTo-UatCompactJson (Merge-UatResults $manyRuns @($lanes[0]) @()) $manyBatch
$many.heal = @(foreach ($i in 1..40) { [ordered]@{ row = 'FS-P3'; pose = "pose$i"; diff = (Join-Path $manyBatch (Get-UatHealFileName 'FS-P3' "pose$i")) } })
$fortyJson = ConvertTo-UatCompactJson $many $manyBatch
Check ($noHealJson.Length -lt 500 -and $manyJson.Length - $noHealJson.Length -le 240 -and $fortyJson.Length - $noHealJson.Length -le 240) "heal adds at most 240 characters, for 5 or 40 proposals ($($noHealJson.Length) + $($manyJson.Length - $noHealJson.Length), $($fortyJson.Length - $noHealJson.Length))"
Check ($manyJson.Length -lt 700 -and $fortyJson.Length -lt 700) "a failed batch with heal proposals stays under 700 characters ($($manyJson.Length), $($fortyJson.Length))"
$many.heal = @(foreach ($p in 'frost', 'supply', 'guard', 'crate', 'console') {
    [ordered]@{ row = 'FS-P3'; pose = $p; diff = (Join-Path $manyBatch (Get-UatHealFileName 'FS-P3' $p)) }
})
Check ((Format-UatSummary $many 't' $manyBatch).Contains("- FS-P3 console: $manyBatch\heal-FS-P3-console.diff")) 'summary.md lists every proposal with its full path'

# -Heal with no hints: nothing is called, and the batch keeps its own code.
$script:calls = @()
$noHeal = @(Invoke-UatHeal @(Get-UatHealHints @((Rec 'default' 1 $false $frost))) 'C:\b' 's' '' 'default' { param($a) $script:calls += , $a })
$plain = Merge-UatResults @((Rec 'default' 1 $false $frost)) @($lanes[0]) @()
Check ($noHeal.Count -eq 0 -and $script:calls.Count -eq 0 -and (Get-UatExitCode $plain) -eq 1) '-Heal with no hints calls nothing and exits as before'
Check ((ConvertTo-UatCompactJson $plain 'C:\b') -notmatch '"heal"' -and (Format-UatSummary $plain 't' 'C:\b') -notmatch 'Heal') 'a batch without a proposal has no heal in -Json or summary.md'

# Through a real process: -Heal -Leases 2 stops with exit 2 before the daemon is asked.
$out = @(& $pwshExe -NoProfile -File (Join-Path $PSScriptRoot 'uat.ps1') 'first-session' -Heal -Leases 2 -Json 2>$null)
$code = $LASTEXITCODE
$obj = ($out | Select-Object -Last 1) | ConvertFrom-Json
Check ($code -eq 2 -and -not $obj.ok -and $obj.exit -eq 2 -and $obj.error -like '-Heal needs -Leases 1*') "-Heal -Leases 2 exits 2 up front ($code)"

# --- -Chaos (lab-chaos CH-02, D-CH7) ---
Check ((Format-UatChaosHeader @('cpu', 'flush:pack')) -ceq 'chaos=cpu; chaos=flush:pack') 'the chaos header has one chaos=<spec> per value, joined with "; "'
Check ($null -eq (Format-UatChaosHeader @())) 'no chaos gives no header'
Check ((Format-UatChaosHeader @('net:latency=120,loss=1')) -ceq 'chaos=net:latency=120,loss=1') 'a quoted multi-knob spec stays one value'
$cs = Merge-UatResults @((Rec 'default' 1 $true @())) @($lanes[0]) @()
$cs['chaos'] = Format-UatChaosHeader @('cpu', 'flush:pack')
Check ((ConvertTo-UatCompactJson $cs 'C:\b' | ConvertFrom-Json).chaos -ceq 'chaos=cpu; chaos=flush:pack') '-Json has the chaos string when the run had chaos'
Check ((Format-UatSummary $cs 't' 'C:\b').StartsWith('chaos=cpu; chaos=flush:pack')) 'the chaos header leads the summary'
$noChaos = Merge-UatResults @((Rec 'default' 1 $true @())) @($lanes[0]) @()
$body = @{}
Check ((Set-UatChaos $body @('', 'cpu')) -ceq 'chaos=cpu' -and (@($body.chaos) -join '|') -ceq 'cpu') 'an empty -Chaos value reaches neither the body nor the header'
$body = @{}
Check ($null -eq (Set-UatChaos $body @()) -and -not $body.ContainsKey('chaos')) 'no -Chaos leaves the body without a chaos key'
$wired = Merge-UatResults @((Rec 'default' 1 $true @())) @($lanes[0]) @()
Add-UatChaosHeader $wired (Set-UatChaos @{} @('flush:pack'))
Check ((ConvertTo-UatCompactJson $wired 'C:\b' | ConvertFrom-Json).chaos -ceq 'chaos=flush:pack') 'the header attached to a merged summary reaches -Json'
Add-UatChaosHeader $noChaos $null
Check ((ConvertTo-UatCompactJson $noChaos 'C:\b') -notmatch '"chaos"' -and (Format-UatSummary $noChaos 't' 'C:\b') -notmatch 'chaos=') 'a run without chaos has no chaos key or header'

# -Chaos reaches the lab_uat_run request body. uat.ps1 is dot-sourced with
# the daemon calls stubbed, and -PlanOnly stops before anything is driven.
$uatScript = Join-Path $PSScriptRoot 'uat.ps1'
$script:sentArgs = $null
function Invoke-UatPlanWithChaos([string[]]$Chaos) {
    . $uatScript -Section 'first-session' -PlanOnly -Chaos $Chaos
    function Get-LabToken { 'token' }
    function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
    function Get-LabdEnv { @{} }
    function Get-LabMcpUrl { 'http://stub' }
    function Get-DefaultSpecsDir { '' }
    function New-McpSession($Url, $Token) { 'session' }
    function Close-McpSession($Session) { }
    function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) {
        $script:sentArgs = $Arguments
        [pscustomobject]@{ rows = @([pscustomobject]@{ row = 'FS-01'; result = 'SKIPPED'; reasons = @() }) }
    }
    Invoke-LabUat
}
Check ((Invoke-UatPlanWithChaos 'cpu', 'flush:pack' 6>$null) -eq 0 -and @($script:sentArgs.chaos) -join '|' -ceq 'cpu|flush:pack') 'two -Chaos values both reach the request body'
Check ((Invoke-UatPlanWithChaos 'net:latency=120,loss=1' 6>$null) -eq 0 -and @($script:sentArgs.chaos).Count -eq 1 -and $script:sentArgs.chaos[0] -ceq 'net:latency=120,loss=1') 'a quoted multi-knob spec reaches the body as one element'
Check ((Invoke-UatPlanWithChaos 6>$null) -eq 0 -and -not $script:sentArgs.ContainsKey('chaos')) 'without -Chaos the body has no chaos key'

# Until CH-08, a real run refuses -Chaos before the daemon is asked.
$script:sentArgs = $null
$refused = & {
    . $uatScript -Section 'first-session' -Chaos 'cpu'
    # Stubbed so a reverted guard reaches the stub, never a live daemon.
    function Get-LabToken { 'token' }
    function Get-LabStatus { [pscustomobject]@{ instances = @([pscustomobject]@{ instance = 'default'; lease = [pscustomobject]@{ held = $false } }) } }
    function Get-LabdEnv { @{} }
    function Get-Process { }
    function Get-LabMcpUrl { 'http://stub' }
    function Get-DefaultSpecsDir { '' }
    function New-McpSession($Url, $Token) { 'session' }
    function Close-McpSession($Session) { }
    function Invoke-McpTool($Session, $Name, $Arguments, $TimeoutSec) { $script:sentArgs = $Arguments; throw 'stub: no run' }
    Invoke-LabUat
} 6>$null
Check ($refused -eq 2 -and $null -eq $script:sentArgs) 'a real run with -Chaos exits 2 and sends nothing until CH-08'
$out = @(& $pwshExe -NoProfile -File $uatScript 'first-session' -Chaos cpu -Json 2>$null)
$code = $LASTEXITCODE
$obj = ($out | Select-Object -Last 1) | ConvertFrom-Json
Check ($code -eq 2 -and -not $obj.ok -and $obj.exit -eq 2 -and $obj.error -like '-Chaos is not wired*') "-Chaos -Json exits 2 with the refusal ($code)"

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
