function Initialize-CimmeriaDatabase {
    <#
    .SYNOPSIS
        Initializes a local PostgreSQL instance and loads the Cimmeria database schemas.

    .DESCRIPTION
        Uses the PostgreSQL 9.2 binaries from external/postgresql_server/ to:
        1. Initialize a data directory in server/pgdata/ (if it doesn't exist)
        2. Start PostgreSQL on port 5433 (avoids conflicts with system PG)
        3. Create the "w-testing" role and "sgw" database
        4. Load db/resources.sql (resource types/tables) then db/sgw.sql (game schema + test data)
        5. Verify key tables exist

        The test account (test/test) is created by sgw.sql. Port 5433 and the
        w-testing credentials match the config files checked into the repo.

    .PARAMETER Port
        PostgreSQL port. Default 5433.

    .EXAMPLE
        Initialize-CimmeriaDatabase
    #>
    [CmdletBinding(SupportsShouldProcess)]
    param(
        [int]$Port = 5433
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

    # Step 1: initdb
    if (-not (Test-Path (Join-Path $pgDataDir "PG_VERSION"))) {
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

    # Check if already running
    $statusResult = & $pgCtl status -D $pgDataDir 2>&1
    if ($LASTEXITCODE -eq 0) {
        Write-Status "PostgreSQL already running." "DarkGray"
    } else {
        if ($PSCmdlet.ShouldProcess("PostgreSQL on port $Port", "Start database server")) {
            Write-Status "Starting PostgreSQL on port $Port..." "White"
            # pg_ctl start spawns postgres which keeps handles open, blocking
            # both pipes and Start-Process -Wait. Fire-and-forget, then poll.
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

    # Step 3: Create role and database
    $psqlExe = Join-Path $pgBin "psql.exe"

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
    $resourcesSql = Join-Path $paths.DbDir "resources.sql"
    $sgwSql = Join-Path $paths.DbDir "sgw.sql"

    # Check if schemas are already loaded by testing for a key table
    Write-Status "Checking for existing schemas..." "DarkGray"
    $tableCheck = & $psqlExe -p $Port -U "w-testing" -d sgw -tAc "SELECT 1 FROM information_schema.tables WHERE table_name='account'" 2>$null
    if ($tableCheck -match '1') {
        Write-Status "Schemas already loaded (account table exists)." "DarkGray"
    } else {
        Write-Status "Loading resources.sql (~126K lines, this may take a minute)..." "White"
        $lineCount = 0
        & $psqlExe -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $resourcesSql 2>&1 | ForEach-Object {
            $lineCount++
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            } elseif ($lineCount % 10000 -eq 0) {
                Write-Status "  $lineCount lines processed..." "DarkGray"
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "resources.sql failed (exit code $LASTEXITCODE). Aborting - a partial schema is worse than none."
        }
        Write-Status "resources.sql loaded ($lineCount statements)." "Green"

        Write-Status "Loading sgw.sql (game schema + test data)..." "White"
        & $psqlExe -p $Port -U "w-testing" -d sgw -v ON_ERROR_STOP=1 -f $sgwSql 2>&1 | ForEach-Object {
            if ($_ -match 'ERROR|FATAL') {
                Write-Status "  $_" "Red"
            }
        }
        if ($LASTEXITCODE -ne 0) {
            throw "sgw.sql failed (exit code $LASTEXITCODE)."
        }
        Write-Status "sgw.sql loaded." "Green"
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
}
