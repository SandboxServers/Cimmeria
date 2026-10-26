# launcher-cache-presync work packets: wave 2

> **Type:** Reference
> **Audience:** The orchestrator, ops, packet-coders and packet-reviewers
> **Last updated:** 2026-10-26 (split from work-packets.md; re-specced for the owner's answers)
> **Companions:** [Ledger](README.md), [work packets: dispatch rules, rules R1-R9, checks, contract, wave 1](work-packets.md), [waves 3 to 5](packets-wave3.md), [resume card](RESUME.md)

Every packet here carries rules R1-R9, the checks table and the contract excerpts it names from [work-packets.md](work-packets.md). The orchestrator gives the packet-coder both: this packet's section, and the rules, checks and contract sections there.

## Wave 2 (gated on the wave 1 merge; LP-10 also on LP-05 "compose works")

D-LP1 and D-LP2 were decided by the owner on 2026-10-26: anonymous routes on 8081 with the per-IP quota (default 60 a minute) and the kill switch; changed entries only, never whole PAKs.

### LP-10 Login-port routes `/api/cooked/v1/*`

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-10-routes`, at `main` · **Worktree:** `lcp-10`
- **Commit:** `feat(admin-api): anonymous cooked-data presync routes on the login port (LP-10)`
- **Rules:** R1-R9. Contract: § Server routes table, the env vars, and the server telemetry rows.

Files and changes:

1. `crates/admin-api/Cargo.toml`: add `cimmeria-resources = { workspace = true }` and `cimmeria-cooked-pak = { workspace = true }`.
2. New `crates/admin-api/src/routes/cooked/mod.rs` (`pub fn routes() -> Router`), `handlers.rs` and `tests.rs`, modelled on `routes/telemetry/launcher_summary/`:
   - `handlers.rs` holds `index_inner(catalog: Option<Published>, disabled: bool, admit: bool)` and `category_inner(..., id, query)`. They are pure, so the tests can inject a catalog built with `ServedCatalog::from_bodies` (enable `cimmeria-resources/test-support` in dev-deps).
   - Check order: quota (429), kill switch (503 `disabled`), catalog state (`None` gives 503 `catalog_unavailable` + `Retry-After: 5`; `Some(Failed(_))` gives 503 `catalog_failed` with no `Retry-After`), then the route's own checks.
   - The axum handlers read `served_catalog::current()`, the env switch and the quota, then call the inner functions.
   - Each response emits `cooked_presync.served`, `.index_served` or `.refused` (route, reason, status, category_id) and bumps `cooked_presync_requests_total`. The refusal `reason` is the JSON error code, so the launcher's failure and the server's refusal share one code.
3. `crates/admin-api/src/routes/mod.rs`: `pub mod cooked;`.
4. `crates/admin-api/src/login_port.rs`:
   - nest `.nest("/cooked", cooked::routes())` under `/api`, beside `/telemetry`;
   - update the module doc ("Only these routes are exposed") and the fn doc to name the two routes and D-LP1;
   - in `login_port_router_does_not_expose_admin_routes`, keep every 404 path, and add `/api/cooked/v1/admin` and `/api/cooked/v2/index`.

Tests, in `routes/cooked/tests.rs`:

- One test per row of the Contract's routes table, through the inner functions: 200 index with `Cache-Control: no-cache`; 200 body that decodes with `decode_body`; 503 unavailable with `Retry-After: 5`; 503 `catalog_failed` with no `Retry-After` on both routes; 503 disabled; 404; 409 carrying the served version; 400.
- `the_kill_switch_wins_over_a_failed_catalog`: disabled plus `Failed` gives `disabled` (the launcher skips on it and launches; D-LP19). It fails if the check order changes.
- `quota_refuses_the_61st_request_in_a_minute`.
- In `login_port.rs` tests: `login_port_router_answers_the_cooked_presync_routes` through `serve()` and `exchange`, 503 before any publish.

Each test fails if its route or check is removed.

Checks: M1, M2, M3 `cimmeria-admin-api`; H1.

Docs rows: 58 (admin-api route table) is owed by LP-12.

### LP-11 Publish the catalog at base-service start

- **Host:** windows · **Agent:** packet-coder · **Branch:** `launcher-cache-presync/LP-11-publish`, at `main` · **Worktree:** `lcp-11`
- **Commit:** `feat(base): build and publish the presync catalog off the boot path (LP-11)`
- **Rules:** R1-R9.

Files and changes:

1. `crates/base/src/base/service.rs` at l.318: once `ResourceCache::load_all` gives `Ok(cache)` and the `Arc` exists, call a new `fn spawn_presync_catalog(cache: Arc<ResourceCache>, data_dir: String)`. It does `tokio::task::spawn_blocking`, then `ServedCatalog::build`. On `Ok` it publishes `Published::Ready` and logs INFO `cooked_presync.catalog_ready` (categories, body_bytes, duration_ms). On `Err(e)` it publishes `Published::Failed(failure_code(&e))` and logs WARN `cooked_presync.catalog_unavailable` (reason = the code, category_id when the error names one). Publishing the failure is what turns the launcher's wait into an immediate `catalog_failed` (D-LP13). Boot never waits on it. A panicking build task publishes `Failed("build_failed")` (match the `JoinError`).
2. If `service.rs` is near 500 lines, put the fn in a new sibling `crates/base/src/base/presync_catalog.rs`.
3. `crates/base/Cargo.toml`: add `cimmeria-resources` if it isn't a direct dependency already (re-grep).

Test: in the new fn's module, `#[tokio::test] a_built_catalog_is_published_and_logged`:

- build a tiny `ResourceCache` from a temp `data_dir`, the same way LP-03's tests do;
- call the build-and-publish helper, factored as `fn build_and_publish(cache, data_dir, publish: impl FnOnce(Published) -> bool)`;
- assert `catalog_ready` is logged and the publish closure got `Ready` with the expected categories.

Second test, `a_failed_build_publishes_failed_and_warns`: remove the raw PAK from the temp `data_dir` after the cache loads; assert the closure got `Failed("raw_pak_unreadable")` and the WARN was logged. It fails if a failed build publishes nothing (the launcher would wait for `catalog_unavailable` until its deadline).

Both fail if the helper stops publishing.

Checks: M1, M2, M3 `cimmeria-base presync`.

Docs rows: none (LP-12).

### LP-12 Server docs

- **Host:** windows · **Agent:** documentation-writer · **Branch:** `launcher-cache-presync/LP-12-server-docs`, at the `lcp-w2` head after LP-10 and LP-11 · **Worktree:** `lcp-12`
- **Commit:** `docs: cooked-data presync server side (LP-12)`
- **Rules:** R1, R3, R4, R5, R7, R8.

Files:

1. `docs/engine/cooked-data-pipeline.md`:
   - a "Launcher presync" section: catalog build, routes, the reply counter. State plainly that the launcher **blocks Play** when the presync fails (D-LP18), that `cooked_sync` stays on the server for other clients but the launcher never falls back to it, and that a failed catalog build makes every launcher on that server fail with `catalog_failed` until it is fixed;
   - an operator note: the kill switch `CIMMERIA_COOKED_PRESYNC_DISABLED=1` is the bypass. With it set, launchers skip the presync with a visible note and launch (D-LP19, D-LP24). Note the shared-IP risk: players behind one NAT share the 60-a-minute quota (F20); a full sync costs about 1 + the category count requests, and the launcher waits out a short `Retry-After`;
   - an alert recipe: `cooked_presync_requests_total{outcome="catalog_failed"}` above 0, or the WARN `cooked_presync.catalog_unavailable`;
   - fix the category 7 note at l.204-209 (D-LP14).
2. `docs/engine/cooked-data-pak-format.md`:
   - the PAK layout `write_pak` produces;
   - the index JSON, the `CPS1` body and the `CPD1` digest;
   - state that the digest is integrity, not authenticity (D-LP10).
3. `docs/tools/admin-api.md`: the two routes and both env vars, in the login-port route table (row 58).
4. `docs/architecture/observability-target-catalog.md`: every server row of the Contract's telemetry table, outside any gen block (row 36). If that table is generated, say so and stop.
5. `docs/operations/telemetry.md`: the two counters, and a SigNoz query recipe for "resyncs avoided vs fired": `cooked_data_version_replies_total` by `outcome`. Also add the env vars wherever the summary-quota var is documented (`container.md`, `colo-deploy.md`; re-grep `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`), and in the env table comment in `crates/server/src/main.rs` (a comment-only hunk).
6. `docs/tools/` or the tools catalog: `cooked-pak-tool` usage (row 20; only outside gen blocks).

Checks: each edited doc passes the R5 check; `python tools/docs-gen/regen.py --check` if it exists (re-grep its usage), without committing its output.
