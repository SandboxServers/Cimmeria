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

    .PARAMETER UseDocker
        When set, only requires client libraries (libpq-dev, psql) on Linux
        rather than the full PostgreSQL server.

    .EXAMPLE
        Install-CimmeriaDependencies

    .EXAMPLE
        Install-CimmeriaDependencies -SkipDownload
    #>
    [CmdletBinding()]
    param(
        [switch]$SkipDownload,
        [switch]$UseDocker
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
    Write-Host "This step handles PostgreSQL and sidecar binaries."
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
        $targetMajor = ($Dependencies.PostgreSQL.Version -split '\.')[0]  # e.g. "17"

        # Check if existing binaries match the target version
        $needsExtract = $false
        $postgresExe = Join-Path $pgServerDir "bin\postgres.exe"
        if (-not (Test-Path (Join-Path $pgDir "include")) -or -not (Test-Path (Join-Path $pgServerDir "bin"))) {
            $needsExtract = $true
        } elseif (Test-Path $postgresExe) {
            $existingVersion = & $postgresExe --version 2>&1
            if ($existingVersion -match 'PostgreSQL\)\s+(\d+)') {
                $existingMajor = $Matches[1]
                if ($existingMajor -ne $targetMajor) {
                    Write-Status "PostgreSQL: existing version is $existingMajor but target is $targetMajor — upgrading..." "Yellow"

                    # Stop PostgreSQL if it's running (files will be locked otherwise)
                    $pgCtlExe = Join-Path $pgServerDir "bin\pg_ctl.exe"
                    $pgDataDir = Join-Path (Get-ProjectPaths).ServerDir "pgdata"
                    if ((Test-Path $pgCtlExe) -and (Test-Path $pgDataDir)) {
                        $statusResult = & $pgCtlExe status -D $pgDataDir 2>&1
                        if ($LASTEXITCODE -eq 0) {
                            Write-Status "PostgreSQL: stopping old server before upgrade..." "Yellow"
                            & $pgCtlExe stop -D $pgDataDir -m immediate 2>&1 | Out-Null
                            Start-Sleep -Seconds 2
                        }
                    }
                    # Also kill any stray postgres processes using the old binaries
                    Get-Process -Name "postgres" -ErrorAction SilentlyContinue | ForEach-Object {
                        if ($_.Path -and $_.Path.StartsWith($pgServerDir)) {
                            Write-Status "PostgreSQL: killing stray process (PID $($_.Id))..." "Yellow"
                            Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
                        }
                    }
                    Start-Sleep -Seconds 1

                    Remove-Item $pgServerDir -Recurse -Force
                    Remove-Item $pgDir -Recurse -Force -ErrorAction SilentlyContinue
                    $needsExtract = $true
                }
            }
        }

        if ($needsExtract) {
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
        # Linux: verify or install system PostgreSQL
        # When -UseDocker: only need client libs (libpq-dev) for Rust compilation + psql for schema loading
        # Without Docker: need full server (pg_ctl, initdb, postgres) for managed local instance
        $needServer = -not $UseDocker
        $stepLabel = if ($UseDocker) { "POSTGRESQL CLIENT LIBRARIES" } else { "POSTGRESQL (SYSTEM PACKAGE)" }
        Write-Step $stepLabel

        $pgConfig = Get-Command pg_config -ErrorAction SilentlyContinue
        if ($pgConfig) {
            $pgVersion = (pg_config --version 2>&1) -join ""
            Write-Status "Found: $pgVersion" "Green"
            if ($pgVersion -notmatch '1[7-9]\.|[2-9]\d\.') {
                Write-Status "WARNING: PostgreSQL 17+ recommended. Found: $pgVersion" "Yellow"
            }
        } else {
            # Also check common Debian/Ubuntu locations (pg_config may not be in PATH)
            foreach ($ver in @(17, 16, 15)) {
                $candidate = "/usr/lib/postgresql/$ver/bin/pg_config"
                if (Test-Path $candidate) {
                    $pgVersion = (& $candidate --version 2>&1) -join ""
                    Write-Status "Found: $pgVersion (not in PATH)" "Green"
                    Write-Status "  Hint: add /usr/lib/postgresql/$ver/bin to your PATH" "DarkGray"
                    $pgConfig = Get-Item $candidate
                    break
                }
            }
        }

        if (-not $pgConfig) {
            # Determine what packages to install
            $packages = if ($needServer) { "postgresql-17 libpq-dev" } else { "libpq-dev postgresql-client-17" }

            # Attempt to install on Debian/Ubuntu
            $aptGet = Get-Command apt-get -ErrorAction SilentlyContinue
            if ($aptGet) {
                Write-Status "PostgreSQL not found — attempting to install via apt..." "Yellow"

                # Test if sudo works (may need a password or not be available)
                $sudoTest = & sudo -n true 2>&1
                if ($LASTEXITCODE -ne 0) {
                    Write-Status "Cannot run sudo non-interactively." "Yellow"
                    Write-Host ""
                    Write-Host "  Install the required packages manually:" -ForegroundColor White
                    Write-Host "    sudo apt install $packages" -ForegroundColor Cyan
                    if ($needServer) {
                        Write-Host ""
                        Write-Host "  Or use Docker instead:" -ForegroundColor White
                        Write-Host "    pwsh setup.ps1 -UseDocker" -ForegroundColor Cyan
                    }
                    Write-Host ""
                    throw "PostgreSQL not found. Install it and re-run$(if ($needServer) { ', or use -UseDocker' })."
                }

                # Add the official PostgreSQL apt repository for PG 17
                $pgaptInstalled = Test-Path "/etc/apt/sources.list.d/pgdg.list"
                if (-not $pgaptInstalled) {
                    Write-Status "  Adding PostgreSQL apt repository..." "DarkGray"
                    & sudo apt-get install -y curl ca-certificates 2>&1 | Out-Null
                    & sudo install -d /usr/share/postgresql-common/pgdg
                    & sudo sh -c 'curl -fsSL https://www.postgresql.org/media/keys/ACCC4CF8.asc -o /usr/share/postgresql-common/pgdg/apt.postgresql.org.asc' 2>&1
                    $codename = (lsb_release -cs 2>&1).Trim()
                    & sudo sh -c "echo 'deb [signed-by=/usr/share/postgresql-common/pgdg/apt.postgresql.org.asc] https://apt.postgresql.org/pub/repos/apt $codename-pgdg main' > /etc/apt/sources.list.d/pgdg.list" 2>&1
                    & sudo apt-get update 2>&1 | Out-Null
                }
                & sudo apt-get install -y $packages.Split(' ') 2>&1 | ForEach-Object {
                    $line = "$_"
                    if ($line -match 'Setting up|is already|Unpacking') {
                        Write-Status "  $line" "DarkGray"
                    }
                }

                # Verify installation
                $pgConfig = Get-Command pg_config -ErrorAction SilentlyContinue
                if (-not $pgConfig) {
                    foreach ($ver in @(17, 16, 15)) {
                        $candidate = "/usr/lib/postgresql/$ver/bin/pg_config"
                        if (Test-Path $candidate) {
                            $pgConfig = Get-Item $candidate
                            break
                        }
                    }
                }

                if ($pgConfig) {
                    $pgVersion = (& $pgConfig --version 2>&1) -join ""
                    Write-Status "PostgreSQL installed: $pgVersion" "Green"

                    if ($needServer) {
                        # Stop the system-managed cluster — we manage our own in server/pgdata/
                        Write-Status "Stopping system PostgreSQL cluster (bootstrap manages its own)..." "DarkGray"
                        & sudo pg_ctlcluster 17 main stop 2>&1 | Out-Null
                        & sudo systemctl disable postgresql 2>&1 | Out-Null
                    }
                } else {
                    throw "PostgreSQL installation failed. Install manually: sudo apt install $packages"
                }
            } else {
                Write-Status "PostgreSQL not found." "Red"
                Write-Status "  Debian/Ubuntu: sudo apt install $packages" "Yellow"
                Write-Status "  Fedora/RHEL:   sudo dnf install postgresql17-server libpq-devel" "Yellow"
                Write-Status "  Or use Docker: pwsh setup.ps1 -UseDocker" "Yellow"
                throw "PostgreSQL not found. Install it and re-run, or use -UseDocker."
            }
        }

        # Ensure libpq-dev is installed (required for Rust pq-sys crate compilation)
        $dpkg = Get-Command dpkg -ErrorAction SilentlyContinue
        if ($dpkg) {
            $libpqCheck = & dpkg -s libpq-dev 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Status "libpq-dev not found (required for Rust PostgreSQL driver)." "Yellow"
                $sudoTest = & sudo -n true 2>&1
                if ($LASTEXITCODE -eq 0) {
                    Write-Status "Installing libpq-dev..." "Yellow"
                    & sudo apt-get install -y libpq-dev 2>&1 | ForEach-Object {
                        $line = "$_"
                        if ($line -match 'Setting up|is already') { Write-Status "  $line" "DarkGray" }
                    }
                } else {
                    Write-Status "  Install manually: sudo apt install libpq-dev" "Yellow"
                }
            }
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

    # ── 7za.exe sidecar for sgw-launcher (Windows only) ────────────────────
    if ($isWin) {
        Write-Step "7-ZIP STANDALONE (SGW-LAUNCHER SIDECAR)"

        $launcherBinDir = Join-Path $paths.ProjectRoot "crates\launcher\binaries"
        $targetTriple = "x86_64-pc-windows-msvc"
        $sidecarPath = Join-Path $launcherBinDir "7za-$targetTriple.exe"
        $gnuSidecarPath = Join-Path $launcherBinDir "7za-x86_64-pc-windows-gnu.exe"

        if ((Test-Path $sidecarPath) -and (Get-Item $sidecarPath).Length -gt 0) {
            Write-Status "7za sidecar: already present" "DarkGray"
        } else {
            if (-not $SkipDownload) {
                # Download 7zr.exe (bootstrap extractor) and the extra archive
                $sevenZrPath = Join-Path $DownloadDir "7zr.exe"
                $extraArchive = Join-Path $DownloadDir $Dependencies.SevenZip.ExtraFileName

                Write-Status "Downloading 7zr.exe (bootstrap extractor)..." "White"
                Get-DownloadFile $Dependencies.SevenZip.BootstrapUrl $sevenZrPath

                Write-Status "Downloading 7-Zip Extra $($Dependencies.SevenZip.Version)..." "White"
                Get-DownloadFile $Dependencies.SevenZip.ExtraUrl $extraArchive
            }

            $sevenZrPath = Join-Path $DownloadDir "7zr.exe"
            $extraArchive = Join-Path $DownloadDir $Dependencies.SevenZip.ExtraFileName
            $extractDir = Join-Path $DownloadDir "7z-extra"

            Write-Status "Extracting 7za.exe from archive..." "White"
            if (Test-Path $extractDir) { Remove-Item $extractDir -Recurse -Force }
            New-Item -ItemType Directory -Path $extractDir -Force | Out-Null
            & $sevenZrPath x $extraArchive "-o$extractDir" -y 2>&1 | Out-Null

            $extracted7za = Join-Path $extractDir "x64\7za.exe"
            if (-not (Test-Path $extracted7za)) {
                # Some versions put it in the root
                $extracted7za = Join-Path $extractDir "7za.exe"
            }

            if (Test-Path $extracted7za) {
                Copy-Item $extracted7za $sidecarPath -Force
                Copy-Item $extracted7za $gnuSidecarPath -Force
                $size = Format-FileSize (Get-Item $sidecarPath).Length
                Write-Status "7za sidecar: installed ($size)" "Green"
            } else {
                Write-Status "WARNING: Could not find 7za.exe in extracted archive" "Yellow"
                Write-Status "  sgw-launcher build will fail — place 7za.exe manually in:" "Yellow"
                Write-Status "  $launcherBinDir" "Yellow"
            }

            Remove-Item $extractDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }

    # ── Recast Navigation (Detour navmesh library, all platforms) ──────────
    Write-Step "RECAST NAVIGATION (DETOUR NAVMESH LIBRARY)"

    $recastDir = Join-Path $ExternalDir "recast"
    $recastDetourHeader = Join-Path $recastDir "Detour" "Include" "DetourNavMesh.h"

    if (Test-Path $recastDetourHeader) {
        Write-Status "Recast Navigation: already present" "DarkGray"
    } else {
        if (-not $SkipDownload) {
            Write-Status "Downloading Recast Navigation $($Dependencies.Recast.Version)..." "White"
            Get-DownloadFile $Dependencies.Recast.Url (Join-Path $DownloadDir $Dependencies.Recast.FileName)
        }

        $recastArchive = Join-Path $DownloadDir $Dependencies.Recast.FileName
        if (Test-Path $recastArchive) {
            Write-Status "Recast Navigation: extracting source..." "White"
            $recastTempDir = Join-Path $ExternalDir "recast_temp"
            Expand-DependencyArchive $recastArchive $recastTempDir

            # The zip extracts to recastnavigation-1.6.0/
            $recastRoot = Get-ChildItem $recastTempDir -Directory | Select-Object -First 1
            if (Test-Path $recastDir) { Remove-Item $recastDir -Recurse -Force }
            Move-Item $recastRoot.FullName $recastDir

            Remove-Item $recastTempDir -Recurse -Force -ErrorAction SilentlyContinue
            Write-Status "Recast Navigation: done (v$($Dependencies.Recast.Version), DT_NAVMESH_VERSION=7)" "Green"
        } else {
            Write-Status "Recast Navigation: archive not found. Download manually:" "Yellow"
            Write-Status "  $($Dependencies.Recast.Url)" "Yellow"
            Write-Status "  Extract to: $recastDir" "Yellow"
        }
    }

    # ── SigNoz observability stack (Docker compose source) ────────────
    # Cloned (not downloaded as a tarball) so `git submodule`-style
    # version pinning is one `git checkout <tag>` away. The single-
    # entry docker compose at docker/compose.yml `include:`s the
    # upstream compose from this path, so the bring-up fails fast
    # with a clear file-not-found error if this step never ran.
    Write-Step "SIGNOZ OBSERVABILITY STACK (DOCKER COMPOSE SOURCE)"

    $signozDir = Join-Path $ExternalDir "signoz"
    $signozCompose = Join-Path $signozDir "deploy" "docker" "clickhouse-setup" "docker-compose.yaml"
    # Pinned tag — bump deliberately, in a commit, with a re-test of
    # the OTLP path. Floating to `main` would make every fresh setup
    # a roll-of-the-dice on whether SigNoz upstream broke our wiring.
    $signozTag = "v0.55.0"

    if (Test-Path $signozCompose) {
        Write-Status "SigNoz: already present at external/signoz" "DarkGray"
    } else {
        $git = Get-Command git -ErrorAction SilentlyContinue
        if (-not $git) {
            Write-Status "SigNoz: git not found in PATH — skipping clone." "Yellow"
            Write-Status "  Install git and re-run setup, or clone manually:" "Yellow"
            Write-Status "    git clone --depth=1 --branch $signozTag https://github.com/SigNoz/signoz $signozDir" "Yellow"
        } else {
            Write-Status "SigNoz: cloning $signozTag into external/signoz..." "White"
            # Remove any stale partial clone so the retry on failure is
            # always from a known state.
            if (Test-Path $signozDir) {
                Remove-Item $signozDir -Recurse -Force -ErrorAction SilentlyContinue
            }
            & git clone --depth=1 --branch $signozTag `
                https://github.com/SigNoz/signoz `
                $signozDir 2>&1 | Out-Host
            if ($LASTEXITCODE -eq 0 -and (Test-Path $signozCompose)) {
                Write-Status "SigNoz: done (tag $signozTag)" "Green"
            } else {
                Write-Status "SigNoz: clone failed (exit $LASTEXITCODE). Verify network access to github.com and re-run." "Yellow"
            }
        }
    }

    Write-Status "Install-CimmeriaDependencies complete." "Green"
}
