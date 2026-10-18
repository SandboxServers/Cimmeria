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
    & git -C $main commit -q -m 'first'
    $first = (& git -C $main rev-parse HEAD).Trim()
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'two'
    & git -C $main commit -q -am 'second'
    $second = (& git -C $main rev-parse HEAD).Trim()
    & git -C $main worktree add -q -b feat/wt $wt
    & git -C $main remote add origin (Join-Path $tmp 'origin.git')
    & git -C $main tag v9 $first
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
    foreach ($plus in '+x', '+main') {
        $r = Invoke-BranchAt $main $plus $first
        Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') "refuses the name '$plus' (a leading + is a force refspec when pushed)"
    }
    foreach ($hex in 'abc1234', ('a' * 40), $second.Substring(0, 7)) {
        $r = Invoke-BranchAt $main $hex $first
        Check ($r.code -eq 2 -and $r.out -match 'looks like a commit hash') "refuses the hash-like name '$hex'"
    }
    $r = Invoke-BranchAt $main 'v9' $second
    Check ($r.code -eq 2 -and $r.out -match 'existing tag') 'refuses a name equal to an existing tag'
    $r = Invoke-BranchAt $main 'bad..name' $first
    Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') 'refuses an invalid branch name'
    foreach ($amb in '@', 'HEAD', 'FETCH_HEAD', 'origin/main', 'origin', 'tags/x', 'heads/x', 'remotes/x', 'refs/heads/zz') {
        $r = Invoke-BranchAt $main $amb $first
        Check ($r.code -eq 2 -and $r.out -match 'status=refused') "refuses the ambiguous name '$amb' (exit $($r.code))"
    }
    foreach ($pseudo in 'REBASE_HEAD', 'AUTO_MERGE', 'REVERT_HEAD', 'BISECT_HEAD', 'head', 'Orig_Head', 'Rebase_Head', 'Auto_Merge', 'Revert_Head') {
        $r = Invoke-BranchAt $main $pseudo $first
        Check ($r.code -eq 2 -and $r.out -match 'ambiguous-branch-name') "refuses the pseudo-ref shaped name '$pseudo' (exit $($r.code))"
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
    foreach ($ok in 'topic', 'Zed', 'topic_x', 'v1') {
        $r = Invoke-BranchAt $main $ok $first
        Check ($r.code -eq 0 -and $r.out -match "status=ok branch=$ok ") "an ordinary name '$ok' stays valid (exit $($r.code)): $($r.out.Trim())"
    }

    # --- a case variant of an existing branch is refused, also once its ref is packed -------
    # `topic` exists (created above). On NTFS a loose refs/heads/Topic would shadow the packed
    # refs/heads/topic and silently re-point it.
    & git -C $main pack-refs --all
    $topicBefore = (& git -C $main rev-parse refs/heads/topic).Trim()
    foreach ($variant in 'Topic', 'tOPIC') {
        $r = Invoke-BranchAt $main $variant $second
        Check ($r.code -eq 2 -and $r.out -match 'branch-exists') "refuses '$variant' while topic exists (exit $($r.code)): $($r.out.Trim())"
    }
    foreach ($df in 'topic/sub', 'Topic/sub') {
        $r = Invoke-BranchAt $main $df $second
        Check ($r.code -eq 2 -and $r.out -match 'branch-exists') "refuses '$df' under the existing branch topic (exit $($r.code))"
    }
    $r = Invoke-BranchAt $main 'feat' $second
    Check ($r.code -eq 2 -and $r.out -match 'branch-exists') "refuses 'feat', a path prefix of the existing feat/wt (exit $($r.code))"
    Check ((& git -C $main rev-parse refs/heads/topic).Trim() -eq $topicBefore -and (& git -C $main rev-parse topic).Trim() -eq $topicBefore) 'topic did not move'
    Check ($null -eq (& git -C $main for-each-ref refs/heads/Topic refs/heads/TOPIC)) 'no case-variant ref was created'

    # A failed `git branch` reports git's last line.
    $gitDir = (& git -C $main rev-parse --absolute-git-dir).Trim()
    New-Item -ItemType Directory (Join-Path $gitDir 'refs/heads') -Force | Out-Null
    New-Item -ItemType File (Join-Path $gitDir 'refs/heads/lockme.lock') | Out-Null
    $r = Invoke-BranchAt $main 'lockme' $first
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=git-branch detail="[^"]*cannot lock') "a failed git branch carries git's lock error as detail (exit $($r.code)): $($r.out.Trim())"
    Remove-Item -LiteralPath (Join-Path $gitDir 'refs/heads/lockme.lock')

    # --- a name that already resolves (refs/<name>) would shadow the branch ------------------
    # git's rev lookup tries refs/<name> before refs/heads/<name>: with a stash present a branch
    # `stash` would never be what `stash` names.
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'stashed edit'
    & git -C $main stash -q
    & git -C $main update-ref refs/bisect/good-x $first
    foreach ($shadowed in 'stash', 'bisect/good-x') {
        $r = Invoke-BranchAt $main $shadowed $second
        Check ($r.code -eq 2 -and $r.out -match 'ambiguous-branch-name: \S+ already resolves') "refuses '$shadowed', which refs/$shadowed already answers (exit $($r.code)): $($r.out.Trim())"
        Check ($null -eq (& git -C $main for-each-ref "refs/heads/$shadowed")) "no branch $shadowed was created"
    }
    & git -C $main stash drop -q
    & git -C $main update-ref -d refs/bisect/good-x

    # GIT_DIR in the environment must not move the branch into another repository.
    $other = Join-Path $tmp 'other'
    & git clone -q $main $other
    $env:GIT_DIR = (Join-Path $other '.git')
    try { $r = Invoke-BranchAt $main 'env/probe' $first }
    finally { Remove-Item Env:GIT_DIR -ErrorAction SilentlyContinue }
    Check ($r.code -eq 0) "creates the branch with GIT_DIR set (exit $($r.code)): $($r.out.Trim())"
    Check ($null -ne (& git -C $main rev-parse --verify --quiet refs/heads/env/probe)) 'the branch is in the repository named by -C'
    Check ($null -eq (& git -C $other rev-parse --verify --quiet refs/heads/env/probe)) 'the repository GIT_DIR named got no branch'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all branch-at checks passed'
exit 0
