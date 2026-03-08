function Assert-CimmeriaPrerequisites {
    <#
    .SYNOPSIS
        Verifies that required tools are installed: Rust toolchain, Node.js (optional).

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
        Write-Status "ERROR: cargo not found" "Red"
        Write-Status "  Install Rust from: https://rustup.rs" "Yellow"
        Write-Status "  Then restart your shell and re-run this script." "Yellow"
        $missing += "cargo"
    }

    $rustup = Get-Command rustup -ErrorAction SilentlyContinue
    if ($rustup) {
        $toolchain = (rustup show active-toolchain 2>&1) -join ""
        Write-Status "Found rustup: $toolchain" "Green"
    } else {
        Write-Status "ERROR: rustup not found" "Red"
        Write-Status "  Install Rust from: https://rustup.rs" "Yellow"
        $missing += "rustup"
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
