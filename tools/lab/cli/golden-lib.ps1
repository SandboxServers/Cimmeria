<#
.SYNOPSIS
    Library for `lab golden`: argument rules, run directories from a batch,
    the lab_golden_record reply as an exit code and compact JSON, and the
    committed golden file as a summary. Dot-source it after uat-lib.ps1 (it uses
    Get-UatField); it defines functions only, and all of them are pure, so tools/lab/cli/test-golden.ps1 tests
    them without a daemon.

    Never print the token or a lease id from here.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# The owner's rule is "golden only when five runs agree" (README D-GD6).
$script:GoldenMinRuns = 5
$script:GoldenMaxRuns = 20
$script:GoldenMaxLeases = 5

# The argument checks that need no daemon: an error message, or $null.
function Test-GoldenArguments([string]$Action, [string]$Section, [int]$Runs, [int]$Leases, [string]$ServerVersion) {
    if (@('record', 'show') -notcontains $Action) { return 'usage: lab golden record|show <section> ...' }
    if (-not $Section) { return "usage: lab golden $Action <section>" }
    # The section names a file under golden/, so it may not reach outside it.
    if ($Section -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]*$' -or $Section -match '\.\.') {
        return "section '$Section' is not a section id (letters, digits, '.', '_' and '-')"
    }
    if ($Runs -lt $script:GoldenMinRuns -or $Runs -gt $script:GoldenMaxRuns) {
        return "-Runs must be $script:GoldenMinRuns to $script:GoldenMaxRuns (a golden needs five agreeing runs)"
    }
    if ($Leases -lt 1 -or $Leases -gt $script:GoldenMaxLeases) { return "-Leases must be 1 to $script:GoldenMaxLeases" }
    if ($Runs % $Leases -ne 0) { return "-Runs $Runs does not split evenly over -Leases $Leases" }
    if ($Action -eq 'record' -and $ServerVersion -cnotmatch '^[0-9a-f]{7,40}$') {
        return '-ServerVersion must be the server build''s commit (7 to 40 lowercase hex characters)'
    }
    return $null
}

# The committed golden of a section under a spec folder, named as the daemon
# names it (crates/lab golden_path): evidence.rs path_safe turns every
# character outside [A-Za-z0-9_-] into '_', so v1.2 is golden/v1_2.json.
function Get-GoldenPath([string]$SpecsDir, [string]$Section) {
    return Join-Path (Join-Path $SpecsDir 'golden') "$($Section -creplace '[^A-Za-z0-9_-]', '_').json"
}

# The child `lab uat` arguments for a golden batch: fingerprint runs, Runs
# split evenly over Leases (Test-GoldenArguments refuses an uneven split, and
# so does this), a lane brake of one failed run, progress on stderr and one
# JSON line on stdout. Each value is its own element, so a path with spaces
# stays one argument.
function Get-GoldenBatchArgs([string]$Section, [string[]]$Rows, [int]$Runs, [int]$Leases, [string]$ServerVersion, [string]$SpecsDir) {
    if ($Leases -lt 1 -or $Runs % $Leases -ne 0) { throw "-Runs $Runs does not split evenly over -Leases $Leases" }
    $a = @($Section)
    $rowList = @($Rows | Where-Object { $_ })
    if ($rowList.Count) { $a += '-Rows', ($rowList -join ',') }
    $a += '-Leases', $Leases, '-RunsPerLease', ($Runs / $Leases), '-ServerVersion', $ServerVersion
    if ($SpecsDir) { $a += '-SpecsDir', $SpecsDir }
    $a += '-Fingerprint', '-MaxConsecutiveFailures', 1, '-ProgressToStderr', '-Json'
    return , $a
}

# The RunDir of every batch.jsonl record, in order; $null when there are no
# records, or any record did not pass or has no RunDir (every run must pass).
function Get-GoldenRunDirs($Records) {
    $dirs = @()
    foreach ($r in @($Records)) {
        if ($null -eq $r) { continue }
        $dir = [string](Get-UatField $r 'RunDir')
        if (-not [bool](Get-UatField $r 'Ok') -or -not $dir) { return $null }
        $dirs += $dir
    }
    if (-not $dirs.Count) { return $null }
    return $dirs
}

# The child `lab uat -Json` result as @{ Exit; Error; Batch }. Its exit 0
# gives the batch folder; 2 and 3 pass through with its error; 1 and 4 are
# exit 1 with the first failure group, because every run must pass (D-GD6).
# Lines: the child's stdout; its JSON object is the last line that is one.
function ConvertFrom-GoldenBatchOutput($Lines, [int]$ExitCode) {
    $obj = $null
    foreach ($l in @($Lines)) {
        $t = "$l".Trim()
        if ($t.StartsWith('{')) { try { $obj = $t | ConvertFrom-Json -Depth 20 } catch { } }
    }
    if ($null -eq $obj) {
        return @{ Exit = 3; Error = "lab uat exited $ExitCode without a result"; Batch = $null }
    }
    $batch = [string](Get-UatField $obj 'batch')
    switch ($ExitCode) {
        0 {
            if (-not $batch) { return @{ Exit = 3; Error = 'lab uat passed but named no batch folder'; Batch = $null } }
            return @{ Exit = 0; Error = $null; Batch = $batch }
        }
        { $_ -in 2, 3 } {
            return @{ Exit = $ExitCode; Error = [string](Get-UatField $obj 'error'); Batch = $null }
        }
        { $_ -in 1, 4 } {
            $first = @(Get-UatField $obj 'failures')[0]
            $why = if ($first) {
                $runs = @(Get-UatField $first 'runs') -join ', '
                "$(Get-UatField $first 'why')$(if ($runs) { " (runs: $runs)" })"
            } else { [string](Get-UatField $obj 'error') }
            $passed = "$(Get-UatField $obj 'passed')/$(Get-UatField $obj 'runs')"
            $msg = "every run must pass to record a golden; $passed passed, first failure: $why"
            if ($batch) { $msg += "; $batch" }
            return @{ Exit = 1; Error = $msg; Batch = $batch }
        }
        default {
            return @{ Exit = 3; Error = "lab uat exited $ExitCode"; Batch = $batch }
        }
    }
}

# lab_golden_record's reply as the command's exit code (README Review outcomes, GD-07).
function ConvertTo-GoldenExit($Reply) {
    if ([bool](Get-UatField $Reply 'ok')) { return 0 }
    switch ([string](Get-UatField $Reply 'code')) {
        'exists' { return 2 }
        'disagree' { return 6 }
        'not_passing' { return 1 }
        'cannot_record' { return 7 }
        default { return 7 }
    }
}

# The reply as one compact JSON object with no nulls: on success
# {ok, exit, path, rows, events, bytes, runs}, else {ok, exit, code, error,
# batch}: a refusal after the runs still names the batch folder they are in.
function ConvertTo-GoldenCompactJson($Reply, [int]$Exit, [string]$Batch) {
    $out = [ordered]@{ ok = ($Exit -eq 0); exit = $Exit }
    $keys = if ($Exit -eq 0) { 'path', 'rows', 'events', 'bytes', 'runs' } else { 'code', 'error' }
    foreach ($k in $keys) {
        $v = Get-UatField $Reply $k
        if ($null -ne $v -and "$v" -ne '') { $out[$k] = $v }
    }
    if ($Exit -ne 0 -and $Batch) { $out.batch = $Batch }
    return $out | ConvertTo-Json -Depth 4 -Compress
}

# The one line `lab golden record` prints on success.
function Format-GoldenRecordLine([string]$Section, $Reply) {
    $f = { param($k) Get-UatField $Reply $k }
    return "golden ${Section}: $(& $f 'rows') rows, $(& $f 'events') events, $(& $f 'bytes') bytes from $(& $f 'runs') runs; $(& $f 'path')"
}

# A committed golden (the parsed JSON) as what `lab golden show` prints:
# section, rows, total ordered events, build, recorded time and evidence run ids.
function Get-GoldenSummary($Golden) {
    $fpRows = @(Get-UatField (Get-UatField $Golden 'fingerprint') 'rows')
    $events = 0
    foreach ($r in $fpRows) { if ($null -ne $r) { $events += @(Get-UatField $r 'events' | Where-Object { $null -ne $_ }).Count } }
    $build = Get-UatField $Golden 'build'
    $out = [ordered]@{
        section = [string](Get-UatField $Golden 'section')
        rows = @(Get-UatField $Golden 'rows' | Where-Object { $_ })
        events = $events
        server = [string](Get-UatField $build 'server')
    }
    $sgw = [string](Get-UatField $build 'sgw_sha256')
    if ($sgw) { $out.sgw_sha256 = $sgw }
    $recorded = [string](Get-UatField $Golden 'recorded_utc')
    if ($recorded) { $out.recorded = $recorded }
    $out.runs = @(@(Get-UatField $Golden 'evidence') | Where-Object { $null -ne $_ } |
        ForEach-Object { [string](Get-UatField $_ 'run_id') })
    return $out
}

# The summary as the lines `lab golden show` prints for people.
function Format-GoldenSummary($Summary) {
    $lines = @(
        "golden $($Summary.section): $(@($Summary.rows).Count) rows, $($Summary.events) events"
        "rows:   $(@($Summary.rows) -join ', ')"
        "server: $($Summary.server)"
    )
    if ($Summary.Contains('sgw_sha256')) { $lines += "sgw:    $($Summary.sgw_sha256)" }
    if ($Summary.Contains('recorded')) { $lines += "when:   $($Summary.recorded)" }
    $lines += "runs:   $(@($Summary.runs) -join ', ')"
    return $lines -join "`n"
}
