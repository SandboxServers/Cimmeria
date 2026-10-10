<#
.SYNOPSIS
    Library for `lab uat`: plan the lanes, call lab_uat_run on the daemon,
    classify and aggregate the runs. Dot-source it; it defines functions only.

.DESCRIPTION
    A batch is Leases lanes (one per hosted lab instance, in the daemon's
    order) that each run the same spec rows RunsPerLease times, one after the
    other. Each run is one lab_uat_run call over the daemon's MCP endpoint and
    takes and releases its own lease. Everything here except the MCP calls is
    pure, so tools/lab/cli/test-uat.ps1 tests it without a daemon.

    Never print the token or a lease id from here.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:UatMaxLeases = 5
$script:UatMaxRunsPerLease = 20

# Failures the lab already knows. A run is tagged with the first entry whose
# row and reason match its first failing row; later rows usually fail as a
# consequence and are not classified separately.
$script:UatKnownIssues = @(
    @{ Issue = '#1341'; Rows = '^FS-[PS]2$'; Reason = 'movie-over'
       Note = 'client dropped the first-login entity batch (iterator residue)' }
)

# A property of an object, or $null when it has none (strict mode throws on a missing one).
function Get-UatField($Object, [string]$Name) {
    if ($null -eq $Object) { return $null }
    if ($Object -is [hashtable]) { return $Object[$Name] }
    if ($Object.PSObject.Properties.Name -contains $Name) { return $Object.$Name }
    return $null
}

# Validates the batch shape and picks the instances. Returns @{ Lanes; Error }:
# Lanes is a list of @{ Lane (1-based); Instance }, Error a message or $null.
# Hosted: the daemon's instance labels in order. Leased: labels a session holds.
# Instance: run a single lane on this label (Leases must then be 1).
function Get-UatPlan([int]$Leases, [int]$RunsPerLease, [string[]]$Hosted, [string[]]$Leased,
                     [int]$MaxClients, [string]$Instance) {
    $fail = { param($m) @{ Lanes = @(); Error = $m } }
    if ($Leases -lt 1 -or $Leases -gt $script:UatMaxLeases) { return & $fail "-Leases must be 1 to $script:UatMaxLeases" }
    if ($RunsPerLease -lt 1 -or $RunsPerLease -gt $script:UatMaxRunsPerLease) {
        return & $fail "-RunsPerLease must be 1 to $script:UatMaxRunsPerLease"
    }
    $Hosted = @($Hosted | Where-Object { $_ })
    $Leased = @($Leased | Where-Object { $_ })
    if ($Instance) {
        if ($Leases -ne 1) { return & $fail '-Instance runs one lane; drop -Leases or -Instance' }
        if ($Hosted -notcontains $Instance) { return & $fail "unknown instance '$Instance'; the daemon hosts: $($Hosted -join ', ')" }
        if ($Leased -contains $Instance) { return & $fail "instance $Instance is leased by another session; wait for it or pick another" }
        return @{ Lanes = @(@{ Lane = 1; Instance = $Instance }); Error = $null }
    }
    $cap = [math]::Min($Hosted.Count, $(if ($MaxClients -gt 0) { $MaxClients } else { $Hosted.Count }))
    if ($Leases -gt $cap) {
        return & $fail "-Leases $Leases is more than the lab allows (${cap}: $($Hosted.Count) hosted, max clients $MaxClients)"
    }
    $free = @($Hosted | Where-Object { $Leased -notcontains $_ })
    if ($free.Count -lt $Leases) {
        $busy = @($Hosted | Where-Object { $Leased -contains $_ }) -join ', '
        return & $fail "only $($free.Count) free instance(s) for $Leases lease(s); leased by other sessions: $busy"
    }
    $lanes = for ($i = 0; $i -lt $Leases; $i++) { @{ Lane = $i + 1; Instance = $free[$i] } }
    return @{ Lanes = @($lanes); Error = $null }
}

# The JSON-RPC body of a streamable-HTTP MCP reply: plain JSON or the last
# SSE "data:" line. $null for an empty body (a notification's 202).
function Read-McpBody([string]$Text) {
    if (-not $Text -or -not $Text.Trim()) { return $null }
    $t = $Text.TrimStart()
    if ($t.StartsWith('{')) { return $t | ConvertFrom-Json -Depth 50 }
    $line = $Text -split "`n" | Where-Object { $_ -like 'data:*' } | Select-Object -Last 1
    if (-not $line) { return $null }
    return $line.Substring(5).Trim() | ConvertFrom-Json -Depth 50
}

# Opens an MCP session on the daemon: initialize, then the initialized notice.
# Returns the session (Url, Headers) for Invoke-McpTool.
function New-McpSession([string]$Url, [string]$Token) {
    $s = @{ Url = $Url; Headers = @{ Authorization = "Bearer $Token"; Accept = 'application/json, text/event-stream' } }
    $null = Send-McpRequest $s @{ jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{
        protocolVersion = '2025-06-18'; capabilities = @{}; clientInfo = @{ name = 'lab-uat'; version = '1' } } } 30
    $null = Send-McpRequest $s @{ jsonrpc = '2.0'; method = 'notifications/initialized' } 30
    return $s
}

function Send-McpRequest($Session, [hashtable]$Body, [int]$TimeoutSec) {
    $r = Invoke-WebRequest -Uri $Session.Url -Method Post -Headers $Session.Headers -ContentType 'application/json' `
        -Body ($Body | ConvertTo-Json -Depth 10 -Compress) -TimeoutSec $TimeoutSec
    $sid = $r.Headers['Mcp-Session-Id']
    if ($sid) { $Session.Headers['Mcp-Session-Id'] = [string]$sid }
    $text = if ($r.Content -is [byte[]]) { [Text.Encoding]::UTF8.GetString($r.Content) } else { [string]$r.Content }
    return Read-McpBody $text
}

# Calls one tool and returns its first text content parsed as JSON. Throws
# with the tool's own message on an MCP or tool error.
function Invoke-McpTool($Session, [string]$Name, [hashtable]$Arguments, [int]$TimeoutSec) {
    $resp = Send-McpRequest $Session @{ jsonrpc = '2.0'; id = 2; method = 'tools/call'
        params = @{ name = $Name; arguments = $Arguments } } $TimeoutSec
    if ($null -eq $resp) { throw "$Name returned nothing" }
    $err = Get-UatField $resp 'error'
    if ($err) { throw "$Name failed: $(Get-UatField $err 'message')" }
    $result = Get-UatField $resp 'result'
    $text = [string](@(Get-UatField $result 'content')[0].text)
    if (Get-UatField $result 'isError') { throw "$Name failed: $text" }
    return $text | ConvertFrom-Json -Depth 50
}

# One run's verdict from lab_uat_run's rows: Ok (every row PASS, or SKIPPED on a
# plan), the first failing row and reason, and the known issue it matches.
function Get-UatRunVerdict($Rows) {
    $first = $null
    foreach ($r in @($Rows)) {
        if (@('PASS', 'SKIPPED') -notcontains [string](Get-UatField $r 'result')) { $first = $r; break }
    }
    if (-not $first) { return @{ Ok = $true; Row = $null; Result = $null; Reason = $null; Issue = $null } }
    $reason = [string](@(Get-UatField $first 'reasons')[0])
    $row = [string](Get-UatField $first 'row')
    $issue = $null
    foreach ($k in $script:UatKnownIssues) {
        if ($row -match $k.Rows -and $reason -match $k.Reason) { $issue = $k.Issue; break }
    }
    return @{ Ok = $false; Row = $row; Result = [string](Get-UatField $first 'result'); Reason = $reason; Issue = $issue }
}

# Whether a lane should stop: after MaxConsecutive failed runs in a row (0 never stops).
function Test-UatBrake([bool[]]$Outcomes, [int]$MaxConsecutive) {
    if ($MaxConsecutive -le 0) { return $false }
    $streak = 0
    foreach ($ok in $Outcomes) { if ($ok) { $streak = 0 } else { $streak++ } }
    return $streak -ge $MaxConsecutive
}

# Aggregates run records (Lane, Instance, Index, Ok, Rows, Verdict, RunDir, Error,
# Seconds) into the batch summary: totals, per-row pass counts, per lane, and
# failures grouped by signature (row + reason or the error) with counts.
function Merge-UatResults($Runs, $Lanes, [string[]]$BrakedInstances) {
    $Runs = @($Runs)
    $rowStats = [ordered]@{}
    foreach ($run in $Runs) {
        foreach ($r in @($run.Rows)) {
            $id = [string](Get-UatField $r 'row')
            if (-not $rowStats.Contains($id)) { $rowStats[$id] = @{ pass = 0; total = 0 } }
            $rowStats[$id].total++
            if ([string](Get-UatField $r 'result') -eq 'PASS') { $rowStats[$id].pass++ }
        }
    }
    $groups = [ordered]@{}
    foreach ($run in ($Runs | Where-Object { -not $_.Ok })) {
        $v = $run.Verdict
        $key = if ($run.Error) { "error: $($run.Error)" } else { "$($v.Row) $($v.Result): $($v.Reason)" }
        if (-not $groups.Contains($key)) {
            $groups[$key] = @{ signature = $key; count = 0; issue = $(if ($run.Error) { $null } else { $v.Issue }); runs = @() }
        }
        $groups[$key].count++
        $groups[$key].runs += "$($run.Instance)#$($run.Index)"
    }
    $laneStats = foreach ($l in @($Lanes)) {
        $mine = @($Runs | Where-Object { $_.Instance -eq $l.Instance })
        [ordered]@{
            lane = $l.Lane; instance = $l.Instance; runs = $mine.Count
            passed = @($mine | Where-Object { $_.Ok }).Count
            braked = [bool]($BrakedInstances -contains $l.Instance)
        }
    }
    $passed = @($Runs | Where-Object { $_.Ok }).Count
    $known = @($Runs | Where-Object { -not $_.Ok -and -not $_.Error -and $_.Verdict.Issue }).Count
    return [ordered]@{
        ok = ($Runs.Count -gt 0 -and $passed -eq $Runs.Count)
        runs = $Runs.Count; passed = $passed; failed = $Runs.Count - $passed
        failed_known = $known; failed_new = $Runs.Count - $passed - $known
        rows = $rowStats; lanes = @($laneStats); failures = @($groups.Values)
    }
}

# The exit code: 0 all passed, 1 a run failed, 4 a lane hit the brake.
function Get-UatExitCode($Summary) {
    if ($Summary.ok) { return 0 }
    if (@($Summary.lanes | Where-Object { $_.braked }).Count) { return 4 }
    return 1
}

# The summary as compact JSON for an agent: no nulls, failures capped at
# $MaxFailures groups (with a count of the rest), rows as "pass/total".
function ConvertTo-UatCompactJson($Summary, [string]$BatchDir, [int]$MaxFailures = 8) {
    $rows = [ordered]@{}
    foreach ($k in $Summary.rows.Keys) { $rows[$k] = "$($Summary.rows[$k].pass)/$($Summary.rows[$k].total)" }
    $fails = @($Summary.failures | Sort-Object { $_.count } -Descending)
    $shown = foreach ($f in ($fails | Select-Object -First $MaxFailures)) {
        $o = [ordered]@{ n = $f.count; why = $f.signature }
        if ($f.issue) { $o.issue = $f.issue }
        $o.runs = @($f.runs | Select-Object -First 5)
        $o
    }
    $out = [ordered]@{
        ok = $Summary.ok; runs = $Summary.runs; passed = $Summary.passed; failed = $Summary.failed
    }
    if ($Summary.failed) { $out.failed_known = $Summary.failed_known; $out.failed_new = $Summary.failed_new }
    $out.rows = $rows
    $out.lanes = @($Summary.lanes | ForEach-Object {
        $l = [ordered]@{ instance = $_.instance; passed = "$($_.passed)/$($_.runs)" }
        if ($_.braked) { $l.braked = $true }
        $l
    })
    if ($shown) { $out.failures = @($shown) }
    if ($fails.Count -gt $MaxFailures) { $out.more_failure_groups = $fails.Count - $MaxFailures }
    $out.batch = $BatchDir
    return $out | ConvertTo-Json -Depth 6 -Compress
}

# The summary as Markdown for people (summary.md and the terminal).
function Format-UatSummary($Summary, [string]$Title, [string]$BatchDir) {
    $sb = [System.Text.StringBuilder]::new()
    $verdict = if ($Summary.ok) { 'PASS' } else { 'FAIL' }
    [void]$sb.AppendLine("# ${Title}: $verdict ($($Summary.passed)/$($Summary.runs) runs passed)")
    if ($Summary.failed) {
        [void]$sb.AppendLine("Failed: $($Summary.failed) ($($Summary.failed_known) known issue, $($Summary.failed_new) new)")
    }
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('| Lane | Instance | Passed | Stopped early |')
    [void]$sb.AppendLine('|---|---|---|---|')
    foreach ($l in $Summary.lanes) {
        [void]$sb.AppendLine("| $($l.lane) | $($l.instance) | $($l.passed)/$($l.runs) | $(if ($l.braked) { 'yes' } else { '' }) |")
    }
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine('| Row | Passed |')
    [void]$sb.AppendLine('|---|---|')
    foreach ($k in $Summary.rows.Keys) { [void]$sb.AppendLine("| $k | $($Summary.rows[$k].pass)/$($Summary.rows[$k].total) |") }
    if (@($Summary.failures).Count) {
        [void]$sb.AppendLine('')
        [void]$sb.AppendLine('Failures, grouped by the first failing row:')
        foreach ($f in (@($Summary.failures) | Sort-Object { $_.count } -Descending)) {
            $tag = if ($f.issue) { " (known: $($f.issue))" } else { '' }
            [void]$sb.AppendLine("- $($f.count)x $($f.signature)$tag; runs: $(@($f.runs) -join ', ')")
        }
    }
    [void]$sb.AppendLine('')
    [void]$sb.AppendLine("Batch: $BatchDir")
    return $sb.ToString()
}
