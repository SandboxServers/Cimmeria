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
    $binDir = Join-Path $paths.ProjectRoot "bin64\$($Configuration.ToLower())"

    if (-not (Test-Path $binDir)) {
        New-Item -ItemType Directory -Path $binDir -Force | Out-Null
    }

    Write-Step "STAGING RUNTIME DLLS"

    $pgServerBin = Join-Path $paths.ExternalDir "postgresql_server\bin"
    $pythonDir = Join-Path $paths.ExternalDir "python"

    # DLL manifest: source path -> description
    $dlls = @(
        @{ Name = "python34.dll";    Source = Join-Path $pythonDir "python34.dll";            Desc = "Python 3.4 runtime (Boost.Python)" }
        @{ Name = "libpq.dll";       Source = Join-Path $pgServerBin "libpq.dll";             Desc = "PostgreSQL client library (SOCI)" }
        @{ Name = "libintl.dll";     Source = Join-Path $pgServerBin "libintl.dll";           Desc = "libpq dependency" }
        @{ Name = "libiconv-2.dll";  Source = Join-Path $pgServerBin "libiconv-2.dll";        Desc = "libpq dependency" }
        @{ Name = "ssleay32.dll";    Source = Join-Path $pgServerBin "ssleay32.dll";          Desc = "libpq dependency (PG OpenSSL)" }
        @{ Name = "libeay32.dll";    Source = Join-Path $pgServerBin "libeay32.dll";          Desc = "libpq dependency (PG OpenSSL)" }
    )

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

    Write-Status "Initialize-CimmeriaRuntime complete." "Green"
}
