# Integration Test Infrastructure

> **Last updated**: 2026-07-25
> **Audience**: Engineers writing tests against PostgreSQL or any other
> live external dependency
> **Type**: Architecture decision + how-to
> **Owner**: Test infrastructure
> **Resolves**: #79 (the "sqlx::test vs testcontainers" call)

## Decision

**Tests run against a developer-supplied `DATABASE_URL`. We do NOT use
testcontainers, and we do NOT use `sqlx::test` macros.**

Integration tests opt in at runtime by reading `DATABASE_URL`. When the
variable isn't set, tests skip with a clear message rather than fail.

## Why not testcontainers

- The test runner spins up a real Docker container per test run — adds
  ~1–3s per `cargo test` invocation even when no DB tests are queued.
- Windows is the primary dev platform for this repo. Docker Desktop on
  Windows adds resource pressure on the same machine that runs the
  game client; making `cargo test` Docker-dependent raises the barrier
  to running tests at all.
- The maintenance surface of containers (image pinning, network setup,
  port collision under parallel test runs) is high relative to the
  value when devs already need a local PostgreSQL for running the
  server itself.

## Why not `sqlx::test`

- The macro template-clones a fresh database per test. The Cimmeria
  schema is large (~hundreds of files included from `db/database.sql`).
  Template creation is a one-time cost but per-test cloning still
  measurably slows the suite.
- The macro hides the test setup behind attribute magic. Devs writing
  their first integration test have to learn the macro's fixture
  conventions, the `DATABASE_URL` discovery rules, and the way it
  forks pools. Plain function calls are easier to read and debug.
- Lock-in to the macro complicates parallel testing patterns (e.g.,
  testing the outbox drainer's "two concurrent passes" behavior, where
  we want explicit control over the connection topology).

## What we do instead

Live-DB tests live alongside their target module's existing
`#[cfg(test)] mod tests;` block — not in `crates/services/tests/`.
The reason is access: most of what we want to integration-test is
internal SQL behavior (transaction boundaries, advisory locks,
rows_affected invariants). Cargo's `tests/` directory only sees the
crate's `pub` API, which would force us to make implementation
modules `pub` purely for testing — leaking surface for no
production gain.

Each live-DB test gates on `DATABASE_URL` at runtime via the
`require_db_or_skip!` macro, which wraps the pool open and the
skip-with-reason branch:

```rust
use crate::test_support::require_db_or_skip;

#[tokio::test]
async fn outbox_round_trip_against_real_db() {
    let pool = require_db_or_skip!();
    // ... real-DB assertions ...
}
```

`test_support::test_pool()` returns
`Result<PgPool, SkipReason>`, where `SkipReason` distinguishes
`NotConfigured` (DATABASE_URL unset or empty — silent skip) from
`ConnectFailed(String)` (variable set but `connect()` failed —
surfaces sqlx's underlying error so the developer can fix it). The
macro logs the reason via `eprintln!("{module_path}: skipping
live-DB test ({reason})")` and returns from the test on either
shape. The unit-test suite stays green on a fresh checkout — only
`DATABASE_URL=postgres://… cargo test` exercises the integration
path.

Each test is responsible for its own data isolation: either work
inside a transaction it rolls back at the end (works for tests that
don't need to span their own commit boundary), or pick a sentinel
from the module's reserved `0x7000_xxxx` slot and delete its own
rows on cleanup. The reserved-slot scheme is documented per-module
(see `crates/services/src/base/character/mod.rs:276-281` and
`crates/services/src/base/world_entry/methods/missions.rs:146-148`
for the canonical doc-comment shape) and is also summarised in the
"Sentinel id discipline" section of [TESTING.md](../../TESTING.md).

## Setup for local dev

1. Install PostgreSQL 17 locally (matches production target). The
   bootstrap module under `bootstrap/CimmeriaBootstrap/` already does
   this for the server; reuse the same install. **Note**: the
   bootstrap binds Postgres to port **5433** (not the default 5432)
   to avoid clashing with any system Postgres install. URLs below
   reflect that.
2. Create a dedicated test database:
   ```sql
   CREATE DATABASE cimmeria_test;
   ```
3. Load the schema into it:
   ```bash
   psql -U <user> -p 5433 -d cimmeria_test -v ON_ERROR_STOP=1 -f db/database.sql
   ```
4. Export the connection string before running tests:
   ```bash
   export DATABASE_URL=postgres://<user>:<pw>@localhost:5433/cimmeria_test
   cargo test -p cimmeria-services
   ```

For PowerShell:
```powershell
$env:DATABASE_URL = "postgres://<user>:<pw>@localhost:5433/cimmeria_test"
cargo test -p cimmeria-services
```

If your local Postgres is on the default 5432, drop `:5433`. The
constraint is "whatever port your test DB is actually on."

Live-DB tests are interleaved with unit tests under the same `cargo
test` invocation — they self-skip when `DATABASE_URL` isn't set, so
the same command works in both modes. To run only the outbox subset:

```bash
cargo test -p cimmeria-services outbox
```

(filters by test name, which matches both the unit `outbox::tests::*`
and the live-DB `outbox::tests::enqueue_*` cases).

## Test isolation

Tests share one database. Strategies for keeping them from stepping
on each other:

- **Transaction rollback** (preferred for read/write tests). Wrap the
  test body in `pool.begin()`, do all work against the `&mut Transaction`,
  return without commit. Postgres rolls back on drop. Works for any
  test that doesn't span its own commit boundary.
- **Per-test row scoping**. Pick a unique sentinel (random `entity_id`,
  fresh `account_name`, etc.) and delete by sentinel in a `Drop` guard
  or test cleanup. Required when the test path itself commits internally
  (e.g., outbox enqueue + drain in two separate connections).

The outbox pilot tests (`crates/services/src/base/outbox/tests/`)
demonstrate both patterns — `enqueue_in_tx` runs inside a rolled-back
tx, while the round-trip test commits real rows and cleans them up by
sentinel `entity_id`.

## When this might change

- If we add a CI workflow that runs integration tests, we'll likely
  use a Postgres service container in the workflow file (GitHub
  Actions / Azure Pipelines have first-class support for this without
  introducing testcontainers as a dependency).
- If parallel-test contention becomes a real problem (today the
  integration suite is small enough that serialising via the
  `ci-live-db` nextest profile, or `cargo test ... -- --test-threads=1`,
  on the integration target is acceptable), revisit `sqlx::test`'s
  per-test-DB mode.

## Future work

- [ ] CI wiring (separate PR — covered in #123 Group D and existing
      issue #75 for pipeline scope).
- [ ] First Group A regression tests for vendor / inventory / outbox
      built on top of this scaffolding (pilot test ships with this PR;
      the rest follows in batched PRs against #79).
