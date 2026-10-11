<#
.SYNOPSIS
    Library for `lab calibrate` and `lab uat -Heal` (lab-spec-vocab SV-12):
    check the arguments, format lab_uat_calibrate's outcomes, apply a
    proposed diff, and heal a batch's failed poses. Dot-source it after
    uat-lib.ps1; it defines functions only.

.DESCRIPTION
    lab_uat_calibrate replies { run_dir, outcomes: [{ ok, section, row,
    pose, score?, confirmed, changed?, diff_path?, probes, reason? }] }.
    `changed` holds only the fields that differ, as { field: [old, new] }
    with "none" for an unset value; `diff_path` is on the first outcome that
    proposed (the one diff carries every proposal). The tool writes its own
    progress lines to calibrate.jsonl in the run folder.

    Everything here except the MCP call is pure or takes the call as a
    scriptblock, so tools/lab/cli/test-calibrate.ps1 and test-uat.ps1 test
    it without a daemon. Nothing here commits or stages: a proposal is a
    file, and -Apply only runs `git apply` on a spec with no local edits.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$script:CalibrateConfirmRuns = 3
$script:CalibrateMaxBudget = 30
# The spec folder, from the repo root: a diff for any other path is not this checkout's to apply.
$script:CalibrateSpecPrefix = 'docs/guides/uat-specs/'

# The argument checks that need no daemon: an error message, or $null.
# BudgetMinutes is $null when not given (the tool's default, 15).
function Test-CalibrateArguments([string]$Section, [string]$Row, $BudgetMinutes) {
    if (-not $Section -or -not $Row) { return 'usage: lab calibrate <section> <row> [-Pose <id>] [-Instance <name>] [-Prepare <row ids>] [-BudgetMinutes 1-30] [-Apply] [-Json] [-Quiet]' }
    if ($null -ne $BudgetMinutes -and ($BudgetMinutes -lt 1 -or $BudgetMinutes -gt $script:CalibrateMaxBudget)) {
        return "-BudgetMinutes must be 1 to $script:CalibrateMaxBudget"
    }
    return $null
}

# A changed field's short name for the line: bearing_deg -> bearing.
function Get-CalibrateFieldLabel([string]$Name) {
    switch ($Name) {
        'radius_m' { return 'radius' }
        'bearing_deg' { return 'bearing' }
        'pitch_deg' { return 'pitch' }
        'yaw_offset_deg' { return 'yaw' }
        default { return $Name }
    }
}

# One value of a changed pair: a number with at least one decimal (330.0),
# or the tool's "none" for a field the pose did not set.
function Format-CalibrateValue($Value) {
    if ($Value -is [string]) { return $Value }
    return ([double]$Value).ToString('0.0##', [Globalization.CultureInfo]::InvariantCulture)
}

# The changed fields of an outcome as label -> "old -> new", in the tool's order.
function Get-CalibrateChanges($Outcome) {
    $out = [ordered]@{}
    $changed = Get-UatField $Outcome 'changed'
    if ($null -eq $changed) { return $out }
    $names = if ($changed -is [System.Collections.IDictionary]) { @($changed.Keys) } else { @($changed.PSObject.Properties.Name) }
    foreach ($n in $names) {
        $pair = @(Get-UatField $changed $n)
        if ($pair.Count -ne 2) { continue }
        $out[(Get-CalibrateFieldLabel $n)] = "$(Format-CalibrateValue $pair[0]) -> $(Format-CalibrateValue $pair[1])"
    }
    return $out
}

# The one line lab calibrate prints for a pose:
#   frost: score 8312, confirmed 3/3, bearing 322.5 -> 330.0; diff <path>
#   frost: score 8312, confirmed 3/3, unchanged; diff <path>
#   frost: no proposal: <reason>
function Format-CalibrateLine($Outcome) {
    $pose = [string](Get-UatField $Outcome 'pose')
    $score = Get-UatField $Outcome 'score'
    $confirmed = "confirmed $([int](Get-UatField $Outcome 'confirmed'))/$script:CalibrateConfirmRuns"
    $reason = Hide-LeaseIds ([string](Get-UatField $Outcome 'reason'))
    if (-not [bool](Get-UatField $Outcome 'ok')) {
        $line = "${pose}: no proposal"
        if ($reason) { $line += ": $reason" }
        if ($null -ne $score) { $line += " (best score $score, $confirmed)" }
        return $line
    }
    $parts = @()
    if ($null -ne $score) { $parts += "score $score" }
    $parts += $confirmed
    $changes = Get-CalibrateChanges $Outcome
    if ($changes.Count) { foreach ($k in $changes.Keys) { $parts += "$k $($changes[$k])" } }
    else { $parts += 'unchanged' }
    $line = "${pose}: $($parts -join ', ')"
    if ($reason) { $line += " ($reason)" }
    $diff = [string](Get-UatField $Outcome 'diff_path')
    if ($diff) { $line += "; diff $diff" }
    return $line
}

# The exit code of a calibrate call: 0 every pose has a proposal (or is
# unchanged), 1 a pose found none (or there were no outcomes).
function Get-CalibrateExitCode($Outcomes) {
    $all = @(@($Outcomes) | Where-Object { $null -ne $_ })
    if (-not $all.Count) { return 1 }
    if (@($all | Where-Object { -not [bool](Get-UatField $_ 'ok') }).Count) { return 1 }
    return 0
}

# The D-SV12 object for -Json: { ok, section, row, outcomes: [{ pose, ok,
# score, confirmed, changed, diff, reason }] }, compact, no nulls and no
# empty arrays, the candidate list never (it is calibrate.jsonl). changed
# is { label: "old -> new" } with only the fields that differ; a reason
# is cut to 40 characters (the full text is on the human line), so two
# outcomes with a long run folder stay under about 400.
function ConvertTo-CalibrateCompactJson([string]$Section, [string]$Row, $Outcomes, [bool]$Applied = $false) {
    $all = @(@($Outcomes) | Where-Object { $null -ne $_ })
    $list = foreach ($o in $all) {
        $e = [ordered]@{ pose = [string](Get-UatField $o 'pose'); ok = [bool](Get-UatField $o 'ok') }
        $score = Get-UatField $o 'score'
        if ($null -ne $score) { $e.score = [int]$score }
        $e.confirmed = [int](Get-UatField $o 'confirmed')
        $changes = Get-CalibrateChanges $o
        if ($changes.Count) { $e.changed = $changes }
        $diff = [string](Get-UatField $o 'diff_path')
        if ($diff) { $e.diff = $diff }
        $reason = Hide-LeaseIds ([string](Get-UatField $o 'reason'))
        if ($reason) { $e.reason = Limit-UatText ($reason -replace '[\r\n]+', ' ') 40 }
        $e
    }
    $out = [ordered]@{ ok = ((Get-CalibrateExitCode $all) -eq 0); section = $Section; row = $Row }
    if ($all.Count) { $out.outcomes = @($list) }
    if ($Applied) { $out.applied = $true }
    return $out | ConvertTo-Json -Depth 6 -Compress
}

# The repo-relative file a proposed diff applies to (its `+++ b/<path>`
# line), or $null when it names none.
function Get-CalibrateDiffTarget([string]$DiffText) {
    foreach ($l in ($DiffText -split "`r?`n")) {
        if ($l -match '^\+\+\+ b/(.+)$') { return $Matches[1].Trim() }
    }
    return $null
}

# Whether a repo-relative file has local edits, staged or not (git diff
# HEAD): $true when it has, or when git cannot tell.
function Test-CalibrateSpecDirty([string]$Repo, [string]$RelPath) {
    & git -C $Repo diff --quiet HEAD -- $RelPath 2>$null
    return ($LASTEXITCODE -ne 0)
}

# -Apply: `git apply` the diff in the checkout, unless its spec has local
# edits (they would be clobbered or the apply would half-land). Never adds
# or commits. Returns @{ Ok; Error; Target }.
function Invoke-CalibrateApply([string]$Repo, [string]$DiffPath) {
    $fail = { param($m, $t) @{ Ok = $false; Error = $m; Target = $t } }
    if (-not $DiffPath -or -not (Test-Path -LiteralPath $DiffPath)) { return & $fail 'no diff was proposed; nothing to apply' $null }
    $target = Get-CalibrateDiffTarget (Get-Content -LiteralPath $DiffPath -Raw)
    if (-not $target) { return & $fail "the diff $DiffPath names no file" $null }
    if (-not $target.StartsWith($script:CalibrateSpecPrefix)) {
        return & $fail "the diff is for $target, not this checkout's $script:CalibrateSpecPrefix; apply it by hand" $target
    }
    if (-not (Test-Path -LiteralPath (Join-Path $Repo $target))) { return & $fail "$target is not in $Repo" $target }
    if (Test-CalibrateSpecDirty $Repo $target) {
        return & $fail "$target has uncommitted changes; commit or revert them, then: git -C `"$Repo`" apply `"$DiffPath`"" $target
    }
    $msg = & git -C $Repo apply $DiffPath 2>&1
    if ($LASTEXITCODE -ne 0) { return & $fail "git apply failed: $((@($msg) | Select-Object -First 1))" $target }
    return @{ Ok = $true; Error = $null; Target = $target }
}

# A failed lab_uat_calibrate call as one line, named once: Invoke-McpTool's
# own errors already start with the tool's name, a timeout does not.
function Format-CalibrateCallError([string]$Message) {
    $m = Hide-LeaseIds (($Message -split "`n")[0])
    if ($m.StartsWith('lab_uat_calibrate')) { return $m }
    return "lab_uat_calibrate failed: $m"
}

# The repo root this CLI runs from (tools/lab/cli/../../..).
function Get-CalibrateRepoRoot {
    return (Resolve-Path -LiteralPath (Join-Path $PSScriptRoot '..\..\..')).Path
}

# One lab_uat_calibrate call over the daemon's MCP endpoint: the reply as an
# object. TimeoutSec covers the sweep's budget plus its confirmation grace.
function Invoke-LabCalibrate([string]$Url, [string]$Token, [hashtable]$Arguments, [int]$TimeoutSec) {
    $s = New-McpSession $Url $Token
    try { return Invoke-McpTool $s 'lab_uat_calibrate' $Arguments $TimeoutSec }
    finally { Close-McpSession $s }
}

# The seconds a calibrate call may take: the budget (15 by default) plus
# 5 minutes for the confirmation grace and the stand-offs around it.
function Get-CalibrateTimeoutSec($BudgetMinutes) {
    $m = if ($null -ne $BudgetMinutes) { [int]$BudgetMinutes } else { 15 }
    return ($m + 5) * 60
}

# The batch folder file a heal proposal is saved as: heal-<row>-<pose>.diff,
# with anything but letters, digits, '.', '_' and '-' made '_'.
function Get-UatHealFileName([string]$Row, [string]$Pose) {
    $safe = { param($s) $s -replace '[^A-Za-z0-9._-]', '_' }
    return "heal-$(& $safe $Row)-$(& $safe $Pose).diff"
}

# lab uat -Heal (D-SV8 (a)): for each (row, pose) hint, one
# lab_uat_calibrate call for that pose on the batch's instance; a proposed
# diff is copied into the batch folder as heal-<row>-<pose>.diff. The spec
# file is never written. $Invoke is the call, { param($Arguments) <reply> },
# so uat.ps1 passes the MCP call and the tests a stub. Returns one record
# per hint: @{ Row; Pose; Diff (the batch copy, or $null); Line; Text }.
function Invoke-UatHeal($Hints, [string]$BatchDir, [string]$Section, [string]$SpecsDir, [string]$Instance, [scriptblock]$Invoke) {
    $results = @()
    foreach ($h in @($Hints | Where-Object { $null -ne $_ })) {
        $row = [string]$h.row; $pose = [string]$h.pose
        $a = @{ section = $Section; row = $row; pose = $pose }
        if ($Instance) { $a.instance = $Instance }
        if ($SpecsDir) { $a.specs_dir = $SpecsDir }
        $rec = @{ Row = $row; Pose = $pose; Diff = $null; Line = $null; Text = $null }
        try {
            $reply = & $Invoke $a
            # Select-Object, not [0]: strict mode throws on an index into an empty list.
            $outcome = @(Get-UatField $reply 'outcomes') | Where-Object { [string](Get-UatField $_ 'pose') -ceq $pose } | Select-Object -First 1
            if ($null -eq $outcome) {
                $rec.Line = "heal $row ${pose}: lab_uat_calibrate returned no outcome for the pose"
            } else {
                $src = [string](Get-UatField $outcome 'diff_path')
                if ([bool](Get-UatField $outcome 'ok') -and $src -and (Test-Path -LiteralPath $src)) {
                    $dest = Join-Path $BatchDir (Get-UatHealFileName $row $pose)
                    Copy-Item -LiteralPath $src -Destination $dest -Force
                    $rec.Diff = $dest
                    $rec.Text = Get-Content -LiteralPath $dest -Raw
                    # The line names the batch copy, the one the summary lists.
                    $shown = [pscustomobject]@{}
                    foreach ($k in @('ok', 'pose', 'score', 'confirmed', 'changed', 'reason')) {
                        $v = Get-UatField $outcome $k
                        if ($null -ne $v) { $shown | Add-Member -NotePropertyName $k -NotePropertyValue $v }
                    }
                    $shown | Add-Member -NotePropertyName 'diff_path' -NotePropertyValue $dest
                    $rec.Line = "heal $row $(Format-CalibrateLine $shown)"
                } else {
                    $rec.Line = "heal $row $(Format-CalibrateLine $outcome)"
                }
            }
        } catch {
            $rec.Line = "heal $row ${pose}: $(Format-CalibrateCallError $_.Exception.Message)"
        }
        $results += $rec
    }
    return $results
}
