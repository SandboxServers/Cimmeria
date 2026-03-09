function Update-CimmeriaClient {
    <#
    .SYNOPSIS
        Patches the Stargate Worlds game client to connect to the local Cimmeria server.

    .DESCRIPTION
        Finds the Stargate Worlds client installation and patches Login.lua to point
        at the local AuthenticationServer (http://localhost:8081/SGWLogin/UserAuth).

        The original file is backed up as Login.lua.bak before patching.

    .PARAMETER ClientPath
        Path to the SGW client install directory. If not specified, searches common
        install locations automatically.

    .PARAMETER ServerUrl
        Server URL to patch into Login.lua. Default: http://localhost:8081

    .EXAMPLE
        Update-CimmeriaClient

    .EXAMPLE
        Update-CimmeriaClient -ClientPath "D:\Games\Stargate Worlds-QA"
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [string]$ClientPath,
        [string]$ServerUrl = "http://localhost:8081"
    )

    Write-Step "PATCHING GAME CLIENT"

    # Find client installation
    if (-not $ClientPath) {
        $searchPaths = @(
            "${env:ProgramFiles(x86)}\FireSky\Stargate Worlds-QA",
            "${env:ProgramFiles}\FireSky\Stargate Worlds-QA",
            "${env:ProgramFiles(x86)}\Cheyenne Mountain Entertainment\Stargate Worlds",
            "${env:ProgramFiles}\Cheyenne Mountain Entertainment\Stargate Worlds"
        )

        foreach ($search in $searchPaths) {
            if (Test-Path $search) {
                $ClientPath = $search
                Write-Status "Found client: $ClientPath" "Green"
                break
            }
        }

        if (-not $ClientPath) {
            Write-Host ""
            Write-Host "Stargate Worlds client not found in common locations." -ForegroundColor Yellow
            Write-Host ""
            Write-Host "Please specify the path manually:" -ForegroundColor White
            Write-Host "  Update-CimmeriaClient -ClientPath 'C:\Path\To\Stargate Worlds-QA'" -ForegroundColor Gray
            Write-Host ""
            Write-Host "The client directory should contain:" -ForegroundColor Gray
            Write-Host "  Working\SGWGame\Content\UI\Startup\Login\Login.lua" -ForegroundColor Gray
            return
        }
    }

    $loginLua = Join-Path $ClientPath "Working\SGWGame\Content\UI\Startup\Login\Login.lua"

    if (-not (Test-Path $loginLua)) {
        throw "Login.lua not found at: $loginLua"
    }

    # Read current content preserving original encoding (game expects ASCII, not UTF-8 BOM)
    $contentBytes = [System.IO.File]::ReadAllBytes($loginLua)
    $content = [System.Text.Encoding]::ASCII.GetString($contentBytes)
    $authUrl = "$ServerUrl/SGWLogin/UserAuth"

    # Check if already patched
    if ($content -match [regex]::Escape($authUrl)) {
        Write-Status "Login.lua already patched for $ServerUrl" "DarkGray"
        return
    }

    # Backup original
    $backupPath = "$loginLua.bak"
    if (-not (Test-Path $backupPath)) {
        if ($PSCmdlet.ShouldProcess($loginLua, "Create backup as Login.lua.bak")) {
            Copy-Item -Path $loginLua -Destination $backupPath -Force
            Write-Status "Backup created: $backupPath" "DarkGray"
        }
    }

    # Patch the auth URL
    if ($PSCmdlet.ShouldProcess($loginLua, "Patch auth URL to $authUrl")) {
        # Match patterns like: url = "https://something/SGWLogin/UserAuth" or similar
        $patched = $content -replace '(https?://[^"'']*)/SGWLogin/UserAuth', "$ServerUrl/SGWLogin/UserAuth"

        if ($patched -eq $content) {
            # If regex didn't match, try a broader approach
            Write-Status "Standard URL pattern not found, searching for server URL references..." "Yellow"
            # Look for any URL that looks like a server endpoint
            $patched = $content -replace 'https?://[^"'']*SGWLogin', "$ServerUrl/SGWLogin"
        }

        if ($patched -eq $content) {
            Write-Status "WARNING: Could not find auth URL pattern in Login.lua" "Yellow"
            Write-Status "You may need to edit Login.lua manually:" "Yellow"
            Write-Status "  File: $loginLua" "Yellow"
            Write-Status "  Set the auth URL to: $authUrl" "Yellow"
            return
        }

        # Write back as ASCII to preserve original encoding (game Lua parser crashes on UTF-8 BOM)
        [System.IO.File]::WriteAllBytes($loginLua, [System.Text.Encoding]::ASCII.GetBytes($patched))
        Write-Status "Login.lua patched: auth -> $authUrl" "Green"
    }

    Write-Host ""
    Write-Host "Client patched. To connect:" -ForegroundColor Green
    Write-Host "  1. Start the server:  pwsh setup.ps1" -ForegroundColor White
    Write-Host "  2. Launch the game client" -ForegroundColor White
    Write-Host "  3. Login with: test / test" -ForegroundColor White
}
