function Assert-CimmeriaPrerequisites {
    <#
    .SYNOPSIS
        Verifies that required tools are installed: MSVC build tools, Rust toolchain, Node.js (optional).
        Automatically installs missing prerequisites and relaunches in a new terminal if needed.

    .PARAMETER RequireNode
        If set, also checks for node and npm (required for Tauri app build).
    #>
    [CmdletBinding()]
    param(
        [switch]$RequireNode
    )

    $missing = @()
    $needsRelaunch = $false
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    # --- MSVC Build Tools (Windows only) ---
    # Rust's msvc target requires link.exe from VS C++ Build Tools
    if ($isWin) {
        $hasLinker = Test-MSVCLinker
        if ($hasLinker) {
            Write-Status "Found MSVC linker (link.exe)" "Green"
        } else {
            Write-Status "MSVC C++ Build Tools not found — required for Rust builds on Windows" "Yellow"
            Install-MSVCBuildTools
            $needsRelaunch = $true
        }
    }

    # --- Rust toolchain ---
    # Ensure default Rust install location is on PATH
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if ((Test-Path $cargoBin) -and ($env:PATH -notlike "*$cargoBin*")) {
        $env:PATH = "$cargoBin;$env:PATH"
        Write-Status "Added $cargoBin to PATH for this session" "Cyan"
    }

    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cargo) {
        $cargoVersion = (cargo --version 2>&1) -join ""
        Write-Status "Found cargo: $cargoVersion" "Green"
    } else {
        Write-Status "Rust toolchain not found — installing via rustup..." "Yellow"
        Install-RustToolchain -RelaunchAfter:(-not $needsRelaunch)
        # If RelaunchAfter was false, we continue (will relaunch at end)
        $needsRelaunch = $true
    }

    $rustup = Get-Command rustup -ErrorAction SilentlyContinue
    if ($rustup) {
        $toolchain = (rustup show active-toolchain 2>&1) -join ""
        Write-Status "Found rustup: $toolchain" "Green"
    } elseif (-not $needsRelaunch) {
        Write-Status "rustup not found — installing via rustup..." "Yellow"
        Install-RustToolchain -RelaunchAfter:$true
        # Does not return
    }

    # Node.js (only when building Tauri app)
    if ($RequireNode) {
        $node = Get-Command node -ErrorAction SilentlyContinue
        if ($node) {
            $nodeVersion = (node --version 2>&1) -join ""
            Write-Status "Found node: $nodeVersion" "Green"
        } else {
            Write-Status "ERROR: node not found" "Red"
            Write-Status "  Install Node.js from: https://nodejs.org" "Yellow"
            $missing += "node"
        }

        $npm = Get-Command npm -ErrorAction SilentlyContinue
        if ($npm) {
            $npmVersion = (npm --version 2>&1) -join ""
            Write-Status "Found npm: $npmVersion" "Green"
        } else {
            Write-Status "ERROR: npm not found" "Red"
            Write-Status "  Install Node.js from: https://nodejs.org (npm is included)" "Yellow"
            $missing += "npm"
        }
    }

    if ($missing.Count -gt 0) {
        throw "Missing required tools: $($missing -join ', '). Install them and re-run."
    }

    # If any installs happened that need a new terminal, relaunch now
    if ($needsRelaunch) {
        Invoke-BootstrapRelaunch
    }
}

# ─── MSVC Build Tools ────────────────────────────────────────────────────────

function Test-MSVCLinker {
    <#
    .SYNOPSIS
        Returns $true if an MSVC link.exe is available (via vswhere or PATH).
    #>

    # Method 1: Use vswhere to find any VS install with the C++ toolset
    $vswherePaths = @(
        "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
        "${env:ProgramFiles}\Microsoft Visual Studio\Installer\vswhere.exe"
    )
    foreach ($vswhere in $vswherePaths) {
        if (Test-Path $vswhere) {
            $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
            if ($vsPath) {
                return $true
            }
        }
    }

    # Method 2: Check if link.exe is already on PATH (e.g. Developer Command Prompt)
    $link = Get-Command link.exe -ErrorAction SilentlyContinue
    if ($link -and $link.Source -match 'MSVC|VC') {
        return $true
    }

    return $false
}

function Install-MSVCBuildTools {
    <#
    .SYNOPSIS
        Installs the Visual Studio Build Tools with the C++ desktop workload.
        Tries winget first, falls back to direct installer download.
    #>

    $paths = Get-ProjectPaths

    # Try winget first (available on Windows 10 1709+ and all Windows 11)
    $winget = Get-Command winget -ErrorAction SilentlyContinue
    if ($winget) {
        Write-Status "Installing Visual Studio Build Tools via winget..." "White"
        Write-Status "This will install the C++ desktop workload (several GB download)." "White"
        Write-Status "A separate installer window may appear — please let it complete." "Yellow"

        & winget install Microsoft.VisualStudio.2022.BuildTools `
            --override "--wait --quiet --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended" `
            --accept-source-agreements --accept-package-agreements 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'Found|Installing|Successfully|Download|Progress|already') {
                Write-Status "  $line" "DarkGray"
            }
        }

        if ($LASTEXITCODE -eq 0 -or (Test-MSVCLinker)) {
            Write-Status "Visual Studio Build Tools installed successfully." "Green"
            return
        }

        Write-Status "winget install did not succeed — trying direct download..." "Yellow"
    }

    # Fallback: download the VS Build Tools bootstrapper directly
    $installerUrl = "https://aka.ms/vs/17/release/vs_BuildTools.exe"
    $installerPath = Join-Path $paths.DownloadDir "vs_BuildTools.exe"
    New-Item -ItemType Directory -Path $paths.DownloadDir -Force | Out-Null

    Write-Status "Downloading Visual Studio Build Tools installer..." "White"
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest -Uri $installerUrl -OutFile $installerPath -UseBasicParsing
    $ProgressPreference = 'Continue'
    Write-Status "Downloaded vs_BuildTools.exe" "Green"

    Write-Status "Running installer (this may take several minutes)..." "White"
    Write-Status "A separate installer window may appear — please let it complete." "Yellow"

    $installArgs = @(
        "--wait", "--quiet", "--norestart",
        "--add", "Microsoft.VisualStudio.Workload.VCTools",
        "--includeRecommended"
    )
    $proc = Start-Process -FilePath $installerPath -ArgumentList $installArgs -Wait -PassThru
    if ($proc.ExitCode -ne 0 -and $proc.ExitCode -ne 3010) {
        # 3010 = success, reboot recommended (but not required for our purposes)
        throw "VS Build Tools installer failed (exit code $($proc.ExitCode)). Install manually from https://visualstudio.microsoft.com/visual-cpp-build-tools/"
    }

    Write-Status "Visual Studio Build Tools installed successfully." "Green"
}

# ─── Rust Toolchain ──────────────────────────────────────────────────────────

function Install-RustToolchain {
    <#
    .SYNOPSIS
        Downloads and runs rustup-init to install the Rust toolchain.

    .PARAMETER RelaunchAfter
        If $true, relaunches bootstrap in a new terminal and exits immediately.
        If $false, returns after install (caller handles relaunch).
    #>
    param(
        [switch]$RelaunchAfter
    )

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    if ($isWin) {
        $rustupInit = Join-Path $paths.DownloadDir "rustup-init.exe"
        New-Item -ItemType Directory -Path $paths.DownloadDir -Force | Out-Null

        # Download rustup-init.exe
        Write-Status "Downloading rustup-init.exe..." "White"
        $ProgressPreference = 'SilentlyContinue'
        Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustupInit -UseBasicParsing
        $ProgressPreference = 'Continue'
        Write-Status "Downloaded rustup-init.exe" "Green"

        # Run the installer (non-interactive, default profile)
        Write-Status "Running rustup-init (this may take a minute)..." "White"
        & $rustupInit -y --default-toolchain stable 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'installed|downloading|default|stable') {
                Write-Status "  $line" "DarkGray"
            }
        }

        if ($LASTEXITCODE -ne 0) {
            throw "rustup-init failed with exit code $LASTEXITCODE. Install Rust manually from https://rustup.rs"
        }

        Write-Status "Rust toolchain installed successfully." "Green"

    } else {
        # Linux / macOS: use the shell installer
        Write-Status "Downloading and running rustup installer..." "White"
        $env:RUSTUP_INIT_SKIP_PATH_CHECK = "yes"
        & bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable" 2>&1 | ForEach-Object {
            $line = "$_"
            if ($line -match 'installed|downloading|default|stable|modified') {
                Write-Status "  $line" "DarkGray"
            }
        }

        if ($LASTEXITCODE -ne 0) {
            throw "rustup installer failed with exit code $LASTEXITCODE. Install Rust manually from https://rustup.rs"
        }

        Write-Status "Rust toolchain installed successfully." "Green"
    }

    if ($RelaunchAfter) {
        Invoke-BootstrapRelaunch
    }
}

# ─── Relaunch Helper ─────────────────────────────────────────────────────────

function Invoke-BootstrapRelaunch {
    <#
    .SYNOPSIS
        Relaunches setup.ps1 in a new terminal window and exits the current process.
        Used after installing tools that modify PATH (Rust, MSVC Build Tools).
    #>

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    Write-Status "" "White"
    Write-Status "A new terminal is required for PATH changes to take effect." "Yellow"
    Write-Status "Relaunching bootstrap in a new window..." "Yellow"

    $setupScript = Join-Path $paths.ProjectRoot "setup.ps1"

    if ($isWin) {
        $relaunchCmd = "Set-Location '$($paths.ProjectRoot)'; & '$setupScript'"
        Start-Process pwsh -ArgumentList @("-NoExit", "-Command", $relaunchCmd) -WorkingDirectory $paths.ProjectRoot
    } else {
        $relaunchCmd = "cd '$($paths.ProjectRoot)' && pwsh '$setupScript'"

        if (Get-Command wt -ErrorAction SilentlyContinue) {
            Start-Process wt -ArgumentList @("pwsh", "-NoExit", "-Command", $relaunchCmd)
        } elseif (Get-Command gnome-terminal -ErrorAction SilentlyContinue) {
            Start-Process gnome-terminal -ArgumentList @("--", "pwsh", "-NoExit", "-Command", $relaunchCmd)
        } elseif (Get-Command xterm -ErrorAction SilentlyContinue) {
            Start-Process xterm -ArgumentList @("-e", "pwsh -NoExit -Command `"$relaunchCmd`"")
        } elseif ($env:TERM_PROGRAM -eq "Apple_Terminal" -or (Get-Command open -ErrorAction SilentlyContinue)) {
            Start-Process open -ArgumentList @("-a", "Terminal", "pwsh", "--args", "-NoExit", "-Command", $relaunchCmd)
        } else {
            Write-Status "Could not detect a terminal emulator to relaunch in." "Yellow"
            Write-Status "Please open a new terminal and run:  pwsh setup.ps1" "Yellow"
        }
    }

    Write-Status "New terminal launched. Closing this one." "Cyan"
    [Environment]::Exit(0)
}
