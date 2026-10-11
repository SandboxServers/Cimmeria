#!/usr/bin/env pwsh
# Builds and (re)starts the five runner/dind pairs on one or more nodes over ssh.
#
#   pwsh tools/ci-runners/deploy.ps1                       # node5 and node2
#   pwsh tools/ci-runners/deploy.ps1 -Nodes node2          # one node
#   pwsh tools/ci-runners/deploy.ps1 -Nodes node5 -Down    # stop and deregister nothing
#
# A runner registers itself on first start with a one-hour registration token this
# script mints through `gh` (repo admin needed). The token is passed in the ssh
# command's environment only; it is never written on the node. After that the
# registration lives in the runner's `state` volume, so restarts and image rebuilds
# need no token.
[CmdletBinding()]
param(
    [string[]]$Nodes = @('node5', 'node2'),
    [string]$Repo = 'SandboxServers/Cimmeria',
    # host:port of the exit proxy for 57.0.0.0/8 (README, "Colo egress"). Empty
    # keeps the value already in the node's .env.
    [string]$EgressExit = $env:CIMMERIA_EGRESS_EXIT,
    # The node that runs the shared compile cache (cache.compose.yml); '' turns it off.
    [string]$CacheNode = 'node2',
    [switch]$Down
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ssh host alias and where the compose project lives (both on the NVMe volume).
$targets = @{
    node5 = @{ Ssh = 'cimmeria-colo'; Dir = '/mnt/nvme/gha-runners'; Mem = '16g' }
    node2 = @{ Ssh = 'cimmeria-node2'; Dir = '/var/lib/gha-runners'; Mem = '20g' }
}

$src = $PSScriptRoot
# The shared compile cache first, so every runner deployed below can use it. Its
# password is generated once and kept in the cache node's .env.
$cacheEndpoint = ''; $cachePassword = ''
if ($CacheNode -and -not $Down) {
    $c = $targets[$CacheNode]
    if (-not $c) { throw "unknown cache node '$CacheNode'" }
    $cdir = $c.Dir -replace 'gha-runners$', 'gha-cache'
    ssh $c.Ssh "sudo install -d -o `$(id -u) -g `$(id -g) $cdir"
    if ($LASTEXITCODE -ne 0) { throw "${CacheNode}: could not create $cdir" }
    $cachePassword = (ssh $c.Ssh "sed -n 's/^CACHE_PASSWORD=//p' $cdir/.env 2>/dev/null") -join ''
    if (-not $cachePassword) {
        $bytes = [byte[]]::new(24); [Security.Cryptography.RandomNumberGenerator]::Fill($bytes)
        $cachePassword = [Convert]::ToHexString($bytes).ToLower()
    }
    scp -q "$src/cache.compose.yml" "$($c.Ssh):$cdir/compose.yml"
    "CACHE_PASSWORD=$cachePassword`n" | ssh $c.Ssh "umask 077 && cat > $cdir/.env"
    Write-Host "== ${CacheNode}: starting the shared compile cache"
    ssh $c.Ssh "cd $cdir && docker compose up -d"
    if ($LASTEXITCODE -ne 0) { throw "${CacheNode}: cache compose up failed" }
    # The address the other nodes reach it on: the cache node's ssh HostName.
    $cacheHost = ((ssh -G $c.Ssh) | Where-Object { $_ -like 'hostname *' }) -replace '^hostname ', ''
    $cacheEndpoint = "redis://${cacheHost}:6379"
}
foreach ($node in $Nodes) {
    $t = $targets[$node]
    if (-not $t) { throw "unknown node '$node' (known: $($targets.Keys -join ', '))" }
    $ssh = $t.Ssh; $dir = $t.Dir

    if ($Down) {
        ssh $ssh "cd $dir && docker compose down"
        if ($LASTEXITCODE -ne 0) { throw "${node}: compose down failed" }
        continue
    }

    Write-Host "== ${node}: copying files to $dir"
    ssh $ssh "sudo install -d -o `$(id -u) -g `$(id -g) $dir $dir/hooks"
    if ($LASTEXITCODE -ne 0) { throw "${node}: could not create $dir" }
    scp -q "$src/Dockerfile" "$src/compose.yml" "$src/entrypoint.sh" "$src/egress.Dockerfile" "$src/egress_proxy.py" "${ssh}:$dir/"
    scp -q "$src/hooks/job-started.sh" "$src/hooks/job-completed.sh" "${ssh}:$dir/hooks/"
    ssh $ssh "chmod 0755 $dir/hooks/*.sh"
    if ($LASTEXITCODE -ne 0) { throw "${node}: scp failed" }
    $exit = $EgressExit
    if (-not $exit) { $exit = (ssh $ssh "sed -n 's/^EGRESS_EXIT=//p' $dir/.env 2>/dev/null") -join '' }
    $envFile = "NODE=$node`nREPO_URL=https://github.com/$Repo`nRUNNER_MEM_LIMIT=$($t.Mem)`nSCCACHE_CACHE_SIZE=20G`nEGRESS_EXIT=$exit`nCACHE_ENDPOINT=$cacheEndpoint`nCACHE_PASSWORD=$cachePassword`n"
    $envFile | ssh $ssh "umask 077 && cat > $dir/.env && sed -i 's/\r$//' $dir/entrypoint.sh $dir/hooks/*.sh"

    $token = gh api -X POST "repos/$Repo/actions/runners/registration-token" --jq .token
    if ($LASTEXITCODE -ne 0 -or -not $token) { throw 'could not mint a registration token (repo admin needed)' }

    Write-Host "== ${node}: building the image and starting the runners"
    ssh $ssh "cd $dir && docker compose build --pull && RUNNER_TOKEN='$token' docker compose up -d --remove-orphans"
    if ($LASTEXITCODE -ne 0) { throw "${node}: compose up failed" }
}

if (-not $Down) {
    Start-Sleep -Seconds 20
    gh api "repos/$Repo/actions/runners" --jq '.runners[] | "\(.name)\t\(.status)\tbusy=\(.busy)\t\([.labels[].name] | join(","))"'
}
