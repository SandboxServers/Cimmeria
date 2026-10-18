function Invoke-CimmeriaDatabaseWipe {
    <#
    .SYNOPSIS
        Prints player counts, backs up and drops the game database (-WipeDatabase).
    .DESCRIPTION
        Counts the accounts and characters (sgw_player). Writes a pg_dump -Fc backup to
        <BackupDir>/<Database>-wipe-<UTC>.dump (the resources refresh never prunes this
        name), terminates the sessions and drops the database.
        Does nothing after the counts when <Database> does not exist. With -CountsOnly
        it prints the counts and returns, so -ResetDatabase can show what it destroys.
        Backups are never pruned here.
    .PARAMETER PsqlExe
        Path to psql; pg_dump is expected beside it.
    .PARAMETER BackupDir
        Default <project>/server/backups.
    .OUTPUTS
        Backup path, or nothing when no backup was written.
    #>
    param(
        [Parameter(Mandatory)][string]$PsqlExe,
        [Parameter(Mandatory)][int]$Port,
        [switch]$UseDocker,
        [string]$Database = 'sgw',
        [string]$BackupDir,
        [switch]$CountsOnly
    )

    $paths = Get-ProjectPaths
    if (-not $BackupDir) { $BackupDir = Join-Path $paths.ProjectRoot 'server/backups' }
    $exeSuffix = [System.IO.Path]::GetExtension($PsqlExe)
    $pgDump = Join-Path (Split-Path $PsqlExe -Parent) "pg_dump$exeSuffix"
    $conn = @{ PsqlExe = $PsqlExe; Port = $Port; UseDocker = $UseDocker }
    $scalar = { param($sql, $db, $what) ((Invoke-CimmeriaPsql @conn -Admin -Database $db -Sql $sql -What $what) -join '').Trim() }

    $exists = (& $scalar "SELECT count(*) FROM pg_database WHERE datname = '$Database'" 'postgres' "check $Database") -eq '1'
    if (-not $exists) {
        Write-Status "Wipe: database '$Database' does not exist; nothing to count, back up or drop." "DarkGray"
        return
    }

    # Counts: a table that is missing from an old schema shows as n/a.
    $count = {
        param($table)
        if ((& $scalar "SELECT to_regclass('public.$table') IS NOT NULL" $Database "check $table") -ne 't') { return 'n/a' }
        & $scalar "SELECT count(*) FROM public.$table" $Database "count $table"
    }
    $accounts = & $count 'account'
    $characters = & $count 'sgw_player'
    Write-Status "Wipe: '$Database' holds $accounts accounts and $characters characters." "Yellow"
    if ($CountsOnly) { return }

    # Backup first: the dump is the only copy of the player data after the drop.
    $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMdd'T'HHmmss'Z'")
    Write-Status "Wipe: backing up '$Database'..." "White"
    New-Item -ItemType Directory $BackupDir -Force | Out-Null
    $backupPath = Join-Path $BackupDir "$Database-wipe-$stamp.dump"
    Invoke-CimmeriaPsql @conn -Admin -Exe $pgDump -Database $Database -ExtraArgs @('-Fc', '-f', $backupPath) -What "backup of $Database" | Out-Null
    if (-not (Test-Path $backupPath) -or (Get-Item $backupPath).Length -eq 0) { throw "Wipe: backup file $backupPath is missing or empty; '$Database' was not dropped." }

    Write-Status "Wipe: dropping '$Database'..." "Yellow"
    Invoke-CimmeriaPsql @conn -Admin -Sql "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$Database' AND pid <> pg_backend_pid()" -What "terminate $Database" | Out-Null
    Invoke-CimmeriaPsql @conn -Admin -Sql "DROP DATABASE `"$Database`"" -What "drop $Database" | Out-Null
    Write-Status "Wipe: backup at $backupPath." "Green"
    return $backupPath
}
