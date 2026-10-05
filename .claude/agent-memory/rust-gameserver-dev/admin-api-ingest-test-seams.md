---
name: admin-api-ingest-test-seams
description: How to test an admin-api ingest route without dev-dependencies - cross-module mint, per-thread capture, unstarted Orchestrator, literal targets, masked rule tests
metadata:
  type: project
---

Facts learned building the launcher-summary ingest (`routes/telemetry/launcher_summary/`, 2026-10-04). They apply to any new admin-api route that verifies a dev-session token.

- **`mint_inner` cannot be re-exported.** It is `pub(super)` inside the private `dev_session::handlers`, so `pub(crate) use` of it is a compile error. Tests under `routes/telemetry` mint through the `#[cfg(test)] dev_session::mint_for_test` wrapper instead. Hand-built `TokenClaims` in an ingest test would not notice a change to what the mint issues.
- **`tracing::subscriber::with_default` is per thread.** A capture layer installed on the test thread sees nothing from a spawned thread. A two-thread test gives each thread its own `with_default` over clones of one `Arc<Mutex<Vec<Row>>>` layer.
- **The admin router can be served in a test.** `Orchestrator::new(Default::default())` builds without a database, a socket or a runtime, as long as nothing calls `start_all`. That lets a route-exposure test nest the real `routes::api_routes()`. admin-api cannot name `ServerConfig` (no `cimmeria-common` dependency), hence `Default::default()`.
- **Socket tests read env without the lock.** An async test cannot hold `env_lock()` across an await (clippy `await_holding_lock`). Under `cargo test` it shares the process with tests that flip `CIMMERIA_TELEMETRY_KILL_SWITCH`, so it asserts "401 or 503", and no test may set a small quota knob.
- **`target: "…"` must be a literal.** `crates/server/src/logging/target_scan_tests.rs` skips const targets. Export a `pub const` for other crates, write the literal in the macro, and pin the two together in a row test.
- **A loop over `CLIENT_TARGETS` guards nothing.** `client_replays_land_only_in_the_client_index` and `is_client_target_matches_the_replay_targets_only` pass whatever the list holds. A new client target needs an assertion that spells the target out.
- **One mutation per rule, or the test is masked.** "`error_code: null` on a failed summary" still passed with null-handling removed, because the missing-code rule rejected it anyway. Mutate the validator once per rule and watch the named test fail; `phases` longer than 32 can never be isolated (four timed phases, no repeats).
- **Wait loops must not `pgrep -f` their own text.** `until ! pgrep -f "lane.sh cargo …"` matches the loop's own command line and never ends. Poll the task's output file for `status=` instead.
