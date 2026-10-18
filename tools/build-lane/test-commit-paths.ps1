<#
.SYNOPSIS
    Tests for commit-paths.ps1 in a throwaway main checkout plus one linked worktree. No Pester
    needed; exits non-zero on a failure. Runs no cargo.

        pwsh -NoProfile -File tools/build-lane/test-commit-paths.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'commit-paths.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs the script the way ops does, as a child `pwsh -File` with one comma list for -Paths and one
# `;;` list for -Trailers, and returns its exit code and printed text. In-process calls would hand
# the script real arrays and hide how a command line binds.
function Invoke-Commit([string]$wt, [string[]]$paths, [string[]]$trailers = @(), [string]$expect = '') {
    $argv = @('-NoProfile', '-File', $script, '-C', $wt, '-Message', 'subject line', "-Paths:$($paths -join ',')")
    if ($trailers.Count -gt 0) { $argv += @('-Trailers', ($trailers -join ';;')) }
    if ($expect) { $argv += @('-ExpectBranch', $expect) }
    $out = (& pwsh @argv 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("commit-paths-test-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $tmp | Out-Null
try {
    $main = Join-Path $tmp 'main'
    $wt = Join-Path $tmp 'wt'
    New-Item -ItemType Directory $main | Out-Null
    & git -C $main init -q -b main
    & git -C $main config user.email 'test@example.invalid'
    & git -C $main config user.name 'test'
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'one'
    Set-Content -LiteralPath (Join-Path $main 'b.txt') 'one'
    & git -C $main add -- a.txt b.txt
    & git -C $main commit -q -m 'init'
    & git -C $main worktree add -q -b feat/x $wt
    New-Item -ItemType Directory (Join-Path $wt 'sub') | Out-Null
    Set-Content -LiteralPath (Join-Path $wt 'sub\c.txt') 'x'
    $head = { & git -C $wt rev-parse HEAD }

    # --- refusals leave HEAD where it was ---------------------------------------------
    $before = & $head
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'two'
    foreach ($bad in '.', '-A', '--all', '*.txt', 'sub', ':/a.txt', 'a.txt/', 'sub/..', 'a.txt,') {
        $r = Invoke-Commit $wt @($bad)
        Check ($r.code -eq 2 -and $r.out -match 'status=refused') "refused path '$bad' (exit $($r.code))"
    }
    Check ((& $head) -eq $before) 'no refusal moved HEAD'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'no refusal staged anything'

    $r = Invoke-Commit $main @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'main-checkout') 'refuses the main checkout'
    $r = Invoke-Commit (Join-Path $wt 'sub') @('c.txt')
    Check ($r.code -eq 2 -and $r.out -match 'not-the-worktree-root') 'refuses a subdirectory of the worktree'

    Set-Content -LiteralPath (Join-Path $wt 'b.txt') 'two'
    & git -C $wt add -- b.txt
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'other-staged-files: b.txt') 'refuses when another file is staged'
    & git -C $wt restore --staged -- b.txt
    Check ((& $head) -eq $before) 'the other-staged refusal did not commit'

    & git -C $wt checkout -q --detach
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'detached-head') 'refuses a detached HEAD'
    & git -C $wt checkout -q feat/x

    # --- the happy path stages and commits only the named files ------------------------
    Set-Content -LiteralPath (Join-Path $wt 'b.txt') 'two'
    $r = Invoke-Commit $wt @('a.txt') @('Co-Authored-By: Test <test@example.invalid>')
    Check ($r.code -eq 0 -and $r.out -match 'status=ok sha=[0-9a-f]{40} branch=feat/x') "commits the named path (exit $($r.code)): $($r.out)"
    $files = @(& git -C $wt show --name-only --format= HEAD | Where-Object { $_ })
    Check ($files.Count -eq 1 -and $files[0] -eq 'a.txt') 'the commit holds only a.txt'
    $status = (& git -C $wt status --porcelain | Out-String)
    Check ($status -match ' M b.txt') 'the unnamed b.txt is left unstaged and modified'
    $msg = (& git -C $wt log -1 --format=%B) -join "`n"
    Check ($msg -match 'subject line' -and $msg -match "`n`nCo-Authored-By: Test") 'the message has the subject and a trailer block'

    # --- several paths and several trailers survive a child pwsh -File ----------------
    $before = & $head
    $r = Invoke-Commit $wt @('b.txt') @() 'other/branch'
    Check ($r.code -eq 2 -and $r.out -match 'branch-mismatch') '-ExpectBranch that differs from HEAD is refused'
    Check ((& $head) -eq $before) 'the branch-mismatch refusal did not commit'
    $trailers = @('Co-Authored-By: Test <test@example.invalid>', 'Claude-Session: https://example.invalid/s/1')
    $r = Invoke-Commit $wt @('b.txt', 'sub/c.txt') $trailers 'feat/x'
    Check ($r.code -eq 0 -and $r.out -match 'status=ok sha=[0-9a-f]{40} branch=feat/x') "commits two paths with -ExpectBranch (exit $($r.code)): $($r.out)"
    $files = @(& git -C $wt show --name-only --format= HEAD | Where-Object { $_ } | Sort-Object)
    Check (($files -join ',') -eq 'b.txt,sub/c.txt') "the commit holds both named files: $($files -join ',')"
    $msg = (& git -C $wt log -1 --format=%B) -join "`n"
    Check ($msg -match "`n`nCo-Authored-By: Test <test@example.invalid>`nClaude-Session: https://example.invalid/s/1") 'both trailers are separate lines in one trailer block'

    # --- a whitespace error stops the commit and leaves nothing staged ------------------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'bad   '
    $before = & $head
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'diff-check') 'refuses a whitespace error'
    Check ((& $head) -eq $before) 'the whitespace refusal did not commit'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'the whitespace refusal left nothing staged'

    # --- a failing hook leaves nothing staged ------------------------------------------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'fine'
    $hook = Join-Path (& git -C $wt rev-parse --git-common-dir) 'hooks/pre-commit'
    [System.IO.File]::WriteAllText($hook, "#!/bin/sh`nexit 1`n")
    $r = Invoke-Commit $wt @('a.txt')
    Remove-Item -LiteralPath $hook -Force
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=git-commit') "a failing hook is a failed commit (exit $($r.code))"
    Check ((& $head) -eq $before) 'the hook failure did not commit'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'the hook failure left nothing staged'
    & git -C $wt restore -- a.txt

    # --- a rename staged by someone else is not swept into the commit -------------------
    Set-Content -LiteralPath (Join-Path $wt 'r.txt') 'rename me, a file long enough to pair'
    & git -C $wt add -- r.txt
    & git -C $wt commit -q -m 'add r'
    $before = & $head
    & git -C $wt mv r.txt r2.txt
    $r = Invoke-Commit $wt @('r2.txt')
    Check ($r.code -eq 2 -and $r.out -match 'other-staged-files: r\.txt') "refuses a staged rename whose source is unnamed (exit $($r.code)): $($r.out)"
    Check ((& $head) -eq $before) 'the rename refusal did not commit'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all commit-paths checks passed'
exit 0
