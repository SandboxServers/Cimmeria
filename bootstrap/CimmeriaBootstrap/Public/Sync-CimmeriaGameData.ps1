function Sync-CimmeriaGameData {
    <#
    .SYNOPSIS
        Copies cooked game data (PAK files) from the SGW client to the server's data/cache directory.

    .DESCRIPTION
        The Rust server serves cached game data (abilities, items, character creation, etc.)
        to the client via Mercury resource fragments. These PAK files must come from the same
        build as the client -- if versions or element structures differ, the client will
        receive malformed data (e.g. character creation breaks).

        This function finds the SGW client installation and copies its Cache.en-US/*.pak
        files into the server's data/cache/ directory. Idempotent: skips files that already
        exist with matching sizes.

    .PARAMETER ClientPath
        Path to the SGW client root directory. If not specified, searches common locations.

    .EXAMPLE
        Sync-CimmeriaGameData

    .EXAMPLE
        Sync-CimmeriaGameData -ClientPath "F:\Stargate Worlds-QA"
    #>
    [CmdletBinding()]
    param(
        [string]$ClientPath
    )

    Write-Step "SYNCING GAME DATA"

    # Find client installation (reuses same search logic as Update-CimmeriaClient)
    if (-not $ClientPath) {
        $scriptDrive = (Split-Path $PSScriptRoot -Qualifier)
        $searchPaths = @(
            "${scriptDrive}\Stargate Worlds-QA",
            "${env:ProgramFiles(x86)}\FireSky\Stargate Worlds-QA",
            "${env:ProgramFiles}\FireSky\Stargate Worlds-QA",
            "${env:ProgramFiles(x86)}\Cheyenne Mountain Entertainment\Stargate Worlds",
            "${env:ProgramFiles}\Cheyenne Mountain Entertainment\Stargate Worlds"
        )

        foreach ($search in $searchPaths) {
            $candidate = Join-Path $search "Working\SGWGame\Cache.en-US"
            if (Test-Path $candidate) {
                $ClientPath = $search
                break
            }
        }

        if (-not $ClientPath) {
            Write-Status "SGW client not found -- skipping game data sync." "Yellow"
            Write-Host "  PAK files must be placed manually in data\cache\." -ForegroundColor Gray
            return
        }
    }

    $sourceDir = Join-Path $ClientPath "Working\SGWGame\Cache.en-US"
    if (-not (Test-Path $sourceDir)) {
        Write-Status "Client cache directory not found: $sourceDir" "Yellow"
        return
    }

    # Resolve server data/cache directory (relative to the repo root, two levels up from bootstrap module)
    $repoRoot = Split-Path (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent) -Parent
    $destDir = Join-Path $repoRoot "data\cache"
    New-Item -ItemType Directory -Path $destDir -Force | Out-Null

    $sourcePaks = Get-ChildItem -Path $sourceDir -Filter "*.pak"
    if ($sourcePaks.Count -eq 0) {
        Write-Status "No PAK files found in: $sourceDir" "Yellow"
        return
    }

    $copied = 0
    $skipped = 0
    foreach ($pak in $sourcePaks) {
        $dest = Join-Path $destDir $pak.Name
        if ((Test-Path $dest) -and (Get-Item $dest).Length -eq $pak.Length) {
            $skipped++
            continue
        }
        Copy-Item -Path $pak.FullName -Destination $dest -Force
        $copied++
    }

    if ($copied -gt 0) {
        Write-Status "Copied $copied PAK files from client ($($sourcePaks.Count) total, $skipped unchanged)." "Green"
    } else {
        Write-Status "Game data already up to date ($skipped PAK files)." "DarkGray"
    }
}
