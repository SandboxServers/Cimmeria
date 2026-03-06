function Initialize-CimmeriaRuntime {
    <#
    .SYNOPSIS
        Stages runtime DLLs into bin64/ alongside the server executables.

    .DESCRIPTION
        Copies required DLLs (python34.dll, libpq.dll and its dependencies) from the
        external dependency directories into the build output directory so the servers
        can find them at runtime.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Initialize-CimmeriaRuntime

    .EXAMPLE
        Initialize-CimmeriaRuntime -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug"
    )

    $paths = Get-ProjectPaths
    $binDir = Join-Path $paths.ProjectRoot "bin64"

    if (-not (Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    }

    Write-Step "STAGING RUNTIME DLLS"

    $pgServerBin = Join-Path $paths.ExternalDir "postgresql_server\bin"
    $pythonDir = Join-Path $paths.ExternalDir "python"

    # DLL manifest: source path -> description
    # Note: PG 17 bundles OpenSSL 3.x (libssl/libcrypto) instead of 1.0.x (ssleay32/libeay32)
    $dlls = @(
        @{ Name = "python34.dll";    Source = Join-Path $pythonDir "python34.dll";            Desc = "Python 3.4 runtime (Boost.Python)" }
        @{ Name = "libpq.dll";       Source = Join-Path $pgServerBin "libpq.dll";             Desc = "PostgreSQL client library (SOCI)" }
    )

    # Discover libpq runtime dependencies from PG server bin (names vary by PG version)
    if (Test-Path $pgServerBin) {
        $pgDepPatterns = @("libintl*.dll", "libiconv*.dll", "iconv.dll", "libssl*.dll", "libcrypto*.dll", "ssleay32.dll", "libeay32.dll", "zlib*.dll")
        foreach ($pattern in $pgDepPatterns) {
            Get-ChildItem $pgServerBin -Filter $pattern -ErrorAction SilentlyContinue | ForEach-Object {
                $dlls += @{ Name = $_.Name; Source = $_.FullName; Desc = "libpq dependency (auto-discovered)" }
            }
        }
    }

    $staged = 0
    $skipped = 0
    $missing = 0

    foreach ($dll in $dlls) {
        $destPath = Join-Path $binDir $dll.Name

        if (-not (Test-Path $dll.Source)) {
            Write-Status "WARNING: $($dll.Name) not found at $($dll.Source)" "Yellow"
            $missing++
            continue
        }

        if (Test-Path $destPath) {
            $srcHash = (Get-FileHash $dll.Source -Algorithm SHA256).Hash
            $dstHash = (Get-FileHash $destPath -Algorithm SHA256).Hash
            if ($srcHash -eq $dstHash) {
                Write-Status "Already staged: $($dll.Name)" "DarkGray"
                $skipped++
                continue
            }
        }

        Copy-Item -Path $dll.Source -Destination $destPath -Force
        Write-Status "Staged: $($dll.Name) - $($dll.Desc)" "Green"
        $staged++
    }

    Write-Status "$staged staged, $skipped already current, $missing missing" "White"

    if ($missing -gt 0) {
        Write-Status "Some DLLs were not found. Servers may fail to start." "Yellow"
        Write-Status "Run Install-CimmeriaDependencies to ensure all dependencies are extracted." "Yellow"
    }

    # Stage Qt DLLs for ServerEd
    $qtDir = Join-Path $paths.ExternalDir "qt"
    $qtBinDir = Join-Path $qtDir "bin"
    if (Test-Path $qtBinDir) {
        $qtDlls = @(
            @{ Name = "Qt5Core.dll";    Desc = "Qt Core module (ServerEd)" }
            @{ Name = "Qt5Gui.dll";     Desc = "Qt GUI module (ServerEd)" }
            @{ Name = "Qt5Widgets.dll"; Desc = "Qt Widgets module (ServerEd)" }
            @{ Name = "Qt5Xml.dll";     Desc = "Qt XML module (ServerEd)" }
            @{ Name = "Qt5Network.dll"; Desc = "Qt Network module (ServerEd)" }
            @{ Name = "Qt5Sql.dll";     Desc = "Qt SQL module (ServerEd)" }
        )

        foreach ($dll in $qtDlls) {
            $srcPath = Join-Path $qtBinDir $dll.Name
            $dstPath = Join-Path $binDir $dll.Name

            if (-not (Test-Path $srcPath)) {
                Write-Status "WARNING: $($dll.Name) not found at $srcPath" "Yellow"
                $missing++
                continue
            }

            if (Test-Path $dstPath) {
                $srcHash = (Get-FileHash $srcPath -Algorithm SHA256).Hash
                $dstHash = (Get-FileHash $dstPath -Algorithm SHA256).Hash
                if ($srcHash -eq $dstHash) {
                    Write-Status "Already staged: $($dll.Name)" "DarkGray"
                    $skipped++
                    continue
                }
            }

            Copy-Item -Path $srcPath -Destination $dstPath -Force
            Write-Status "Staged: $($dll.Name) - $($dll.Desc)" "Green"
            $staged++
        }

        # Stage Qt platform plugins (required for any Qt GUI app on Windows)
        $qtPlatformsDir = Join-Path $qtDir "plugins\platforms"
        $destPlatformsDir = Join-Path $binDir "platforms"
        if (Test-Path $qtPlatformsDir) {
            if (-not (Test-Path (Join-Path $destPlatformsDir "qwindows.dll"))) {
                Write-Status "Staging Qt platforms plugins..." "White"
                New-Item -ItemType Directory -Path $destPlatformsDir -Force | Out-Null
                Copy-Item -Path (Join-Path $qtPlatformsDir "*.dll") -Destination $destPlatformsDir -Force
                Write-Status "Staged: platforms/ - Qt platform plugins (qwindows.dll)" "Green"
            } else {
                Write-Status "Already staged: platforms/" "DarkGray"
            }
        }

        # Stage Qt SQL driver plugins (for database connectivity)
        $qtSqlDriversDir = Join-Path $qtDir "plugins\sqldrivers"
        $destSqlDriversDir = Join-Path $binDir "sqldrivers"
        if (Test-Path $qtSqlDriversDir) {
            if (-not (Test-Path $destSqlDriversDir) -or
                (Get-ChildItem $destSqlDriversDir -Filter "*.dll" -ErrorAction SilentlyContinue | Measure-Object).Count -eq 0) {
                Write-Status "Staging Qt SQL driver plugins..." "White"
                New-Item -ItemType Directory -Path $destSqlDriversDir -Force | Out-Null
                Copy-Item -Path (Join-Path $qtSqlDriversDir "*.dll") -Destination $destSqlDriversDir -Force
                Write-Status "Staged: sqldrivers/ - Qt SQL driver plugins" "Green"
            } else {
                Write-Status "Already staged: sqldrivers/" "DarkGray"
            }
        }
    } else {
        Write-Status "Qt not found - ServerEd DLLs not staged (run Install-CimmeriaDependencies)" "DarkGray"
    }

    # Stage Python standard library (embedded Python needs encodings, codecs, etc.)
    $pythonLib = Join-Path $pythonDir "Lib"
    $destLib = Join-Path $binDir "Lib"
    if (Test-Path $pythonLib) {
        if (-not (Test-Path (Join-Path $destLib "encodings"))) {
            Write-Status "Staging Python standard library (Lib/)..." "White"
            Copy-Item -Path $pythonLib -Destination $destLib -Recurse -Force
            Write-Status "Staged: Lib/ - Python 3.4 standard library" "Green"
        } else {
            Write-Status "Already staged: Lib/" "DarkGray"
        }
    } else {
        Write-Status "WARNING: Python Lib/ not found. Run Install-CimmeriaDependencies to extract." "Yellow"
    }

    # Stage Python compiled extension modules (_socket.pyd, _ssl.pyd, etc.)
    $pythonDlls = Join-Path $pythonDir "DLLs"
    $destDlls = Join-Path $binDir "DLLs"
    if (Test-Path $pythonDlls) {
        if (-not (Test-Path (Join-Path $destDlls "_socket.pyd"))) {
            Write-Status "Staging Python extension modules (DLLs/)..." "White"
            Copy-Item -Path $pythonDlls -Destination $destDlls -Recurse -Force
            Write-Status "Staged: DLLs/ - Python 3.4 compiled extensions" "Green"
        } else {
            Write-Status "Already staged: DLLs/" "DarkGray"
        }
    } else {
        Write-Status "WARNING: Python DLLs/ not found. Run Install-CimmeriaDependencies to extract." "Yellow"
    }

    Write-Status "Initialize-CimmeriaRuntime complete." "Green"
}
