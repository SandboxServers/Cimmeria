<#
.SYNOPSIS
    Tests for `lab uat` (uat-lib.ps1), no Pester and no daemon. Exits non-zero
    when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-uat.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'uat-lib.ps1')

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
Check ($fiveObj.golden.match -eq 3 -and $fiveObj.golden.diverged -eq 2 -and $fiveObj.golden.first -ceq $divA -and
       -not ($fiveObj.golden.PSObject.Properties.Name -contains 'errors')) 'compact golden is {match, diverged, first}, errors only when there are some'
$longDiv = Merge-UatResults @((GRec 'default' 1 $true ('row FS-P3: ' + ('z' * 290)))) @($lanes[0]) @()
Check (((ConvertTo-UatCompactJson $longDiv 'C:\b' | ConvertFrom-Json).golden.first).Length -le 200) 'the compact first line is capped at 200 characters'
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
} finally { Remove-Item -LiteralPath $emptySpecs -Recurse -ErrorAction SilentlyContinue }

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
