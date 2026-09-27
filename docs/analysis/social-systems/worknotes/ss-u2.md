# SS-U2 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [SS-D1 worknote](ss-d1.md).

## Contract

- **Packet:** SS-U2, duel test partner: the `sparbot` wireclient binary, and the GM `.duel_status [name]` / `.duel_end <name>` console commands.
- **Decisions in force:** D-SS18/19/21 (the duel timings, range and cooldown, read from SS-D1's `duel/limits.rs`, not changed), D-SS25 (no 151/153 send: nothing here sends either), the owner's plain-English preference for GM console commands.
- **Depends on:** SS-D1 (#888, the `DuelRegistry`), SS-C2 (#887, `registry/commands/social.rs`), NA37 (`GameSession`).
- **Base:** `origin/main` @ `e0d5cecf7`. Branch `social/u2-duel-test-partner`, worktree `.claude/worktrees/ss-u2`.
- **Owned paths (new):**
  - `crates/wireclient/src/sparbot.rs`, `crates/wireclient/src/bin/sparbot.rs`, `crates/wireclient/src/world_entry.rs`, `crates/wireclient/tests/it/sparbot_duel.rs`
  - `crates/cell-world/src/cell/duel/gm.rs`, `crates/cell-world/src/cell/duel/tests/gm.rs`
  - `crates/cell-console/src/cell/console/duel.rs`, `crates/cell-console/src/cell/console/tests/ss_u2_duel.rs`
  - this file
- **Edited:**
  - `cell-world/.../duel/registry.rs`: additive only. `DuelState::name`, the `GmAborted` enum, `challenge_from`, `gm_abort`.
  - `cell-world/.../duel/{mod.rs, tests/mod.rs}`: `mod gm;` and the `GmAborted` re-export.
  - `cell-console/src/cell/{mod.rs, console/mod.rs, console/dispatch.rs, console/registry/mod.rs, console/registry/commands/social.rs, console/tests/mod.rs}`.
  - `wireclient/{Cargo.toml, src/lib.rs, src/error.rs, src/session.rs, tests/it/main.rs, tests/it/support/mod.rs}`, `Cargo.lock`.
  - Docs: `docs/architecture/wireclient.md`, `docs/commands.md`, `TESTING.md` (type 11), `crates/README.md`.
- **Read set:** `SS-WORKER-RULES.md`; `work-packets.md` SS-U2 and SS-UAT; the SS-D1 worknote; `crates/cell-world/src/cell/duel/*`; the wireclient crate (`session.rs`, `bundle.rs`, `auth.rs`, `tests/it/*`); `cimmeria-mercury` `test_harness/{peer.rs, clock.rs}`, `channel/channel_core.rs` (`is_timed_out`, `keepalive_due`) and `lib.rs` consts; `base-session/src/base/tick_sync.rs`; `cell-console` `console/{mod.rs, dispatch.rs, social.rs, registry/*, travel/*, tests/{mod.rs, ss_c2_announce.rs, cr05_allcraft.rs}}`; `space_manager/queries.rs` (`find_online_player_by_name`); `SGWPlayer.def` (`sendDuelResponse`, `duelForfeit`); `docs/architecture/{wireclient.md, negative-logging-convention.md}`; `.claude/agent-memory/main-session/reference_client_idle_send_cadence.md`.

## Evidence: can a `GameSession` stay in the world?

Not as it stood. It can now, with the heartbeat in `sparbot::run`.

1. **The server reaps a silent client after 60 s.** `base-session/src/base/tick_sync.rs` defines `INACTIVITY_TIMEOUT = 60 s` on `last_recv`, the time the last datagram arrived from the client.
2. **A passive `GameSession` never sends.** `LoopbackPeer` queues acks for the server's reliable packets and piggybacks them on the next outbound send (`peer.rs` `build_and_register`). Its own `tick()` keepalive reads the injected `TestClock`, which `GameSession` never advances, so no keepalive ever fires. A listen-only session therefore sends nothing and acks nothing.
3. **The real client is never silent.** `reference_client_idle_send_cadence.md` (colo, 2026-09-19) records an idle client sending about six packets a second, mostly `AUTHENTICATE`, which the server ignores.
4. **The fix.** `sparbot::run` sends `AUTHENTICATE` unreliably every 250 ms. The packet carries the pending acks and refreshes `last_recv`. It also stops, with `StopReason::ServerSilent`, when nothing has arrived for 15 s: the server's tickSync runs at 10 Hz, so that silence means the session is gone.
5. **Proof.** `sparbot_session_outlives_the_inactivity_reap` holds the bot for 70 s, has a second session challenge it, and asserts the accept. It passed. With the heartbeat disabled it fails (see Regression proof). The SS-UAT two-client fallback is not needed.

## Design decisions

- **Sparbot logic is in the library and the binary is thin.** `sparbot.rs` holds the duel builders, the `classify` decoder, the `Sparbot` state machine on an explicit clock, and `run`. `bin/sparbot.rs` only parses arguments, logs in and calls `run`. The state machine is unit-tested without a network, and SS-D2's wireclient test can drive `run` directly.
- **Forfeit rules.**
  - The forfeit is sent `forfeit_after` after the accept. The default is 30 s; `0` means never.
  - It is sent only if the duel is still on: a "Duel aborted" line clears the bot's duel state first.
  - Under SS-D1 the countdown aborts at 5 s, so with the default the bot never forfeits. The smoke test uses 1 s, so the forfeit arrives during the countdown.
  - The server logs `UNIMPLEMENTED: duelForfeit` until SS-D3 (seen in the test log at 13:49:14).
- **A new challenge is always accepted**, even over a duel the bot believes is still on. The server decides whether the bot is free.
- **Credentials.** They come from flags or `SPARBOT_*` environment variables and are never built into the binary. The password is hashed to the uppercase-hex SHA-1 that SOAP Phase 1 carries (the `sha1` workspace dependency).
- **`GameSession::enter_world`.** The world-entry sequence moved from `tests/it/support` into the library, and it returns `Error::WorldEntry` where the old code asserted. `enter_castle_with_timeout` now wraps it, and all five NA37 visibility and chaos tests still pass on it.
- **Socket binding.** A `GameSession` whose BaseApp is not loopback binds `0.0.0.0:0`, so the bot can reach a remote server. Loopback keeps `127.0.0.1:0`.
- **`.duel_status` and `.duel_end` resolve players by name.** Both use `find_online_player_by_name` (the `.summon` lookup, case-sensitive) and take the `player_id` from the entity; `InTransition` counts as found, because the registry is keyed by `player_id`. With no name, `.duel_status` uses the caller's `player_id`.
- **`.duel_end` is a GM abort** (`DuelRegistry::gm_abort`).
  - It removes whatever the named player is part of: a duel in any stage, or a challenge in either direction.
  - It starts **no** pair cooldown. A GM ending the duel is not the pair declining.
  - It sends 878 to both players who are still in the world. A duelist who is not in the world gets `duel.notify_skipped why=gm_end`.
- **`min = 0` for `.duel_end`** (SS-C2's `.announce` precedent). A bare `.duel_end` reaches the command's own usage line and `reason=no_name`, not the generic argument-count refusal.
- **The code is split by crate.** `cell::duel::gm` (cell-world) owns the registry change, the sends to the duelists and the `duel.gm_ended` row. `console/duel.rs` (cell-console) owns name resolution, the GM's feedback and the refusal rows. The duel module keeps its sends private (`outbound` is `pub(super)`).
- **`DuelState::name` is an exhaustive match**, so a new state from SS-D2 fails to compile until it has a status label.

## Telemetry

No new server log target: the GM rows use the existing `duel` target (`duel=debug` in `OTEL_FILTER`). The `sparbot` target is the standalone binary's own and never reaches the server's exporter.

| Event | Level | Where | Fields | Test |
|---|---|---|---|---|
| `duel.gm_ended` | INFO | cell-world `duel/gm.rs` | GM `account_id`, `player_id`, `entity_id`; `subject_player_id` (the named player), `opponent_player_id`, `duel_id`, `stage = challenge \| countdown \| engaged` | `gm_end_tells_both_and_clears_the_duel`, `duel_end_clears_both_entries_and_tells_both` |
| `duel.gm_rejected` | DEBUG | cell-console `console/duel.rs` | GM ids, `command`, `reason = no_name \| target_not_found \| target_ambiguous \| target_not_player \| caller_not_player \| nothing_to_end`, `subject_player_id` when known | `bare_duel_end_logs_no_name`, `unknown_name_logs_target_not_found` (both commands), `duel_end_on_an_idle_player_logs_nothing_to_end` |
| `duel.gm_status` | DEBUG | cell-console | GM ids, `subject_player_id` | `duel_status_reports_self_and_a_named_duelist` (behaviour) |
| `duel.notify_skipped` | DEBUG | cell-world | `why = gm_end`, `reason = player_not_in_world`, `player_id`, `target_player_id`, `duel_id` | SS-D1's shape, reused |
| `sparbot.in_world`, `.challenge_accepted`, `.forfeit_sent`, `.server_line`, `.session_lost` (`reason = server_silent \| send_failed`) | INFO / WARN | the `sparbot` binary's stdout | `entity_id`, `challenger_entity_id`, `text` | seen in the live test log |

The dispatcher's existing "GM .-console command accepted" row still logs every accepted `.duel_*` line with `command` and the GM's ids.

SigNoz queries:

| Question | Query |
|---|---|
| Which duels did GMs end, and whose? | `scope_name = 'duel' AND event = 'duel.gm_ended'`; `player_id` is the GM, `subject_player_id` the named player, and `duel_id` joins the rest of that duel's rows |
| Why did a GM's `.duel_end` do nothing? | `scope_name = 'duel' AND event = 'duel.gm_rejected' AND player_id = <gm>`; `reason` says why |
| Was the bot's challenge accepted? | `scope_name = 'duel' AND event = 'duel.accepted' AND player_id = <bot player id>` |

## Tests

- **cell-world** `duel::tests::gm` (6):
  - `gm_abort_ends_a_duel_for_both_without_a_cooldown`
  - `gm_abort_withdraws_a_challenge_from_either_side`
  - `status_reports_each_side`
  - `gm_end_tells_both_and_clears_the_duel` (type 12 on the `gm_ended` row)
  - `gm_end_with_nothing_to_end_sends_nothing`
  - `online_name_resolves_a_connected_player`
- **cell-console** `console::tests::ss_u2_duel` (10):
  - parse: `duel_status_parse_takes_an_optional_name`, `duel_end_parse_requires_a_name`
  - `duel_end_clears_both_entries_and_tells_both`: 878 to both, both entries cleared, the GM line and the audit row
  - `duel_end_withdraws_a_pending_challenge`
  - type 12 refusals: `bare_duel_end_logs_no_name`, `unknown_name_logs_target_not_found`, `duel_end_on_an_idle_player_logs_nothing_to_end`
  - `duel_status_reports_self_and_a_named_duelist`
  - the GM gate: `non_gm_duel_end_is_chat`
  - `help_duel_end_shows_argument_detail`
- **wireclient** `sparbot::tests` (6), wire bytes and the state machine:
  - `challenge_bytes`, `response_and_forfeit_bytes`
  - `classify_challenge_only_on_own_entity`, `classify_feedback_line`
  - `accepts_then_forfeits_once`, `abort_cancels_the_forfeit_and_none_never_forfeits`
- **wireclient** `tests/it/sparbot_duel.rs` (3):
  - `sparbot_wire_matches_the_server` (no DB): the bot's constants and payloads against `cimmeria-wire`'s own constants and decoders.
  - `sparbot_accepts_a_duel_challenge` (live DB): a real challenger session sends 0xD9. The bot accepts, both get "Duel accepted…", and the bot sends one forfeit.
  - `sparbot_session_outlives_the_inactivity_reap` (live DB, `#[ignore]`, 86 s): the keep-alive test.

## Commands run

Every command ran from the worktree root through the lane. Each log was grepped for `^error` and for the result line, because the lane's exit code can hide a cargo failure.

| Command | Result |
|---|---|
| `lane.sh cargo test -p cimmeria-cell-world --lib duel::tests::gm` | 6 passed |
| `lane.sh cargo test -p cimmeria-cell-console --lib ss_u2` | 10 passed |
| `lane.sh cargo test -p cimmeria-cell-console --lib console::tests` | 174 passed (before the help test was added) |
| `lane.sh cargo test -p cimmeria-wireclient --lib --test it` (no DB) | lib 43 passed; it 11 passed, 1 ignored. The live-DB modules self-skipped in this run |
| `bash tools/build-lane/reload-db.sh` | `sgw_ss_u2` loaded (283 content chains) |
| `DATABASE_URL=…/sgw_ss_u2 lane.sh cargo test -p cimmeria-wireclient --test it sparbot_duel -- --test-threads=1 --nocapture --include-ignored` | 3 passed in 86 s. The log shows `duel.accepted`, `sparbot.forfeit_sent`, `UNIMPLEMENTED: duelForfeit`, and, after 70 s, a second accept |
| `DATABASE_URL=…/sgw_ss_u2 lane.sh cargo test -p cimmeria-wireclient --test it -- --test-threads=1` | 11 passed, 1 ignored. This includes the five NA37 visibility and chaos tests on the refactored `enter_world` |
| `lane.sh cargo nextest run -p cimmeria-wireclient -p cimmeria-cell-world -p cimmeria-cell-console` | 776 passed, 1 skipped (the ignored test) |
| `lane.sh cargo clippy -p cimmeria-wireclient -p cimmeria-cell-world -p cimmeria-cell-console --all-targets -- -D warnings` | clean. The first run flagged `chunks_exact_to_as_chunks`, which was fixed |
| `lane.sh cargo fmt --all -- --check` | clean |
| `lane.sh cargo hakari generate` / `manage-deps --yes` | no changes |
| `python tools/crate-graph/crate_graph.py --check` | exit 0 |
| `lane.sh cargo run -p cimmeria-wireclient --bin sparbot -- --help` and with a missing `--player-id` | usage printed; exit 0 and 2 |

## Regression proof

Each mutation was applied, the named filter was run, and the file was restored and touched (`/tmp/ssu2_mut.py`). `git status` was clean afterwards.

| Mutation | Result |
|---|---|
| `HEARTBEAT_EVERY` set to 3600 s (no keep-alive) | `sparbot_session_outlives_the_inactivity_reap` failed at `stopped == RunTimeElapsed`: the bot stopped with `ServerSilent` after the reap |
| The bot answers a challenge with nothing | `sparbot_accepts_a_duel_challenge` failed (no accept, `challenges_accepted` 0) |
| `gm_abort` skips the in-duel branch | `duel_end_clears_both_entries_and_tells_both` failed |
| `gm_end` does not send 878 | `duel_end_clears_both_entries_and_tells_both` and `duel_end_withdraws_a_pending_challenge` failed |
| `.duel_end` spec `min` back to 1 | `bare_duel_end_logs_no_name` and `help_duel_end_shows_argument_detail` failed |

## PR #910 review

- **Sentinel collision.** The accounts and players in `tests/it/sparbot_duel.rs` were 900_401-900_404, which overlap `two_client_castle_visibility_chaos.rs` (900_401-900_403). Both are modules of one test binary and can run concurrently. They moved to 900_601-900_604, and the module now reserves the block 900_600-900_699 in a comment. Every sentinel in `tests/it/` was checked: 1xx and 2xx are `two_client_castle_visibility`, and 3xx, 4xx and 5xx are the chaos module.
- **IPv6 BaseApp.** The bind address was IPv4-only. `session::local_bind_addr` now picks by IP family: a loopback BaseApp gets the same family's loopback, and any other address gets that family's wildcard (`0.0.0.0:0` or `[::]:0`). Guard: `session::tests::bind_address_matches_the_base_family` covers both families, loopback and routable.
- **Rebased** onto `origin/main` with no conflicts.
- **Commands:**
  - `lane.sh cargo fmt --all -- --check`: clean.
  - clippy on wireclient, cell-world and cell-console with `--all-targets -- -D warnings`: clean.
  - nextest on the same three crates: 777 passed, 1 skipped (the ignored 70 s test).
  - the live-DB `it` binary on `sgw_ss_u2` with `--test-threads=1`: 11 passed, 1 ignored.

## Known gaps

- **Cross-space moves strand the bot.** A cross-world `.summon` or a gate trip restarts the world-entry handshake, and `run` does not answer it. The documented workflow logs the bot's character into the tester's world and uses a same-space `.summon`. Handling the re-entry (`RESET_ENTITIES`, then `ENABLE_ENTITIES`, `mapLoaded` and `onClientReady` again) is a follow-up if testers need it.
- **The binary has never run against the colo.** It needs an account the owner provides and the colo's auth URL. It has only run against the in-process test server, through `run`. The binary's own login path is the same `GameSession::connect` plus `enter_world`.
- **Players are looked up by name only.** `.duel_end` and `.duel_status` cannot name a player who is not on this cell. Registry entries for such a player expire on their own (30 s challenge, 5 s countdown). An engaged duel after SS-D2 relies on SS-D3's disconnect hook.
- **`sparbot_session_outlives_the_inactivity_reap` is `#[ignore]`d.** It takes 86 s. The wireclient `it` binary is not in CI's live-DB run anyway (`tools/test-live-db.sh` covers `--lib` only).
- **The smoke test does not assert what follows the accept.** It checks the accept and the forfeit send only, so SS-D2 replacing the countdown abort does not break it.

## Integration edits for the coordinator

1. **SS-D2.** When a duel can be `Engaged`, `cell::duel::gm::gm_end` must run the same engaged-duel teardown as a normal end: the PvP flag, and `onDuelEntitiesClear` if SS-D2 sends it. The simplest way is to have `gm_end` call SS-D2's or SS-D3's end function instead of `DuelRegistry::gm_abort` plus its own 878 loop. Any new `DuelState` variant needs a `DuelState::name` arm, which the compiler enforces.
2. **SS-D2's wireclient test** can start from `tests/it/sparbot_duel.rs::sparbot_accepts_a_duel_challenge`: `challenge_and_listen` drives the challenger, and `sparbot::run` drives the bot. After the engage lands, add assertions for what follows the accept there.
3. **SS-D3.** The bot already sends `duelForfeit` (CM 103). When the forfeit handler lands, add an assertion to the smoke test that the challenger is told the result.
4. **SS-U3.** The `debug-hub.md` "What the hub cannot test" row for duels can point to `docs/architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing`, which is the anchor `docs/commands.md` already uses.
5. **SS-99.** `docs/gameplay/duel-system.md` can mention the GM tools and the sparbot. This packet did not edit it.
6. **Contended files.** In `registry.rs` the new code sits after `cancel_pending` and before `decline`, and the `GmAborted` enum sits before `DuelRegistry`. In `duel/mod.rs` there is one `pub mod gm;` line and one re-export. A merge conflict with SS-D2 there would be adjacent additions only.

## Open questions

- Should the bot re-enter the world after a cross-space move (see Known gaps)? Only if testers ask for it.
- A colo account for the bot is an owner decision (SS-UAT step 11's solo fallback).
