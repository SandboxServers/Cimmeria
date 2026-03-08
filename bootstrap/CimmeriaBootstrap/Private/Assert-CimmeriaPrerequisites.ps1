function Assert-CimmeriaPrerequisites {
    <#
    .SYNOPSIS
        Verifies that required tools are installed: Rust toolchain, Node.js (optional).
        Automatically installs Rust via rustup if missing, then relaunches in a new terminal.

    .PARAMETER RequireNode
        If set, also checks for node and npm (required for Tauri app build).
    #>
    [CmdletBinding()]
    param(
        [switch]$RequireNode
    )

    $missing = @()

    # Ensure default Rust install location is on PATH
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    if ((Test-Path $cargoBin) -and ($env:PATH -notlike "*$cargoBin*")) {
        $env:PATH = "$cargoBin;$env:PATH"
        Write-Status "Added $cargoBin to PATH for this session" "Cyan"
    }

    # Rust toolchain
    $cargo = Get-Command cargo -ErrorAction SilentlyContinue
    if ($cargo) {
        $cargoVersion = (cargo --version 2>&1) -join ""
        Write-Status "Found cargo: $cargoVersion" "Green"
    } else {
        Write-Status "Rust toolchain not found — installing via rustup..." "Yellow"
        Install-RustToolchain
        # Does not return — relaunches in a new terminal
    }

    $rustup = Get-Command rustup -ErrorAction SilentlyContinue
    if ($rustup) {
        $toolchain = (rustup show active-toolchain 2>&1) -join ""
        Write-Status "Found rustup: $toolchain" "Green"
    } else {
        Write-Status "rustup not found — installing via rustup..." "Yellow"
        Install-RustToolchain
        # Does not return — relaunches in a new terminal
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
}

function Install-RustToolchain {
    <#
    .SYNOPSIS
        Downloads and runs rustup-init to install the Rust toolchain,
        then relaunches the bootstrap in a new terminal window and exits the current one.
    #>

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
        Write-Status "" "White"
        Write-Status "A new terminal is required for PATH changes to take effect." "Yellow"
        Write-Status "Relaunching bootstrap in a new window..." "Yellow"

        # Build the command to relaunch setup.ps1 with the same arguments
        $setupScript = Join-Path $paths.ProjectRoot "setup.ps1"
        $relaunchCmd = "Set-Location '$($paths.ProjectRoot)'; & '$setupScript'"

        # Open a new PowerShell window with the bootstrap command
        Start-Process pwsh -ArgumentList @("-NoExit", "-Command", $relaunchCmd) -WorkingDirectory $paths.ProjectRoot

        Write-Status "New terminal launched. Closing this one." "Cyan"

        # Exit the current process
        [Environment]::Exit(0)

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
        Write-Status "" "White"
        Write-Status "A new terminal is required for PATH changes to take effect." "Yellow"
        Write-Status "Relaunching bootstrap in a new window..." "Yellow"

        $setupScript = Join-Path $paths.ProjectRoot "setup.ps1"
        $relaunchCmd = "cd '$($paths.ProjectRoot)' && pwsh '$setupScript'"

        # Detect available terminal emulator and relaunch
        $terminal = $null
        if (Get-Command wt -ErrorAction SilentlyContinue) {
            # Windows Terminal (available in WSL too)
            $terminal = "wt"
            Start-Process wt -ArgumentList @("pwsh", "-NoExit", "-Command", $relaunchCmd)
        } elseif (Get-Command gnome-terminal -ErrorAction SilentlyContinue) {
            Start-Process gnome-terminal -ArgumentList @("--", "pwsh", "-NoExit", "-Command", $relaunchCmd)
        } elseif (Get-Command xterm -ErrorAction SilentlyContinue) {
            Start-Process xterm -ArgumentList @("-e", "pwsh -NoExit -Command `"$relaunchCmd`"")
        } elseif ($env:TERM_PROGRAM -eq "Apple_Terminal" -or (Get-Command open -ErrorAction SilentlyContinue)) {
            # macOS
            Start-Process open -ArgumentList @("-a", "Terminal", "pwsh", "--args", "-NoExit", "-Command", $relaunchCmd)
        } else {
            Write-Status "Could not detect a terminal emulator to relaunch in." "Yellow"
            Write-Status "Please open a new terminal and run:  pwsh setup.ps1" "Yellow"
        }

        Write-Status "New terminal launched. Closing this one." "Cyan"
        [Environment]::Exit(0)
    }
}
