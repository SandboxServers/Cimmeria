<#
.SYNOPSIS
    Show each lab instance's account, profile seed and client, or write the extra account files.

.DESCRIPTION
    Runs tools/lab/instances.ps1 with the same arguments and passes its exit
    code on: `lab instances status` prints one line per instance, and
    `lab instances init [-InstallDir <dir>] [-Count <n>] [-Force]` writes the
    account files for p2..p<Count>. The runbook is docs/guides/live-research-lab.md.

.EXAMPLE
    pwsh tools/lab/lab.ps1 instances status
    pwsh tools/lab/lab.ps1 instances init
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

pwsh -NoProfile -File "$PSScriptRoot\..\instances.ps1" @args
exit $LASTEXITCODE
