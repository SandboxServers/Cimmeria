function Find-PostgreSQL {
    <#
    .SYNOPSIS
        Finds PostgreSQL server binaries. Returns the bin directory path, or $null.

    .DESCRIPTION
        On Windows: looks in external/postgresql_server/bin/ (downloaded binary).
        On Linux/macOS: looks for pg_ctl in PATH (system-installed).
    #>
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    if ($isWin) {
        $paths = Get-ProjectPaths
        $pgBin = Join-Path $paths.ExternalDir "postgresql_server\bin"

        if (Test-Path (Join-Path $pgBin "postgres.exe")) {
            Write-Status "Found PostgreSQL: $pgBin" "Green"
            return $pgBin
        }
        if (Test-Path (Join-Path $pgBin "pg_ctl.exe")) {
            Write-Status "Found PostgreSQL: $pgBin" "Green"
            return $pgBin
        }

        Write-Status "PostgreSQL server binaries not found in external/postgresql_server/bin/" "Yellow"
        Write-Status "  Run Install-CimmeriaDependencies to extract them." "Yellow"
        return $null
    } else {
        # Linux/macOS: find in PATH
        $pgCtl = Get-Command pg_ctl -ErrorAction SilentlyContinue
        if ($pgCtl) {
            $pgBin = Split-Path $pgCtl.Source
            Write-Status "Found PostgreSQL: $pgBin" "Green"
            return $pgBin
        }

        # Search common Debian/Ubuntu locations (pg_ctl is often not in PATH)
        foreach ($ver in @(17, 16, 15)) {
            $candidate = "/usr/lib/postgresql/$ver/bin"
            if (Test-Path (Join-Path $candidate "pg_ctl")) {
                Write-Status "Found PostgreSQL: $candidate" "Green"
                return $candidate
            }
        }

        # Try pg_config to discover the bin directory
        $pgConfig = Get-Command pg_config -ErrorAction SilentlyContinue
        if ($pgConfig) {
            $pgBinDir = (& pg_config --bindir 2>&1).Trim()
            if ($pgBinDir -and (Test-Path (Join-Path $pgBinDir "pg_ctl"))) {
                Write-Status "Found PostgreSQL: $pgBinDir (via pg_config)" "Green"
                return $pgBinDir
            }
        }

        Write-Status "pg_ctl not found in PATH or common locations." "Yellow"
        Write-Status "  Install PostgreSQL 17+ via your system package manager." "Yellow"
        return $null
    }
}
