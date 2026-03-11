#Requires -Version 7.0
<#
.SYNOPSIS
    Patches the Stargate Worlds game client and launches it.

.DESCRIPTION
    Writes a LoginInternal.lua file into the SGW client that configures it to
    connect to the Cimmeria server, then launches the game. This uses the client's
    built-in Lua override mechanism -- no binary patching required.

.PARAMETER ClientPath
    Path to the SGW client install directory. Searches common locations if omitted.

.PARAMETER ServerAddress
    Full URL of the login server. Default: 127.0.0.1:8081

.PARAMETER ServerName
    Display name for the server in the login dropdown. Default: Cimmeria

.PARAMETER NoLaunch
    Patch only -- do not launch SGW.exe.

.PARAMETER Atrea
    Launch via AtreaLoader instead of SGW.exe directly. Enables Mercury protocol
    logging, network packet capture (PCAP), and AES session key dumping.
    Output files are written to {ClientBinaries}/sessions/.

.EXAMPLE
    pwsh patch-client.ps1

.EXAMPLE
    pwsh patch-client.ps1 -ClientPath "D:\Games\Stargate Worlds-QA"

.EXAMPLE
    pwsh patch-client.ps1 -NoLaunch

.EXAMPLE
    pwsh patch-client.ps1 -Atrea
#>
param(
    [string]$ClientPath,
    [string]$ServerAddress = "127.0.0.1:8081",
    [string]$ServerName = "Cimmeria",
    [switch]$NoLaunch,
    [switch]$Atrea
)

Import-Module (Join-Path $PSScriptRoot "bootstrap/CimmeriaBootstrap") -Force
Update-CimmeriaClient @PSBoundParameters
