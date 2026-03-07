function Initialize-CimmeriaDatabase {
    <#
    .SYNOPSIS
        Initializes a local PostgreSQL instance and loads the Cimmeria database schemas.

    .DESCRIPTION
        On Windows: uses PostgreSQL binaries from external/postgresql_server/ to
        initialize pgdata, start PG on port 5433, and load schemas.

        On Linux/macOS: assumes PostgreSQL is already running (via systemd/brew)
        and loads schemas into the configured database.

    .PARAMETER Port
        PostgreSQL port. Default 5433.

    .PARAMETER Force
        Drop and recreate the sgw database before loading schemas.

    .EXAMPLE
        Initialize-CimmeriaDatabase

    .EXAMPLE
        Initialize-CimmeriaDatabase -Force
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [int]$Port = 5433,
        [switch]$Force
    )

    $paths = Get-ProjectPaths
    $isWin = $IsWindows -or (-not (Test-Path variable:IsWindows))
    $exeSuffix = if ($isWin) { ".exe" } else { "" }

    Write-Step "INITIALIZING DATABASE"

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

        # Step 1: initdb
        if (-not (Test-Path (Join-Path $pgDataDir "PG_VERSION"))) {
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

        # Step 2: Start PostgreSQL
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
                        Get-Content $pgLogFile -Tail 5 | ForEach-Object { Write-Status "  $_" "Red" }
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

    # Step 3: Create role and database (cross-platform from here)
    Write-Status "Waiting for PostgreSQL to accept queries..." "DarkGray"
    $ready = $false
    for ($i = 0; $i -lt 15; $i++) {
        $check = & $psqlExe -p $Port -U postgres -tAc "SELECT 1" 2>$null
        if ($check -match '1') { $ready = $true; break }
        Start-Sleep -Seconds 1
    }
    if (-not $ready) {
        throw "PostgreSQL is not accepting queries after 15 seconds."
    }

    Write-Status "Ensuring w-testing role exists..." "DarkGray"
    & $psqlExe -p $Port -U postgres -c "CREATE ROLE `"w-testing`" WITH LOGIN PASSWORD 'w-testing'" 2>&1 | ForEach-Object {
        if ($_ -notmatch 'already exists') { Write-Status "  $_" "DarkGray" }
    }
    & $psqlExe -p $Port -U postgres -c "ALTER ROLE `"w-testing`" WITH LOGIN CREATEDB PASSWORD 'w-testing'" 2>&1 | Out-Null

    Write-Status "Checking for existing database..." "DarkGray"
    $dbCheck = & $psqlExe -p $Port -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='sgw'" 2>$null
    if ($dbCheck -match '1') {
        Write-Status "Database 'sgw' already exists." "DarkGray"
        & $psqlExe -p $Port -U postgres -c "ALTER DATABASE sgw OWNER TO `"w-testing`"" 2>&1 | Out-Null
        & $psqlExe -p $Port -U postgres -d sgw -c "GRANT ALL ON DATABASE sgw TO `"w-testing`"; GRANT ALL ON SCHEMA public TO `"w-testing`"" 2>&1 | Out-Null
    } else {
        Write-Status "Creating database..." "White"
        & $psqlExe -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" 2>&1 | ForEach-Object {
            Write-Status "  $_" "DarkGray"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to create database 'sgw'."
        }
        Write-Status "Database 'sgw' created." "Green"
    }

    # Step 4: Load schemas
    $databaseSql = Join-Path $paths.DbDir "database.sql"

    if ($Force) {
        if ($PSCmdlet.ShouldProcess("database 'sgw'", "Drop and recreate")) {
            Write-Status "Force flag set - dropping and recreating database 'sgw'..." "Yellow"
            & $psqlExe -p $Port -U postgres -c "DROP DATABASE IF EXISTS sgw" | Out-Null
            & $psqlExe -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to recreate database 'sgw' under -Force."
            }
            Write-Status "Database 'sgw' recreated." "Green"
        }
        $tableCheck = ""
    } else {
        Write-Status "Checking for existing schemas..." "DarkGray"
        $tableCheck = & $psqlExe -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>$null
    }

    if ($tableCheck -match '1') {
        Write-Status "Schemas already loaded (account table exists). Use -Force to reload." "DarkGray"
    } else {
        Write-Status "Loading database.sql (resources + public schemas)..." "White"
        & $psqlExe -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $databaseSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "database.sql failed (exit code $LASTEXITCODE)."
        }
        Write-Status "database.sql loaded." "Green"
    }

    # Step 5: Verify key tables
    Write-Status "Verifying schema..." "White"
    $verifyTables = @("account", "sgw_player", "shards")
    $allGood = $true
    foreach ($table in $verifyTables) {
        $check = & $psqlExe -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='$table'" 2>$null
        if ($check -match '1') {
            Write-Status "  [OK] $table" "Green"
        } else {
            Write-Status "  [--] $table" "Red"
            $allGood = $false
        }
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
