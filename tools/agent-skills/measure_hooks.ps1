<#
.SYNOPSIS
Times a hook command the way Claude Code runs it: through Git Bash, with a JSON payload on stdin.

.DESCRIPTION
Runs -Command under the shell -Warmup + -Runs times, feeding -Payload on stdin, and prints one
compressed JSON object with the median, 90th percentile and minimum wall time in milliseconds,
plus the exit code of the last run. CLAUDE_PROJECT_DIR is set to the repository root, as Claude Code does.

.EXAMPLE
pwsh tools/agent-skills/measure_hooks.ps1 -Command 'python -c pass' -Runs 10
#>
param(
    [Parameter(Mandatory)][string]$Command,
    [string]$Payload = '{}',
    [int]$Runs = 30,
    [int]$Warmup = 3,
    [string]$Shell = ''
)

if (-not $Shell) {
    if ($IsWindows) {
        # ProgramFiles can name the 32-bit folder, so also try the 64-bit one.
        $Shell = @($env:ProgramFiles, $env:ProgramW6432) | Where-Object { $_ } |
            ForEach-Object { Join-Path $_ 'Git\bin\bash.exe' } | Where-Object { Test-Path $_ } |
            Select-Object -First 1
    } else {
        $Shell = '/bin/sh'
    }
}
if (-not $Shell -or -not (Test-Path $Shell)) { throw "Shell not found: $Shell" }

$env:CLAUDE_PROJECT_DIR = (git rev-parse --show-toplevel)
$times = @()
$exit = 0
for ($i = 0; $i -lt $Warmup + $Runs; $i++) {
    $sw = [Diagnostics.Stopwatch]::StartNew()
    $null = $Payload | & $Shell -c $Command 2>&1
    $sw.Stop()
    $exit = $LASTEXITCODE
    if ($i -ge $Warmup) { $times += $sw.Elapsed.TotalMilliseconds }
}

$sorted = @($times | Sort-Object)
$n = $sorted.Count
if ($n % 2 -eq 1) {
    $median = $sorted[($n - 1) / 2]
} else {
    $median = ($sorted[$n / 2 - 1] + $sorted[$n / 2]) / 2
}
$p90 = $sorted[[Math]::Ceiling(0.9 * $n) - 1]

[pscustomobject]@{
    runs      = $n
    median_ms = [Math]::Round($median, 1)
    p90_ms    = [Math]::Round($p90, 1)
    min_ms    = [Math]::Round($sorted[0], 1)
    exit      = $exit
} | ConvertTo-Json -Compress
