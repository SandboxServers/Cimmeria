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

# Runs the script in-process and returns its exit code and printed text.
function Invoke-Commit([string]$wt, [string[]]$paths, [string[]]$trailers = @()) {
    $out = (& $script -C $wt -Message 'subject line' -Paths $paths -Trailers $trailers 2>&1 | Out-String)
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
    foreach ($bad in '.', '-A', '--all', '*.txt', 'sub', ':/a.txt', 'a.txt/', 'sub/..', '') {
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

    # --- a whitespace error stops the commit ------------------------------------------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'bad   '
    $before = & $head
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'diff-check') 'refuses a whitespace error'
    Check ((& $head) -eq $before) 'the whitespace refusal did not commit'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all commit-paths checks passed'
exit 0
