# Minigames telemetry review

The happy path is well covered: every hop from content chain to cell result logs at INFO with names, and 13 of 13 colo sessions in the last 7 days can be followed end to end.
The failure paths are weaker. The base drops a lost result silently, `endCurrentMinigame` (the #1303 trigger) has never logged once, and a session's end row doesn't say why it ended.
Ten candidate packets: 2 high, 4 med, 4 low. None needs a domain agent. TG-MG-10 waits on D-TG1.

Reviewed against `main` (tg-ledger worktree) plus the D-TG1 branch (`tg-mgpeer`). The colo build is older than #1297-#1301, so some colo rows below (pre-auth WARNs, empty `reason`) are already fixed on `main`.

## 1. Inventory

**Routing.** `cimmeria_minigame=debug` is pinned in `OTEL_FILTER` (`crates/server/src/logging/filters.rs:308`). Every minigame row at DEBUG and above reaches `cimmeria-server`, and no file layer exists for this crate. Every event uses its module-path target, and there are no hand-named targets. The neighbours are base-world-entry, cell, cell-methods and cell-content, all covered by their crate rows. The client→cell call reaches SigNoz only as `wire.in` INFO.

**Fired in the last 7 days on the colo** (`cimmeria-server`, aggregate):

| Row | Level | n |
|---|---|---|
| Minigame connection accepted | DEBUG | 148 (130 distinct peers; scanners, T10) |
| Unknown SFS message type (old build, protocol scope) | WARN | 79 |
| Minigame read error / send error | DEBUG | 19 / 4 |
| Content: starting minigame (cell) | INFO | 14 |
| Starting minigame session (base) | INFO | 14 |
| Sending onStartMinigame / login successful / Minigame started / session ended | INFO | 13 each |
| Minigame result received (base) / Minigame result (cell) | INFO | 13 / 13 |
| Minigame victory | INFO | 11 |
| Minigame aborted -- client closed without reporting a result | INFO | 2 |
| Entity already has an active minigame session + Failed to register (duplicate?) | WARN | 1 + 1 |
| `wire.in` `endCurrentMinigame` | INFO | **113** |
| `UNIMPLEMENTED: endCurrentMinigame` | INFO | **0** |

Never fired: the result-delivery ERROR (`result_dispatch.rs:63`), the stale-teardown WARN (`session.rs:315`), the pending-expiry INFO (`session.rs:356`), every Livewire rejection WARN, the placeholder rows, and the idle-timeout INFO.

## 2. Positive gaps

- **No span (Rule 1).** `handle_connection` (`server/mod.rs:50`) is the dispatch entrypoint for a whole TCP session, but nothing opens a span. Rows that have no fields of their own therefore can't be correlated by trace either: `framing.rs:100,104` (send error/timeout) and all of `games/livewire/mod.rs` (199-273).
- **No `event=` on transitions (Rule 2).** Session lifecycle rows (registered, claimed, started, outcome, ended, expired) are told apart by body text only.
- **`Minigame session ended` (`server/mod.rs:122`)** has no `end_reason`, `outcome`, `duration_ms` or `room_id`. The 2026-10-06 colo session ran 39 minutes and then aborted, and nothing says whether that was the idle timeout, a FIN, keepalive or a send failure.
- **`Minigame login successful` (`handshake.rs:252`)** has no `peer`, `player_id`/`player_name` or `ticket_age_ms`. Register-to-login latency is how long the SWF took to load, and it's the first number you want for a "minigame won't open" report.
- **Victory rows (`server/mod.rs:359,494`)** don't say who validated the outcome. A placeholder game (Hack, Bypass, Activate, Analyze, Converse) wins when the SWF sends `victory` (`placeholder.rs:38-47`). In SigNoz and Discord that looks exactly like a Livewire win the server validated.
- **Cell `Minigame result` (`cell/.../base_messages/minigame.rs:23`)** has no game name, because `MinigameResult` doesn't carry one. You can still correlate by entity id and a timestamp within a millisecond, as the colo sequence shows, so this isn't worth a wire change.

## 3. Negative gaps

| Site | Shape | Effect |
|---|---|---|
| `base-world-entry/.../cell_dispatch/minigame.rs:152-159` | `if let Some(cell_tx)` + `let _ = cell_tx.send(..)` (Pattern A) | The second hop of the result is lost silently. Victory chains never fire, and the only trace is a cell `Minigame result` row that never appears. The first hop (`result_dispatch.rs:55`) is guarded; this one is not. |
| same file `:46` | `if let Some(registry)` with no else | With the minigame server disabled, the player clicks and nothing opens. The last row is `Starting minigame session` at INFO. |
| same file `:112` | `Failed to register minigame session (duplicate?)` | No `game_name` and no `reason`. The cause is only on the session.rs row. |
| `session.rs:157` | duplicate-session WARN | No `existing_connected`, `existing_age_ms` or `existing_game`. A double-fire 230 ms apart (colo, 2026-10-05) and the stale connected session of #1303 log identical WARNs, and the double-fire is benign. |
| `cell-methods/.../minigame.rs:86-103` (`END_CURRENT`) | `if args.len() >= 12 { log }` then `true` | The .def gives `endCurrentMinigame` no args. On the wire it carries 4 bytes (`00000000`), so the arm returns silently every time: 113 calls, 0 rows. The stub's 3×INT32 layout is invented, and `minigame_names_tests.rs:27` pins it (a theatre test). The same problem affects `startMinigame` (needs 8 bytes, has none), `debugSpectate`/`debugJoin`/`requestSpectateList`/`minigameCallAbort` (no args per .def), `debugMinigameInstance` (3×INT32, the stub reads 1), and `registerToMinigameHelp`/`updateRegisterToMinigameHelp` (`WSTRING,UINT8[,UINT8]`, the stub reads 2×INT32). |
| `cell/.../base_messages/minigame.rs:37-40` | `.unwrap_or(0)` on `player_id` | A victory for an entity the cell no longer knows (relog mid-game, or a reused entity id) fires the chains with `player_id 0` and no row. |
| `server/mod.rs:563-571` | abort row text | Says "client closed" for every non-result exit, including a server-side send timeout (`framing.rs:104`, DEBUG, no entity) and the idle timeout. |
| `handshake.rs:150,154` | `send_null_terminated(..).ok()?` | The pre-auth policy/apiOK send fails silently apart from the DEBUG in framing, which has no peer. Acceptable pre-auth (scanner case). Listed only. |
| `games/mod.rs:20` | Unknown-type WARN | Has no entity id. A misspelled `minigame_type` in a seed row would show as a WARN with only the game name. |

## 4. Noise

- 79 pre-auth WARNs on the old colo build come from scanners. They're already DEBUG on `main` (`handshake.rs` module doc) and are mostly removed by D-TG1 (T10).
- The duplicate-session WARN fires on a benign double interact (pending session, under 1 s old) and reaches the Discord WARN harvest (TG-MG-04).
- After D-TG1, the per-connection DEBUG `unexpected_peer` row (`tg-mgpeer accept.rs:116`) duplicates the throttled INFO for every refused socket. It's DEBUG, so it isn't harvested, but it still exports one row per scanner connection. Consider dropping it.
- Nothing fires per tick. The Livewire 250 ms tick logs nothing.

## 5. Seams (two hops)

| Hand-off | Sender logs | Receiver logs | Can we tell who dropped it? |
|---|---|---|---|
| Interaction → content chain (`fire_interact_tag`) → `StartMinigame` | interact row + `Content: starting minigame` (with chain name) | base `Starting minigame session` | Yes. The cell logs a send failure (`dispatch.rs:189`). The 2026-10-05 double start (two chains 230 ms apart) shows up only as two content rows; the interaction side never says why it fired twice. |
| Base → registry `register` | `Starting minigame session` | duplicate WARN only | Partly (TG-MG-04). A registry that's switched off is silent (TG-MG-01). |
| Failed start → content chain continues | none | the chain's `set_interaction_type` still runs (#1303 step 3) | No. The cell never learns the start failed, and nothing links the stripped interaction to the failed register. That's a behaviour fix, owned by #1303. |
| Base → client `onStartMinigame` | `Sending onStartMinigame` + `ticket_prefix` | client: none | No. Between this row and `login successful` there is nothing. If the SWF never connects, the only row is the 180 s expiry INFO, which has never fired. A client-DLL row for Flash surface open/close belongs to the pipeline review (D-TG4). |
| SWF → listener (D-TG1 gate) | — | throttled INFO `unexpected_peer` | Not for a real player whose TCP source differs from the UDP one (CGNAT, v4/v6). The row looks the same as a scanner's (TG-MG-10). |
| Client `endCurrentMinigame` → cell | `wire.in` INFO | stub never logs | No (TG-MG-02). Also, 113 calls against 13 sessions: the client sends it far more often than games open (from `Minigame.lua` close/hide). The #1303 fix must treat "no session" as DEBUG, not WARN. |
| Minigame → base `MinigameResult` | outcome INFO + ERROR on send failure | `Minigame result received` | Yes. |
| Base → cell `MinigameResult` | none on failure | cell `Minigame result` | **No** (TG-MG-01). |
| Base → client `onEndMinigame` | `send_to_witness_reliable` WARN on a missing address | — | Yes, but a player who logs out with the window open produces a Canceled result and this WARN, which is normal at logout. |
| Cell → `fire_chain_by_id` | `Minigame result` (chains list) | `executing` INFO / missing-chain WARN | Yes, apart from `player_id 0` (TG-MG-09). |
| Result → Discord | `discord_result_event` (Canceled excluded) | Discord router | The minigame side is fine. Discord queue drops belong to the pipeline review. |

## 6. Adversarial

1. **#1303: close with the X, then reopen (players have hit this).** Today: `wire.in endCurrentMinigame`, then nothing from the cell, then `Content: starting minigame`, `Entity already has an active minigame session` WARN and `Failed to register (duplicate?)` WARN, then the chain's interaction-type row. Nothing says the old session is still *connected* or how old it is. It should show `endCurrentMinigame` with `session_state=connected|pending|none`, a duplicate row with `existing_connected=true existing_age_ms=…`, and, once #1303 lands, `session_end end_reason=client_end`.
2. **The base→cell channel is closed at victory.** Today: `Minigame victory`, then `Minigame result received`, then silence. The chains never fire, and the cell's missing row looks the same as a cell that dropped the message. It should show an ERROR `reason=cell_channel_closed chain_count=N`.
3. **The Livewire window stalls (client hitch over 10 s).** Today: a DEBUG `send blocked past the deadline` with no entity, then INFO `aborted -- client closed`, which is wrong. It should show `session_end end_reason=send_failed outcome=canceled duration_ms=…` on one row.
4. **A NAT player after D-TG1.** Today: `Sending onStartMinigame`, then at most one INFO `unexpected_peer` a minute (it may be suppressed), then 180 s later `expiring session whose client never connected`. It should show `pending_sessions=1` on the refusal, so a refusal while a session is waiting reads as a likely real player.
5. **A Hack terminal "won" by a tampered SWF.** Today: `Minigame victory`, and a Discord "won" line exactly like a validated Livewire win. It should show `validation=client_declared` on the victory row.

## Candidate packets

| ID | Title | Sev | Files | Status |
|---|---|---|---|---|
| TG-MG-01 | Base minigame seam: lost result and disabled server | high | 1 + test | Ready |
| TG-MG-02 | MinigamePlayer cell stubs log every call per .def | high | 1 + test | Ready (coordinate with #1303) |
| TG-MG-03 | Session end reason, outcome and duration | med | 1-2 + test | Ready |
| TG-MG-04 | Duplicate-session row says why | med | 2 + test | Ready |
| TG-MG-05 | Livewire rejection rows name the player | med | 1 + test | Ready |
| TG-MG-06 | `minigame.connection` span | med | 1 + test | After TG-MG-03 (same file) |
| TG-MG-07 | Victory rows carry `validation` | low | 2 + test | After TG-MG-03 |
| TG-MG-08 | Login row: peer, player, ticket age | low | 1 + test | Ready |
| TG-MG-09 | Cell result for an unknown entity | low | 1 + test | Ready |
| TG-MG-10 | `pending_sessions` on the D-TG1 refusal | low | 2 + test | BlockedDependency (D-TG1 merge) |

**TG-MG-01: Base minigame seam: lost result and disabled server** (high)
- Files: `crates/base-world-entry/src/base/world_entry/cell_dispatch/minigame.rs` (`minigame_result`, `start_minigame`). Test in `cell_dispatch/tests.rs`.
- Replace `let _ = cell_tx.send(..)` with an ERROR `"Minigame: result forward to cell failed -- chains will not fire"` carrying `entity_id, entity_name, result_code, result, chain_count, reason="cell_channel_closed"`. Add a WARN `reason="no_cell_channel"` when `cell_tx` is `None`.
- In `start_minigame`, add a WARN `reason="minigame_server_disabled"` (with `entity_id, entity_name, game_name`) when `minigame_registry` is `None`. Add `game_name` and `reason="register_refused"` to the `Failed to register` row.
- Test: LogCapture unit test. Drop the cell receiver, call `minigame_result`, and assert the ERROR with `reason` and `chain_count`. It fails on revert because no row is emitted. A second case covers the `None` registry WARN.
- OTEL_FILTER: none.

**TG-MG-02: MinigamePlayer cell stubs log every call per .def** (high)
- Files: `crates/cell-methods/src/cell/cell_methods/minigame.rs` (`dispatch`), `minigame_names_tests.rs`.
- Every arm logs unconditionally with `args_len`. Parse arguments only as `entities/defs/interfaces/MinigamePlayer.def` (CellMethods, lines 351-540) declares them:
  - INT32 for `debugStartMinigame`, `spectateMinigame`, `minigameCallAccept`, `minigameCallDecline` and `minigameContactRequest`.
  - 3×INT32 for `debugMinigameInstance`.
  - None for `debugSpectate`, `debugJoin`, `startMinigame`, `endCurrentMinigame`, `requestSpectateList`, `minigameStartCancel` and `minigameCallAbort`.
  - For the two `*RegisterToMinigameHelp` methods, log `args_len` only.
- Drop the invented `winner`/`loser` fields. Keep `UNIMPLEMENTED: <name>` bodies at INFO.
- Test: replace `end_current_minigame_row_names_the_caller_winner_and_loser` with a test that dispatches `END_CURRENT` with the 4-byte wire payload `00000000` and asserts the INFO row. It fails on revert because the `>= 12` guard is silent.
- If #1303 lands first and implements `END_CURRENT`, this packet shrinks to the other arms.
- Open question for the engine advisor, not blocking: why does a no-arg exposed method carry 4 bytes?

**TG-MG-03: Session end reason, outcome and duration** (med)
- Files: `crates/minigame/src/minigame/server/mod.rs` (`run_session` returns an `EndReason`; `handle_connection` logs it), and optionally `framing.rs`, so that `send_null_terminated` returns whether the failure was a timeout or an error.
- Add `event="minigame.session_end"`, `end_reason` ∈ {victory, defeat, client_closed, read_error, frame_too_long, idle_timeout, send_failed}, `outcome` (`minigame_result_name`), `duration_ms` (from the claim) and `room_id` to `Minigame session ended`. The abort row (`:564`) takes the same `end_reason`, and its body becomes "Minigame aborted without a result".
- Test: LogCapture server tests (the `timeout_tests.rs` harness). Cover an idle timeout giving `end_reason=idle_timeout outcome=canceled`, and a client FIN giving `client_closed`. They fail on revert because the fields are missing.
- OTEL_FILTER: none.

**TG-MG-04: Duplicate-session row says why** (med)
- Files: `crates/minigame/src/minigame/session.rs` (`register`), and `base-world-entry/.../cell_dispatch/minigame.rs` if TG-MG-01 hasn't added `game_name` yet.
- Add `reason="duplicate_session"`, `existing_connected`, `existing_age_ms`, `existing_game` and `requested_game`.
- Level: INFO when the existing session is pending and under 2 s old (a double interact, seen on the colo 2026-10-05), WARN otherwise.
- Test: LogCapture. Register twice in a row and assert INFO with `existing_connected=false`. Register, `authenticate_and_claim`, register again, and assert WARN with `existing_connected=true`. Both fail on revert.

**TG-MG-05: Livewire rejection rows name the player** (med)
- Files: `crates/minigame/src/minigame/games/livewire/mod.rs` (`LivewireGame::new` stores `entity_id`, `player_id` and `player_name` from the session; the `message` arms use them).
- Every rejection WARN at lines 199-273 gets the identity plus `reason` ∈ {not_started, unknown_wire, already_cut, playfield_inactive, illegal_prefix, unknown_command}. Levels are unchanged.
- Test: LogCapture in `livewire/tests.rs`. Send `processmove` with an unknown wire and assert `entity_id` and `reason=unknown_wire`. It fails on revert.

**TG-MG-06: `minigame.connection` span** (med)
- Files: `server/mod.rs` (`handle_connection`).
- Add `info_span!("minigame.connection", peer, entity_id = Empty, player_id = Empty, game = Empty)` and `.instrument()` the body. After login, `record` the fields.
- Test: LogCapture (or a span-recording layer) asserts that a framing send-error row emitted inside a session has the span as its parent. It fails on revert.

**TG-MG-07: Victory rows carry `validation`** (low)
- Files: `games/mod.rs` (`pub fn validation(game_name) -> &'static str`: `"server"` for Livewire, `"client_declared"` for placeholders), and `server/mod.rs` (the victory and failure rows). Also add `entity_id` to the unknown-type WARN at `games/mod.rs:20`.
- Test: a unit test for the function, plus a server test that plays `Hack`, sends `victory` and asserts `validation=client_declared`.

**TG-MG-08: Login row: peer, player, ticket age** (low)
- Files: `server/handshake.rs` (`read_and_handle_login`).
- Add `peer`, `player_id`, `player_name`, `ticket_age_ms` (`session.created_at.elapsed()`) and `event="minigame.claimed"`.
- Test: a listener test that asserts the fields on the login row. It fails on revert.

**TG-MG-09: Cell result for an unknown entity** (low)
- Files: `crates/cell/src/cell/service/base_messages/minigame.rs`, with the test in `base_messages/tests/minigame.rs`.
- When `get_entity` misses on a victory, emit a WARN `reason="result_for_unknown_entity"` with `entity_id`, `chain_count` and `result`. Behaviour is unchanged.
- Test: LogCapture. Deliver a victory for an absent entity and assert the WARN. It fails on revert.

**TG-MG-10: `pending_sessions` on the D-TG1 refusal** (low, BlockedDependency)
- Files: `server/accept.rs`, and `session.rs` (`pending_count()`).
- Add `pending_sessions` to the throttled `unexpected_peer` INFO, and drop the per-socket DEBUG duplicate.
- Test: a listener test with one pending session and a peer that isn't expected, asserting `pending_sessions=1`.
