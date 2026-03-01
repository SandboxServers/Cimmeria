function Find-PostgreSQL {
    <#
    .SYNOPSIS
        Finds PostgreSQL server binaries in external/postgresql_server/bin/.
        Returns the bin directory path, or $null if not found.
    #>
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
}
