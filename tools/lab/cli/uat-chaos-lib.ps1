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
    # Several runs differ only by seed: name the seed once as <per run> (each
    # run's own is in batch.jsonl and its divergence ref), which keeps -Json
    # inside its budget with 20-digit seeds.
    $norm = @($seen | ForEach-Object { $_ -replace 'seed=\d+', 'seed=<per run>' } | Select-Object -Unique)
    if ($norm.Count -eq 1) { return $norm[0] }
    return "$($norm[0]) (+$($norm.Count - 1) other spec(s) in batch.jsonl)"
}

# Attaches the chaos header to a run summary, so -Json and summary.md carry it.
function Add-UatChaosHeader($Summary, [string]$Header) {
    if ($Header) { $Summary['chaos'] = $Header }
}

# The runs of a batch that ran under chaos: those carrying the runner's
# header. The rest are compared under golden as usual.
function Select-UatChaosRuns($Runs, [bool]$Chaos) {
    return @(@($Runs) | Where-Object { $null -ne $_ -and [bool](Get-UatField $_ 'Chaos') -eq $Chaos })
}

# A chaos run's ref in a divergence group, "<instance>#<index> seed=<seed>",
# so summary.md and batch.json name the run that replays (its full header is
# its batch.jsonl record). The seeds are unique per run, so one is shown when
# every spec has the same; a header without a seed is shown whole.
function Get-UatChaosRunRef($Run) {
    $ref = "$(Get-UatField $Run 'Instance')#$(Get-UatField $Run 'Index')"
    $header = [string](Get-UatField $Run 'Chaos')
    $seeds = @([regex]::Matches($header, 'seed=\S+') | ForEach-Object { $_.Value.TrimEnd(';') } | Select-Object -Unique)
    if ($seeds.Count) { return "$ref $($seeds -join ',')" }
    if ($header) { return "$ref ($header)" }
    return $ref
}

# The golden comparison of a batch's chaos runs (D-CH7), reported as
# chaos_divergence in place of golden: Merge-UatGolden's shape and grouping,
# on the same unprefixed lines, so five runs with one divergence are one
# group of five. The batch's header is the top-level chaos key; each group's
# runs carry their seeds. $null only when no chaos run was compared; a batch
# whose compared runs all matched gives {match: n, diverged: 0}, as golden
# does, so "compared and clean" differs from "never compared".
function Merge-UatChaosDivergence($Runs) {
    return Merge-UatGolden (Select-UatChaosRuns $Runs $true) { param($r) Get-UatChaosRunRef $r }
}
