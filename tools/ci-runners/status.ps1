#!/usr/bin/env pwsh
# Cache and disk report for the self-hosted runners, from each runner's
# state/jobs.jsonl (written by hooks/job-completed.sh) and the node's volumes.
#
#   pwsh tools/ci-runners/status.ps1            # last 10 jobs per node
#   pwsh tools/ci-runners/status.ps1 -Last 30
[CmdletBinding()]
param(
    [string[]]$Nodes = @('node5', 'node2'),
    [int]$Last = 10
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$targets = @{
    node5 = @{ Ssh = 'cimmeria-colo'; Dir = '/mnt/nvme/gha-runners' }
    node2 = @{ Ssh = 'cimmeria-node2'; Dir = '/var/lib/gha-runners' }
}

foreach ($node in $Nodes) {
    $t = $targets[$node]
    Write-Host "== $node" -ForegroundColor Cyan

    # Every runner's job log, newest last. The state volume is read from inside
    # each runner container, which owns it.
    $raw = ssh $t.Ssh "cd $($t.Dir) && for i in 1 2 3 4 5; do docker compose exec -T runner`$i cat /home/runner/state/jobs.jsonl 2>/dev/null; done"
    $jobs = @($raw | Where-Object { $_ -match '^\{' } | ForEach-Object { $_ | ConvertFrom-Json } | Sort-Object at | Select-Object -Last $Last)
    if ($jobs.Count -eq 0) {
        Write-Host '  no jobs recorded yet'
    } else {
        $jobs | ForEach-Object {
            $cacheable = $_.hits + $_.misses
            [pscustomobject]@{
                At        = ([datetime]$_.at).ToString('MM-dd HH:mm')
                Runner    = $_.runner
                Job       = $_.job
                Requests  = $_.requests
                'Hit %'   = if ($cacheable) { [math]::Round(100 * $_.hits / $cacheable) } else { '-' }
                'Target GB' = [math]::Round($_.target_kb / 1MB, 1)
                'sccache GB' = [math]::Round($_.sccache_kb / 1MB, 1)
                'Free GB' = [math]::Round($_.work_free_kb / 1MB)
            }
        } | Format-Table -AutoSize | Out-String -Width 200 | Write-Host
    }

    # What the runner volumes hold right now, summed by kind.
    $vols = ssh $t.Ssh "docker system df -v --format '{{range .Volumes}}{{.Name}} {{.Size}}{{println}}{{end}}' | grep '^gha-runners_'"
    $byKind = @{}
    foreach ($line in $vols) {
        if ($line -notmatch '^gha-runners_([a-z-]+?)\d+ (\S+)$') { continue }
        $kind = $Matches[1]; $size = $Matches[2]
        $bytes = if ($size -match '^([\d.]+)([kMGT]?B)$') {
            [double]$Matches[1] * @{ B = 1; kB = 1e3; MB = 1e6; GB = 1e9; TB = 1e12 }[$Matches[2]]
        } else { 0 }
        $byKind[$kind] = ($byKind[$kind] ?? 0) + $bytes
    }
    $total = ($byKind.Values | Measure-Object -Sum).Sum
    $line = ($byKind.GetEnumerator() | Sort-Object Name | ForEach-Object { '{0} {1:N1} GB' -f $_.Name, ($_.Value / 1e9) }) -join ', '
    Write-Host ("  volumes: {0} (total {1:N1} GB)" -f $line, ($total / 1e9))
    $free = ssh $t.Ssh 'df -h --output=avail,pcent "$(docker info -f "{{.DockerRootDir}}")" | tail -1'
    Write-Host "  docker root free/used: $($free.Trim())"
    Write-Host ''
}
