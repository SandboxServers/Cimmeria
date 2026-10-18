<#
.SYNOPSIS
    Tests for branch-at.ps1 in a throwaway repository with a linked worktree. No Pester needed;
    exits non-zero on a failure. Runs no cargo.

        pwsh -NoProfile -File tools/build-lane/test-branch-at.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'branch-at.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs the script in-process and returns its exit code and printed text.
function Invoke-BranchAt([string]$repo, [string]$name, [string]$sha) {
    $out = (& $script -C $repo -Name $name -Sha $sha 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("branch-at-test-" + [guid]::NewGuid().ToString('N'))
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
    & git -C $main commit -q -m 'first'
    $first = (& git -C $main rev-parse HEAD).Trim()
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'two'
    & git -C $main commit -q -am 'second'
    $second = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main worktree add -q -b feat/wt $wt
    & git -C $main remote add origin (Join-Path $tmp 'origin.git')
    $refsBefore = (& git -C $main for-each-ref --format='%(refname) %(objectname)' | Out-String)

    # --- refusals -----------------------------------------------------------------------
    $r = Invoke-BranchAt $main 'feat/a' $first.ToUpper()
    Check ($r.code -eq 2 -and $r.out -match 'sha-not-full-40-hex') 'refuses an upper-case SHA'
    $r = Invoke-BranchAt $main 'feat/a' $first.Substring(0, 12)
    Check ($r.code -eq 2 -and $r.out -match 'sha-not-full-40-hex') 'refuses a short SHA'
    $r = Invoke-BranchAt $main 'feat/a' ('0' * 40)
    Check ($r.code -eq 2 -and $r.out -match 'no-such-commit') 'refuses a SHA that names no commit'
    $r = Invoke-BranchAt $main '--force' $first
    Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') 'refuses a name that starts with a dash'
    $r = Invoke-BranchAt $main 'bad..name' $first
    Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') 'refuses an invalid branch name'
    foreach ($amb in '@', 'HEAD', 'FETCH_HEAD', 'origin/main', 'origin', 'tags/x', 'heads/x', 'remotes/x', 'refs/heads/zz') {
        $r = Invoke-BranchAt $main $amb $first
        Check ($r.code -eq 2 -and $r.out -match 'status=refused') "refuses the ambiguous name '$amb' (exit $($r.code))"
    }
    $r = Invoke-BranchAt $main 'feat/wt' $second
    Check ($r.code -eq 2 -and $r.out -match 'branch-exists') 'refuses to overwrite an existing branch'
    $r = Invoke-BranchAt $wt 'feat/wt' $second
    Check ($r.code -eq 2 -and $r.out -match 'branch-exists') 'refuses an existing branch from a worktree too'
    $refsAfter = (& git -C $main for-each-ref --format='%(refname) %(objectname)' | Out-String)
    Check ($refsAfter -eq $refsBefore) 'the refusals changed no ref'

    # --- creation -----------------------------------------------------------------------
    $r = Invoke-BranchAt $wt 'feat/at-first' $first
    Check ($r.code -eq 0 -and $r.out -match "status=ok branch=feat/at-first sha=$first") "creates a branch at the SHA (exit $($r.code))"
    Check ((& git -C $main rev-parse refs/heads/feat/at-first).Trim() -eq $first) 'the new ref points at the SHA'
    Check ((& git -C $main rev-parse --abbrev-ref HEAD).Trim() -eq 'main') 'the checkout of main did not move'
    Check ((& git -C $wt rev-parse --abbrev-ref HEAD).Trim() -eq 'feat/wt') 'the worktree did not move'
    $r = Invoke-BranchAt $main 'originals/x' $first
    Check ($r.code -eq 0) 'a name that only starts with a remote name as a prefix is allowed'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all branch-at checks passed'
exit 0
