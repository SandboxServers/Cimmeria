<#
.SYNOPSIS
    Tests for rm-worktree.ps1 --merged-into in a throwaway repository: three packet worktrees,
    one integration branch merged into PR 7's head, and gh and psql stubs. No real gh, no
    database. Exits non-zero on a failure.

        pwsh -NoProfile -File tools/build-lane/test-rm-worktree.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'rm-worktree.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs rm-worktree.ps1 in a child pwsh, from the main checkout; returns its exit code and text.
function Invoke-Rm([string]$main, [string[]]$argv) {
    $psi = [System.Diagnostics.ProcessStartInfo]::new((Get-Process -Id $PID).Path)
    foreach ($a in @('-NoProfile', '-File', $script) + $argv) { $psi.ArgumentList.Add($a) }
    $psi.WorkingDirectory = $main
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.UseShellExecute = $false
    $p = [System.Diagnostics.Process]::Start($psi)
    $out = $p.StandardOutput.ReadToEnd()
    $err = $p.StandardError.ReadToEnd()
    $p.WaitForExit()
    return @{ code = $p.ExitCode; out = $out; err = $err }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ('rm-worktree-test-' + [guid]::NewGuid().ToString('N'))
$varNames = 'PATH', 'PSQL', 'LANE_ROOT', 'CIMMERIA_TARGET_ROOT', 'GH_STUB_STATE', 'GIT_CONFIG_GLOBAL', 'GIT_CONFIG_SYSTEM'
$saved = @{}
foreach ($v in $varNames) { $saved[$v] = [Environment]::GetEnvironmentVariable($v) }
try {
    New-Item -ItemType Directory $tmp | Out-Null
    # Isolate git from the developer's global and system config, as test-branch-at.ps1 does.
    $emptyConfig = Join-Path $tmp 'empty.gitconfig'
    New-Item -ItemType File $emptyConfig | Out-Null
    $env:GIT_CONFIG_GLOBAL = $emptyConfig
    $env:GIT_CONFIG_SYSTEM = $emptyConfig

    # gh answers `pr list` with null (no PR of its own) and `pr view` with GH_STUB_STATE.
    $bin = Join-Path $tmp 'bin'
    New-Item -ItemType Directory $bin | Out-Null
    Set-Content -LiteralPath (Join-Path $bin 'gh.cmd') -Value @'
@echo off
if "%1 %2"=="pr list" (echo null& exit /b 0)
if "%1 %2"=="pr view" (echo %GH_STUB_STATE%& exit /b 0)
exit /b 2
'@
    Set-Content -LiteralPath (Join-Path $bin 'psql.cmd') -Value '@exit /b 0'
    $env:PATH = "$bin;$env:PATH"
    $env:PSQL = Join-Path $bin 'psql.cmd'
    $env:LANE_ROOT = Join-Path $tmp 'lane'
    $env:CIMMERIA_TARGET_ROOT = Join-Path $tmp 'targets'
    New-Item -ItemType Directory $env:CIMMERIA_TARGET_ROOT | Out-Null
    $env:GH_STUB_STATE = 'MERGED'

    $origin = Join-Path $tmp 'origin.git'
    $main = Join-Path $tmp 'main'
    & git init -q --bare -b main $origin
    & git init -q -b main $main
    & git -C $main config user.email 'test@example.invalid'
    & git -C $main config user.name 'test'
    & git -C $main config core.autocrlf false
    Set-Content -LiteralPath (Join-Path $main '.gitignore') -Value '.claude/worktrees/'
    Set-Content -LiteralPath (Join-Path $main 'base.txt') -Value 'base'
    & git -C $main add -- .gitignore base.txt
    & git -C $main commit -q -m 'base'
    & git -C $main remote add origin $origin
    & git -C $main push -q origin main

    # Three packet worktrees, each with one commit of its own file.
    New-Item -ItemType Directory (Join-Path $main '.claude/worktrees') | Out-Null
    $dirs = @{}
    foreach ($x in 'a', 'b', 'c') {
        $wt = Join-Path $main ".claude/worktrees/zz-s5-$x"
        $dirs[$x] = $wt
        & git -C $main worktree add -q -b "zz-s5/$x" $wt origin/main
        Set-Content -LiteralPath (Join-Path $wt "$x.txt") -Value $x
        & git -C $wt add -- "$x.txt"
        & git -C $wt commit -q -m "packet $x"
    }
    # The integration branch merges a and c only; b is not in PR 7's head.
    & git -C $main checkout -q -b 'zz-s5/int' origin/main
    & git -C $main merge -q --no-ff -m 'integration' zz-s5/a zz-s5/c
    $intSha = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main push -q origin 'zz-s5/int'
    & git -C $main checkout -q main
    & git -C $origin update-ref refs/pull/7/head $intSha

    # --- retirement -----------------------------------------------------------------------
    $r = Invoke-Rm $main @('--merged-into', '7', 'zz-s5-a')
    Check ($r.code -eq 0 -and $r.out -match 'retire zz-s5-a \(zz-s5/a, MERGED_INTO\)') 'retires a packet merged only into PR 7 head'
    Check (-not (Test-Path -LiteralPath $dirs['a'])) 'the zz-s5-a folder is gone'
    Check ([string]::IsNullOrWhiteSpace((& git -C $main branch --list 'zz-s5/a' | Out-String))) 'the zz-s5/a branch is deleted'

    $r = Invoke-Rm $main @('zz-s5-c')
    Check (($r.code -eq 0) -and ($r.out -match 'skip zz-s5-c: branch zz-s5/c is NO_PR')) 'without --merged-into, zz-s5-c is skipped as NO_PR'
    Check (Test-Path -LiteralPath $dirs['c']) 'the zz-s5-c folder stays'

    $r = Invoke-Rm $main @('--merged-into', '7', 'zz-s5-b')
    Check (($r.code -eq 0) -and ($r.out -match 'skip zz-s5-b: branch zz-s5/b is NO_PR') -and (Test-Path -LiteralPath $dirs['b'])) 'a branch with a commit past the PR head is skipped'

    $junk = Join-Path $dirs['c'] 'junk.txt'
    Set-Content -LiteralPath $junk -Value 'untracked'
    $r = Invoke-Rm $main @('--merged-into', '7', 'zz-s5-c')
    Check (($r.code -eq 0) -and ($r.out -match 'skip zz-s5-c: uncommitted changes')) 'uncommitted changes skip even with --merged-into'
    Remove-Item -LiteralPath $junk
    $r = Invoke-Rm $main @('--merged-into', '7', 'zz-s5-c')
    Check (($r.code -eq 0) -and ($r.out -match 'retire zz-s5-c \(zz-s5/c, MERGED_INTO\)')) 'zz-s5-c is retired once it is clean'

    # --- refusals -------------------------------------------------------------------------
    $env:GH_STUB_STATE = 'OPEN'
    try { $r = Invoke-Rm $main @('--merged-into', '7', 'zz-s5-b') } finally { $env:GH_STUB_STATE = 'MERGED' }
    Check (($r.code -eq 1) -and ($r.err -match 'not merged')) 'refuses when PR 7 is not merged'
    Check (Test-Path -LiteralPath $dirs['b']) 'the zz-s5-b folder stays after the refusal'

    $usage = @(
        @{ argv = @('--merged-into'); want = 'needs a PR number' },
        @{ argv = @('--merged-into', 'x', 'zz'); want = 'needs a PR number, got x' },
        @{ argv = @('--merged-into', '7', '--merged'); want = 'does not combine with --merged' },
        @{ argv = @('--merged-into', '7'); want = 'usage:' }
    )
    foreach ($u in $usage) {
        $r = Invoke-Rm $main $u.argv
        Check (($r.code -eq 2) -and ($r.err -match [regex]::Escape($u.want))) "usage: $($u.argv -join ' ') exits 2"
    }

    $r = Invoke-Rm $main @('--dry-run', '--merged-into', '7', 'zz-s5-b')
    Check (($r.code -eq 0) -and ($r.out -match 'skip zz-s5-b: branch zz-s5/b is NO_PR') -and (Test-Path -LiteralPath $dirs['b'])) 'a dry run of a skip removes nothing'
} finally {
    foreach ($v in $varNames) { [Environment]::SetEnvironmentVariable($v, $saved[$v]) }
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue }
}

if ($script:failed) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all checks passed'
