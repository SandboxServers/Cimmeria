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

        [int]$Port = 5433,

        [string]$LogLevel
    )

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    Write-Step "STARTING CIMMERIA SERVER"

    # Ensure PostgreSQL is running (check Docker container first, then local)
    $pgReady = $false

    # Check Docker container
    $docker = Get-Command docker -ErrorAction SilentlyContinue
    if ($docker) {
        $containerStatus = & docker inspect --format '{{.State.Status}}' cimmeria-postgres 2>&1
        if ($LASTEXITCODE -eq 0 -and $containerStatus -eq "running") {
            Write-Status "PostgreSQL running via Docker container." "DarkGray"
            $pgReady = $true
        } elseif ($LASTEXITCODE -eq 0) {
            # Container exists but not running — start it
            Write-Status "Starting Docker PostgreSQL container..." "White"
            & docker start cimmeria-postgres 2>&1 | Out-Null
            if (Wait-ForPort -Port $Port -TimeoutSeconds 15) {
                Write-Status "Docker PostgreSQL started." "Green"
                $pgReady = $true
            }
        }
    }

    # Fall back to local PostgreSQL
    if (-not $pgReady) {
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
            # Linux/macOS: check for managed instance first, then external
            $pgDataDir = Join-Path $paths.ServerDir "pgdata"
            if (Test-Path $pgDataDir) {
                # We have a managed PG instance — start it if needed
                $pgBin = Find-PostgreSQL
                if ($pgBin) {
                    $pgCtl = Join-Path $pgBin "pg_ctl"
                    $statusResult = & $pgCtl status -D $pgDataDir 2>&1
                    if ($LASTEXITCODE -ne 0) {
                        Write-Status "Starting managed PostgreSQL..." "White"
                        $pgLogFile = Join-Path $paths.ServerDir "logs/postgresql.log"
                        New-Item -ItemType Directory -Path (Split-Path $pgLogFile) -Force | Out-Null
                        & $pgCtl start -D $pgDataDir -l $pgLogFile -o "-p $Port" 2>&1 | ForEach-Object {
                            Write-Status "  $_" "DarkGray"
                        }
                        if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 15)) {
                            throw "Failed to start PostgreSQL. Check $pgLogFile"
                        }
                        Write-Status "PostgreSQL started on port $Port." "Green"
                    } else {
                        Write-Status "PostgreSQL already running." "DarkGray"
                    }
                }
            } elseif (Wait-ForPort -Port $Port -TimeoutSeconds 2) {
                Write-Status "PostgreSQL reachable on port $Port." "DarkGray"
            } else {
                Write-Status "WARNING: PostgreSQL not reachable on port $Port." "Yellow"
                Write-Status "  Start it with: sudo systemctl start postgresql (Linux)" "Yellow"
                Write-Status "  Or:            brew services start postgresql@17 (macOS)" "Yellow"
                Write-Status "  Or:            pwsh setup.ps1 -UseDocker" "Yellow"
            }
        }
    }

    # Find server binary
    $profile = if ($Configuration -eq "Release") { "release" } else { "debug" }
    $exeSuffix = if ($isWin) { ".exe" } else { "" }
    $serverBin = Join-Path $paths.ProjectRoot "target/$profile/cimmeria-server$exeSuffix"

    if (-not (Test-Path $serverBin)) {
        throw "Server binary not found: $serverBin. Run Build-CimmeriaServer first."
    }

    # Set RUST_LOG for console log level
    if ($LogLevel) {
        $env:RUST_LOG = $LogLevel
        Write-Status "RUST_LOG=$LogLevel" "Cyan"
    }

    # On WSL: set BASE_EXTERNAL to the WSL IP so the Windows game client can reach BaseApp.
    # The auth server tells the client "connect to BASE_EXTERNAL:BASE_PORT" for the game session.
    # Default 127.0.0.1 only works when client and server are on the same machine.
    if ($script:IsWSL -and -not $env:BASE_EXTERNAL) {
        $wslIp = (hostname -I 2>$null).Trim().Split(' ')[0]
        if ($wslIp) {
            $env:BASE_EXTERNAL = $wslIp
            Write-Status "BASE_EXTERNAL=$wslIp (auto-detected WSL IP)" "Cyan"
        }
    }

    # Launch (Start-Process inherits the current environment)
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
    $extAddr = if ($env:BASE_EXTERNAL) { $env:BASE_EXTERNAL } else { "127.0.0.1" }
    Write-Host " Auth server:   http://${extAddr}:8081 (client login)" -ForegroundColor White
    Write-Host " Shard server:  ${extAddr}:32832 (game client)" -ForegroundColor White
    Write-Host " PostgreSQL:    localhost:$Port" -ForegroundColor White
    Write-Host " Log level:     $(if ($env:RUST_LOG) { $env:RUST_LOG } else { 'info (default)' })" -ForegroundColor White
    Write-Host " Test account:  test / test" -ForegroundColor White
    Write-Host "=============================================" -ForegroundColor Green
    Write-Host ""
    Write-Host " To connect a game client:" -ForegroundColor Gray
    Write-Host "   1. Run: pwsh patch-client.ps1" -ForegroundColor White
    Write-Host "   2. Launch the game client" -ForegroundColor Gray
    Write-Host "   3. Login with: test / test" -ForegroundColor Gray
    Write-Host ""
    Write-Host " Press Ctrl+C to stop the server" -ForegroundColor Gray
    Write-Host "=============================================" -ForegroundColor Green

    # Block until the server exits or the user presses Ctrl+C.
    # PowerShell's CancelKeyPress scriptblock handler crashes on Linux signal threads
    # (no runspace available), so we use a compiled C# handler that sets a thread-safe flag.
    Add-Type -TypeDefinition @"
using System;
using System.Threading;
public static class CtrlCHandler {
    private static int _pressed = 0;
    private static ConsoleCancelEventHandler _handler;
    public static bool Pressed => Volatile.Read(ref _pressed) != 0;
    public static void Register() {
        _pressed = 0;
        _handler = (s, e) => { e.Cancel = true; Volatile.Write(ref _pressed, 1); };
        Console.CancelKeyPress += _handler;
    }
    public static void Unregister() {
        if (_handler != null) { Console.CancelKeyPress -= _handler; _handler = null; }
    }
}
"@ -ErrorAction SilentlyContinue

    [CtrlCHandler]::Register()

    try {
        # Poll until the server exits or Ctrl+C is pressed
        while (-not [CtrlCHandler]::Pressed) {
            $proc = Get-Process -Id $serverProc.Id -ErrorAction SilentlyContinue
            if (-not $proc -or $proc.HasExited) {
                Write-Host ""
                Write-Status "Server exited (code $($serverProc.ExitCode))." "Yellow"
                break
            }
            Start-Sleep -Milliseconds 500
        }

        if ([CtrlCHandler]::Pressed) {
            Write-Host ""
            Write-Host "Ctrl+C received — stopping server gracefully..." -ForegroundColor Yellow
            Stop-ServerGracefully $serverProc.Id
        }
    } finally {
        [CtrlCHandler]::Unregister()

        # Safety net: ensure the process is dead
        $proc = Get-Process -Id $serverProc.Id -ErrorAction SilentlyContinue
        if ($proc -and -not $proc.HasExited) {
            Stop-ServerGracefully $serverProc.Id
        }
    }
}

function Stop-ServerGracefully {
    <#
    .SYNOPSIS
        Sends a graceful shutdown signal to the server process and waits for it to exit.
        Uses SIGTERM on Linux/macOS (handled by tokio::signal::ctrl_c) and
        CTRL_C_EVENT via taskkill on Windows.
    #>
    param([int]$ProcessId)

    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    $proc = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
    if (-not $proc -or $proc.HasExited) { return }

    if ($isWin) {
        # Send CTRL_C_EVENT via taskkill (not /F which is SIGKILL)
        & taskkill /PID $ProcessId 2>&1 | Out-Null
    } else {
        # SIGTERM — tokio::signal::ctrl_c() catches this on Unix
        & kill -TERM $ProcessId 2>&1 | Out-Null
    }

    # Wait up to 10 seconds for graceful shutdown
    $deadline = (Get-Date).AddSeconds(10)
    while ((Get-Date) -lt $deadline) {
        $proc = Get-Process -Id $ProcessId -ErrorAction SilentlyContinue
        if (-not $proc -or $proc.HasExited) {
            Write-Status "Server stopped gracefully." "Green"
            return
        }
        Start-Sleep -Milliseconds 250
    }

    # Forceful fallback
    Write-Status "Server did not exit in time — force killing..." "Yellow"
    Stop-Process -Id $ProcessId -Force -ErrorAction SilentlyContinue
}
