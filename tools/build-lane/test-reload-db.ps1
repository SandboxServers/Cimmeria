<#
.SYNOPSIS
    Tests for reload-db.ps1's database-name rules, through its -DryRun switch, against a
    scratch git repo (a main checkout and one worktree). No Postgres, no cargo, no bash.
    Exits non-zero on a failure.

        pwsh -NoProfile -File tools/build-lane/test-reload-db.ps1
#>
$ErrorActionPreference = 'Stop'
$script = (Resolve-Path (Join-Path $PSScriptRoot 'reload-db.ps1')).Path

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs reload-db.ps1 -DryRun in $dir with CIMMERIA_TEST_DB set to $dbOverride (or unset when
# $null). Returns the exit code and the combined output.
function Run([string]$dir, $dbOverride) {
    $saved = $env:CIMMERIA_TEST_DB
    Push-Location $dir
    $ErrorActionPreference = 'Continue'
    try {
        $env:CIMMERIA_TEST_DB = $dbOverride
        $out = (& pwsh -NoProfile -File $script -DryRun 2>&1 | Out-String)
        $code = $LASTEXITCODE
    } finally {
        Pop-Location
        $env:CIMMERIA_TEST_DB = $saved
        $ErrorActionPreference = 'Stop'
    }
    [pscustomobject]@{ Code = $code; Out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ('pp15-reload-' + [guid]::NewGuid().ToString('N'))
$main = Join-Path $tmp 'main'
$wt = Join-Path $tmp ('wt-' + [guid]::NewGuid().ToString('N'))
try {
    New-Item -ItemType Directory -Force (Join-Path $main 'db') | Out-Null
    New-Item -ItemType File -Force (Join-Path $main 'db/database.sql') | Out-Null
    git -C $main init -q
    git -C $main add -A
    git -C $main -c user.name=t -c user.email=t@t.invalid commit -q -m init
    git -C $main worktree add -q $wt -b pp15test

    $m = $main
    $w = $wt

    $r = Run $m $null
    Check ($r.Code -eq 0 -and $r.Out -match 'target database sgwtest') 'main checkout, no override: exit 0 and target sgwtest'

    $r = Run $m 'sgw'
    Check ($r.Code -ne 0 -and $r.Out -match 'refusing') 'main checkout, CIMMERIA_TEST_DB=sgw: refused'

    $r = Run $m 'SGW'
    Check ($r.Code -ne 0) 'main checkout, CIMMERIA_TEST_DB=SGW (case): refused'

    $r = Run $w $null
    Check ($r.Code -eq 0 -and $r.Out -match 'target database sgw_wt_') 'worktree, no override: exit 0 and target sgw_wt_*'

    $r = Run $w 'sgw'
    Check ($r.Code -ne 0) 'worktree, CIMMERIA_TEST_DB=sgw: refused'

    $r = Run $m 'sgw_custom'
    Check ($r.Code -eq 0 -and $r.Out -match 'sgw_custom') 'override to sgw_custom: exit 0 and names it'
} finally {
    try { git -C $main worktree remove --force $wt 2>$null | Out-Null } catch {}
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "FAILED: $($script:failed) check(s)"; exit 1 }
Write-Host 'all reload-db checks passed'
