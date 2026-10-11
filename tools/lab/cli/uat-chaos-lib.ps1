<#
.SYNOPSIS
    The -Chaos helpers of `lab uat` (lab-chaos CH-02, CH-08, CH-09): the
    header, the request body, the cpu and -Leases check, and the chaos
    divergence from the golden. uat-lib.ps1 dot-sources it; it defines
    functions only, and needs uat-lib.ps1's Get-UatField and Merge-UatGolden.
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# -Chaos values as the run's header: one chaos=<spec> per value, joined with
# "; ". Null when there was no chaos (no key in -Json, no line in the summary).
# Values are not split on commas: a multi-knob spec is one value.
function Format-UatChaosHeader([string[]]$Chaos) {
    $specs = @($Chaos | Where-Object { $_ })
    if (-not $specs.Count) { return $null }
    return (@($specs | ForEach-Object { "chaos=$_" }) -join '; ')
}

# Puts the non-empty -Chaos values on the lab_uat_run arguments and returns
# the header. One filter for both, so the body and the header always agree.
# This header is the specs as typed (no resolved level or seed); the caller
# replaces it with the runner's own (Get-UatRunChaos, Select-UatChaosHeader).
function Set-UatChaos([hashtable]$BaseArgs, [string[]]$Chaos) {
    $specs = @($Chaos | Where-Object { $_ })
    if (-not $specs.Count) { return $null }
    $BaseArgs.chaos = $specs
    return Format-UatChaosHeader $specs
}

# cpu chaos loads the whole machine, and each lane's lab_uat_run starts its
# own load, so -Leases 2 would double it: cpu runs on one lease. The message,
# or $null when the combination is fine.
function Test-UatChaosLeases([string[]]$Chaos, [int]$Leases) {
    $cpu = @($Chaos | Where-Object { $_ -and ($_.Trim() -split ':')[0] -eq 'cpu' })
    if ($cpu.Count -and $Leases -gt 1) {
        return "-Chaos cpu loads the whole machine and each lease's run would start its own load; run it with -Leases 1"
    }
    return $null
}

# The runner's own chaos header from a lab_uat_run result (lab-chaos CH-08):
# its `chaos` list (one header per spec, seed resolved) joined with "; ".
# $null when the result carries none.
function Get-UatRunChaos($Result) {
    $headers = @(Get-UatField $Result 'chaos' | Where-Object { $_ })
    if (-not $headers.Count) { return $null }
    return ($headers -join '; ')
}

# The batch's header: the runs' own headers replace the typed one ($Fallback,
# kept only when no run returned one). Runs differ in seed (each run's seed
# is its run id's hash), so after the first the rest are counted, not listed.
function Select-UatChaosHeader($Runs, [string]$Fallback) {
    $seen = @(@($Runs) | ForEach-Object { [string](Get-UatField $_ 'Chaos') } | Where-Object { $_ } | Select-Object -Unique)
    if (-not $seen.Count) { return $(if ($Fallback) { $Fallback } else { $null }) }
    if ($seen.Count -eq 1) { return $seen[0] }
    return "$($seen[0]) (+$($seen.Count - 1) other seed(s) in batch.jsonl)"
}

# Attaches the chaos header to a run summary, so -Json and summary.md carry it.
function Add-UatChaosHeader($Summary, [string]$Header) {
    if ($Header) { $Summary['chaos'] = $Header }
}

# Whether a batch ran under chaos: some run carries the runner's header.
function Test-UatChaosRuns($Runs) {
    return [bool]@(@($Runs) | Where-Object { $null -ne $_ -and (Get-UatField $_ 'Chaos') }).Count
}

# The golden comparison of a chaos batch (D-CH7), reported as
# chaos_divergence in place of golden: the shape of Merge-UatGolden, with
# each divergence line led by its own run's chaos header (seed included, so
# the line names a run that replays). An error line, a diff that could not
# be made, stays as it is. $null when every compared run matched or none
# was compared: a chaos run that matches the golden has nothing to report.
function Merge-UatChaosDivergence($Runs) {
    $tagged = foreach ($run in @(@($Runs) | Where-Object { Get-UatField $_ 'Golden' })) {
        $line = [string](Get-UatField $run 'Golden')
        $header = [string](Get-UatField $run 'Chaos')
        if ($header -and $line -cne 'match' -and -not $line.StartsWith('error: ')) { $line = "$header $line" }
        [ordered]@{ Instance = Get-UatField $run 'Instance'; Index = Get-UatField $run 'Index'; Golden = $line }
    }
    $merged = Merge-UatGolden @($tagged)
    if (-not $merged -or ($merged.diverged + $merged.errors) -eq 0) { return $null }
    return $merged
}
