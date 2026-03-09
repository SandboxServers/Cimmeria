function Invoke-CimmeriaBootstrap {
    <#
    .SYNOPSIS
        Runs the full Cimmeria bootstrap pipeline: prerequisites, dependencies, build, database, launch.

    .DESCRIPTION
        Orchestrates the complete bootstrap sequence with fail-fast semantics:

        Step 1: Assert-CimmeriaPrerequisites    (Rust toolchain, optionally Node.js)
        Step 2: Install-CimmeriaDependencies     (PostgreSQL server binaries)
        Step 3: Build-CimmeriaServer             (cargo build --workspace)
        Step 4: Build admin panel                (npm install + cargo build) [if -WithAdmin]
        Step 5: Build launcher                   (cargo build sgw-launcher) [if -WithLauncher]
        Step N: Initialize-CimmeriaDatabase      (PostgreSQL + schema)
        Step N: Start-CimmeriaServer             (launch cimmeria-server)

        Each step is idempotent. Safe to re-run - completed work is skipped.
        On failure, the pipeline aborts with a clear error message.

    .PARAMETER SkipDownload
        Skip downloading PostgreSQL archives (use cached files).

    .PARAMETER SkipBuild
        Skip building the server and apps.

    .PARAMETER WithAdmin
        Also build the Tauri admin panel (requires Node.js).

    .PARAMETER WithLauncher
        Also build the SGW game launcher.

    .PARAMETER NoLaunch
        Stop after setup - do not launch the server.

    .PARAMETER Configuration
        Build configuration: "Debug" (default) or "Release".

    .PARAMETER ForceDatabase
        Drop and recreate the sgw database before loading schemas.

    .PARAMETER ResetDatabase
        Nuclear option: stop PostgreSQL, delete pgdata, re-initialize everything from scratch.

    .PARAMETER UseDocker
        Use a Docker container for PostgreSQL instead of local binaries.

    .EXAMPLE
        Invoke-CimmeriaBootstrap
        # Full pipeline: check prereqs -> download PG -> build server -> database -> launch

    .EXAMPLE
        Invoke-CimmeriaBootstrap -WithAdmin -WithLauncher
        # Build everything including admin panel and launcher

    .EXAMPLE
        Invoke-CimmeriaBootstrap -SkipBuild -ResetDatabase -NoLaunch
        # Wipe PostgreSQL and reload the database only
    #>
    [CmdletBinding()]
    param(
        [switch]$SkipDownload,
        [switch]$SkipBuild,
        [switch]$WithAdmin,
        [switch]$WithLauncher,
        [switch]$NoLaunch,
        [ValidateSet("Debug", "Release")]
        [string]$Configuration = "Debug",
        [switch]$ForceDatabase,
        [switch]$ResetDatabase,
        [switch]$UseDocker
    )

    $ErrorActionPreference = "Stop"
    $pipelineStart = Get-Date

    # Count steps dynamically
    $steps = @("Prerequisites", "Dependencies", "Build Server")
    if (-not $SkipBuild) {
        if ($WithAdmin) { $steps += "Build Admin Panel" }
        if ($WithLauncher) { $steps += "Build Launcher" }
    }
    $steps += "Database"
    if (-not $NoLaunch) { $steps += "Launch" }
    $totalSteps = $steps.Count

    # Register cleanup handler for Ctrl+C / exit
    $cleanupEvent = Register-EngineEvent PowerShell.Exiting -Action {
        Stop-CimmeriaServer -ErrorAction SilentlyContinue
    }

    try {
        Write-Host ""
        Write-Host "=============================================" -ForegroundColor Yellow
        Write-Host " Cimmeria Bootstrap Pipeline (Rust)" -ForegroundColor Yellow
        Write-Host " Configuration: $Configuration" -ForegroundColor Yellow
        if ($WithAdmin)    { Write-Host " + Admin Panel" -ForegroundColor Yellow }
        if ($WithLauncher) { Write-Host " + Game Launcher" -ForegroundColor Yellow }
        if ($UseDocker)    { Write-Host " + Docker PostgreSQL" -ForegroundColor Yellow }
        Write-Host "=============================================" -ForegroundColor Yellow
        Write-Host ""

        # Step 1: Prerequisites
        $step = 1
        Write-Host "--- Step $step/$totalSteps`: Prerequisites ---" -ForegroundColor Cyan
        try {
            Assert-CimmeriaPrerequisites -RequireNode:$WithAdmin
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step $step`: Assert-CimmeriaPrerequisites" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        # Step 2: Dependencies (PostgreSQL)
        $step++
        Write-Host ""
        Write-Host "--- Step $step/$totalSteps`: Dependencies ---" -ForegroundColor Cyan
        try {
            Install-CimmeriaDependencies -SkipDownload:$SkipDownload
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step $step`: Install-CimmeriaDependencies" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        if (-not $SkipBuild) {
            # Step 3: Build server
            $step++
            Write-Host ""
            Write-Host "--- Step $step/$totalSteps`: Build Server ---" -ForegroundColor Cyan
            try {
                Build-CimmeriaServer -Configuration $Configuration -WithAdmin:$WithAdmin -WithLauncher:$WithLauncher
            } catch {
                Write-Host ""
                Write-Host "FAILED at Step $step`: Build-CimmeriaServer" -ForegroundColor Red
                Write-Host "  $_" -ForegroundColor Red
                throw
            }

            # Step 4: Build admin panel (optional)
            if ($WithAdmin) {
                $step++
                Write-Host ""
                Write-Host "--- Step $step/$totalSteps`: Build Admin Panel ---" -ForegroundColor Cyan
                try {
                    Build-CimmeriaApp -Configuration $Configuration
                } catch {
                    Write-Host ""
                    Write-Host "WARNING: Step $step (Build-CimmeriaApp) failed - continuing" -ForegroundColor Yellow
                    Write-Host "  $_" -ForegroundColor Yellow
                    Write-Host "  The admin panel is optional. The game server will work without it." -ForegroundColor DarkGray
                }
            }

            # Step 5: Build launcher (optional)
            if ($WithLauncher) {
                $step++
                Write-Host ""
                Write-Host "--- Step $step/$totalSteps`: Build Launcher ---" -ForegroundColor Cyan
                try {
                    Build-CimmeriaLauncher -Configuration $Configuration
                } catch {
                    Write-Host ""
                    Write-Host "WARNING: Step $step (Build-CimmeriaLauncher) failed - continuing" -ForegroundColor Yellow
                    Write-Host "  $_" -ForegroundColor Yellow
                    Write-Host "  The launcher is optional. Use Update-CimmeriaClient instead." -ForegroundColor DarkGray
                }
            }
        } else {
            Write-Host ""
            Write-Host "--- Build steps: Skipped (-SkipBuild) ---" -ForegroundColor DarkGray
        }

        # Database
        $step++
        Write-Host ""
        Write-Host "--- Step $step/$totalSteps`: Database ---" -ForegroundColor Cyan
        try {
            Initialize-CimmeriaDatabase -Force:$ForceDatabase -ResetDatabase:$ResetDatabase -UseDocker:$UseDocker
        } catch {
            Write-Host ""
            Write-Host "FAILED at Step $step`: Initialize-CimmeriaDatabase" -ForegroundColor Red
            Write-Host "  $_" -ForegroundColor Red
            throw
        }

        # Launch
        if (-not $NoLaunch) {
            $step++
            Write-Host ""
            Write-Host "--- Step $step/$totalSteps`: Launch ---" -ForegroundColor Cyan
            try {
                Start-CimmeriaServer -Configuration $Configuration
            } catch {
                Write-Host ""
                Write-Host "FAILED at Step $step`: Start-CimmeriaServer" -ForegroundColor Red
                Write-Host "  $_" -ForegroundColor Red
                throw
            }
        } else {
            Write-Host ""
            Write-Host "--- Launch: Skipped (-NoLaunch) ---" -ForegroundColor DarkGray
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
