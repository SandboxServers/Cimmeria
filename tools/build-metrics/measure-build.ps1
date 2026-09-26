<#
.SYNOPSIS
  Measures Cimmeria build cost: cold build time, edit-rebuild loop, peak memory and target size.

.DESCRIPTION
  Run it from a worktree with an EMPTY target dir so "cold" means cold for workspace crates.
  Third-party crates still come from sccache when RUSTC_WRAPPER is set, which matches how
  agents and developers build day to day.

  Phases:
    1. cold   - `cargo build` of the CI-gated workspace, all targets, with --timings.
    2. edit   - a real source edit in -EditFile, then `cargo test -p <crate> --no-run`
                (the develop-and-test loop). The edit is reverted afterwards.
    3. check  - another edit, then `cargo check -p <crate>`.

  A background sampler records, every 2 s, free physical memory and the summed working set
  of rustc / linker / cargo processes. Results go to <OutDir>\summary.json plus the cargo
  timing report.

.EXAMPLE
  pwsh tools/build-metrics/measure-build.ps1 -Label baseline -OutDir $env:TEMP\cimmeria-metrics\baseline
#>
param(
    [Parameter(Mandatory)] [string] $Label,
    [Parameter(Mandatory)] [string] $OutDir,
    [string] $EditFile = 'crates/services/src/cell/content/mod.rs',
    [string] $EditCrate = 'cimmeria-services',
    [switch] $SkipCold
)
$ErrorActionPreference = 'Stop'
$root = (git rev-parse --show-toplevel).Trim()
Set-Location $root
New-Item -ItemType Directory -Force $OutDir | Out-Null

$excludes = @('cimmeria-app', 'cimmeria-content-editor', 'cimmeria-scene-editor', 'sgw-launcher',
    'cimmeria-client-telemetry', 'cimmeria-lab') | ForEach-Object { '--exclude', $_ }

# --- memory sampler ---------------------------------------------------------------------------
$samples = Join-Path $OutDir 'memory-samples.csv'
$stopFlag = Join-Path $OutDir 'sampler.stop'
Remove-Item $stopFlag -ErrorAction SilentlyContinue
$sampler = Start-Job -ArgumentList $samples, $stopFlag -ScriptBlock {
    param($samples, $stopFlag)
    'time,phase,free_mb,build_ws_mb,rustc_n,linker_n' | Set-Content $samples
    $phaseFile = [IO.Path]::ChangeExtension($samples, '.phase')
    while (-not (Test-Path $stopFlag)) {
        $os = Get-CimInstance Win32_OperatingSystem
        $procs = Get-Process rustc, rust-lld, lld-link, link, cargo, cargo-nextest, clippy-driver -ErrorAction SilentlyContinue
        $ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB
        $phase = if (Test-Path $phaseFile) { Get-Content $phaseFile } else { '' }
        '{0},{1},{2:F0},{3:F0},{4},{5}' -f (Get-Date -Format 'HH:mm:ss'), $phase, ($os.FreePhysicalMemory / 1KB),
            $ws, @($procs | Where-Object Name -eq 'rustc').Count,
            @($procs | Where-Object Name -in 'rust-lld', 'lld-link', 'link').Count | Add-Content $samples
        Start-Sleep -Seconds 2
    }
}
$phaseFile = [IO.Path]::ChangeExtension($samples, '.phase')
function Set-Phase($p) { Set-Content $phaseFile $p }

function Invoke-Timed($name, [scriptblock] $block) {
    Set-Phase $name
    $sw = [Diagnostics.Stopwatch]::StartNew()
    & $block
    $code = $LASTEXITCODE
    $sw.Stop()
    Set-Phase ''
    if ($code -ne 0) { throw "$name failed with exit code $code" }
    [math]::Round($sw.Elapsed.TotalSeconds, 1)
}

function Get-DirStats($path) {
    if (-not (Test-Path $path)) { return @{ gb = 0; files = 0 } }
    $o = robocopy $path NUL /L /S /NJH /BYTES /NFL /NDL /NC /NP /R:0 /W:0 2>$null
    $bytes = [double](((($o | Select-String 'Bytes :').Line) -split '\s+')[3])
    $files = [int](((($o | Select-String 'Files :').Line) -split '\s+')[3])
    @{ gb = [math]::Round($bytes / 1GB, 2); files = $files }
}

function Add-Edit($file, $tag) {
    Add-Content $file "`n#[allow(dead_code)]`nfn __build_metrics_edit_$tag() -> u64 { $(Get-Random) }"
}

$result = [ordered]@{ label = $Label; when = (Get-Date -Format s); commit = (git rev-parse --short HEAD).Trim();
    rustc = (rustc --version).Trim(); jobs = $env:CARGO_BUILD_JOBS; wrapper = $env:RUSTC_WRAPPER }
$orig = Get-Content $EditFile -Raw
try {
    if (-not $SkipCold) {
        $result.cold_build_s = Invoke-Timed 'cold' { cargo build --workspace --all-targets @excludes --timings 2>&1 | Out-File (Join-Path $OutDir 'cold.log') }
        Copy-Item (Join-Path $root 'target/cargo-timings/cargo-timing.html') (Join-Path $OutDir 'cargo-timing.html') -ErrorAction SilentlyContinue
        $result.target_after_cold = Get-DirStats (Join-Path $root 'target')
    }
    Add-Edit $EditFile 'a'
    $result.edit_test_build_s = Invoke-Timed 'edit' { cargo test -p $EditCrate --no-run 2>&1 | Out-File (Join-Path $OutDir 'edit.log') }
    Add-Edit $EditFile 'b'
    $result.edit_check_s = Invoke-Timed 'check' { cargo check -p $EditCrate 2>&1 | Out-File (Join-Path $OutDir 'check.log') }
}
finally {
    [IO.File]::WriteAllText((Resolve-Path $EditFile), $orig)
    New-Item $stopFlag -ItemType File -Force | Out-Null
    Wait-Job $sampler -Timeout 30 | Out-Null
    Remove-Job $sampler -Force
}
$rows = Import-Csv $samples
foreach ($p in 'cold', 'edit', 'check') {
    $r = $rows | Where-Object phase -eq $p
    if ($r) {
        $result["${p}_peak_build_ws_gb"] = [math]::Round((($r | ForEach-Object { [double]($_.build_ws_mb -replace ',', '') }) | Measure-Object -Maximum).Maximum / 1024, 1)
        $result["${p}_min_free_gb"] = [math]::Round((($r | ForEach-Object { [double]($_.free_mb -replace ',', '') }) | Measure-Object -Minimum).Minimum / 1024, 1)
        $result["${p}_max_parallel_links"] = (($r | ForEach-Object { [int]$_.linker_n }) | Measure-Object -Maximum).Maximum
    }
}
$result.target_final = Get-DirStats (Join-Path $root 'target')
$result | ConvertTo-Json | Tee-Object (Join-Path $OutDir 'summary.json')
