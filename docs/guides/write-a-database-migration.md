---
title: How to write a database migration
type: how-to
audience: engineers (schema changes, seed data, content additions)
last_updated: 2026-05-27
companion_docs:
  - ../../db/README.md
  - ../architecture/integration-test-infra.md
  - ../../TESTING.md
---

# How to write a database migration

Cimmeria has 348 SQL files under [`db/`](../../db/) organised into per-system directories. Most of them are the canonical schema (loaded on fresh database init). A handful are migrations under [`db/scripts/`](../../db/scripts/) (run against existing databases without destroying data). This guide explains the difference and walks through writing each kind.

If you're adding **seed content** rather than changing the schema (a new mission row, a new ability), skip to the "Adding seed content" section near the bottom.

---

## Schema vs. migration vs. seed

Three different kinds of SQL change. Pick the right one.

| Change | Where it goes | When it runs | Idempotent? |
|---|---|---|---|
| **Schema** (new table, new column, new index) | `db/sgw/` or `db/resources/<system>/Tables/` | Fresh-DB load via `setup.ps1` | No — re-running on the same DB errors |
| **Migration** (apply schema change to an existing DB without dropping data) | `db/scripts/<descriptive_name>.sql` | Manually, or by the operator during an upgrade | **Yes** — must use `IF NOT EXISTS` / `IF EXISTS` |
| **Seed content** (new row in an existing table) | `db/resources/<system>/Seed/` or `db/scripts/` | Fresh-DB load, or migration | Mostly yes — `ON CONFLICT DO NOTHING` |

The reason we keep both schema and migration is: developers nuke and reload their DBs constantly (the schema is canonical for that), but operators running production servers can't drop their data (so they need an idempotent migration to apply the same change).

**Most schema changes require both.** Update the canonical schema file *and* write a migration script.

---

## Schema changes

For a new table or column, you almost always touch two files: the canonical schema and a migration script.

### 1. Update the canonical schema

Find the right home. Use [`db/README.md`](../../db/README.md) to identify the system:

- Player-state tables (accounts, characters, inventory rows, mission progress) → [`db/sgw/`](../../db/sgw/) (in the appropriate `Tables/` file).
- Content tables (abilities, effects, items, dialogs, missions) → [`db/resources/<system>/Tables/`](../../db/resources/).

Add the table or column to the canonical file directly. Don't use `IF NOT EXISTS` here — the schema is loaded against a fresh DB.

### 2. Write the migration

Migrations live in [`db/scripts/`](../../db/scripts/). Naming convention is descriptive lowercase with underscores — examples already in the tree:

- `add_login_audit.sql`
- `add_bandolier_cur_ammo_type.sql`
- `add_cell_event_outbox.sql`

Template:

```sql
-- Migration: <one-line description of what this changes and why>.
-- Safe to run on existing databases (uses IF NOT EXISTS / IF EXISTS).
-- See db/<canonical schema path> for the canonical definition.

CREATE TABLE IF NOT EXISTS your_new_table (
    id              BIGSERIAL PRIMARY KEY,
    -- columns ...
);

CREATE INDEX IF NOT EXISTS idx_your_new_table_lookup
    ON your_new_table (some_column);
```

For adding a column to an existing table:

```sql
-- Migration: add foo_bar column to bandolier slots.
-- Safe to run on existing databases (uses IF NOT EXISTS via DO block).

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'bandolier_slots' AND column_name = 'foo_bar'
    ) THEN
        ALTER TABLE bandolier_slots ADD COLUMN foo_bar INTEGER NOT NULL DEFAULT 0;
    END IF;
END $$;
```

PostgreSQL doesn't have `ALTER TABLE ... ADD COLUMN IF NOT EXISTS` in older versions; the `DO $$ BEGIN ... END $$` block is the portable pattern.

### 3. Reference the canonical schema from the migration

The first comment line should point at the canonical schema file. `add_cell_event_outbox.sql` is the model:

```sql
-- Migration: add cell_event_outbox table for durable base→cell content events.
-- Safe to run on existing databases (uses IF NOT EXISTS).
-- See db/sgw/Outbox/Tables/cell_event_outbox.sql for the canonical definition
-- and rationale.
```

When the migration is applied months later by an operator, they need to be able to find the canonical definition to understand intent.

---

## Adding seed content

For new content rows — a new ability, a new mission, a new dialog — you usually add to the **seed** SQL rather than writing a migration. Seeds are in `db/resources/<system>/Seed/`.

The pattern is `INSERT ... ON CONFLICT DO NOTHING` so re-running is idempotent:

```sql
INSERT INTO abilities (id, name, /* ... */)
VALUES (12345, 'YourNewAbility', /* ... */)
ON CONFLICT (id) DO NOTHING;
```

If the content is **fixing existing data** rather than adding new data, use `ON CONFLICT (id) DO UPDATE` or write a targeted `UPDATE`. Be specific about the conflict target and avoid clobbering operator customisations.

For content that should ship to existing databases too, mirror the seed into a migration in `db/scripts/` (see `add_health_slappack_use_chain.sql` as an example of a content-fix migration).

---

## Loading and applying

### During `setup.ps1`

Canonical schema files in `db/sgw/` and `db/resources/` load automatically on fresh DB init or `-ForceDatabase`. You don't need to register them anywhere.

### Migrations

`db/scripts/` files are **not** auto-applied. Operators apply them manually:

```powershell
$env:PGPASSWORD = "w-testing"
psql -h localhost -p 5433 -U w-testing -d sgw -f db/scripts/add_your_migration.sql
```

There is no migration framework (intentionally — the project's small enough that this overhead isn't justified). If you change this, update [`db/README.md`](../../db/README.md), [`bootstrap/README.md`](../../bootstrap/README.md), and the [`docs/operations/`](../operations/) runbooks.

---

## Test the change

Per [`TESTING.md`](../../TESTING.md), schema and content changes need **live-DB regression guards**:

```rust
#[tokio::test]
async fn your_new_table_persists_correctly() {
    let pool = require_db_or_skip!();
    cleanup_sentinel(&pool, SENTINEL_ID).await;

    // Insert via your handler
    your_handler(&pool, SENTINEL_ID).await.unwrap();

    // Verify the row exists with the expected shape
    let row: YourRow = sqlx::query_as("SELECT * FROM your_new_table WHERE id = $1")
        .bind(SENTINEL_ID)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row.some_column, expected);

    cleanup_sentinel(&pool, SENTINEL_ID).await;
}
```

Three non-negotiables:

1. **Use `require_db_or_skip!`** so the test self-skips when `DATABASE_URL` is unset.
2. **Sentinel IDs fit in `i32`** and are unique per test. Don't reuse sentinels across tests in the same crate.
3. **Cleanup deletes by exact sentinel**, not by range. Range deletes will collide with other tests.

The full pattern is in [`docs/architecture/integration-test-infra.md`](../architecture/integration-test-infra.md) and [`crates/services/src/test_support.rs`](../../crates/services/src/test_support.rs).

---

## Verify locally before pushing

```powershell
# 1. Apply your migration to a clean DB:
pwsh setup.ps1 -SkipBuild -ForceDatabase -NoLaunch

# 2. Apply the migration script (simulating an operator):
$env:PGPASSWORD = "w-testing"
psql -h localhost -p 5433 -U w-testing -d sgw -f db/scripts/add_your_migration.sql

# 3. Run the live-DB tests:
$env:DATABASE_URL = "postgres://w-testing:w-testing@localhost:5433/sgw"
cargo nextest run --profile=ci-live-db -p cimmeria-services --lib

# 4. Verify idempotency — run the migration a second time, expect no errors:
psql -h localhost -p 5433 -U w-testing -d sgw -f db/scripts/add_your_migration.sql
```

If step 4 errors, your migration isn't idempotent. Fix it before pushing — operators *will* re-run.

---

## Update the docs

Per the CLAUDE.md doc-update map, schema changes that operators need to know about update:

- [`db/README.md`](../../db/README.md) — if you're adding a new schema directory or table family.
- The relevant runbook in [`docs/operations/`](../operations/) — if operators need to apply the migration as part of an upgrade.
- The relevant per-system doc under [`docs/gameplay/`](../gameplay/) or [`docs/content/`](../content/) — if the change affects user-visible behaviour.

For a routine column addition with a migration script, the file itself is the documentation. For a new table or a non-trivial schema change, add a row in `db/README.md` and a brief note in the system's doc.

---

## See also

- [`../../db/README.md`](../../db/README.md) — overall schema organization.
- [`../architecture/integration-test-infra.md`](../architecture/integration-test-infra.md) — live-DB test patterns, sentinel discipline.
- [`../../TESTING.md`](../../TESTING.md) → "Live-DB" type — the test shape you need.
- [`add-a-message-handler.md`](add-a-message-handler.md) — for handlers that consume the new schema.
- [`extend-the-content-engine.md`](extend-the-content-engine.md) — for content-engine seed work.
