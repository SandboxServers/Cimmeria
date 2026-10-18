<#
.SYNOPSIS
    Tests for rebase-onto.ps1 in a throwaway main checkout plus one linked worktree. No network and
    no Pester needed; exits non-zero on a failure. Runs no cargo.

        pwsh -NoProfile -File tools/build-lane/test-rebase-onto.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'rebase-onto.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs the script in-process and returns its exit code and printed text.
function Invoke-Rebase([string]$wt, [string]$onto, [string]$upstream = '') {
    $params = @{ C = $wt; Onto = $onto }
    if ($upstream) { $params.Upstream = $upstream }
    $out = (& $script @params 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rebase-onto-test-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $tmp | Out-Null
try {
    $main = Join-Path $tmp 'main'
    $wt = Join-Path $tmp 'wt'
    New-Item -ItemType Directory $main | Out-Null
    & git -C $main init -q -b main
    & git -C $main config user.email 'test@example.invalid'
    & git -C $main config user.name 'test'
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'one'
    & git -C $main add -- a.txt
    & git -C $main commit -q -m 'init'
    & git -C $main worktree add -q -b feat/x $wt
    Set-Content -LiteralPath (Join-Path $wt 'b.txt') 'feat'
    & git -C $wt add -- b.txt
    & git -C $wt commit -q -m 'feat one'
    Set-Content -LiteralPath (Join-Path $main 'c.txt') 'main'
    & git -C $main add -- c.txt
    & git -C $main commit -q -m 'main advances'
    $mainTip = (& git -C $main rev-parse HEAD).Trim()

    # --- refusals leave the branch alone ---------------------------------------------
    $r = Invoke-Rebase $main 'main'
    Check ($r.code -eq 2 -and $r.out -match 'main-checkout') 'refuses the main checkout'
    $r = Invoke-Rebase $wt 'no-such-rev'
    Check ($r.code -eq 2 -and $r.out -match 'not-a-commit') 'refuses an -Onto that names no commit'
    Set-Content -LiteralPath (Join-Path $wt 'dirty.txt') 'untracked'
    $before = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt 'main'
    Check ($r.code -eq 2 -and $r.out -match 'dirty-tree') 'refuses a dirty tree'
    Check ((& git -C $wt rev-parse HEAD).Trim() -eq $before) 'the dirty refusal did not move HEAD'
    Remove-Item -LiteralPath (Join-Path $wt 'dirty.txt')

    # --- a clean rebase replays the branch onto the new tip --------------------------
    $r = Invoke-Rebase $wt 'main'
    $tip = (& git -C $wt rev-parse HEAD).Trim()
    Check ($r.code -eq 0 -and $r.out -match "status=ok tip=$tip branch=feat/x") "rebases onto main (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wt merge-base HEAD main).Trim() -eq $mainTip) 'HEAD now contains main'
    Check (Test-Path -LiteralPath (Join-Path $wt 'b.txt')) 'the branch commit survived the rebase'
    Check ((& git -C $wt status --porcelain | Out-String).Trim() -eq '') 'the tree is clean after the rebase'

    $r = Invoke-Rebase $wt 'main'
    Check ($r.code -eq 0 -and $r.out -match 'status=up-to-date') "a second rebase onto main is up to date (exit $($r.code))"

    # --- -Upstream must be an ancestor of HEAD ----------------------------------------
    Set-Content -LiteralPath (Join-Path $main 'd.txt') 'main'
    & git -C $main add -- d.txt
    & git -C $main commit -q -m 'main d'
    $newMain = (& git -C $main rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt 'main' $newMain
    Check ($r.code -eq 2 -and $r.out -match 'upstream-not-ancestor') 'refuses an -Upstream that HEAD does not contain'

    # --- a conflict is aborted and leaves the worktree as it was ---------------------
    Set-Content -LiteralPath (Join-Path $wt 'd.txt') 'feat'
    & git -C $wt add -- d.txt
    & git -C $wt commit -q -m 'feat d'
    $before = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt 'main'
    $gitDir = (& git -C $wt rev-parse --absolute-git-dir).Trim()
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=rebase-aborted') "a conflicting rebase fails and aborts (exit $($r.code))"
    Check ((& git -C $wt rev-parse HEAD).Trim() -eq $before) 'the conflict left HEAD where it was'
    Check ((& git -C $wt status --porcelain | Out-String).Trim() -eq '') 'the conflict left the tree clean'
    Check (-not (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-merge')) -and -not (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-apply'))) 'no rebase is left in progress'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all rebase-onto checks passed'
exit 0
