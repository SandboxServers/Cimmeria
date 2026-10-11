function Install-CimmeriaLauncher {
    <#
    .SYNOPSIS
        Downloads the released desktop launcher and unpacks it beside the server.

    .DESCRIPTION
        Fetches StargateWorlds-Launcher-windows-x64.zip and the .sha256 published
        beside it from the rolling launcher-current release, checks the zip against
        that digest, and unpacks it into launcher\ in the project root, next to
        cimmeria-server.exe: cimmeria-launcher-desktop.exe with its windows\ and
        graphics\ folders. The launcher is no longer built here (the egui launcher
        was retired in LX-18; the desktop launcher is its own workspace under
        crates/launcher/desktop and players run the released zip).

        Every run downloads afresh, so a re-run picks up a newer release. While no
        release is published at the URL the download is a 404: that is reported
        and the function returns without failing. Any other download error, or a
        zip that does not match its .sha256, fails.

        The launcher keeps its settings and state in the user's app data, not in
        launcher\, so replacing the folder loses nothing. Close the launcher first.

    .PARAMETER BaseUrl
        Release download base. Default: launcher-current on SandboxServers/Cimmeria.

    .EXAMPLE
        Install-CimmeriaLauncher
    #>
    [CmdletBinding()]
    param(
        [string]$BaseUrl = "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current"
    )

    $zipName = "StargateWorlds-Launcher-windows-x64.zip"
    $exeName = "cimmeria-launcher-desktop.exe"
    $paths = Get-ProjectPaths

    Write-Step "SGW LAUNCHER (RELEASED ZIP)"

    $onWindows = $IsWindows -or (-not (Test-Path variable:IsWindows))
    if (-not $onWindows) {
        Write-Status "SKIP: the launcher zip is for Windows. On a Mac, download" "Yellow"
        Write-Status "  $BaseUrl/StargateWorlds-Launcher-macos-arm64.zip" "Yellow"
        return
    }

    $downloadDir = $paths.DownloadDir
    New-Item -ItemType Directory -Path $downloadDir -Force | Out-Null
    $zipPath = Join-Path $downloadDir $zipName
    $shaPath = "$zipPath.sha256"
    $dest = Join-Path $paths.ProjectRoot "launcher"
    $staging = Join-Path $downloadDir "launcher-staging"

    # The small .sha256 first: a missing release shows up before the zip
    # download starts.
    if (-not (Get-LauncherAsset "$BaseUrl/$zipName.sha256" $shaPath)) {
        Write-Status "No launcher release is published at $BaseUrl yet ($zipName.sha256 is a 404)." "Yellow"
        Write-Status "  Skipping the launcher. Re-run with -WithLauncher once a release is published." "Yellow"
        return
    }

    # sha256sum format: "<64 hex>  <file name>".
    $line = (Get-Content -LiteralPath $shaPath -TotalCount 1)
    if ($line -notmatch '^\s*([0-9a-fA-F]{64})\s+\*?(\S+)\s*$') {
        throw "$zipName.sha256 from $BaseUrl is not a SHA-256 line: '$line'"
    }
    $expected = $Matches[1].ToLowerInvariant()
    if ($Matches[2] -ne $zipName) {
        throw "$zipName.sha256 from $BaseUrl names '$($Matches[2])', not $zipName"
    }

    if (-not (Get-LauncherAsset "$BaseUrl/$zipName" $zipPath)) {
        throw "$BaseUrl has $zipName.sha256 but $zipName is a 404; the release is incomplete."
    }

    $actual = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
        throw "$zipName is $actual, but its .sha256 says $expected. Not installed."
    }
    Write-Status "Checked: $zipName matches its .sha256 ($expected)" "Green"

    if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
    Expand-Archive -LiteralPath $zipPath -DestinationPath $staging -Force
    if (-not (Test-Path (Join-Path $staging $exeName))) {
        Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
        throw "$zipName has no $exeName at its root. Not installed."
    }

    if (Test-Path $dest) {
        try {
            Remove-Item $dest -Recurse -Force -ErrorAction Stop
        } catch {
            Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
            throw "Could not replace $dest (is the launcher running? Close it and re-run): $_"
        }
    }
    Move-Item -LiteralPath $staging -Destination $dest

    $size = Format-FileSize (Get-Item (Join-Path $dest $exeName)).Length
    Write-Status "OK: $exeName ($size) -> $dest" "Green"
    Write-Status "  Keep the windows and graphics folders beside it." "DarkGray"
}
