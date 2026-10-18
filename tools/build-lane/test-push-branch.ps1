<#
.SYNOPSIS
    Tests for push-branch.ps1 in a throwaway main checkout, one linked worktree and a local bare
    remote named origin. No network and no Pester needed; exits non-zero on a failure. Runs no cargo.

        pwsh -NoProfile -File tools/build-lane/test-push-branch.ps1
#>
$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'push-branch.ps1'

$script:failed = 0
function Check([bool]$cond, [string]$what) {
    if ($cond) { Write-Host "ok   $what" } else { Write-Host "FAIL $what"; $script:failed++ }
}

# Runs the script in-process and returns its exit code and printed text.
function Invoke-Push([string]$wt, [string]$branch, [switch]$force, [string]$expect = '') {
    $params = @{ C = $wt; Branch = $branch }
    if ($force) { $params.Force = $true }
    if ($expect) { $params.ExpectRemoteSha = $expect }
    $out = (& $script @params 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("push-branch-test-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory $tmp | Out-Null
# Isolate git from the developer's global and system config (commit.gpgsign, core.hooksPath,
# rebase.updateRefs ...): both point at an empty file inside the throwaway directory.
$emptyConfig = Join-Path $tmp 'empty.gitconfig'
New-Item -ItemType File $emptyConfig | Out-Null
$env:GIT_CONFIG_GLOBAL = $emptyConfig
$env:GIT_CONFIG_SYSTEM = $emptyConfig
try {
    $bare = Join-Path $tmp 'origin.git'
    $main = Join-Path $tmp 'main'
    $wt = Join-Path $tmp 'wt'
    & git init -q --bare -b main $bare
    New-Item -ItemType Directory $main | Out-Null
    & git -C $main init -q -b main
    & git -C $main config user.email 'test@example.invalid'
    & git -C $main config user.name 'test'
    Set-Content -LiteralPath (Join-Path $main 'a.txt') 'one'
    & git -C $main add -- a.txt
    & git -C $main commit -q -m 'init'
    & git -C $main remote add origin $bare
    & git -C $main push -q -u origin main
    & git -C $main worktree add -q -b feat/x $wt
    Set-Content -LiteralPath (Join-Path $wt 'b.txt') 'two'
    & git -C $wt add -- b.txt
    & git -C $wt commit -q -m 'feat one'
    $remoteRef = { (& git -C $bare rev-parse --verify --quiet refs/heads/feat/x) }

    # --- refusals push nothing --------------------------------------------------------
    $r = Invoke-Push $main 'main'
    Check ($r.code -eq 2 -and $r.out -match 'main-checkout') 'refuses the main checkout'
    $r = Invoke-Push (Join-Path $wt 'nope') 'feat/x'
    Check ($r.code -eq 2 -and $r.out -match 'not-a-directory') 'refuses a path that is not a directory'
    $r = Invoke-Push $wt 'main'
    Check ($r.code -eq 2 -and $r.out -match 'protected-branch: main') 'refuses to push main'
    $r = Invoke-Push $wt 'master'
    Check ($r.code -eq 2 -and $r.out -match 'protected-branch: master') 'refuses to push master'
    $r = Invoke-Push $wt 'other'
    Check ($r.code -eq 2 -and $r.out -match 'not-current-branch') 'refuses a branch that is not the current one'
    $r = Invoke-Push $wt 'bad..name'
    Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') 'refuses an invalid branch name'
    Check ($null -eq (& $remoteRef)) 'no refusal created the remote branch'

    & git -C $wt checkout -q --detach
    $r = Invoke-Push $wt 'feat/x'
    Check ($r.code -eq 2 -and $r.out -match 'detached-head') 'refuses a detached HEAD'
    & git -C $wt checkout -q feat/x

    # --- a plain push publishes the branch and sets its upstream ----------------------
    $r = Invoke-Push $wt 'feat/x'
    $head = (& git -C $wt rev-parse HEAD).Trim()
    Check ($r.code -eq 0 -and $r.out -match "status=ok branch=feat/x sha=$head") "plain push ok (exit $($r.code)): $($r.out.Trim())"
    Check ((& $remoteRef) -eq $head) 'the remote branch is at HEAD'
    Check ((& git -C $wt rev-parse --abbrev-ref '@{u}').Trim() -eq 'origin/feat/x') 'the upstream is origin/feat/x'

    # --- a rewritten branch needs -Force; a plain push is rejected -------------------
    & git -C $wt commit -q --amend -m 'feat one rewritten'
    $rewritten = (& git -C $wt rev-parse HEAD).Trim()
    $r = Invoke-Push $wt 'feat/x'
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=git-push') "a plain push of a rewritten branch fails (exit $($r.code))"
    Check ((& $remoteRef) -eq $head) 'the failed plain push left the remote unchanged'

    # -Force needs the remote sha the caller last saw: a bare lease is defeated by any fetch.
    $r = Invoke-Push $wt 'feat/x' -force
    Check ($r.code -eq 2 -and $r.out -match 'force-needs-expect-remote-sha') "-Force without -ExpectRemoteSha is refused (exit $($r.code)): $($r.out.Trim())"
    $r = Invoke-Push $wt 'feat/x' -force -expect $head.ToUpper()
    Check ($r.code -eq 2 -and $r.out -match 'force-needs-expect-remote-sha') '-ExpectRemoteSha must be 40 lowercase hex digits'
    $r = Invoke-Push $wt 'feat/x' -expect $head
    Check ($r.code -eq 2 -and $r.out -match 'expect-remote-sha-needs-force') '-ExpectRemoteSha without -Force is refused'
    Check ((& $remoteRef) -eq $head) 'the -Force refusals left the remote unchanged'

    $r = Invoke-Push $wt 'feat/x' -force -expect $head
    Check ($r.code -eq 0 -and $r.out -match "sha=$rewritten") "-Force with the current remote sha pushes the rewritten branch (exit $($r.code))"
    Check ((& $remoteRef) -eq $rewritten) 'the remote branch is at the rewritten HEAD'

    # --- the lease holds: someone else's push to the remote is not overwritten ---------
    # A second clone advances origin/feat/x after our last push. The lease still expects our tip,
    # so -Force must be rejected; a bare --force would overwrite it and this check would fail.
    $other = Join-Path $tmp 'other'
    & git clone -q $bare $other
    & git -C $other config user.email 'test@example.invalid'
    & git -C $other config user.name 'test'
    & git -C $other checkout -q feat/x
    Set-Content -LiteralPath (Join-Path $other 'c.txt') 'someone else'
    & git -C $other add -- c.txt
    & git -C $other commit -q -m 'remote advances'
    & git -C $other push -q origin feat/x
    $theirs = (& git -C $other rev-parse HEAD).Trim()
    & git -C $wt commit -q --amend -m 'feat one rewritten again'
    $r = Invoke-Push $wt 'feat/x' -force -expect $rewritten
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=git-push') "-Force is rejected when the remote moved (exit $($r.code)): $($r.out.Trim())"
    Check ((& $remoteRef) -eq $theirs) 'the rejected -Force left the other push in place'

    # A fetch refreshes origin/feat/x, which a bare --force-with-lease then trusts and overwrites.
    # The explicit expected sha is the one the caller saw before the remote moved, so it still fails.
    & git -C $wt fetch -q origin
    Check ((& git -C $wt rev-parse origin/feat/x).Trim() -eq $theirs) 'the fetch refreshed the remote-tracking ref'
    $r = Invoke-Push $wt 'feat/x' -force -expect $rewritten
    Check ($r.code -eq 1 -and $r.out -match 'status=failed reason=git-push') "-Force after a fetch is still rejected (exit $($r.code)): $($r.out.Trim())"
    Check ((& $remoteRef) -eq $theirs) 'the fetch-then-force left the other push in place'
    $r = Invoke-Push $wt 'feat/x' -force -expect $theirs
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "-Force with the sha the caller really saw succeeds (exit $($r.code))"
    Check ((& $remoteRef) -eq (& git -C $wt rev-parse HEAD).Trim()) 'the remote is at the local tip after the informed force'

    # --- the full refspec is pushed: a tag named like the branch must not make the push ambiguous ----
    & git -C $main worktree add -q -b amb (Join-Path $tmp 'wtamb') main
    $wtAmb = Join-Path $tmp 'wtamb'
    Set-Content -LiteralPath (Join-Path $wtAmb 'amb.txt') 'amb'
    & git -C $wtAmb add -- amb.txt
    & git -C $wtAmb commit -q -m 'amb commit'
    & git -C $wtAmb tag amb main
    $r = Invoke-Push $wtAmb 'amb'
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "pushes branch amb although a tag amb exists (exit $($r.code)): $($r.out.Trim())"
    Check ((& git -C $bare rev-parse refs/heads/amb).Trim() -eq (& git -C $wtAmb rev-parse HEAD).Trim()) 'the remote branch amb is at the branch tip, not the tag'

    # --- the branch name compare is case-sensitive -----------------------------------------
    $r = Invoke-Push $wt 'FEAT/X'
    Check ($r.code -eq 2 -and $r.out -match 'not-current-branch') "refuses -Branch FEAT/X for the branch feat/x (exit $($r.code))"

    # --- a name that starts with + is refused: pushed raw it is a force refspec ------------
    # Remote `victim` holds a commit that local branch `+victim` does not contain. Pushing the raw
    # name `+victim` would force-push `victim` over it and bypass the main/master check.
    & git -C $bare branch victim $theirs
    $wtPlus = Join-Path $tmp 'wtplus'
    & git -C $main worktree add -q -b '+victim' $wtPlus main
    Set-Content -LiteralPath (Join-Path $wtPlus 'v.txt') 'v'
    & git -C $wtPlus add -- v.txt
    & git -C $wtPlus commit -q -m 'victim rewrite'
    foreach ($plus in '+victim', '+main') {
        if ($plus -eq '+main') { & git -C $wtPlus checkout -q -b '+main' }
        $r = Invoke-Push $wtPlus $plus
        Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') "refuses the branch '$plus' (exit $($r.code)): $($r.out.Trim())"
        $r = Invoke-Push $wtPlus $plus -force
        Check ($r.code -eq 2 -and $r.out -match 'bad-branch-name') "refuses the branch '$plus' with -Force too (exit $($r.code))"
    }
    Check ((& git -C $bare rev-parse refs/heads/victim).Trim() -eq $theirs) 'the remote victim branch was not overwritten'
    Check ($null -eq (& git -C $bare rev-parse --verify --quiet refs/heads/+victim)) 'no +victim branch was created on the remote'
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all push-branch checks passed'
exit 0
