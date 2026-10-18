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

# Runs the script the way ops does, as a child `pwsh -File`, and returns its exit code and printed
# text. In-process calls would skip how a real command line binds its parameters.
function Invoke-Rebase([string]$wt, [string]$onto, [string]$upstream = '') {
    $argv = @('-NoProfile', '-File', $script, '-C', $wt, '-Onto', $onto)
    if ($upstream) { $argv += @('-Upstream', $upstream) }
    $out = (& pwsh @argv 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("rebase-onto-test-" + [guid]::NewGuid().ToString('N'))
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
    $r = Invoke-Rebase $main $mainTip
    Check ($r.code -eq 2 -and $r.out -match 'main-checkout') 'refuses the main checkout'
    $r = Invoke-Rebase $wt ('0' * 40)
    Check ($r.code -eq 2 -and $r.out -match 'not-a-commit') 'refuses an -Onto that names no commit'
    foreach ($loose in 'main', 'HEAD', $mainTip.Substring(0, 12), $mainTip.ToUpper()) {
        $r = Invoke-Rebase $wt $loose
        Check ($r.code -eq 2 -and $r.out -match 'sha-not-full-40-hex') "refuses -Onto '$loose', which is not a full 40-hex SHA"
        $r = Invoke-Rebase $wt $mainTip $loose
        Check ($r.code -eq 2 -and $r.out -match 'sha-not-full-40-hex') "refuses -Upstream '$loose', which is not a full 40-hex SHA"
    }
    Set-Content -LiteralPath (Join-Path $wt 'dirty.txt') 'untracked'
    $before = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt $mainTip
    Check ($r.code -eq 2 -and $r.out -match 'dirty-tree') 'refuses a dirty tree'
    Check ((& git -C $wt rev-parse HEAD).Trim() -eq $before) 'the dirty refusal did not move HEAD'
    Remove-Item -LiteralPath (Join-Path $wt 'dirty.txt')

    # --- a clean rebase replays the branch onto the new tip --------------------------
    $r = Invoke-Rebase $wt $mainTip
    $tip = (& git -C $wt rev-parse HEAD).Trim()
    Check ($r.code -eq 0 -and $r.out -match "status=ok tip=$tip branch=feat/x") "rebases onto main (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wt merge-base HEAD main).Trim() -eq $mainTip) 'HEAD now contains main'
    Check (Test-Path -LiteralPath (Join-Path $wt 'b.txt')) 'the branch commit survived the rebase'
    Check ((& git -C $wt status --porcelain | Out-String).Trim() -eq '') 'the tree is clean after the rebase'

    $r = Invoke-Rebase $wt $mainTip
    Check ($r.code -eq 0 -and $r.out -match 'status=up-to-date') "a second rebase onto main is up to date (exit $($r.code))"

    # --- -Upstream must be an ancestor of HEAD ----------------------------------------
    Set-Content -LiteralPath (Join-Path $main 'd.txt') 'main'
    & git -C $main add -- d.txt
    & git -C $main commit -q -m 'main d'
    $newMain = (& git -C $main rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt $newMain $newMain
    Check ($r.code -eq 2 -and $r.out -match 'upstream-not-ancestor') 'refuses an -Upstream that HEAD does not contain'

    # --- a conflict is aborted and leaves the worktree as it was ---------------------
    Set-Content -LiteralPath (Join-Path $wt 'd.txt') 'feat'
    & git -C $wt add -- d.txt
    & git -C $wt commit -q -m 'feat d'
    $before = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt $newMain
    $gitDir = (& git -C $wt rev-parse --absolute-git-dir).Trim()
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=rebase-aborted branch=feat/x detail="[^"]*(?:CONFLICT|could not apply)') "a conflicting rebase fails, aborts and says why (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wt rev-parse HEAD).Trim() -eq $before) 'the conflict left HEAD where it was'
    Check ((& git -C $wt status --porcelain | Out-String).Trim() -eq '') 'the conflict left the tree clean'
    Check (-not (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-merge')) -and -not (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-apply'))) 'no rebase is left in progress'

    # --- a rebase left by someone else is refused, not aborted -------------------------
    # An interrupted rebase, then `checkout -f` back onto the branch and a new commit: HEAD is
    # attached and the tree clean, but the stale rebase-merge directory remains. Running the rebase
    # there fails on it, and an abort would reset the branch to that rebase's starting commit.
    & git -C $wt rebase $newMain *> $null
    Check (Test-Path -LiteralPath (Join-Path $gitDir 'rebase-merge')) 'setup: a hand-run rebase stopped on its conflict'
    & git -C $wt checkout -q -f feat/x
    Set-Content -LiteralPath (Join-Path $wt 'precious.txt') 'precious work'
    & git -C $wt add -- precious.txt
    & git -C $wt commit -q -m 'precious work'
    $precious = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wt $newMain (& git -C $wt rev-parse 'HEAD~2').Trim()
    Check ($r.code -eq 2 -and $r.out -match 'operation-in-progress: rebase-merge') "a stale rebase directory is refused (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wt rev-parse refs/heads/feat/x).Trim() -eq $precious -and (Test-Path -LiteralPath (Join-Path $wt 'precious.txt'))) 'the stale-rebase refusal kept the branch and its newest commit'
    & git -C $wt rebase --quit *> $null
    & git -C $wt merge -q --no-commit --no-ff -s ours $newMain *> $null
    $r = Invoke-Rebase $wt $newMain
    Check ($r.code -eq 2 -and $r.out -match 'operation-in-progress: MERGE_HEAD') "a merge in progress is refused (exit $($r.code)): $($r.out.Trim())"
    & git -C $wt merge --abort *> $null
    & git -C $wt reset -q --hard $before

    # --- guards that nothing else covers ------------------------------------------------
    $mainSub = Join-Path $main 'sub'
    New-Item -ItemType Directory $mainSub | Out-Null
    $mainHead = (& git -C $main rev-parse HEAD).Trim()
    $r = Invoke-Rebase $mainSub $newMain
    Check ($r.code -eq 2 -and $r.out -match 'not-the-worktree-root') "refuses a subdirectory of the main checkout (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $main rev-parse HEAD).Trim() -eq $mainHead) 'the main checkout did not move'
    $before = (& git -C $wt rev-parse HEAD).Trim()
    & git -C $wt checkout -q --detach
    $r = Invoke-Rebase $wt $mainTip
    Check ($r.code -eq 2 -and $r.out -match 'detached-head') "refuses a detached HEAD (exit $($r.code)): $($r.out.Trim())"
    & git -C $wt checkout -q feat/x
    Check ((& git -C $wt rev-parse HEAD).Trim() -eq $before) 'the guard refusals moved nothing'

    # GIT_DIR and GIT_OBJECT_DIRECTORY in the environment must not retarget the rebase.
    $wtE = Join-Path $tmp 'wte'
    $eBase = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main worktree add -q -b feat/e $wtE $eBase
    Set-Content -LiteralPath (Join-Path $wtE 'e1.txt') 'e1'
    & git -C $wtE add -- e1.txt
    & git -C $wtE commit -q -m 'e1'
    Set-Content -LiteralPath (Join-Path $main 'e0.txt') 'e0'
    & git -C $main add -- e0.txt
    & git -C $main commit -q -m 'main e0'
    $eOnto = (& git -C $main rev-parse HEAD).Trim()
    $objDir = Join-Path $tmp 'odd-objects'
    New-Item -ItemType Directory $objDir | Out-Null
    $env:GIT_DIR = (Join-Path $main '.git')
    $env:GIT_OBJECT_DIRECTORY = $objDir
    try { $r = Invoke-Rebase $wtE $eOnto }
    finally { Remove-Item Env:GIT_DIR, Env:GIT_OBJECT_DIRECTORY -ErrorAction SilentlyContinue }
    Check ($r.code -eq 0 -and $r.out -match 'status=ok' -and $r.out -match 'branch=feat/e') "rebases with GIT_DIR and GIT_OBJECT_DIRECTORY set (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $main rev-parse HEAD).Trim() -eq $eOnto) 'main did not move'
    Check ((& git -C $wtE merge-base HEAD $eOnto).Trim() -eq $eOnto) 'feat/e now sits on the new tip'

    # --- the -Upstream (--onto) path: success, stacked branches, and no hidden reset ----
    function New-Commit([string]$dir, [string]$file, [string]$msg) {
        Set-Content -LiteralPath (Join-Path $dir $file) $msg
        & git -C $dir add -- $file
        & git -C $dir commit -q -m $msg
    }
    $base = (& git -C $main rev-parse HEAD).Trim()
    $wtY = Join-Path $tmp 'wty'
    $wtZ = Join-Path $tmp 'wtz'
    & git -C $main worktree add -q -b feat/y $wtY $base
    New-Commit $wtY 'y.txt' 'y1'
    $yTip = (& git -C $wtY rev-parse HEAD).Trim()
    & git -C $main worktree add -q -b feat/z $wtZ $yTip
    New-Commit $wtZ 'z.txt' 'z1'
    New-Commit $main 'e.txt' 'main e'
    $newTip = (& git -C $main rev-parse HEAD).Trim()

    # The parent branch is rebased first; its tip changes, so the child still sits on the old y1.
    $r = Invoke-Rebase $wtY $newTip $base
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "-Upstream rebases the parent branch (exit $($r.code)): $($r.out.Trim())"
    $newY = (& git -C $wtY rev-parse HEAD).Trim()
    Check ((& git -C $wtY rev-parse 'HEAD~1').Trim() -eq $newTip) 'the parent branch now sits on the new main tip'

    # The child replays only z1 on top of the rewritten parent, not the old y1 again.
    $r = Invoke-Rebase $wtZ $newY $yTip
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "-Upstream rebases a stacked branch (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wtZ rev-parse 'HEAD~1').Trim() -eq $newY) 'the stacked branch sits directly on the new parent tip'
    $subjects = @(& git -C $wtZ log --format=%s "$newTip..HEAD")
    Check (($subjects -join ',') -eq 'z1,y1') "the stacked branch holds z1 and one y1 only: $($subjects -join ',')"

    # -Upstream equal to HEAD would replay nothing and move the branch to -Onto: a hidden reset.
    $before = (& git -C $wtZ rev-parse HEAD).Trim()
    $r = Invoke-Rebase $wtZ $base $before
    Check ($r.code -eq 2 -and $r.out -match 'nothing-to-replay') '-Upstream equal to HEAD is refused'
    Check ((& git -C $wtZ rev-parse HEAD).Trim() -eq $before) 'the nothing-to-replay refusal did not move HEAD'

    # An -Onto already inside HEAD with an -Upstream above it would drop the commits between them
    # (a partial reset): `--onto <old> <newer>` keeps only the commits after <newer>.
    $r = Invoke-Rebase $wtZ $base $newY
    Check ($r.code -eq 2 -and $r.out -match 'would-drop-commits') "-Onto below -Upstream is refused (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $wtZ rev-parse HEAD).Trim() -eq $before) 'the would-drop-commits refusal did not move HEAD'
    Check (Test-Path -LiteralPath (Join-Path $wtZ 'y.txt')) 'the commits that would have been dropped are still there'

    # A merge commit on the branch survives the rebase (--rebase-merges).
    $wtM = Join-Path $tmp 'wtm'
    $wtS = Join-Path $tmp 'wts'
    $mBase = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main worktree add -q -b feat/m $wtM $mBase
    New-Commit $wtM 'm1.txt' 'm1'
    & git -C $main worktree add -q -b side $wtS $mBase
    New-Commit $wtS 's1.txt' 's1'
    & git -C $wtM merge -q --no-ff -m 'merge side' side
    New-Commit $main 'f.txt' 'main f'
    $mTip = (& git -C $main rev-parse HEAD).Trim()
    Check (@(& git -C $wtM rev-list --merges "$mBase..HEAD").Count -eq 1) 'setup: the branch holds one merge commit'
    $r = Invoke-Rebase $wtM $mTip $mBase
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "a branch with a merge commit rebases (exit $($r.code)): $($r.out.Trim())"
    Check (@(& git -C $wtM rev-list --merges "$mTip..HEAD").Count -eq 1) 'the merge commit survived the rebase'

    # rebase.updateRefs=true would move other local branches that point into the rebased range.
    $wtU = Join-Path $tmp 'wtu'
    & git -C $main worktree add -q -b feat/u $wtU $mTip
    New-Commit $wtU 'u1.txt' 'u1'
    & git -C $wtU branch inner
    $inner = (& git -C $main rev-parse refs/heads/inner).Trim()
    New-Commit $wtU 'u2.txt' 'u2'
    New-Commit $main 'g.txt' 'main g'
    $gTip = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main config rebase.updateRefs true
    # A tag with the branch's name makes `rev-parse --abbrev-ref HEAD` answer `heads/feat/u`.
    & git -C $main tag feat/u $mTip
    $r = Invoke-Rebase $wtU $gTip
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "rebases with rebase.updateRefs=true set (exit $($r.code)): $($r.out.Trim())"
    Check ($r.out -match 'branch=feat/u(?:\s|$)') "the branch is named plainly although a tag has its name: $($r.out.Trim())"
    Check ((& git -C $wtU merge-base HEAD $gTip).Trim() -eq $gTip) 'setup: the branch now sits on the new main tip'
    Check ((& git -C $main rev-parse refs/heads/inner).Trim() -eq $inner) 'the unrelated branch inside the range was not moved'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all rebase-onto checks passed'
exit 0
