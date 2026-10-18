# Tests for Invoke-CimmeriaResourcesRefresh (local resources-only reseed) against a
# throwaway database sgw_pp13_refresh on the local test PostgreSQL (:5433). Never
# touches `sgw`.
#   Case 1: sentinel account and character survive; a mutated resources row is
#           restored; a deleted seed account comes back with exactly one
#           insert_seed_account log row; the image id starts with `local-`; the
#           backup file exists; <db>_prev exists.
#   Case 2: a carry-over that fails (CHECK dropped, violating row inserted) throws
#           and leaves the database unchanged, with no _next/_fresh left behind.
#   Case 3: -ForceDatabase routing (Refresh) keeps the sentinel rows (PP-14).
#   Routing table: every flag/state row of Resolve-CimmeriaDatabaseAction, and the
#           -Force + -WipeDatabase throw. These need no database and always run.
# Skips (exit 0) when :5433 is unreachable or psql is missing; with CI_REQUIRE_PG=1 it fails.
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

Import-Module (Join-Path $root 'bootstrap/CimmeriaBootstrap/CimmeriaBootstrap.psd1') -Force
$mod = Get-Module CimmeriaBootstrap

# Routing table (Resolve-CimmeriaDatabaseAction). Pure, no database, always runs.
$routes = @(
    @{ N = 'none, absent';                   F = @{};                              S = $false; C = $false; A = $false; E = 'FullLoad' },
    @{ N = 'none, fully seeded';             F = @{};                              S = $true;  C = $true;  A = $true;  E = 'Skip' },
    @{ N = 'none, incomplete, accounts';     F = @{};                              S = $true;  C = $false; A = $true;  E = 'Refresh' },
    @{ N = 'none, incomplete, no accounts';  F = @{};                              S = $true;  C = $false; A = $false; E = 'Replay' },
    @{ N = 'Force, fully seeded';            F = @{ Force = $true };               S = $true;  C = $true;  A = $true;  E = 'Refresh' },
    @{ N = 'Force, incomplete, no accounts'; F = @{ Force = $true };               S = $true;  C = $false; A = $false; E = 'Refresh' },
    @{ N = 'Force, absent';                  F = @{ Force = $true };               S = $false; C = $false; A = $false; E = 'FullLoad' },
    @{ N = 'Wipe, absent';                   F = @{ Wipe = $true };                S = $false; C = $false; A = $false; E = 'Wipe' },
    @{ N = 'Wipe, fully seeded';             F = @{ Wipe = $true };                S = $true;  C = $true;  A = $true;  E = 'Wipe' },
    @{ N = 'Wipe, incomplete, accounts';     F = @{ Wipe = $true };                S = $true;  C = $false; A = $true;  E = 'Wipe' },
    @{ N = 'Force+Wipe, absent';             F = @{ Force = $true; Wipe = $true }; S = $false; C = $false; A = $false; E = 'Throw' },
    @{ N = 'Force+Wipe, fully seeded';       F = @{ Force = $true; Wipe = $true }; S = $true;  C = $true;  A = $true;  E = 'Throw' },
    @{ N = 'Reset, fully seeded';            F = @{ Reset = $true };               S = $true;  C = $true;  A = $true;  E = 'Skip' },
    @{ N = 'Reset, incomplete, no accounts'; F = @{ Reset = $true };               S = $true;  C = $false; A = $false; E = 'Replay' }
)
foreach ($r in $routes) {
    $got = & $mod { param($f, $s, $c, $a) Resolve-CimmeriaDatabaseAction @f -SchemaExists:$s -SeedComplete:$c -HasAccounts:$a } $r.F $r.S $r.C $r.A
    Check "route: $($r.N) -> $($r.E)" ($got -eq $r.E) "got $got"
}
# The throw happens before any docker or psql work, so it needs no PostgreSQL.
$threw = ''
try { Initialize-CimmeriaDatabase -Port 5433 -Force -WipeDatabase | Out-Null } catch { $threw = $_.Exception.Message }
Check 'throw: -Force with -WipeDatabase' ($threw -match 'cannot be combined') $threw
# setup.ps1 -> Invoke-CimmeriaBootstrap must reject the pair before the dependency install,
# the builds and the server stop (so a typo costs nothing). Do not run this check against a
# build with the early throw removed: it would then start the real pipeline.
$threw = ''
try { Invoke-CimmeriaBootstrap -ForceDatabase -WipeDatabase -SkipDownload -SkipBuild -NoLaunch | Out-Null } catch { $threw = $_.Exception.Message }
Check 'throw: bootstrap -ForceDatabase with -WipeDatabase, before any step' (($threw -match 'cannot be combined') -and ($threw -match '^-ForceDatabase')) $threw

$psql = @(
    (Join-Path $root 'external/postgresql_server/bin/psql.exe'),
    (Join-Path (Split-Path $root -Parent | Split-Path -Parent | Split-Path -Parent) 'external/postgresql_server/bin/psql.exe')
) | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $psql) { $c = Get-Command psql -ErrorAction SilentlyContinue; if ($c) { $psql = $c.Source } }
# CI_REQUIRE_PG=1 (bootstrap-lint.yml): a missing PostgreSQL fails the run instead of skipping.
if (-not $psql) { Write-Host 'skip: psql not found'; if ($env:CI_REQUIRE_PG -eq '1' -or $failed) { exit 1 }; exit 0 }
$up = $false
try { $t = [Net.Sockets.TcpClient]::new(); $t.Connect('localhost', $port); $t.Close(); $up = $true } catch { }
if (-not $up) { Write-Host "skip: PostgreSQL not reachable on :$port"; if ($env:CI_REQUIRE_PG -eq '1' -or $failed) { exit 1 }; exit 0 }

# Every psql, pg_dump and Invoke-CimmeriaPsql call below connects over TCP: the CI service
# container listens on localhost:5433 and has no unix socket (Invoke-CimmeriaPsql adds -h
# only with -UseDocker). psql and pg_dump read PGHOST.
$env:PGHOST = 'localhost'

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
# Case 3 (end to end, -ForceDatabase path): the routing for a loaded database with
# accounts is Refresh; running it keeps the sentinel account and character. Uses the
# private resolver and refresh functions directly, not Initialize-CimmeriaDatabase (see
# the PP-14 report). Database sgw_pp14_force, never `sgw`.
$db = 'sgw_pp14_force'
try {
    NewGameDb
    AddSentinel
    $state = & $mod { param($p, $port, $d) Get-CimmeriaDatabaseState -PsqlExe $p -Port $port -Database $d } $psql $port $db
    $action = & $mod { param($f) Resolve-CimmeriaDatabaseAction -Force -SchemaExists:$f.SchemaExists -SeedComplete:$f.SeedComplete -HasAccounts:$f.HasAccounts } $state
    Check 'case3 -Force routes to Refresh' ($action -eq 'Refresh') "got $action"
    if ($action -eq 'Refresh') {
        & $mod { param($p, $port, $d, $bd) Invoke-CimmeriaResourcesRefresh -PsqlExe $p -Port $port -Database $d -BackupDir $bd | Out-Null } $psql $port $db $backupDir
    }
    Check 'case3 sentinel account survives' ((Q "SELECT count(*) FROM account WHERE account_id = 1879048193") -eq '1')
    Check 'case3 sentinel character survives' ((Q "SELECT count(*) FROM sgw_player WHERE player_id = 1879048194") -eq '1')
    Check 'case3 _prev exists' (DbExists "${db}_prev")
    # Carried rows keep their ids, so every sequence must sit at or above max(column), else the
    # next new character or account collides with a carried row.
    Check 'case3 player id sequence at max' ((Q "SELECT last_value >= (SELECT max(player_id) FROM sgw_player) FROM sgw_characters_character_id_seq") -eq 't')
    Check 'case3 account id sequence at max' ((Q "SELECT last_value >= (SELECT max(account_id) FROM account) FROM accounts_account_id_seq") -eq 't')
} finally {
    try { DropAll } catch { Write-Host "cleanup: $($_.Exception.Message)" }
    Remove-Item $backupDir -Recurse -Force -ErrorAction SilentlyContinue
}

# Case 4: Invoke-CimmeriaDatabaseWipe (-WipeDatabase). Database sgw_pp13_wipe, never `sgw`.
$db = 'sgw_pp13_wipe'
try {
    NewGameDb
    AddSentinel
    $w = { param($p, $port, $d, $bd, $co) if ($co) { Invoke-CimmeriaDatabaseWipe -PsqlExe $p -Port $port -Database $d -BackupDir $bd -CountsOnly } else { Invoke-CimmeriaDatabaseWipe -PsqlExe $p -Port $port -Database $d -BackupDir $bd } }
    & $mod $w $psql $port $db $backupDir $true | Out-Null
    Check 'case4 counts only keeps the database and writes no backup' ((DbExists $db) -and -not (Test-Path $backupDir))

    # A backup that cannot be written must stop the wipe before the drop.
    $blocker = Join-Path ([IO.Path]::GetTempPath()) "cimmeria-wipe-blocker-$PID"
    Set-Content $blocker 'x'
    $threw = $false
    try { & $mod $w $psql $port $db (Join-Path $blocker 'sub') $false | Out-Null } catch { $threw = $true }
    Remove-Item $blocker -Force -ErrorAction SilentlyContinue
    Check 'case4 failed backup throws' $threw
    Check 'case4 failed backup leaves the database and sentinel' ((DbExists $db) -and (Q "SELECT count(*) FROM sgw_player WHERE player_id = 1879048194") -eq '1')

    $backup = & $mod $w $psql $port $db $backupDir $false | Select-Object -Last 1
    Check 'case4 wipe returns a non-empty backup' ($backup -and (Test-Path $backup) -and (Get-Item $backup).Length -gt 0) "got $backup"
    Check 'case4 wipe drops the database' (-not (DbExists $db))

    # A later refresh prunes its own dumps only, even with -BackupKeep 0.
    NewGameDb
    & $mod { param($p, $port, $d, $bd) Invoke-CimmeriaResourcesRefresh -PsqlExe $p -Port $port -Database $d -BackupDir $bd -BackupKeep 0 | Out-Null } $psql $port $db $backupDir
    Check 'case4 refresh prune keeps the wipe backup' ($backup -and (Test-Path $backup))
} finally {
    try { DropAll } catch { Write-Host "cleanup: $($_.Exception.Message)" }
    Remove-Item $backupDir -Recurse -Force -ErrorAction SilentlyContinue
    $db = 'sgw_pp13_refresh'
}

# Case 5: the state check fails closed. A database it cannot query must throw, not read as
# "no schema" (-Force drops a database that looks empty).
$threw = ''
try { & $mod { param($p, $port) Get-CimmeriaDatabaseState -PsqlExe $p -Port $port -Database 'sgw_pp13_no_such_db' | Out-Null } $psql $port } catch { $threw = $_.Exception.Message }
Check 'case5 state of an unreadable database throws' ($threw -match 'Cannot read the state') $threw
if ($failed) { Write-Host "$failed check(s) failed"; exit 1 }
Write-Host 'all checks passed'
