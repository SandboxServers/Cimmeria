# SS-D1 Worknotes

> Type: reference. Audience: social-systems coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [SS-00 worknote](ss-00.md).

## Contract

- **Packet:** SS-D1, duel challenge and response.
- **Decisions in force:** D-SS13 (name resolution), D-SS15 (Ignore, as a seam), D-SS18 (30 s expiry, 5 s countdown), D-SS19 (20-unit challenge range, same space), D-SS21 (bucket, one open challenge per side, 60 s pair cooldown), D-SS25 (no 151/153 send).
- **SS-E1 evidence used:** unmerged PR #875, `origin/social/se1-re`: `docs/reverse-engineering/findings/duel-wire-formats.md` § "SS-E1 client evidence" and `worknotes/ss-e1.md`. D-Q1 (the countdown length is whatever the server sends; no client constant), D-Q6 (send the duel texts as literal feedback lines; `onErrorCode` ruled out), the `onDuelChallenge` [143] and CM 102 shapes, and D-Q5 (151/153 are safe for SS-D2, but not sent here).
- **Base:** `origin/main` @ `88d7da73a` (SS-00, #880). Branch `social/d1-duel-challenge`, worktree `.claude/worktrees/ss-d1`.
- **Owned paths (new):**
  - `crates/cell-world/src/cell/duel/` (`mod.rs`, `registry.rs`, `challenge.rs`, `response.rs`, `tick.rs`, `outbound.rs`, `limits.rs`, `tests/{mod,registry,challenge,response,tick}.rs`)
  - `crates/base/src/base/dispatch/duel.rs`, `crates/base/src/base/dispatch/tests/duel_challenge.rs`
  - `crates/wire/src/base/duel.rs` (0xD9 decoder), `crates/wire/src/cell/cell_methods/player/duel.rs` (CM 102 decoder), `crates/wire/src/cell/client_methods/duel.rs` (`build_on_duel_challenge`, the duel texts), `crates/wire/src/cell/messages/duel_base_to_cell.rs` (`DuelBaseToCell`)
  - `crates/cell/src/cell/service/base_messages/tests/duel.rs`
  - this file
- **Edited:** `crates/base/src/base/dispatch/mod.rs` (contended: `mod duel;` and the one 0xD9 arm), `crates/base/src/base/dispatch/tests/mod.rs`, `crates/wire/src/{lib.rs, cell/messages/{mod.rs, base_to_cell.rs}, cell/client_methods/mod.rs, cell/cell_methods/player/mod.rs}`, `crates/cell-world/src/cell/{mod.rs, space_manager/mod.rs}` (the `duels` field), `crates/cell/src/cell/{mod.rs, service/base_messages/mod.rs, service/base_messages/tests/mod.rs, service/message_loop.rs}`, `crates/cell-methods/src/cell/{mod.rs, cell_methods/player/social.rs}` (the CM 102 arm), `crates/server/src/logging/{filters.rs, target_scan_tests.rs}`, and the docs under "Docs".
- **Read set:** the ledger (README D-SS13/15/18/19/21/25, work-packets Contract / Contended files / SS-D1, audit A-41, A-42, A-50 and § 6 CAT-M-12/13); SS-E1's duel finding and worknote on `origin/social/se1-re`; `entities/defs/SGWPlayer.def:509-513, 970-985, 1372-1375`; `db/resources/Texts/Seed/texts.sql` (monikers 872-880); SS-00's `player_index/`, `rate_limit/`, `feedback.rs`; the organizations pattern (`OrgBaseToCell`, `base_messages/org.rs`, `dispatch/organization.rs`); `cell_methods/player/{social.rs, dispatch.rs}`; `service/message_loop.rs`; `docs/architecture/{instrumentation-discipline.md, negative-logging-convention.md}`.

## Design decisions

- **The registry is keyed by `player_id`** (contract). It holds the pending challenge per target, a challenger reverse index, the duels, a per-player duel index and the directed `(challenger, target)` cooldowns. Handlers resolve a player's current entity at send time (`duel::find_player`), because gate travel changes it. The registry is pure state on an injected clock; `challenge.rs`, `response.rs` and `tick.rs` send and log.
- **`DuelId`** is a per-cell counter allocated at the challenge and kept by the duel it becomes, so every row of one challenge and its duel correlates on `duel_id`.
- **Base order: bucket first.** The ledger lists "squad refused; the duel bucket; …". The base takes the token first, as the chat path does (`dispatch/chat.rs`), so a malformed, squad or misspelled challenge still costs a token and a flood of bad packets cannot become a flood of feedback lines. The order of the remaining checks is the ledger's.
- **Squad duels get Cimmeria's own line** ("Squad duels are not available."), not text 874 ("You cannot start a squad duel when not in a squad"). 874 would be false for a squad member, and the base does not know squad membership. The ledger allows either.
- **Self, space and range run on the cell, in the ledger's order**, then busy (challenger first, then target) and the pair cooldown in the registry. The registry also refuses a self-challenge, as a second layer.
- **Cross-space uses text 877**, the same as out of range. Both mean "not close enough"; the `reason` field tells them apart in SigNoz.
- **The cell re-checks both entities.** The base's ids can be stale by the time the message is handled (entity recycled, player mid-teardown). `connected_player` requires the entity to be connected (`space.players`) and its `player_id` to match; a mismatch on the target is refused as "not online" (`reason = target_gone`), and on the challenger the message is dropped (`reason = challenger_gone`).
- **Response values.** 1 is accept and 0 is decline (audit A-50: Yes calls `duelResponse(true)`). Any other value, or a length other than one byte, is refused as malformed (WARN, not answered), and does **not** consume the challenge, so a garbage packet cannot cancel a real prompt.
- **The challenge is consumed on every path that finds it**: accept, decline and a late answer. A late answer that the sweep had not yet removed is treated exactly as the sweep would treat it (878 to both, cooldown).
- **Accept after the challenger left** (logged off, or changed space while the prompt was up): no duel, 878 to the responder, the pair cooldown starts, `event = duel.accept_refused reason = challenger_gone`.
- **The countdown end, until SS-D2.** SS-D1 cannot engage a duel (no PvP flag, no harm gate), and an `Engaged` duel with no end path (SS-D3) would leave both players busy for the life of the cell process. So `tick::on_countdown_end` aborts the duel with 878 and `reason = engage_not_implemented`. SS-D2 replaces that one function's body. `DuelRegistry::can_harm` is true only for `DuelState::Engaged`, which nothing in SS-D1 enters.
- **No countdown display.** SS-E1 D-Q1 found that `Event_UI_DuelTimerStart(float)` takes the duration from the wire, but no server message that fires it was traced (the `onTimerUpdate` type 14 subscriber is `SGWBeing_onBigWorldTimeComplete`, not the duel UI, per `.claude/agent-memory/game-archaeology-specialist/timer-system-extended.md`). Both players get the literal line "Duel accepted. The duel starts in 5 seconds." instead. SS-D2 should pick this up if the timer's driver is found.
- **Texts.** Monikers 872, 873, 877 and 878 are sent verbatim as feedback lines (SS-E1 D-Q6); `moniker_texts_match_the_seed` pins each against `texts.sql`. The other lines (not online, ambiguous, ignoring, target busy, pair cooldown, challenge sent, no pending challenge, accepted, squad) are Cimmeria's, in `cimmeria_wire::cell::client_methods::duel`. `RateCategory::DuelChallenge.feedback_text()` from SS-00 is kept as the rate-limit line.
- **The target sees nothing on a refused challenge**; only the challenger gets a line. Decline, expiry and the countdown abort tell both.
- **`DuelBaseToCell::Challenge` also carries `account_id`** (beside the contract's `player_id` and `entity_id` for both sides), so the cell's challenge rows carry the challenger's account even when the entity lookup fails.
- **The duel tick runs every AoI tick** (100 ms) and returns at once when the registry is idle (`is_idle`).

## Telemetry (owner rule: debuggable from SigNoz alone)

New log target `duel`, `duel=debug` in `OTEL_FILTER`, pinned at DEBUG and WARN in `scan_finds_known_targets`; catalog row in `docs/architecture/observability.md`. Info spans: `duel.challenge_request` (base, `peer`, `payload_len`), `duel.challenge` (cell, the ids of both players) and `duel.response` (cell, `account_id`, `player_id`, `entity_id`). No span inside the tick (rule 3).

| Event | Level | Where | Fields beyond the ids | Test |
|---|---|---|---|---|
| `duel.challenge_refused` | DEBUG (WARN for `not_in_world`, `no_cell_channel`, `cell_channel_closed`) | base | `reason = not_in_world \| challenger_loading \| squad_duel \| target_not_online \| target_loading \| target_ambiguous \| target_not_in_world \| target_ignoring \| no_cell_channel \| cell_channel_closed`, `target_name` (first 64 chars), `squad_duel` | `challenge_rejects_squad_duel`, `challenge_rejects_offline_target`, `challenge_rejects_ambiguous_target` |
| `duel.challenge_malformed` | WARN | base | `reason` from the decoder (`truncated`, `trailing_bytes`, `lone_surrogate`) | `challenge_malformed_payload_is_dropped` |
| `duel.challenge_forwarded` | DEBUG | base | logged only after the cell channel accepted the challenge; `target_entity_id` | `challenge_forwards_session_ids_to_the_cell` (forward asserted) |
| `rate_limit.exceeded` | WARN / DEBUG | base (SS-00 helper) | `category = duel_challenge` and the bucket state | `challenge_rate_limited` |
| `duel.challenge_refused` | DEBUG | cell | `reason = challenger_gone \| self_challenge \| target_gone \| cross_space \| out_of_range \| challenger_busy \| target_busy \| pair_cooldown`, `distance`, `range` | one test per reason in `tests/challenge.rs` |
| `duel.challenge_sent` | DEBUG | cell | logged only after the prompt was queued; `duel_id`, `target_entity_id`, `target_account_id`, `space_id`, `distance`, `expires_in_ms` | `challenge_prompts_the_target_byte_exact` |
| `duel.response_refused` | DEBUG (WARN for `not_a_player`) | cell | `reason = no_pending_challenge \| expired \| not_a_player`, `response`, `expired_ms_ago` | `response_without_challenge_rejected`, `response_replay_rejected`, `response_after_expiry_rejected` |
| `duel.response_malformed` | WARN | cell | `reason = bad_length \| unknown_response`, `args_len`, and `target_player_id` + `duel_id` of the challenge addressed to the caller, if any | `malformed_response_does_not_consume_the_challenge` |
| `duel.declined` | DEBUG | cell | `duel_id` | `decline_tells_both_sides` |
| `duel.accepted` | DEBUG | cell | `duel_id`, `state = start_pending`, `space_id`, `target_account_id` | `accept_starts_the_countdown_for_both` |
| `duel.accept_refused` | DEBUG | cell | `reason = challenger_gone` | `accept_after_the_challenger_left_aborts` |
| `duel.challenge_expired` | DEBUG | cell tick | `reason = no_answer` | `unanswered_challenge_expires_and_tells_both` |
| `duel.aborted` | DEBUG | cell tick | `reason = engage_not_implemented`, `state`, `space_id` | `countdown_end_aborts_until_ss_d2` |
| `duel.notify_skipped` | DEBUG | cell | `why`, `reason = player_not_in_world`, the other duelist as `target_player_id` | exercised by `accept_after_the_challenger_left_aborts` |
| `duel.challenge_undelivered` | WARN | cell | `duel_id`, `target_entity_id`, `reason = prompt_not_queued`; the challenge is withdrawn | `undelivered_prompt_withdraws_the_challenge` |
| `duel.send_failed` | WARN | cell | the recipient's `account_id`, `player_id`, `entity_id`, the other duelist as `target_player_id`, `method_index`, `duel_id`, `reason = cell_to_base_closed` | `send_failure_logs_the_recipient_and_the_other_duelist` (receiver dropped, so every send fails) |

Every row carries `player_id` (the actor; the challenger on tick rows) and `target_player_id` (the other duelist), with `account_id` and `entity_id` whenever that player is still in the world. Every row after the challenge is stored carries `duel_id`.

### SigNoz queries

| Question | Query |
|---|---|
| Why was my challenge refused? | `scope_name = 'duel' AND event = 'duel.challenge_refused' AND player_id = <challenger>`; `reason` names the check. For `rate_limit`, `scope_name = 'rate_limit' AND category = 'duel_challenge'` |
| What happened to one challenge? | find its `duel_id` on `duel.challenge_sent` (or on `duel.challenge_undelivered`, if the prompt could not be queued; a challenge logs exactly one of the two), then `scope_name = 'duel' AND duel_id = <id>`: sent, then declined / accepted / expired / aborted |
| Did the target ever get the prompt? | `duel.challenge_sent` with `target_player_id = <target>` means the prompt was queued to the base; `duel.challenge_undelivered` means it was not and the challenge was withdrawn; the same trace carries the `EntityMethodCall` to `target_entity_id` |
| Who answered what? | `event = 'duel.response_refused' OR event = 'duel.accepted' OR event = 'duel.declined'`, grouped by `player_id` |

## Commands run

All from the worktree root through the lane. Exit codes are the lane's; each log was grepped for `^error` and the `test result` / `Summary` line, per the lane-exit memory note.

| Command | Result |
|---|---|
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell-world` | exit 0 |
| `bash tools/build-lane/lane.sh cargo check -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-base` | exit 0 |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-cell-world --lib duel` | 24 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-base --lib duel` | 7 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-cell-methods --lib send_duel_response` | 1 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-cell --lib duel` | 1 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-wire --lib duel` | 8 passed |
| `bash tools/build-lane/lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` | 52 passed (the first run failed on a malformed `OTEL_FILTER` line of mine, fixed before commit) |
| `bash tools/build-lane/lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell -p cimmeria-base` | 1333 passed, 0 skipped |
| `bash tools/build-lane/lane.sh cargo fmt --all` | exit 0, clean |
| `bash tools/build-lane/lane.sh cargo clippy -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-base -p cimmeria-server --all-targets -- -D warnings` | exit 0, no warnings |

**Live-DB:** not run. SS-D1 has no SQL and no live-DB test; nothing it touches reads the database.

## Tests

- Audit § 6 CAT-M-12: `challenge_rejects_self`, `challenge_rejects_cross_space`, `challenge_rejects_out_of_range`, `challenge_rejects_when_target_busy` (cell, `duel::tests::challenge`); `challenge_rate_limited`, `challenge_rejects_squad_duel` (base, `dispatch::tests::duel_challenge`). Names kept.
- Audit § 6 CAT-M-13: `response_without_challenge_rejected`, `response_replay_rejected`, `response_after_expiry_rejected` (cell, `duel::tests::response`). Names kept.
- Byte-exact `onDuelChallenge`: `on_duel_challenge_empty_squad_is_byte_exact`, `on_duel_challenge_with_squad_is_byte_exact` (wire) and `challenge_prompts_the_target_byte_exact` (the handler's actual send).
- Expiry on an injected clock: `unanswered_challenge_expires_and_tells_both` (one millisecond before and at 30 s), `response_after_expiry_rejected`, `take_after_expiry_is_expired_and_starts_cooldown`.
- Feedback on each refusal: every cell refusal test asserts the challenger's exact line and that the target is sent nothing; decline, expiry and the countdown abort assert 878 to both.
- Extra: `challenge_rejects_when_challenger_busy`, `challenge_rejects_during_pair_cooldown`, `challenge_rejects_a_target_entity_that_no_longer_matches`, `decline_tells_both_sides`, `accept_starts_the_countdown_for_both`, `malformed_response_does_not_consume_the_challenge`, `accept_after_the_challenger_left_aborts`, `countdown_end_aborts_until_ss_d2`, seven registry unit tests, the decoders' tests, `moniker_texts_match_the_seed`, `challenge_forwards_session_ids_to_the_cell`, `challenge_rejects_offline_target`, `challenge_rejects_ambiguous_target`, `challenge_malformed_payload_is_dropped`.
- Routing: `dispatch_routes_0xd9_to_the_duel_handler` (base), `duel_challenge_reaches_the_duel_registry` (cell `BaseToCellMsg::Duel` arm), `send_duel_response_routes_to_the_duel_handler` (CM 102 through the player router).
- D-SS25: every cell handler test drains through `tests::drain`, which panics on method 151 or 153.

## Regression proof

Each mutation was applied, the named filter run, and the file restored (`/tmp/ssd1_mut.py`: copy, replace, run, move back; `git status` unchanged afterwards).

| Mutation | Result |
|---|---|
| `take_pending_for` reads instead of removing (no consume) | 9 failed, including `response_replay_rejected`, `take_consumes_once`, `challenge_rejects_during_pair_cooldown` |
| `take_pending_for` expiry check removed | `response_after_expiry_rejected`, `take_after_expiry_is_expired_and_starts_cooldown` failed |
| space check removed | `challenge_rejects_cross_space` failed |
| range check removed | `challenge_rejects_out_of_range` failed |
| self check removed on the cell only | `challenge_rejects_self` still passed: the registry refuses it too, with the same line and `reason` |
| self check removed on the cell **and** in the registry | `challenge_rejects_self` failed |
| duel bucket bypassed (base) | `challenge_rate_limited` failed |
| squad check removed (base) | `challenge_rejects_squad_duel` failed |
| 0xD9 arm removed from `dispatch/mod.rs` | `dispatch_routes_0xd9_to_the_duel_handler` failed |
| CM 102 arm back to a log-only stub | `send_duel_response_routes_to_the_duel_handler` failed |
| `BaseToCellMsg::Duel` arm made a no-op | `duel_challenge_reaches_the_duel_registry` failed |

### PR #888 review follow-up

- **Copilot finding (`outbound.rs`):** `duel.send_failed` carried only `entity_id` and `duel_id`. Every send now takes an `outbound::Recipient` (entity, account, player, and the other duelist), built by each caller from state it already has, and the failure row logs all of them. Guard: `tests/outbound.rs::send_failure_logs_the_recipient_and_the_other_duelist` drops the receiver so both challenge sends fail, then checks each row's `account_id`, `player_id`, `entity_id` and `target_player_id`.
- **Cooldown pruning:** `expire_pending` already dropped expired cooldowns, but `DuelRegistry::is_idle` ignored the cooldown map, so the wall-clock `tick::run` short-circuited once the last challenge or duel was gone and a cooldown stayed until the next challenge anywhere on the cell. `is_idle` now counts cooldowns, so the tick keeps running until each has expired and been pruned. `cooldown_count()` was added for the tests. The other maps (`pending`, `pending_from`, `duels`, `in_duel`) are removed on consume, expiry and `end_duel`, and a player's entries are covered by the 30 s expiry and the 5 s countdown. Guards: `registry::expired_cooldowns_are_pruned` and `tick::wall_clock_tick_prunes_an_expired_cooldown`.
- **Regression proof:** each mutation was run with `cargo test -p cimmeria-cell-world --lib duel`. `is_idle` without the cooldown clause failed both cooldown tests. Removing the `cooldowns.retain` prune failed both. Dropping `account_id`/`player_id` from the failure row failed the outbound test, and so did dropping `target_player_id`.
- **Commands:** `lane.sh cargo fmt --all` (clean); `lane.sh cargo test -p cimmeria-cell-world --lib duel` (27 passed); `lane.sh cargo clippy -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods --all-targets -- -D warnings` (clean); `lane.sh cargo nextest run -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods duel` (29 passed).

### PR #888 review round 2

- **`duel.notify_skipped`** now carries `target_player_id` (the other duelist). Audit of every `duel.*` event: `target_player_id` is present wherever the other player is known. `duel.response_malformed` now also names the challenger of the caller's pending challenge, if there is one. The rows that still omit it have no other player: base `not_in_world` and `challenge_malformed` (no name decoded yet), the name-lookup misses, and the cell's `response_refused` for `no_pending_challenge` and `not_a_player`.
- **Failed prompt delivery.** `send_challenge_prompt` now returns whether the `onDuelChallenge` send was queued. If it was not, `challenge.rs` withdraws the challenge with `DuelRegistry::cancel_pending` (no cooldown: the target never saw it), logs `duel.challenge_undelivered` (WARN, `reason = prompt_not_queued`), and sends the challenger "Your duel challenge could not be delivered." The target cannot be unresolved at send time: `connected_player` resolves it in the same synchronous handler just before the send. The cell only sees whether the base channel accepted the message; a base-side drop (no address for the entity) is invisible to the cell. The guard `undelivered_prompt_withdraws_the_challenge` drops the receiver. It cannot observe the challenger's line, because that send fails on the same closed channel; it shows up as a second `duel.send_failed` row instead.
- **Loading state.** The base now requires both players to be client-ready (`is_client_ready`): `listed_online` set, and none of `pending_world_entry`, `pending_map_loaded`, `pending_client_ready` set. Gate travel keeps the listing but sets those three in turn, so a traveller mid-load was reachable before this change. The refusals are `challenger_loading` ("You cannot send a duel challenge while entering the world.") and `target_loading` ("That player is entering the world. Try again in a moment."). The guards are `challenge_rejects_a_challenger_still_loading` and `challenge_rejects_a_target_still_loading`. **Known gap:** the cell has no client-ready state on `CellEntity`, so an accept (CM 102) is not checked for it. An accept from a responder mid-load cannot happen, because the prompt is only sent to a ready target and CM 102 comes from the responder's own client. A challenger who starts gate travel during the prompt is caught by the accept's same-space check (`challenger_gone`). A same-space reanchor during the prompt is not caught.
- **Rebased** onto `origin/main` @ `1eeb42c69` (PT-01 #870, CR-03/04 #884, the chat split #885). No conflicts.
- **Regression proof** (each mutation run with the duel filter): no withdrawal on a failed prompt failed `undelivered_prompt_withdraws_the_challenge`. `notify_skipped` without `target_player_id` failed `accept_after_the_challenger_left_aborts`. `response_malformed` without it failed `malformed_response_does_not_consume_the_challenge`. `is_client_ready` forced true failed both loading tests, and so did `is_client_ready` without the `pending_*` checks.
- **Tooling note:** the mutation script restored files from a `.bak` copy whose mtime was older than the mutated build, so cargo kept the mutated binary and the next full run failed. The script now touches the restored file. Every result above comes from a run that recompiled the mutated file. The final verification was run after touching every restored file.
- **Commands (after the rebase):** `lane.sh cargo fmt --all -- --check` (clean); `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-base` (1398 passed, 0 skipped); `lane.sh cargo clippy` on those five plus `cimmeria-server`, `--all-targets -- -D warnings` (clean); `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` (52 passed).

### PR #888 review round 3

- **Bucket before identity (`dispatch/duel.rs`).** The not-in-world guard returned before `rate_limits.check`, so a session without a player in the world (character select, mid-teardown) could send 0xD9 without spending tokens. The duel bucket is now taken first, using the session's optional ids for the `rate_limit.exceeded` fields, then a session with no active player is refused (`not_in_world`). Guard: `out_of_world_flood_is_rate_limited` (three calls give two `not_in_world` refusals and one `rate_limit.exceeded category=duel_challenge`). With the old order it failed: all three refused, no rate-limit row.
- **Resolved target on refusals.** `target_loading` and `target_not_in_world` now carry `Some(target.player_id)` into `refuse`, so the row logs `target_player_id`. Guards: `challenge_rejects_a_target_still_loading` (extended) and the new `challenge_rejects_a_target_not_in_world_and_names_it`. Each failed when its id was put back to `None`.
- **`duel.challenge_sent` after the prompt is queued.** It was logged before `send_challenge_prompt`, so a failed send produced both `challenge_sent` and `challenge_undelivered`. It is now logged only after a successful queue, so each challenge logs exactly one of the two. `undelivered_prompt_withdraws_the_challenge` now asserts that no `challenge_sent` row exists; logging it early again failed that test and `challenge_prompts_the_target_byte_exact`. The SigNoz queries above were updated.
- **Rebased** onto `origin/main` @ `a53c6c3c8` (SS-C2, #887). Two conflicts, both adjacent-line additions: the `messages/mod.rs` module doc (SS-C2's `chat_cell_to_base` line and this packet's `duel_base_to_cell` line, both kept) and the `observability.md` target table (SS-C2's `chat` row and the `duel` row, both kept once each).
- **Commands (after the rebase, restored files touched first):** `lane.sh cargo fmt --all -- --check` (clean); `lane.sh cargo nextest run -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-base` (1403 passed, 0 skipped); `lane.sh cargo clippy` on those five plus `cimmeria-server`, `--all-targets -- -D warnings` (clean); `lane.sh cargo test -p cimmeria-server --bin cimmeria-server logging` (52 passed).

### PR #888 review round 4

- **Feedback when the cell is unreachable.** The `no_cell_channel` and `cell_channel_closed` refusals returned silently, although the challenger has a valid entity. Both now send "Your duel challenge could not be delivered." (`TEXT_CHALLENGE_UNDELIVERED`, the line the cell uses for an undelivered prompt) before returning. This follows the visible-feedback-on-first-press rule.
- **`duel.challenge_forwarded` after the send.** It is now logged only after `tx.send` succeeds. The error path logs only `cell_channel_closed`.
- **Guards:** `challenge_with_no_cell_channel_tells_the_challenger` covers both branches (a dropped receiver and `cell_tx = None`). It asserts the line, the refusal row with `target_player_id`, and no `challenge_forwarded` row. `challenge_forwards_session_ids_to_the_cell` now also asserts the `challenge_forwarded` row on success. Removing either feedback line failed the new test, and so did logging `challenge_forwarded` before the send.
- **No rebase** this round; head still sits on `origin/main` @ `a53c6c3c8`.
- **Commands:** `lane.sh cargo fmt --all -- --check` (clean); `lane.sh cargo clippy -p cimmeria-wire -p cimmeria-cell-world -p cimmeria-cell -p cimmeria-cell-methods -p cimmeria-base --all-targets -- -D warnings` (clean); `lane.sh cargo nextest run` on those five with the `duel` filter (50 passed).

### Rebase onto SS-M1

- **Rebased** onto `origin/main` @ `e44af31dd` (SS-M1, #894), after the coordinator's `8041783a8`.
- **Conflicts:** all were lines added side by side, and both sides were kept.
  - `messages/mod.rs`: the new `MailSend` / `MailSendReject` re-exports, plus `DuelBaseToCell`.
  - `logging/filters.rs`: the `mail` and `duel` doc paragraphs and the `mail=debug` and `duel=debug` rows.
  - `target_scan_tests.rs`: the `mail` and `duel` pins.
  - `observability.md`: the `chat`, `mail` and `duel` rows, one each.
- **Gap-analysis totals:** recomputed from the rows. Dueling is now 6 = 2 IM + 4 KM, and the totals are 471 / CW 169 / NT 65 / IM 100 / KM 133 / NU 4, which the 45 rows sum to. The summary percentages and the wire-doc table row were updated, and so was the Dueling row in `docs/project-status.md`.
- **Out of scope:** the overall table in `docs/project-status.md` still carries the 2026-09-25 numbers (NT 58, KM 142) and was already out of step with gap-analysis before this packet.
- **Commands:** `lane.sh cargo fmt --all -- --check` (clean); clippy on wire, cell-world, cell, cell-methods, base and server, `--all-targets -- -D warnings` (clean); nextest on the five crates (1444 passed, 0 skipped); server logging tests (52 passed).

## Docs

- `docs/gameplay/duel-system.md`: status, the implementation table and the feature rows.
- `docs/game-systems.md` § Dueling, `docs/gap-analysis.md` § 27 (challenge and response rows KM → IM).
- `docs/protocol/message-catalog.md`: the Dueling rows and the coverage summary.
- `docs/architecture/observability.md`: the `duel` target row.
- The dispatch tables (`sgwplayer-base-method-dispatch-table.md`, `cell-method-dispatch-table.md`, `client-method-dispatch-table.md`) have no status column; their 0xD9, 102 and 143 rows were already correct and are unchanged.

## Known gaps

- **Ignore (D-SS15) is a seam only.** `dispatch/duel.rs::ignores` returns `false`; the refusal path behind it (`reason = target_ignoring`, `TEXT_TARGET_IGNORING`) is wired but unreachable and untested until SS-C1's cache lands.
- **No countdown display** (see Design decisions). Both players get a text line instead.
- **The countdown ends in "Duel aborted"** until SS-D2 engages duels.
- **Disconnect does not clear the registry.** A pending challenge naming a player who left expires after 30 s, and a duel in the countdown ends at 5 s, so nothing is stranded. SS-D3 owns the `disconnect_entity` hook.
- **The tick is not covered by a loop test.** `duel::tick::run` is called from `message_loop.rs`; the tests call `run_at` directly.
- D-SS18, D-SS19 and D-SS21 values are project policy, stated as such in `duel/limits.rs`.

## Contended files touched

- `crates/base/src/base/dispatch/mod.rs`: `mod duel;` and one arm, `cimmeria_wire::base::duel::SEND_DUEL_CHALLENGE => duel::handle_send_duel_challenge(...)`, placed before the catch-all. No constant was added to `sgw_player_base`, to keep the hunk small beside SS-C1's 0xC5 arm.
- `crates/cell-methods/src/cell/cell_methods/player/social.rs`: only the `SEND_DUEL_RESPONSE` arm (and the `space_mgr` parameter, previously `_space_mgr`), plus one test. `DUEL_FORFEIT` is untouched (SS-D3).
- `crates/cell-world/src/cell/space_manager/mod.rs`: one field and its initialiser. Not on the contended list.

## Integration edits for the coordinator

1. **SS-C1 (Ignore):** replace the body of `crates/base/src/base/dispatch/duel.rs::ignores(target, challenger_player_id)` with the Ignore-cache check on the target's `ConnectedClientState`, then add a test beside `challenge_rejects_offline_target` asserting `TEXT_TARGET_IGNORING` and `reason = target_ignoring` with no forward. The call site already reads the target's session under the same lock as the lookup.
2. **`dispatch/mod.rs` merge order with SS-C1:** both packets add one `mod` line and one arm; the arms are independent.
3. **SS-D2:** replace `crates/cell-world/src/cell/duel/tick.rs::on_countdown_end` with the engage (set `DuelState::Engaged`, the PvP flag, 151), and call `DuelRegistry::can_harm` from `combat::player_may_harm`. If the driver of `Event_UI_DuelTimerStart` is found, send it on accept in `response.rs`.
4. **SS-D3:** `duelForfeit` (CM 103) in `social.rs`, and the `disconnect_entity` hook should call `DuelRegistry` to end a duel or drop a pending challenge (`end_duel`, and a new `drop_player` if needed).
5. `cimmeria-services` does not re-export `cell::duel`; add it to `crates/services/src/cell/mod.rs` only if a facade caller needs it.

## Open questions

- Should a squad member's squad-duel request get text 874 once squads exist (ORG-03)? Today every squad duel gets the same Cimmeria line.
- The 30 s, 5 s, 20-unit and 60 s values await owner confirmation (all PROPOSED in the README).
