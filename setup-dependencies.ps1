#Requires -Version 7.0
<#
.SYNOPSIS
    Downloads, patches, and builds all external dependencies for the Cimmeria project.
    Replaces the need to store the 724MB external/ directory in git.

.DESCRIPTION
    This script is the single entry point for bootstrapping a fresh Cimmeria build
    environment. It performs these phases:

    Phase 0:   Detect Visual Studio 2015+ (optionally install VS Community)
    Phase 0.5: Detect Perl (for OpenSSL build)
    Phase 1:   Download all dependency archives
    Phase 2:   Extract and arrange into external/
    Phase 3:   Apply VS2015+ compatibility patches from bootstrap/patches/
    Phase 4:   Build libraries (Boost, OpenSSL, SOCI)
    Phase 5:   Summary and verification

    Prerequisites:
    - PowerShell 7.0+ (pwsh)
    - Windows 10/11 or Windows Server
    - Internet connection (for downloads)
    - ~2 GB free disk space

    VS and Perl are detected automatically. Pass -InstallVS to install
    VS Community if not found.

.PARAMETER SkipDownload
    Skip downloading (use already-downloaded archives in external/_downloads/)

.PARAMETER SkipBuild
    Only download, extract, and patch; do not compile anything

.PARAMETER InstallVS
    If Visual Studio is not found, download and install VS Community with
    the C++ Desktop workload. Requires a script restart after install.

.PARAMETER BuildArch
    Architecture to build for: "x64" (default), "x86", or "both"
#>

param(
    [switch]$SkipDownload,
    [switch]$SkipBuild,
    [switch]$InstallVS,
    [ValidateSet("x64", "x86", "both")]
    [string]$BuildArch = "x64"
)

$ErrorActionPreference = "Stop"
$ProjectRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$ExternalDir = Join-Path $ProjectRoot "external"
$DownloadDir = Join-Path $ExternalDir "_downloads"
$BootstrapDir = Join-Path $ProjectRoot "bootstrap"
$PatchDir = Join-Path $BootstrapDir "patches"
$TemplateDir = Join-Path $BootstrapDir "templates"

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
        Notes        = "Needs Perl + MSVC to build. Produces headers + .lib files."
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
        Notes        = "Source tree used directly for headers. Libs built separately via build-soci.bat."
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
Write-Host "This script will download, patch, and build all external dependencies."
Write-Host "Total download size: ~200 MB (vs 724 MB vendored)"
Write-Host "Target directory: $ExternalDir"
Write-Host ""

# Create directories
New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
New-Item -ItemType Directory -Path $ExternalDir -Force | Out-Null

# =============================================================================
# PHASE 0: Visual Studio Detection / Bootstrap
# =============================================================================

Write-Step "PHASE 0: VISUAL STUDIO DETECTION"

$script:VSInstallPath = $null
$script:VCToolsVersion = $null
$script:ClExePath = $null

if ($IsWindows -or (-not (Test-Path variable:IsWindows))) {
    # Find vswhere.exe
    $vswhereExe = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhereExe)) {
        # Try alternate location
        $vswhereExe = "$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe"
    }

    if (Test-Path $vswhereExe) {
        # Query for VS with C++ workload, version 15.0+ (VS2017+)
        $vsInfo = & $vswhereExe -latest -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($vsInfo) {
            $script:VSInstallPath = $vsInfo.Trim()
        } else {
            # Fall back to any VS installation
            $vsInfo = & $vswhereExe -latest -property installationPath 2>$null
            if ($vsInfo) {
                $script:VSInstallPath = $vsInfo.Trim()
                Write-Status "WARNING: VS found but C++ tools may not be installed" "Yellow"
            }
        }

        if ($script:VSInstallPath) {
            # Get VS display name and version
            $vsName = & $vswhereExe -latest -property displayName 2>$null
            $vsVersion = & $vswhereExe -latest -property installationVersion 2>$null
            Write-Status "Found: $vsName (v$vsVersion)" "Green"
            Write-Status "Path:  $script:VSInstallPath" "DarkGray"

            # Find MSVC tools version
            $vcToolsDir = Join-Path $script:VSInstallPath "VC\Tools\MSVC"
            if (Test-Path $vcToolsDir) {
                $script:VCToolsVersion = (Get-ChildItem $vcToolsDir -Directory | Sort-Object Name -Descending | Select-Object -First 1).Name
                Write-Status "MSVC Tools: $script:VCToolsVersion" "DarkGray"

                # Find cl.exe
                $clPath = Join-Path $vcToolsDir "$script:VCToolsVersion\bin\Hostx64\x64\cl.exe"
                if (Test-Path $clPath) {
                    $script:ClExePath = $clPath
                    Write-Status "cl.exe: $script:ClExePath" "DarkGray"
                }
            }
        }
    }

    if (-not $script:VSInstallPath) {
        if ($InstallVS) {
            Write-Status "Visual Studio not found. Downloading VS Community installer..." "Yellow"
            $vsInstallerUrl = "https://aka.ms/vs/18/release/vs_community.exe"
            $vsInstallerPath = Join-Path $env:TEMP "vs_community.exe"

            $prevPref = $ProgressPreference
            $ProgressPreference = 'SilentlyContinue'
            try {
                Invoke-WebRequest -Uri $vsInstallerUrl -OutFile $vsInstallerPath -UseBasicParsing
            } finally {
                $ProgressPreference = $prevPref
            }

            Write-Status "Running VS Community installer (this may take 10-30 minutes)..." "Yellow"
            Write-Status "Installing workloads: C++ Desktop Development" "DarkGray"

            $vsArgs = @(
                "--add", "Microsoft.VisualStudio.Workload.NativeDesktop",
                "--add", "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
                "--add", "Microsoft.VisualStudio.Component.Windows11SDK.26100",
                "--passive", "--norestart", "--wait"
            )
            $vsProc = Start-Process -FilePath $vsInstallerPath -ArgumentList $vsArgs -Wait -PassThru
            if ($vsProc.ExitCode -ne 0 -and $vsProc.ExitCode -ne 3010) {
                Write-Host ""
                Write-Host "ERROR: VS installer exited with code $($vsProc.ExitCode)" -ForegroundColor Red
                Write-Host "Check the installer log for details." -ForegroundColor Red
                exit 1
            }

            Write-Host ""
            Write-Host "Visual Studio Community installed successfully." -ForegroundColor Green
            Write-Host "IMPORTANT: Restart your PowerShell session, then re-run this script." -ForegroundColor Yellow
            Write-Host "  pwsh setup-dependencies.ps1" -ForegroundColor White
            exit 0
        } else {
            Write-Host ""
            Write-Host "ERROR: Visual Studio with C++ tools not found." -ForegroundColor Red
            Write-Host ""
            Write-Host "Options:" -ForegroundColor White
            Write-Host "  1. Install manually from: https://visualstudio.microsoft.com/downloads/" -ForegroundColor Gray
            Write-Host "     Select 'Desktop development with C++' workload" -ForegroundColor Gray
            Write-Host ""
            Write-Host "  2. Re-run this script with -InstallVS to auto-install VS Community:" -ForegroundColor Gray
            Write-Host "     pwsh setup-dependencies.ps1 -InstallVS" -ForegroundColor White
            exit 1
        }
    }
} else {
    Write-Status "Not running on Windows - skipping VS detection" "DarkGray"
    Write-Status "Build steps will be skipped (download + extract + patch only)" "DarkGray"
}

# =============================================================================
# PHASE 0.5: Perl Detection (needed for OpenSSL build)
# =============================================================================

Write-Step "PHASE 0.5: PERL DETECTION"

$script:PerlPath = $null

if ($IsWindows -or (-not (Test-Path variable:IsWindows))) {
    # Check for standalone Perl first (e.g. Strawberry Perl)
    $perlCandidates = Get-Command perl -ErrorAction SilentlyContinue -All
    foreach ($candidate in $perlCandidates) {
        if ($candidate.Source -notmatch '\\usr\\bin\\') {
            $script:PerlPath = $candidate.Source
            break
        }
    }

    # Fall back to Git's bundled Perl
    if (-not $script:PerlPath) {
        $gitCmd = Get-Command git -ErrorAction SilentlyContinue
        if ($gitCmd) {
            $gitDir = Split-Path (Split-Path $gitCmd.Source)
            $gitPerl = Join-Path $gitDir "usr\bin\perl.exe"
            if (Test-Path $gitPerl) {
                $script:PerlPath = $gitPerl
            }
        }
    }

    if ($script:PerlPath) {
        Write-Status "Found Perl: $script:PerlPath" "Green"
    } else {
        Write-Status "WARNING: Perl not found. OpenSSL build will fail." "Yellow"
        Write-Status "  Install Git for Windows (bundles Perl) or Strawberry Perl." "Yellow"
    }
} else {
    # Linux/macOS - perl is usually available
    $perlCmd = Get-Command perl -ErrorAction SilentlyContinue
    if ($perlCmd) {
        $script:PerlPath = $perlCmd.Source
        Write-Status "Found Perl: $script:PerlPath" "Green"
    } else {
        Write-Status "WARNING: Perl not found." "Yellow"
    }
}

# =============================================================================
# PHASE 1: Download all archives
# =============================================================================

if (-not $SkipDownload) {
    Write-Step "PHASE 1: DOWNLOADING DEPENDENCIES"

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

# =============================================================================
# PHASE 2: Extract and arrange
# =============================================================================

Write-Step "PHASE 2: EXTRACTING AND ARRANGING"

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

# --- OpenSSL (extract source, will be patched and built later) ---
$opensslSrcDir = Join-Path $ExternalDir "openssl_src"
$opensslDir = Join-Path $ExternalDir "openssl"
if (-not (Test-Path (Join-Path $opensslSrcDir "Configure"))) {
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

# =============================================================================
# PHASE 3: Apply VS2015+ compatibility patches
# =============================================================================

Write-Step "PHASE 3: APPLYING COMPATIBILITY PATCHES"

# Patch mapping: source file in bootstrap/patches/ -> target in external/
$patchMap = @{
    "boost_auto_link.hpp"   = "boost\boost\config\auto_link.hpp"
    "soci_platform.h"       = "soci\src\core\soci-platform.h"
    "openssl_e_os.h"        = "openssl_src\e_os.h"
    "openssl_e_padlock.c"   = "openssl_src\engines\e_padlock.c"
}

$patchDescriptions = @{
    "boost_auto_link.hpp"   = "Add vc140/vc145 toolset entries for VS2015+ auto-linking"
    "soci_platform.h"       = "Guard snprintf/strtoll macros for VS2013+ native CRT"
    "openssl_e_os.h"        = "Skip stdin/stdout/stderr redefinition on VS2015+ UCRT"
    "openssl_e_padlock.c"   = "Guard x86 inline asm with _M_IX86 for x64 builds"
}

$patchesApplied = 0
foreach ($patchFile in $patchMap.Keys) {
    $sourcePath = Join-Path $PatchDir $patchFile
    $targetPath = Join-Path $ExternalDir $patchMap[$patchFile]

    if (-not (Test-Path $sourcePath)) {
        Write-Status "WARNING: Patch file not found: $sourcePath" "Yellow"
        continue
    }

    if (-not (Test-Path $targetPath)) {
        Write-Status "SKIP: Target not yet extracted: $($patchMap[$patchFile])" "DarkGray"
        continue
    }

    # Check if patch is already applied by comparing file contents
    $sourceHash = (Get-FileHash $sourcePath -Algorithm SHA256).Hash
    $targetHash = (Get-FileHash $targetPath -Algorithm SHA256).Hash

    if ($sourceHash -eq $targetHash) {
        Write-Status "Already patched: $patchFile" "DarkGray"
    } else {
        Copy-Item -Path $sourcePath -Destination $targetPath -Force
        Write-Status "Patched: $($patchMap[$patchFile])" "Green"
        Write-Status "  -> $($patchDescriptions[$patchFile])" "DarkGray"
        $patchesApplied++
    }
}

Write-Status "$patchesApplied patch(es) applied" "White"

# --- Generate user-config.jam for Boost ---
Write-Status "" "White"
Write-Status "Generating Boost user-config.jam..." "White"

$userConfigPath = Join-Path $boostDir "user-config.jam"
$templatePath = Join-Path $TemplateDir "user-config.jam.template"

if ($script:ClExePath -and (Test-Path $templatePath)) {
    # Determine MSVC toolset version for b2
    # VCToolsVersion is like "14.50.35717" - we want "14.5" for b2
    $toolsetVer = $script:VCToolsVersion
    if ($toolsetVer -match '^(\d+)\.(\d+)') {
        $major = $Matches[1]
        $minor = $Matches[2]
        # b2 toolset version: major.minor with trailing zeros stripped
        # 14.50 -> 14.5, 14.30 -> 14.3, etc.
        $minorTrimmed = $minor.TrimEnd('0')
        if ($minorTrimmed -eq '') { $minorTrimmed = '0' }
        $b2Toolset = "$major.$minorTrimmed"
    } else {
        $b2Toolset = "14.5"
        Write-Status "WARNING: Could not parse VCToolsVersion, defaulting to 14.5" "Yellow"
    }

    $clPathEscaped = $script:ClExePath -replace '\\', '\\\\'
    $template = Get-Content $templatePath -Raw
    $generated = $template -replace '\{\{MSVC_TOOLSET_VERSION\}\}', $b2Toolset
    $generated = $generated -replace '\{\{MSVC_CL_PATH\}\}', $clPathEscaped

    Set-Content -Path $userConfigPath -Value $generated -NoNewline
    Write-Status "Generated: $userConfigPath" "Green"
    Write-Status "  MSVC toolset: msvc-$b2Toolset" "DarkGray"
    Write-Status "  cl.exe: $script:ClExePath" "DarkGray"
} elseif (Test-Path $userConfigPath) {
    Write-Status "user-config.jam already exists, keeping current version" "DarkGray"
} else {
    Write-Status "WARNING: Cannot generate user-config.jam (no cl.exe found or template missing)" "Yellow"
    Write-Status "  You will need to create external/boost/user-config.jam manually" "Yellow"
}

# =============================================================================
# PHASE 4: Build libraries
# =============================================================================

if (-not $SkipBuild) {
    Write-Step "PHASE 4: BUILDING LIBRARIES"

    if (-not ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        Write-Status "Not running on Windows - skipping builds" "Yellow"
        Write-Status "Build the libraries on Windows using the .bat scripts" "Yellow"
    } else {
        # --- Build 1/3: Boost ---
        Write-Host ""
        Write-Host "[Build 1/3] Boost $($Dependencies.Boost.Version)" -ForegroundColor White

        $boostLibDir = Join-Path $boostDir "lib64\lib"
        $boostLibExists = (Test-Path $boostLibDir) -and
            ((Get-ChildItem $boostLibDir -Filter "libboost_system-*.lib" -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0)

        if ($boostLibExists) {
            Write-Status "Boost: libraries already built, skipping" "DarkGray"
            $boostLibCount = (Get-ChildItem $boostLibDir -Filter "*.lib" | Measure-Object).Count
            Write-Status "  Found $boostLibCount .lib files in lib64/lib/" "DarkGray"
        } else {
            Write-Status "Building Boost (this may take several minutes)..." "White"
            Write-Status "  Running init-boost.bat..." "DarkGray"

            $initResult = cmd /c "cd /d `"$BootstrapDir`" && init-boost.bat < nul" 2>&1
            $initExit = $LASTEXITCODE
            if ($initExit -ne 0) {
                Write-Status "WARNING: init-boost.bat returned exit code $initExit" "Yellow"
                # Show last few lines of output for debugging
                $initResult | Select-Object -Last 5 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            }

            Write-Status "  Running build-boost.bat..." "DarkGray"
            $buildResult = cmd /c "cd /d `"$BootstrapDir`" && build-boost.bat < nul" 2>&1
            $buildExit = $LASTEXITCODE

            # Verify output
            $boostLibExists = (Test-Path $boostLibDir) -and
                ((Get-ChildItem $boostLibDir -Filter "libboost_system-*.lib" -ErrorAction SilentlyContinue | Measure-Object).Count -gt 0)

            if ($boostLibExists) {
                $boostLibCount = (Get-ChildItem $boostLibDir -Filter "*.lib" | Measure-Object).Count
                Write-Status "Boost: build successful ($boostLibCount .lib files)" "Green"
            } else {
                Write-Status "ERROR: Boost build failed - no output libraries found" "Red"
                Write-Status "  Check build-boost.bat output for errors" "Red"
                $buildResult | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            }
        }

        # --- Build 2/3: OpenSSL ---
        Write-Host ""
        Write-Host "[Build 2/3] OpenSSL $($Dependencies.OpenSSL.Version)" -ForegroundColor White

        $opensslLibDir = Join-Path $opensslDir "lib64"
        $opensslLibExists = (Test-Path (Join-Path $opensslLibDir "libeay32MT.lib"))

        if ($opensslLibExists) {
            Write-Status "OpenSSL: libraries already built, skipping" "DarkGray"
        } elseif (-not $script:PerlPath) {
            Write-Status "OpenSSL: SKIPPED - Perl not found (required for Configure)" "Yellow"
        } else {
            Write-Status "Building OpenSSL (this may take 5-15 minutes)..." "White"
            Write-Status "  Running build-openssl.bat..." "DarkGray"

            $buildResult = cmd /c "cd /d `"$BootstrapDir`" && build-openssl.bat < nul" 2>&1
            $buildExit = $LASTEXITCODE

            # Verify output
            $opensslLibExists = (Test-Path (Join-Path $opensslLibDir "libeay32MT.lib"))
            $opensslDbgExists = (Test-Path (Join-Path $opensslLibDir "libeay32MTd.lib"))

            if ($opensslLibExists) {
                $libs = @()
                if ($opensslLibExists) { $libs += "libeay32MT.lib" }
                if ($opensslDbgExists) { $libs += "libeay32MTd.lib" }
                Write-Status "OpenSSL: build successful ($($libs -join ', '))" "Green"
            } else {
                Write-Status "ERROR: OpenSSL build failed - no output libraries found" "Red"
                $buildResult | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
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
            Write-Status "  Running build-soci.bat..." "DarkGray"

            $buildResult = cmd /c "cd /d `"$BootstrapDir`" && build-soci.bat < nul" 2>&1
            $buildExit = $LASTEXITCODE

            # Verify output
            $sociLibExists = (Test-Path $sociDebugLib) -and (Test-Path $sociReleaseLib)

            if ($sociLibExists) {
                Write-Status "SOCI: build successful" "Green"
                Write-Status "  lib64/debug/libsoci_core_3_2.lib" "DarkGray"
                Write-Status "  lib64/debug/libsoci_postgresql_3_2.lib" "DarkGray"
                Write-Status "  lib64/release/libsoci_core_3_2.lib" "DarkGray"
                Write-Status "  lib64/release/libsoci_postgresql_3_2.lib" "DarkGray"
            } else {
                Write-Status "ERROR: SOCI build failed - no output libraries found" "Red"
                $buildResult | Select-Object -Last 10 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            }
        }
    }
}

# =============================================================================
# PHASE 5: Summary and verification
# =============================================================================

Write-Host ""
Write-Step "PHASE 5: SUMMARY"
$totalElapsed = (Get-Date) - $script:ScriptStartTime
Write-Host ""
Write-Host "Total time: $("{0:mm\:ss}" -f $totalElapsed)" -ForegroundColor White
Write-Host ""

# --- Verification checklist ---
Write-Host "Verification checklist:" -ForegroundColor White
Write-Host ""

$checks = @(
    @{ Name = "Boost headers";           Path = Join-Path $ExternalDir "boost\boost\config.hpp" }
    @{ Name = "Boost auto_link patched"; Path = Join-Path $ExternalDir "boost\boost\config\auto_link.hpp" }
    @{ Name = "Boost user-config.jam";   Path = Join-Path $ExternalDir "boost\user-config.jam" }
    @{ Name = "Boost built libs";        Path = Join-Path $ExternalDir "boost\lib64\lib" }
    @{ Name = "OpenSSL source";          Path = Join-Path $ExternalDir "openssl_src\Configure" }
    @{ Name = "OpenSSL e_os.h patched";  Path = Join-Path $ExternalDir "openssl_src\e_os.h" }
    @{ Name = "OpenSSL built libs";      Path = Join-Path $ExternalDir "openssl\lib64\libeay32MT.lib" }
    @{ Name = "PostgreSQL headers";      Path = Join-Path $ExternalDir "postgresql\include\libpq-fe.h" }
    @{ Name = "Python headers";          Path = Join-Path $ExternalDir "python\include\Python.h" }
    @{ Name = "SOCI source";             Path = Join-Path $ExternalDir "soci\src\core\soci.h" }
    @{ Name = "SOCI patched";            Path = Join-Path $ExternalDir "soci\src\core\soci-platform.h" }
    @{ Name = "SOCI built libs";         Path = Join-Path $ExternalDir "..\lib64\release\libsoci_core_3_2.lib" }
    @{ Name = "SDL headers";             Path = Join-Path $ExternalDir "SDL\include\SDL.h" }
    @{ Name = "Recast source";           Path = Join-Path $ExternalDir "recast\Recast\Include\Recast.h" }
)

$passCount = 0
$failCount = 0
foreach ($check in $checks) {
    $exists = Test-Path $check.Path
    if ($exists) {
        Write-Host "  [OK] $($check.Name)" -ForegroundColor Green
        $passCount++
    } else {
        Write-Host "  [--] $($check.Name)" -ForegroundColor DarkGray
        $failCount++
    }
}

Write-Host ""
Write-Host "  $passCount passed, $failCount not yet ready" -ForegroundColor White
Write-Host ""

Write-Host "Dependency layout:" -ForegroundColor White
Write-Host "  external/" -ForegroundColor Gray
Write-Host "    boost/          - Boost 1.55.0 source + built libs in lib64/lib/" -ForegroundColor Gray
Write-Host "    openssl_src/    - OpenSSL 1.0.1e patched source" -ForegroundColor Gray
Write-Host "    openssl/        - OpenSSL 1.0.1e headers + built libs in lib64/" -ForegroundColor Gray
Write-Host "    postgresql/     - PostgreSQL 9.2.24 headers + libpq.lib" -ForegroundColor Gray
Write-Host "    python/         - Python 3.4.1 headers + import libs" -ForegroundColor Gray
Write-Host "    soci/           - SOCI 3.2.1 patched source" -ForegroundColor Gray
Write-Host "    SDL/            - SDL 1.2.15 headers + libs" -ForegroundColor Gray
Write-Host "    recast/         - Recast/Detour source (built by Recast.vcxproj)" -ForegroundColor Gray
Write-Host "    _downloads/     - Cached archives (safe to delete)" -ForegroundColor Gray
Write-Host ""
Write-Host "  lib64/" -ForegroundColor Gray
Write-Host "    debug/          - SOCI debug libraries" -ForegroundColor Gray
Write-Host "    release/        - SOCI release libraries" -ForegroundColor Gray
Write-Host ""

if ($failCount -eq 0) {
    Write-Host "All dependencies ready. Open W-NG.sln in Visual Studio and build!" -ForegroundColor Green
} elseif ($SkipBuild) {
    Write-Host "Extract and patch complete. Re-run without -SkipBuild to build libraries." -ForegroundColor Yellow
} else {
    Write-Host "Some dependencies are not ready. Check the warnings above." -ForegroundColor Yellow
}

Write-Host ""
Write-Host "To clean up download cache:" -ForegroundColor DarkGray
Write-Host "  Remove-Item '$DownloadDir' -Recurse" -ForegroundColor DarkGray
Write-Host ""
