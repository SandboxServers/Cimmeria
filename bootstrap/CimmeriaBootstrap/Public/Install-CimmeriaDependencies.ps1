function Install-CimmeriaDependencies {
    <#
    .SYNOPSIS
        Downloads, extracts, and patches all external dependencies for the Cimmeria project.

    .DESCRIPTION
        Performs Phases 0-3 of the bootstrap pipeline:
        - Phase 0:   Detect Visual Studio and Perl
        - Phase 1:   Download all dependency archives
        - Phase 2:   Extract and arrange into external/
        - Phase 3:   Apply VS2015+ compatibility patches

        Each step is idempotent - safe to re-run at any time.

    .PARAMETER SkipDownload
        Skip downloading archives (use already-cached files in external/_downloads/).

    .PARAMETER InstallVS
        If Visual Studio is not found, download and install VS Community with the C++ Desktop workload.

    .PARAMETER BuildArch
        Architecture to set up: "x64" (default), "x86", or "both".

    .EXAMPLE
        Install-CimmeriaDependencies
        # Full download, extract, and patch

    .EXAMPLE
        Install-CimmeriaDependencies -SkipDownload
        # Re-apply patches only (archives already cached)
    #>
    [CmdletBinding()]
    param(
        [switch]$SkipDownload,
        [switch]$InstallVS,
        [ValidateSet("x64", "x86", "both")]
        [string]$BuildArch = "x64"
    )

    $paths = Get-ProjectPaths
    $ExternalDir = $paths.ExternalDir
    $DownloadDir = $paths.DownloadDir
    $BootstrapDir = $paths.BootstrapDir
    $PatchDir = $paths.PatchDir
    $TemplateDir = $paths.TemplateDir

    $Dependencies = Import-PowerShellDataFile (Join-Path $PSScriptRoot "..\Data\Dependencies.psd1")

    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host " Cimmeria Dependency Setup" -ForegroundColor Yellow
    Write-Host " Project: Stargate Worlds Emulator" -ForegroundColor Yellow
    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host ""
    Write-Host "This will download, patch, and arrange all external dependencies."
    Write-Host "Total download size: ~200 MB (vs 724 MB vendored)"
    Write-Host "Target directory: $ExternalDir"
    Write-Host ""

    New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
    New-Item -ItemType Directory -Path $ExternalDir -Force | Out-Null

    # =========================================================================
    # PHASE 0: Visual Studio + Perl Detection
    # =========================================================================

    Write-Step "PHASE 0: VISUAL STUDIO DETECTION"
    $script:VSInfo = Find-VisualStudio -InstallVS:$InstallVS

    if (-not $script:VSInfo -and ($IsWindows -or (-not (Test-Path variable:IsWindows)))) {
        throw "Visual Studio with C++ tools is required. Re-run with -InstallVS to auto-install."
    }

    Write-Step "PHASE 0.5: PERL DETECTION"
    $script:PerlPath = Find-Perl

    # =========================================================================
    # PHASE 1: Download all archives
    # =========================================================================

    if (-not $SkipDownload) {
        Write-Step "PHASE 1: DOWNLOADING DEPENDENCIES"

        Write-Host "`n[1/7] Boost $($Dependencies.Boost.Version)" -ForegroundColor White
        Get-DownloadFile $Dependencies.Boost.Url (Join-Path $DownloadDir $Dependencies.Boost.FileName)

        Write-Host "`n[2/7] OpenSSL $($Dependencies.OpenSSL.Version)" -ForegroundColor White
        Get-DownloadFile $Dependencies.OpenSSL.Url (Join-Path $DownloadDir $Dependencies.OpenSSL.FileName)

        Write-Host "`n[3/7] PostgreSQL $($Dependencies.PostgreSQL.Version)" -ForegroundColor White
        Get-DownloadFile $Dependencies.PostgreSQL.Url (Join-Path $DownloadDir $Dependencies.PostgreSQL.FileName)

        Write-Host "`n[4/7] Python $($Dependencies.Python.Version)" -ForegroundColor White
        if ($BuildArch -eq "x86" -or $BuildArch -eq "both") {
            Get-DownloadFile $Dependencies.Python.Url32 (Join-Path $DownloadDir $Dependencies.Python.FileName32)
        }
        if ($BuildArch -eq "x64" -or $BuildArch -eq "both") {
            Get-DownloadFile $Dependencies.Python.Url64 (Join-Path $DownloadDir $Dependencies.Python.FileName64)
        }

        Write-Host "`n[5/7] SOCI $($Dependencies.SOCI.Version)" -ForegroundColor White
        Get-DownloadFile $Dependencies.SOCI.Url (Join-Path $DownloadDir $Dependencies.SOCI.FileName)

        Write-Host "`n[6/7] SDL $($Dependencies.SDL.Version)" -ForegroundColor White
        Get-DownloadFile $Dependencies.SDL.Url (Join-Path $DownloadDir $Dependencies.SDL.FileName)

        Write-Host "`n[7/7] Recast/Detour" -ForegroundColor White
        Get-DownloadFile $Dependencies.Recast.Url (Join-Path $DownloadDir $Dependencies.Recast.FileName)
    }

    # =========================================================================
    # PHASE 2: Extract and arrange
    # =========================================================================

    Write-Step "PHASE 2: EXTRACTING AND ARRANGING"

    # --- Boost ---
    $boostDir = Join-Path $ExternalDir "boost"
    if (-not (Test-Path (Join-Path $boostDir "boost"))) {
        Write-Status "Boost: extracting..." "White"
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.Boost.FileName) $ExternalDir
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
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.SOCI.FileName) $ExternalDir
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
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.SDL.FileName) $ExternalDir
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
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.Recast.FileName) $ExternalDir
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

    # --- PostgreSQL (headers + lib for compilation, full server for runtime) ---
    $pgDir = Join-Path $ExternalDir "postgresql"
    $pgServerDir = Join-Path $ExternalDir "postgresql_server"
    if (-not (Test-Path (Join-Path $pgDir "include"))) {
        Write-Status "PostgreSQL: extracting binary distribution..." "White"
        $pgTempDir = Join-Path $ExternalDir "postgresql_temp"
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.PostgreSQL.FileName) $pgTempDir

        Write-Status "PostgreSQL: copying headers and libs for compilation..." "DarkGray"
        New-Item -ItemType Directory -Path $pgDir -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $pgDir "include") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $pgDir "lib64") -Force | Out-Null

        $pgRoot = Get-ChildItem $pgTempDir -Directory | Select-Object -First 1
        $pgBinRoot = Join-Path $pgRoot.FullName "pgsql"
        if (-not (Test-Path $pgBinRoot)) { $pgBinRoot = $pgRoot.FullName }

        Copy-Item -Path (Join-Path $pgBinRoot "include" "*") -Destination (Join-Path $pgDir "include") -Recurse -Force
        Copy-Item -Path (Join-Path $pgBinRoot "lib" "libpq.lib") -Destination (Join-Path $pgDir "lib64" "libpq.lib") -Force -ErrorAction SilentlyContinue

        # Preserve full server distribution for runtime use
        Write-Status "PostgreSQL: preserving server distribution for runtime..." "DarkGray"
        if (Test-Path $pgServerDir) { Remove-Item $pgServerDir -Recurse -Force }
        if (Test-Path (Join-Path $pgBinRoot "bin\postgres.exe")) {
            Move-Item $pgBinRoot $pgServerDir
        } else {
            # The pgsql directory itself is the root
            $pgRootDir = $pgRoot.FullName
            Move-Item $pgRootDir $pgServerDir
        }

        Write-Status "PostgreSQL: cleaning temp files..." "DarkGray"
        Remove-Item $pgTempDir -Recurse -Force -ErrorAction SilentlyContinue
        Write-Status "PostgreSQL: done (compilation headers + runtime server)" "Green"
    } else {
        Write-Status "PostgreSQL: already extracted" "DarkGray"
        # Check if server distribution exists; if not, note it
        if (-not (Test-Path (Join-Path $pgServerDir "bin"))) {
            Write-Status "PostgreSQL: WARNING - server distribution missing. Delete external/postgresql/ and re-run to extract." "Yellow"
        }
    }

    # --- Python (headers + libs + python34.dll from MSI) ---
    $pythonDir = Join-Path $ExternalDir "python"
    if (-not (Test-Path (Join-Path $pythonDir "include"))) {
        Write-Status "Python: extracting development files from MSI..." "White"

        New-Item -ItemType Directory -Path $pythonDir -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $pythonDir "include") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $pythonDir "lib") -Force | Out-Null
        New-Item -ItemType Directory -Path (Join-Path $pythonDir "lib64") -Force | Out-Null

        try {
            if ($BuildArch -eq "x64" -or $BuildArch -eq "both") {
                $msiPath = Join-Path $DownloadDir $Dependencies.Python.FileName64
                $pyTempDir = Join-Path $ExternalDir "python_temp64"
                New-Item -ItemType Directory -Path $pyTempDir -Force | Out-Null
                Expand-DependencyArchive $msiPath $pyTempDir

                Write-Status "Python: searching for headers, libs, and DLL..." "DarkGray"
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

                # Extract python34.dll for runtime
                $pythonDll = Get-ChildItem $pyTempDir -Recurse -Filter "python34.dll" | Select-Object -First 1
                if ($pythonDll) {
                    Copy-Item -Path $pythonDll.FullName -Destination (Join-Path $pythonDir "python34.dll") -Force
                    Write-Status "Python: extracted python34.dll for runtime" "DarkGray"
                } else {
                    Write-Status "Python: WARNING - python34.dll not found in MSI extract" "Yellow"
                }

                # Extract standard library (needed for embedded Python to find encodings, etc.)
                $libDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "Lib" | Select-Object -First 1
                if ($libDir) {
                    $destLib = Join-Path $pythonDir "Lib"
                    if (Test-Path $destLib) { Remove-Item $destLib -Recurse -Force }
                    Copy-Item -Path $libDir.FullName -Destination $destLib -Recurse -Force
                    Write-Status "Python: extracted standard library (Lib/)" "DarkGray"
                } else {
                    Write-Status "Python: WARNING - Lib/ directory not found in MSI extract" "Yellow"
                }

                # Extract compiled extension modules (_socket.pyd, _ssl.pyd, etc.)
                $dllsDir = Get-ChildItem $pyTempDir -Recurse -Directory -Filter "DLLs" | Select-Object -First 1
                if ($dllsDir) {
                    $destDlls = Join-Path $pythonDir "DLLs"
                    if (Test-Path $destDlls) { Remove-Item $destDlls -Recurse -Force }
                    Copy-Item -Path $dllsDir.FullName -Destination $destDlls -Recurse -Force
                    Write-Status "Python: extracted extension modules (DLLs/)" "DarkGray"
                } else {
                    Write-Status "Python: WARNING - DLLs/ directory not found in MSI extract" "Yellow"
                }

                Write-Status "Python: cleaning temp files..." "DarkGray"
                Remove-Item $pyTempDir -Recurse -Force -ErrorAction SilentlyContinue
            }

            if ($BuildArch -eq "x86" -or $BuildArch -eq "both") {
                $msiPath = Join-Path $DownloadDir $Dependencies.Python.FileName32
                $pyTempDir = Join-Path $ExternalDir "python_temp32"
                New-Item -ItemType Directory -Path $pyTempDir -Force | Out-Null
                Expand-DependencyArchive $msiPath $pyTempDir

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
            Write-Status "  Continuing with remaining dependencies..." "Yellow"
        }
    } else {
        Write-Status "Python: already extracted" "DarkGray"
    }

    # --- OpenSSL (extract source, will be patched and built later) ---
    # NOTE: OpenSSL's source tree contains a file named 'NUL' which is a reserved
    # device name on Windows. We must delete it immediately after extraction and
    # use cmd /c rd for directory removal since PowerShell can't handle NUL files.
    $opensslSrcDir = Join-Path $ExternalDir "openssl_src"
    if (-not (Test-Path (Join-Path $opensslSrcDir "Configure"))) {
        Write-Status "OpenSSL: extracting source..." "White"
        if (Test-Path $opensslSrcDir) { cmd /c "rd /s /q `"$opensslSrcDir`"" 2>$null }
        Expand-DependencyArchive (Join-Path $DownloadDir $Dependencies.OpenSSL.FileName) $ExternalDir
        $extracted = Join-Path $ExternalDir "openssl-OpenSSL_1_0_1e"
        if (Test-Path $extracted) {
            # Delete reserved 'NUL' filename before any move/rename/delete operations
            cmd /c "del /f /q `"$extracted\NUL`"" 2>$null
            Write-Status "OpenSSL: renaming to openssl_src/" "DarkGray"
            Rename-Item $extracted $opensslSrcDir
        }
        Write-Status "OpenSSL: done" "Green"
    } else {
        Write-Status "OpenSSL: already extracted" "DarkGray"
    }

    # =========================================================================
    # PHASE 3: Apply VS2015+ compatibility patches
    # =========================================================================

    Write-Step "PHASE 3: APPLYING COMPATIBILITY PATCHES"

    $patchMap = @{
        "boost_auto_link.hpp"              = "boost\boost\config\auto_link.hpp"
        "soci_platform.h"                  = "soci\src\core\soci-platform.h"
        "soci_backends_config.h"           = "soci\src\core\soci_backends_config.h"
        "soci_postgresql_statement.cpp"    = "soci\src\backends\postgresql\statement.cpp"
        "openssl_e_os.h"                   = "openssl_src\e_os.h"
        "openssl_e_padlock.c"              = "openssl_src\engines\e_padlock.c"
    }

    $patchDescriptions = @{
        "boost_auto_link.hpp"              = "Add vc140/vc145 toolset entries for VS2015+ auto-linking"
        "soci_platform.h"                  = "Guard snprintf/strtoll macros for VS2013+ native CRT"
        "soci_backends_config.h"           = "Generate CMake config header for static-link build"
        "soci_postgresql_statement.cpp"    = "Treat unknown PG types (custom enums) as strings instead of throwing"
        "openssl_e_os.h"                   = "Skip stdin/stdout/stderr redefinition on VS2015+ UCRT"
        "openssl_e_padlock.c"              = "Guard x86 inline asm with _M_IX86 for x64 builds"
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

    if ($script:VSInfo -and $script:VSInfo.ClExePath -and (Test-Path $templatePath)) {
        $toolsetVer = $script:VSInfo.VCToolsVersion
        if ($toolsetVer -match '^(\d+)\.(\d+)') {
            $major = $Matches[1]
            $minor = $Matches[2]
            $minorTrimmed = $minor.TrimEnd('0')
            if ($minorTrimmed -eq '') { $minorTrimmed = '0' }
            $b2Toolset = "$major.$minorTrimmed"
        } else {
            $b2Toolset = "14.5"
            Write-Status "WARNING: Could not parse VCToolsVersion, defaulting to 14.5" "Yellow"
        }

        $clPathEscaped = $script:VSInfo.ClExePath -replace '\\', '\\\\'
        $pythonInclude = (Join-Path $ExternalDir "python\include") -replace '\\', '\\\\'
        $pythonLib = (Join-Path $ExternalDir "python\lib64") -replace '\\', '\\\\'

        $template = Get-Content $templatePath -Raw
        $generated = $template -replace '\{\{MSVC_TOOLSET_VERSION\}\}', $b2Toolset
        $generated = $generated -replace '\{\{MSVC_CL_PATH\}\}', $clPathEscaped
        $generated = $generated -replace '\{\{PYTHON_INCLUDE_PATH\}\}', $pythonInclude
        $generated = $generated -replace '\{\{PYTHON_LIB_PATH\}\}', $pythonLib

        Set-Content -Path $userConfigPath -Value $generated -NoNewline
        Write-Status "Generated: $userConfigPath" "Green"
        Write-Status "  MSVC toolset: msvc-$b2Toolset" "DarkGray"
        Write-Status "  cl.exe: $($script:VSInfo.ClExePath)" "DarkGray"
    } elseif (Test-Path $userConfigPath) {
        Write-Status "user-config.jam already exists, keeping current version" "DarkGray"
    } else {
        Write-Status "WARNING: Cannot generate user-config.jam (no cl.exe found or template missing)" "Yellow"
    }

    Write-Status "Install-CimmeriaDependencies complete." "Green"
}
