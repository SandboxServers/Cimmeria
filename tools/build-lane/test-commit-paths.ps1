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

# Runs the script with a hand-built command line (no -Paths: joining), for binding checks.
function Invoke-CommitRaw([string[]]$argv) {
    $out = (& pwsh -NoProfile -File $script @argv 2>&1 | Out-String)
    return @{ code = $LASTEXITCODE; out = $out }
}

$tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("commit-paths-test-" + [guid]::NewGuid().ToString('N'))
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
    # Set-Content writes CRLF on Windows; the isolated config no longer carries the system autocrlf.
    & git -C $main config core.whitespace 'cr-at-eol'
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
    # A rooted path used to dodge the directory guard: Join-Path put it under -C, found nothing,
    # and `git add <worktree root>` staged every file below it.
    foreach ($abs in $wt, (Join-Path $wt 'sub'), (Join-Path $wt 'a.txt'), ($wt -replace '\\', '/'), '/a.txt', '//server/share/a.txt', 'C:a.txt') {
        $r = Invoke-Commit $wt @($abs)
        Check ($r.code -eq 2 -and $r.out -match 'rooted-path') "refused rooted path '$abs' (exit $($r.code)): $($r.out.Trim())"
    }
    Check ((& $head) -eq $before) 'no refusal moved HEAD'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'no refusal staged anything'

    # A space-separated list used to bind the second path to -Trailers and commit only the first.
    $r = Invoke-CommitRaw @('-C', $wt, '-Message', 'subject line', '-Paths', 'a.txt', 'sub/c.txt')
    Check ($r.code -ne 0 -and $r.out -notmatch 'status=ok') "-Paths a b (space separated) is an error, not a partial commit (exit $($r.code))"
    Check ((& $head) -eq $before) 'the space-separated list did not commit'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'the space-separated list staged nothing'

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

    # A named path that is already staged is refused too, and stays staged: the failure paths
    # unstage what they added, so letting it through would drop the caller's staging on failure.
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'bad   '
    & git -C $wt add -- a.txt
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 2 -and $r.out -match 'other-staged-files: a.txt') "refuses a named path that is already staged (exit $($r.code)): $($r.out.Trim())"
    Check ((@(& git -C $wt diff --cached --name-only) -join ',') -eq 'a.txt') 'the earlier staging of the named path is left as found'
    & git -C $wt restore --staged -- a.txt
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'two'

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
    & git -C $wt reset -q
    & git -C $wt checkout -q -- r.txt
    Remove-Item -LiteralPath (Join-Path $wt 'r2.txt') -ErrorAction SilentlyContinue

    # --- a deleted file can be committed by name -----------------------------------------
    Remove-Item -LiteralPath (Join-Path $wt 'r.txt')
    $r = Invoke-Commit $wt @('r.txt')
    Check ($r.code -eq 0 -and $r.out -match 'status=ok') "commits the deletion of a named file (exit $($r.code)): $($r.out.Trim())"
    $deleted = @(& git -C $wt show --name-status --format= HEAD)
    Check ($deleted.Count -eq 1 -and $deleted[0] -match '^D\s+r\.txt$') "the commit is the deletion of r.txt: $($deleted -join ';')"

    # --- naming a deleted directory must not stage every deletion beneath it -----------------
    New-Item -ItemType Directory (Join-Path $wt 'd') | Out-Null
    Set-Content -LiteralPath (Join-Path $wt 'd\x.txt') 'x'
    Set-Content -LiteralPath (Join-Path $wt 'd\y.txt') 'y'
    & git -C $wt add -- d
    & git -C $wt commit -q -m 'add d'
    Remove-Item -LiteralPath (Join-Path $wt 'd') -Recurse -Force
    $before = & $head
    $r = Invoke-Commit $wt @('d')
    Check ($r.code -eq 2 -and $r.out -match 'widened-pathspec') "refuses a deleted directory name (exit $($r.code)): $($r.out.Trim())"
    Check ((& $head) -eq $before) 'the deleted-directory refusal did not commit'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'the deleted-directory refusal left nothing staged'
    $r = Invoke-Commit $wt @('d/x.txt')
    $deleted = @(& git -C $wt show --name-status --format= HEAD)
    Check ($r.code -eq 0 -and $deleted.Count -eq 1 -and $deleted[0] -match '^D\s+d/x\.txt$') "a deleted file inside the directory can still be named (exit $($r.code)): $($deleted -join ';')"
    & git -C $wt reset -q --hard HEAD

    # --- a junction inside the worktree must not carry a file from outside it --------------
    $outside = Join-Path $tmp 'outside'
    New-Item -ItemType Directory $outside | Out-Null
    Set-Content -LiteralPath (Join-Path $outside 'secret.txt') 'outside the worktree'
    $junction = Join-Path $wt 'jd'
    New-Item -ItemType Junction -Path $junction -Target $outside | Out-Null
    $before = & $head
    $r = Invoke-Commit $wt @('jd/secret.txt')
    [System.IO.Directory]::Delete($junction)
    Check ($r.code -eq 2 -and $r.out -match 'reparse-point') "refuses a path below a junction (exit $($r.code)): $($r.out.Trim())"
    Check ((& $head) -eq $before) 'the junction refusal did not commit'
    Check (@(& git -C $wt diff --cached --name-only).Count -eq 0) 'the junction refusal left nothing staged'

    # --- GIT_DIR / GIT_WORK_TREE in the environment cannot retarget the commit ----------------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'env change'
    $mainTip = (& git -C $main rev-parse refs/heads/main).Trim()
    $env:GIT_DIR = (Join-Path $main '.git')
    $env:GIT_WORK_TREE = $wt
    try { $r = Invoke-Commit $wt @('a.txt') }
    finally { Remove-Item Env:GIT_DIR, Env:GIT_WORK_TREE -ErrorAction SilentlyContinue }
    Check ((& git -C $main rev-parse refs/heads/main).Trim() -eq $mainTip) 'a GIT_DIR pointing at the main checkout did not move main'
    Check ($r.code -eq 0 -and $r.out -match 'branch=feat/x') "the commit still lands on the worktree's branch (exit $($r.code)): $($r.out.Trim())"

    # --- the branch compare is case-sensitive --------------------------------------------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'case change'
    $before = & $head
    $r = Invoke-Commit $wt @('a.txt') @() 'FEAT/X'
    Check ($r.code -eq 2 -and $r.out -match 'branch-mismatch') "-ExpectBranch FEAT/X does not match feat/x (exit $($r.code))"
    Check ((& $head) -eq $before) 'the case mismatch did not commit'

    # --- whitespace: a refusal only where the repo configures a whitespace policy -----------
    # core.whitespace is set above, so a trailing-space line was refused. Without it the same
    # content commits and a warning is printed: plain `git commit` would accept it too.
    & git -C $main config --unset core.whitespace
    # A whitespace attribute is a policy too: that file stays a refusal.
    $attrFile = Join-Path $wt '.gitattributes'
    Set-Content -LiteralPath $attrFile '*.ws whitespace=trailing-space'
    Set-Content -LiteralPath (Join-Path $wt 'w.ws') 'attr   '
    $before = & $head
    $r = Invoke-Commit $wt @('w.ws')
    Remove-Item -LiteralPath $attrFile
    Check ($r.code -eq 2 -and $r.out -match 'diff-check') "a whitespace attribute keeps the whitespace refusal (exit $($r.code)): $($r.out.Trim())"
    Check ((& $head) -eq $before -and @(& git -C $wt diff --cached --name-only).Count -eq 0) 'the attribute refusal committed and staged nothing'
    Remove-Item -LiteralPath (Join-Path $wt 'w.ws')
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'trailing   '
    $r = Invoke-Commit $wt @('a.txt')
    Check ($r.code -eq 0 -and $r.out -match 'status=ok' -and $r.out -match 'warning') "without core.whitespace a whitespace error is a warning, not a refusal (exit $($r.code)): $($r.out.Trim())"

    # --- GIT_OBJECT_DIRECTORY would send the new objects to a store the branch cannot read -------
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'object dir'
    $objDir = Join-Path $tmp 'odd-objects'
    New-Item -ItemType Directory $objDir | Out-Null
    $env:GIT_OBJECT_DIRECTORY = $objDir
    try { $r = Invoke-Commit $wt @('a.txt') }
    finally { Remove-Item Env:GIT_OBJECT_DIRECTORY -ErrorAction SilentlyContinue }
    $tipOk = & git -C $wt rev-parse --verify --quiet 'HEAD^{commit}'
    Check ($r.code -eq 0 -and $null -ne $tipOk -and $LASTEXITCODE -eq 0) "a GIT_OBJECT_DIRECTORY in the environment does not strand the commit (exit $($r.code)): $($r.out.Trim())"

    # --- a merge in progress is not concluded by a path commit ------------------------------
    # `git merge --no-commit` leaves MERGE_HEAD with an index equal to HEAD; a plain `git commit`
    # would then record a two-parent commit holding only the edited file.
    $mergeSide = (& git -C $wt commit-tree 'HEAD^{tree}' -p HEAD -m 'side').Trim()
    & git -C $wt merge -q --no-commit --no-ff -s ours $mergeSide *> $null
    Set-Content -LiteralPath (Join-Path $wt 'a.txt') 'edited during a merge'
    $before = & $head
    $r = Invoke-Commit $wt @('a.txt')
    & git -C $wt rev-parse -q --verify MERGE_HEAD *> $null
    $mergeStillOpen = ($LASTEXITCODE -eq 0)
    Check ($r.code -eq 2 -and $r.out -match 'merge-in-progress') "refuses while a merge is in progress (exit $($r.code)): $($r.out.Trim())"
    Check ((& $head) -eq $before -and $mergeStillOpen) 'the merge-in-progress refusal committed nothing and left the merge open'
    & git -C $wt merge --abort *> $null
}
finally {
    Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
}

if ($script:failed -gt 0) { Write-Host "$($script:failed) check(s) failed"; exit 1 }
Write-Host 'all commit-paths checks passed'
exit 0
