function Find-Qt {
    <#
    .SYNOPSIS
        Detects a Qt installation with qmake.exe.
        Returns a hashtable with InstallPath, BinPath, QmakePath, Version.
        Returns $null if not found.
    #>

    # 1. Check bootstrap-managed external/qt/
    $paths = Get-ProjectPaths
    $managedQt = Join-Path $paths.ExternalDir "qt"
    $managedQmake = Join-Path $managedQt "bin\qmake.exe"
    if (Test-Path $managedQmake) {
        $version = & $managedQmake -query QT_VERSION 2>$null
        if (-not $version) { $version = "5.15.x" }
        Write-Status "Found Qt: $managedQt (managed, v$version)" "Green"
        return @{
            InstallPath = $managedQt
            BinPath     = Join-Path $managedQt "bin"
            QmakePath   = $managedQmake
            Version     = $version
        }
    }

    # 2. Check QTDIR environment variable
    if ($env:QTDIR -and (Test-Path $env:QTDIR)) {
        $qmake = Join-Path $env:QTDIR "bin\qmake.exe"
        if (Test-Path $qmake) {
            $version = & $qmake -query QT_VERSION 2>$null
            if (-not $version) { $version = "unknown" }
            Write-Status "Found Qt: $env:QTDIR (QTDIR, v$version)" "Green"
            return @{
                InstallPath = $env:QTDIR
                BinPath     = Join-Path $env:QTDIR "bin"
                QmakePath   = $qmake
                Version     = $version
            }
        }
    }

    # 3. Scan C:\Qt\ for msvc2022/msvc2019 directories with 5.15.x
    $qtRoot = "C:\Qt"
    if (Test-Path $qtRoot) {
        $candidates = Get-ChildItem $qtRoot -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match '^\d+\.\d+' } |
            Sort-Object Name -Descending
        foreach ($verDir in $candidates) {
            $msvcDirs = Get-ChildItem $verDir.FullName -Directory -ErrorAction SilentlyContinue |
                Where-Object { $_.Name -match 'msvc\d+_64' } |
                Sort-Object Name -Descending
            foreach ($msvcDir in $msvcDirs) {
                $qmake = Join-Path $msvcDir.FullName "bin\qmake.exe"
                if (Test-Path $qmake) {
                    $version = & $qmake -query QT_VERSION 2>$null
                    if (-not $version) { $version = $verDir.Name }
                    Write-Status "Found Qt: $($msvcDir.FullName) (system, v$version)" "Green"
                    return @{
                        InstallPath = $msvcDir.FullName
                        BinPath     = Join-Path $msvcDir.FullName "bin"
                        QmakePath   = $qmake
                        Version     = $version
                    }
                }
            }
        }
    }

    Write-Status "Qt not found. Run Install-CimmeriaDependencies to download Qt." "Yellow"
    return $null
}
