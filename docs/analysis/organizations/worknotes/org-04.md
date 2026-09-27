# ORG-04 Worknotes

> Type: reference. Audience: the organizations campaign coordinator.
> Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [org-03.md](org-03.md), [org-e1.md](org-e1.md).

## Contract

- **Packet:** ORG-04, squad chat and minimap ping.
- **Decisions in force:** D-ORG03 (squads are cell state, service-wide), D-ORG05 (squad ids from `0x4000_0000`; routing is not authorization), D-ORG14 (EChannel ids are client constants; squad is 4, confirmed by ORG-E1 Q5), D-ORG24 (squad telemetry on the `squad` target). ORG-E1 Q3: no client method shows another member's minimap ping, so CM 10 is validated and logged only.
- **Base:** `origin/main` @ `2ae914aca` (ORG-03, #886). Branch `org/04-squad-chat`, worktree `.claude/worktrees/org-04`, test database `sgw_org_04`.
- **Owned paths:**
  - `crates/cell-world/src/cell/squad/` (`ping.rs` new, `ping_and_join_tests.rs` new, `registry.rs`: `force_join`, `ForceJoinReject`, the `last_ping` field, `mod.rs`)
  - `crates/cell-methods/src/cell/cell_methods/organization/` (`mod.rs`: the CM 10 arm; `squad/ping.rs`, `squad/gm.rs` new; `squad/invite.rs`: `issue` split out of `handle_invite`; `squad/telemetry.rs`: `Action::Ping`, `Outcome::recipients`, two `From` impls; `squad/feedback.rs`; `tests/squad_ping_gm.rs` new)
  - `crates/cell-console/` (`Cargo.toml`: the `cimmeria-cell-methods` edge; `cell/mod.rs`; `console/chat/mod.rs`: the `CHAN_SQUAD` arm only; `console/chat/squad.rs` new; `console/squad.rs` new; `console/registry/commands/squad.rs` new; `console/{dispatch,mod}.rs`, `registry/commands/mod.rs`; tests `chat/tests/squad.rs`, `tests/org04_squad.rs`)
  - `crates/base/src/base/dispatch/chat.rs` (the `squad.chat` rows for base-side refusals), `dispatch/tests/chat_squad_refusals.rs` new, `dispatch/tests/chat_flood_limit.rs` (the harness is `pub(super)`, `speak(channel, ..)`)
  - `Cargo.lock`, `README.md` and `crates/README.md` (regenerated crate graph; the `cell-console` row)
  - Docs: `docs/gameplay/group-system.md`, `organization-system.md`, `docs/gap-analysis.md` §21 and §23, `docs/project-status.md`, `docs/commands.md`, `docs/architecture/observability.md` (the `org` / `squad` row), this worknote.
- **Read set:** the ORG-04 packet and the telemetry section of `work-packets.md`; `worknotes/org-03.md`, `org-e1.md` (Q3, Q5); `crates/cell-console/src/cell/console/chat/`, `console/{mod,dispatch}.rs`, `registry/`; `crates/cell-methods/src/cell/cell_methods/organization/` (all); `crates/cell-world/src/cell/squad/`; `crates/base/src/base/dispatch/chat.rs` and its tests; `crates/base-session/src/base/rate_limit/`; `deprecated/python/base/Chat.py` (`ChatChannel.sendMessage`), `deprecated/python/cell/SGWPlayer.py` (`processPlayerCommunication`, `BroadcastMinimapPing`); `docs/architecture/services-crate-split.md` (C5a/C5b notes on sibling edges).

## Evidence

- **The flood limit and the text rules already cover channel 4.** `send_player_communication_at` (`crates/base/src/base/dispatch/chat.rs`) takes the `RateCategory::Chat` token and runs `org_text::validate(TextField::ChatText, ..)` for every channel before `BaseToCellMsg::ChatMessage`. Nothing was re-implemented on the cell.
- **The legacy channel sent every line to every member, the speaker included.** `ChatChannel.sendMessage` in `deprecated/python/base/Chat.py` loops over `self.players` with no speaker exclusion; the squad channel had `CHANNEL_FLAG_OnCell`. The legacy cell never distributed it (`processPlayerCommunication` answered "not supported yet" for anything but say/emote/yell).
- **`BroadcastMinimapPing` was `pass` in the legacy server** (`SGWPlayer.py`), and ORG-E1 Q3 found only `Event_NetOut_BroadcastMinimapPing` in the client, so there is no receive path to target.

## Design decisions

1. **Squad chat lives in `cimmeria-cell-console`** (`chat/squad.rs`), with only `cimmeria-cell-world` and `cimmeria-wire`, as the task asked; `chat/mod.rs` gains one routing arm. Its outcome row is written there on `squad`, with `count_action` from cell-world, rather than through cell-methods' `Outcome`.
2. **Recipients** are every member whose live entity resolves (`player_entity_by_player_id`), in any space, the speaker included. A member in gate transit misses the line (nothing queued for chat). `recipients` counts delivered copies to others, not the speaker's own.
3. **Base-side refusals are `squad.chat` rows too.** A squad line refused for the flood limit or the text rules never reaches the cell, so the base writes the `squad.chat` row (`rate_limited`, `text_invalid`) on the `squad` target, beside its existing `rate_limit` / `chat` events. The `rate_limited` row follows the feedback throttle (`notify`, once per 5 s) so a flooding client cannot turn each dropped packet into an INFO row; the counter counts every drop. This departs from "exactly one row per action" for flood drops, deliberately.
4. **Ping.** `SquadRegistry::check_ping` holds the state (a `last_ping` map keyed by `player_id`, cleared on any departure). Only an accepted ping starts a new second. The router sends CM 10 to the squad handler unless the id routes to the base, as CM 9 does; a Team or Command id keeps ORG-01's answer. An accepted ping sends nothing to any client. A `not_in_squad` or `wrong_squad` ping gets `onErrorCode` and a line; a `rate_limited` ping gets nothing, because the client drew the ping locally already and nobody else would have seen it, and a line per click would flood the chat window. Accepted pings add a DEBUG `squad.ping_location` row with the coordinates.
5. **The GM commands are split between the console and cell-methods.** The console (`console/squad.rs`) does the GM check (again, past the chat gate: reason `not_gm`), resolves names, lists squads for `.squad_info` and writes `org.gm_action`. Cell-methods (`squad/gm.rs`) has two thin `pub` entry points, `gm_invite(gm_player_id, gm_entity, name)` and `gm_join(gm_entity, host_entity)`, which take resolved identities and return a `GmOutcome { squad_id, reason }` (coordinator's condition when approving the edge). The console calling them adds the dependency edge `cimmeria-cell-console → cimmeria-cell-methods`. They need the invite handler and the join fanout (`fanout::announce_join`), both in cell-methods; the alternatives were duplicating the fanout in the console or moving ORG-03's handlers down into cell-world. The edge is a legal DAG edge (cell-methods never names the console); the cost is that cell-console no longer compiles in parallel with cell-methods. The coordinator approved the edge, on the condition that the entry points stay thin.
6. **`.squad_join` uses `SquadRegistry::force_join`**: no invite, no answer, no leader check (any member's squad can be joined), but the membership rules hold (the GM must be squadless, the squad must have room). A squadless named player founds a squad and leads it. The fanout is the ordinary join fanout.
7. **`.squad_invite`** calls the same invite path as `/squadinvite`; `handle_invite` now delegates to `invite::issue`, which returns the refusal reason for the GM row. The `squad.invite` span moved onto `issue`, so both entries keep it.
8. **`.squad_info [name]`** takes an optional name (the packet names no argument); without one it shows the GM's own squad.
9. **GM rows** are one INFO `org.gm_action` on the `org` target, written by the console, with the GM as actor and the named player as target (per the packet), and count on `squad_actions_total` (`gm_squad_invite` / `gm_squad_join` / `gm_squad_info`), since no `org_actions_total` helper exists yet.

## Telemetry

| Kind | Event | Target | Level | Fields | SigNoz filter |
|---|---|---|---|---|---|
| Span | `squad.chat`, `squad.ping` | `squad` | INFO | `entity_id` (`squad_id` on ping) | traces: span name = the event |
| Outcome | `squad.chat` (cell) | `squad` | INFO | `outcome`, `reason` (`not_in_squad`), `account_id`, `player_id`, `entity_id`, `squad_id`, `recipients`, `text_units` | `event = 'squad.chat'` (refusals: `AND outcome = 'rejected'`) |
| Outcome | `squad.chat` (base) | `squad` | INFO | `outcome = rejected`, `reason` (`rate_limited`, `text_invalid`), `account_id`, `player_id`, `entity_id`, `recipients = 0`, `text_units` | `event = 'squad.chat' AND reason IN ('rate_limited','text_invalid')` |
| Outcome | `squad.ping` | `squad` | INFO | `outcome`, `reason` (`not_in_squad`, `wrong_squad`, `rate_limited`, `not_ready`), `account_id`, `player_id`, `entity_id`, `squad_id`, `recipients = 0` | `event = 'squad.ping'` |
| Detail | `squad.ping_location` | `squad` | DEBUG | `x`, `y`, `z`, identity, `squad_id` | `event = 'squad.ping_location'` |
| Seam | `squad.send_failed` (chat) | `squad` | WARN | `entity_id` (recipient), `method_index = 28`, `squad_id`, `reason = cell_to_base_closed` | `event = 'squad.send_failed' AND method_index = 28` |
| GM | `org.gm_action` | `org` | INFO | `action` (`gm_squad_invite`, `gm_squad_join`, `gm_squad_info`), `outcome`, `reason`, GM `account_id` / `player_id` / `entity_id`, `target_account_id` / `target_player_id`, `squad_id` | `event = 'org.gm_action' AND action = 'gm_squad_join'` |
| Transition | `squad_created`, `member_joined` (from ORG-03's fanout, on `.squad_join`) | `squad` | DEBUG | as ORG-03 | `event = 'member_joined'` |
| Metric | `squad_actions_total{action = chat \| ping \| gm_squad_*}` | | counter | `reason = none` on `ok` | metrics: `squad_actions_total` by `action`, `reason` |

The generic chat logs in `chat/` keep `CHAT_LOG_TARGET`; every ORG-04 row above is on `squad` (or `org` for `org.gm_action`), per D-ORG24. The catalog row in `docs/architecture/observability.md` lists them.

## Commands run

All from the worktree root through `tools/build-lane/lane.sh` (target `B:\targets/org-04`).

| Command | Exit | Result |
|---|---|---|
| `cargo test -p cimmeria-cell-world --lib squad` | 0 | 26 passed |
| `cargo test -p cimmeria-base --lib chat_` | 0 | 22 passed |
| `cargo test -p cimmeria-cell-console --lib chat::` | 0 | 11 passed |
| `cargo test -p cimmeria-cell-methods --lib organization` | 0 | 52 passed |
| `cargo hakari generate`, `cargo hakari manage-deps --yes` | 0, 0 | no changes |
| `cargo fmt --all` then `cargo fmt --all -- --check` | 0 | |
| `cargo clippy -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base -p cimmeria-cell -p cimmeria-services --all-targets -- -D warnings` | 0 | |
| `cargo test -p cimmeria-cell-world -p cimmeria-cell-methods -p cimmeria-cell-console -p cimmeria-base -p cimmeria-cell --lib --no-fail-fast` | 0 | base 104, cell 468, cell-console 322, cell-methods 331, cell-world 446 |
| `cargo test -p cimmeria-server --bin cimmeria-server logging` | 0 | 53 passed |
| `python tools/crate-graph/crate_graph.py` | 0 | README graphs regenerated (the new edge) |
| `live-db-test.sh squad chat organization` (reloads `sgw_org_04`) | 0 | 196 run, 196 passed, the rest filtered out |

No test in this packet needs a database; the live-DB run checks the touched crates' existing live-DB tests.

## Regression proof

Each revert on the committed tree (head `74b6b51b1`), run, then restored with `git checkout HEAD -- <file>` and `touch`; `git status` clean after the run.

| Revert | Command filter | Exit | Failing guards |
|---|---|---|---|
| A. The `CHAN_SQUAD` arm disabled (falls to "not supported") | `-p cimmeria-cell-console --lib -- chat::tests::squad` | 101 | `squad_line_reaches_members_in_two_spaces_and_no_one_else`, `non_member_sends_nothing_and_gets_feedback`, `member_in_transit_is_skipped`, `dropped_send_warns` |
| B. Recipients limited to the speaker's space | same | 101 | `squad_line_reaches_members_in_two_spaces_and_no_one_else`, `dropped_send_warns` |
| C. The squadless speaker's feedback line removed | same | 101 | `non_member_sends_nothing_and_gets_feedback` |
| D. The ping's own-squad check removed | `-p cimmeria-cell-world -p cimmeria-cell-methods --lib -- ping` | 101 | `ping_is_members_only_and_names_their_own_squad`, `ping_naming_another_squad_is_refused` |
| E. The ping rate limit disabled | same | 101 | `ping_is_limited_to_one_per_second_per_member`, `ping_rate_limit_is_one_per_second` |
| F. `force_join`'s room check disabled | `-p cimmeria-cell-world --lib -- force_join` | 101 | `force_join_keeps_the_membership_rules` |
| G. The console's `.squad_*` arms unrouted | `-p cimmeria-cell-console --lib -- org04_squad every_spec_is_dispatched` | 101 | `gm_squad_join_then_info`, `gm_squad_invite_issues_an_invite`, `every_spec_is_dispatched` |
| H. The base's `squad.chat` refusal rows disabled | `-p cimmeria-base --lib -- chat_squad_refusals` | 101 | `squad_line_with_a_forbidden_character_logs_text_invalid`, `squad_flood_logs_rate_limited_once_per_notice` |
| I. The chat send-failure WARN downgraded to DEBUG | `-p cimmeria-cell-console --lib -- chat::tests::squad` | 101 | `dropped_send_warns` |
| J. The console's own GM check disabled (thin-entry-point round) | `-p cimmeria-cell-console --lib -- org04_squad` | 101 | `squad_commands_check_gm_themselves` |
| K. The console's `org.gm_action` row not emitted | same | 101 | `gm_squad_join_then_info`, `gm_squad_invite_issues_an_invite`, `gm_squad_join_unknown_name_is_refused` |

## Known gaps

- **Not tried with a real client.** Which channel id `/squad` sends is D-ORG14's constant (4, ORG-E1 Q5); the echo to the speaker follows the legacy channel. If the client also echoes squad lines locally, the speaker sees their line twice; drop the speaker from the recipients then.
- **A member in gate transit misses squad lines**; nothing is queued for chat.
- **No two-client wireclient test** for squad chat: the fan-out tests drive the cell with members in two spaces. ORG-03's `two_client_squad` could be extended to send a squad line.
- **The ping is never relayed**, by evidence (ORG-E1 Q3). If a receive path is found later, `receivedMinimapPing` would need a client method index that does not exist today.
- **`org_actions_total` does not exist yet**; the GM rows count on `squad_actions_total`.

## Integration edits for the coordinator

- Ledger: record decision 5 (the `cell-console → cell-methods` edge) and decision 3 (base-side `squad.chat` rows, `rate_limited` throttled with the feedback) against ORG-04.
- ORG-09 adds its team, command and officer arms to `chat/mod.rs` beside the `CHAN_SQUAD` arm, and can model them on `chat/squad.rs`.
- PR #893 (social, tells and ignore) also edits `chat/mod.rs`; this branch changes only the `mod squad;` line, the module doc line, the `CHAN_SQUAD` arm and one word of the fallback arm's comment.
