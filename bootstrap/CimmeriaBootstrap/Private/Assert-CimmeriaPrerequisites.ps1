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

    # --- Fast linker prerequisites ---
    # mold + clang on Linux/WSL, llvm-tools-preview on Windows
    # These are used by .cargo/config.toml to speed up incremental builds.
    $isLinux = (Test-Path variable:IsLinux) -and $IsLinux

    if ($isWin) {
        # Windows: ensure llvm-tools-preview rustup component is installed (provides lld-link)
        if ($rustup) {
            $lldInstalled = (rustup component list 2>&1) -join "`n" | Select-String "llvm-tools.*installed"
            if ($lldInstalled) {
                Write-Status "Found rustup component: llvm-tools-preview (lld linker)" "Green"
            } else {
                Write-Status "Installing rustup component: llvm-tools-preview (faster linker)..." "Yellow"
                rustup component add llvm-tools-preview 2>&1 | ForEach-Object {
                    Write-Status "  $_" "DarkGray"
                }
                if ($LASTEXITCODE -eq 0) {
                    Write-Status "llvm-tools-preview installed successfully." "Green"
                } else {
                    Write-Status "WARNING: Failed to install llvm-tools-preview. Builds will use default linker." "Yellow"
                }
            }
        }
    } elseif ($isLinux) {
        # Linux/WSL: check for mold and clang (used by .cargo/config.toml)
        $moldCmd = Get-Command mold -ErrorAction SilentlyContinue
        if ($moldCmd) {
            $moldVersion = (mold --version 2>&1) -join ""
            Write-Status "Found mold: $moldVersion" "Green"
        } else {
            Write-Status "mold linker not found — installing..." "Yellow"
            & sudo apt-get install -y mold 2>&1 | ForEach-Object {
                $line = "$_"
                if ($line -match 'Setting up|is already') {
                    Write-Status "  $line" "DarkGray"
                }
            }
            $moldCmd = Get-Command mold -ErrorAction SilentlyContinue
            if ($moldCmd) {
                Write-Status "mold installed successfully." "Green"
            } else {
                Write-Status "WARNING: Failed to install mold. Builds will use default linker." "Yellow"
                Write-Status "  Install manually: sudo apt install mold" "Yellow"
            }
        }

        $clangCmd = Get-Command clang -ErrorAction SilentlyContinue
        if ($clangCmd) {
            $clangVersion = (clang --version 2>&1 | Select-Object -First 1) -join ""
            Write-Status "Found clang: $clangVersion" "Green"
        } else {
            Write-Status "clang not found — installing..." "Yellow"
            & sudo apt-get install -y clang 2>&1 | ForEach-Object {
                $line = "$_"
                if ($line -match 'Setting up|is already') {
                    Write-Status "  $line" "DarkGray"
                }
            }
            $clangCmd = Get-Command clang -ErrorAction SilentlyContinue
            if ($clangCmd) {
                Write-Status "clang installed successfully." "Green"
            } else {
                Write-Status "WARNING: Failed to install clang. Builds will use default linker." "Yellow"
                Write-Status "  Install manually: sudo apt install clang" "Yellow"
            }
        }
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
        Detects the hosting terminal, opens a new window of the same type with
        setup.ps1, then closes the current terminal window.
    #>

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    Write-Status "" "White"
    Write-Status "A new terminal is required for PATH changes to take effect." "Yellow"
    Write-Status "Relaunching bootstrap in a new window..." "Yellow"

    $setupScript = Join-Path $paths.ProjectRoot "setup.ps1"

    if ($isWin) {
        $relaunchCmd = "Set-Location '$($paths.ProjectRoot)'; & '$setupScript'"

        if ($env:WT_SESSION) {
            # Running inside Windows Terminal — open a new WT window
            Write-Status "Detected: Windows Terminal" "DarkGray"
            Start-Process wt.exe -ArgumentList @(
                "new-tab", "--title", "Cimmeria Bootstrap",
                "-d", $paths.ProjectRoot,
                "pwsh", "-NoExit", "-Command", $relaunchCmd
            )
        } elseif ($env:TERM_PROGRAM -eq 'vscode') {
            # VS Code integrated terminal — can't open a new VS Code terminal programmatically,
            # fall back to opening a standalone pwsh window
            Write-Status "Detected: VS Code terminal (opening standalone PowerShell window)" "DarkGray"
            Start-Process pwsh -ArgumentList @("-NoExit", "-Command", $relaunchCmd) -WorkingDirectory $paths.ProjectRoot
        } else {
            # Plain conhost / PowerShell console window
            Write-Status "Detected: PowerShell console" "DarkGray"
            Start-Process pwsh -ArgumentList @("-NoExit", "-Command", $relaunchCmd) -WorkingDirectory $paths.ProjectRoot
        }

        Write-Status "New terminal launched. Closing this window..." "Cyan"
        Start-Sleep -Milliseconds 500

        # Close the current console window via Win32 API
        Close-CurrentConsoleWindow

    } else {
        $relaunchCmd = "cd '$($paths.ProjectRoot)' && pwsh '$setupScript'"
        $launched = $false

        if ($env:WT_SESSION -and (Get-Command wt -ErrorAction SilentlyContinue)) {
            Write-Status "Detected: Windows Terminal (WSL)" "DarkGray"
            Start-Process wt -ArgumentList @("new-tab", "-d", $paths.ProjectRoot, "pwsh", "-NoExit", "-Command", $relaunchCmd)
            $launched = $true
        } elseif ($env:TERM_PROGRAM -eq 'vscode') {
            Write-Status "Detected: VS Code terminal (opening standalone window)" "DarkGray"
        } elseif ($env:GNOME_TERMINAL_SERVICE -or $env:COLORTERM -eq 'truecolor' -and (Get-Command gnome-terminal -ErrorAction SilentlyContinue)) {
            Write-Status "Detected: GNOME Terminal" "DarkGray"
            Start-Process gnome-terminal -ArgumentList @("--", "pwsh", "-NoExit", "-Command", $relaunchCmd)
            $launched = $true
        } elseif ($env:KONSOLE_VERSION -and (Get-Command konsole -ErrorAction SilentlyContinue)) {
            Write-Status "Detected: Konsole" "DarkGray"
            Start-Process konsole -ArgumentList @("-e", "pwsh", "-NoExit", "-Command", $relaunchCmd)
            $launched = $true
        } elseif ($env:TERM_PROGRAM -eq "iTerm.app") {
            Write-Status "Detected: iTerm2" "DarkGray"
            Start-Process open -ArgumentList @("-a", "iTerm", "pwsh", "--args", "-NoExit", "-Command", $relaunchCmd)
            $launched = $true
        } elseif ($env:TERM_PROGRAM -eq "Apple_Terminal" -or ($IsMacOS -and (Get-Command open -ErrorAction SilentlyContinue))) {
            Write-Status "Detected: macOS Terminal" "DarkGray"
            Start-Process open -ArgumentList @("-a", "Terminal", "pwsh", "--args", "-NoExit", "-Command", $relaunchCmd)
            $launched = $true
        } elseif (Get-Command xterm -ErrorAction SilentlyContinue) {
            Write-Status "Detected: xterm" "DarkGray"
            Start-Process xterm -ArgumentList @("-e", "pwsh -NoExit -Command `"$relaunchCmd`"")
            $launched = $true
        }

        if (-not $launched) {
            Write-Status "Could not detect terminal emulator." "Yellow"
            Write-Status "Please open a new terminal and run:  pwsh setup.ps1" "Yellow"
            [Environment]::Exit(0)
        }

        Write-Status "New terminal launched. Closing this window..." "Cyan"
        Start-Sleep -Milliseconds 500
        [Environment]::Exit(0)
    }
}

function Close-CurrentConsoleWindow {
    <#
    .SYNOPSIS
        Closes the current console window using Win32 API (Windows only).
        Posts WM_CLOSE to the console window handle, which closes the tab/pane
        in Windows Terminal or the whole window in conhost.
    #>

    try {
        Add-Type -TypeDefinition @"
using System;
using System.Runtime.InteropServices;
public static class ConsoleWindowHelper {
    [DllImport("kernel32.dll")]
    public static extern IntPtr GetConsoleWindow();
    [DllImport("user32.dll")]
    [return: MarshalAs(UnmanagedType.Bool)]
    public static extern bool PostMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);
    public const uint WM_CLOSE = 0x0010;
    public static void CloseConsole() {
        IntPtr hwnd = GetConsoleWindow();
        if (hwnd != IntPtr.Zero) {
            PostMessage(hwnd, WM_CLOSE, IntPtr.Zero, IntPtr.Zero);
        }
    }
}
"@ -ErrorAction SilentlyContinue

        [ConsoleWindowHelper]::CloseConsole()
    } catch {
        # If P/Invoke fails for any reason, fall back to process exit
        [Environment]::Exit(0)
    }
}
