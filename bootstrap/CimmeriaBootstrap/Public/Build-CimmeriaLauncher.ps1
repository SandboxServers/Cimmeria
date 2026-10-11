function Build-CimmeriaLauncher {
    <#
    .SYNOPSIS
        Builds the SGW game launcher (sgw-launcher, the egui launcher).

    .DESCRIPTION
        Builds the sgw-launcher crate with cargo and copies the executable to
        the project root.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Build-CimmeriaLauncher

    .EXAMPLE
        Build-CimmeriaLauncher -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug"
    )

    $paths = Get-ProjectPaths
    $profile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $exeSuffix = if ($IsWindows -or (-not (Test-Path variable:IsWindows))) { ".exe" } else { "" }

    Write-Step "BUILDING SGW LAUNCHER"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    $releaseFlag = if ($Configuration -eq "Release") { @("--release") } else { @() }
    $launcherArgs = @("build", "-p", "sgw-launcher") + $releaseFlag
    Write-Status "Running: cargo $($launcherArgs -join ' ')" "White"

    $prevRustflags = $env:RUSTFLAGS
    $env:RUSTFLAGS = "$prevRustflags -Awarnings"
    try {
        & cargo @launcherArgs 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'Compiling|Finished|error|warning|Running') {
                Write-Status "  $line" "DarkGray"
            }
        }
    } finally {
        $env:RUSTFLAGS = $prevRustflags
    }
    $sw.Stop()

    if ($LASTEXITCODE -ne 0) {
        throw "sgw-launcher build failed with exit code $LASTEXITCODE."
    }

    # Copy to project root for easy access
    $launcherBin = Join-Path $paths.ProjectRoot "target/$profile/sgw-launcher$exeSuffix"
    if (Test-Path $launcherBin) {
        $dest = Join-Path $paths.ProjectRoot "sgw-launcher$exeSuffix"
        Copy-Item $launcherBin $dest -Force
        $size = Format-FileSize (Get-Item $dest).Length
        Write-Status "OK: sgw-launcher ($size) -> copied to project root" "Green"
    }

    Write-Status "Build-CimmeriaLauncher complete ($Configuration, $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)." "Green"
}
