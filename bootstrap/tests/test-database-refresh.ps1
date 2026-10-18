# Tests for Invoke-CimmeriaResourcesRefresh (local resources-only reseed) against a
# throwaway database sgw_pp13_refresh on the local test PostgreSQL (:5433). Never
# touches `sgw`.
#   Case 1: sentinel account and character survive; a mutated resources row is
#           restored; a deleted seed account comes back with exactly one
#           insert_seed_account log row; the image id starts with `local-`; the
#           backup file exists; <db>_prev exists.
#   Case 2: a carry-over that fails (CHECK dropped, violating row inserted) throws
#           and leaves the database unchanged, with no _next/_fresh left behind.
# Skips (exit 0) when :5433 is unreachable or psql is missing.
#
# Usage: pwsh -NoProfile -File bootstrap/tests/test-database-refresh.ps1
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path (Join-Path $PSScriptRoot '../..')).Path
$port = 5433
$db = 'sgw_pp13_refresh'
$backupDir = Join-Path ([IO.Path]::GetTempPath()) "cimmeria-refresh-backups-$PID"
$failed = 0

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    if ($ok) { Write-Host "ok   $name" } else { Write-Host "FAIL $name $detail"; $script:failed++ }
}

$psql = @(
    (Join-Path $root 'external/postgresql_server/bin/psql.exe'),
    (Join-Path (Split-Path $root -Parent | Split-Path -Parent | Split-Path -Parent) 'external/postgresql_server/bin/psql.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $psql) { $c = Get-Command psql -ErrorAction SilentlyContinue; if ($c) { $psql = $c.Source } }
if (-not $psql) { Write-Host 'skip: psql not found'; exit 0 }
$up = $false
try { $t = [Net.Sockets.TcpClient]::new(); $t.Connect('localhost', $port); $t.Close(); $up = $true } catch { }
if (-not $up) { Write-Host "skip: PostgreSQL not reachable on :$port"; exit 0 }

Import-Module (Join-Path $root 'bootstrap/CimmeriaBootstrap/CimmeriaBootstrap.psd1') -Force
$mod = Get-Module CimmeriaBootstrap

function Q([string]$sql, [string]$d = $db) {
    $r = & $psql -X -q -p $port -U postgres -d $d -tAc $sql 2>&1
    if ($LASTEXITCODE -ne 0) { throw "psql failed: $r" }
    "$r".Trim()
}
function DbExists([string]$n) { (Q "SELECT count(*) FROM pg_database WHERE datname = '$n'" 'postgres') -eq '1' }
function DropAll {
    foreach ($n in $db, "${db}_fresh", "${db}_next", "${db}_prev") {
        Q "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname = '$n' AND pid <> pg_backend_pid()" 'postgres' | Out-Null
        Q "DROP DATABASE IF EXISTS `"$n`"" 'postgres' | Out-Null
    }
}
function Refresh {
    & $mod { param($p, $port, $db, $bd) Invoke-CimmeriaResourcesRefresh -PsqlExe $p -Port $port -Database $db -BackupDir $bd } $psql $port $db $backupDir
}
# Loads database.sql (plus the role) into the throwaway database, like a first setup.
function NewGameDb {
    DropAll
    Q "DO `$`$ BEGIN CREATE ROLE `"w-testing`" LOGIN PASSWORD 'w-testing' CREATEDB; EXCEPTION WHEN duplicate_object THEN NULL; END `$`$" 'postgres' | Out-Null
    Q "CREATE DATABASE `"$db`" OWNER `"w-testing`"" 'postgres' | Out-Null
    $sql = & $mod { param($r) Resolve-SqlIncludes (Join-Path $r 'db/database.sql') } $root
    & $psql -X -q -p $port -U 'w-testing' -d $db -v ON_ERROR_STOP=1 -f $sql 2>&1 | Out-Null
    $code = $LASTEXITCODE
    Remove-Item $sql -ErrorAction SilentlyContinue
    if ($code -ne 0) { throw 'database.sql load failed' }
}
$digest = "SELECT md5((SELECT string_agg(a::text, '|' ORDER BY account_id) FROM account a) || (SELECT string_agg(p::text, '|' ORDER BY player_id) FROM sgw_player p))"
function AddSentinel {
    Q "INSERT INTO account (account_id, account_name, password) VALUES (1879048193, 'pp13_sentinel', 'x')" | Out-Null
    Q "INSERT INTO sgw_player (account_id, player_id, alignment, archetype, gender, player_name, extra_name, bodyset, world_location, pos_x, pos_y, pos_z, skin_color_id, world_id)
       SELECT 1879048193, 1879048194, 1, 1, 1, 'pp13sentinel', '', 'BS_HumanMale.BS_HumanMale', w.world, 0, 0, 0, 0, w.world_id FROM resources.worlds w WHERE w.world = 'Castle_CellBlock'" | Out-Null
}

try {
    NewGameDb
    AddSentinel
    $origName = Q "SELECT name FROM resources.items ORDER BY item_id LIMIT 1"
    Q "UPDATE resources.items SET name = 'pp13 mutated' WHERE item_id = (SELECT min(item_id) FROM resources.items)" | Out-Null
    Q "DELETE FROM account WHERE account_id = 3" | Out-Null

    $res = Refresh
    Check 'case1 account survives' ((Q "SELECT count(*) FROM account WHERE account_id = 1879048193") -eq '1')
    Check 'case1 character survives' ((Q "SELECT count(*) FROM sgw_player WHERE player_id = 1879048194") -eq '1')
    Check 'case1 resources row restored' ((Q "SELECT name FROM resources.items ORDER BY item_id LIMIT 1") -eq $origName)
    Check 'case1 seed account back' ((Q "SELECT count(*) FROM account WHERE account_id = 3") -eq '1')
    Check 'case1 one insert_seed_account log row' ((Q "SELECT count(*) FROM cimmeria_meta.reconcile_log WHERE action = 'insert_seed_account'") -eq '1')
    Check 'case1 image id local-' ((Q "SELECT image_id FROM cimmeria_meta.db_state WHERE id = 1") -like 'local-*')
    Check 'case1 state live' ((Q "SELECT state FROM cimmeria_meta.db_state WHERE id = 1") -eq 'live')
    Check 'case1 backup exists' ($res.BackupPath -and (Test-Path $res.BackupPath))
    Check 'case1 _prev exists' (DbExists "${db}_prev")
    Check 'case1 no _fresh/_next left' (-not (DbExists "${db}_fresh") -and -not (DbExists "${db}_next"))

    # Case 2: fresh throwaway database; the carry-over must be rejected.
    NewGameDb
    AddSentinel
    Q "ALTER TABLE sgw_player DROP CONSTRAINT gender_sanity" | Out-Null
    Q "INSERT INTO sgw_player (account_id, player_id, alignment, archetype, gender, player_name, extra_name, bodyset, world_location, pos_x, pos_y, pos_z, skin_color_id)
       VALUES (1879048193, 1879048195, 1, 1, 9, 'pp13bad', '', 'BS_HumanMale.BS_HumanMale', 'Castle_CellBlock', 0, 0, 0, 0)" | Out-Null
    $before = Q $digest
    $beforePlayers = Q "SELECT count(*) FROM sgw_player"
    $threw = $false; $msg = ''
    try { Refresh | Out-Null } catch { $threw = $true; $msg = $_.Exception.Message }
    Check 'case2 refresh throws' $threw
    Check 'case2 names the carry-over' ($msg -match 'carry-over') $msg
    Check 'case2 sentinel still there' ((Q "SELECT count(*) FROM sgw_player WHERE player_id IN (1879048194, 1879048195)") -eq '2')
    Check 'case2 player count unchanged' ((Q "SELECT count(*) FROM sgw_player") -eq $beforePlayers)
    Check 'case2 database unchanged' ((Q $digest) -eq $before)
    Check 'case2 no _fresh/_next/_prev left' (-not (DbExists "${db}_fresh") -and -not (DbExists "${db}_next") -and -not (DbExists "${db}_prev"))
} finally {
    try { DropAll } catch { Write-Host "cleanup: $($_.Exception.Message)" }
    Remove-Item $backupDir -Recurse -Force -ErrorAction SilentlyContinue
}
if ($failed) { Write-Host "$failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
