# Wipe and reload THIS worktree's own test database from ./db/database.sql, on the
# bundled Postgres at localhost:5433. Prints the DATABASE_URL to use. PowerShell 7 twin
# of reload-db.sh.
#
# Usage (from a repo or worktree root): pwsh tools/build-lane/reload-db.ps1
#
# Database name: sgw_<worktree dir name> (non [A-Za-z0-9_] characters become `_`); the
# main checkout keeps `sgw`. Override with CIMMERIA_TEST_DB=<name>. One database per
# worktree lets live-DB runs from different worktrees proceed concurrently without one
# reload landing under another's tests.
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath 'db/database.sql')) {
    [Console]::Error.WriteLine("reload-db: run from a repo/worktree root (db/database.sql not found in $((Get-Location).Path))"); exit 2
}
$Top = git rev-parse --show-toplevel 2>$null
if (-not $Top) { $Top = (Get-Location).Path }
$Main = Split-Path -Parent (git rev-parse --path-format=absolute --git-common-dir)
$Name = ((Split-Path -Leaf $Top) -replace '[^A-Za-z0-9_]', '_').ToLowerInvariant()
$same = [System.IO.Path]::GetFullPath($Top).TrimEnd('\', '/') -eq [System.IO.Path]::GetFullPath($Main).TrimEnd('\', '/')
$Db = if ($same) { 'sgw' } else { "sgw_$Name" }
if ($env:CIMMERIA_TEST_DB) { $Db = $env:CIMMERIA_TEST_DB }
$Psql = if ($env:PSQL) { $env:PSQL } else { Join-Path $Main 'external/postgresql_server/bin/psql.exe' }
if (-not (Test-Path -LiteralPath $Psql)) { $Psql = (Get-Command psql -CommandType Application).Source }
if (-not $env:PGPASSWORD) { $env:PGPASSWORD = 'w-testing' }
$Port = if ($env:PGPORT) { $env:PGPORT } else { '5433' }
$HostArgs = @('-h', 'localhost', '-p', $Port, '-U', 'w-testing')
$Log = Join-Path ([System.IO.Path]::GetTempPath()) "reload-db-$Db.log"
$ErrorActionPreference = 'Continue'
$start = Get-Date
& $Psql @HostArgs -d postgres -q -v ON_ERROR_STOP=1 `
    -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname='$Db' AND pid<>pg_backend_pid();" `
    -c "DROP DATABASE IF EXISTS `"$Db`";" `
    -c "CREATE DATABASE `"$Db`" OWNER `"w-testing`";" *> $Log
if ($LASTEXITCODE -ne 0) {
    [Console]::Error.WriteLine("reload-db: drop/create of $Db FAILED:")
    Get-Content -LiteralPath $Log -Tail 20 | ForEach-Object { [Console]::Error.WriteLine($_) }; exit 1
}
& $Psql @HostArgs -d $Db -q -v ON_ERROR_STOP=1 -f db/database.sql *>> $Log
if ($LASTEXITCODE -ne 0) {
    [Console]::Error.WriteLine("reload-db: schema load into $Db FAILED. Last 40 lines of ${Log}:")
    Get-Content -LiteralPath $Log -Tail 40 | ForEach-Object { [Console]::Error.WriteLine($_) }; exit 1
}
$n = (& $Psql @HostArgs -d $Db -tAc 'select count(*) from resources.content_chains' | Select-Object -First 1)
Write-Output "reload-db: OK in $([int]((Get-Date) - $start).TotalSeconds)s into $Db from $((Get-Location).Path) ($("$n".Trim()) content chains)"
Write-Output "DATABASE_URL=postgres://w-testing:w-testing@localhost:$Port/$Db"
