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
        release is published at the URL both files are a 404: that is reported
        and the function returns without failing. Any other download error, a zip
        that does not match its .sha256, or a zip entry that would land outside
        launcher\ fails. When only one of the two files is there, or they do not
        match, launcher-current may be mid-refresh: retry in a minute.

        The .sha256 check proves the zip arrived intact, not who made it: the
        .sha256 comes from the same release. Trust rests on TLS and on who can
        publish GitHub releases for the repository. The launcher's own
        self-update verifies its Minisign signature; this download does not.

        The launcher keeps its settings and state in the user's app data, not in
        launcher\, so replacing the folder loses nothing. The old folder is kept
        as launcher.old until the new one is in place, and put back if the swap
        fails. Close the launcher first.

    .PARAMETER BaseUrl
        Release download base. Default: launcher-current on SandboxServers/Cimmeria.
        Must be https; plain http is allowed only to a loopback host (tests).

    .EXAMPLE
        Install-CimmeriaLauncher
    #>
    [CmdletBinding()]
    param(
        [string]$BaseUrl = "https://github.com/SandboxServers/Cimmeria/releases/download/launcher-current"
    )

    $zipName = "StargateWorlds-Launcher-windows-x64.zip"
    $exeName = "cimmeria-launcher-desktop.exe"
    $retry = "launcher-current may be mid-refresh; retry in a minute."
    $paths = Get-ProjectPaths

    Write-Step "SGW LAUNCHER (RELEASED ZIP)"

    $uri = $null
    if (-not [Uri]::TryCreate($BaseUrl, [UriKind]::Absolute, [ref]$uri)) {
        throw "-BaseUrl '$BaseUrl' is not an absolute URL."
    }
    $loopbackHttp = $uri.Scheme -eq 'http' -and $uri.Host -in @('127.0.0.1', '[::1]', '::1', 'localhost')
    if ($uri.Scheme -ne 'https' -and -not $loopbackHttp) {
        throw "-BaseUrl must be https (plain http only to 127.0.0.1, ::1 or localhost): '$BaseUrl'"
    }

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
    $old = "$dest.old"
    $staging = Join-Path $downloadDir "launcher-staging"

    # The small .sha256 first: a missing release shows up before the zip
    # download starts.
    if (-not (Get-LauncherAsset "$BaseUrl/$zipName.sha256" $shaPath)) {
        if (Test-LauncherAsset "$BaseUrl/$zipName") {
            throw "$BaseUrl has $zipName but its .sha256 is a 404; $retry"
        }
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
        throw "$BaseUrl has $zipName.sha256 but $zipName is a 404; $retry"
    }

    $actual = (Get-FileHash -LiteralPath $zipPath -Algorithm SHA256).Hash.ToLowerInvariant()
    if ($actual -ne $expected) {
        Remove-Item -LiteralPath $zipPath -Force -ErrorAction SilentlyContinue
        throw "$zipName is $actual, but its .sha256 says $expected. Not installed; $retry"
    }
    Write-Status "Checked: $zipName matches its .sha256 ($expected)" "Green"

    if (Test-Path $staging) { Remove-Item $staging -Recurse -Force }
    New-Item -ItemType Directory -Path $staging -Force | Out-Null
    # Before anything is written: every entry must resolve inside the staging
    # folder. Expand-Archive on Windows PowerShell 5.1 writes a `..\` entry
    # outside it, and on 7 it only reports a non-terminating error.
    Assert-ZipEntriesInside $zipPath $staging
    Expand-Archive -LiteralPath $zipPath -DestinationPath $staging -Force -ErrorAction Stop
    if (-not (Test-Path (Join-Path $staging $exeName))) {
        Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
        throw "$zipName has no $exeName at its root. Not installed."
    }

    # Swap with a way back: the old folder becomes launcher.old, the new one
    # moves in, and launcher.old goes only once that worked.
    if (Test-Path $old) { Remove-Item $old -Recurse -Force }
    if (Test-Path $dest) {
        try {
            Rename-Item -LiteralPath $dest -NewName (Split-Path -Leaf $old) -ErrorAction Stop
        } catch {
            Remove-Item $staging -Recurse -Force -ErrorAction SilentlyContinue
            throw "Could not replace $dest (is the launcher running? Close it and re-run): $_"
        }
    }
    try {
        Move-Item -LiteralPath $staging -Destination $dest -ErrorAction Stop
    } catch {
        if ((Test-Path $old) -and -not (Test-Path $dest)) {
            Rename-Item -LiteralPath $old -NewName (Split-Path -Leaf $dest)
        }
        throw "Could not move the new launcher into $dest; the previous one was kept: $_"
    }
    if (Test-Path $old) {
        Remove-Item $old -Recurse -Force -ErrorAction SilentlyContinue
        if (Test-Path $old) { Write-Status "  Could not delete $old; remove it by hand." "Yellow" }
    }

    $size = Format-FileSize (Get-Item (Join-Path $dest $exeName)).Length
    Write-Status "OK: $exeName ($size) -> $dest" "Green"
    Write-Status "  Keep the windows and graphics folders beside it." "DarkGray"
}
