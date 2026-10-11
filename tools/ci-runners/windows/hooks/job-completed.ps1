# Runs after every job, pass or fail (ACTIONS_RUNNER_HOOK_JOB_COMPLETED). The Windows
# twin of hooks/job-completed.sh: it records what the job did with the compile cache,
# then keeps the runner's disk use bounded.
#   - one JSON line per job in <runner root>\state\jobs.jsonl (sccache requests, hits,
#     misses, the backend, target\ and cache sizes, free space; status.ps1 reads it),
#     and the same figures as one line in the job log;
#   - the job's target\ (the bulk: tens of GB) goes now, not at the next checkout;
#   - RUNNER_TEMP is emptied, and the runner's TEMP loses entries older than an hour.
# sccache trims SCCACHE_DIR to SCCACHE_CACHE_SIZE itself.
$ErrorActionPreference = 'Continue'
$root = $env:CIMMERIA_RUNNER_ROOT
$sccache = $env:RUSTC_WRAPPER
$ws = $env:GITHUB_WORKSPACE

function Get-SizeKB([string]$Path) {
    if (-not $Path -or -not (Test-Path -LiteralPath $Path)) { return 0 }
    $sum = (Get-ChildItem -LiteralPath $Path -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object Length -Sum).Sum
    [long]([double]$sum / 1KB)
}
# Handles paths past MAX_PATH, which deep target\ trees reach.
function Remove-Tree([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return }
    Remove-Item -LiteralPath $Path -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $Path) { cmd.exe /d /c "rd /s /q `"\\?\$Path`"" 2>$null | Out-Null }
}
function Get-Count($Counts) {
    if ($null -eq $Counts) { return 0 }
    $n = 0
    foreach ($p in $Counts.PSObject.Properties) { $n += [long]$p.Value }
    $n
}

$targetKb = if ($ws) { Get-SizeKB "$ws\target" } else { 0 }

# Stats are per sccache server; stopping it resets them for the next job.
$s = $null
if ($sccache) {
    try { $s = ((& $sccache --show-stats --stats-format json 2>$null) -join "`n" | ConvertFrom-Json).stats } catch { }
    & $sccache --stop-server *> $null
}
$backend = 'local'
if ($env:SCCACHE_CONF -and (Test-Path -LiteralPath $env:SCCACHE_CONF) -and ((Get-Content -Raw -LiteralPath $env:SCCACHE_CONF) -match 'cache\.redis')) { $backend = 'redis' }
$hits = 0; $misses = 0; $requests = 0; $nonCacheable = 0
if ($s) {
    $requests = [long]$s.compile_requests
    if ($s.cache_hits) { $hits = Get-Count $s.cache_hits.counts }
    if ($s.cache_misses) { $misses = Get-Count $s.cache_misses.counts }
    $nonCacheable = [long]$s.non_cacheable_compilations
}
$drive = New-Object IO.DriveInfo ([IO.Path]::GetPathRoot($root))
$rec = [ordered]@{
    at = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ss+00:00')
    runner = $env:RUNNER_NAME; job = $env:GITHUB_JOB; run = $env:GITHUB_RUN_ID; ref = $env:GITHUB_REF
    backend = $backend
    requests = $requests; hits = $hits; misses = $misses; non_cacheable = $nonCacheable
    target_kb = $targetKb; sccache_kb = (Get-SizeKB "$root\sccache")
    registry_kb = (Get-SizeKB "$env:CARGO_HOME\registry")
    work_free_kb = [long]($drive.AvailableFreeSpace / 1KB)
}
$cacheable = $hits + $misses
$rate = if ($cacheable) { '{0:N0}%' -f (100 * $hits / $cacheable) } else { '-' }
Write-Host ("sccache ({0}): {1} requests, {2} hits, {3} misses, hit rate {4}, {5} not cacheable; target {6:N1} GB deleted; {7} {8:N0} GB free" -f `
        $backend, $requests, $hits, $misses, $rate, $nonCacheable, ($targetKb / 1MB), $drive.Name, ($drive.AvailableFreeSpace / 1GB))
$log = "$root\state\jobs.jsonl"
try {
    Add-Content -LiteralPath $log -Value ($rec | ConvertTo-Json -Compress)
    # Keep the log itself small.
    $lines = @(Get-Content -LiteralPath $log)
    if ($lines.Count -gt 500) { Set-Content -LiteralPath $log -Value ($lines | Select-Object -Last 500) }
} catch { }

if ($ws -and (Test-Path -LiteralPath $ws)) { Remove-Tree "$ws\target" }
if ($env:RUNNER_TEMP -and (Test-Path -LiteralPath $env:RUNNER_TEMP)) {
    Get-ChildItem -LiteralPath $env:RUNNER_TEMP -Force -ErrorAction SilentlyContinue | ForEach-Object { Remove-Tree $_.FullName }
}
$cutoff = (Get-Date).AddHours(-1)
if ($env:TEMP -and $root -and $env:TEMP.StartsWith($root, [StringComparison]::OrdinalIgnoreCase)) {
    Get-ChildItem -LiteralPath $env:TEMP -Force -ErrorAction SilentlyContinue | Where-Object { $_.LastWriteTime -lt $cutoff } | ForEach-Object { Remove-Tree $_.FullName }
}
exit 0
