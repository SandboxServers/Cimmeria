function Stop-CimmeriaServer {
    <#
    .SYNOPSIS
        Gracefully shuts down all Cimmeria server processes and PostgreSQL.

    .DESCRIPTION
        Stops game servers in reverse order (CellApp, BaseApp, AuthenticationServer),
        then stops the local PostgreSQL instance.

    .EXAMPLE
        Stop-CimmeriaServer
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param()

    $paths = Get-ProjectPaths

    Write-Step "STOPPING CIMMERIA SERVER"

    # Stop game servers in reverse order
    $serverNames = @("CellApp*", "BaseApp*", "AuthenticationServer*")
    foreach ($pattern in $serverNames) {
        $procs = Get-Process -Name $pattern -ErrorAction SilentlyContinue
        if ($procs) {
            foreach ($proc in $procs) {
                if ($PSCmdlet.ShouldProcess($proc.ProcessName, "Stop process (PID $($proc.Id))")) {
                    Write-Status "Stopping $($proc.ProcessName) (PID $($proc.Id))..." "White"
                    Stop-Process -Id $proc.Id -Force -ErrorAction SilentlyContinue
                }
            }
        } else {
            Write-Status "$($pattern): not running" "DarkGray"
        }
    }

    # Stop PostgreSQL
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

    Write-Host ""
    Write-Host "All Cimmeria services stopped." -ForegroundColor Green
}
