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
    scp -q "$src/Dockerfile" "$src/compose.yml" "$src/entrypoint.sh" "${ssh}:$dir/"
    scp -q "$src/hooks/job-started.sh" "$src/hooks/job-completed.sh" "${ssh}:$dir/hooks/"
    if ($LASTEXITCODE -ne 0) { throw "${node}: scp failed" }
    $envFile = "NODE=$node`nREPO_URL=https://github.com/$Repo`nRUNNER_MEM_LIMIT=$($t.Mem)`nSCCACHE_CACHE_SIZE=20G`n"
    $envFile | ssh $ssh "cat > $dir/.env && sed -i 's/\r$//' $dir/entrypoint.sh $dir/hooks/*.sh"

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
