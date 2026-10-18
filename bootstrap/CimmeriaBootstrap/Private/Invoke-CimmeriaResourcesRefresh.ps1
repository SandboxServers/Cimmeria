function Invoke-CimmeriaResourcesRefresh {
    <#
    .SYNOPSIS
        Reseeds resources in the local database and keeps every player row.
    .DESCRIPTION
        Local mirror of docker/db-rebuild.sh (campaign persistent-player-data).
        Builds <Database>_fresh from the seeds, copies it to <Database>_next, carries
        the player tables of <Database> over (lists in db/persistent/), reconciles
        them against the new resources, tops up missing seed accounts, then swaps by
        rename: <Database> becomes <Database>_prev, <Database>_next becomes
        <Database>. Any failure before the swap drops _fresh and _next, rethrows,
        and leaves <Database> untouched. A pg_dump backup is written first.
    .PARAMETER PsqlExe
        Path to psql; pg_dump is expected beside it.
    .PARAMETER BackupDir
        Default <project>/server/backups. The newest -BackupKeep dumps of
        <Database> are kept.
    .OUTPUTS
        Object with ImageId, BackupPath, PrevDatabase.
    #>
    param(
        [Parameter(Mandatory)][string]$PsqlExe,
        [Parameter(Mandatory)][int]$Port,
        [switch]$UseDocker,
        [string]$Database = 'sgw',
        [string]$BackupDir,
        [int]$BackupKeep = 3
    )

    $paths = Get-ProjectPaths
    if (-not $BackupDir) { $BackupDir = Join-Path $paths.ProjectRoot 'server/backups' }
    $persist = Join-Path $paths.DbDir 'persistent'
    $fresh = "${Database}_fresh"; $next = "${Database}_next"; $prev = "${Database}_prev"
    $exeSuffix = [System.IO.Path]::GetExtension($PsqlExe)
    $pgDump = Join-Path (Split-Path $PsqlExe -Parent) "pg_dump$exeSuffix"
    $work = Join-Path ([System.IO.Path]::GetTempPath()) "cimmeria-refresh-$PID"

    $conn = @{ PsqlExe = $PsqlExe; Port = $Port; UseDocker = $UseDocker }
    $adminSql = { param($sql, $what) Invoke-CimmeriaPsql @conn -Admin -Sql $sql -What $what | Out-Null }
    $kill = { param($db) & $adminSql "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$db' AND pid <> pg_backend_pid()" "terminate $db" }
    $dropDb = { param($db) & $kill $db; & $adminSql "DROP DATABASE IF EXISTS `"$db`"" "drop $db" }

    # Table lists: strip comments and blanks, drop reseed tables, always carry account.
    $readList = { param($f) @(Get-Content (Join-Path $persist $f) | ForEach-Object { ($_ -replace '#.*$', '').Trim() } | Where-Object { $_ }) }
    $reseed = & $readList 'reseed-public-tables.txt'
    $carry = @((& $readList 'persistent-public-tables.txt') + 'account' | Select-Object -Unique | Where-Object { $reseed -notcontains $_ })
    if (-not $carry.Count) { throw 'carry list is empty' }

    $swapped = $false
    try {
        # a. refuse unless <Database> is a loaded game database
        Write-Status "Refresh: checking '$Database'..." "White"
        $has = ''
        try { $has = Invoke-CimmeriaPsql @conn -Database $Database -Sql "SELECT to_regclass('public.account') IS NOT NULL" -What "check $Database" } catch { }
        if ("$has".Trim() -ne 't') { throw "Database '$Database' has no public.account; run setup.ps1 first (refresh needs an existing game database)." }
        & $dropDb $fresh; & $dropDb $next

        # b. backup
        $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMdd'T'HHmmss'Z'")
        Write-Status "Refresh: backing up '$Database'..." "White"
        New-Item -ItemType Directory $BackupDir -Force | Out-Null
        $backupPath = Join-Path $BackupDir "$Database-$stamp.dump"
        Invoke-CimmeriaPsql @conn -Admin -Exe $pgDump -Database $Database -ExtraArgs @('-Fc', '-f', $backupPath) -What "backup of $Database" | Out-Null
        # Only this function's own dumps are pruned; a -WipeDatabase backup (<Database>-wipe-...)
        # is the only copy of the player data after the drop and is never matched.
        Get-ChildItem $BackupDir -Filter "$Database-*.dump" |
            Where-Object { $_.Name -match "^$([regex]::Escape($Database))-\d{8}T\d{6}Z\.dump$" } |
            Sort-Object Name -Descending | Select-Object -Skip $BackupKeep | Remove-Item -Force

        # c. fresh database from the seeds, plus cimmeria_meta
        Write-Status "Refresh: loading seeds into '$fresh'..." "White"
        New-Item -ItemType Directory $work -Force | Out-Null
        $sha12 = 'nogit'
        try { $g = (& git -C $paths.ProjectRoot rev-parse --short=12 HEAD 2>$null); if ($LASTEXITCODE -eq 0 -and $g) { $sha12 = "$g".Trim() } } catch { }
        $imageId = "local-$sha12-$((Get-Date).ToUniversalTime().ToString("yyyyMMdd'T'HHmmss'Z'"))"
        & $adminSql "CREATE DATABASE `"$fresh`" OWNER `"w-testing`"" "create $fresh"
        $resolved = Resolve-SqlIncludes (Join-Path $paths.DbDir 'database.sql')
        $srcSha = (Get-FileHash $resolved -Algorithm SHA256).Hash.ToLower()
        try { Invoke-CimmeriaPsql @conn -Database $fresh -File $resolved -What "seed load into $fresh" | Out-Null }
        finally { Remove-Item $resolved -ErrorAction SilentlyContinue }
        Invoke-CimmeriaPsql @conn -Database $fresh -File (Join-Path $persist 'meta.sql') -What 'meta.sql' | Out-Null
        Invoke-CimmeriaPsql @conn -Database $fresh -Sql "INSERT INTO cimmeria_meta.db_state (id, state, image_id, db_source_sha256) VALUES (1, 'baked', '$imageId', '$srcSha')" -What 'db_state insert' | Out-Null

        # d. next = copy of fresh; carry player tables from the old database
        Write-Status "Refresh: carrying player data into '$next'..." "White"
        & $kill $fresh
        & $adminSql "CREATE DATABASE `"$next`" TEMPLATE `"$fresh`" OWNER `"w-testing`"" "create $next"
        $oldRebuilds = '0'   # a database that predates cimmeria_meta has no count
        try {
            $r = "$(Invoke-CimmeriaPsql @conn -Database $Database -Sql 'SELECT COALESCE(rebuild_count, 0) FROM cimmeria_meta.db_state WHERE id = 1' -What 'read db_state')".Trim()
            if ($r -match '^\d+$') { $oldRebuilds = $r }
        } catch { }

        $oldEnv = $env:PGOPTIONS
        $env:PGOPTIONS = "-c cimmeria.image_id=$imageId"
        try {
            # e1. stage seed accounts: seed-accounts-stage.sql minus its `TRUNCATE account;`.
            # That lone TRUNCATE fails once foreign keys exist ("Truncate table sgw_player at
            # the same time"); the TRUNCATE of every carried table below does the same job.
            $stageFile = Join-Path $work 'seed-accounts-stage.sql'
            $stage = (Get-Content (Join-Path $persist 'seed-accounts-stage.sql')) | Where-Object { $_ -notmatch '^\s*TRUNCATE\s+account\s*;' }
            Set-Content -Path $stageFile -Value $stage -Encoding utf8
            Invoke-CimmeriaPsql @conn -Database $next -File $stageFile -What 'seed-accounts-stage.sql' | Out-Null

            $tableArgs = @($carry | ForEach-Object { @('-t', "public.$_") })
            $truncate = ($carry | ForEach-Object { "public.`"$_`"" }) -join ', '
            Invoke-CimmeriaPsql @conn -Admin -Database $next -Sql "TRUNCATE $truncate" -What 'truncate carried tables' | Out-Null
            $dataFile = Join-Path $work 'carry.sql'
            Invoke-CimmeriaPsql @conn -Admin -Exe $pgDump -Database $Database -ExtraArgs (@('-a', '--disable-triggers', '-f', $dataFile) + $tableArgs) -What "dump of player tables from $Database" | Out-Null
            try { Invoke-CimmeriaPsql @conn -Admin -Database $next -File $dataFile -What 'carry-over' | Out-Null }
            catch { throw "carry-over rejected: $($_.Exception.Message)" }

            # e2. reconcile, then top up missing seed accounts
            Write-Status "Refresh: reconciling against the new resources..." "White"
            Invoke-CimmeriaPsql @conn -Database $next -File (Join-Path $persist 'reconcile.sql') -What 'reconcile.sql' | Out-Null
            Invoke-CimmeriaPsql @conn -Database $next -File (Join-Path $persist 'seed-accounts.sql') -What 'seed-accounts.sql' | Out-Null
        } finally { $env:PGOPTIONS = $oldEnv }

        # Sequences: pg_dump -a -t emits setval for each sequence owned by a dumped column, so the
        # carry-over also moves them (the container path drops those lines and fixes them in step 6).
        # Guarded by the case 3 sequence checks in test-database-refresh.ps1.

        Invoke-CimmeriaPsql @conn -Database $next -Sql "ANALYZE" -What 'analyze' | Out-Null
        Invoke-CimmeriaPsql @conn -Database $next -Sql "UPDATE cimmeria_meta.db_state SET state = 'live', image_id = '$imageId', first_seeded_at = COALESCE(first_seeded_at, now()), last_rebuild_at = now(), rebuild_count = $oldRebuilds + 1 WHERE id = 1" -What 'db_state update' | Out-Null

        # f. swap by rename; the live database is renamed, never dropped
        Write-Status "Refresh: swapping '$next' in as '$Database'..." "White"
        & $dropDb $fresh
        & $dropDb $prev
        & $kill $Database
        & $adminSql "ALTER DATABASE `"$Database`" RENAME TO `"$prev`"" "rename $Database to $prev"
        try {
            & $kill $next
            & $adminSql "ALTER DATABASE `"$next`" RENAME TO `"$Database`"" "rename $next to $Database"
            $swapped = $true
        } catch {
            & $kill $prev
            & $adminSql "ALTER DATABASE `"$prev`" RENAME TO `"$Database`"" "restore $Database from $prev"
            throw
        }

        # g. done
        Write-Status "Refresh complete: image $imageId, backup $backupPath, previous database '$prev'." "Green"
        return [pscustomobject]@{ ImageId = $imageId; BackupPath = $backupPath; PrevDatabase = $prev }
    } catch {
        if (-not $swapped) {
            try { & $dropDb $fresh; & $dropDb $next } catch { Write-Status "  cleanup of $fresh/$next failed: $($_.Exception.Message)" "Yellow" }
        }
        throw
    } finally {
        Remove-Item $work -Recurse -Force -ErrorAction SilentlyContinue
    }
}
