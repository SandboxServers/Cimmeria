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
        [switch]$UseDocker
    )

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    $exeSuffix = if ($isWin) { ".exe" } else { "" }

    Write-Step "INITIALIZING DATABASE"

    # ── Docker mode ───────────────────────────────────────────────────────────
    if ($UseDocker) {
        Initialize-DockerPostgreSQL -Port $Port -ResetDatabase:$ResetDatabase
        $psqlExe = Find-PsqlExecutable -Port $Port -UseDocker
        Invoke-SchemaLoad -PsqlExe $psqlExe -Port $Port -Force:$Force -UseDocker
        return
    }

    # ── Local mode ────────────────────────────────────────────────────────────
    if ($isWin) {
        # Windows: managed PostgreSQL instance
        $pgBin = Find-PostgreSQL
        if (-not $pgBin) {
            throw "PostgreSQL server binaries not found. Run Install-CimmeriaDependencies first."
        }

        $pgDataDir = Join-Path $paths.ServerDir "pgdata"
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
                $pgCtlArgs = "start -D `"$pgDataDir`" -l `"$pgLogFile`" -o `"-p $Port`""
                Start-Process -FilePath $pgCtl -ArgumentList $pgCtlArgs -WindowStyle Hidden
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
    } else {
        # Linux/macOS: verify PG is reachable, use psql from PATH
        if (-not (Wait-ForPort -Port $Port -TimeoutSeconds 2)) {
            throw "PostgreSQL is not reachable on port $Port. Start it first."
        }
        Write-Status "PostgreSQL reachable on port $Port." "DarkGray"

        $psqlCmd = Get-Command psql -ErrorAction SilentlyContinue
        if (-not $psqlCmd) {
            throw "psql not found in PATH. Install PostgreSQL client tools."
        }
        $psqlExe = $psqlCmd.Source
    }

    Invoke-SchemaLoad -PsqlExe $psqlExe -Port $Port -Force:$Force
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

    # Reset: remove existing container
    if ($ResetDatabase) {
        Write-Status "Removing existing container '$containerName'..." "Yellow"
        & docker rm -f $containerName 2>&1 | Out-Null
        Write-Status "Container removed." "Yellow"
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
        # Create and start new container
        Write-Status "Creating PostgreSQL container '$containerName' on port $Port..." "White"
        & docker run -d `
            --name $containerName `
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
        [switch]$UseDocker
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

    # ── Force mode: drop and recreate ─────────────────────────────────────
    if ($Force) {
        if ($PSCmdlet.ShouldProcess("database 'sgw'", "Drop and recreate")) {
            Write-Status "Force flag set - dropping and recreating database 'sgw'..." "Yellow"
            # Terminate existing connections before dropping
            & $PsqlExe @hostArgs -p $Port -U postgres -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='sgw' AND pid <> pg_backend_pid()" 2>&1 | Out-Null
            & $PsqlExe @hostArgs -p $Port -U postgres -c "DROP DATABASE IF EXISTS sgw" | Out-Null
            & $PsqlExe @hostArgs -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to recreate database 'sgw' under -Force."
            }
            Write-Status "Database 'sgw' recreated." "Green"
            $dbExists = $false
        }
    }

    # ── Check existing schema and data ────────────────────────────────────
    $schemaExists = $false
    $seedDataExists = $false

    if ($dbExists -or (-not $Force)) {
        # Check if schemas are loaded (look for key tables)
        $accountTableCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>$null
        $resourcesSchemaCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.schemata WHERE schema_name='resources'" 2>$null

        if (($accountTableCheck -match '1') -and ($resourcesSchemaCheck -match '1')) {
            $schemaExists = $true
            Write-Status "Schema exists (account table + resources schema found)." "DarkGray"

            # Check if seed data exists
            $testAccountCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM account WHERE account_id = 2" 2>$null
            $shardsCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM shards WHERE shard_id = 1" 2>$null
            $resourcesDataCheck = & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM resources.worlds LIMIT 1" 2>$null

            if (($testAccountCheck -match '1') -and ($shardsCheck -match '1') -and ($resourcesDataCheck -match '1')) {
                $seedDataExists = $true
                Write-Status "Seed data exists (test account, shards, resources data found)." "DarkGray"
            } else {
                Write-Status "Schema loaded but seed data appears incomplete." "Yellow"
            }
        }
    }

    # ── Load schemas and seed data ────────────────────────────────────────
    $databaseSql = Join-Path $paths.DbDir "database.sql"

    if ($schemaExists -and $seedDataExists) {
        Write-Status "Database is fully initialized. Use -Force to drop and reload, or -ResetDatabase for a full reset." "DarkGray"
    } elseif ($schemaExists -and -not $seedDataExists) {
        Write-Status "Schema exists but seed data is incomplete." "Yellow"
        Write-Status "  Attempting to reload database.sql (may show constraint errors for existing objects)..." "Yellow"
        & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -f $databaseSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                # Only show errors that aren't "already exists" — those are expected
                if ($_ -notmatch 'already exists|duplicate key') {
                    Write-Status "  $_" "Red"
                }
            }
        }
        Write-Status "Schema reload attempted." "Yellow"
    } else {
        Write-Status "Loading database.sql (resources + public schemas)..." "White"
        & $PsqlExe @hostArgs -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $databaseSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "database.sql failed (exit code $LASTEXITCODE)."
        }
        Write-Status "database.sql loaded." "Green"
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
