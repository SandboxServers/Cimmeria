# Launcher observability implementation handoff (track O)

> **Type:** Reference
> **Audience:** The maintainer and reviewers of the launcher campaign
> **Date:** 2026-10-04
> **Companions:** [Assignment](observability-implementation-assignment.md), [discovery](observability-discovery.md), [design reference](../../../../architecture/launcher-summary-telemetry.md), [desktop contract](../../../../../crates/launcher/desktop/docs/launcher-summaries.md), [operator views](../../../../operations/signoz/launcher-summary-views.md)

The consented launcher-summary flow is implemented from the journal to local
ingest and query fixtures. Nothing is activated: the distributed build has no
endpoint, so no launcher collects or sends anything.

Two owner decisions on 2026-10-04 changed the work after the first draft PR:

1. **The upload is anonymous.** "Allow anonymous upload of launcher telemetry
   data, but only our structured payload; anything else gets rejected. Put a
   low rate limit on it." The token, the summary session kind and the scope
   from the first design are gone.
2. **This session took over the coordinator's role** for the track, so the
   files the assignment reserved for the coordinator were edited here.

## Revision boundary

- Base: `ba4b20b6bc97b8cacbdffa78acb6c199f881965f` (`prototype/launcher-packaging-proof`).
- Branch: `launcher/observability-implementation`, draft PR #1205.

| Commit | Contents |
|---|---|
| `1f276aa94095ea0027a833b6e3fd86a2cbc98bd3` | Owned new files (first, token-based design) |
| `5fff618f9c8cd95f1e10da0e0a6ea659851ec3a9` | Shared integration edits |
| `25ae129751ca434f75ee3dd90f50d5351b43b635` | Public login-listener mount, alone |
| `35f29c0ad396e47bbecae18865f07b2f3587395e` | Docs and first handoff |
| `4361679e796702de32dd1c7f0a71574c972e74dc` | Merge of the integration branch at `ece32e585517c82c54e8991b15b3263b6aa03aff` |
| `36067b517db76a787eec3f5a820ccfcb74adbbeb` | Anonymous, strict, low-rate-limited upload; coordinator items |
| `98398690d` and this note's commit | Docs, indexes, ledger, this worknote and project memory |

No history was rewritten. The merge commit exists because the integration
branch moved and a conflicted PR gets no CI. It keeps both sides of the two
conflicting hunks in `engine/src/storage/mod.rs`, and it leaves the base's new
`OperationKind::Adopt` unsummarized, because wire schema v1 has no value for it.

Only full trees were built and tested, never the commits one by one.

## Delivered behaviour

- **Producer.** A journal observer in `engine/src/storage/mod.rs` tracks each
  operation from its admission and builds one terminal summary per attempt.
  Attempt and event ids are engine-generated; the renderer's operation id is
  never exported.
- **Consent.** The existing `launcher_summary_consent` preference is the only
  switch, default off. An attempt is eligible only if the gate was open when it
  was admitted. Opt-out closes the gate and aborts the request in flight before
  the preferences write, then empties the queue; if the write fails the gate
  stays closed for the run. Opt-in empties the queue first.
- **Queue.** One whole-file atomic `launcher-summaries.json`: at most 64
  entries, 24-hour TTL, acknowledged removal, disposable on any read error.
- **Exporter.** One task that posts once per attempt, with no `Authorization`
  header and a pinned header set. 2 s timeout, at most two retries, a 429 ends
  the cycle without a retry. It is not started when the endpoint is `None`.
- **Phase durations.** Each summary carries a bounded `phases` list measured
  in-process; the server emits one `launcher_phase` row per entry, so attempt
  counts are not double-counted. Timings are omitted after a restart.
- **Server.** `POST /api/telemetry/launcher-summary` takes no token. It accepts
  only the exact v1 JSON object and refuses everything else with a static body.
  The per-address limit defaults to 12 per window and is charged before any
  body is read. Rows are typed INFO records in the `cimmeria-client` index.
- **Operator fixtures.** A dashboard and a saved view under
  `docs/operations/signoz/`, with an offline test that checks every key and
  enum literal against the rows the ingest really emits.

## Assumptions made without an answer

1. **Host.** The session ran in WSL2 with no native Windows Rust toolchain, so
   nothing was built natively on Windows locally. CI is the native evidence.
2. **Rate limit value.** "Low" was read as 12 requests per address per window
   (the existing 3600 s window). Behind a shared address, such as a NAT or a
   tunnel, every launcher shares it and the operator must raise
   `CIMMERIA_TELEMETRY_SUMMARY_QUOTA_PER_IP`.
3. **The anonymous decision is not the public-mount decision.** It was read as
   deciding how the route is exposed, not as the maintainer's yes to serving it
   on the public login listener. That commit stays separate.
4. **Phase durations** use the assignment's "equivalently bounded typed
   representation": a `phases` list on the terminal summary.
5. **Launch rows carry no play-session length.** The journal commits `Running`
   before the game is spawned and the terminal at process exit, so the `running`
   phase and the total duration would be how long the person played. Launch rows
   carry the `starting` (preparation) duration only. Reverse this in
   `launcher_summary/attempt.rs` if session length is wanted.
6. **Finer launch phases are not timed.** Host-started and process-started
   boundaries would need a call inside `engine/src/storage/launch/`.
7. **Lost observation.** Entering `ReconciliationRequired`, including at
   reopen, emits one `unknown` row without timing for the open phase and stops
   tracking. A later reconciled terminal emits nothing.
8. **Install phases.** `download` and `extraction` accumulate across the seed
   and every patch. `extraction` also includes content verification and
   promotion after the last unpack.
9. **Adoption is not summarized** in wire schema v1.
10. **No frontend edit.** The copy "This build sends nothing" stays true.
11. **Shipping.** `ship.sh pr` hardcodes `--base main`, so the branch was
    pushed with git and the draft PR opened with `gh`.

## Coordinator items closed here

- `crates/admin-api/src/lib.rs`: the admin router's request span records the
  URI path only, through the same `request_span` as the login port. This drops
  the query from the span on every admin route.
- `engine/src/storage/install_intent/mod.rs`: install admission finalizes
  pending summary rows before it commits.
- `crates/admin-api/src/routes/dev_session/quota.rs`: `ip_key` keys an
  IPv4-mapped IPv6 peer as its IPv4 address. This also changes the mint and
  refresh quotas on a dual-stack listener.
- `docker/compose.yml`: the kill switch and the summary limit reach the
  container. The compose file was parsed only; no container was started.
- Indexes and records: `docs/readme.md`, the desktop README, the doc-update
  map, `observability.md`, the ledger, the acceptance checklist and the memory
  index.

## Commands and results

All compiling commands ran through `tools/build-lane/lane.sh` on Linux, on the
tree of `36067b517db76a787eec3f5a820ccfcb74adbbeb`.

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | clean |
| `cargo fmt --all --manifest-path crates/launcher/desktop/Cargo.toml -- --check` | clean |
| `cargo clippy -p cimmeria-admin-api -p cimmeria-server --all-targets -- -D warnings` | ok |
| `cargo nextest run -p cimmeria-admin-api -p cimmeria-server` | 236 passed |
| `cargo test -p cimmeria-admin-api --lib` (one process) | 160 passed |
| `cargo clippy --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop --all-targets -- -D warnings` | ok |
| `cargo test --locked --manifest-path crates/launcher/desktop/Cargo.toml -p cimmeria-launcher-engine --target-dir target/desktop` | 423 passed, 4 ignored |

CI on PR #1205 passed all 22 checks at `4361679e796702de32dd1c7f0a71574c972e74dc`
(the token-based design), including `native-engine` on windows-latest and
macos-latest. Those jobs were the first native run of the engine suite and the
first real compile, lint and test of the shell files. Read the PR for the
result at the anonymous head; it is not recorded here.

Guards that were revert-verified (the fix removed from one file, the named test
failing, the file restored): the journal observer call, admission-time
eligibility, the `export_blocked` gate, the exporter's re-checks and its
in-flight cancel, the 429 no-retry rule, the finalize after an install terminal
and at install admission, phase re-entry, the unknown and launch timing rules,
the default limit of 12, the charge before the body read, the content-type,
query and repeated-key refusals, the non-object refusal, address
canonicalisation, the dedup insert and its single lock, the `CLIENT_TARGETS`
entry, the request span on both routers and both router merges.

## Exclusions

- **No local native Windows or macOS run.** See CI above.
- **No workspace-wide local build or nextest**, and no `cargo hakari` check
  (not installed). No manifest or lockfile changed.
- **No live service.** No upload, collector or SigNoz request was made. The
  dashboard and view fixtures were never imported into a SigNoz.
- **No container was started** to check the compose change.
- **No frontend change and no frontend UAT.**

## Known limits

- **Rows are self-reported and anonymous.** Anyone can post correctly shaped
  rows within the limit. They are for spotting failure patterns, never for
  alerts, success rates or SLOs.
- **Invisible attempts.** A failure to open the state, and an attempt that
  never exports within 24 hours, are not reported.
- **Dedup is in memory.** A resend after a server restart is counted twice.
- **The admin span change is wider than this route.** No admin route's request
  span carries its query string any more.
- **Compose passes only the summary limit.** The mint and refresh quota
  variables still do not reach the container.
- **Pre-existing flaky shell test on Linux:**
  `host::repair::integration_tests::host_progress_and_precommit_cancel_preserve_original_without_backup`
  timed out in about half of the local scratch-crate runs, before and after
  this change. It passed in CI.

## Remaining public-activation gate

Two maintainer decisions are open, and neither is implied by this branch:

1. **The login-listener mount** (`25ae129751ca434f75ee3dd90f50d5351b43b635`).
   The recorded decision covers four routes; this is a fifth. Removing that
   commit's merge line leaves the route on the admin listener only.
2. **A production endpoint.** The exporter accepts only `https`, or `http` to
   loopback, and uses bundled webpki roots, so the plain-HTTP login port cannot
   be the endpoint and a publicly trusted certificate is required. A rollout
   packet must also change the consent copy and add frontend UAT.
