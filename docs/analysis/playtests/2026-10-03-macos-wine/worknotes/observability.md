# Launcher observability implementation handoff (track O)

> **Type:** Reference
> **Audience:** Launcher coordinator, the maintainer and reviewers
> **Date:** 2026-10-04
> **Companions:** [Assignment](observability-implementation-assignment.md), [discovery](observability-discovery.md), [design reference](../../../../architecture/launcher-summary-telemetry.md), [desktop contract](../../../../../crates/launcher/desktop/docs/launcher-summaries.md), [operator views](../../../../operations/signoz/launcher-summary-views.md)

The consented launcher-summary flow is implemented from the journal to local
ingest and query fixtures. Nothing is activated: the distributed build has no
endpoint, so no launcher collects or sends anything, and the public
login-listener mount waits for the maintainer.

## Revision boundary

- Base: `ba4b20b6bc97b8cacbdffa78acb6c199f881965f` (`prototype/launcher-packaging-proof`).
- Branch: `launcher/observability-implementation`. Integrate the chain once, in order, without squashing:

| Commit | Contents |
|---|---|
| `1f276aa94095ea0027a833b6e3fd86a2cbc98bd3` | Owned new files: engine module, server module, shell `summary.rs`, golden fixtures, SigNoz fixtures |
| `5fff618f9c8cd95f1e10da0e0a6ea659851ec3a9` | Shared integration: edits to existing engine, shell, admin-api and server files |
| `25ae129751ca434f75ee3dd90f50d5351b43b635` | Public login-listener mount, alone, for maintainer review |
| (this note's commit) | Docs, this worknote and project memory; no runtime change |

Only the full chain was built and tested. The first three commits were not
built one by one.

The integration branch moved to `ece32e585517c82c54e8991b15b3263b6aa03aff`
while this ran, and the draft PR conflicted in
`engine/src/storage/mod.rs`. Because a conflicted PR gets no CI, the branch
carries one merge commit of `prototype/launcher-packaging-proof` after the four
commits above. It keeps both sides of the two conflicting hunks (summary
finalize first, then the new updater-idle check) and handles the new
`OperationKind::Adopt`: adoption has no value in wire schema v1, so it is not
tracked or reported, and a test pins that. No history was rewritten.

## Delivered behaviour

- **Producer.** A journal observer in `engine/src/storage/mod.rs` tracks each
  operation from its admission and builds one terminal summary per attempt.
  Attempt and event ids are engine-generated; the renderer's operation id is
  never exported. Install, runtime setup, repair, uninstall and launch are
  covered with no hook in their own modules.
- **Consent.** The existing `launcher_summary_consent` preference is the only
  switch, default off. An attempt is eligible only if the gate was open when it
  was admitted. Opt-out closes the gate and aborts the request in flight before
  the preferences write, then empties the queue; if the write fails the gate
  stays closed for the run. Opt-in empties the queue first.
- **Queue.** One whole-file atomic `launcher-summaries.json`: at most 64
  entries, 24-hour TTL, acknowledged removal, disposable on any read error.
- **Exporter.** One task, 2 s timeouts, at most two retries, `Retry-After`
  capped at 60 s, token never stored. It is not started when the endpoint is
  `None`.
- **Phase durations.** Each summary carries a bounded `phases` list
  (`starting`, `running`, `download`, `extraction`) measured in-process. The
  server emits one `launcher_phase` row per entry, so attempt counts are not
  double-counted. Timings are omitted after a restart.
- **Server.** A `launcher_summary` session kind with its own scope and mint
  allowance, a scoped ingest route with per-element validation, in-memory
  dedup, a per-address quota, static error bodies and typed INFO rows routed to
  the `cimmeria-client` index.
- **Operator fixtures.** A dashboard and a saved view under
  `docs/operations/signoz/`, with an offline test that checks every key and
  enum literal against the rows the ingest really emits.

## Assumptions made without a coordinator answer

1. **Host.** The session ran in WSL2. The machine has no native Windows Rust
   toolchain and no MSVC build tools, so nothing was built natively on Windows.
2. **Phase durations** use the assignment's "equivalently bounded typed
   representation": a `phases` list on the terminal summary, not separate queue
   entries. A phase row can therefore never evict a terminal row.
3. **Launch rows carry no play-session length.** The journal commits `Running`
   before the game is spawned and the terminal at process exit, so the `running`
   phase and the total duration would be how long the person played. Launch rows
   carry the `starting` (preparation) duration only. Reverse this in
   `launcher_summary/attempt.rs` if session length is wanted.
4. **Finer launch phases are not timed.** Host-started and process-started
   boundaries need a call inside `engine/src/storage/launch/`, which is outside
   the ownership table.
5. **Lost observation.** Entering `ReconciliationRequired`, including at
   reopen, emits one `unknown` row without timing for the open phase and stops
   tracking. A later reconciled terminal emits nothing.
6. **Install phases.** `download` and `extraction` accumulate across the seed
   and every patch. `extraction` also includes content verification and
   promotion after the last unpack.
7. **Validation.** The server enforces closed enums and bounds, not an
   operation-by-phase matrix.
8. **Token.** The common 8-hour TTL and refresh are unchanged; separation is by
   scope. A summary token's `sub` is a server constant.
9. **New operator knob.** `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`, default
   120 per window, `0` disables. It is charged before the token check, so
   tokenless requests count.
10. **No frontend edit.** The copy "This build sends nothing" stays true.
11. **Shipping.** `ship.sh pr` hardcodes `--base main`, so the branch was
    pushed with git and the draft PR opened with `gh` against the integration
    branch.
12. **Server half** was developed in a second, local-only worktree and merged
    into this branch; that branch was never pushed.

## Files outside the ownership table

- `crates/admin-api/src/routes/dev_session/summary_mint.rs` and
  `summary_mint_tests.rs`: split out so `handlers.rs` stays under the size cap.
- `engine/src/storage/install_worker/summary_tests.rs` and `export_tests.rs`:
  they need the private `dispatch_with` and `publish`.
- `engine/src/storage/launcher_summary/fixtures/request-install-failure.json`:
  the body recorded from a real install worker.

These need a coordinator ownership record.

## Commands and results

All compiling commands ran through `tools/build-lane/lane.sh` on Linux, on the
final tree.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check` | clean |
| `cargo clippy -p cimmeria-admin-api -p cimmeria-server --all-targets -- -D warnings` | ok |
| `cargo nextest run -p cimmeria-admin-api -p cimmeria-server` | 212 passed |
| `cargo test -p cimmeria-admin-api --lib` (one process) | 136 passed |
| `cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop --all-targets -- -D warnings` | ok |
| `cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop` | 407 passed, 3 ignored before the merge; 423 passed, 4 ignored on the merged tree |

Guards that were revert-verified (the fix removed from one file, the named test
failing, the file restored): the journal observer call, the admission-time
eligibility rule, the `export_blocked` gate, the three exporter re-checks, the
in-flight cancel, the finalize after an install terminal, phase re-entry, the
unknown and launch timing rules, the summary scope at mint, the scope check at
ingest, the dedup insert and its single lock, the mint charge order, the
`CLIENT_TARGETS` entry, the login-port request span and both router merges.

## Exclusions

- **No native Windows or macOS run.** The engine suite ran on Linux only. The
  `desktop launcher` workflow on the draft PR is the first Windows and macOS
  run.
- **The shell crate was never compiled with real Tauri.** It does not build on
  this host (no GTK or WebKit). `shell/src/host/**` and most of `main.rs` were
  type-checked in a scratch crate against a Tauri stub; CI is the first real
  compile of `host.rs`, `main.rs` and `host/summary.rs`.
- **No workspace-wide build or nextest run**, and no `cargo hakari` check
  (not installed here). No manifest or lockfile changed.
- **No live service.** No mint, upload, collector or SigNoz request was made.
  The dashboard and view fixtures were never imported into a SigNoz.
- **No frontend change and no frontend UAT.** With no endpoint there is no
  exporter behaviour for the UI to show.
- **The first three commits were not built separately.**

## Known limits and follow-ups

- **Admin listener request span.** `crates/admin-api/src/lib.rs` (not in the
  table) still records the full URI, so a query string on the summary route
  reaches the logs there. The login-port router is fixed.
- **Error code lost on a direct launch-then-install.** Install admission
  commits without passing `operations_mut()`, which is where pending rows are
  finalized. If the exporter is dead, a launch row finalized after an install
  was admitted reports `unspecified`. The fix is one `finalize_summaries()` call
  at the start of `admit_install_with` in `storage/install_intent/mod.rs`
  (not in the table).
- **Invisible attempts.** A failure to open the state, and an attempt that
  never exports within 24 hours, are not reported.
- **Dedup is in memory.** A resend after a server restart is counted twice.
- **Tokenless requests spend the ingest quota**, so anyone behind the same
  address as real launchers can exhaust it. The check order is the spec's.
- **Stale docs the coordinator owns:** `docs/readme.md` does not list the two
  new docs; `crates/launcher/desktop/README.md` still says there is no
  exporter; `docs/agents/doc-update-map.md` has no row for the new design doc;
  `docs/architecture/observability.md` omits `launcher.summary` in its
  client-index paragraph.
- **Pre-existing flaky shell test on Linux:**
  `host::repair::integration_tests::host_progress_and_precommit_cancel_preserve_original_without_backup`
  timed out in about half of the scratch-crate runs, before and after this
  change.

## Remaining public-activation gate

Two maintainer decisions are open, and neither is implied by this branch:

1. **The login-listener mount** (`25ae129751ca434f75ee3dd90f50d5351b43b635`).
   The recorded decision covers four routes; this is a fifth. Dropping the
   commit leaves the route on the admin listener only.
2. **A production endpoint.** The exporter accepts only `https`, or `http` to
   loopback, and uses bundled webpki roots, so the plain-HTTP login port cannot
   be the endpoint and a publicly trusted certificate is required. A rollout
   packet must also change the consent copy and add frontend UAT.
