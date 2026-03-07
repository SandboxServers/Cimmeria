function Install-CimmeriaDependencies {
    <#
    .SYNOPSIS
        Downloads and extracts PostgreSQL server binaries (Windows),
        or verifies system PostgreSQL (Linux/macOS).

    .DESCRIPTION
        On Windows: downloads the PG binary distribution and extracts headers,
        libs, and server binaries into external/.

        On Linux: checks for pg_config in PATH and verifies version >= 17.

        On macOS: checks for Homebrew PostgreSQL installation.

        Cargo handles all Rust dependencies automatically.

    .PARAMETER SkipDownload
        Skip downloading archives (use already-cached files).

    .EXAMPLE
        Install-CimmeriaDependencies

    .EXAMPLE
        Install-CimmeriaDependencies -SkipDownload
    #>
    [CmdletBinding()]
    param(
        [switch]$SkipDownload
    )

    $paths = Get-ProjectPaths
    $ExternalDir = $paths.ExternalDir
    $DownloadDir = $paths.DownloadDir
    $Dependencies = Import-PowerShellDataFile (Join-Path $PSScriptRoot "..\Data\Dependencies.psd1")

    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host " Cimmeria Dependency Setup" -ForegroundColor Yellow
    Write-Host " Project: Stargate Worlds Emulator (Rust)" -ForegroundColor Yellow
    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "Rust crate dependencies are managed by Cargo automatically."
    Write-Host "This step handles PostgreSQL server setup only."
    Write-Host ""

    New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
    New-Item -ItemType Directory -Path $ExternalDir -Force | Out-Null

    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    $isMac = (Test-Path variable:IsMacOS) -and $IsMacOS
    $isLinux = (Test-Path variable:IsLinux) -and $IsLinux

    if ($isWin) {
        # Windows: download and extract PG binary distribution
        Write-Step "POSTGRESQL (WINDOWS BINARY DISTRIBUTION)"

        if (-not $SkipDownload) {
            Write-Status "Downloading PostgreSQL $($Dependencies.PostgreSQL.Version)..." "White"
            Get-DownloadFile $Dependencies.PostgreSQL.Windows.Url (Join-Path $DownloadDir $Dependencies.PostgreSQL.Windows.FileName)
        }

        $pgDir = Join-Path $ExternalDir "postgresql"
        $pgServerDir = Join-Path $ExternalDir "postgresql_server"
        if (-not (Test-Path (Join-Path $pgDir "include")) -or -not (Test-Path (Join-Path $pgServerDir "bin"))) {
            Write-Status "PostgreSQL: extracting binary distribution..." "White"
            $pgTempDir = Join-Path $ExternalDir "postgresql_temp"
            Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.PostgreSQL.Windows.FileName) $pgTempDir

            Write-Status "PostgreSQL: copying headers and libs..." "DarkGray"
            New-Item -ItemType Directory -Path $pgDir -Force | Out-Null
            New-Item -ItemType Directory -Path (Join-Path $pgDir "include") -Force | Out-Null
            New-Item -ItemType Directory -Path (Join-Path $pgDir "lib64") -Force | Out-Null

            $pgRoot = Get-ChildItem $pgTempDir -Directory | Select-Object -First 1
            $pgBinRoot = Join-Path $pgRoot.FullName "pgsql"
            if (-not (Test-Path $pgBinRoot)) { $pgBinRoot = $pgRoot.FullName }

            Copy-Item -Path (Join-Path $pgBinRoot "include" "*") -Destination (Join-Path $pgDir "include") -Recurse -Force
            Copy-Item -Path (Join-Path $pgBinRoot "lib" "libpq.lib") -Destination (Join-Path $pgDir "lib64" "libpq.lib") -Force -ErrorAction SilentlyContinue

            Write-Status "PostgreSQL: preserving server distribution for runtime..." "DarkGray"
            if (Test-Path $pgServerDir) { Remove-Item $pgServerDir -Recurse -Force }
            if (Test-Path (Join-Path $pgBinRoot "bin\postgres.exe")) {
                Move-Item $pgBinRoot $pgServerDir
            } else {
                Move-Item $pgRoot.FullName $pgServerDir
            }

            Write-Status "PostgreSQL: cleaning temp files..." "DarkGray"
            Remove-Item $pgTempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Status "PostgreSQL: done" "Green"
        } else {
            Write-Status "PostgreSQL: already extracted" "DarkGray"
        }
    } elseif ($isLinux) {
        # Linux: verify system PostgreSQL
        Write-Step "POSTGRESQL (SYSTEM PACKAGE)"

        $pgConfig = Get-Command pg_config -ErrorAction SilentlyContinue
        if ($pgConfig) {
            $pgVersion = (pg_config --version 2>&1) -join ""
            Write-Status "Found: $pgVersion" "Green"
            if ($pgVersion -notmatch '1[7-9]\.|[2-9]\d\.') {
                Write-Status "WARNING: PostgreSQL 17+ recommended. Found: $pgVersion" "Yellow"
            }
        } else {
            Write-Status "PostgreSQL not found in PATH." "Red"
            Write-Status "  Install: sudo apt install $($Dependencies.PostgreSQL.Linux.PackageName)" "Yellow"
            Write-Status "  Or:      sudo dnf install $($Dependencies.PostgreSQL.Linux.PackageName)-server" "Yellow"
            throw "PostgreSQL not found. Install it and re-run."
        }
    } elseif ($isMac) {
        # macOS: verify Homebrew PostgreSQL
        Write-Step "POSTGRESQL (HOMEBREW)"

        $pgConfig = Get-Command pg_config -ErrorAction SilentlyContinue
        if ($pgConfig) {
            $pgVersion = (pg_config --version 2>&1) -join ""
            Write-Status "Found: $pgVersion" "Green"
        } else {
            Write-Status "PostgreSQL not found." "Red"
            Write-Status "  Install: brew install $($Dependencies.PostgreSQL.MacOS.BrewFormula)" "Yellow"
            throw "PostgreSQL not found. Install it and re-run."
        }
    }

    Write-Status "Install-CimmeriaDependencies complete." "Green"
}
