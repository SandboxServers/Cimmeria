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
    $r = Invoke-Merge $wt 'feat/src' 'FEAT/TARGET'
    Check ($r.code -eq 2 -and $r.out -match 'branch-mismatch') '-ExpectBranch is compared case-sensitively'
    # A bare second token must be a parameter error: with positional binding it would become -Branch
    # and merge feat/src, a mergeable branch here.
    $raw = (& pwsh -NoProfile -File $script -C $wt feat/src 2>&1 | Out-String)
    Check ($LASTEXITCODE -ne 0 -and $raw -notmatch 'status=ok') "a positional branch token is a parameter error, not a merge (exit $LASTEXITCODE)"
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

    # --- --no-ff: a target that could fast-forward still gets a merge commit ----------------
    $ffWt = Join-Path $tmp 'ff'
    & git -C $main worktree add -q -b feat/ff $ffWt (Git-Out $wt @('rev-parse', 'HEAD'))
    Set-Content -LiteralPath (Join-Path $ffWt 'ff.txt') 'ff'
    & git -C $ffWt add -- ff.txt
    & git -C $ffWt commit -q -m 'ff commit'
    $ffTip = Git-Out $ffWt @('rev-parse', 'HEAD')
    $r = Invoke-Merge $wt 'feat/ff'
    $parents = (Git-Out $wt @('rev-list', '--parents', '-n', '1', 'HEAD')) -split ' '
    Check ($r.code -eq 0 -and $parents.Count -eq 3 -and $parents[2] -eq $ffTip) "a fast-forwardable merge still has two parents (exit $($r.code)): $($parents -join ' ')"

    # --- the merge subject names the branch, not refs/heads/<branch> -------------------------
    $subject = Git-Out $wt @('log', '-1', '--format=%s')
    Check ($subject -ceq "Merge branch 'feat/ff' into feat/target") "the merge subject is: $subject"

    # --- the strategy is pinned: pull.twohead=ours must not drop the branch ------------------
    $thWt = Join-Path $tmp 'th'
    $initSha = Git-Out $main @('rev-list', '--max-parents=0', 'HEAD')
    & git -C $main worktree add -q -b feat/th $thWt $initSha
    Set-Content -LiteralPath (Join-Path $thWt 'th.txt') 'th'
    & git -C $thWt add -- th.txt
    & git -C $thWt commit -q -m 'th commit'
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'diverge for th'
    & git -C $wt commit -q -am 'diverge for th'
    & git -C $main config pull.twohead ours
    $r = Invoke-Merge $wt 'feat/th'
    & git -C $main config --unset pull.twohead
    Check ($r.code -eq 0 -and (Test-Path -LiteralPath (Join-Path $wt 'th.txt'))) "a configured pull.twohead=ours does not drop the branch's changes (exit $($r.code)): $($r.out.Trim())"

    # --- guards that nothing else covers -------------------------------------------------
    $before = Git-Out $wt @('rev-parse', 'HEAD')
    & git -C $main branch feat/guard $initSha
    $subdir = Join-Path $main 'sub'
    New-Item -ItemType Directory $subdir -Force | Out-Null
    $mainBefore = Git-Out $main @('rev-parse', 'HEAD')
    $r = Invoke-Merge $subdir 'feat/guard'
    Check ($r.code -eq 2 -and $r.out -match 'not-the-worktree-root') "refuses a subdirectory of the main checkout (exit $($r.code)): $($r.out.Trim())"
    Check ((Git-Out $main @('rev-parse', 'HEAD')) -eq $mainBefore) 'the main checkout did not move'
    & git -C $wt checkout -q --detach
    $r = Invoke-Merge $wt 'feat/old'
    Check ($r.code -eq 2 -and $r.out -match 'detached-head') "refuses a detached HEAD (exit $($r.code)): $($r.out.Trim())"
    & git -C $wt checkout -q feat/target
    Check ((Git-Out $wt @('rev-parse', 'HEAD')) -eq $before) 'the guard refusals moved nothing'

    # GIT_OBJECT_DIRECTORY / GIT_DIR must not redirect the merge: with an empty object store the
    # merge cannot read a single object, so the script has to clear the variable.
    $objDir = Join-Path $tmp 'odd-objects'
    New-Item -ItemType Directory $objDir | Out-Null
    & git -C $main worktree add -q -b feat/env (Join-Path $tmp 'envwt') $initSha
    Set-Content -LiteralPath (Join-Path $tmp 'envwt\env.txt') 'env'
    & git -C (Join-Path $tmp 'envwt') add -- env.txt
    & git -C (Join-Path $tmp 'envwt') commit -q -m 'env commit'
    $env:GIT_OBJECT_DIRECTORY = $objDir
    $env:GIT_DIR = (Join-Path $main '.git')
    try { $r = Invoke-Merge $wt 'feat/env' }
    finally { Remove-Item Env:GIT_OBJECT_DIRECTORY, Env:GIT_DIR -ErrorAction SilentlyContinue }
    Check ($r.code -eq 0 -and $r.out -match 'status=ok' -and (Test-Path -LiteralPath (Join-Path $wt 'env.txt'))) "merges with GIT_OBJECT_DIRECTORY and GIT_DIR set (exit $($r.code)): $($r.out.Trim())"
    Check ((Git-Out $main @('rev-parse', 'refs/heads/main')) -eq $mainBefore) 'a GIT_DIR pointing at the main checkout did not move main'
    Check ((Git-Out $wt @('rev-parse', '--verify', 'HEAD^{commit}')) -match '^[0-9a-f]{40}$') 'the worktree branch tip is a readable commit'

    # --- a merge already in progress is the caller's: refused, never aborted ----------------
    # `merge --no-commit -s ours` leaves MERGE_HEAD with an index equal to HEAD (a clean tree).
    $script:side = 0
    function New-Side([string]$branch, [string]$base, [string]$file, [string]$text) {
        $script:side++
        $dir = Join-Path $tmp "side$($script:side)"
        & git -C $main worktree add -q -b $branch $dir $base
        Set-Content -LiteralPath (Join-Path $dir $file) $text
        & git -C $dir add -- $file
        & git -C $dir commit -q -m "$branch $file"
    }
    New-Side 'side/open' $initSha 'open.txt' 'open'
    New-Side 'side/other' $initSha 'other.txt' 'other'
    & git -C $wt merge -q --no-commit --no-ff -s ours side/open *> $null
    $openHead = Git-Out $wt @('rev-parse', '-q', '--verify', 'MERGE_HEAD')
    $r = Invoke-Merge $wt 'side/other'
    Check ($r.code -eq 2 -and $r.out -match 'operation-in-progress: MERGE_HEAD') "a merge in progress is refused (exit $($r.code)): $($r.out.Trim())"
    Check ((Git-Out $wt @('rev-parse', '-q', '--verify', 'MERGE_HEAD')) -eq $openHead) "the caller's merge is still open"
    & git -C $wt merge --abort *> $null

    # --- branch.<current>.mergeOptions does not reach the merge ------------------------------
    # git parses it before argv and `-s` accumulates: `-s ours` would drop the branch, `--no-commit`
    # would leave the merge open, `-Xours` would resolve a conflict silently to this side.
    New-Side 'side/mo1' $initSha 'mo1.txt' 'mo1'
    & git -C $main config branch.feat/target.mergeoptions '-s ours'
    $r = Invoke-Merge $wt 'side/mo1'
    Check ($r.code -eq 0 -and (Test-Path -LiteralPath (Join-Path $wt 'mo1.txt'))) "mergeoptions '-s ours' does not drop the branch (exit $($r.code)): $($r.out.Trim())"
    New-Side 'side/mo2' $initSha 'mo2.txt' 'mo2'
    & git -C $main config branch.feat/target.mergeoptions '--no-commit'
    $r = Invoke-Merge $wt 'side/mo2'
    $parents = (Git-Out $wt @('rev-list', '--parents', '-n', '1', 'HEAD')) -split ' '
    & git -C $wt rev-parse -q --verify MERGE_HEAD *> $null
    $leftOpen = ($LASTEXITCODE -eq 0)
    Check ($r.code -eq 0 -and $parents.Count -eq 3 -and -not $leftOpen) "mergeoptions '--no-commit' does not leave the merge open (exit $($r.code)): $($r.out.Trim())"
    if ($leftOpen) { & git -C $wt merge --abort *> $null }
    New-Side 'side/mo3' $initSha 'a.txt' 'mo3 side of a'
    & git -C $main config branch.feat/target.mergeoptions '-Xours'
    $before = Git-Out $wt @('rev-parse', 'HEAD')
    $r = Invoke-Merge $wt 'side/mo3'
    Check ($r.code -eq 1 -and $r.out -match 'reason=merge-aborted files="a.txt"' -and (Git-Out $wt @('rev-parse', 'HEAD')) -eq $before) "mergeoptions '-Xours' does not resolve the conflict silently (exit $($r.code)): $($r.out.Trim())"
    & git -C $main config --unset branch.feat/target.mergeoptions
    # `-c key=value` splits at the first `=`, so a branch named `mo=x` needs the override another way.
    $eqWt = Join-Path $tmp 'eq'
    & git -C $main worktree add -q -b 'mo=x' $eqWt $initSha
    New-Side 'side/mo4' $initSha 'mo4.txt' 'mo4'
    & git -C $main config 'branch.mo=x.mergeoptions' '-s ours'
    $r = Invoke-Merge $eqWt 'side/mo4'
    Check ($r.code -eq 0 -and (Test-Path -LiteralPath (Join-Path $eqWt 'mo4.txt'))) "mergeoptions of a branch named 'mo=x' is overridden too (exit $($r.code)): $($r.out.Trim())"

    # --- a conflicted file name cannot plant a status= token in the failed line -------------
    $injWt = Join-Path $tmp 'inj'
    $injName = 'q status=ok.txt'
    & git -C $main worktree add -q -b feat/inj $injWt $initSha
    Set-Content -LiteralPath (Join-Path $injWt $injName) 'target'
    & git -C $injWt add -- $injName
    & git -C $injWt commit -q -m 'inj target'
    New-Side 'side/inj' $initSha $injName 'side'
    $r = Invoke-Merge $injWt 'side/inj'
    Check ($r.code -eq 1 -and $r.out -match 'files="q status%3Dok.txt"' -and [regex]::Matches($r.out, 'status=').Count -eq 1) "a file name holding status= is encoded (exit $($r.code)): $($r.out.Trim())"

    # --- conflict: aborted, worktree back to its pre-merge state ----------------------
    & git -C $main worktree add -q (Join-Path $tmp 'conf') feat/conf
    Set-Content -LiteralPath (Join-Path $tmp 'conf\a.txt') 'conflicting'
    & git -C (Join-Path $tmp 'conf') commit -q -am 'conflict side'
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'other side'
    & git -C $wt commit -q -am 'other side'
    $before = Git-Out $wt @('rev-parse', 'HEAD')
    $r = Invoke-Merge $wt 'feat/conf'
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=merge-aborted files="a.txt" detail="CONFLICT \(content\): Merge conflict in a.txt"') "a conflict exits 1, names the file and carries git's CONFLICT line (exit $($r.code)): $($r.out)"
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
