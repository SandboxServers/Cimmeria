function Stop-CimmeriaServer {
    <#
    .SYNOPSIS
        Stops the Cimmeria server process and (on Windows) the local PostgreSQL instance.

    .EXAMPLE
        Stop-CimmeriaServer
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param()

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))

    Write-Step "STOPPING CIMMERIA SERVER"

    # Stop cimmeria-server
    $procs = Get-Process -Name "cimmeria-server" -ErrorAction SilentlyContinue
    if ($procs) {
        foreach ($proc in $procs) {
            if ($PSCmdlet.ShouldProcess($proc.ProcessName, "Stop process (PID $($proc.Id))")) {
                Write-Status "Stopping cimmeria-server (PID $($proc.Id))..." "White"
                Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
            }
        }
    } else {
        Write-Status "cimmeria-server: not running" "DarkGray"
    }

    # Stop PostgreSQL (Windows managed instance only)
    if ($isWin) {
        $pgDataDir = Join-Path $paths.ServerDir "pgdata"
        if (Test-Path $pgDataDir) {
            $pgBin = Find-PostgreSQL
            if ($pgBin) {
                $pgCtl = Join-Path $pgBin "pg_ctl.exe"
                $statusResult = & $pgCtl status -D $pgDataDir 2>&1
                if ($LASTEXITCODE -eq 0) {
                    if ($PSCmdlet.ShouldProcess("PostgreSQL", "Stop database server")) {
                        Write-Status "Stopping PostgreSQL..." "White"
                        & $pgCtl stop -D $pgDataDir -m fast 2>&1 | ForEach-Object {
                            Write-Status "  $_" "DarkGray"
                        }
                        Write-Status "PostgreSQL stopped." "Green"
                    }
                } else {
                    Write-Status "PostgreSQL: not running" "DarkGray"
                }
            }
        }
    }

    Write-Host ""
    Write-Host "Cimmeria server stopped." -ForegroundColor Green
}
