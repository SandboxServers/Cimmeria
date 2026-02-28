function Start-CimmeriaServer {
    <#
    .SYNOPSIS
        Launches the Cimmeria server stack (Auth, Base, Cell).

    .DESCRIPTION
        Ensures PostgreSQL is running, then starts the three game server processes
        in order, waiting for each to begin accepting connections before proceeding.

        After launch, prints a status banner with connection details.

    .PARAMETER Configuration
        Build configuration to run: "Debug" (default) or "Release".

    .PARAMETER Port
        PostgreSQL port. Default 5433.

    .EXAMPLE
        Start-CimmeriaServer

    .EXAMPLE
        Start-CimmeriaServer -Configuration Release
    #>
    [CmdletBinding()]
    param(
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug",

        [int]$Port = 5433
    )

    $paths = Get-ProjectPaths
    $binDir = Join-Path $paths.ProjectRoot "bin64"

    Write-Step "STARTING CIMMERIA SERVER"

    # Ensure PostgreSQL is running
    $pgBin = Find-PostgreSQL
    if ($pgBin) {
        $pgCtl = Join-Path $pgBin "pg_ctl.exe"
        $pgDataDir = Join-Path $paths.ServerDir "pgdata"

        if (Test-Path $pgDataDir) {
            $statusResult = & $pgCtl status -D $pgDataDir 2>&1
            if ($LASTEXITCODE -ne 0) {
                Write-Status "Starting PostgreSQL..." "White"
                $pgLogFile = Join-Path $paths.ServerDir "logs\postgresql.log"
                New-Item -ItemType Directory -Path (Split-Path $pgLogFile) -Force | Out-Null
                & $pgCtl start -D $pgDataDir -l $pgLogFile -w -o "-p $Port" 2>&1 | ForEach-Object {
                    Write-Status "  $_" "DarkGray"
                }
                if ($LASTEXITCODE -ne 0) {
                    throw "Failed to start PostgreSQL. Check $pgLogFile"
                }
            } else {
                Write-Status "PostgreSQL already running." "DarkGray"
            }
        } else {
            Write-Status "WARNING: pgdata not found. Run Initialize-CimmeriaDatabase first." "Yellow"
        }
    }

    # Determine executable suffix
    $suffix = if ($Configuration -eq "Debug") { "_d" } else { "" }

    # Verify executables exist
    $authExe = Join-Path $binDir "AuthenticationServer${suffix}.exe"
    $baseExe = Join-Path $binDir "BaseApp${suffix}.exe"
    $cellExe = Join-Path $binDir "CellApp${suffix}.exe"

    foreach ($exe in @($authExe, $baseExe, $cellExe)) {
        if (-not (Test-Path $exe)) {
            throw "Server executable not found: $exe. Run Build-CimmeriaSolution first."
        }
    }

    # Start AuthenticationServer
    Write-Status "Starting AuthenticationServer..." "White"
    Start-Process -FilePath $authExe -WorkingDirectory $binDir -WindowStyle Normal
    if (Wait-ForPort -Port 8080 -TimeoutSeconds 15) {
        Write-Status "AuthenticationServer ready (port 8080)." "Green"
    } else {
        Write-Status "WARNING: AuthenticationServer did not respond on port 8080 within 15s." "Yellow"
        $authProc = Get-Process -Name "AuthenticationServer${suffix}" -ErrorAction SilentlyContinue
        if ($authProc) {
            Write-Status "  Process is alive (PID $($authProc.Id)) - may still be initializing." "Yellow"
        } else {
            throw "AuthenticationServer failed to start."
        }
    }

    # Start BaseApp
    Write-Status "Starting BaseApp..." "White"
    Start-Process -FilePath $baseExe -WorkingDirectory $binDir -WindowStyle Normal
    if (Wait-ForPort -Port 32832 -TimeoutSeconds 15) {
        Write-Status "BaseApp ready (port 32832)." "Green"
    } else {
        Write-Status "WARNING: BaseApp did not respond on port 32832 within 15s." "Yellow"
        $baseProc = Get-Process -Name "BaseApp${suffix}" -ErrorAction SilentlyContinue
        if ($baseProc) {
            Write-Status "  Process is alive (PID $($baseProc.Id)) - may still be initializing." "Yellow"
        } else {
            throw "BaseApp failed to start."
        }
    }

    # Start CellApp
    Write-Status "Starting CellApp..." "White"
    Start-Process -FilePath $cellExe -WorkingDirectory $binDir -WindowStyle Normal
    if (Wait-ForPort -Port 8990 -TimeoutSeconds 10) {
        Write-Status "CellApp ready (port 8990)." "Green"
    } else {
        # CellApp may not open console port if py_console_password is empty
        $cellProc = Get-Process -Name "CellApp${suffix}" -ErrorAction SilentlyContinue
        if ($cellProc) {
            Write-Status "CellApp running (PID $($cellProc.Id)) - console port not detected (normal if py_console_password is unset)." "Yellow"
        } else {
            throw "CellApp failed to start."
        }
    }

    # Print status banner
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host " Cimmeria Server Running" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host " Auth server:   http://localhost:8080 (client login)" -ForegroundColor White
    Write-Host " Shard server:  localhost:32832 (game client)" -ForegroundColor White
    Write-Host " PostgreSQL:    localhost:$Port" -ForegroundColor White
    Write-Host " Test account:  test / test" -ForegroundColor White
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host ""
    Write-Host " To connect a game client:" -ForegroundColor Gray
    Write-Host "   1. Run: Update-CimmeriaClient" -ForegroundColor Gray
    Write-Host "   2. Launch the game client" -ForegroundColor Gray
    Write-Host "   3. Login with: test / test" -ForegroundColor Gray
    Write-Host ""
    Write-Host " Stop with:" -ForegroundColor Gray
    Write-Host "   pwsh -c `"Import-Module ./bootstrap/CimmeriaBootstrap; Stop-CimmeriaServer`"" -ForegroundColor White
    Write-Host "=============================================" -ForegroundColor Green
}
