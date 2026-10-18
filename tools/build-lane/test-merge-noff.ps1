<#
.SYNOPSIS
    Tests for merge-noff.ps1 in a throwaway main checkout plus linked worktrees. No Pester
    needed; exits non-zero on a failure. Runs no cargo.

        pwsh -NoProfile -File tools/build-lane/test-merge-noff.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'merge-noff.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs the script in-process and returns its exit code and printed text.
function Invoke-Merge([string]$wt, [string]$branch, [string]$expect = '') {
    $params = @{ C = $wt; Branch = $branch }
    if ($expect) { $params.ExpectBranch = $expect }
    $out = (& $script @params 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

function Git-Out([string]$wt, [string[]]$gitArgs) {
    return ((& git -C $wt @gitArgs 2>&1) | Out-String).Trim()
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("merge-noff-test-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $tmp | Out-Null
# Isolate git from the developer's global and system config (commit.gpgsign, core.hooksPath,
# rebase.updateRefs ...): both point at an empty file inside the throwaway directory.
$emptyConfig = Join-Path $tmp 'empty.gitconfig'
New-Item -ItemType File $emptyConfig | Out-Null
$env:GIT_CONFIG_GLOBAL = $emptyConfig
$env:GIT_CONFIG_SYSTEM = $emptyConfig
try {
    $main = Join-Path $tmp 'main'
    $wt = Join-Path $tmp 'wt'
    New-Item -ItemType Directory $main | Out-Null
    & git -C $main init -q -b main
    & git -C $main config user.email 'test@example.invalid'
    & git -C $main config user.name 'test'
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'base'
    Set-Content -LiteralPath (Join-Path $main 'b.txt') 'base'
    & git -C $main add -- a.txt b.txt
    & git -C $main commit -q -m 'init'
    & git -C $main worktree add -q -b feat/target $wt
    & git -C $main branch feat/src
    & git -C $main branch feat/old
    & git -C $main branch feat/conf

    # feat/src changes b.txt; feat/target changes a.txt, so the merge is clean and not a fast-forward.
    & git -C $main worktree add -q (Join-Path $tmp 'src') feat/src
    Set-Content -LiteralPath (Join-Path $tmp 'src\b.txt') 'from src'
    & git -C (Join-Path $tmp 'src') commit -q -am 'src change'
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'from target'
    & git -C $wt commit -q -am 'target change'
    $before = Git-Out $wt @('rev-parse', 'HEAD')

    # --- refusals -----------------------------------------------------------------------
    foreach ($bad in '--no-ff', '-x', 'no-such', 'feat/target') {
        $r = Invoke-Merge $wt $bad
        Check ($r.code -eq 2 -and $r.out -match 'status=refused') "refused branch '$bad' (exit $($r.code))"
    }
    $r = Invoke-Merge $main 'feat/src'
    Check ($r.code -eq 2 -and $r.out -match 'main-checkout') 'refuses the main checkout'
    Set-Content -LiteralPath (Join-Path $wt 'junk.txt') 'untracked'
    $r = Invoke-Merge $wt 'feat/src'
    Check ($r.code -eq 2 -and $r.out -match 'dirty-tree') 'refuses a dirty tree, untracked files included'
    Remove-Item -LiteralPath (Join-Path $wt 'junk.txt')
    $r = Invoke-Merge $wt 'feat/src' 'some/other-branch'
    Check ($r.code -eq 2 -and $r.out -match 'branch-mismatch') '-ExpectBranch that differs from HEAD is refused'
    Check ((Git-Out $wt @('rev-parse', 'HEAD')) -eq $before) 'the refusals moved nothing'

    # --- clean --no-ff merge -----------------------------------------------------------
    $r = Invoke-Merge $wt 'feat/src' 'feat/target'
    $parents = (Git-Out $wt @('rev-list', '--parents', '-n', '1', 'HEAD')) -split ' '
    Check ($r.code -eq 0 -and $r.out -match 'status=ok merge=[0-9a-f]{40} branch=feat/src') "merges with --no-ff (exit $($r.code)): $($r.out)"
    Check ($parents.Count -eq 3) 'the merge commit has two parents'
    $sha = Git-Out $wt @('rev-parse', 'HEAD')
    Check ($r.out -match [regex]::Escape("merge=$sha")) 'the printed merge SHA is HEAD'

    # --- already contained: no new commit ----------------------------------------------
    $r = Invoke-Merge $wt 'feat/old'
    Check ($r.code -eq 0 -and $r.out -match 'status=up-to-date') 'an already-merged branch reports up-to-date'
    Check ((Git-Out $wt @('rev-parse', 'HEAD')) -eq $sha) 'up-to-date moved nothing'

    # --- a tag with the branch's name does not win: the branch is what is merged ------------
    # `t1` is a tag at s2 and a branch at s3. A bare `t1` resolves to the tag (git warns that the
    # name is ambiguous), which would merge the wrong commit or report up-to-date.
    $srcWt = Join-Path $tmp 'src'
    Set-Content -LiteralPath (Join-Path $srcWt 's2.txt') 's2'
    & git -C $srcWt add -- s2.txt
    & git -C $srcWt commit -q -m 's2'
    & git -C $srcWt tag t1
    Set-Content -LiteralPath (Join-Path $srcWt 's3.txt') 's3'
    & git -C $srcWt add -- s3.txt
    & git -C $srcWt commit -q -m 's3'
    & git -C $srcWt branch t1
    $s3 = Git-Out $srcWt @('rev-parse', 'refs/heads/t1')
    $r = Invoke-Merge $wt 't1'
    $parents = (Git-Out $wt @('rev-list', '--parents', '-n', '1', 'HEAD')) -split ' '
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "merges the branch t1, not the tag (exit $($r.code)): $($r.out.Trim())"
    Check ($parents.Count -eq 3 -and $parents[2] -eq $s3) "the second parent is the branch tip: $($parents -join ' ')"
    Check (Test-Path -LiteralPath (Join-Path $wt 's3.txt')) 'the branch-only commit s3 is in the worktree'

    # --- conflict: aborted, worktree back to its pre-merge state ----------------------
    & git -C $main worktree add -q (Join-Path $tmp 'conf') feat/conf
    Set-Content -LiteralPath (Join-Path $tmp 'conf\a.txt') 'conflicting'
    & git -C (Join-Path $tmp 'conf') commit -q -am 'conflict side'
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'other side'
    & git -C $wt commit -q -am 'other side'
    $before = Git-Out $wt @('rev-parse', 'HEAD')
    $r = Invoke-Merge $wt 'feat/conf'
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=merge-aborted files=a.txt') "a conflict exits 1 and names the file (exit $($r.code)): $($r.out)"
    Check ((Git-Out $wt @('rev-parse', 'HEAD')) -eq $before) 'the aborted merge left HEAD where it was'
    & git -C $wt rev-parse -q --verify MERGE_HEAD *> $null
    Check ($LASTEXITCODE -ne 0) 'no merge is left in progress'
    Check ([string]::IsNullOrEmpty((Git-Out $wt @('status', '--porcelain')))) 'the tree is clean after the abort'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all merge-noff checks passed'
exit 0
