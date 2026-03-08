function Build-CimmeriaApp {
    <#
    .SYNOPSIS
        Builds the Cimmeria Tauri admin app (Rust backend + SolidJS frontend).

    .DESCRIPTION
        Runs npm install in the frontend/ directory, then cargo tauri build.
        Requires Node.js and npm to be installed.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Build-CimmeriaApp

    .EXAMPLE
        Build-CimmeriaApp -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug"
    )

    $paths = Get-ProjectPaths
    $frontendDir = Join-Path $paths.ProjectRoot "frontend"

    Write-Step "BUILDING CIMMERIA APP (TAURI)"

    # Step 1: Install frontend dependencies
    # Always run npm install to ensure deps match package.json (handles new/changed deps)
    Write-Status "Installing frontend dependencies..." "White"
    Push-Location $frontendDir
    try {
        & npm install 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'added|up to date|warn|error') { Write-Status "  $line" "DarkGray" }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "npm install failed with exit code $LASTEXITCODE."
        }
        Write-Status "Frontend dependencies installed." "Green"
    } finally {
        Pop-Location
    }

    # Step 2: Build Tauri app
    if ($Configuration -eq "Debug") {
        # Debug mode launches a dev server that runs indefinitely — start it in the
        # background so the bootstrap pipeline is not blocked.
        Write-Status "Running: cargo tauri dev (background)" "White"
        Write-Status "NOTE: Dev server launching in background. Use -Configuration Release for a packaged build." "Yellow"
        Start-Process -FilePath "cargo" -ArgumentList "tauri", "dev" -WindowStyle Normal
        Write-Status "Build-CimmeriaApp complete ($Configuration, dev server launched in background)." "Green"
    } else {
        $tauriArgs = @("tauri", "build")
        Write-Status "Running: cargo $($tauriArgs -join ' ')" "White"

        $sw = [System.Diagnostics.Stopwatch]::StartNew()
        & cargo @tauriArgs 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'Compiling|Finished|error|warning|Running') {
                Write-Status "  $line" "DarkGray"
            }
        }
        $sw.Stop()

        if ($LASTEXITCODE -ne 0) {
            throw "cargo tauri build failed with exit code $LASTEXITCODE."
        }

        Write-Status "Build-CimmeriaApp complete ($Configuration, $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)." "Green"
    }
}
