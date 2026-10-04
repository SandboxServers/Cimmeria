<#
.SYNOPSIS
    `headersHelper` for the shared lab daemon's .mcp.json entry: prints
    {"Authorization":"Bearer <CIMMERIA_LAB_DAEMON_TOKEN>"} on stdout.

.DESCRIPTION
    Reads the token from the *user* environment (the registry), not just
    this process, so a token that `tools/lab/daemon.ps1 install` just
    generated works without restarting Claude Code. The token never lands
    in .mcp.json.
#>
$ErrorActionPreference = 'Stop'
$name = 'CIMMERIA_LAB_DAEMON_TOKEN'
$token = [Environment]::GetEnvironmentVariable($name, 'User')
if (-not $token) { $token = [Environment]::GetEnvironmentVariable($name, 'Process') }
if (-not $token) {
    [Console]::Error.WriteLine("$name is not set; run: pwsh tools/lab/daemon.ps1 install")
    exit 1
}
@{ Authorization = "Bearer $token" } | ConvertTo-Json -Compress
