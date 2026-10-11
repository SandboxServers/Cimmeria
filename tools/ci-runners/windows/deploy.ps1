#!/usr/bin/env pwsh
# Sets up or updates the Windows runner pool on node3 over ssh: copies setup.ps1, the
# job hooks and the egress proxy, hands over the secrets, and runs setup.ps1 there as
# admin. See the "Windows pool" section of tools/ci-runners/README.md.
#
#   pwsh tools/ci-runners/windows/deploy.ps1                  # set up or update
#   pwsh tools/ci-runners/windows/deploy.ps1 -EgressExit host:port
#   pwsh tools/ci-runners/windows/deploy.ps1 -Stop            # stop the runner services
#
# Secrets never touch this machine's disk or the command line. The registration token
# (minted through `gh`, repo admin needed, valid an hour) and the compile cache's
# password (read from the cache host's .env) go over ssh stdin into a file in an
# admin-only directory on node3, which setup.ps1 deletes when it ends (and this script
# again in any case). The cache password and endpoint, and the egress exit, are kept on
# node3 in an admin-only file, so a later deploy without them keeps working.
[CmdletBinding()]
param(
    [string]$Ssh = 'cimmeria-node3',
    [string]$Node = 'node3',
    [string]$Root = 'F:\gha',
    [int]$Runners = 3,
    [int]$Jobs = 10,
    [string]$Repo = 'SandboxServers/Cimmeria',
    # host:port of the exit proxy for 57.0.0.0/8 (README, "Colo egress"). Empty keeps
    # the value node3 already has.
    [string]$EgressExit = $env:CIMMERIA_EGRESS_EXIT,
    # ssh alias of the Linux node whose compose `cache` (Redis) is the shared sccache
    # backend, and that compose project's directory. '' keeps what node3 already has.
    [string]$CacheHost = 'cimmeria-node2',
    [string]$CacheDir = '/var/lib/gha-runners',
    [switch]$Stop
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$Pwsh = 'C:\Program Files\PowerShell\7\pwsh.exe'
# Runs a script on node3 under pwsh 7 (the ssh default shell there is Windows
# PowerShell 5.1). -Stdin is piped to it, for secrets.
function Invoke-Node([string]$Script, [string]$What, [string]$Stdin = $null) {
    $enc = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($Script))
    # Text output: native commands' stderr would otherwise come back as CLIXML noise.
    $cmd = "& '$Pwsh' -NoProfile -NonInteractive -OutputFormat Text -EncodedCommand $enc"
    if ($null -ne $Stdin) { $Stdin | ssh $Ssh $cmd } else { ssh $Ssh $cmd }
    if ($LASTEXITCODE -ne 0) { throw "${What} failed (ssh $Ssh, exit $LASTEXITCODE)" }
}

if ($Stop) {
    Invoke-Node "Get-Service 'actions.runner.*' | Stop-Service -PassThru | ForEach-Object { `"`$(`$_.Name): `$(`$_.Status)`" }" 'stopping the runner services'
    return
}

$src = $PSScriptRoot
$channel = (Select-String -LiteralPath "$src/../../../rust-toolchain.toml" -Pattern '^channel *= *"([^"]+)"').Matches[0].Groups[1].Value
if (-not $channel) { throw 'no channel in rust-toolchain.toml' }

$deploy = "$Root\deploy"
$secrets = "$deploy\secrets\deploy.json"
Write-Host "== ${Node}: copying files to $deploy"
# The deploy directory (and the secrets directory in it) is SYSTEM and Administrators only.
Invoke-Node @"
`$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force '$deploy\hooks', '$deploy\secrets' | Out-Null
icacls '$deploy' /inheritance:r /grant:r '*S-1-5-18:(OI)(CI)F' '*S-1-5-32-544:(OI)(CI)F' /Q | Out-Null
if (`$LASTEXITCODE) { throw 'icacls failed' }
"@ 'creating the deploy directory'
$remote = '/' + ($deploy -replace '\\', '/')
scp -q "$src/setup.ps1" "$src/../egress_proxy.py" "${Ssh}:$remote/"
if ($LASTEXITCODE -ne 0) { throw 'copying setup.ps1 and egress_proxy.py failed' }
scp -q "$src/hooks/job-started.ps1" "$src/hooks/job-completed.ps1" "${Ssh}:$remote/hooks/"
if ($LASTEXITCODE -ne 0) { throw 'copying the hooks failed' }

$s = @{ token = ''; cache_endpoint = ''; cache_password = ''; egress_exit = [string]$EgressExit }
if ($CacheHost) {
    $line = (ssh $CacheHost "sudo grep -m1 '^CACHE_PASSWORD=' $CacheDir/.env") -join ''
    if ($LASTEXITCODE -ne 0 -or -not $line) { throw "could not read CACHE_PASSWORD on $CacheHost" }
    $s.cache_password = $line -replace '^CACHE_PASSWORD=', ''
    $line = $null
    $ip = ((ssh -G $CacheHost) | Where-Object { $_ -like 'hostname *' }) -replace '^hostname ', ''
    $s.cache_endpoint = "redis://${ip}:6379"
}
$s.token = gh api -X POST "repos/$Repo/actions/runners/registration-token" --jq .token
if ($LASTEXITCODE -ne 0 -or -not $s.token) { throw 'could not mint a registration token (repo admin needed)' }

try {
    Invoke-Node "[IO.File]::WriteAllText('$secrets', [Console]::In.ReadToEnd())" 'handing over the secrets' ($s | ConvertTo-Json -Compress)
    $s = $null
    Write-Host "== ${Node}: running setup.ps1 (Rust $channel, $Runners runners)"
    Invoke-Node "& '$deploy\setup.ps1' -RustVersion '$channel' -Root '$Root' -Runners $Runners -Jobs $Jobs -Node '$Node' -Repo '$Repo' -Secrets '$secrets'" 'setup.ps1'
} finally {
    # setup.ps1 normally deleted it already.
    Invoke-Node "if (Test-Path '$secrets') { Remove-Item -Force '$secrets' }" 'removing the secrets file'
}

(gh api "repos/$Repo/actions/runners" | ConvertFrom-Json).runners | Where-Object name -like "$Node-*" |
    ForEach-Object { "{0}`t{1}`tbusy={2}`t{3}" -f $_.name, $_.status, $_.busy, (($_.labels | ForEach-Object name) -join ',') }
