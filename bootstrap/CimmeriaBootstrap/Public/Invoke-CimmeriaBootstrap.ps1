function Invoke-CimmeriaBootstrap {
    <#
    .SYNOPSIS
        Runs the full Cimmeria bootstrap pipeline: download, build, database, launch.

    .DESCRIPTION
        Orchestrates the complete bootstrap sequence with fail-fast semantics:

        Step 1: Install-CimmeriaDependencies   (download, extract, patch)
        Step 2: Build-CimmeriaLibraries         (Boost, OpenSSL, SOCI)
        Step 3: Build-CimmeriaSolution          (MSBuild W-NG.sln)
        Step 4: Build-ServerEd                  (qmake + nmake ServerEd)
        Step 5: Initialize-CimmeriaDatabase     (PostgreSQL + schema)
        Step 6: Initialize-CimmeriaRuntime      (stage DLLs)
        Step 7: Start-CimmeriaServer            (launch Auth, Base, Cell)

        Each step is idempotent. Safe to re-run - completed work is skipped.
        On failure, the pipeline aborts with a clear error message.

    .PARAMETER SkipDownload
        Skip downloading dependency archives (use cached files).

    .PARAMETER SkipBuild
        Skip building libraries and the solution.

    .PARAMETER InstallVS
        Auto-install VS Community if not found.

    .PARAMETER NoLaunch
        Stop after setup - do not launch the servers.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .EXAMPLE
        Invoke-CimmeriaBootstrap
        # Full pipeline: download -> build -> database -> launch

    .EXAMPLE
        Invoke-CimmeriaBootstrap -SkipDownload -NoLaunch
        # Re-build and set up database without downloading or launching
    #>
    [CmdletBinding()]
    param(
        [switch]$SkipDownload,
        [switch]$SkipBuild,
        [switch]$InstallVS,
        [switch]$NoLaunch,
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug"
    )

    $ErrorActionPreference = "Stop"
    $pipelineStart = Get-Date

    # Register cleanup handler for Ctrl+C / exit
    $cleanupEvent = Register-EngineEvent PowerShell.Exiting -Action {
        Stop-CimmeriaServer -ErrorAction SilentlyContinue
    }

    try {
        Write-Host ""
        Write-Host "=============================================" -ForegroundColor Yellow
        Write-Host " Cimmeria Full Bootstrap Pipeline" -ForegroundColor Yellow
        Write-Host " Configuration: $Configuration" -ForegroundColor Yellow
        Write-Host "=============================================" -ForegroundColor Yellow
        Write-Host ""

        # Step 1: Download, extract, patch
        Write-Host "--- Step 1/7: Dependencies ---" -ForegroundColor Cyan
        try {
            Install-CimmeriaDependencies -SkipDownload:$SkipDownload -InstallVS:$InstallVS
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step 1: Install-CimmeriaDependencies" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        if (-not $SkipBuild) {
            # Step 2: Build libraries
            Write-Host ""
            Write-Host "--- Step 2/7: Build Libraries ---" -ForegroundColor Cyan
            try {
                Build-CimmeriaLibraries
            } catch {
                Write-Host ""
                Write-Host "FAILED at Step 2: Build-CimmeriaLibraries" -ForegroundColor Red
                Write-Host "  $_" -ForegroundColor Red
                throw
            }

            # Step 3: Build solution
            Write-Host ""
            Write-Host "--- Step 3/7: Build Solution ---" -ForegroundColor Cyan
            try {
                Build-CimmeriaSolution -Configuration $Configuration
            } catch {
                Write-Host ""
                Write-Host "FAILED at Step 3: Build-CimmeriaSolution" -ForegroundColor Red
                Write-Host "  $_" -ForegroundColor Red
                throw
            }
            # Step 4: Build ServerEd (non-fatal - optional tool)
            Write-Host ""
            Write-Host "--- Step 4/7: Build ServerEd ---" -ForegroundColor Cyan
            try {
                Build-ServerEd -Configuration $Configuration
            } catch {
                Write-Host ""
                Write-Host "WARNING: Step 4 (Build-ServerEd) failed - continuing" -ForegroundColor Yellow
                Write-Host "  $_" -ForegroundColor Yellow
                Write-Host "  ServerEd is optional. The game servers will work without it." -ForegroundColor DarkGray
            }
        } else {
            Write-Host ""
            Write-Host "--- Steps 2-4: Skipped (-SkipBuild) ---" -ForegroundColor DarkGray
        }

        # Step 5: Database
        Write-Host ""
        Write-Host "--- Step 5/7: Database ---" -ForegroundColor Cyan
        try {
            Initialize-CimmeriaDatabase
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step 5: Initialize-CimmeriaDatabase" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        # Step 6: Runtime DLLs
        Write-Host ""
        Write-Host "--- Step 6/7: Runtime Setup ---" -ForegroundColor Cyan
        try {
            Initialize-CimmeriaRuntime -Configuration $Configuration
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step 6: Initialize-CimmeriaRuntime" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        # Step 7: Launch
        if (-not $NoLaunch) {
            Write-Host ""
            Write-Host "--- Step 7/7: Launch ---" -ForegroundColor Cyan
            try {
                Start-CimmeriaServer -Configuration $Configuration
            } catch {
                Write-Host ""
                Write-Host "FAILED at Step 7: Start-CimmeriaServer" -ForegroundColor Red
                Write-Host "  $_" -ForegroundColor Red
                throw
            }
        } else {
            Write-Host ""
            Write-Host "--- Step 7: Skipped (-NoLaunch) ---" -ForegroundColor DarkGray
        }

        $elapsed = (Get-Date) - $pipelineStart
        Write-Host ""
        Write-Host "=============================================" -ForegroundColor Green
        Write-Host " Bootstrap Complete ($("{0:mm\:ss}" -f $elapsed))" -ForegroundColor Green
        Write-Host "=============================================" -ForegroundColor Green

    } catch {
        $elapsed = (Get-Date) - $pipelineStart
        Write-Host ""
        Write-Host "=============================================" -ForegroundColor Red
        Write-Host " Bootstrap Failed ($("{0:mm\:ss}" -f $elapsed))" -ForegroundColor Red
        Write-Host " $_" -ForegroundColor Red
        Write-Host "=============================================" -ForegroundColor Red
        $host.SetShouldExit(1)
        throw
    } finally {
        Unregister-Event -SourceIdentifier $cleanupEvent.Name -ErrorAction SilentlyContinue
    }
}
