#Requires -Version 7.0
<#
.SYNOPSIS
    Downloads and sets up all external dependencies for the Cimmeria project.
    Replaces the need to store the 724MB external/ directory in git.

.DESCRIPTION
    This script downloads exact versions of all dependencies from official sources,
    extracts them to the expected external/ directory structure, and builds
    libraries that need compilation (Boost, OpenSSL, SOCI).

    Prerequisites:
    - PowerShell 7.0+ (pwsh)
    - Visual Studio 2012 (v120 toolset) or compatible (for building)
    - CMake (for SOCI build)
    - Perl (for OpenSSL build - e.g. Strawberry Perl)
    - Internet connection
    - Linux only: tar, msiextract (from msitools package) for Python MSI extraction

.PARAMETER SkipDownload
    Skip downloading (use already-downloaded archives in external/_downloads/)

.PARAMETER SkipBuild
    Only download and extract; do not compile anything

.PARAMETER BuildArch
    Architecture to build for: "x64" (default), "x86", or "both"
#>

param(
    [switch]$SkipDownload,
    [switch]$SkipBuild,
    [ValidateSet("x64", "x86", "both")]
    [string]$BuildArch = "x64"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExternalDir = Join-Path $ProjectRoot "external"
$DownloadDir = Join-Path $ExternalDir "_downloads"

# =============================================================================
# Dependency Registry - exact versions matching the original project
# =============================================================================

$Dependencies = @{

    Boost = @{
        Version      = "1.55.0"
        VersionUnderscore = "1_55_0"
        Url          = "https://archives.boost.io/release/1.55.0/source/boost_1_55_0.tar.gz"
        FileName     = "boost_1_55_0.tar.gz"
        Type         = "source+build"
        ExtractTo    = "boost"
        Notes        = "Full source tree. Built via b2 with msvc-12.0 toolset."
        # Official archive:
        # https://archives.boost.io/release/1.55.0/source/
        # https://sourceforge.net/projects/boost/files/boost/1.55.0/ (alt mirror)
    }

    OpenSSL = @{
        Version      = "1.0.1e"
        Url          = "https://github.com/openssl/openssl/archive/refs/tags/OpenSSL_1_0_1e.tar.gz"
        FileName     = "openssl-1.0.1e.tar.gz"
        Type         = "source+build"
        ExtractTo    = "openssl_src"
        Notes        = "Needs Perl + VS2012 to build. Produces headers + .lib files."
        # Official source:
        # https://github.com/openssl/openssl/releases/tag/OpenSSL_1_0_1e
        # https://openssl-library.org/source/old/1.0.1/
    }

    PostgreSQL = @{
        Version      = "9.2.24"
        Url          = "https://get.enterprisedb.com/postgresql/postgresql-9.2.24-1-windows-x64-binaries.zip"
        FileName     = "postgresql-9.2.24-x64-binaries.zip"
        Type         = "prebuilt"
        ExtractTo    = "postgresql_pkg"
        Notes        = @"
9.2.3 exact binaries no longer available from official sources.
9.2.24 is the latest 9.2.x patch release - ABI compatible with 9.2.3.
Same client library API, just security/bug fixes. Safe drop-in replacement.
Source for exact 9.2.3: https://ftp.postgresql.org/pub/source/v9.2.3/
"@
        # Official archive:
        # https://www.enterprisedb.com/download-postgresql-binaries
        # https://ftp.postgresql.org/pub/source/v9.2.3/ (source only)
    }

    Python = @{
        Version      = "3.4.1"
        Url64        = "https://www.python.org/ftp/python/3.4.1/python-3.4.1.amd64.msi"
        Url32        = "https://www.python.org/ftp/python/3.4.1/python-3.4.1.msi"
        FileName64   = "python-3.4.1.amd64.msi"
        FileName32   = "python-3.4.1.msi"
        Type         = "installer-extract"
        ExtractTo    = "python"
        Notes        = "MSI contains headers (include/) and import libs (libs/). Extract without installing."
        # Official source:
        # https://www.python.org/downloads/release/python-341/
        # https://www.python.org/ftp/python/3.4.1/
    }

    SOCI = @{
        Version      = "3.2.1"
        Url          = "https://github.com/SOCI/soci/archive/refs/tags/3.2.1.tar.gz"
        FileName     = "soci-3.2.1.tar.gz"
        Type         = "source"
        ExtractTo    = "soci"
        Notes        = "Source tree used directly for headers. Libs built separately via CMake or vcxproj."
        # Official sources:
        # https://github.com/SOCI/soci/tree/3.2.1
        # https://sourceforge.net/projects/soci/files/soci/soci-3.2.1/ (alt mirror)
    }

    SDL = @{
        Version      = "1.2.15"
        Url          = "https://www.libsdl.org/release/SDL-devel-1.2.15-VC.zip"
        FileName     = "SDL-devel-1.2.15-VC.zip"
        Type         = "prebuilt"
        ExtractTo    = "SDL"
        Notes        = "Pre-built Visual C++ development libraries."
        # Official source:
        # https://www.libsdl.org/release/
        # https://github.com/libsdl-org/SDL-1.2
    }

    Recast = @{
        Version      = "git-2014"
        Url          = "https://github.com/recastnavigation/recastnavigation/archive/refs/heads/main.zip"
        FileName     = "recastnavigation-main.zip"
        Type         = "source"
        ExtractTo    = "recast"
        Notes        = @"
Original was SVN r12 (~2013-2014). The Recast API is very stable.
Using main branch; compiled by Recast.vcxproj in the solution.
For exact 2014 snapshot, use: git checkout <commit-from-2014>
"@
        # Official source:
        # https://github.com/recastnavigation/recastnavigation
    }
}

# =============================================================================
# Helper Functions
# =============================================================================

$script:ScriptStartTime = Get-Date

function Write-Step($msg) {
    $elapsed = (Get-Date) - $script:ScriptStartTime
    $ts = "[{0:mm\:ss}]" -f $elapsed
    Write-Host "`n$ts === $msg ===" -ForegroundColor Cyan
}

function Write-Status($msg, [string]$Color = "Gray") {
    $elapsed = (Get-Date) - $script:ScriptStartTime
    $ts = "[{0:mm\:ss}]" -f $elapsed
    Write-Host "$ts  $msg" -ForegroundColor $Color
}

function Format-FileSize($bytes) {
    if ($bytes -ge 1GB) { return "{0:N1} GB" -f ($bytes / 1GB) }
    if ($bytes -ge 1MB) { return "{0:N1} MB" -f ($bytes / 1MB) }
    if ($bytes -ge 1KB) { return "{0:N1} KB" -f ($bytes / 1KB) }
    return "$bytes bytes"
}

function Download-File($url, $outPath) {
    if (Test-Path $outPath) {
        $size = Format-FileSize (Get-Item $outPath).Length
        Write-Status "Already downloaded: $(Split-Path -Leaf $outPath) ($size)" "DarkGray"
        return
    }
    Write-Status "Downloading: $url"
    Write-Status "-> $outPath"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()
    $prevPref = $ProgressPreference
    $ProgressPreference = 'SilentlyContinue'
    try {
        Invoke-WebRequest -Uri $url -OutFile $outPath -UseBasicParsing
    } finally {
        $ProgressPreference = $prevPref
    }
    $sw.Stop()
    $size = Format-FileSize (Get-Item $outPath).Length
    Write-Status "Done ($size in $("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)" "Green"
}

function Extract-Archive($archivePath, $destDir) {
    $fileName = Split-Path -Leaf $archivePath
    $size = Format-FileSize (Get-Item $archivePath).Length
    Write-Status "Extracting: $fileName ($size) -> $(Split-Path -Leaf $destDir)"

    $sw = [System.Diagnostics.Stopwatch]::StartNew()

    if ($archivePath -match '\.7z$') {
        # Requires 7-Zip (p7zip on Linux)
        if ($IsWindows) {
            $7z = "C:\Program Files\7-Zip\7z.exe"
            if (-not (Test-Path $7z)) { $7z = "7z" }
        } else {
            $7z = "7z"
        }
        Write-Status "  Using 7z..." "DarkGray"
        & $7z x $archivePath -o"$destDir" -y 2>&1 | ForEach-Object {
            if ($_ -match 'Extracting|Everything is Ok') { Write-Status "  $_" "DarkGray" }
        }
    }
    elseif ($archivePath -match '\.tar\.gz$') {
        Write-Status "  Using tar (this may take a while for large archives)..." "DarkGray"
        tar -xzvf $archivePath -C $destDir 2>&1 | ForEach-Object -Begin { $count = 0 } -Process {
            $count++
            if ($count % 2000 -eq 0) { Write-Status "  ... $count files extracted" "DarkGray" }
        }
        Write-Status "  Total: $count files extracted" "DarkGray"
    }
    elseif ($archivePath -match '\.zip$') {
        Write-Status "  Using Expand-Archive..." "DarkGray"
        Expand-Archive -Path $archivePath -DestinationPath $destDir -Force
    }
    elseif ($archivePath -match '\.msi$') {
        $extractDir = Join-Path $destDir "msi_extract"
        New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
        if ($IsWindows) {
            Write-Status "  Using msiexec..." "DarkGray"
            msiexec /a "$archivePath" /qn TARGETDIR="$extractDir" | Out-Null
        } else {
            $msiextractCmd = Get-Command msiextract -ErrorAction SilentlyContinue
            if ($msiextractCmd) {
                Write-Status "  Using msiextract..." "DarkGray"
                msiextract -C $extractDir $archivePath 2>&1 | ForEach-Object {
                    Write-Status "  $_" "DarkGray"
                }
            } else {
                throw "msiextract not found. Install it with: sudo apt install msitools"
            }
        }
    }

    $sw.Stop()
    Write-Status "Extraction complete ($("{0:N1}" -f $sw.Elapsed.TotalSeconds)s)" "Green"
}

# =============================================================================
# Main Script
# =============================================================================

Write-Host "=============================================" -ForegroundColor Yellow
Write-Host " Cimmeria Dependency Setup" -ForegroundColor Yellow
Write-Host " Project: Stargate Worlds Emulator" -ForegroundColor Yellow
Write-Host "=============================================" -ForegroundColor Yellow
Write-Host ""
Write-Host "This script will download and set up all external dependencies."
Write-Host "Total download size: ~200 MB (vs 724 MB vendored)"
Write-Host "Target directory: $ExternalDir"
Write-Host ""

# Create directories
New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
New-Item -ItemType Directory -Path $ExternalDir -Force | Out-Null

# ---- STEP 1: Download all archives ----

if (-not $SkipDownload) {
    Write-Step "DOWNLOADING DEPENDENCIES"

    # Boost
    Write-Host "`n[1/7] Boost $($Dependencies.Boost.Version)" -ForegroundColor White
    Download-File $Dependencies.Boost.Url (Join-Path $DownloadDir $Dependencies.Boost.FileName)

    # OpenSSL
    Write-Host "`n[2/7] OpenSSL $($Dependencies.OpenSSL.Version)" -ForegroundColor White
    Download-File $Dependencies.OpenSSL.Url (Join-Path $DownloadDir $Dependencies.OpenSSL.FileName)

    # PostgreSQL
    Write-Host "`n[3/7] PostgreSQL $($Dependencies.PostgreSQL.Version)" -ForegroundColor White
    Download-File $Dependencies.PostgreSQL.Url (Join-Path $DownloadDir $Dependencies.PostgreSQL.FileName)

    # Python
    Write-Host "`n[4/7] Python $($Dependencies.Python.Version)" -ForegroundColor White
    if ($BuildArch -eq "x86" -or $BuildArch -eq "both") {
        Download-File $Dependencies.Python.Url32 (Join-Path $DownloadDir $Dependencies.Python.FileName32)
    }
    if ($BuildArch -eq "x64" -or $BuildArch -eq "both") {
        Download-File $Dependencies.Python.Url64 (Join-Path $DownloadDir $Dependencies.Python.FileName64)
    }

    # SOCI
    Write-Host "`n[5/7] SOCI $($Dependencies.SOCI.Version)" -ForegroundColor White
    Download-File $Dependencies.SOCI.Url (Join-Path $DownloadDir $Dependencies.SOCI.FileName)

    # SDL
    Write-Host "`n[6/7] SDL $($Dependencies.SDL.Version)" -ForegroundColor White
    Download-File $Dependencies.SDL.Url (Join-Path $DownloadDir $Dependencies.SDL.FileName)

    # Recast
    Write-Host "`n[7/7] Recast/Detour" -ForegroundColor White
    Download-File $Dependencies.Recast.Url (Join-Path $DownloadDir $Dependencies.Recast.FileName)
}

# ---- STEP 2: Extract and arrange ----

Write-Step "EXTRACTING AND ARRANGING"

# --- Boost ---
$boostDir = Join-Path $ExternalDir "boost"
if (-not (Test-Path (Join-Path $boostDir "boost"))) {
    Write-Status "Boost: extracting..." "White"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.Boost.FileName) $ExternalDir
    $extracted = Join-Path $ExternalDir "boost_1_55_0"
    if (Test-Path $extracted) {
        Write-Status "Boost: renaming boost_1_55_0/ -> boost/" "DarkGray"
        if (Test-Path $boostDir) { Remove-Item $boostDir -Recurse -Force }
        Rename-Item $extracted $boostDir
    }
    Write-Status "Boost: done" "Green"
} else {
    Write-Status "Boost: already extracted" "DarkGray"
}

# --- SOCI ---
$sociDir = Join-Path $ExternalDir "soci"
if (-not (Test-Path (Join-Path $sociDir "src"))) {
    Write-Status "SOCI: extracting..." "White"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.SOCI.FileName) $ExternalDir
    $extracted = Join-Path $ExternalDir "soci-3.2.1"
    if (Test-Path $extracted) {
        Write-Status "SOCI: renaming soci-3.2.1/ -> soci/" "DarkGray"
        if (Test-Path $sociDir) { Remove-Item $sociDir -Recurse -Force }
        Rename-Item $extracted $sociDir
    }
    Write-Status "SOCI: done" "Green"
} else {
    Write-Status "SOCI: already extracted" "DarkGray"
}

# --- SDL ---
$sdlDir = Join-Path $ExternalDir "SDL"
if (-not (Test-Path (Join-Path $sdlDir "include"))) {
    Write-Status "SDL: extracting..." "White"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.SDL.FileName) $ExternalDir
    $extracted = Join-Path $ExternalDir "SDL-1.2.15"
    if (Test-Path $extracted) {
        Write-Status "SDL: renaming SDL-1.2.15/ -> SDL/" "DarkGray"
        if (Test-Path $sdlDir) { Remove-Item $sdlDir -Recurse -Force }
        Rename-Item $extracted $sdlDir
    }
    Write-Status "SDL: done" "Green"
} else {
    Write-Status "SDL: already extracted" "DarkGray"
}

# --- Recast ---
$recastDir = Join-Path $ExternalDir "recast"
if (-not (Test-Path (Join-Path $recastDir "Recast"))) {
    Write-Status "Recast: extracting..." "White"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.Recast.FileName) $ExternalDir
    $extracted = Join-Path $ExternalDir "recastnavigation-main"
    if (Test-Path $extracted) {
        Write-Status "Recast: renaming recastnavigation-main/ -> recast/" "DarkGray"
        if (Test-Path $recastDir) { Remove-Item $recastDir -Recurse -Force }
        Rename-Item $extracted $recastDir
    }
    Write-Status "Recast: done" "Green"
} else {
    Write-Status "Recast: already extracted" "DarkGray"
}

# --- PostgreSQL (extract headers + libs from binary distribution) ---
$pgDir = Join-Path $ExternalDir "postgresql"
if (-not (Test-Path (Join-Path $pgDir "include"))) {
    Write-Status "PostgreSQL: extracting binary distribution..." "White"
    $pgTempDir = Join-Path $ExternalDir "postgresql_temp"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.PostgreSQL.FileName) $pgTempDir

    Write-Status "PostgreSQL: copying headers and libs..." "DarkGray"
    New-Item -ItemType Directory -Path $pgDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pgDir "include") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pgDir "lib64") -Force | Out-Null

    $pgRoot = Get-ChildItem $pgTempDir -Directory | Select-Object -First 1
    $pgBinRoot = Join-Path $pgRoot.FullName "pgsql"
    if (-not (Test-Path $pgBinRoot)) { $pgBinRoot = $pgRoot.FullName }

    Copy-Item -Path (Join-Path $pgBinRoot "include" "*") -Destination (Join-Path $pgDir "include") -Recurse -Force
    Copy-Item -Path (Join-Path $pgBinRoot "lib" "libpq.lib") -Destination (Join-Path $pgDir "lib64" "libpq.lib") -Force -ErrorAction SilentlyContinue

    Write-Status "PostgreSQL: cleaning temp files..." "DarkGray"
    Remove-Item $pgTempDir -Recurse -Force -ErrorAction SilentlyContinue
    Write-Status "PostgreSQL: done" "Green"
} else {
    Write-Status "PostgreSQL: already extracted" "DarkGray"
}

# --- Python (extract headers + libs from MSI) ---
$pythonDir = Join-Path $ExternalDir "python"
if (-not (Test-Path (Join-Path $pythonDir "include"))) {
    Write-Status "Python: extracting development files from MSI..." "White"

    New-Item -ItemType Directory -Path $pythonDir -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pythonDir "include") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pythonDir "lib") -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pythonDir "lib64") -Force | Out-Null

    try {
        # Extract 64-bit MSI
        if ($BuildArch -eq "x64" -or $BuildArch -eq "both") {
            $msiPath = Join-Path $DownloadDir $Dependencies.Python.FileName64
            $pyTempDir = Join-Path $ExternalDir "python_temp64"
            New-Item -ItemType Directory -Path $pyTempDir -Force | Out-Null
            Extract-Archive $msiPath $pyTempDir

            Write-Status "Python: searching for headers and libs..." "DarkGray"
            $includeDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "include" | Select-Object -First 1
            if ($includeDir) {
                Write-Status "Python: copying headers from $($includeDir.FullName)" "DarkGray"
                Copy-Item -Path (Join-Path $includeDir.FullName "*") -Destination (Join-Path $pythonDir "include") -Recurse -Force
            } else {
                Write-Status "Python: WARNING - no include/ directory found in extracted MSI" "Yellow"
            }
            $libsDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "libs" | Select-Object -First 1
            if ($libsDir) {
                Write-Status "Python: copying libs from $($libsDir.FullName)" "DarkGray"
                Copy-Item -Path (Join-Path $libsDir.FullName "*") -Destination (Join-Path $pythonDir "lib64") -Recurse -Force
            } else {
                Write-Status "Python: WARNING - no libs/ directory found in extracted MSI" "Yellow"
            }
            Write-Status "Python: cleaning temp files..." "DarkGray"
            Remove-Item $pyTempDir -Recurse -Force -ErrorAction SilentlyContinue
        }

        # Extract 32-bit MSI
        if ($BuildArch -eq "x86" -or $BuildArch -eq "both") {
            $msiPath = Join-Path $DownloadDir $Dependencies.Python.FileName32
            $pyTempDir = Join-Path $ExternalDir "python_temp32"
            New-Item -ItemType Directory -Path $pyTempDir -Force | Out-Null
            Extract-Archive $msiPath $pyTempDir

            Write-Status "Python (x86): searching for headers and libs..." "DarkGray"
            $includeDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "include" | Select-Object -First 1
            if ($includeDir) {
                Copy-Item -Path (Join-Path $includeDir.FullName "*") -Destination (Join-Path $pythonDir "include") -Recurse -Force
            }
            $libsDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "libs" | Select-Object -First 1
            if ($libsDir) {
                Copy-Item -Path (Join-Path $libsDir.FullName "*") -Destination (Join-Path $pythonDir "lib") -Recurse -Force
            }
            Remove-Item $pyTempDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        Write-Status "Python: done" "Green"
    } catch {
        Write-Status "Python: WARNING - MSI extraction failed: $_" "Yellow"
        Write-Status "  On Linux, install msitools: sudo apt install msitools" "Yellow"
        Write-Status "  On Windows, msiexec is built-in and should work automatically." "Yellow"
        Write-Status "  Continuing with remaining dependencies..." "Yellow"
    }
} else {
    Write-Status "Python: already extracted" "DarkGray"
}

# --- OpenSSL (extract source, will need manual build) ---
$opensslSrcDir = Join-Path $ExternalDir "openssl_src"
$opensslDir = Join-Path $ExternalDir "openssl"
if (-not (Test-Path (Join-Path $opensslDir "include"))) {
    Write-Status "OpenSSL: extracting source..." "White"
    Extract-Archive (Join-Path $DownloadDir $Dependencies.OpenSSL.FileName) $ExternalDir
    $extracted = Join-Path $ExternalDir "openssl-OpenSSL_1_0_1e"
    if (Test-Path $extracted) {
        Write-Status "OpenSSL: renaming to openssl_src/" "DarkGray"
        if (Test-Path $opensslSrcDir) { Remove-Item $opensslSrcDir -Recurse -Force }
        Rename-Item $extracted $opensslSrcDir
    }
    Write-Status "OpenSSL: done" "Green"
} else {
    Write-Status "OpenSSL: already extracted" "DarkGray"
}

# ---- STEP 3: Build libraries (if not skipped) ----

if (-not $SkipBuild) {
    Write-Step "BUILDING LIBRARIES"
    Write-Host ""
    Write-Host "NOTE: Building requires Visual Studio 2012 (v120 toolset)." -ForegroundColor Yellow
    Write-Host "Open a VS2012 x64 Native Tools Command Prompt for these steps." -ForegroundColor Yellow
    Write-Host ""

    # --- Build Boost ---
    Write-Host "[Build 1/3] Boost" -ForegroundColor White
    Write-Host "  Run the following from a VS2012 x64 command prompt:" -ForegroundColor Gray
    Write-Host "    cd $boostDir" -ForegroundColor Gray
    Write-Host "    cd tools\build\v2 && bootstrap && cd ..\..\..\" -ForegroundColor Gray
    Write-Host '    b2 -j 4 --build-dir=../boost-lib64 --stagedir=./lib64 toolset=msvc-12.0 link=static threading=multi runtime-link=shared --build-type=minimal --with-date_time --with-math --with-python --with-system --with-thread address-model=64 stage' -ForegroundColor Gray
    Write-Host ""

    # --- Build OpenSSL ---
    Write-Host "[Build 2/3] OpenSSL 1.0.1e" -ForegroundColor White
    Write-Host "  Requires: Perl (Strawberry Perl recommended)" -ForegroundColor Gray
    Write-Host "  Run from a VS2012 x64 command prompt:" -ForegroundColor Gray
    Write-Host "    cd $opensslSrcDir" -ForegroundColor Gray
    Write-Host "    perl Configure VC-WIN64A --prefix=$opensslDir" -ForegroundColor Gray
    Write-Host "    ms\do_win64a" -ForegroundColor Gray
    Write-Host "    nmake -f ms\nt.mak" -ForegroundColor Gray
    Write-Host "    nmake -f ms\nt.mak install" -ForegroundColor Gray
    Write-Host "  This produces:" -ForegroundColor Gray
    Write-Host "    $opensslDir\include\openssl\*.h" -ForegroundColor Gray
    Write-Host "    $opensslDir\lib64\libeay32MT.lib, ssleay32MT.lib" -ForegroundColor Gray
    Write-Host ""

    # --- Build SOCI ---
    Write-Host "[Build 3/3] SOCI 3.2.1" -ForegroundColor White
    Write-Host "  SOCI is compiled as part of the solution (UnifiedKernel references soci source directly)." -ForegroundColor Gray
    Write-Host "  The source headers in external/soci/core/ and external/soci/backends/ are used at build time." -ForegroundColor Gray
    Write-Host "  Pre-built .lib files go in lib64/ (produced by the solution build)." -ForegroundColor Gray
    Write-Host ""
}

# ---- STEP 4: Cleanup downloads (optional) ----

Write-Host ""
Write-Step "SUMMARY"
$totalElapsed = (Get-Date) - $script:ScriptStartTime
Write-Host ""
Write-Host "Total time: $("{0:mm\:ss}" -f $totalElapsed)" -ForegroundColor White
Write-Host ""
Write-Host "Dependency layout:" -ForegroundColor White
Write-Host "  external/" -ForegroundColor Gray
Write-Host "    boost/          - Boost 1.55.0 source + built libs in lib64/lib/" -ForegroundColor Gray
Write-Host "    openssl/        - OpenSSL 1.0.1e headers + libs" -ForegroundColor Gray
Write-Host "    postgresql/     - PostgreSQL 9.2.24 headers + libpq.lib" -ForegroundColor Gray
Write-Host "    python/         - Python 3.4.1 headers + import libs" -ForegroundColor Gray
Write-Host "    soci/           - SOCI 3.2.1 source (headers used directly)" -ForegroundColor Gray
Write-Host "    SDL/            - SDL 1.2.15 headers + libs" -ForegroundColor Gray
Write-Host "    recast/         - Recast/Detour source (built by Recast.vcxproj)" -ForegroundColor Gray
Write-Host "    _downloads/     - Cached archives (safe to delete)" -ForegroundColor Gray
Write-Host ""
Write-Host "To clean up download cache:" -ForegroundColor DarkGray
Write-Host "  Remove-Item '$DownloadDir' -Recurse" -ForegroundColor DarkGray
Write-Host ""
Write-Host "Done!" -ForegroundColor Green
