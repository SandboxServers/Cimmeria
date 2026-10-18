function Initialize-CimmeriaDatabase {
    <#
    .SYNOPSIS
        Initializes a local PostgreSQL instance and loads the Cimmeria database schemas.

    .DESCRIPTION
        On Windows: uses PostgreSQL binaries from external/postgresql_server/ to
        initialize pgdata, start PG on port 5433, and load schemas.

        On Linux/macOS: assumes PostgreSQL is already running (via systemd/brew)
        and loads schemas into the configured database.

        Docker mode: uses a Docker container for PostgreSQL instead of local binaries.

    .PARAMETER Port
        PostgreSQL port. Default 5433.

    .PARAMETER Force
        Drop and recreate the sgw database before loading schemas.

    .PARAMETER WipeDatabase
        Prints player counts, backs up the database to server/backups, drops it and
        reloads it from the seeds. Destroys player data (after the backup). Not with -Force.
    .PARAMETER ResetDatabase
        Nuclear option: stop PostgreSQL, delete pgdata entirely, re-initialize from scratch.
        On Docker mode, removes and recreates the container.

    .PARAMETER UseDocker
        Use Docker for PostgreSQL instead of local binaries.

    .EXAMPLE
        Initialize-CimmeriaDatabase

    .EXAMPLE
        Initialize-CimmeriaDatabase -Force

    .EXAMPLE
        Initialize-CimmeriaDatabase -ResetDatabase

    .EXAMPLE
        Initialize-CimmeriaDatabase -UseDocker
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [int]$Port = 5433,
        [switch]$Force,
        [switch]$ResetDatabase,
        [switch]$UseDocker,
        [switch]$WipeDatabase
    )

    # Routing errors and the reset counts come before any docker or psql work.
    if ((Resolve-CimmeriaDatabaseAction -Force:$Force -Wipe:$WipeDatabase) -eq 'Throw') {
        throw "-Force and -WipeDatabase cannot be combined: -Force keeps player data, -WipeDatabase destroys it."
    }
    if ($ResetDatabase) {
        # Counts first, while the old data can still be reached. Best effort.
        try { Invoke-CimmeriaDatabaseWipe -PsqlExe (Find-PsqlExecutable -Port $Port -UseDocker:$UseDocker) -Port $Port -UseDocker:$UseDocker -CountsOnly } catch { }
    }

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    $exeSuffix = if ($isWin) { ".exe" } else { "" }

    Write-Step "INITIALIZING DATABASE"

    # ── Docker mode ───────────────────────────────────────────────────────────
    if ($UseDocker) {
        Initialize-DockerPostgreSQL -Port $Port -ResetDatabase:$ResetDatabase
        $psqlExe = Find-PsqlExecutable -Port $Port -UseDocker
        Invoke-SchemaLoad -PsqlExe $psqlExe -Port $Port -Force:$Force -UseDocker -WipeDatabase:$WipeDatabase
        return
    }

    # ── Local mode ────────────────────────────────────────────────────────────
    # Determine whether to manage a private PG instance or use an external one.
    # On Windows: always managed (PG binaries are downloaded by Install-CimmeriaDependencies).
    # On Linux/macOS: managed if server/pgdata/ exists or no external PG is reachable;
    #                 uses external PG (systemd, brew services) if already running.
    $pgDataDir = Join-Path $paths.ServerDir "pgdata"
    $managePg = $false

    if ($isWin) {
        $managePg = $true
    } elseif (Test-Path $pgDataDir) {
        # We have an existing managed pgdata directory — keep using it
        $managePg = $true
    } elseif (-not (Wait-ForPort -Port $Port -TimeoutSeconds 2)) {
        # No PG running externally — create a managed instance
        $managePg = $true
    }
    # else: external PG is running and no managed pgdata — use it

    if ($managePg) {
        # ── Managed PostgreSQL instance ───────────────────────────────────
        $pgBin = Find-PostgreSQL
        if (-not $pgBin) {
            if ($isWin) {
                throw "PostgreSQL server binaries not found. Run Install-CimmeriaDependencies first."
            } else {
                throw "PostgreSQL is not running on port $Port and pg_ctl was not found.`n  Install: sudo apt install postgresql-17`n  Or use:  pwsh setup.ps1 -UseDocker"
            }
        }

        $pgLogDir = Join-Path $paths.ServerDir "logs"
        $pgLogFile = Join-Path $pgLogDir "postgresql.log"
        New-Item -ItemType Directory -Path $pgLogDir -Force | Out-Null

        # ── Reset: nuke pgdata entirely ───────────────────────────────────
        if ($ResetDatabase) {
            if ($PSCmdlet.ShouldProcess("PostgreSQL data directory", "Stop server and delete pgdata")) {
                $pgCtl = Join-Path $pgBin "pg_ctl$exeSuffix"
                $statusResult = & $pgCtl status -D $pgDataDir 2>&1
                if ($LASTEXITCODE -eq 0) {
                    Write-Status "Stopping PostgreSQL for reset..." "Yellow"
                    & $pgCtl stop -D $pgDataDir -m immediate 2>&1 | Out-Null
                    Start-Sleep -Seconds 2
                }

                if (Test-Path $pgDataDir) {
                    Write-Status "Deleting pgdata directory..." "Yellow"
                    Remove-Item -Path $pgDataDir -Recurse -Force
                    Write-Status "pgdata deleted." "Yellow"
                }
            }
        }

        # ── Validate pgdata version ───────────────────────────────────────
        $pgVersionFile = Join-Path $pgDataDir "PG_VERSION"
        if (Test-Path $pgVersionFile) {
            $pgDataVersion = (Get-Content $pgVersionFile -Raw).Trim()

            # Get installed PG major version
            $postgresExe = Join-Path $pgBin "postgres$exeSuffix"
            $installedVersionRaw = & $postgresExe --version 2>&1
            $installedMajor = if ($installedVersionRaw -match '(\d+)') { $Matches[1] } else { "" }

            if ($pgDataVersion -and $installedMajor -and ($pgDataVersion -ne $installedMajor)) {
                Write-Status "WARNING: pgdata version ($pgDataVersion) does not match installed PostgreSQL ($installedMajor)." "Red"
                Write-Status "  This will cause startup failures." "Red"
                Write-Status "  Use -ResetDatabase to reinitialize pgdata for the current version." "Yellow"
                throw "pgdata version mismatch: pgdata=$pgDataVersion, installed=$installedMajor. Use -ResetDatabase to fix."
            } elseif ($pgDataVersion -and $installedMajor) {
                Write-Status "pgdata version: $pgDataVersion (matches installed PG $installedMajor)" "DarkGray"
            }
        }

        # ── Step 1: initdb ────────────────────────────────────────────────
        if (-not (Test-Path $pgVersionFile)) {
            if ($PSCmdlet.ShouldProcess("server/pgdata", "Initialize PostgreSQL data directory")) {
                Write-Status "Running initdb..." "White"
                $initdb = Join-Path $pgBin "initdb$exeSuffix"
                & $initdb -D $pgDataDir -U postgres -A trust -E UTF8 2>&1 | ForEach-Object {
                    if ($_ -match 'Success|creating|copying') { Write-Status "  $_" "DarkGray" }
                }
                if ($LASTEXITCODE -ne 0) {
                    throw "initdb failed with exit code $LASTEXITCODE."
                }
                Write-Status "initdb complete." "Green"
            }
        } else {
            Write-Status "PostgreSQL data directory already initialized." "DarkGray"
        }

        # ── Step 2: Start PostgreSQL ──────────────────────────────────────
        $pgCtl = Join-Path $pgBin "pg_ctl$exeSuffix"
        $statusResult = & $pgCtl status -D $pgDataDir 2>&1
        $pgAlreadyRunning = ($LASTEXITCODE -eq 0) -or (Wait-ForPort -Port $Port -TimeoutSeconds 1)
        if ($pgAlreadyRunning) {
            Write-Status "PostgreSQL already running." "DarkGray"
        } else {
            if ($PSCmdlet.ShouldProcess("PostgreSQL on port $Port", "Start database server")) {
                Write-Status "Starting PostgreSQL on port $Port..." "White"
                if ($isWin) {
                    $pgCtlArgs = "start -D `"$pgDataDir`" -l `"$pgLogFile`" -o `"-p $Port`""
                    Start-Process -FilePath $pgCtl -ArgumentList $pgCtlArgs -WindowStyle Hidden
                } else {
                    & $pgCtl start -D $pgDataDir -l $pgLogFile -o "-p $Port" 2>&1 | ForEach-Object {
                        Write-Status "  $_" "DarkGray"
                    }
                }
                if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 15)) {
                    Write-Status "PostgreSQL failed to start. Check $pgLogFile" "Red"
                    if (Test-Path $pgLogFile) {
                        Get-Content $pgLogFile -Tail 10 | ForEach-Object { Write-Status "  $_" "Red" }
                    }
                    throw "PostgreSQL did not start within 15 seconds."
                }
                Write-Status "PostgreSQL started on port $Port." "Green"
            }
        }

        $psqlExe = Join-Path $pgBin "psql$exeSuffix"
        if (-not (Test-Path $psqlExe)) {
            # On some Linux installs, psql may be in a different location than pg_ctl
            $psqlCmd = Get-Command psql -ErrorAction SilentlyContinue
            if ($psqlCmd) { $psqlExe = $psqlCmd.Source }
            else { throw "psql not found in PATH or PostgreSQL bin directory." }
        }
    } else {
        # ── External PostgreSQL (systemd, brew services, etc.) ────────────
        Write-Status "PostgreSQL reachable on port $Port." "DarkGray"

        # Force TCP connection (required for Docker — no Unix socket on non-default port)
        $env:PGHOST = "localhost"

        $psqlCmd = Get-Command psql -ErrorAction SilentlyContinue
        if (-not $psqlCmd) {
            throw "psql not found in PATH. Install PostgreSQL client tools."
        }
        $psqlExe = $psqlCmd.Source
    }

    Invoke-SchemaLoad -PsqlExe $psqlExe -Port $Port -Force:$Force -WipeDatabase:$WipeDatabase
}

# ─── Docker PostgreSQL ──────────────────────────────────────────────────────

function Initialize-DockerPostgreSQL {
    <#
    .SYNOPSIS
        Ensures a PostgreSQL Docker container is running for Cimmeria.
    #>
    param(
        [int]$Port = 5433,
        [switch]$ResetDatabase
    )

    $containerName = "cimmeria-postgres"

    # Verify Docker is available
    $docker = Get-Command docker -ErrorAction SilentlyContinue
    if (-not $docker) {
        throw "Docker not found in PATH. Install Docker Desktop or Docker Engine first."
    }

    # Check if Docker daemon is running
    $dockerInfo = & docker info 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Docker daemon is not running. Start Docker Desktop or the Docker service."
    }

    Write-Status "Docker is available." "Green"

    # Reset: remove existing container and volume
    $volumeName = "cimmeria-pgdata"
    if ($ResetDatabase) {
        Write-Status "Removing existing container and volume..." "Yellow"
        & docker rm -f $containerName 2>&1 | Out-Null
        & docker volume rm $volumeName 2>&1 | Out-Null
        Write-Status "Container and volume removed." "Yellow"
    }

    # Check if container exists and is running
    $containerStatus = & docker inspect --format '{{.State.Status}}' $containerName 2>&1
    if ($LASTEXITCODE -eq 0) {
        if ($containerStatus -eq "running") {
            Write-Status "Container '$containerName' is already running on port $Port." "DarkGray"
        } else {
            Write-Status "Starting existing container '$containerName'..." "White"
            & docker start $containerName 2>&1 | Out-Null
            if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 15)) {
                throw "PostgreSQL container did not become ready within 15 seconds."
            }
            Write-Status "Container started." "Green"
        }
    } else {
        # Create and start new container with a named volume for data persistence
        Write-Status "Creating PostgreSQL container '$containerName' on port $Port..." "White"
        & docker run -d `
            --name $containerName `
            -v "${volumeName}:/var/lib/postgresql/data" `
            -e POSTGRES_USER=postgres `
            -e POSTGRES_PASSWORD=postgres `
            -e POSTGRES_HOST_AUTH_METHOD=trust `
            -p "${Port}:5432" `
            postgres:17 2>&1 | ForEach-Object {
            Write-Status "  $_" "DarkGray"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to create PostgreSQL Docker container."
        }

        if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 30)) {
            Write-Status "Container logs:" "Red"
            & docker logs --tail 10 $containerName 2>&1 | ForEach-Object { Write-Status "  $_" "Red" }
            throw "PostgreSQL container did not become ready within 30 seconds."
        }
        Write-Status "PostgreSQL container created and running on port $Port." "Green"
    }
}

function Find-PsqlExecutable {
    <#
    .SYNOPSIS
        Finds psql for use with Docker or local PostgreSQL.
    #>
    param(
        [int]$Port = 5433,
        [switch]$UseDocker
    )

    # Try local psql first
    $psqlCmd = Get-Command psql -ErrorAction SilentlyContinue
    if ($psqlCmd) {
        return $psqlCmd.Source
    }

    # Try the downloaded PostgreSQL binaries
    $pgBin = Find-PostgreSQL
    if ($pgBin) {
        $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
        $exeSuffix = if ($isWin) { ".exe" } else { "" }
        $psqlPath = Join-Path $pgBin "psql$exeSuffix"
        if (Test-Path $psqlPath) {
            return $psqlPath
        }
    }

    if ($UseDocker) {
        throw "psql not found. Install PostgreSQL client tools (psql) to connect to the Docker container.`n  On Windows: the bootstrap downloads PG binaries automatically — run Install-CimmeriaDependencies.`n  On Linux:   sudo apt install postgresql-client`n  On macOS:   brew install libpq"
    }

    throw "psql not found in PATH."
}

# ─── SQL Include Resolver ────────────────────────────────────────────────────

function Resolve-SqlIncludes {
    <#
    .SYNOPSIS
        Resolves psql \ir (include-relative) directives into a single concatenated SQL file.
        This avoids per-file round-trips when loading through Docker or remote connections.
    #>
    param(
        [Parameter(Mandatory)][string]$SqlFile
    )

    $baseDir = Split-Path $SqlFile -Parent
    $outputFile = Join-Path ([System.IO.Path]::GetTempPath()) "cimmeria-database-resolved.sql"
    $writer = [System.IO.StreamWriter]::new($outputFile, $false, [System.Text.Encoding]::UTF8)

    try {
        $writer.WriteLine("-- Auto-resolved from: $SqlFile")
        $writer.WriteLine("-- Generated at: $(Get-Date -Format 'yyyy-MM-dd HH:mm:ss')")
        $writer.WriteLine("")
        $writer.WriteLine("BEGIN;")
        $writer.WriteLine("")

        Expand-SqlFile $SqlFile $baseDir $writer

        $writer.WriteLine("")
        $writer.WriteLine("COMMIT;")
    } finally {
        $writer.Close()
    }

    return $outputFile
}

function Expand-SqlFile {
    <#
    .SYNOPSIS
        Recursively expands a SQL file, inlining all \ir and \i directives.
    #>
    param(
        [string]$FilePath,
        [string]$BaseDir,
        [System.IO.StreamWriter]$Writer
    )

    foreach ($line in [System.IO.File]::ReadLines($FilePath)) {
        if ($line -match '^\s*\\ir\s+(.+)$') {
            # \ir = include relative to the current file's directory
            $includeDir = Split-Path $FilePath -Parent
            $includePath = Join-Path $includeDir $Matches[1].Trim()
            if (Test-Path $includePath) {
                $Writer.WriteLine("-- \\ir $($Matches[1].Trim())")
                Expand-SqlFile $includePath $BaseDir $Writer
            } else {
                $Writer.WriteLine("-- WARNING: include not found: $includePath")
            }
        } elseif ($line -match '^\s*\\i\s+(.+)$') {
            # \i = include relative to the base directory
            $includePath = Join-Path $BaseDir $Matches[1].Trim()
            if (Test-Path $includePath) {
                $Writer.WriteLine("-- \\i $($Matches[1].Trim())")
                Expand-SqlFile $includePath $BaseDir $Writer
            } else {
                $Writer.WriteLine("-- WARNING: include not found: $includePath")
            }
        } else {
            $Writer.WriteLine($line)
        }
    }
}

# ─── Schema Loading (shared by local and Docker modes) ──────────────────────

function Invoke-SchemaLoad {
    <#
    .SYNOPSIS
        Creates the database role, database, and loads schemas/seed data.
        Checks for existing data to avoid duplicate inserts.
    #>
    param(
        [string]$PsqlExe,
        [int]$Port = 5433,
        [switch]$Force,
        [switch]$UseDocker,
        [switch]$WipeDatabase
    )

    $paths = Get-ProjectPaths
    $pgHost = if ($UseDocker) { "localhost" } else { $null }
    $hostArgs = if ($pgHost) { @("-h", $pgHost) } else { @() }

    # Wait for PostgreSQL to accept queries
    Write-Status "Waiting for PostgreSQL to accept queries..." "DarkGray"
    $ready = $false
    for ($i = 0; $i -lt 20; $i++) {
        $check = & $PsqlExe @hostArgs -p $Port -U postgres -tAc "SELECT 1" 2>$null
        if ($check -match '1') { $ready = $true; break }
        Start-Sleep -Seconds 1
    }
    if (-not $ready) {
        throw "PostgreSQL is not accepting queries after 20 seconds."
    }

    # Ensure role exists
    Write-Status "Ensuring w-testing role exists..." "DarkGray"
    & $PsqlExe @hostArgs -p $Port -U postgres -c "CREATE ROLE `"w-testing`" WITH LOGIN PASSWORD 'w-testing'" 2>&1 | ForEach-Object {
        if ($_ -notmatch 'already exists') { Write-Status "  $_" "DarkGray" }
    }
    & $PsqlExe @hostArgs -p $Port -U postgres -c "ALTER ROLE `"w-testing`" WITH LOGIN CREATEDB PASSWORD 'w-testing'" 2>&1 | Out-Null

    # Wipe mode: counts, backup and drop; the create step below rebuilds 'sgw' empty
    if ($WipeDatabase) {
        Invoke-CimmeriaDatabaseWipe -PsqlExe $PsqlExe -Port $Port -UseDocker:$UseDocker | Out-Null
    }

    # Check for existing database
    Write-Status "Checking for existing database..." "DarkGray"
    $dbExists = (& $PsqlExe @hostArgs -p $Port -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='sgw'" 2>$null) -match '1'

    if ($dbExists) {
        Write-Status "Database 'sgw' already exists." "DarkGray"
        & $PsqlExe @hostArgs -p $Port -U postgres -c "ALTER DATABASE sgw OWNER TO `"w-testing`"" 2>&1 | Out-Null
        & $PsqlExe @hostArgs -p $Port -U postgres -d sgw -c "GRANT ALL ON DATABASE sgw TO `"w-testing`"; GRANT ALL ON SCHEMA public TO `"w-testing`"" 2>&1 | Out-Null
    } else {
        Write-Status "Creating database..." "White"
        & $PsqlExe @hostArgs -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" 2>&1 | ForEach-Object {
            Write-Status "  $_" "DarkGray"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to create database 'sgw'."
        }
        Write-Status "Database 'sgw' created." "Green"
    }

    # ── Check existing schema and data, then route (see Resolve-CimmeriaDatabaseAction) ──
    $state = Get-CimmeriaDatabaseState -PsqlExe $PsqlExe -Port $Port -UseDocker:$UseDocker
    $dbAction = Resolve-CimmeriaDatabaseAction -Force:$Force -Wipe:$WipeDatabase -SchemaExists:$state.SchemaExists -SeedComplete:$state.SeedComplete -HasAccounts:$state.HasAccounts

    # ── Load schemas and seed data ────────────────────────────────────────
    $databaseSql = Join-Path $paths.DbDir "database.sql"

    if ($dbAction -eq 'Skip') {
        Write-Status "Database is fully initialized. Use -Force to refresh resources, -WipeDatabase to reload, or -ResetDatabase for a full reset." "DarkGray"
    } elseif ($dbAction -eq 'Refresh') {
        Invoke-CimmeriaResourcesRefresh -PsqlExe $PsqlExe -Port $Port -UseDocker:$UseDocker | Out-Null
    } elseif ($dbAction -eq 'Replay') {
        Write-Status "Schema exists but seed data is incomplete." "Yellow"
        Write-Status "  Attempting to reload database.sql (may show constraint errors for existing objects)..." "Yellow"
        $resolvedSql = Resolve-SqlIncludes $databaseSql
        & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -f $resolvedSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                # Only show errors that aren't "already exists" — those are expected
                if ($_ -notmatch 'already exists|duplicate key') {
                    Write-Status "  $_" "Red"
                }
            }
        }
        Remove-Item $resolvedSql -ErrorAction SilentlyContinue
        Write-Status "Schema reload attempted." "Yellow"
    } else {
        if ($Force -and $dbExists) {
            # No public.account, so no player rows can exist: recreate the empty database
            Write-Status "Force: 'sgw' has no schema; dropping and recreating it." "Yellow"
            & $PsqlExe @hostArgs -p $Port -U postgres -c "DROP DATABASE IF EXISTS sgw WITH (FORCE)" -c "CREATE DATABASE sgw OWNER `"w-testing`"" | Out-Null
        }
        Write-Status "Loading database schema and seed data..." "White"
        $resolvedSql = Resolve-SqlIncludes $databaseSql
        $resolvedSize = Format-FileSize (Get-Item $resolvedSql).Length
        $includeCount = (Select-String -Path $databaseSql -Pattern '\ir' -SimpleMatch).Count
        Write-Status "  Resolved $resolvedSize from $includeCount includes into single file" "DarkGray"
        & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $resolvedSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            }
        }
        $loadResult = $LASTEXITCODE
        Remove-Item $resolvedSql -ErrorAction SilentlyContinue
        if ($loadResult -ne 0) {
            throw "database.sql failed (exit code $loadResult)."
        }
        Write-Status "Database loaded." "Green"
    }

    # ── Verify key tables ─────────────────────────────────────────────────
    Write-Status "Verifying schema..." "White"
    $verifyTables = @("account", "sgw_player", "shards")
    $allGood = $true
    foreach ($table in $verifyTables) {
        $check = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='$table'" 2>$null
        if ($check -match '1') {
            Write-Status "  [OK] $table" "Green"
        } else {
            Write-Status "  [--] $table" "Red"
            $allGood = $false
        }
    }

    # Also verify resources schema
    $resourcesCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT count(*)::int FROM information_schema.tables WHERE table_schema='resources'" 2>$null
    $resourcesCount = if ($resourcesCheck -match '(\d+)') { [int]$Matches[1] } else { 0 }
    if ($resourcesCount -gt 0) {
        Write-Status "  [OK] resources schema ($resourcesCount tables)" "Green"
    } else {
        Write-Status "  [--] resources schema (no tables)" "Red"
        $allGood = $false
    }

    if (-not $allGood) {
        throw "Schema verification failed - some tables are missing."
    }

    Write-Status "Initialize-CimmeriaDatabase complete." "Green"
    Write-Status "  PostgreSQL: localhost:$Port" "DarkGray"
    Write-Status "  Database:   sgw" "DarkGray"
    Write-Status "  Role:       w-testing" "DarkGray"
    Write-Status "  Test login: test / test (SHA1 hashed)" "DarkGray"
}
