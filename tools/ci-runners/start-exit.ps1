#!/usr/bin/env pwsh
# Starts the exit proxy (egress_proxy.py, exit role) in Docker on a machine whose
# internet path reaches the Azure Storage ranges the colo cannot (57.0.0.0/8 and
# 52.239.0.0/16), published only on that machine's VPN address. The runners'
# `egress` service sends CONNECTs for those ranges here (README, "Colo egress").
# Docker restarts it with Docker Desktop.
#
#   pwsh tools/ci-runners/start-exit.ps1 -Listen <vpn-address>:3128
#
# The container only connects to those ranges. Docker Desktop hides the client's
# address from the container, so restrict who may connect with a Windows firewall
# rule on the VPN adapter (-RemoteAddress = the colo runner subnet), as the README
# shows.
[CmdletBinding()]
param(
    [Parameter(Mandatory)][string]$Listen,
    # An empty list would give `--allow-dst` no values, which egress_proxy.py reads
    # as "any destination": an open proxy. Require at least one range.
    [ValidateCount(1, 64)][ValidateNotNullOrEmpty()][string[]]$AllowDst = @('57.0.0.0/8', '52.239.0.0/16'),
    [string]$Name = 'cimmeria-egress-exit'
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

docker build -q -t cimmeria-egress-proxy:latest -f "$PSScriptRoot/egress.Dockerfile" $PSScriptRoot
if ($LASTEXITCODE -ne 0) { throw 'docker build failed' }
docker rm -f $Name 2>$null | Out-Null
docker run -d --name $Name --restart unless-stopped -p "${Listen}:3128" `
    cimmeria-egress-proxy:latest --listen 0.0.0.0:3128 --allow-dst @AllowDst
if ($LASTEXITCODE -ne 0) { throw 'docker run failed' }
Start-Sleep -Seconds 2
docker logs $Name 2>&1 | Select-Object -Last 1
