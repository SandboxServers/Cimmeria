#!/usr/bin/env pwsh
# Builds and (re)starts a node's runner stack over ssh: five runner/dind pairs, the
# compile cache, buildkitd and the egress proxy (compose.yml).
#
#   pwsh tools/ci-runners/deploy.ps1                       # every node (cache host first)
#   pwsh tools/ci-runners/deploy.ps1 -Nodes node2          # one node
#   pwsh -c "& tools/ci-runners/deploy.ps1 -Nodes node5,node1"   # several (a bare
#                                                          # `pwsh <script> -Nodes a,b` passes one string)
#   pwsh tools/ci-runners/deploy.ps1 -Nodes node2 -Build   # after changing an image (recreates its runners)
#   pwsh tools/ci-runners/deploy.ps1 -Nodes node5 -Down    # stop the stack
#
# A runner registers itself on first start with a one-hour registration token this
# script mints through `gh` (repo admin needed). The token reaches the runners as the
# file `runner-token` in the compose directory, mounted read-only; it is emptied
# about a minute after the stack starts. It is never an environment variable,
# because a changed variable makes compose recreate every runner, killing running
# jobs. A registered runner keeps its registration in its `state` volume, so
# restarts and image rebuilds need no token, and an unchanged runner is not
# recreated by a deploy.
[CmdletBinding()]
param(
    [string[]]$Nodes = @('node5', 'node2', 'node1'),
    [string]$Repo = 'SandboxServers/Cimmeria',
    # host:port of the exit proxy for the unreachable Azure Storage ranges (README, "Colo egress"). Empty
    # keeps the value already in the node's .env.
    [string]$EgressExit = $env:CIMMERIA_EGRESS_EXIT,
    # The node whose compose `cache` serves every node's runners (a UniFi policy
    # admits TCP 6379 to it from the others). '' gives each node its own cache.
    [string]$CacheHost = 'node2',
    # Rebuild the images (and pull their bases). A rebuild gives a new image ID even
    # when every layer is cached, which recreates every runner, so it's opt-in;
    # without it, images are built only when missing.
    [switch]$Build,
    [switch]$Down
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ssh host alias and where the compose project lives (on the node's large volume).
# Jobs: CARGO_BUILD_JOBS for that node's runners (absent = the node's core count).
# node1 runs 8 as an experiment (2026-10-11): five jobs x 16 compiles on 16 cores
# spent ~19% of node1's CPU in the kernel; compare its job times with the others'.
$targets = @{
    node5 = @{ Ssh = 'cimmeria-colo'; Dir = '/mnt/nvme/gha-runners'; Mem = '16g'; Cache = '12gb'; BuildkitMb = 50000; Jobs = 8 }
    node1 = @{ Ssh = 'cimmeria-node1'; Dir = '/var/lib/gha-runners'; Mem = '16g'; Cache = '16gb'; BuildkitMb = 80000; Jobs = 8 }
    node2 = @{ Ssh = 'cimmeria-node2'; Dir = '/var/lib/gha-runners'; Mem = '20g'; Cache = '24gb'; BuildkitMb = 80000; Jobs = 8 }
}
foreach ($n in $Nodes) { if (-not $targets.ContainsKey($n)) { throw "unknown node '$n' (known: $($targets.Keys -join ', '))" } }
if ($CacheHost -and -not $targets.ContainsKey($CacheHost)) { throw "unknown cache host '$CacheHost'" }

function Invoke-Ssh([string]$Ssh, [string]$Command, [string]$What) {
    ssh $Ssh $Command
    if ($LASTEXITCODE -ne 0) { throw "${What} failed (ssh $Ssh, exit $LASTEXITCODE)" }
}
function Copy-ToNode([string]$Ssh, [string[]]$Files, [string]$Dest) {
    scp -q @Files "${Ssh}:$Dest"
    if ($LASTEXITCODE -ne 0) { throw "copying $($Files -join ', ') to ${Ssh}:$Dest failed" }
}
function Get-NodeEnv([hashtable]$T, [string]$Key) {
    (ssh $T.Ssh "sed -n 's/^${Key}=//p' $($T.Dir)/.env 2>/dev/null") -join ''
}

if ($Down) {
    foreach ($node in $Nodes) {
        $t = $targets[$node]
        Invoke-Ssh $t.Ssh "cd $($t.Dir) && docker compose down" "${node}: compose down"
    }
    return
}

# The compile cache's password, settled once for the whole run: every node's runners
# authenticate to the cache host's Redis with it. Kept in the cache host's .env (or,
# with per-node caches, generated for nodes that have none).
function New-Secret { $b = [byte[]]::new(24); [Security.Cryptography.RandomNumberGenerator]::Fill($b); [Convert]::ToHexString($b).ToLower() }
$sharedPassword = ''
$cacheEndpoint = ''
if ($CacheHost) {
    $sharedPassword = Get-NodeEnv $targets[$CacheHost] 'CACHE_PASSWORD'
    if (-not $sharedPassword) { $sharedPassword = New-Secret }
    $cacheIp = ((ssh -G $targets[$CacheHost].Ssh) | Where-Object { $_ -like 'hostname *' }) -replace '^hostname ', ''
    $cacheEndpoint = "redis://${cacheIp}:6379"
    # The cache host first, so its Redis is up before the other nodes point at it.
    $Nodes = @($Nodes | Where-Object { $_ -eq $CacheHost }) + @($Nodes | Where-Object { $_ -ne $CacheHost })
}

$src = $PSScriptRoot
foreach ($node in $Nodes) {
    $t = $targets[$node]
    $ssh = $t.Ssh; $dir = $t.Dir

    Write-Host "== ${node}: copying files to $dir"
    Invoke-Ssh $ssh "sudo install -d -o `$(id -u) -g `$(id -g) $dir $dir/hooks" "${node}: creating $dir"
    Copy-ToNode $ssh @("$src/Dockerfile", "$src/compose.yml", "$src/entrypoint.sh", "$src/egress.Dockerfile", "$src/egress_proxy.py") "$dir/"
    Copy-ToNode $ssh @("$src/hooks/job-started.sh", "$src/hooks/job-completed.sh") "$dir/hooks/"

    $exit = $EgressExit
    if (-not $exit) { $exit = Get-NodeEnv $t 'EGRESS_EXIT' }
    $cachePassword = $sharedPassword
    if (-not $cachePassword) { $cachePassword = Get-NodeEnv $t 'CACHE_PASSWORD' }
    if (-not $cachePassword) { $cachePassword = New-Secret }
    $endpoint = ''; $publish = ''
    if ($CacheHost -and $node -eq $CacheHost) { $publish = '0.0.0.0:6379' }
    elseif ($CacheHost) { $endpoint = $cacheEndpoint }

    $jobs = if ($t.ContainsKey('Jobs')) { $t.Jobs } else { ((ssh $ssh nproc) -join '').Trim() }
    if ($jobs -notmatch '^\d+$') { throw "${node}: could not work out CARGO_BUILD_JOBS (got '$jobs')" }
    $envFile = "NODE=$node`nREPO_URL=https://github.com/$Repo`nRUNNER_MEM_LIMIT=$($t.Mem)`nSCCACHE_CACHE_SIZE=20G`nEGRESS_EXIT=$exit`nCACHE_PASSWORD=$cachePassword`nCACHE_MAXMEMORY=$($t.Cache)`nCACHE_ENDPOINT=$endpoint`nCACHE_PUBLISH=$publish`nBUILDKIT_MAX_MB=$($t.BuildkitMb)`nCARGO_BUILD_JOBS=$jobs`n"
    $envFile | ssh $ssh "umask 077 && cat > $dir/.env && sed -i 's/\r$//' $dir/entrypoint.sh $dir/hooks/*.sh && chmod 0755 $dir/hooks/*.sh"
    if ($LASTEXITCODE -ne 0) { throw "${node}: writing .env failed" }

    $token = gh api -X POST "repos/$Repo/actions/runners/registration-token" --jq .token
    if ($LASTEXITCODE -ne 0 -or -not $token) { throw 'could not mint a registration token (repo admin needed)' }
    # The file always exists (an absent bind-mount source becomes a directory).
    # tr: PowerShell pipes a CRLF line ending, and a CR left in the token makes GitHub reject it.
    $token.Trim() | ssh $ssh "umask 077 && tr -d '\r\n' > $dir/runner-token"
    if ($LASTEXITCODE -ne 0) { throw "${node}: writing the registration token failed" }

    Write-Host "== ${node}: building the images and starting the stack"
    $buildCmd = if ($Build) { 'docker compose build --pull' } else { '{ docker image inspect cimmeria-gha-runner:latest cimmeria-egress-proxy:latest > /dev/null 2>&1 || docker compose build; }' }
    Invoke-Ssh $ssh "cd $dir && $buildCmd && docker compose up -d --remove-orphans" "${node}: compose up"
    # Long enough for a new runner to register; the token is useless once emptied.
    Start-Sleep -Seconds 60
    Invoke-Ssh $ssh ": > $dir/runner-token" "${node}: emptying the registration token"
}

gh api "repos/$Repo/actions/runners" --jq '.runners[] | "\(.name)\t\(.status)\tbusy=\(.busy)\t\([.labels[].name] | join(","))"'
