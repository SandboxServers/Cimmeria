function Build-CimmeriaLibraries {
    <#
    .SYNOPSIS
        Builds Boost, OpenSSL, and SOCI libraries from source.

    .DESCRIPTION
        Runs the bootstrap .bat scripts to compile the three libraries that
        require building (as opposed to header-only or pre-built dependencies).

        Each build is idempotent - if output .lib files already exist, the step
        is skipped.

    .EXAMPLE
        Build-CimmeriaLibraries
    #>
    [CmdletBinding()]
    param()

    $paths = Get-ProjectPaths
    $ExternalDir = $paths.ExternalDir
    $BootstrapDir = $paths.BootstrapDir
    $ProjectRoot = $paths.ProjectRoot

    $Dependencies = Import-PowerShellDataFile (Join-Path $PSScriptRoot "..\Data\Dependencies.psd1")

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        Write-Status "Not running on Windows - skipping builds" "Yellow"
        return
    }

    Write-Step "BUILDING LIBRARIES"

    # --- Build 1/3: Boost ---
    Write-Host ""
    Write-Host "[Build 1/3] Boost $($Dependencies.Boost.Version)" -ForegroundColor White

    $boostDir = Join-Path $ExternalDir "boost"
    $boostLibDir = Join-Path $boostDir "lib64\lib"
    $boostLibExists = (Test-Path $boostLibDir) -and
        ((Get-ChildItem $boostLibDir -Filter "libboost_system-*.lib" -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0)

    if ($boostLibExists) {
        Write-Status "Boost: libraries already built, skipping" "DarkGray"
        $boostLibCount = (Get-ChildItem $boostLibDir -Filter "*.lib" | Measure-Object).Count
        Write-Status "  Found $boostLibCount .lib files in lib64/lib/" "DarkGray"
    } else {
        Write-Status "Building Boost (this may take several minutes)..." "White"

        $initResult = Invoke-BatchScript (Join-Path $BootstrapDir "init-boost.bat")
        if ($initResult.ExitCode -ne 0) {
            Write-Status "WARNING: init-boost.bat returned exit code $($initResult.ExitCode)" "Yellow"
            $initResult.Output | Select-Object -Last 5 | ForEach-Object { Write-Status "  $_" "DarkGray" }
        }

        $buildResult = Invoke-BatchScript (Join-Path $BootstrapDir "build-boost.bat")

        $boostLibExists = (Test-Path $boostLibDir) -and
            ((Get-ChildItem $boostLibDir -Filter "libboost_system-*.lib" -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0)

        if ($boostLibExists) {
            $boostLibCount = (Get-ChildItem $boostLibDir -Filter "*.lib" | Measure-Object).Count
            Write-Status "Boost: build successful ($boostLibCount .lib files)" "Green"
        } else {
            Write-Status "ERROR: Boost build failed - no output libraries found" "Red"
            $buildResult.Output | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            throw "Boost build failed."
        }
    }

    # --- Build 2/3: OpenSSL ---
    Write-Host ""
    Write-Host "[Build 2/3] OpenSSL $($Dependencies.OpenSSL.Version)" -ForegroundColor White

    $opensslDir = Join-Path $ExternalDir "openssl"
    $opensslLibDir = Join-Path $opensslDir "lib64"
    $opensslLibExists = (Test-Path (Join-Path $opensslLibDir "libeay32MT.lib"))

    if ($opensslLibExists) {
        Write-Status "OpenSSL: libraries already built, skipping" "DarkGray"
    } elseif (-not $script:PerlPath) {
        # Try to find Perl now if Install-CimmeriaDependencies wasn't called in this session
        $script:PerlPath = Find-Perl
        if (-not $script:PerlPath) {
            Write-Status "OpenSSL: SKIPPED - Perl not found (required for Configure)" "Yellow"
        }
    }

    if (-not $opensslLibExists -and $script:PerlPath) {
        Write-Status "Building OpenSSL (this takes 5-15 minutes, two full nmake passes)..." "White"

        # Stream the build with progress — OpenSSL is slow and silent otherwise
        $scriptPath = Join-Path $BootstrapDir "build-openssl.bat"
        $workDir = Split-Path $scriptPath
        $scriptName = Split-Path -Leaf $scriptPath
        Write-Status "  Running $scriptName..." "DarkGray"

        $lineCount = 0
        $output = @()
        cmd /c "cd /d `"$workDir`" && `"$scriptName`" < nul" 2>&1 | ForEach-Object {
            $output += $_
            $lineCount++
            $line = "$_"
            # Show key milestones and periodic progress
            if ($line -match '===|Configure|Running nmake|copied to|ERROR|FAILED|Using Perl|Found Visual') {
                Write-Status "  $_" "DarkGray"
            } elseif ($lineCount % 250 -eq 0) {
                Write-Status "  $lineCount lines processed..." "DarkGray"
            }
        }
        $buildExitCode = $LASTEXITCODE

        $opensslLibExists = (Test-Path (Join-Path $opensslLibDir "libeay32MT.lib"))
        $opensslDbgExists = (Test-Path (Join-Path $opensslLibDir "libeay32MTd.lib"))

        if ($opensslLibExists) {
            $libs = @()
            if ($opensslLibExists) { $libs += "libeay32MT.lib" }
            if ($opensslDbgExists) { $libs += "libeay32MTd.lib" }
            Write-Status "OpenSSL: build successful ($($libs -join ', '))" "Green"
        } else {
            Write-Status "ERROR: OpenSSL build failed - no output libraries found" "Red"
            $output | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            throw "OpenSSL build failed."
        }
    }

    # --- Build 3/3: SOCI ---
    Write-Host ""
    Write-Host "[Build 3/3] SOCI $($Dependencies.SOCI.Version)" -ForegroundColor White

    $sociDebugLib = Join-Path $ProjectRoot "lib64\debug\libsoci_core_3_2.lib"
    $sociReleaseLib = Join-Path $ProjectRoot "lib64\release\libsoci_core_3_2.lib"
    $sociLibExists = (Test-Path $sociDebugLib) -and (Test-Path $sociReleaseLib)

    if ($sociLibExists) {
        Write-Status "SOCI: libraries already built, skipping" "DarkGray"
    } else {
        Write-Status "Building SOCI (this should take about a minute)..." "White"

        $buildResult = Invoke-BatchScript (Join-Path $BootstrapDir "build-soci.bat")

        $sociLibExists = (Test-Path $sociDebugLib) -and (Test-Path $sociReleaseLib)

        if ($sociLibExists) {
            Write-Status "SOCI: build successful" "Green"
            Write-Status "  lib64/debug/libsoci_core_3_2.lib" "DarkGray"
            Write-Status "  lib64/debug/libsoci_postgresql_3_2.lib" "DarkGray"
            Write-Status "  lib64/release/libsoci_core_3_2.lib" "DarkGray"
            Write-Status "  lib64/release/libsoci_postgresql_3_2.lib" "DarkGray"
        } else {
            Write-Status "ERROR: SOCI build failed - no output libraries found" "Red"
            $buildResult.Output | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            throw "SOCI build failed."
        }
    }

    Write-Status "Build-CimmeriaLibraries complete." "Green"
}
