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
    if (-not (Test-Path (Join-Path $frontendDir "node_modules"))) {
        Write-Status "Installing frontend dependencies..." "White"
        Push-Location $frontendDir
        try {
            & npm install 2>&1 | ForEach-Object {
                $line = "$_"
                if ($line -match 'added|warn|error') { Write-Status "  $line" "DarkGray" }
            }
            if ($LASTEXITCODE -ne 0) {
                throw "npm install failed with exit code $LASTEXITCODE."
            }
            Write-Status "Frontend dependencies installed." "Green"
        } finally {
            Pop-Location
        }
    } else {
        Write-Status "Frontend dependencies already installed." "DarkGray"
    }

    # Step 2: Build Tauri app
    $tauriArgs = @("tauri", "build")
    if ($Configuration -eq "Debug") {
        $tauriArgs = @("tauri", "dev")
        Write-Status "Running: cargo $($tauriArgs -join ' ')" "White"
        Write-Status "NOTE: Debug mode launches the dev server. Use -Configuration Release for a packaged build." "Yellow"
    } else {
        Write-Status "Running: cargo $($tauriArgs -join ' ')" "White"
    }

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
