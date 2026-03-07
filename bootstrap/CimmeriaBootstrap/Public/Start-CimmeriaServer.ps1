function Start-CimmeriaServer {
    <#
    .SYNOPSIS
        Launches the Cimmeria server (single binary: Auth + Base + Cell).

    .DESCRIPTION
        Ensures PostgreSQL is running, then starts the cimmeria-server binary.
        The server reads configuration and starts all services in one process.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

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
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    Write-Step "STARTING CIMMERIA SERVER"

    # Ensure PostgreSQL is running
    if ($isWin) {
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
                    $pgCtlArgs = "start -D `"$pgDataDir`" -l `"$pgLogFile`" -o `"-p $Port`""
                    Start-Process -FilePath $pgCtl -ArgumentList $pgCtlArgs -WindowStyle Hidden
                    if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 15)) {
                        throw "Failed to start PostgreSQL. Check $pgLogFile"
                    }
                } else {
                    Write-Status "PostgreSQL already running." "DarkGray"
                }
            } else {
                Write-Status "WARNING: pgdata not found. Run Initialize-CimmeriaDatabase first." "Yellow"
            }
        }
    } else {
        # Linux/macOS: verify PG is reachable
        if (Wait-ForPort -Port $Port -TimeoutSeconds 2) {
            Write-Status "PostgreSQL reachable on port $Port." "DarkGray"
        } else {
            Write-Status "WARNING: PostgreSQL not reachable on port $Port." "Yellow"
            Write-Status "  Start it with: sudo systemctl start postgresql (Linux)" "Yellow"
            Write-Status "  Or:            brew services start postgresql@17 (macOS)" "Yellow"
        }
    }

    # Find server binary
    $profile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $exeSuffix = if ($isWin) { ".exe" } else { "" }
    $serverBin = Join-Path $paths.ProjectRoot "target/$profile/cimmeria-server$exeSuffix"

    if (-not (Test-Path $serverBin)) {
        throw "Server binary not found: $serverBin. Run Build-CimmeriaServer first."
    }

    # Launch
    Write-Status "Starting cimmeria-server ($Configuration)..." "White"
    $serverProc = Start-Process -FilePath $serverBin -WorkingDirectory $paths.ProjectRoot -PassThru
    if (-not $serverProc) {
        throw "Failed to start cimmeria-server."
    }

    # Wait for auth service to be ready
    if (Wait-ForPort -Port 8081 -TimeoutSeconds 15) {
        Write-Status "Auth service ready (port 8081)." "Green"
    } else {
        $proc = Get-Process -Id $serverProc.Id -ErrorAction SilentlyContinue
        if ($proc -and -not $proc.HasExited) {
            Write-Status "Server is alive (PID $($serverProc.Id)) - may still be initializing." "Yellow"
        } else {
            throw "cimmeria-server exited unexpectedly."
        }
    }

    # Print status banner
    Write-Host ""
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host " Cimmeria Server Running" -ForegroundColor Green
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host " Server PID:    $($serverProc.Id)" -ForegroundColor White
    Write-Host " Auth server:   http://localhost:8081 (client login)" -ForegroundColor White
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
