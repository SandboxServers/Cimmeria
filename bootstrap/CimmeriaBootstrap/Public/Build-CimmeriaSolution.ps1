function Build-CimmeriaSolution {
    <#
    .SYNOPSIS
        Builds the W-NG.sln Visual Studio solution via MSBuild.

    .DESCRIPTION
        Locates MSBuild from the Visual Studio installation and builds the Cimmeria
        solution for the specified configuration and platform.

        Verifies that the expected server executables are produced in bin64/.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .PARAMETER Platform
        Build platform. Default is "x64".

    .EXAMPLE
        Build-CimmeriaSolution

    .EXAMPLE
        Build-CimmeriaSolution -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug",

        [string]$Platform = "x64"
    )

    $paths = Get-ProjectPaths

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        throw "Build-CimmeriaSolution requires Windows with Visual Studio."
    }

    Write-Step "BUILDING SOLUTION"

    # Find Visual Studio
    if (-not $script:VSInfo) {
        $script:VSInfo = Find-VisualStudio
    }
    if (-not $script:VSInfo) {
        throw "Visual Studio not found. Install VS with C++ tools first."
    }

    # Find MSBuild
    $msbuildPath = Join-Path $script:VSInfo.InstallPath "MSBuild\Current\Bin\amd64\MSBuild.exe"
    if (-not (Test-Path $msbuildPath)) {
        # Try older VS layout
        $msbuildPath = Join-Path $script:VSInfo.InstallPath "MSBuild\Current\Bin\MSBuild.exe"
    }
    if (-not (Test-Path $msbuildPath)) {
        throw "MSBuild.exe not found at expected path: $msbuildPath"
    }
    Write-Status "MSBuild: $msbuildPath" "DarkGray"

    # Build
    $slnPath = Join-Path $paths.ProjectRoot "W-NG.sln"
    if (-not (Test-Path $slnPath)) {
        throw "Solution file not found: $slnPath"
    }

    Write-Status "Building $Configuration|$Platform..." "White"
    $buildArgs = @(
        $slnPath,
        "/p:Configuration=$Configuration",
        "/p:Platform=$Platform",
        "/m",
        "/nologo",
        "/v:minimal"
    )

    & $msbuildPath @buildArgs
    if ($LASTEXITCODE -ne 0) {
        throw "MSBuild failed with exit code $LASTEXITCODE."
    }

    # Verify executables
    $binDir = Join-Path $paths.ProjectRoot "bin64"
    $suffix = if ($Configuration -eq "Debug") { "_d" } else { "" }

    $expectedExes = @(
        "AuthenticationServer${suffix}.exe",
        "BaseApp${suffix}.exe",
        "CellApp${suffix}.exe"
    )

    $allFound = $true
    foreach ($exe in $expectedExes) {
        $exePath = Join-Path $binDir $exe
        if (Test-Path $exePath) {
            Write-Status "  Found: $exe" "Green"
        } else {
            Write-Status "  Missing: $exe" "Red"
            $allFound = $false
        }
    }

    if (-not $allFound) {
        throw "Not all server executables were produced. Check build output."
    }

    Write-Status "Build-CimmeriaSolution complete ($Configuration)." "Green"
}
