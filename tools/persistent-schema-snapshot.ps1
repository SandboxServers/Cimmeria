<#
.SYNOPSIS
Writes db/persistent/persistent-schema.snapshot.json from a live test database.
.DESCRIPTION
Records, for every table in db/persistent/persistent-public-tables.txt, its columns
(name, type, nullability, default), its constraints (name, type) and its index names.
The PP-11 live-DB test (crates/content-engine/src/persistent_schema_snapshot_tests.rs)
fails when a committed column, constraint or index is dropped or changed, and when the
live schema has something the snapshot does not. Regenerate and commit after an additive
change only (D-PP7).
Usage (from a repo or worktree root): pwsh tools/persistent-schema-snapshot.ps1 [-Database <name>]
The database defaults to the one tools/build-lane/reload-db.ps1 loads for this checkout
(sgw_<worktree dir name>, or sgwtest in the main checkout); reload it first. The play
database `sgw` is refused.
#>
param([string]$Database)
$ErrorActionPreference = 'Stop'
if (-not (Test-Path -LiteralPath 'db/persistent/persistent-public-tables.txt')) {
    [Console]::Error.WriteLine("persistent-schema-snapshot: run from a repo/worktree root"); exit 2
}
$Top = git rev-parse --show-toplevel 2>$null
if (-not $Top) { $Top = (Get-Location).Path }
$Main = Split-Path -Parent (git rev-parse --path-format=absolute --git-common-dir)
$Name = ((Split-Path -Leaf $Top) -replace '[^A-Za-z0-9_]', '_').ToLowerInvariant()
$same = [System.IO.Path]::GetFullPath($Top).TrimEnd('\', '/') -eq [System.IO.Path]::GetFullPath($Main).TrimEnd('\', '/')
if (-not $Database) { $Database = if ($env:CIMMERIA_TEST_DB) { $env:CIMMERIA_TEST_DB } elseif ($same) { 'sgwtest' } else { "sgw_$Name" } }
if ($Database -ieq 'sgw') {
    [Console]::Error.WriteLine("persistent-schema-snapshot: refusing to read 'sgw': it is the play database."); exit 2
}
$tables = @(Get-Content -LiteralPath 'db/persistent/persistent-public-tables.txt' |
    ForEach-Object { ($_ -replace '#.*', '').Trim() } | Where-Object { $_ })
foreach ($t in $tables) {
    if ($t -notmatch '^[a-z_][a-z0-9_]*$') { [Console]::Error.WriteLine("persistent-schema-snapshot: bad table name '$t'"); exit 2 }
}
$Psql = if ($env:PSQL) { $env:PSQL } else { Join-Path $Main 'external/postgresql_server/bin/psql.exe' }
if (-not (Test-Path -LiteralPath $Psql)) { $Psql = (Get-Command psql -CommandType Application).Source }
if (-not $env:PGPASSWORD) { $env:PGPASSWORD = 'w-testing' }
$Port = if ($env:PGPORT) { $env:PGPORT } else { '5433' }
$arr = "'{" + ($tables -join ',') + "}'::text[]"
# Same catalog queries as the Rust loader in persistent_schema_snapshot_tests.rs; keep them in step.
$sql = @"
SELECT * FROM (
SELECT 'column', c.table_name::text, c.column_name::text, c.data_type::text, c.is_nullable::text, COALESCE(c.column_default, '')::text
FROM information_schema.columns c WHERE c.table_schema = 'public' AND c.table_name = ANY($arr)
UNION ALL
SELECT 'constraint', rel.relname::text, con.conname::text, con.contype::text, '', ''
FROM pg_constraint con JOIN pg_class rel ON rel.oid = con.conrelid
JOIN pg_namespace n ON n.oid = rel.relnamespace
WHERE n.nspname = 'public' AND rel.relname = ANY($arr)
UNION ALL
SELECT 'index', i.tablename::text, i.indexname::text, '', '', '' FROM pg_indexes i
WHERE i.schemaname = 'public' AND i.tablename = ANY($arr)
) AS u(kind, tbl, nm, ty, nul, def)
ORDER BY kind, tbl COLLATE "C", nm COLLATE "C"
"@
$HostArgs = @('-h', 'localhost', '-p', $Port, '-U', 'w-testing', '-d', $Database)
$rows = & $Psql @HostArgs -q -tA -F "`t" -v ON_ERROR_STOP=1 -c $sql
if ($LASTEXITCODE -ne 0) { [Console]::Error.WriteLine("persistent-schema-snapshot: query on $Database failed"); exit 1 }

$shape = [ordered]@{}
foreach ($t in $tables) {
    $shape[$t] = [ordered]@{ columns = [System.Collections.Generic.List[object]]::new(); constraints = [System.Collections.Generic.List[object]]::new(); indexes = [System.Collections.Generic.List[object]]::new() }
}
foreach ($row in $rows) {
    $f = $row -split "`t", 6
    switch ($f[0]) {
        'column' { $shape[$f[1]].columns.Add([ordered]@{ name = $f[2]; data_type = $f[3]; is_nullable = $f[4]; column_default = $f[5] }) }
        'constraint' { $shape[$f[1]].constraints.Add([ordered]@{ name = $f[2]; type = $f[3] }) }
        'index' { $shape[$f[1]].indexes.Add($f[2]) }
    }
}
$snapshot = [ordered]@{ tables = $shape }
$json = $snapshot | ConvertTo-Json -Depth 6
$out = Join-Path $Top 'db/persistent/persistent-schema.snapshot.json'
[System.IO.File]::WriteAllText($out, $json + "`n", [System.Text.UTF8Encoding]::new($false))
Write-Output "persistent-schema-snapshot: wrote $($tables.Count) tables from $Database to db/persistent/persistent-schema.snapshot.json"
