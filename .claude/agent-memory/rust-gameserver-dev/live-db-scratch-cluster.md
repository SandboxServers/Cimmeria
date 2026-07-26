# Standing up a throwaway Postgres for live-DB revert-verification

`db.bat init` only runs `initdb` — it does **not** create the `sgw` database,
the `w-testing` role, or load `db/database.sql`. And its `PGDATA` lives under
`external/`, which is shared with the main checkout (and with any other agent
session junction-linking it). Don't touch it from a worktree.

Instead build an isolated cluster in the scratchpad using the bundled 17.9
binaries. Needed whenever you have to *actually run* a live-DB guard —
`require_db_or_skip!` makes a skipped test look green, so revert-verification
is impossible without a real DB.

```powershell
$bin  = "C:\Users\Steve\source\projects\Cimmeria\external\postgresql_server\bin"
$data = "<scratchpad>\pgdata"
& "$bin\initdb.exe" -D $data -U postgres -E UTF8 --locale=C -A trust
Add-Content "$data\postgresql.conf" "port = 5544"
Add-Content "$data\postgresql.conf" "listen_addresses = 'localhost'"
& "$bin\pg_ctl.exe" -D $data -l "$data\pg.log" -w start
& "$bin\psql.exe" -h 127.0.0.1 -p 5544 -U postgres -d postgres `
    -c "CREATE ROLE ""w-testing"" LOGIN SUPERUSER PASSWORD 'w-testing';" `
    -c "CREATE DATABASE sgw OWNER ""w-testing"";"
```

Then load the schema from the repo root (psql `\ir` paths are relative to
`db/database.sql`, so run it from the worktree root, not from `db/`):

```bash
PGPASSWORD=w-testing psql -h 127.0.0.1 -p 5544 -U w-testing -d sgw \
  -v ON_ERROR_STOP=1 -q -f db/database.sql
```

Run with `DATABASE_URL=postgres://w-testing:w-testing@127.0.0.1:5544/sgw`.
Stop with `pg_ctl -D $data -w stop` when done — don't leak the process.

Timings on this host: `initdb` + start exceeds the 300s Bash-tool default
(run it backgrounded); schema load ~1 min; full `ci-live-db` suite ~54s for
1,687 tests.
