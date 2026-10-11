<#
.SYNOPSIS
    Library for `lab watch`: pure functions over a `/status` object, a batch
    folder, `labd.log` lines and `SGW` pids. Dot-source it after common.ps1 and
    uat-lib.ps1. Never print a token or a lease id.

.DESCRIPTION
    Get-LabWatchSnapshot builds the whole model; Format-LabWatchText and
    ConvertTo-LabWatchJson render it as the `-Once` text and the `-Once -Json`
    line. Nothing here asks the daemon, lists processes or reads the clock:
    the caller passes /status, the SGW pids and the time ($Now, UTC), so
    tools/lab/cli/test-watch.ps1 drives it all from recorded fixtures.

    Every string that comes from a record, a lease or the log goes through
    Hide-LeaseIds (D-LW3).

.EXAMPLE
    . .\common.ps1; . .\uat-lib.ps1; . .\watch-lib.ps1
    $snap = Get-LabWatchSnapshot -Status (Get-LabStatus) -BatchDir (Find-LabWatchBatch (Get-UatRoot)) `
        -LogLines @() -SgwPids @() -Now ([datetime]::UtcNow)
    Format-LabWatchText $snap 120
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# A JSON time as a UTC DateTime, or $null. ConvertFrom-Json turns ISO strings
# into DateTime of Kind Utc ('Z') or Local (an offset), and a [datetime] cast of
# a 'Z' string is Local too, so everything is normalised before subtracting.
function ConvertTo-LabWatchUtc($Value) {
    if ($null -eq $Value) { return $null }
    if ($Value -is [datetime]) {
        if ($Value.Kind -eq [DateTimeKind]::Local) { return $Value.ToUniversalTime() }
        return [datetime]::SpecifyKind($Value, [DateTimeKind]::Utc)
    }
    if ($Value -is [datetimeoffset]) { return $Value.UtcDateTime }
    $text = "$Value".Trim()
    if (-not $text) { return $null }
    $parsed = [datetimeoffset]::MinValue
    if ([datetimeoffset]::TryParse($text, [cultureinfo]::InvariantCulture, [Globalization.DateTimeStyles]::AssumeUniversal, [ref]$parsed)) {
        return $parsed.UtcDateTime
    }
    return $null
}

# The batch-* folder under the UAT root with the greatest name, or $null. Names
# are batch-yyyyMMdd-HHmmss, so name order is start order (file times are not).
function Find-LabWatchBatch([string]$UatRoot) {
    if (-not $UatRoot -or -not (Test-Path -LiteralPath $UatRoot)) { return $null }
    $dirs = @(Get-ChildItem -LiteralPath $UatRoot -Directory -Filter 'batch-*' | Sort-Object Name)
    if (-not $dirs.Count) { return $null }
    return $dirs[-1].FullName
}

# batch.jsonl's records. A lane may be appending while we read, so a line that
# does not parse (a half-written last line) is skipped, never thrown (F16).
function Read-LabWatchRecords([string]$Path) {
    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) { return @() }
    $out = foreach ($l in (Get-Content -LiteralPath $Path)) {
        if (-not $l.Trim()) { continue }
        try { $r = $l | ConvertFrom-Json -Depth 12 } catch { continue }
        if ($r -is [pscustomobject]) { $r }
    }
    return @($out)
}

# A batch folder: @{ Dir; Name; Plan; Records; State }. State (D-LW5): done when
# batch.json exists, unknown without a plan.json (a pre-LW-02 batch), live when
# the plan's pid is alive, else interrupted. $IsAlive takes a pid, returns a bool.
function Read-LabWatchBatch([string]$Dir, [scriptblock]$IsAlive = { param($p) [bool](Get-Process -Id $p -ErrorAction SilentlyContinue) }) {
    $plan = Read-UatPlan (Join-Path $Dir 'plan.json')
    $records = @(Read-LabWatchRecords (Join-Path $Dir 'batch.jsonl'))
    $state = if (Test-Path -LiteralPath (Join-Path $Dir 'batch.json')) { 'done' }
        elseif ($null -eq $plan) { 'unknown' }
        else {
            $planPid = Get-UatField $plan 'pid'
            if ($null -ne $planPid -and [bool](& $IsAlive ([int]$planPid))) { 'live' } else { 'interrupted' }
        }
    return [ordered]@{ Dir = $Dir; Name = (Split-Path -Leaf $Dir); Plan = $plan; Records = $records; State = $state }
}

# A record's known issue from the one table (D-LW4): Get-UatKnownIssue first, the
# record's own Verdict.Issue only as a fallback; $null for an Error record.
function Get-LabWatchIssue($Record) {
    if (Get-UatField $Record 'Error') { return $null }
    $verdict = Get-UatField $Record 'Verdict'
    $known = Get-UatKnownIssue ([string](Get-UatField $verdict 'Row')) ([string](Get-UatField $verdict 'Reason'))
    if ($known) { return $known }
    $own = Get-UatField $verdict 'Issue'
    if ($own) { return [string]$own }
    return $null
}

# A record as a failure line: "<row> <result>: <reason>" or "error: <error>",
# redacted and cut to 72 characters.
function Get-LabWatchWhy($Record) {
    $err = Get-UatField $Record 'Error'
    if ($err) { $why = "error: $err" }
    else {
        $v = Get-UatField $Record 'Verdict'
        $why = "$(Get-UatField $v 'Row') $(Get-UatField $v 'Result'): $(Get-UatField $v 'Reason')"
    }
    return Limit-UatText (Hide-LeaseIds $why) 72
}

# A record's progress line as lab uat printed it, but tagged with the
# re-classified issue so it agrees with the footer and the counts.
function Format-LabWatchLast($Record, [int]$Total) {
    $copy = [ordered]@{}
    foreach ($p in $Record.PSObject.Properties) { $copy[$p.Name] = $p.Value }
    $verdict = [ordered]@{}
    $v = Get-UatField $Record 'Verdict'
    if ($null -ne $v) { foreach ($p in $v.PSObject.Properties) { $verdict[$p.Name] = $p.Value } }
    $verdict['Issue'] = Get-LabWatchIssue $Record
    $copy['Verdict'] = $verdict
    return Hide-LeaseIds (Format-UatProgressLine $copy $Total)
}

# The batch's counts and lanes. Index 0 records are lane deaths, not runs: they
# brake their lane and can be the last failure, but are not counted as runs.
function Get-LabWatchBatchPanel($Batch) {
    $plan = Get-UatField $Batch 'Plan'
    $records = @(Get-UatField $Batch 'Records')
    $perLane = if ($null -ne $plan) { [int](Get-UatField $plan 'runs_per_lease') } else { $null }

    $lanes = [ordered]@{}
    if ($null -ne $plan) {
        foreach ($l in @(Get-UatField $plan 'lanes')) {
            $name = [string](Get-UatField $l 'instance')
            if ($name -and -not $lanes.Contains($name)) { $lanes[$name] = @{ count = 0; total = $perLane; braked = $false; last = $null } }
        }
    }
    $runs = 0; $passed = 0; $failed = 0; $known = 0; $lastFailure = $null
    foreach ($r in $records) {
        $name = [string](Get-UatField $r 'Instance')
        if (-not $lanes.Contains($name)) { $lanes[$name] = @{ count = 0; total = $perLane; braked = $false; last = $null } }
        $lane = $lanes[$name]
        $isRun = [int](Get-UatField $r 'Index') -gt 0
        $ok = [bool](Get-UatField $r 'Ok')
        if ($isRun) {
            $runs++; $lane['count']++
            if ($ok) { $passed++ } else { $failed++; if (Get-LabWatchIssue $r) { $known++ } }
        }
        if ([bool](Get-UatField $r 'Braked')) { $lane.braked = $true }
        $lane.last = $r
        if (-not $ok) {
            $lastFailure = @{ run = "$name#$(Get-UatField $r 'Index')"; why = (Get-LabWatchWhy $r); issue = (Get-LabWatchIssue $r) }
        }
    }

    $section = if ($null -ne $plan) { [string](Get-UatField $plan 'section') } else { $null }
    if (-not $section -and $records.Count) {
        $section = [string](Get-UatField (@(Get-UatField $records[0] 'Rows') | Select-Object -First 1) 'section')
    }
    $total = if ($null -ne $plan) { @(Get-UatField $plan 'lanes').Count * $perLane } else { $null }
    return [ordered]@{ section = $section; runs = $runs; total = $total; passed = $passed; failed = $failed
                       known = $known; last_failure = $lastFailure; lanes = $lanes }
}

# Whole seconds since the newest *.png under <RunDir>\rows, or $null.
function Get-LabWatchShotAge([string]$RunDir, [datetime]$Now) {
    if (-not $RunDir) { return $null }
    $rows = Join-Path $RunDir 'rows'
    if (-not (Test-Path -LiteralPath $rows)) { return $null }
    $newest = Get-ChildItem -LiteralPath $rows -Recurse -File -Filter '*.png' | Sort-Object LastWriteTimeUtc | Select-Object -Last 1
    if ($null -eq $newest) { return $null }
    $age = (ConvertTo-LabWatchUtc $Now) - $newest.LastWriteTimeUtc
    return [int][Math]::Max(0, [Math]::Floor($age.TotalSeconds))
}

# Relaunches per instance and login retries in labd.log lines stamped at or
# after $Since (an ordinal compare of the ISO prefix; both are UTC 'Z' stamps).
function Get-LabWatchLogCounts([string[]]$Lines, [string]$Since) {
    $relaunches = [ordered]@{}; $retries = 0
    foreach ($line in @($Lines)) {
        if (-not $line -or $line -notmatch '^(\d{4}-\d\d-\d\dT\S+)') { continue }
        if ($Since -and [string]::CompareOrdinal($Matches[1], $Since) -lt 0) { continue }
        if ($line.Contains('relaunched after crash')) {
            # A line without an instance span comes from a one-instance daemon.
            $inst = if ($line -match 'instance="([^"]+)"') { $Matches[1] } else { 'default' }
            $relaunches[$inst] = 1 + [int]$relaunches[$inst]
        } elseif ($line.Contains('credential typing missed')) {
            $retries++
        }
    }
    return @{ relaunches = $relaunches; login_retries = $retries }
}

# An instance's LEASE cell: 'free', or "<owner> (<purpose>)" with the purpose
# redacted and then cut to 40 characters. remaining_s is never shown.
function Format-LabWatchLease($Lease) {
    if (-not [bool](Get-UatField $Lease 'held')) { return 'free' }
    $inner = Get-UatField $Lease 'lease'
    $owner = Hide-LeaseIds ([string](Get-UatField $inner 'owner'))
    $purpose = Hide-LeaseIds ([string](Get-UatField $inner 'purpose'))
    if ($purpose.Length -gt 40) { $purpose = $purpose.Substring(0, 40) + '...' }
    if (-not $purpose) { return $owner }
    return "$owner ($purpose)"
}

# One instance row: RUN from the batch lane, ROW from /status uat (? when the
# daemon predates LW-01), SHOT from the run dir (uat's, else the lane's last).
function New-LabWatchInstance([string]$Name, $Row, [bool]$HasRow, $Batch, $Panel, [datetime]$Now) {
    $lane = $null
    if ($null -ne $Panel -and $Panel.lanes.Contains($Name)) { $lane = $Panel.lanes[$Name] }
    $run = '-'; $last = $null; $runDir = $null
    if ($null -ne $lane) {
        $total = $lane.total
        if ($null -eq $total) { $run = "$($lane['count'])" }
        elseif ($lane.braked) { $run = "$($lane['count'])/$total braked" }
        elseif ($Batch.State -eq 'live' -and $lane['count'] -lt $total) { $run = "$($lane['count'] + 1)/$total" }
        else { $run = "$($lane['count'])/$total" }
        if ($null -ne $lane.last) {
            $last = Format-LabWatchLast $lane.last $(if ($null -ne $total) { $total } else { $lane['count'] })
            $runDir = [string](Get-UatField $lane.last 'RunDir')
        }
    }
    $rowId = '-'; $rowS = $null; $client = $null; $lease = $null
    if ($HasRow) {
        $client = Get-UatField $Row 'client_pid'
        $lease = Format-LabWatchLease (Get-UatField $Row 'lease')
        if ($Row.PSObject.Properties.Name -notcontains 'uat') { $rowId = '?' }
        else {
            $uat = $Row.uat
            if ($null -ne $uat) {
                $cur = Get-UatField $uat 'current_row'
                if ($cur) {
                    $rowId = Hide-LeaseIds ([string]$cur)
                    $started = ConvertTo-LabWatchUtc (Get-UatField $uat 'row_started_at')
                    if ($null -ne $started) { $rowS = [int][Math]::Max(0, [Math]::Floor(($Now - $started).TotalSeconds)) }
                }
                $uatDir = [string](Get-UatField $uat 'run_dir')
                if ($uatDir) { $runDir = $uatDir }
            }
        }
    }
    return [ordered]@{ instance = $Name; client_pid = $client; lease = $lease; run = $run; row = $rowId
                       row_s = $rowS; shot_s = (Get-LabWatchShotAge $runDir $Now); last = $last }
}

# The whole watch model (see the LW-03 packet). -Status is the /status object
# or $null (daemon down); -Now is UTC; -IsAlive decides a live batch's pid.
function Get-LabWatchSnapshot {
    param($Status, [string]$BatchDir, [string[]]$LogLines = @(), [int[]]$SgwPids = @(), [datetime]$Now,
          [scriptblock]$IsAlive = { param($p) [bool](Get-Process -Id $p -ErrorAction SilentlyContinue) })
    $nowUtc = ConvertTo-LabWatchUtc $Now
    $batch = $null; $panel = $null; $raw = $null
    if ($BatchDir) {
        $raw = Read-LabWatchBatch $BatchDir $IsAlive
        $panel = Get-LabWatchBatchPanel $raw
        $batch = [ordered]@{ Name = $raw.Name; State = $raw.State }
        foreach ($k in $panel.Keys) { $batch[$k] = $panel[$k] }
    }

    $statusRows = if ($null -ne $Status) { @(Get-UatField $Status 'instances' | Where-Object { $null -ne $_ }) } else { @() }
    $instances = @(); $seen = @{}
    foreach ($row in $statusRows) {
        $name = [string](Get-UatField $row 'instance')
        $seen[$name] = $true
        $instances += New-LabWatchInstance $name $row $true $raw $panel $nowUtc
    }
    if ($null -ne $panel) {
        foreach ($name in $panel.lanes.Keys) {
            if (-not $seen.Contains($name)) { $instances += New-LabWatchInstance $name $null $false $raw $panel $nowUtc }
        }
    }

    $oldDaemon = $false
    foreach ($row in $statusRows) { if ($row.PSObject.Properties.Name -notcontains 'uat') { $oldDaemon = $true } }

    $orphans = @()
    if ($null -ne $Status) {
        $clients = @($statusRows | ForEach-Object { Get-UatField $_ 'client_pid' } | Where-Object { $null -ne $_ } | ForEach-Object { [int]$_ })
        $orphans = @(@($SgwPids) | Where-Object { $clients -notcontains $_ } | Select-Object -Unique)
    }

    $logs = $null
    $plan = if ($null -ne $raw) { $raw.Plan } else { $null }
    if ($null -ne $plan) {
        $since = ConvertTo-LabWatchUtc (Get-UatField $plan 'started')
        $sinceText = if ($null -ne $since) { $since.ToString("yyyy-MM-dd'T'HH:mm:ss.fffffff'Z'", [cultureinfo]::InvariantCulture) } else { '' }
        $logs = Get-LabWatchLogCounts $LogLines $sinceText
    }

    return [ordered]@{ daemon = $(if ($null -ne $Status) { 'up' } else { 'down' }); old_daemon = $oldDaemon
                       batch = $batch; instances = $instances; orphans = $orphans; logs = $logs }
}

# An instance's ROW cell: "<row> <s>s", '-' or '?'.
function Format-LabWatchRowCell($Instance) {
    if ($null -ne $Instance.row_s) { return "$($Instance.row) $($Instance.row_s)s" }
    return $Instance.row
}

# The -Once text lines, each at most $Width characters.
function Format-LabWatchText($Snapshot, [int]$Width = 120) {
    $lines = [System.Collections.Generic.List[string]]::new()
    $b = $Snapshot.batch
    if ($null -eq $b) {
        $lines.Add($(if ($Snapshot.daemon -eq 'down') { 'no batch  daemon down' } else { 'no batch' }))
    } else {
        $runs = if ($null -ne $b.total) { "$($b.runs)/$($b.total)" } else { "$($b.runs)" }
        $failed = "failed $($b.failed)" + $(if ($b.known) { " ($($b.known) known)" } else { '' })
        $lines.Add((@($b.Name, $b.section, $b.State, "runs $runs", "passed $($b.passed)", $failed) | Where-Object { $_ }) -join '  ')
        $lf = $b.last_failure
        if ($null -ne $lf) {
            $lines.Add("last failure  $($lf.run)  $($lf.why)" + $(if ($lf.issue) { "  [$($lf.issue)]" } else { '' }))
        }
    }
    $lines.Add('')

    $cols = @('INSTANCE', 'CLIENT', 'LEASE', 'RUN', 'ROW', 'SHOT', 'LAST')
    $cells = @(, $cols) + @(foreach ($i in $Snapshot.instances) {
        , @($i.instance, $(if ($null -ne $i.client_pid) { "$($i.client_pid)" } else { '-' }), $(if ($i.lease) { $i.lease } else { '-' }),
            $i.run, (Format-LabWatchRowCell $i), $(if ($null -ne $i.shot_s) { "$($i.shot_s)s" } else { '-' }),
            $(if ($i.last) { $i.last } else { '-' }))
    })
    $widths = foreach ($c in 0..($cols.Count - 2)) { ($cells | ForEach-Object { "$($_[$c])".Length } | Measure-Object -Maximum).Maximum + 2 }
    foreach ($row in $cells) {
        $text = ''
        for ($c = 0; $c -lt $cols.Count - 1; $c++) { $text += "$($row[$c])".PadRight($widths[$c]) }
        $lines.Add($text + $row[-1])
    }

    $foot = @()
    if (@($Snapshot.orphans).Count) {
        $shown = @($Snapshot.orphans | Select-Object -First 3) -join ', '
        $more = if (@($Snapshot.orphans).Count -gt 3) { " +$(@($Snapshot.orphans).Count - 3)" } else { '' }
        $foot += "orphan SGW.exe: $shown$more (#1342?)"
    }
    if ($null -ne $Snapshot.logs) {
        $rl = $Snapshot.logs.relaunches
        if ($rl.Count) { $foot += 'relaunches: ' + (@($rl.Keys | ForEach-Object { "$_ $($rl[$_])" }) -join ', ') }
        if ($Snapshot.logs.login_retries) { $foot += "login retries: $($Snapshot.logs.login_retries) (#1343)" }
    }
    if ($foot.Count) { $lines.Add(''); $lines.Add($foot -join '  ') }
    if ($Snapshot.old_daemon) { $lines.Add('daemon has no run progress: lab install, then lab restart') }
    if ($Snapshot.daemon -eq 'down' -and $null -ne $b) { $lines.Add('daemon down: showing the batch files only') }

    return @($lines | ForEach-Object { Limit-UatText (Hide-LeaseIds $_) $Width })
}

# The -Once -Json line (D-LW6): keys in contract order, and a key whose value
# is null, empty or zero is left out, except passed and failed.
function ConvertTo-LabWatchJson($Snapshot) {
    $o = [ordered]@{ ok = $true; daemon = $Snapshot.daemon }
    $b = $Snapshot.batch
    if ($null -eq $b) { $o.state = 'none' }
    else {
        $o.batch = $b.Name; $o.state = $b.State
        if ($b.section) { $o.section = $b.section }
        $o.runs = if ($null -ne $b.total) { "$($b.runs)/$($b.total)" } else { "$($b.runs)" }
        $o.passed = $b.passed; $o.failed = $b.failed
        if ($b.known) { $o.known = $b.known }
        $lf = $b.last_failure
        if ($null -ne $lf) {
            $f = [ordered]@{ run = $lf.run; why = $lf.why }
            if ($lf.issue) { $f.issue = $lf.issue }
            $o.last_failure = $f
        }
        $lanes = [ordered]@{}
        foreach ($i in $Snapshot.instances) {
            if ($i.run -eq '-') { continue }
            $parts = @($i.run)
            $rowCell = Format-LabWatchRowCell $i
            if ($rowCell -ne '-') { $parts += $rowCell }
            if ($null -ne $i.shot_s) { $parts += "shot $($i.shot_s)s" }
            $lanes[$i.instance] = $parts -join ' '
        }
        if ($lanes.Count) { $o.lanes = $lanes }
    }
    if (@($Snapshot.orphans).Count) { $o.orphans = @($Snapshot.orphans | Select-Object -First 3) }
    if ($null -ne $Snapshot.logs) {
        $total = 0; foreach ($v in $Snapshot.logs.relaunches.Values) { $total += $v }
        if ($total) { $o.relaunches = $total }
        if ($Snapshot.logs.login_retries) { $o.login_retries = $Snapshot.logs.login_retries }
    }
    return Hide-LeaseIds ($o | ConvertTo-Json -Depth 5 -Compress)
}
