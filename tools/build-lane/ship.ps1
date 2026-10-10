# The mechanical end of a packet in one call, with one line of output. PowerShell 7 twin
# of ship.sh; usage, exit codes and details are in ship.py.
#
#   pwsh tools/build-lane/ship.ps1 pr -C <worktree> (-m MSG | -F FILE) [--title T] [--body-file F] ...
#   pwsh tools/build-lane/ship.ps1 merge <PR> [--retire NAME] [--timeout 30m] [--no-wait] [-v]
$py = if ($env:PYTHON) { $env:PYTHON } else { 'python' }
& $py (Join-Path $PSScriptRoot 'ship.py') @args
exit $LASTEXITCODE
