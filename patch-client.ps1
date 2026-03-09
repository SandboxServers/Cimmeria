<#
.SYNOPSIS
    Patches the Stargate Worlds game client to connect to the local Cimmeria server.

.DESCRIPTION
    Finds the SGW client installation and patches Login.lua to point at the local
    AuthenticationServer (http://localhost:8081/SGWLogin/UserAuth).

    The original Login.lua is backed up as Login.lua.bak before patching.

    Common install locations are searched automatically. If the client is not found,
    pass the path manually with -ClientPath.

.PARAMETER ClientPath
    Path to the SGW client install directory. If not specified, searches common
    install locations automatically.

.PARAMETER ServerUrl
    Auth server URL to patch into Login.lua. Default: http://localhost:8081

.EXAMPLE
    pwsh patch-client.ps1

.EXAMPLE
    pwsh patch-client.ps1 -ClientPath "D:\Games\Stargate Worlds-QA"

.EXAMPLE
    pwsh patch-client.ps1 -ServerUrl "http://192.168.1.100:8081"
#>
#Requires -Version 7.0
param(
    [string]$ClientPath,
    [string]$ServerUrl = "http://localhost:8081"
)

Import-Module (Join-Path $PSScriptRoot "bootstrap/CimmeriaBootstrap") -Force
Update-CimmeriaClient @PSBoundParameters
