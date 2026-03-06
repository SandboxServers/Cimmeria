function Initialize-CimmeriaDatabase {
    <#
    .SYNOPSIS
        Initializes a local PostgreSQL instance and loads the Cimmeria database schemas.

    .DESCRIPTION
        Uses the PostgreSQL 17 binaries from external/postgresql_server/ to:
        1. Initialize a data directory in server/pgdata/ (if it doesn't exist)
        2. Start PostgreSQL on port 5433 (avoids conflicts with system PG)
        3. Create the "w-testing" role and "sgw" database
        4. Load db/database.sql (resources schema + public schema + test data)
        5. Verify key tables exist

        The test account (test/test) is created by the seed data in database.sql.
        Port 5433 and the w-testing credentials match the config files checked into
        the repo.

        Use -Force to drop and recreate the sgw database before loading, giving a
        completely clean slate without touching the PostgreSQL cluster itself.

    .PARAMETER Port
        PostgreSQL port. Default 5433.

    .PARAMETER Force
        Drop and recreate the sgw database before loading schemas.
        Bypasses the idempotency check — always performs a full clean load.

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

    # Find PG binaries
    $pgBin = Find-PostgreSQL
    if (-not $pgBin) {
        throw "PostgreSQL server binaries not found. Run Install-CimmeriaDependencies first."
    }

    $pgDataDir = Join-Path $paths.ServerDir "pgdata"
    $pgLogDir = Join-Path $paths.ServerDir "logs"
    $pgLogFile = Join-Path $pgLogDir "postgresql.log"

    New-Item -ItemType Directory -Path $pgLogDir -Force | Out-Null

    Write-Step "INITIALIZING DATABASE"

    # Step 1: initdb (with version mismatch detection)
    $pgVersionFile = Join-Path $pgDataDir "PG_VERSION"
    if (Test-Path $pgVersionFile) {
        $dataVersion = (Get-Content $pgVersionFile -First 1).Trim()
        # Determine the server's major version from postgres.exe --version
        $pgServerVersion = & (Join-Path $pgBin "postgres.exe") --version 2>&1
        if ($pgServerVersion -match '(\d+)\.') {
            $serverMajor = $Matches[1]
            $dataMajor = $dataVersion.Split('.')[0]
            if ($dataMajor -ne $serverMajor) {
                Write-Status "pgdata version mismatch: data=$dataVersion, server=PG $serverMajor" "Yellow"
                Write-Status "Removing old pgdata/ directory (PG major version upgrade)..." "Yellow"
                # pg_ctl stop in case it's somehow running with old binaries
                $pgCtlCheck = Join-Path $pgBin "pg_ctl.exe"
                & $pgCtlCheck stop -D $pgDataDir -m immediate 2>&1 | Out-Null
                Remove-Item $pgDataDir -Recurse -Force
                Write-Status "Old pgdata/ removed. Will re-initialize." "Green"
            }
        }
    }
    if (-not (Test-Path $pgVersionFile)) {
        if ($PSCmdlet.ShouldProcess("server/pgdata", "Initialize PostgreSQL data directory")) {
            Write-Status "Running initdb..." "White"
            $initdb = Join-Path $pgBin "initdb.exe"
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
    $pgCtl = Join-Path $pgBin "pg_ctl.exe"

    # Check if already running: pg_ctl status is unreliable from WSL/non-Windows
    # environments because it can't locate sibling .exe files.  Fall back to a
    # direct TCP probe so we never attempt a double-start.
    $statusResult = & $pgCtl status -D $pgDataDir 2>&1
    $pgAlreadyRunning = ($LASTEXITCODE -eq 0) -or (Wait-ForPort -Port $Port -TimeoutSeconds 1)
    if ($pgAlreadyRunning) {
        Write-Status "PostgreSQL already running." "DarkGray"
    } else {
        if ($PSCmdlet.ShouldProcess("PostgreSQL on port $Port", "Start database server")) {
            Write-Status "Starting PostgreSQL on port $Port..." "White"
            # pg_ctl start spawns postgres which keeps handles open, blocking
            # both pipes and Start-Process -Wait. Fire-and-forget, then poll.
            $pgCtlArgs = "start -D `"$pgDataDir`" -l `"$pgLogFile`" -o `"-p $Port`""
            $spArgs = @{ FilePath = $pgCtl; ArgumentList = $pgCtlArgs }
            # -WindowStyle is Windows-only; omit on Linux (e.g. WSL validation runs)
            if ($IsWindows -or (-not (Test-Path variable:IsWindows))) {
                $spArgs['WindowStyle'] = 'Hidden'
            }
            Start-Process @spArgs
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

    # Step 3: Create role and database
    $psqlExe = Join-Path $pgBin "psql.exe"

    # Wait for PostgreSQL to be ready for queries (TCP open != accepting queries)
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

    # Check if database already exists
    Write-Status "Checking for existing database..." "DarkGray"
    $dbCheck = & $psqlExe -p $Port -U postgres -tAc "SELECT 1 FROM pg_database WHERE datname='sgw'" 2>$null
    if ($dbCheck -match '1') {
        Write-Status "Database 'sgw' already exists." "DarkGray"
    } else {
        Write-Status "Creating role and database..." "White"

        & $psqlExe -p $Port -U postgres -c "CREATE ROLE `"w-testing`" WITH LOGIN PASSWORD 'w-testing'" 2>&1 | ForEach-Object {
            if ($_ -notmatch 'already exists') { Write-Status "  $_" "DarkGray" }
        }

        & $psqlExe -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" 2>&1 | ForEach-Object {
            Write-Status "  $_" "DarkGray"
        }
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to create database 'sgw'."
        }
        Write-Status "Database 'sgw' created." "Green"
    }

    # Step 4: Load schemas (strict order, fail fast)
    $databaseSql = Join-Path $paths.DbDir "database.sql"

    # -Force: drop and recreate the database for a guaranteed clean load
    if ($Force) {
        if ($PSCmdlet.ShouldProcess("database 'sgw'", "Drop and recreate")) {
            Write-Status "Force flag set — dropping and recreating database 'sgw'..." "Yellow"
            & $psqlExe -p $Port -U postgres -c "DROP DATABASE IF EXISTS sgw" | Out-Null
            & $psqlExe -p $Port -U postgres -c "CREATE DATABASE sgw OWNER `"w-testing`"" | Out-Null
            if ($LASTEXITCODE -ne 0) {
                throw "Failed to recreate database 'sgw' under -Force."
            }
            Write-Status "Database 'sgw' recreated." "Green"
        }
        # Bypass idempotency check — always load
        $tableCheck = ""
    } else {
        # Check if schemas are already loaded by testing for a key table
        Write-Status "Checking for existing schemas..." "DarkGray"
        $tableCheck = & $psqlExe -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>$null
    }

    if ($tableCheck -match '1') {
        Write-Status "Schemas already loaded (account table exists). Use -Force to reload." "DarkGray"
    } else {
        Write-Status "Loading database.sql (resources + public schemas, ~126K statements)..." "White"
        & $psqlExe -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $databaseSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "database.sql failed (exit code $LASTEXITCODE). Aborting — a partial schema is worse than none."
        }
        Write-Status "database.sql loaded." "Green"
    }

    # Step 5: Verify key tables
    Write-Status "Verifying schema..." "White"
    $verifyTables = @("account", "sgw_player")
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
    Write-Status "  Tip: use -Force to drop and reload from scratch." "DarkGray"
}
