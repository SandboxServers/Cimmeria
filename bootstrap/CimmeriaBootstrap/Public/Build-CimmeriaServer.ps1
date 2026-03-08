function Build-CimmeriaServer {
    <#
    .SYNOPSIS
        Builds the Cimmeria server via cargo build.

    .DESCRIPTION
        Runs cargo build for the workspace. In Release mode, uses --release.
        Verifies that the cimmeria-server binary is produced.

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

    Write-Step "BUILDING CIMMERIA SERVER"

    $cargoArgs = @("build", "--workspace", "--exclude", "cimmeria-app")
    if ($Configuration -eq "Release") {
        $cargoArgs += "--release"
    }

    Write-Status "Running: cargo $($cargoArgs -join ' ')" "White"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    & cargo @cargoArgs 2>&1 | ForEach-Object {
        $line = "$_"
        if ($line -match 'Compiling|Finished|error|warning') {
            Write-Status "  $line" "DarkGray"
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

    Write-Status "Build-CimmeriaServer complete ($Configuration, $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)." "Green"
}
