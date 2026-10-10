# Rebase a PR branch onto origin/main with git alone. PowerShell 7 twin of rebase-pr.sh;
# usage, exit codes and details are in rebase_pr.py.
#
#   pwsh tools/build-lane/rebase-pr.ps1 [--push] [--json] [-v] [--no-fetch] [--onto REF] <PR|branch|worktree>
$py = if ($env:PYTHON) { $env:PYTHON } else { 'python' }
& $py (Join-Path $PSScriptRoot 'rebase_pr.py') @args
exit $LASTEXITCODE
