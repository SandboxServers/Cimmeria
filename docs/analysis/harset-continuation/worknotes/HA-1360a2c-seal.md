# HA-1360a2c seal: world-entry continuation fence

> Type: packet seal. Audience: Haiku packet-coder, reviewers. Sealed 2026-10-18 against campaign tip `9455f2af2` by the mission-systems advisor. Docs-only: nothing here was compiled or run, and no lab or client was used. Parent contract: [HA-1360a seal, A2c paragraph](HA-1360a-seal.md). Companions: [work-packets](../work-packets.md#packet-status-reset), [RESUME](../RESUME.md).

## 1. What the race is (verified in source)

`handle_on_client_ready` (`crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs`, 699 lines, hard cap 700) captures a `SessionSnapshot` (`client_ready/cell_handoff.rs`: shared `cancelled` Arc, `connected_at` generation, token digest of the session key, entry epoch) together with the pending finalization. A2a added `session.is_current(..)` (via the local `current()` closure) at exactly two places after the handoff: once before the burst bundle and once after it. Everything later runs unguarded across awaits:

| Order | Step (mod.rs today) | Guarded today |
|---|---|---|
| 1 | burst bundle (`send_bundle_to_witness_reliable`) | check before only |
| 2 | GM-only redirect line (`send_gm_feedback_to_client`) | no |
| 3 | `push_contact_lists_on_login` (DB + sends) | no |
| 4 | `restore_on_login` (organization) | no |
| 5 | `run_world_entry_hook(ClientReadyAfterOrgRestore)` (crafting plugin) | no |
| 6 | `resync_ignore_cache` | no |
| 7 | `spawn_login_sweep` (mail expiry, spawned) | no |
| 8 | `fanout_login_status` (spawned) | no |
| 9 | first-login `send_cinematic` plus `first_login = 0` UPDATE | no |
| 10 | `cinematic_aoi_hold::on_client_ready` or `flush_deferred_aoi` | no |

The callees resolve their target by `entity_id -> entity_to_addr -> addr` and then read the key from `connected[addr]` at send time. If the session at `addr` is replaced with the same address and entity id while one of these awaits is in flight, the old entry's packets are sealed with the new session's key and sequence numbers and reach the new client. The existing test `on_client_ready_stops_old_continuation_after_ring_permit_replacement` proves this only for the handoff permits.

Verified send chokepoints (this is what makes a small fix possible): no callee under `crates/base-session/src/base/{contact_list,organization,plugin}`, `crates/base-world-entry/src/base/world_entry_appearance` or `.../world_entry/cell_dispatch/deferred_flush.rs` calls `transport.send_to` directly (grep for `transport\s*\.\s*send|\.send_to\(` lists none of them). They all reach the wire through three sites in `crates/base-session/src/base/helpers/`: the unreliable path in `mod.rs` (`send_to_witness`, session read at about line 404, `transport.send_to` at 443), the single reliable path in `reliable_fit.rs` (`send_reliable_to_addr`, sequence reservation about line 127) and the bundle path in `reliable_fit.rs` (`send_bundle_reliable_to_addr`, about line 381, sequence reservation 417). Each reads `connected.get(&addr)` and consumes a sequence number inside one `connected` lock window.

Out of scope, stated so nobody claims more: this is a general preexisting race, not a letter blocker (authority review 2026-10-14: A2b validates before A1 and A3 rejects stale results). The packet does not claim full world-entry replacement safety. Residual after both steps: (a) the two `tokio::spawn`ed tasks (mail sweep, login fanout) run outside the fence and are character-scoped, not entry-scoped; (b) character-level persistence the old entry still performs (`first_login = 0` UPDATE, mail expiry writes, organization online marking) is idempotent per character and is not suppressed; (c) `letter_turn_in_send.rs` keeps its own bounded sender and is untouched.

## 2. Split and budget

The whole is about 55-65k working tokens, over the shared 40k envelope, so it is two independently committable Haiku packets, A then B on the same branch. Each commit must pass its own checks. Haiku budget: A about 30k, B about 30k working tokens. If A needs more than about 35k, stop after the lane-green extraction (step A1 below) and commit that alone as `... (wip 1/2)`; do not start B in the same session.

| Item | Value |
|---|---|
| Branch / worktree | `harset/ha-1360a2c-entry-fence` / `harset-ha-1360a2c` |
| Base SHA | the `content/harset-campaign` tip at dispatch (record it in the report); never a stale SHA from this document |
| Commit subjects | A: `Harset HA-1360a2c: guard world-entry continuation stages`; B: `Harset HA-1360a2c: fence reliable sends by session entry` |
| Packet type | `packet-coder`; cargo only through the lane; no push, no PR; commit locally and report |

## 3. Step A: stage-boundary guards and extraction

### A allowlist (edit nothing else)

1. `crates/base-world-entry/src/base/world_entry_appearance/client_ready/mod.rs`: must end **shorter** than 699 lines (target about 450).
2. NEW `.../client_ready/continuation.rs`: the tail of `handle_on_client_ready` from the burst bundle (today's `let appearance_args = pending.appearance_args;` line) to the final `World entry finalized` log, moved verbatim into `pub(super) async fn run_continuation(ctx: ContinuationCtx<'_>) -> Result<(), Box<dyn Error + Send + Sync>>`. `ContinuationCtx` carries borrowed references to exactly the values the tail already uses (addr, entity_id, pending fields, player_name, plugins, aoi_hold, transport, connected, entity_to_addr, db_pool, cell_tx, `&SessionSnapshot`). Behaviour must be unchanged except for the added guards.
3. NEW `.../client_ready/continuation_tests.rs`.
4. `.../client_ready/session_token_tests.rs` (394 lines): only change `fn state`, `fn arm`, `fn start_with_transport`, `fn enter` (and the `Clients`, `Pause` aliases if needed) from private to `pub(super)`. No logic change. If `enter` does not exist under that name, reuse the helper the existing tests use to await the entered signal and report its name.
5. The `mod continuation;` and `#[cfg(test)] mod continuation_tests;` lines in `client_ready/mod.rs`.

### A1: extraction (commit boundary if the budget runs out)

Move the tail unchanged, call it from `mod.rs` after the existing post-handoff `current()` check, run `lane clippy` and the existing `-p cimmeria-base-world-entry` nextest. All existing tests must still pass before guards are added. Do not change any behaviour in this step.

### A2: guards, stage marks, pause points

Inside `continuation.rs` define one macro or helper `stage!(ctx, "<name>")` that, in this order: (1) `#[cfg(test)] session_token_tests::pause_after(addr, "<name>").await`; (2) `if !current() { return Ok(()); }` (where `current` is the same `session.is_current(connected, addr, entity_id, player_id)` test); (3) `#[cfg(test)] continuation_tests::mark(addr, "<name>")`; (4) `#[cfg(test)] session_token_tests::pause_after(addr, "<name>_late").await`. The stage precedes the side-effecting callee. Stage names and positions, in order:

`burst`, `gm_redirect`, `contact_lists`, `org_restore`, `plugin_hook`, `ignore_resync`, `mail_sweep`, `login_fanout`, `cinematic` (inside the `if pending.first_login != 0` block, before `send_cinematic`), `aoi_flush` (before the `aoi_hold` branch). Stages `mail_sweep` and `login_fanout` mark before their `if let Some(..)` so a database-less test reaches them. Add one more `current()` check (no stage mark) after the `first_login` UPDATE block is not required.

`mark` appends to a `static REACHED: LazyLock<Mutex<HashMap<SocketAddr, Vec<&'static str>>>>` in `continuation_tests.rs`; `fn reached(addr) -> Vec<&'static str>` reads and clears.

### A tests (names are the contract; use a distinct localhost port per test, `127.0.0.1:560xx`)

| Test | Assertion | Revert proof |
|---|---|---|
| `continuation_runs_every_stage_in_order_for_a_current_session` | `first_login = 1`, no replacement: `reached(addr)` equals the ten stage names in the order above (control: proves the harness reaches every stage) | Delete the `stage!` for `cinematic`: control fails on the list |
| `continuation_stops_at_each_stage_boundary_when_session_replaced` | For each of the ten stages in turn (fresh addr): arm `<name>`, start, wait entered, replace `connected[addr]` with `state(entity_id, 42, [2;32], generation + 1ns)` (same address, entity, player, new key and generation), release, join with a 3 s timeout. `reached(addr)` must equal exactly the stages before `<name>`; `typed.filter_to(addr)` must hold no packet produced after the replacement. Failure message names the stage | Remove the `if !current()` line from any one stage: that stage's iteration fails because `<name>` (or a later stage) appears in `reached` |
| `continuation_replacement_with_no_session_for_addr_stops_quietly` | Remove `connected[addr]` entirely at stage `org_restore`; handler returns `Ok(())`, no later mark, no panic | Remove that stage's guard: later marks appear |

### A lane commands (run from the worktree; `Set-Location <wt>;` in every call)

```powershell
pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base-world-entry --all-targets
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-world-entry
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-world-entry --features lab-chaos
git diff --check
```

The `lab-chaos` run is mandatory: `cinematic_aoi_hold` has feature-gated delay tests that share these fixtures. Confirm exact lane flags from the `lane-build` skill before the first call. Report the pass counts and the revert-probe results (one disposable deletion per stage guard, restored from a backup each time, `git status` clean afterwards).

## 4. Step B: entry guard at the three send chokepoints

### B allowlist (edit nothing else)

1. NEW `crates/base-session/src/base/helpers/entry_guard.rs` and NEW `.../helpers/entry_guard_tests.rs`.
2. `crates/base-session/src/base/helpers/mod.rs` (679 lines): add `mod entry_guard;`, `pub use` of its public names, the `#[cfg(test)] mod entry_guard_tests;` line, and the unreliable-path check below. **Hard stop:** if the file would exceed 695 lines, report `blocked` instead of editing; the fix is to move the unreliable-send block, not to exceed the cap.
3. `crates/base-session/src/base/helpers/reliable_fit.rs` (551): the two checks below.
4. `crates/base-world-entry/src/base/world_entry_appearance/client_ready/{continuation.rs,continuation_tests.rs}` and `cell_handoff.rs` (only `#[derive(Clone)]` on `SessionSnapshot`; `LetterTurnInToken` and `LetterTurnInEntryEpoch` are already copied by value there, so they are `Copy`; if either is not `Clone`, report `blocked`).

### B design (closed)

```rust
// entry_guard.rs
pub struct EntryGuard { addr: SocketAddr, check: Arc<dyn Fn(&ConnectedClientState) -> bool + Send + Sync> }
tokio::task_local! { static ENTRY_GUARD: EntryGuard; }
pub async fn scoped<F: Future>(guard: EntryGuard, fut: F) -> F::Output;   // ENTRY_GUARD.scope(guard, fut).await
pub(crate) fn superseded(addr: SocketAddr, state: &ConnectedClientState) -> bool; // ENTRY_GUARD.try_with(|g| g.addr == addr && !(g.check)(state)).unwrap_or(false)
```

`superseded` takes the already-locked `&ConnectedClientState`, so the guard never re-locks `connected` (a `std::sync::Mutex`; the unreliable path holds the lock at that point, so a closure that locked `connected` would deadlock) and there is no check-then-lock window. The guard applies only when the send target equals the entry's `addr`; sends to other players' addresses (organization notifications to other online members, contact fan-out) are never dropped.

Checks, each placed **immediately after** `clients.get(&addr)` yields `c` and **before** any ACK drain, sequence reservation or `next_unreliable_seq()`:

1. `helpers/mod.rs`, unreliable path (`Some(c) =>` arm, before `c.next_unreliable_seq()`): if `entry_guard::superseded(addr, c)` log at DEBUG `reason = "entry_superseded"` and return `None` (the existing `ClientDisconnected` path).
2. `reliable_fit.rs`, `send_reliable_to_addr`: in the first `session` read and in the `reserved` closure (before `fetch_add`): superseded returns the existing `WitnessSendOutcome::ClientDisconnected`.
3. `reliable_fit.rs`, `send_bundle_reliable_to_addr` (before `take_piggyback_acks`): superseded returns `BundleSendOutcome::ClientDisconnected`.

No new enum variant (exhaustive matches exist across crates). `log_client_disconnected` may be reused with an `entry_superseded` reason field.

Wire-up in `continuation.rs`: `entry_guard::scoped(EntryGuard::new(addr, { let s = session.clone(); move |c| s.matches(c, entity_id, player_id) }), async { ...the whole run_continuation body... }).await`. The scope covers the burst, so even the burst's own window is fenced. `tokio::spawn`ed work does not inherit the scope (residual (a) above); do not try to propagate it.

### B tests (names are the contract)

In `entry_guard_tests.rs` (base-session, use the crate's `TestTransport` and a `ConnectedClientState` test default):

| Test | Assertion | Revert proof |
|---|---|---|
| `entry_guard_none_leaves_sends_unchanged` | with no scope: reliable, bundle and unreliable witness sends return `Sent` and consume sequences exactly as before | none needed (control); must pass on the base commit too |
| `entry_guard_current_allows_all_three_send_paths` | scope whose check always returns true: all three `Sent` | control |
| `entry_guard_superseded_reliable_send_reserves_no_sequence_and_sends_nothing` | scope whose check always returns false, matching addr: outcome `ClientDisconnected`, `next_seq` unchanged, `typed.filter_to(addr)` empty, pending ACKs unchanged | delete the `reliable_fit.rs` single-path check |
| `entry_guard_superseded_bundle_send_reserves_no_sequence_and_drains_no_acks` | same for `send_bundle_to_witness_reliable`: `BundleSendOutcome::ClientDisconnected`, `next_seq` and `pending_acks` unchanged | delete the bundle-path check |
| `entry_guard_superseded_unreliable_send_takes_no_unreliable_sequence` | same for `send_to_witness`: `next_unreliable_seq` not advanced, nothing sent | delete the unreliable-path check |
| `entry_guard_other_address_is_not_affected` | superseded guard for addr A; send to addr B is `Sent` | make `superseded` ignore `addr` |
| `entry_guard_scope_ends_with_the_future` | after `scoped(..).await` returns, a send to the same addr is `Sent` | none (control) |

In `continuation_tests.rs` (base-world-entry), the end-to-end proof that only the chokepoint can supply:

`replacement_inside_a_callee_window_drops_the_old_entry_send`: `first_login = 1`; arm the pause `cinematic_late` (placed by step A after the stage guard passed and before `send_cinematic`); wait entered; replace `connected[addr]` with the new-key state; release. Assertions: `reached(addr)` contains `cinematic` (Step A's guard had passed), and `typed.filter_to(addr)` contains **no** cinematic packet. Revert proof: delete the `superseded` check in the reliable path (or the `scoped` wrap) and a packet addressed to `addr` appears, failing the test.

### B lane commands

```powershell
pwsh tools/build-lane/lane.ps1 cargo fmt --all -- --check
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base-session -p cimmeria-base-world-entry --all-targets
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-session -p cimmeria-base-world-entry
pwsh tools/build-lane/lane.ps1 cargo nextest run -p cimmeria-base-session -p cimmeria-base-world-entry --features lab-chaos
pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-base-methods -p cimmeria-services --all-targets
git diff --check
```

The last clippy proves downstream crates still compile against the changed helper surface. No live-DB filter applies: nothing in this packet touches a seed, SQL or the mission writer. Do not run `reload-db.ps1`.

## 5. Stop conditions (report `blocked`, never improvise)

- `mod.rs` or `helpers/mod.rs` would exceed the line caps stated above.
- A callee in the list in section 1 turns out to send without going through the three chokepoints (re-run `rg "transport\s*\.\s*send|\.send_to\("` over the callee directories; expected zero non-test hits).
- The control test `continuation_runs_every_stage_in_order_for_a_current_session` cannot reach all ten stages without a database pool; report which stage and why instead of dropping it.
- Any change needed outside the allowlists, or a lane failure that is not caused by this diff.

## 6. Review and integration

Independent read-only `packet-reviewer`, then `server-authority-enforcer` on the diff (the question: can a replaced session ever receive an old entry's packet, and can the guard drop a legitimate packet to a different player?). Integrate by merge into `content/harset-campaign` after both. The packet is undeployable-neutral: it changes no seed or gameplay, so it does not gate HA-1360 UAT.

## 7. Not claimed

No client or lab behaviour; no proof that a real replacement during login is rare or common; no cover for residuals (a)-(c) in section 1; actual token cost is unmeasured and the 30k figures are estimates.

### A result (Haiku attempt 2)

- Commits: `a8063de5b` (wip 1/2, extraction) and `c06dc74b4` (wip 2/2, guards, stage marks, tests), then the final step commit.
- Deviation: `stage!(addr, current, "<name>")` takes the address, the existing `current()` closure and the stage name, not `(ctx, name)`. The closure already carries the session check, so the macro reuses it.
- Placement: the `burst` stage sits before the burst bundle send; the post-bundle check became the `gm_redirect` stage, so the guard count and order match the ten names.
- Checks: `cargo fmt -p` and `fmt --all --check` ok; clippy `--all-targets` 0 warnings; nextest `cimmeria-base-world-entry` 254 passed (251 existing + 3 new); nextest with `--features lab-chaos` 278 passed.
- Revert probes (one guard replaced by `|| true` at a time, file restored from backup, `restored=True`): `burst`, `gm_redirect`, `contact_lists`, `plugin_hook`, `ignore_resync`, `mail_sweep`, `login_fanout`, `cinematic`, `aoi_flush`: the replacement test fails (1 failed of 3). `org_restore`: replacement and no-session tests fail (2 failed). Deleting the `cinematic` stage: control and replacement tests fail (2 failed).
- No lab, client, push or PR. Lines: `client_ready/mod.rs` 429, `continuation.rs` 336, `session_token_tests.rs` 376, `continuation_tests.rs` 114.

### B result (Haiku attempt 2)

- Commits: `59445b904` (wip 1/2, `entry_guard` module, its seven tests, the unreliable, single-path and bundle checks) and `1bb783ee9` (wip 2/2, `scoped` wrap, `SessionSnapshot` Clone, replacement test), then the final step commit.
- Deviation: the wrap is a split, not an inline block. `run_continuation` builds the guard from the snapshot and calls `scoped(guard, run_stages(ctx))`; `run_stages` is the unchanged former body.
- Line cap: `helpers/mod.rs` is 696 lines after wip 1/2, one over the 695 figure in section 4. Step B made no further edit to it; it is under the 700 cap.
- Checks: `cargo fmt --all -- --check` ok; clippy `-p cimmeria-base-session -p cimmeria-base-world-entry --all-targets` and `-p cimmeria-base-methods -p cimmeria-services --all-targets` ok; nextest both crates 756 passed; nextest with `--features lab-chaos` 812 passed; `git diff --check` clean.
- Revert probes (one check removed at a time, file restored from a scratchpad backup, `git diff` empty after restore): single-path reliable (both filters): `entry_guard_superseded_reliable_send_...` fails; bundle arm: `entry_guard_superseded_bundle_send_...` fails; unreliable arm: `entry_guard_superseded_unreliable_send_...` fails; `superseded` ignoring addr: `entry_guard_other_address_is_not_affected` fails; `scoped` wrap removed: `replacement_inside_a_callee_window_drops_the_old_entry_send` fails (left 2, right 1), no other test fails.
- Probe artifact: restoring a file from a backup kept its old mtime, so cargo reused the previous probe build for `entry_guard.rs`. The `scoped` probe was re-run after touching the restored files; only the replacement test failed then.
- Lines: `helpers/entry_guard.rs` 58, `helpers/entry_guard_tests.rs` 196, `helpers/mod.rs` 696, `helpers/reliable_fit.rs` 559, `client_ready/continuation.rs` 376, `continuation_tests.rs` 171, `cell_handoff.rs` 246.
- No lab, client, push or PR.
