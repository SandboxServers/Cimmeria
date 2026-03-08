function Build-CimmeriaServer {
    <#
    .SYNOPSIS
        Builds all Cimmeria binaries: server, admin app, and launcher.

    .DESCRIPTION
        Runs three separate cargo builds so that a failure in one does not block the others:
          1. cimmeria-server  (game server + libraries)
          2. cimmeria-app     (Tauri admin panel — requires Node.js + frontend deps)
          3. sgw-launcher     (Tauri game launcher)

        The server build is required. App and launcher failures are reported as warnings.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Build-CimmeriaServer

    .EXAMPLE
        Build-CimmeriaServer -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug"
    )

    $paths = Get-ProjectPaths
    $profile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $exeSuffix = if ($IsWindows -or (-not (Test-Path variable:IsWindows))) { ".exe" } else { "" }
    $releaseFlag = if ($Configuration -eq "Release") { @("--release") } else { @() }

    Write-Step "BUILDING CIMMERIA WORKSPACE"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $failed = @()

    # ── 1. Server (required) ────────────────────────────────────────────────
    Write-Status "" "White"
    Write-Status "[1/3] Building cimmeria-server..." "Cyan"

    $serverArgs = @("build", "--workspace", "--exclude", "cimmeria-app", "--exclude", "sgw-launcher") + $releaseFlag
    Write-Status "  cargo $($serverArgs -join ' ')" "DarkGray"

    Invoke-CargoBuild $serverArgs

    if ($LASTEXITCODE -ne 0) {
        throw "cimmeria-server build failed with exit code $LASTEXITCODE."
    }

    # Verify server binary
    $serverBin = Join-Path $paths.ProjectRoot "target/$profile/cimmeria-server$exeSuffix"
    if (-not (Test-Path $serverBin)) {
        $serverBin = Join-Path $paths.ProjectRoot "target/x86_64-pc-windows-gnu/$profile/cimmeria-server.exe"
    }
    if (Test-Path $serverBin) {
        $size = Format-FileSize (Get-Item $serverBin).Length
        Write-Status "  OK: cimmeria-server ($size)" "Green"
    } else {
        throw "cimmeria-server binary not found at: $serverBin"
    }

    # ── 2. Admin App (optional — needs Node.js) ─────────────────────────────
    Write-Status "" "White"
    Write-Status "[2/3] Building cimmeria-app..." "Cyan"

    $frontendDir = Join-Path $paths.ProjectRoot "frontend"
    $nodeAvailable = $null -ne (Get-Command node -ErrorAction SilentlyContinue)

    if ($nodeAvailable -and (Test-Path (Join-Path $frontendDir "package.json"))) {
        Write-Status "  Installing frontend dependencies (npm install)..." "White"
        Push-Location $frontendDir
        try {
            & npm install 2>&1 | ForEach-Object {
                $line = "$_"
                if ($line -match 'added|up to date|warn|error') { Write-Status "    $line" "DarkGray" }
            }
            if ($LASTEXITCODE -ne 0) {
                Write-Status "  WARN: npm install failed" "Yellow"
                $nodeAvailable = $false
            }
        } finally {
            Pop-Location
        }
    }

    if (-not $nodeAvailable) {
        Write-Status "  SKIP: Node.js not available — cimmeria-app requires it" "Yellow"
        $failed += "cimmeria-app (no Node.js)"
    } else {
        $appArgs = @("build", "-p", "cimmeria-app") + $releaseFlag
        Write-Status "  cargo $($appArgs -join ' ')" "DarkGray"

        Invoke-CargoBuild $appArgs

        if ($LASTEXITCODE -ne 0) {
            Write-Status "  FAIL: cimmeria-app build failed (exit code $LASTEXITCODE)" "Red"
            $failed += "cimmeria-app"
        } else {
            $appBin = Join-Path $paths.ProjectRoot "target/$profile/cimmeria-app$exeSuffix"
            if (Test-Path $appBin) {
                $size = Format-FileSize (Get-Item $appBin).Length
                Write-Status "  OK: cimmeria-app ($size)" "Green"
            } else {
                Write-Status "  OK: cimmeria-app (built)" "Green"
            }
        }
    }

    # ── 3. Launcher (optional) ──────────────────────────────────────────────
    Write-Status "" "White"
    Write-Status "[3/3] Building sgw-launcher..." "Cyan"

    $launcherArgs = @("build", "-p", "sgw-launcher") + $releaseFlag
    Write-Status "  cargo $($launcherArgs -join ' ')" "DarkGray"

    Invoke-CargoBuild $launcherArgs

    if ($LASTEXITCODE -ne 0) {
        Write-Status "  FAIL: sgw-launcher build failed (exit code $LASTEXITCODE)" "Red"
        $failed += "sgw-launcher"
    } else {
        $launcherBin = Join-Path $paths.ProjectRoot "target/$profile/sgw-launcher$exeSuffix"
        if (Test-Path $launcherBin) {
            $dest = Join-Path $paths.ProjectRoot "sgw-launcher$exeSuffix"
            Copy-Item $launcherBin $dest -Force
            $size = Format-FileSize (Get-Item $dest).Length
            Write-Status "  OK: sgw-launcher ($size) -> copied to project root" "Green"
        } else {
            Write-Status "  OK: sgw-launcher (built)" "Green"
        }
    }

    # ── Summary ─────────────────────────────────────────────────────────────
    $sw.Stop()
    Write-Status "" "White"

    if ($failed.Count -gt 0) {
        Write-Status "Build finished with failures: $($failed -join ', ')" "Yellow"
        Write-Status "  The game server built successfully — failed apps are optional." "DarkGray"
    }

    Write-Status "Build-CimmeriaServer complete ($Configuration, $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)." "Green"
}

function Invoke-CargoBuild {
    <#
    .SYNOPSIS
        Runs cargo with the given arguments, showing all output with color-coded lines.
        Warnings are suppressed to keep bootstrap output clean.
    #>
    param([string[]]$CargoArgs)

    $prevRustflags = $env:RUSTFLAGS
    $env:RUSTFLAGS = "$prevRustflags -Awarnings"
    try {
    & cargo @CargoArgs 2>&1 | ForEach-Object {
        $line = "$_"
        if ($line -match 'error\[') {
            Write-Status "  $line" "Red"
        } elseif ($line -match '^error') {
            Write-Status "  $line" "Red"
        } elseif ($line -match 'warning:.*generated|Finished') {
            Write-Status "  $line" "DarkGray"
        } elseif ($line -match 'warning') {
            Write-Status "  $line" "Yellow"
        } elseif ($line -match 'Compiling') {
            Write-Status "  $line" "DarkGray"
        } else {
            Write-Status "  $line" "Gray"
        }
    }
    } finally {
        $env:RUSTFLAGS = $prevRustflags
    }
}
