<#
.SYNOPSIS
    Print the lab CLI version and the daemon version.

.DESCRIPTION
    The CLI version is the short git sha in cli\VERSION (written when the
    CLI is installed), else 'dev'. The daemon version comes from GET /status,
    or 'daemon: not running' when the daemon or its endpoint is down.

.EXAMPLE
    pwsh tools/lab/lab.ps1 version
#>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'common.ps1')

$sha = 'dev'
$versionFile = Join-Path $PSScriptRoot 'VERSION'
if (Test-Path -LiteralPath $versionFile) {
    $text = "$(Get-Content -LiteralPath $versionFile -TotalCount 1)".Trim()
    if ($text) { $sha = $text }
}
Write-Host "lab CLI $sha"

$status = Get-LabStatus
if ($status) {
    Write-Host "daemon $($status.daemon.version)"
} else {
    Write-Host 'daemon: not running'
}
exit 0
