function Build-CimmeriaServer {
    <#
    .SYNOPSIS
        Builds the full Cimmeria workspace via cargo build.

    .DESCRIPTION
        Runs cargo build for the entire workspace (server, admin API, launcher, Tauri app).
        In Release mode, uses --release. Verifies that the cimmeria-server binary is produced.

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

    Write-Step "BUILDING CIMMERIA WORKSPACE"

    # Install frontend dependencies if Node.js is available (required for cimmeria-app Tauri build)
    $frontendDir = Join-Path $paths.ProjectRoot "frontend"
    $nodeAvailable = $null -ne (Get-Command node -ErrorAction SilentlyContinue)
    if ($nodeAvailable -and (Test-Path (Join-Path $frontendDir "package.json"))) {
        Write-Status "Installing frontend dependencies (npm install)..." "White"
        Push-Location $frontendDir
        try {
            & npm install 2>&1 | ForEach-Object {
                $line = "$_"
                if ($line -match 'added|up to date|warn|error') { Write-Status "  $line" "DarkGray" }
            }
            if ($LASTEXITCODE -ne 0) {
                Write-Status "npm install failed — cimmeria-app will be excluded from build" "Yellow"
                $nodeAvailable = $false
            } else {
                Write-Status "Frontend dependencies installed." "Green"
            }
        } finally {
            Pop-Location
        }
    }

    $cargoArgs = @("build", "--workspace")
    if ($Configuration -eq "Release") {
        $cargoArgs += "--release"
    }

    Write-Status "Running: cargo $($cargoArgs -join ' ')" "White"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    # Show ALL cargo output so build errors are visible (not just filtered lines)
    & cargo @cargoArgs 2>&1 | ForEach-Object {
        $line = "$_"
        if ($line -match 'error') {
            Write-Status "  $line" "Red"
        } elseif ($line -match 'warning:.*generated|Finished') {
            Write-Status "  $line" "DarkGray"
        } elseif ($line -match 'warning') {
            Write-Status "  $line" "Yellow"
        } elseif ($line -match 'Compiling') {
            Write-Status "  $line" "DarkGray"
        } else {
            # Show everything else too — build script output, notes, etc.
            Write-Status "  $line" "Gray"
        }
    }
    $sw.Stop()

    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE."
    }

    # Verify binary — check both default and target-specific output directories
    $profile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $exeSuffix = if ($IsWindows -or (-not (Test-Path variable:IsWindows))) { ".exe" } else { "" }
    $serverBin = Join-Path $paths.ProjectRoot "target/$profile/cimmeria-server$exeSuffix"
    if (-not (Test-Path $serverBin)) {
        # Cross-compile via --target puts output under target/<triple>/<profile>/
        $serverBin = Join-Path $paths.ProjectRoot "target/x86_64-pc-windows-gnu/$profile/cimmeria-server.exe"
    }

    if (Test-Path $serverBin) {
        $size = Format-FileSize (Get-Item $serverBin).Length
        Write-Status "Built: cimmeria-server ($size)" "Green"
    } else {
        throw "cimmeria-server binary not found at: $serverBin"
    }

    Write-Status "Build complete ($Configuration, $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)." "Green"
}
