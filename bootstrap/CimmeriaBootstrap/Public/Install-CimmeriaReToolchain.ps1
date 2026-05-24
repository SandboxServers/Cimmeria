function Install-CimmeriaReToolchain {
    <#
    .SYNOPSIS
        Downloads, installs, and wires the Cimmeria reverse-engineering toolchain:
        Ghidra 12, the GhidraMCP plugin, x64dbg, the x64dbg-automate plugin, and
        Python venvs for both MCP bridges. Generates .mcp.json from the template.

    .DESCRIPTION
        Opt-in bootstrap step gated by `-WithReToolchain` on setup.ps1. Idempotent —
        re-runs detect existing installs and skip work that's already done.

        Layout produced (all paths under the Cimmeria repo root, all gitignored):

          ghidra/ghidra_12.0.4_PUBLIC/        Ghidra install
          external/ghidra-mcp/                bethington/ghidra-mcp clone + bridge
          dbg/release/                        x64dbg snapshot + x96dbg.exe launcher
          dbg/release/x32/plugins/            x64dbg-automate.dp32 + libzmq-mt-4_3_5.dll
          dbg/release/x64/plugins/            x64dbg-automate.dp64 + libzmq-mt-4_3_5.dll
          .venvs/ghidra-mcp/                  Python venv for the Ghidra MCP bridge
          .venvs/x64dbg-mcp/                  Python venv with x64dbg-automate PyPI pkg
          .mcp.json                           generated from .mcp.json.example
                                              (never overwrites an existing file)

        The Ghidra plugin install lands in the user-profile dir at
        %APPDATA%\ghidra\ghidra_12.0.4_PUBLIC\Extensions\GhidraMCP\ — that path is
        outside the repo because Ghidra discovers extensions there.

        Prerequisites checked at the top: JDK 21+, Python 3.11+. Both must be on
        PATH. The function prints install hints and aborts if either is missing
        (use -SkipPrereqCheck to bypass during partial re-runs).

    .PARAMETER SkipDownload
        Skip downloading archives. Use already-cached files in external/_downloads/.

    .PARAMETER SkipPrereqCheck
        Skip the JDK / Python prerequisite check. Useful for partial re-runs when
        you know your shell can't see the right Java/Python but the tools are fine.

    .PARAMETER Force
        Re-run every step even if the install looks complete. Does NOT overwrite
        an existing .mcp.json — that's always protected.

    .PARAMETER CimmeriaRagKey
        Optional. If supplied, written into the cimmeria-rag block of the
        generated .mcp.json as the x-functions-key value (the Azure Functions
        key issued for the cimmeria-rag MCP server — ask in the project
        Discord / Slack if you don't have one). Otherwise the
        REPLACE_WITH_FUNCTIONS_KEY placeholder is left in place for you to
        substitute manually.

    .EXAMPLE
        Install-CimmeriaReToolchain

        Full install with default versions and download.

    .EXAMPLE
        Install-CimmeriaReToolchain -SkipDownload

        Re-run skipping network steps (useful after a partial failure).

    .EXAMPLE
        pwsh setup.ps1 -WithReToolchain -NoLaunch

        Recommended entry point — runs Install-CimmeriaReToolchain as part of the
        main bootstrap pipeline without launching the game server afterwards.
    #>
    [CmdletBinding(SupportsShouldProcess = $true)]
    param(
        [switch]$SkipDownload,
        [switch]$SkipPrereqCheck,
        [switch]$Force,
        [Alias("GhidraServerKey")]
        [string]$CimmeriaRagKey
    )

    $ErrorActionPreference = "Stop"

    $paths = Get-ProjectPaths
    $ProjectRoot = $paths.ProjectRoot
    $DownloadDir = $paths.DownloadDir
    $ExternalDir = $paths.ExternalDir
    $Dependencies = Import-PowerShellDataFile (Join-Path $PSScriptRoot "..\Data\Dependencies.psd1")

    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    if (-not $isWin) {
        Write-Status "Install-CimmeriaReToolchain currently supports Windows only." "Yellow"
        Write-Status "  The MCP toolchain depends on Windows-specific tooling (x64dbg,"  "DarkGray"
        Write-Status "  %APPDATA%\ghidra\ extensions dir). Cross-platform is future work." "DarkGray"
        return
    }

    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host " Cimmeria RE Toolchain Setup" -ForegroundColor Yellow
    Write-Host " Ghidra $($Dependencies.Ghidra.Version) + GhidraMCP $($Dependencies.GhidraMcp.Version)" -ForegroundColor Yellow
    Write-Host " x64dbg snapshot (manual-fetch) + x64dbg-automate $($Dependencies.X64DbgAutomate.Version)" -ForegroundColor Yellow
    Write-Host "=============================================" -ForegroundColor Yellow
    Write-Host ""

    New-Item -ItemType Directory -Path $DownloadDir -Force | Out-Null
    New-Item -ItemType Directory -Path $ExternalDir -Force | Out-Null

    # `python` resolved once in the prereq check and reused for all venv creation
    # below — avoids the trap where `Get-Command python` succeeds via the `py`
    # launcher in prereqs but later `python -m venv` calls fail because `python`
    # is not directly on PATH.
    $script:ResolvedPython = $null

    # ---------------------------------------------------------------------- #
    # Step 1: Prerequisites — Git, JDK 21+, Python 3.11+
    #   When missing on Windows we attempt `winget install` first; if winget is
    #   unavailable or the install fails, we print a manual download URL and
    #   abort. PATH is refreshed after each install so the freshly-installed
    #   binary is reachable without restarting the shell.
    # ---------------------------------------------------------------------- #
    if (-not $SkipPrereqCheck) {
        Write-Step "RE PREREQUISITES (Git, JDK 21+, Python 3.11+)"

        $wingetCmd = Get-Command winget -ErrorAction SilentlyContinue

        # Helper: refresh PATH from machine + user env so freshly-installed
        # tools become reachable without a shell restart.
        function Update-CimmeriaPath {
            $machine = [System.Environment]::GetEnvironmentVariable("Path", "Machine")
            $user    = [System.Environment]::GetEnvironmentVariable("Path", "User")
            $env:PATH = ($machine, $user, $env:PATH) -join ";"
        }

        # Helper: best-effort winget install with a clear fallback message.
        function Install-ViaWinget {
            param(
                [string]$WingetId,
                [string]$DisplayName,
                [string]$ManualUrl
            )
            if (-not $wingetCmd) {
                Write-Status "  winget not available — install $DisplayName manually:" "Yellow"
                Write-Status "    $ManualUrl" "Cyan"
                return $false
            }
            Write-Status "  Installing $DisplayName via winget ($WingetId)..." "White"
            $wingetArgs = @(
                "install", "--id", $WingetId,
                "--silent",
                "--accept-source-agreements",
                "--accept-package-agreements",
                "--scope", "user"
            )
            & winget @wingetArgs 2>&1 | ForEach-Object {
                if ($_ -match 'Found|Successfully|Installing|Already installed|No applicable upgrade') {
                    Write-Status "    $_" "DarkGray"
                }
            }
            $wingetExit = $LASTEXITCODE
            Update-CimmeriaPath
            if ($wingetExit -ne 0) {
                Write-Status "  winget install returned exit code $wingetExit." "Yellow"
                Write-Status "  Manual download: $ManualUrl" "Cyan"
                return $false
            }
            return $true
        }

        # Git — required to clone the ghidra-mcp bridge.
        $gitCmd = Get-Command git -ErrorAction SilentlyContinue
        if (-not $gitCmd) {
            Write-Status "Git not found on PATH." "Yellow"
            $installed = Install-ViaWinget -WingetId "Git.Git" -DisplayName "Git" -ManualUrl "https://git-scm.com/download/win"
            if ($installed) { $gitCmd = Get-Command git -ErrorAction SilentlyContinue }
            if (-not $gitCmd) {
                throw "Git required to clone ghidra-mcp. Install and re-run, or pass -SkipPrereqCheck."
            }
        }
        Write-Status "Git: $((& git --version 2>&1) -join '')" "Green"

        # JDK 21+ — required by Ghidra 12.
        $javaCmd = Get-Command java -ErrorAction SilentlyContinue
        $javaMajor = $null
        if ($javaCmd) {
            $javaVersionOutput = (& java -version 2>&1) -join "`n"
            if ($javaVersionOutput -match '"(\d+)\.\d') { $javaMajor = [int]$Matches[1] }
        }
        if (-not $javaCmd -or ($javaMajor -ne $null -and $javaMajor -lt 21)) {
            if (-not $javaCmd) {
                Write-Status "Java not found on PATH." "Yellow"
            } else {
                Write-Status "Java $javaMajor detected — Ghidra 12 requires 21+." "Yellow"
            }
            $installed = Install-ViaWinget -WingetId "EclipseAdoptium.Temurin.21.JDK" -DisplayName "Eclipse Temurin 21 JDK" -ManualUrl "https://adoptium.net/temurin/releases/?version=21"
            if ($installed) {
                $javaCmd = Get-Command java -ErrorAction SilentlyContinue
                if ($javaCmd) {
                    $javaVersionOutput = (& java -version 2>&1) -join "`n"
                    if ($javaVersionOutput -match '"(\d+)\.\d') { $javaMajor = [int]$Matches[1] }
                }
            }
            if (-not $javaCmd -or ($javaMajor -ne $null -and $javaMajor -lt 21)) {
                throw "JDK 21+ required for Ghidra. Install Eclipse Temurin 21 and re-run."
            }
        }
        Write-Status "Java: $((& java -version 2>&1) -split "`n" | Select-Object -First 1)" "Green"

        # Python 3.11+ — required by both MCP bridges.
        $pythonCmd = Get-Command python -ErrorAction SilentlyContinue
        if (-not $pythonCmd) { $pythonCmd = Get-Command py -ErrorAction SilentlyContinue }
        $pythonOk = $false
        if ($pythonCmd) {
            $pyVerOutput = (& $pythonCmd.Source --version 2>&1) -join ""
            if ($pyVerOutput -match 'Python\s+(\d+)\.(\d+)') {
                $pyMajor = [int]$Matches[1]; $pyMinor = [int]$Matches[2]
                if ($pyMajor -gt 3 -or ($pyMajor -eq 3 -and $pyMinor -ge 11)) {
                    $pythonOk = $true
                } else {
                    Write-Status "Python $pyMajor.$pyMinor detected — MCP bridges require 3.11+." "Yellow"
                }
            }
        }
        if (-not $pythonOk) {
            if (-not $pythonCmd) { Write-Status "Python not found on PATH." "Yellow" }
            $installed = Install-ViaWinget -WingetId "Python.Python.3.13" -DisplayName "Python 3.13" -ManualUrl "https://www.python.org/downloads/"
            if ($installed) {
                $pythonCmd = Get-Command python -ErrorAction SilentlyContinue
                if (-not $pythonCmd) { $pythonCmd = Get-Command py -ErrorAction SilentlyContinue }
                if ($pythonCmd) {
                    $pyVerOutput = (& $pythonCmd.Source --version 2>&1) -join ""
                    if ($pyVerOutput -match 'Python\s+(\d+)\.(\d+)') {
                        $pyMajor = [int]$Matches[1]; $pyMinor = [int]$Matches[2]
                        if ($pyMajor -gt 3 -or ($pyMajor -eq 3 -and $pyMinor -ge 11)) {
                            $pythonOk = $true
                        }
                    }
                }
            }
            if (-not $pythonOk) {
                throw "Python 3.11+ required for MCP bridges. Install and re-run."
            }
        }
        Write-Status "Python: $((& $pythonCmd.Source --version 2>&1) -join '')" "Green"

        # Capture the resolved executable path. Prefer a real `python.exe`; if
        # we only have the `py` launcher, resolve it to the actual interpreter
        # because `py -m venv` produces venvs that other tooling may not find.
        if ($pythonCmd.Name -eq 'py.exe' -or $pythonCmd.Name -eq 'py') {
            $pyResolved = (& $pythonCmd.Source -3 -c "import sys; print(sys.executable)" 2>&1) -join ""
            if ($pyResolved -and (Test-Path $pyResolved)) {
                $script:ResolvedPython = $pyResolved
                Write-Status "  Resolved via py launcher: $pyResolved" "DarkGray"
            } else {
                $script:ResolvedPython = $pythonCmd.Source
            }
        } else {
            $script:ResolvedPython = $pythonCmd.Source
        }
    } else {
        Write-Status "Prerequisite check: skipped (-SkipPrereqCheck)" "DarkGray"
        # Still need a python to create venvs later.
        $fallback = Get-Command python -ErrorAction SilentlyContinue
        if (-not $fallback) { $fallback = Get-Command py -ErrorAction SilentlyContinue }
        if ($fallback) { $script:ResolvedPython = $fallback.Source }
    }

    # ---------------------------------------------------------------------- #
    # Step 2: Ghidra
    # ---------------------------------------------------------------------- #
    Write-Step "GHIDRA $($Dependencies.Ghidra.Version)"

    $ghidraRoot = Join-Path $ProjectRoot "ghidra"
    $ghidraInstall = Join-Path $ghidraRoot "ghidra_$($Dependencies.Ghidra.Version)_PUBLIC"
    $ghidraRunBat = Join-Path $ghidraInstall "ghidraRun.bat"

    if ((Test-Path $ghidraRunBat) -and -not $Force) {
        Write-Status "Ghidra: already installed at $ghidraInstall" "DarkGray"
    } else {
        $ghidraZip = Join-Path $DownloadDir $Dependencies.Ghidra.FileName
        if (-not $SkipDownload -and (-not (Test-Path $ghidraZip) -or $Force)) {
            Write-Status "Downloading Ghidra $($Dependencies.Ghidra.Version) (~400 MB)..." "White"
            Get-DownloadFile $Dependencies.Ghidra.Url $ghidraZip
        }

        if (-not (Test-Path $ghidraZip)) {
            throw "Ghidra archive not found at $ghidraZip. Re-run without -SkipDownload."
        }

        New-Item -ItemType Directory -Path $ghidraRoot -Force | Out-Null
        if ($PSCmdlet.ShouldProcess($ghidraRoot, "Extract Ghidra")) {
            Write-Status "Extracting Ghidra to $ghidraRoot ..." "White"
            Expand-DependencyArchive $ghidraZip $ghidraRoot

            if (-not (Test-Path $ghidraRunBat)) {
                # The zip extracts to ghidra_<version>_PUBLIC/ — confirm the
                # canonical install path matches what other tooling expects.
                $candidate = Get-ChildItem $ghidraRoot -Directory -Filter "ghidra_*_PUBLIC" | Select-Object -First 1
                if ($candidate -and $candidate.Name -ne "ghidra_$($Dependencies.Ghidra.Version)_PUBLIC") {
                    Write-Status "  Renaming $($candidate.Name) -> ghidra_$($Dependencies.Ghidra.Version)_PUBLIC" "DarkGray"
                    Rename-Item $candidate.FullName -NewName "ghidra_$($Dependencies.Ghidra.Version)_PUBLIC"
                }
            }
            Write-Status "Ghidra: installed" "Green"
        }
    }

    # ---------------------------------------------------------------------- #
    # Step 3: GhidraMCP — clone bridge + install extension
    # ---------------------------------------------------------------------- #
    Write-Step "GHIDRAMCP BRIDGE + PLUGIN $($Dependencies.GhidraMcp.Version)"

    $ghidraMcpDir = Join-Path $ExternalDir "ghidra-mcp"
    $bridgePy = Join-Path $ghidraMcpDir "bridge_mcp_ghidra.py"

    if ((Test-Path $bridgePy) -and -not $Force) {
        Write-Status "ghidra-mcp: bridge already cloned at $ghidraMcpDir" "DarkGray"
    } else {
        # Git presence was validated (and auto-installed if needed) in the
        # prereq step at the top of this function; nothing to do here.
        if (Test-Path $ghidraMcpDir) {
            Write-Status "ghidra-mcp: updating existing clone..." "DarkGray"
            & git -C $ghidraMcpDir fetch --tags 2>&1 | ForEach-Object { Write-Status "  $_" "DarkGray" }
            & git -C $ghidraMcpDir checkout "v$($Dependencies.GhidraMcp.Version)" 2>&1 | ForEach-Object { Write-Status "  $_" "DarkGray" }
        } else {
            Write-Status "Cloning bethington/ghidra-mcp -> external/ghidra-mcp/" "White"
            & git clone --branch "v$($Dependencies.GhidraMcp.Version)" --depth 1 $Dependencies.GhidraMcp.RepoUrl $ghidraMcpDir 2>&1 | ForEach-Object {
                if ($_ -match 'Cloning|Resolving|Receiving|Updating') { Write-Status "  $_" "DarkGray" }
            }
        }

        if (-not (Test-Path $bridgePy)) {
            throw "ghidra-mcp clone failed — bridge_mcp_ghidra.py missing in $ghidraMcpDir."
        }
        Write-Status "ghidra-mcp: cloned" "Green"
    }

    # Download the pre-built extension zip and deploy it to the Ghidra user dir.
    $extensionZip = Join-Path $DownloadDir $Dependencies.GhidraMcp.ExtensionFile
    if (-not $SkipDownload -and (-not (Test-Path $extensionZip) -or $Force)) {
        Write-Status "Downloading GhidraMCP extension zip..." "White"
        Get-DownloadFile $Dependencies.GhidraMcp.ExtensionUrl $extensionZip
    }

    $ghidraUserExtRoot = Join-Path $env:APPDATA "ghidra\ghidra_$($Dependencies.Ghidra.Version)_PUBLIC\Extensions"
    $ghidraMcpExtTarget = Join-Path $ghidraUserExtRoot "GhidraMCP"
    $extensionPropertiesPath = Join-Path $ghidraMcpExtTarget "extension.properties"

    if ((Test-Path $extensionPropertiesPath) -and -not $Force) {
        Write-Status "GhidraMCP extension: already deployed at $ghidraMcpExtTarget" "DarkGray"
    } elseif (Test-Path $extensionZip) {
        if ($PSCmdlet.ShouldProcess($ghidraMcpExtTarget, "Deploy GhidraMCP extension")) {
            Write-Status "Deploying GhidraMCP extension -> $ghidraMcpExtTarget" "White"
            if (Test-Path $ghidraMcpExtTarget) { Remove-Item $ghidraMcpExtTarget -Recurse -Force }
            New-Item -ItemType Directory -Path $ghidraUserExtRoot -Force | Out-Null

            $extractTemp = Join-Path $DownloadDir "ghidramcp_extract"
            if (Test-Path $extractTemp) { Remove-Item $extractTemp -Recurse -Force }
            Expand-DependencyArchive $extensionZip $extractTemp

            # The zip typically extracts to a single GhidraMCP/ folder.
            $extractedRoot = Get-ChildItem $extractTemp -Directory | Select-Object -First 1
            if ($extractedRoot) {
                Move-Item $extractedRoot.FullName $ghidraMcpExtTarget
            } else {
                # Some packaging variants put files at the zip root.
                New-Item -ItemType Directory -Path $ghidraMcpExtTarget -Force | Out-Null
                Copy-Item -Path (Join-Path $extractTemp "*") -Destination $ghidraMcpExtTarget -Recurse -Force
            }
            Remove-Item $extractTemp -Recurse -Force -ErrorAction SilentlyContinue
            Write-Status "GhidraMCP extension: deployed" "Green"
        }
    } else {
        Write-Status "GhidraMCP extension zip not cached — skipping deploy." "Yellow"
        Write-Status "  Build from source manually:" "Yellow"
        Write-Status "    cd $ghidraMcpDir" "Cyan"
        Write-Status "    .\ghidra-mcp-setup.ps1 -Deploy -GhidraPath '$ghidraInstall'" "Cyan"
    }

    # ---------------------------------------------------------------------- #
    # Step 4: ghidra-mcp Python venv
    # ---------------------------------------------------------------------- #
    Write-Step "GHIDRA-MCP PYTHON VENV"

    $ghidraVenv = Join-Path $ProjectRoot ".venvs\ghidra-mcp"
    $ghidraVenvPython = Join-Path $ghidraVenv "Scripts\python.exe"

    if ((Test-Path $ghidraVenvPython) -and -not $Force) {
        Write-Status ".venvs\ghidra-mcp: already created" "DarkGray"
    } else {
        if (-not $script:ResolvedPython) {
            throw "No Python interpreter resolved. Re-run without -SkipPrereqCheck, or ensure python.exe is on PATH."
        }
        if ($PSCmdlet.ShouldProcess($ghidraVenv, "Create ghidra-mcp venv")) {
            Write-Status "Creating Python venv at $ghidraVenv ..." "White"
            New-Item -ItemType Directory -Path (Split-Path $ghidraVenv -Parent) -Force | Out-Null
            # Use the interpreter resolved in the prereq step rather than `python`
            # directly — handles the case where only the `py` launcher was on PATH.
            & $script:ResolvedPython -m venv $ghidraVenv 2>&1 | ForEach-Object { Write-Status "  $_" "DarkGray" }

            $reqFile = Join-Path $ghidraMcpDir "requirements.txt"
            if (Test-Path $reqFile) {
                Write-Status "Installing requirements from $reqFile ..." "White"
                & $ghidraVenvPython -m pip install --upgrade pip 2>&1 | ForEach-Object {
                    if ($_ -match 'Successfully|already satisfied') { Write-Status "  $_" "DarkGray" }
                }
                & $ghidraVenvPython -m pip install -r $reqFile 2>&1 | ForEach-Object {
                    if ($_ -match 'Successfully|already satisfied') { Write-Status "  $_" "DarkGray" }
                }
            } else {
                Write-Status "  No requirements.txt in $ghidraMcpDir — installing minimal deps." "DarkGray"
                & $ghidraVenvPython -m pip install --upgrade pip 2>&1 | Out-Null
                & $ghidraVenvPython -m pip install mcp httpx 2>&1 | Out-Null
            }
            Write-Status "ghidra-mcp venv: ready" "Green"
        }
    }

    # ---------------------------------------------------------------------- #
    # Step 5: x64dbg snapshot (manual-fetch — snapshot URLs rotate weekly)
    # ---------------------------------------------------------------------- #
    Write-Step "X64DBG SNAPSHOT"

    $dbgRoot = Join-Path $ProjectRoot "dbg"
    $dbgRelease = Join-Path $dbgRoot "release"
    $x96dbgExe = Join-Path $dbgRelease "x96dbg.exe"

    if ((Test-Path $x96dbgExe) -and -not $Force) {
        Write-Status "x64dbg: already installed at $dbgRelease" "DarkGray"
    } else {
        # Look for any cached snapshot zip the user may have dropped into
        # external\_downloads\ — name doesn't matter, just match "snapshot*.zip".
        $cachedSnapshot = Get-ChildItem $DownloadDir -Filter "snapshot*.zip" -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($cachedSnapshot) {
            if ($PSCmdlet.ShouldProcess($dbgRoot, "Extract cached x64dbg snapshot")) {
                Write-Status "Extracting cached snapshot: $($cachedSnapshot.Name)" "White"
                New-Item -ItemType Directory -Path $dbgRoot -Force | Out-Null
                Expand-DependencyArchive $cachedSnapshot.FullName $dbgRoot
                if (-not (Test-Path $x96dbgExe)) {
                    $candidate = Get-ChildItem $dbgRoot -Directory | Where-Object {
                        Test-Path (Join-Path $_.FullName "release\x96dbg.exe")
                    } | Select-Object -First 1
                    if ($candidate) {
                        # If $dbgRelease already exists from a partial earlier
                        # run, Move-Item -Force into it would nest the new
                        # release/ inside the old one (producing dbg/release/
                        # release/...). Wipe before move so the final layout is
                        # always dbg/release/{x32,x64,x96dbg.exe,...}.
                        if (Test-Path $dbgRelease) {
                            Remove-Item $dbgRelease -Recurse -Force
                        }
                        Move-Item (Join-Path $candidate.FullName "release") $dbgRelease -Force
                        Remove-Item $candidate.FullName -Recurse -Force
                    }
                }
                if (Test-Path $x96dbgExe) {
                    Write-Status "x64dbg: installed" "Green"
                } else {
                    Write-Status "x64dbg: extraction completed but x96dbg.exe not found — manual install required." "Yellow"
                }
            }
        } else {
            Write-Status "x64dbg snapshot zip not found in $DownloadDir." "Yellow"
            Write-Status "  x64dbg snapshot asset URLs rotate weekly — the bootstrap cannot" "DarkGray"
            Write-Status "  pin one URL safely. Download manually:" "DarkGray"
            Write-Status "    1. Open $($Dependencies.X64Dbg.SnapshotPageUrl)" "Cyan"
            Write-Status "    2. Download snapshot_<latest-date>.zip" "Cyan"
            Write-Status "    3. Save as $DownloadDir\snapshot.zip (any name starting with 'snapshot' works)" "Cyan"
            Write-Status "    4. Re-run: pwsh setup.ps1 -WithReToolchain -SkipDownload" "Cyan"
            Write-Status "  Continuing without x64dbg — Ghidra MCP will still work for static analysis." "DarkGray"
        }
    }

    # ---------------------------------------------------------------------- #
    # Step 6: x64dbg-automate plugin DLLs
    #   Gated on x64dbg being installed (Step 5 succeeded). Otherwise we'd
    #   silently create stub dbg/release/{x32,x64}/plugins/ directories with
    #   plugin DLLs that have no host debugger to load them, and report
    #   "installed" even though the toolchain is half-built.
    # ---------------------------------------------------------------------- #
    Write-Step "X64DBG-AUTOMATE PLUGIN $($Dependencies.X64DbgAutomate.Version)"

    if (-not (Test-Path $x96dbgExe)) {
        Write-Status "x64dbg not installed (no x96dbg.exe at $dbgRelease) — skipping plugin install." "Yellow"
        Write-Status "  Drop a snapshot zip into $DownloadDir and re-run (see Step 5 hints above)." "DarkGray"
    } else {
        $plugin32Dir = Join-Path $dbgRelease "x32\plugins"
        $plugin64Dir = Join-Path $dbgRelease "x64\plugins"
        $dp32Target = Join-Path $plugin32Dir "x64dbg-automate.dp32"
        $dp64Target = Join-Path $plugin64Dir "x64dbg-automate.dp64"
        $zmq32Target = Join-Path $plugin32Dir "libzmq-mt-4_3_5.dll"
        $zmq64Target = Join-Path $plugin64Dir "libzmq-mt-4_3_5.dll"

        if ((Test-Path $dp32Target) -and (Test-Path $dp64Target) -and -not $Force) {
            Write-Status "x64dbg-automate plugin: already installed" "DarkGray"
        } else {
            $dp32Cached = Join-Path $DownloadDir "x64dbg-automate.dp32"
            $dp64Cached = Join-Path $DownloadDir "x64dbg-automate.dp64"
            $zmqCached  = Join-Path $DownloadDir "libzmq-mt-4_3_5.dll"

            if (-not $SkipDownload) {
                if (-not (Test-Path $dp32Cached) -or $Force) {
                    Write-Status "Downloading x64dbg-automate.dp32 ..." "White"
                    Get-DownloadFile $Dependencies.X64DbgAutomate.Dp32Url $dp32Cached
                }
                if (-not (Test-Path $dp64Cached) -or $Force) {
                    Write-Status "Downloading x64dbg-automate.dp64 ..." "White"
                    Get-DownloadFile $Dependencies.X64DbgAutomate.Dp64Url $dp64Cached
                }
                if (-not (Test-Path $zmqCached) -or $Force) {
                    Write-Status "Downloading libzmq-mt-4_3_5.dll ..." "White"
                    Get-DownloadFile $Dependencies.X64DbgAutomate.ZmqDllUrl $zmqCached
                }
            }

            if ((Test-Path $dp32Cached) -and (Test-Path $dp64Cached) -and (Test-Path $zmqCached)) {
                if ($PSCmdlet.ShouldProcess("$plugin32Dir + $plugin64Dir", "Install x64dbg-automate plugin")) {
                    New-Item -ItemType Directory -Path $plugin32Dir -Force | Out-Null
                    New-Item -ItemType Directory -Path $plugin64Dir -Force | Out-Null
                    Copy-Item $dp32Cached $dp32Target -Force
                    Copy-Item $dp64Cached $dp64Target -Force
                    Copy-Item $zmqCached  $zmq32Target -Force
                    Copy-Item $zmqCached  $zmq64Target -Force
                    Write-Status "x64dbg-automate plugin: installed" "Green"
                }
            } else {
                Write-Status "Plugin artifacts not all cached — skipping install." "Yellow"
            }
        }
    }

    # ---------------------------------------------------------------------- #
    # Step 7: x64dbg-automate Python venv (provides x64dbg-automate-mcp)
    # ---------------------------------------------------------------------- #
    Write-Step "X64DBG-MCP PYTHON VENV"

    $x64Venv = Join-Path $ProjectRoot ".venvs\x64dbg-mcp"
    $x64VenvMcp = Join-Path $x64Venv "Scripts\x64dbg-automate-mcp.exe"

    if ((Test-Path $x64VenvMcp) -and -not $Force) {
        Write-Status ".venvs\x64dbg-mcp: already created" "DarkGray"
    } else {
        if (-not $script:ResolvedPython) {
            throw "No Python interpreter resolved. Re-run without -SkipPrereqCheck, or ensure python.exe is on PATH."
        }
        if ($PSCmdlet.ShouldProcess($x64Venv, "Create x64dbg-mcp venv")) {
            Write-Status "Creating Python venv at $x64Venv ..." "White"
            New-Item -ItemType Directory -Path (Split-Path $x64Venv -Parent) -Force | Out-Null
            # Same as the ghidra-mcp venv above — use the resolved interpreter.
            & $script:ResolvedPython -m venv $x64Venv 2>&1 | ForEach-Object { Write-Status "  $_" "DarkGray" }

            $x64Python = Join-Path $x64Venv "Scripts\python.exe"
            & $x64Python -m pip install --upgrade pip 2>&1 | Out-Null
            Write-Status "Installing $($Dependencies.X64DbgAutomate.PyPiPackage) ..." "White"
            & $x64Python -m pip install $Dependencies.X64DbgAutomate.PyPiPackage 2>&1 | ForEach-Object {
                if ($_ -match 'Successfully|already satisfied') { Write-Status "  $_" "DarkGray" }
            }

            if (-not (Test-Path $x64VenvMcp)) {
                Write-Status "  WARNING: x64dbg-automate-mcp.exe not produced by install — check PyPI package name." "Yellow"
            } else {
                Write-Status "x64dbg-mcp venv: ready" "Green"
            }
        }
    }

    # ---------------------------------------------------------------------- #
    # Step 8: .mcp.json (only if it doesn't exist — never clobber)
    # ---------------------------------------------------------------------- #
    Write-Step "MCP CLIENT CONFIG (.mcp.json)"

    $mcpJsonPath = Join-Path $ProjectRoot ".mcp.json"
    $mcpExamplePath = Join-Path $ProjectRoot ".mcp.json.example"

    if (Test-Path $mcpJsonPath) {
        Write-Status ".mcp.json: already present — leaving untouched" "DarkGray"
        Write-Status "  (rename or delete it if you want the bootstrap to regenerate)" "DarkGray"
    } elseif (-not (Test-Path $mcpExamplePath)) {
        Write-Status ".mcp.json.example: missing — cannot generate config" "Yellow"
        Write-Status "  Re-run from a clean checkout that includes .mcp.json.example." "DarkGray"
    } else {
        if ($PSCmdlet.ShouldProcess($mcpJsonPath, "Generate .mcp.json")) {
            $template = Get-Content $mcpExamplePath -Raw
            # Use literal .Replace() rather than -replace: the cimmeria-rag
            # functions key can contain `$` sequences that -replace would
            # interpret as backreferences and silently corrupt.
            $resolved = $template.Replace('<CIMMERIA_ROOT>', ($ProjectRoot -replace '\\', '\\\\'))
            if ($CimmeriaRagKey) {
                $resolved = $resolved.Replace('REPLACE_WITH_FUNCTIONS_KEY', $CimmeriaRagKey)
            }
            Set-Content -Path $mcpJsonPath -Value $resolved -Encoding UTF8
            Write-Status ".mcp.json: generated from .mcp.json.example" "Green"
            if (-not $CimmeriaRagKey) {
                Write-Status "  Note: cimmeria-rag key left as REPLACE_WITH_FUNCTIONS_KEY." "DarkGray"
                Write-Status "  Edit .mcp.json and replace it, or delete the cimmeria-rag block if not using it." "DarkGray"
            }
        }
    }

    # ---------------------------------------------------------------------- #
    # Final guidance
    # ---------------------------------------------------------------------- #
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host " RE Toolchain Install Complete" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host ""
    Write-Host "Next steps:" -ForegroundColor Yellow
    Write-Host "  1. Open Ghidra:  $ghidraInstall\ghidraRun.bat"
    Write-Host "     Then File > Configure > Configure All Plugins > tick GhidraMCP."
    Write-Host "     Then Tools > GhidraMCP > Start MCP Server."
    Write-Host "     Confirm the bound port from Ghidra's console (default 8089;"
    Write-Host "     on Windows iphlpsvc often forces a fallback to 8100)."
    Write-Host ""
    Write-Host "  2. Load SGW.exe and run docs/reverse-engineering/annotation-scripts/"
    Write-Host "     scripts 01-10 in order to get to the 60.6% named-function baseline."
    Write-Host ""
    Write-Host "  3. Launch x64dbg:  $dbgRelease\x96dbg.exe"
    Write-Host "     Plugins menu > about — confirm x64dbg-automate is listed."
    Write-Host ""
    Write-Host "  4. Restart Claude Code so it re-reads .mcp.json."
    Write-Host "     Verify with: 'List MCP tools, grouped by server prefix.'"
    Write-Host ""
    Write-Host "Reference:"
    Write-Host "  docs/guides/re-toolchain-setup.md             Setup walkthrough"
    Write-Host "  docs/guides/reverse-engineering-with-claude.md Workflow + agents"
    Write-Host "  docs/reverse-engineering/toolchain/install-ghidra-mcp.md  Plugin reference"
    Write-Host ""
}
