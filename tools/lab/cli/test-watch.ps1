<#
.SYNOPSIS
    Tests for `lab watch` (watch-lib.ps1), no Pester and no daemon. Exits
    non-zero when any check fails:

        pwsh -NoProfile -File tools/lab/cli/test-watch.ps1

.DESCRIPTION
    Library checks read the recorded fixtures in fixtures/watch/ from a temp
    copy, with a fixed clock and fake pid checks. Command checks run watch.ps1
    in child pwsh processes through its -StatusFile and -LogFile seams, with a
    temp lab home whose labd.pid points at a closed port and an empty UAT
    root, so nothing here touches the lab or a running daemon.

    Not covered: the live loop and its terminal restore, and -Once/-Json on
    their own (a child's stdout is always redirected here, which takes the
    one-snapshot path anyway). LW-05 checks those in a real console.

.EXAMPLE
    pwsh -NoProfile -File tools/lab/cli/test-watch.ps1
#>
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'common.ps1')
. (Join-Path $PSScriptRoot 'uat-lib.ps1')
. (Join-Path $PSScriptRoot 'watch-lib.ps1')

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

$Now = [datetime]'2026-10-11T01:41:23Z'
$nowUtc = $Now.ToUniversalTime()
$tmp = Join-Path ([IO.Path]::GetTempPath()) "lab-watch-test-$PID"
New-Item -ItemType Directory -Force -Path $tmp | Out-Null
try {
    Copy-Item -Recurse -Path (Join-Path $PSScriptRoot 'fixtures\watch\*') -Destination $tmp
    $newDir = Join-Path $tmp 'batch-20261011-013906'
    $oldDir = Join-Path $tmp 'batch-20261010-193906'
    $twoLanes = Get-Content -Raw (Join-Path $tmp 'status-two-lanes.json') | ConvertFrom-Json
    $oldStatus = Get-Content -Raw (Join-Path $tmp 'status-old-daemon.json') | ConvertFrom-Json
    $logLines = @(Get-Content (Join-Path $tmp 'labd-tail.log'))
    $alive = { param($p) $true }
    $dead = { param($p) $false }

    # --- finding and reading a batch ---
    (Get-Item $oldDir).LastWriteTime = [datetime]::Now.AddMinutes(5)
    Check ((Split-Path -Leaf (Find-LabWatchBatch $tmp)) -eq 'batch-20261011-013906') 'the newest batch is picked by name, not by file time'
    Check ($null -eq (Find-LabWatchBatch (Join-Path $tmp 'nowhere'))) 'no UAT root, no batch'

    $records = @(Read-LabWatchRecords (Join-Path $newDir 'batch.jsonl'))
    Check ($records.Count -eq 3) "the half-written last line is skipped ($($records.Count) records)"

    Check ((Read-LabWatchBatch $newDir $alive).State -eq 'live') 'a plan whose pid is alive is live'
    Check ((Read-LabWatchBatch $newDir $dead).State -eq 'interrupted') 'a plan whose pid is dead is interrupted'
    $old = Read-LabWatchBatch $oldDir $alive
    Check ($old.State -eq 'done' -and $null -eq $old.Plan) 'the pre-LW-02 batch with batch.json is done, with no plan'
    Remove-Item (Join-Path $oldDir 'batch.json')
    Check ((Read-LabWatchBatch $oldDir $alive).State -eq 'unknown') 'without plan.json or batch.json the state is unknown'

    # --- the panel ---
    $panel = Get-LabWatchBatchPanel (Read-LabWatchBatch $newDir $alive)
    Check ($panel.runs -eq 3 -and $panel.total -eq 6) "runs 3 of 6 ($($panel.runs)/$($panel.total))"
    Check ($panel.passed -eq 1 -and $panel.failed -eq 2) 'passed 1, failed 2'
    Check ($panel.known -eq 2) "the known-issue table re-classifies the untagged login failure (known $($panel.known))"
    Check ($panel.last_failure.run -eq 'default#2' -and $panel.last_failure.issue -eq '#1343') 'the last failure is default#2, #1343'
    Check ($panel.last_failure.why.Length -le 72 -and $panel.last_failure.why.EndsWith('...')) 'the failure reason is cut to 72 with ...'
    Check ($null -eq (Get-LabWatchIssue ([pscustomobject]@{ Error = 'lane: boom'; Verdict = $null }))) 'an Error record has no issue'

    # --- snapshot with the two-lane status ---
    $logOnly = @($logLines)
    $snap = Get-LabWatchSnapshot -Status $twoLanes -BatchDir $newDir -LogLines $logOnly -SgwPids @(49448, 63904) -Now $Now -IsAlive $alive
    $byName = @{}; foreach ($i in $snap.instances) { $byName[$i.instance] = $i }
    Check ($snap.daemon -eq 'up' -and -not $snap.old_daemon) 'the daemon is up and serves run progress'
    Check ($byName['default'].run -eq '3/3' -and $byName['p2'].run -eq '2/3' -and $byName['p3'].run -eq '-') "RUN default 3/3, p2 2/3, p3 - ($($byName['default'].run), $($byName['p2'].run), $($byName['p3'].run))"
    Check ($byName['default'].row -eq 'FS-P3' -and $byName['default'].row_s -eq 42) "ROW default FS-P3 42s ($($byName['default'].row_s))"
    Check ($byName['p2'].row -eq 'FS-02' -and $byName['p2'].row_s -eq 3) "ROW p2 FS-02 3s ($($byName['p2'].row_s))"
    Check ($byName['p3'].row -eq '-' -and $byName['p3'].lease -eq 'free') 'p3 is free with no row'
    Check ($byName['p2'].lease -like 'lab_uat_run (*...)' -and $byName['p2'].lease -like '*lease-<redacted>*') "the lease purpose is redacted and cut ($($byName['p2'].lease))"
    Check ($byName['p2'].last -like '*FAIL FS-P2*[known #1341]*') 'LAST is the lane''s last progress line'
    Check ($byName['default'].last -like '*[known #1343]*') 'LAST carries the re-classified issue'
    $text = @(Format-LabWatchText $snap 120)
    $json = ConvertTo-LabWatchJson $snap
    $everything = ($snap | ConvertTo-Json -Depth 10) + ($text -join "`n") + $json
    Check (-not $everything.Contains('0123456789abcdef')) 'no lease id in the snapshot, text or JSON'
    Check ((@($snap.orphans) -join ',') -eq '63904') "orphans are SGW pids no instance owns ($(@($snap.orphans) -join ','))"
    $down = Get-LabWatchSnapshot -Status $null -BatchDir $newDir -LogLines $logOnly -SgwPids @(49448, 63904) -Now $Now -IsAlive $alive
    Check (@($down.orphans).Count -eq 0 -and $down.daemon -eq 'down') 'no orphans when the daemon is down'
    Check ((@($down.instances | ForEach-Object { $_.instance }) -join ',') -eq 'default,p2') 'daemon down: one row per plan lane'
    $extra = [pscustomobject]@{ daemon = $twoLanes.daemon; instances = @($twoLanes.instances | Where-Object { $_.instance -ne 'p2' }) }
    $mixed = Get-LabWatchSnapshot -Status $extra -BatchDir $newDir -Now $Now -IsAlive $alive
    Check ((@($mixed.instances | ForEach-Object { $_.instance }) -join ',') -eq 'default,p3,p2') 'a plan lane /status lacks is added after its rows'

    # --- the old daemon ---
    $oldSnap = Get-LabWatchSnapshot -Status $oldStatus -BatchDir $newDir -Now $Now -IsAlive $alive
    Check ($oldSnap.old_daemon -and @($oldSnap.instances | Where-Object { $_.row -ne '?' }).Count -eq 0) 'a daemon without uat reads ROW ? and old_daemon'
    Check ((@(Format-LabWatchText $oldSnap 120)) -contains 'daemon has no run progress: lab install, then lab restart') 'the old-daemon footer'

    # --- a braked lane ---
    $braked = (Get-Content -Raw (Join-Path $newDir 'batch.jsonl')) -replace '\{"Lane":2,"Instance":"p2","Ind$', ''
    $brakeRec = [ordered]@{ Lane = 2; Instance = 'p2'; Index = 2; Ok = $false; Rows = @(); Verdict = $null; RunDir = $null
                            Error = 'lab_uat_run failed: bridge down'; Seconds = 3; Braked = $true }
    $brakeDir = Join-Path $tmp 'batch-braked'
    New-Item -ItemType Directory -Path $brakeDir | Out-Null
    Copy-Item (Join-Path $newDir 'plan.json') $brakeDir
    [IO.File]::WriteAllText((Join-Path $brakeDir 'batch.jsonl'), $braked + ($brakeRec | ConvertTo-Json -Compress -Depth 6) + "`n", [Text.UTF8Encoding]::new($false))
    $bSnap = Get-LabWatchSnapshot -Status $twoLanes -BatchDir $brakeDir -Now $Now -IsAlive $alive
    Check (@($bSnap.instances | Where-Object { $_.instance -eq 'p2' })[0].run -eq '2/3 braked') 'a braked lane reads <n>/3 braked'

    # --- screenshot age ---
    $runDir = Join-Path $tmp 'run-shot'
    New-Item -ItemType Directory -Force -Path (Join-Path $runDir 'rows\s\A') | Out-Null
    $png = Join-Path $runDir 'rows\s\A\final.png'
    [IO.File]::WriteAllBytes($png, [byte[]](1, 2, 3))
    (Get-Item $png).LastWriteTimeUtc = $nowUtc.AddSeconds(-5)
    Check ((Get-LabWatchShotAge $runDir $Now) -eq 5) 'the newest screenshot is 5 s old'
    Check ($null -eq (Get-LabWatchShotAge $tmp $Now)) 'a run dir without rows has no screenshot age'
    # SHOT reads /status uat's run_dir ahead of the lane's last record's RunDir.
    $shotStatus = Get-Content -Raw (Join-Path $tmp 'status-two-lanes.json') | ConvertFrom-Json
    @($shotStatus.instances | Where-Object { $_.instance -eq 'default' })[0].uat.run_dir = $runDir
    $shotSnap = Get-LabWatchSnapshot -Status $shotStatus -BatchDir $newDir -Now $Now -IsAlive $alive
    Check (@($shotSnap.instances | Where-Object { $_.instance -eq 'default' })[0].shot_s -eq 5) 'SHOT ages the screenshot under /status uat''s run_dir'
    Check ((ConvertTo-LabWatchJson $shotSnap | ConvertFrom-Json).lanes.default -eq '3/3 FS-P3 42s shot 5s') 'the JSON lane string carries the shot age'

    # --- a lane death (Index 0) ---
    $deathRec = [ordered]@{ Lane = 2; Instance = 'p2'; Index = 0; Ok = $false; Rows = @(); Verdict = $null; RunDir = $null
                            Error = 'lane: bridge never came up'; Seconds = 1; Braked = $true }
    $deathDir = Join-Path $tmp 'batch-death'
    New-Item -ItemType Directory -Path $deathDir | Out-Null
    Copy-Item (Join-Path $newDir 'plan.json') $deathDir
    [IO.File]::WriteAllText((Join-Path $deathDir 'batch.jsonl'), $braked + ($deathRec | ConvertTo-Json -Compress -Depth 6) + "`n", [Text.UTF8Encoding]::new($false))
    $dPanel = Get-LabWatchBatchPanel (Read-LabWatchBatch $deathDir $alive)
    Check ($dPanel.runs -eq 3 -and $dPanel.failed -eq 2) "a lane death is not a run (runs $($dPanel.runs), failed $($dPanel.failed))"
    Check ($dPanel.last_failure.run -eq 'p2#0') "a lane death can be the last failure ($($dPanel.last_failure.run))"
    $dSnap = Get-LabWatchSnapshot -Status $twoLanes -BatchDir $deathDir -Now $Now -IsAlive $alive
    Check (@($dSnap.instances | Where-Object { $_.instance -eq 'p2' })[0].run -eq '1/3 braked') 'a lane death brakes its lane'

    # --- labd.log counts ---
    $counts = Get-LabWatchLogCounts $logLines '2026-10-11T01:39:06.0000000Z'
    Check ($counts.relaunches['p2'] -eq 1 -and $counts.relaunches.Count -eq 1) 'one relaunch on p2 since the plan started (the earlier one excluded)'
    Check ($counts.login_retries -eq 1) 'one login retry'
    Check ($snap.logs.relaunches['p2'] -eq 1 -and $snap.logs.login_retries -eq 1) 'the snapshot counts the log since the plan start'
    $oneInstance = Get-LabWatchLogCounts @('2026-10-11T01:40:00.000000Z  INFO lab_watchdog{pid=1}: relaunched after crash; logging back in new_pid=2') ''
    Check ($oneInstance.relaunches['default'] -eq 1) 'a relaunch line without an instance span counts for default'

    # --- JSON ---
    $parsed = $json | ConvertFrom-Json
    Check ($json.Length -lt 500 -and -not $json.Contains('null')) "two-lane JSON under 500 characters with no null ($($json.Length))"
    Check ($parsed.ok -and $parsed.state -eq 'live' -and $parsed.runs -eq '3/6' -and $parsed.known -eq 2 -and $parsed.lanes.default -eq '3/3 FS-P3 42s') 'two-lane JSON fields'
    Check ($json.StartsWith('{"ok":true,"daemon":"up","batch":"batch-20261011-013906","state":"live","section":"first-session","runs":"3/6","passed":1,"failed":2,"known":2,"last_failure":')) 'JSON keys are in contract order'
    Write-Host "two-lane JSON: $json"
    Check ((ConvertTo-LabWatchJson (Get-LabWatchSnapshot -Status $null -BatchDir '' -Now $Now -IsAlive $alive)) -ceq '{"ok":true,"daemon":"down","state":"none"}') 'daemon down with no batch'

    # The worst five-lane case: 20/20 runs, a long reason, 5 orphans, a relaunch per instance.
    $names = 'default', 'p2', 'p3', 'p4', 'p5'
    $worstDir = Join-Path $tmp 'batch-20261011-090000'
    New-Item -ItemType Directory -Path $worstDir | Out-Null
    Write-UatPlan (Join-Path $worstDir 'plan.json') ([ordered]@{ schema = 1; section = 'first-session'; rows = @('FS-01')
        lanes = @(for ($k = 0; $k -lt 5; $k++) { [ordered]@{ lane = $k + 1; instance = $names[$k] } }); runs_per_lease = 20
        started = '2026-10-11T01:00:00.0000000Z'; pid = 1 })
    $long = 'x' * 600
    $lines = foreach ($n in $names) { foreach ($k in 1..20) {
        [ordered]@{ Lane = 1; Instance = $n; Index = $k; Ok = $false; Rows = @(); RunDir = $null; Error = $null; Seconds = 99; Braked = $false
                    Verdict = [ordered]@{ Ok = $false; Row = 'FS-P2'; Result = 'FAIL'; Reason = "movie-over $long"; Issue = $null } } | ConvertTo-Json -Compress -Depth 6
    } }
    [IO.File]::WriteAllText((Join-Path $worstDir 'batch.jsonl'), ($lines -join "`n") + "`n", [Text.UTF8Encoding]::new($false))
    $wStatus = [pscustomobject]@{ daemon = $twoLanes.daemon; instances = @(for ($k = 0; $k -lt 5; $k++) {
        [pscustomobject]@{ instance = $names[$k]; client_pid = 50000 + $k; lease = [pscustomobject]@{ held = $true; lease = [pscustomobject]@{ owner = 'lab_uat_run'; purpose = $long } }
            uat = [pscustomobject]@{ run_dir = $runDir; current_row = 'FS-P5'; row_started_at = '2026-10-11T01:30:00.000Z' } } }) }
    $wLog = foreach ($n in $names) { "2026-10-11T01:40:00.000000Z  INFO lab_watchdog{instance=`"$n`" pid=1}: relaunched after crash; logging back in new_pid=2" }
    $worst = Get-LabWatchSnapshot -Status $wStatus -BatchDir $worstDir -LogLines @($wLog) -SgwPids @(1, 2, 3, 4, 5) -Now $Now -IsAlive $alive
    $wJson = ConvertTo-LabWatchJson $worst
    $wParsed = $wJson | ConvertFrom-Json
    Check ($wJson.Length -lt 700) "the worst five-lane JSON is under 700 characters ($($wJson.Length))"
    Check (@($wParsed.orphans).Count -eq 3 -and $wParsed.relaunches -eq 5 -and $wParsed.runs -eq '100/100') 'worst case: 3 orphans shown, 5 relaunches, 100/100 runs'
    Check (@(Format-LabWatchText $worst 120 | Where-Object { $_.Length -gt 120 }).Count -eq 0) 'the worst-case text fits 120 columns'

    # --- text ---
    Check (@($text | Where-Object { $_.Length -gt 120 }).Count -eq 0) 'every text line fits 120 columns'
    Check (@($text | Where-Object { $_ -like 'INSTANCE *' }).Count -eq 1 -and @($text | Where-Object { $_ -match '^p3\s.*\sfree\s' }).Count -eq 1) 'the INSTANCE header and a free p3 row'
    Check (@($text | Where-Object { $_ -like '*orphan SGW.exe: 63904 (#1342?)*' }).Count -eq 1) 'the orphan footer'
    Check ($text[0] -eq 'batch-20261011-013906  first-session  live  runs 3/6  passed 1  failed 2 (2 known)') "the header line ($($text[0]))"
    Check (@($text | Where-Object { $_ -like '*FS-P3 42s*' }).Count -eq 1) 'the default row shows FS-P3 42s'
    $pIdx = [array]::FindIndex([string[]]$text, [Predicate[string]] { param($l) $l -match '^p2\s' })
    Check ($pIdx -ge 0 -and $text[$pIdx + 1] -like '    `[p2 1/3`] FAIL FS-P2: clause movie-over failed*') "LAST is whole on its own line under its row ($($text[$pIdx + 1]))"
    Check (@($text | Where-Object { $_ -match '^(default|p2)\s.*\[(default|p2) ' }).Count -eq 0) 'no instance row also carries LAST'
    Write-Host ($text -join "`n")

    # --- the pre-LW-02 batch's real records ---
    try {
        $oldSnapshot = Get-LabWatchSnapshot -Status $twoLanes -BatchDir $oldDir -Now $Now -IsAlive $alive
        $oj = ConvertTo-LabWatchJson $oldSnapshot
        $null = Format-LabWatchText $oldSnapshot 120
        Check ($oldSnapshot.batch.passed -eq 2 -and $oldSnapshot.batch.State -eq 'unknown' -and ($oj | ConvertFrom-Json).runs -eq '2') "the pre-LW-02 records aggregate under strict mode ($oj)"
        $oldLasts = @($oldSnapshot.instances | Where-Object { $_.last } | ForEach-Object { $_.last })
        Check ($oldLasts.Count -gt 0 -and @($oldLasts | Where-Object { $_ -notmatch '^\[\S+ \d+/\?\] ' }).Count -eq 0) "without a plan LAST reads <i>/? ($($oldLasts -join ' | '))"
    } catch {
        Check $false "the pre-LW-02 records aggregate under strict mode: $_"
    }

    # --- the command: watch.ps1 in a child pwsh, against the seams only ---
    # Every case but the usage ones passes -StatusFile, and the temp lab home's
    # labd.pid points at a closed port, so no case can reach a running daemon
    # (the token is a user environment variable, so a temp home alone is not enough).
    $watch = Join-Path $PSScriptRoot 'watch.ps1'
    $savedEnv = @{ Home = $env:CIMMERIA_LAB_HOME; UatDir = $env:CIMMERIA_LAB_UAT_DIR }
    try {
        $env:CIMMERIA_LAB_HOME = Join-Path $tmp 'home'
        $env:CIMMERIA_LAB_UAT_DIR = Join-Path $tmp 'no-batches'
        New-Item -ItemType Directory -Path $env:CIMMERIA_LAB_HOME, $env:CIMMERIA_LAB_UAT_DIR | Out-Null
        [IO.File]::WriteAllText((Join-Path $env:CIMMERIA_LAB_HOME 'labd.pid'), '{"pid":1,"bind":"127.0.0.1:9"}')
        $statusPath = Join-Path $tmp 'status-two-lanes.json'
        $logPath = Join-Path $tmp 'labd-tail.log'
        $states = 'live', 'done', 'interrupted', 'unknown', 'none'

        $out = @(pwsh -NoProfile -File $watch -Once -Json -Batch $newDir -StatusFile $statusPath -LogFile $logPath)
        $code = $LASTEXITCODE
        $cj = if ($out.Count -eq 1) { try { $out[0] | ConvertFrom-Json } catch { $null } } else { $null }
        Check ($code -eq 0 -and $null -ne $cj) "-Once -Json exits 0 with one JSON line (exit $code, $($out.Count) lines)"
        Check ($null -ne $cj -and $states -contains $cj.state -and $cj.PSObject.Properties.Name -contains 'lanes') "the JSON has a state of the five and lanes ($($out -join ' '))"
        Check ($out.Count -eq 1 -and $out[0].Length -lt 500 -and -not $out[0].Contains('0123456789abcdef')) 'the command''s JSON is under 500 characters with no lease id'
        Write-Host "watch -Once -Json: $($out -join ' ')"

        $out = @(pwsh -NoProfile -File $watch -Once -Batch $newDir -StatusFile $statusPath -LogFile $logPath)
        $code = $LASTEXITCODE
        $all = $out -join "`n"
        Check ($code -eq 0 -and @($out | Where-Object { $_ -like 'INSTANCE *' }).Count -eq 1) "-Once prints the text with its INSTANCE header (exit $code)"
        Check (@($out | Where-Object { $_.Length -gt 120 }).Count -eq 0 -and -not $all.Contains([char]27) -and -not $all.Contains('0123456789abcdef')) '-Once text: 120 columns, no escape codes, no lease id'
        Write-Host "watch -Once:`n$all"

        $null = pwsh -NoProfile -File $watch -Once -IntervalSeconds 0 -StatusFile $statusPath 2>$null
        Check ($LASTEXITCODE -eq 2) "-IntervalSeconds 0 exits 2 (got $LASTEXITCODE)"
        $out = @(pwsh -NoProfile -File $watch -Json -IntervalSeconds 61 -StatusFile $statusPath)
        $code = $LASTEXITCODE
        Check ($code -eq 2 -and $out.Count -eq 1 -and $out[0].StartsWith('{"ok":false,"exit":2,')) "-IntervalSeconds 61 -Json exits 2 with lab uat's error shape ($($out -join ' '))"

        $empty = Join-Path $tmp 'empty-batch'
        New-Item -ItemType Directory -Path $empty | Out-Null
        $null = pwsh -NoProfile -File $watch -Once -Batch $empty -StatusFile $statusPath 2>$null
        Check ($LASTEXITCODE -eq 2) "a -Batch folder with neither plan.json nor batch.jsonl exits 2 (got $LASTEXITCODE)"
        $null = pwsh -NoProfile -File $watch -Once -StatusFile (Join-Path $tmp 'no-such-status.json') 2>$null
        Check ($LASTEXITCODE -eq 2) "a missing -StatusFile exits 2 (got $LASTEXITCODE)"

        # Daemon down (a status file without the daemon's shape) and no batch under the UAT root.
        $downStatus = Join-Path $tmp 'status-down.json'
        [IO.File]::WriteAllText($downStatus, 'null')
        $out = @(pwsh -NoProfile -File $watch -Json -StatusFile $downStatus -LogFile $logPath)
        Check ($LASTEXITCODE -eq 1 -and ($out -join '') -ceq '{"ok":true,"daemon":"down","state":"none"}') "daemon down and no batch exits 1 ($LASTEXITCODE, $($out -join ' '))"

        # A labd.log held open with no sharing can't be read: exit 3 with the
        # error shape, not exit 1 as if the daemon were down.
        $lockedLog = Join-Path $tmp 'locked.log'
        [IO.File]::WriteAllText($lockedLog, "x`n")
        $lock = [IO.File]::Open($lockedLog, 'Open', 'Read', 'None')
        try { $out = @(pwsh -NoProfile -File $watch -Json -Batch $newDir -StatusFile $statusPath -LogFile $lockedLog) }
        finally { $lock.Dispose() }
        Check ($LASTEXITCODE -eq 3 -and ($out -join '').StartsWith('{"ok":false,"exit":3,')) "an unreadable labd.log exits 3 with the error shape ($LASTEXITCODE, $($out -join ' '))"

        # Redirected stdout without -Once: one snapshot, never the live loop. A
        # 20 s timeout fails the check instead of hanging the job.
        $redir = Join-Path $tmp 'redirected.txt'
        $redirErr = Join-Path $tmp 'redirected-err.txt'
        $argLine = "-NoProfile -File `"$watch`" -Batch `"$newDir`" -StatusFile `"$statusPath`" -LogFile `"$logPath`""
        $p = Start-Process pwsh -ArgumentList $argLine -NoNewWindow -PassThru -RedirectStandardOutput $redir -RedirectStandardError $redirErr
        if ($p.WaitForExit(20000)) {
            $p.WaitForExit()
            $text = [IO.File]::ReadAllText($redir)
            Check ($p.ExitCode -eq 0 -and $text.Contains('INSTANCE') -and -not $text.Contains([char]27)) "redirected output prints one snapshot and exits (exit $($p.ExitCode))"
        } else {
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            Check $false 'redirected output prints one snapshot and exits (still running after 20 s: it entered the live loop)'
        }

        # Read-only (D-LW3): the command never drives the daemon, and the docs never say --once (F11).
        $src = Get-Content -Raw $watch
        Check (-not ($src -match '/mcp|Get-LabMcpUrl|lab_uat_run|lease_acquire|lab_lease')) 'watch.ps1 calls no /mcp tool and takes no lease'
        $guide = Join-Path $PSScriptRoot '..\..\..\docs\guides\live-research-lab.md'
        Check ((Test-Path $guide) -and -not (Get-Content -Raw $guide).Contains('--once')) 'the lab guide has no --once'
    } finally {
        $env:CIMMERIA_LAB_HOME = $savedEnv.Home
        $env:CIMMERIA_LAB_UAT_DIR = $savedEnv.UatDir
    }
} finally {
    Remove-Item -Recurse -Force -LiteralPath $tmp -ErrorAction SilentlyContinue
}

if ($script:failed) { Write-Host "$script:failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
