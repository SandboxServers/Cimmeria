# Movement and teleport telemetry review

Reviewer: movement-teleport-advisor, 2026-10-10. Data: colo SigNoz, last 7 days, aggregate queries only.
The reject side of movement is well instrumented (identity, world, gate, throttle, counters). Teleports are not. Every server-authoritative move is a `TeleportPlayer` whose base row says `reason="teleport_player"` whatever caused it. That row claims "sent" without checking the send result, and nothing records whether the destination was walkable.
The method 116 drop seen live is real and systematic: 195 of the 200 `client.dispatch.method_dropped` rows in 7 days are `onPlayerTeleport` on `SGWGmPlayer`, one per forced position (199 sent). No server row names the receiving client class, so the join has to be made by hand.

## 1. Inventory

| Target / scope | Level | In OTEL_FILTER | 7-day rows |
|---|---|---|---|
| `movement.validation` (reject, recovered, speed, gm bypass, space mismatch, suppressed) | WARN/ERROR, `entity_missing` DEBUG | yes (`movement.validation=debug`) | 2,548 WARN: speed 2,261, gm_bypass 281, reject 6 |
| `movement.player` (1-in-10 sampled) | DEBUG | yes | 14,259 |
| `movement.position_sample` | DEBUG | yes | 2,619 |
| `movement.navmesh` (loaded, mode summary, missing, advisory off-mesh) | INFO/WARN/TRACE | yes | 541 INFO, 4 WARN (`navmesh_missing`: CimmeriaLab, MapperDebug), 340 TRACE |
| `navmesh.load` / `cimmeria_entity::navigation::load` | ERROR / INFO | ERROR via `info`. `cimmeria_entity` has no row, so its DEBUG never ships | 290 INFO |
| `wire.out.forced_position` | DEBUG | yes | 199 |
| `cimmeria_base_world_entry::…::teleport` "snapping avatar" | INFO | yes | 199 |
| `cimmeria_cell::…::base_messages::movement` `EntityMove` | TRACE, unsampled, per packet | via the derived TRACE filter (`cimmeria-trace`) | 142,517 |
| `movement.movement_type` | DEBUG / TRACE sampled | yes | 4,818 / 82,857 |
| `movement.npc` (neighbour) | DEBUG | yes | **5,152,109** |
| GM travel (`console::gm::travel`, `console::travel`) | INFO/WARN | crate rows | 71 gmGotoXYZ, 73 in-space, 37 cross-space, 5 gmDHD |
| Gate travel (base + cell) | INFO/DEBUG/WARN | crate rows | 45 RESET_ENTITIES pairs, 47 dial-hub WARN |
| Ring transport | INFO/DEBUG/WARN/ERROR | crate rows | 20 INFO + 50 DEBUG. No abort or stall row fired |
| Respawn (`player.respawn`, `respawn::resync`) | INFO | yes | 6 / 185 |
| Client: `client.dispatch.method_dropped` | WARN | `cimmeria-client` | 200, of which 195 are `onPlayerTeleport`. Class `SGWGmPlayer` on 170, empty on 21 |

Nothing in this system is DEBUG-only and invisible on the colo. The gaps are in what the rows say, not whether they ship.

## 2. Positive gaps

- **No row for a server-authoritative move.** Ring, respawn, content teleport, content move, GM goto/summon/location and console travel all write the position and then call `SpaceManager::note_authorized_teleport` (`crates/cell-world/src/cell/space_manager/client_move.rs:661`). That writes only a `player.journal` `TELEPORT "authorized"` entry. There is no per-move row with identity, world, destination or a walkability verdict, so an off-mesh destination only shows up later as a client-side snap-back.
- **`wire.out.forced_position` cannot say why it was sent.** `crates/base-world-entry/src/base/world_entry/teleport.rs:145` hard-codes `reason = "teleport_player"`. The cause (snap-back, GM goto, ring, content, placement) is known only on the cell side, at the seven `CellToBaseMsg::TeleportPlayer` send sites.
- **The receiving client class is missing** from the forced-position row, although `ConnectedClientState::player_class_id` is in hand (`reanchor_player.rs:209`). It is what makes the 116 drop explainable.
- **The ring FSM has no transition rows (Rule 2).** `transporter/{mod,source,destination}.rs` hold zero log calls. `runtime/tick.rs::run_one_deadline` (`:126`) applies hide, warmup, remote_warmup and cooldown with no `event=` row. A trip is visible only through `wire_helpers` side effects.
- **Rule 5 misses.**
  - Every row in `crates/cell-console/src/cell/console/gm/travel.rs` (`:75`, `:247`, `:350`, `:484`, `:606`, and all refusals) has `entity_id` and a label but no `account_id`/`player_id`. Compare `console/travel/mod.rs:145`, which has them.
  - Content `teleport` / `cross_world_teleport` rows: `cell-content/src/cell/content/executor/transport.rs:46,114,160,202`.
  - Cross-world respawn: `cell-interactions/src/cell/respawn/mod.rs:221`.
  - `movement.player`: `cell/src/cell/service/base_messages/movement.rs:112`.
  - The ring abort rows (`cell-world/src/cell/ring_transport/transporter/manager.rs:204`) carry only `participants`, a count.

## 3. Negative gaps

- `teleport.rs:118`: `send_bundle_to_witness_reliable(...)` returns a `BundleSendOutcome`, and the call discards it. Line 123 then logs "FORCED_POSITION sent" unconditionally. A send the helper refused still reads as delivered. The helper logs its own miss, but on a separate, uncorrelated row.
- `respawn/mod.rs:155`: `let _ = tx.send(EntityMethodCall { ON_END_AID_WAIT })`. If the send fails, the Defeat Window stays open and nothing is logged.
- `ring_transport/runtime/tick.rs:268,275,278`: `return false` on an unknown deadline kind or a missing transporter, with no row. These should be unreachable, but nothing confirms that.
- `content/executor/world/movement.rs:181`: `let Some(t) = space_mgr.get_entity(target_id) else { return; }`. The INFO "move waypoint" row has already fired at that point, so the log shows a move that never happened.
- `gm/travel.rs:476-481`: gmGoto "caller entity not found" sends GM feedback but logs nothing. Its siblings at `:65` and `:201` do log.
- `space_manager/lifecycle.rs:59`: `navmesh_missing` is a once-per-space WARN with no counter, so a meshless world can't be alerted on or put on a dashboard. Containment, LoS and pathing all fail open there.

## 4. Noise

- **`movement.speed_warning` (2,261 WARN/week) is mostly packet bunching.** 1,059 rows (47%) have `dt_secs < 0.05` with ratio ≥ 10, at an average distance of 0.25 u. When dt ≤ 1e-4, `check_kinematics` (`crates/entity/src/movement_validation/mod.rs:308`) sets `implied_speed = INFINITY`, so `ratio` is infinite and `quantile(implied_speed)` comes back `nan` in ClickHouse. That poisons the "calibrate before enforcing" pipeline the warn exists for. All 2,261 rows came from 5 accounts, mostly DebugArea and CimmeriaLab (lab).
- **`navmesh_gm_bypass` WARN is per packet with no throttle** (`client_move.rs:298`): 281 rows/week, all from a GM in Castle_CellBlock. It is a normal GM action. It should be an INFO audit row, throttled per entity.
- **`EntityMove` TRACE** (`base_messages/movement.rs:54`) fires on every inbound position packet (142k/week). It is an unsampled per-packet firehose, which observability.md's pin rule forbids outside `wire.firehose.*`. `movement.player` (sampled, same fields plus velocity) and `movement.position_sample` already cover it.
- **Gate dial hub WARN** "granting a GM every enterable gate" (47/week, `cell-interactions/src/cell/gate_travel/dial_hub.rs`) is expected GM behaviour logged at WARN.
- **`movement.npc` DEBUG at 5.15M rows/week (~8.5/s)** is the largest stream in the movement family. It belongs to the NPC reviewer. The first-5-steps-of-every-leg rule dominates.

## 5. Seams (two hops)

| Hand-off | Sender logs | Receiver logs | Can tell who dropped it? |
|---|---|---|---|
| Cell `TeleportPlayer` → base `handle_teleport_player` (mpsc) | Each sender logs its cause row, and on send failure a WARN/ERROR. The snap-back is in `movement.validation` | INFO "snapping avatar" + DEBUG forced_position, but with no cause | Only by `entity_id` + time. With concurrent senders (snap-back during a GM goto) it's ambiguous |
| Base → client FORCED_POSITION + 116 bundle | forced_position row, send outcome discarded | Client `method_dropped` (116, class) and `client.sequence.dropped` | No. The server row has no `base_seq` and no class. The client row has `entity_id`, `msg_id=61`, `type_id=3` |
| Forced move → AoI refresh (neighbour) | Teleport rows | Witnesses learn the move from the next AoI `EntityMoved`, sampled 1-in-101 (`wire.out.avatar_update`) | No. Whether a witness was told the new position after a snap is unobservable 99% of the time. AoI reviewer: an unsampled "first `EntityMoved` after a forced position" row per witness |
| Teleport → regions/triggers (neighbour) | `player.journal` TELEPORT | Region dispatch DEBUG, journal `region_hint`. Region ids are client-supplied, and containment is never re-checked after a server move | Partly. The journal `seq` orders them, but a teleport into a trigger region the client never reports leaves no row |
| Gate travel → world entry (neighbour) | INFO RESET_ENTITIES sent, "awaiting ENABLE_ENTITIES" | `persist_arrival` DEBUG (45 = 45) | Yes for a completed trip. World-entry reviewer: confirm a timeout row exists when ENABLE_ENTITIES never comes |
| Ring FSM → base (`TeleportPlayer` / `GateTravel`) → world entry | dispatch ERROR on send failure. Abort WARN by region | As above | Abort: region yes, player no |
| Navmesh load → NPC pathing / combat LoS (neighbours) | Once-per-space WARN `navmesh_missing` | `npc_ai.path_fail reason=no_mesh` (counted). LoS fails open silently (`npc_ai.los` logs only blocked) | Pathing yes. LoS no: a hit through a wall in a meshless world looks like a normal hit |
| Respawn → reanchor → resync | `player.respawn`, respawn span with `target_world`/`same_world` | `respawn::resync` INFO | Yes, apart from the `ON_END_AID_WAIT` drop above |

## 6. Adversarial

1. **GM `.gotoxyz` (players hit this: ~once per goto).** SigNoz today shows `gmGotoXYZ: teleporting GM` (no account), `TeleportPlayer: snapping avatar`, `wire.out.forced_position reason=teleport_player`, and on the client index `client.dispatch.method_dropped method_index=116 class_name=SGWGmPlayer`. Nothing says the server sent 116 to a GM-class client, or whether the drop costs streaming (terrain not pre-loaded at a distant goto). It *should* show one forced-position row with `client_class=SGWGmPlayer`, `streaming_hint=sent`, `send=sent`, `base_seq`, plus the cell-side cause row. Then the client drop is an expected, classified pair, not a mystery. Whether to keep sending 116 to `SGWGmPlayer` is an RE question (TG-MOV-09).
2. **A player arrives in a world with no `.nav` (seen live: CimmeriaLab, MapperDebug).** Today SigNoz shows one WARN at space creation and nothing per player. Validation silently falls back to `SpaceBounds::FALLBACK`, LoS returns clear, and `position_sample` omits `on_navmesh`. It *should* also show a `navmesh_missing` counter by world, so a meshless world in a release is a dashboard tile, not a grep.
3. **A content or GM teleport lands off the mesh in an `enforce` world** (the gate-arrival class in my memory). Today the cause row shows (content teleport has no identity), then `validation_reject reason=navmesh` rows, then perhaps `correction_suppressed` ERROR. Only the timestamps connect them. It *should* show `movement.teleport event=authorized_teleport dest_on_mesh=false` at WARN in the same instant as the write.
4. **The speed tolerance is calibrated from the warn rows.** Today half the rows are infinite-ratio packet bunching, and a quantile over the set returns NaN. It *should* show bunched packets as their own DEBUG reason and only finite measurements at WARN.
5. **A ring trip stalls or a passenger disconnects mid-trip.** Today SigNoz shows `ring transport aborted reason=timeout participants=N` by region. A player reporting "stuck invisible after the rings" can't be found by `account_id`, and no transition rows show which state stalled. It *should* show one release row per passenger with identity, and DEBUG transition rows.

Out of scope, noted for the owner: respawn's same-world write `update_entity_position(entity_id, spawn_pos, [0,0,0], …)` (`respawn/mod.rs:285`) zeroes facing. The reanchor that follows probably masks it.

## Candidate packets

| ID | Title | Sev | Files | Domain agent? |
|---|---|---|---|---|
| TG-MOV-01 | Truthful, joinable forced-position row | high | 1 | no |
| TG-MOV-02 | `movement.teleport` row for every server-authoritative move | high | 2 + catalog | no |
| TG-MOV-03 | Speed warn: separate bunched packets from real measurements | med | 1 | no |
| TG-MOV-04 | Ring passenger release row with identity | med | 1 | no |
| TG-MOV-05 | Ring FSM transition rows | med | 1 | no |
| TG-MOV-06 | Split `gm/travel.rs` (704 lines) | low | 3-4 new | no |
| TG-MOV-07 | Rule 5 on GM travel rows (after 06) | med | ≤3 | no |
| TG-MOV-08 | Rule 5 on content teleport rows + silent move-waypoint return | med | 2 | no |
| TG-MOV-09 | Method 116 to `SGWGmPlayer`: RE the drop, decide send policy | med | RE + 1 | **needs-domain-agent** |
| TG-MOV-10 | GM navmesh bypass: INFO, throttled | low | 1 | no |
| TG-MOV-11 | Drop the per-packet `EntityMove` TRACE | low | 1 | no |
| TG-MOV-12 | `navmesh_missing` counter by world | low | 1 + doc | no |
| TG-MOV-13 | Respawn: log the `ON_END_AID_WAIT` send failure + Rule 5 on cross-world row | low | 1 | no |
| TG-MOV-14 | Dial-hub GM grant WARN → INFO | low | 1 | no |

**TG-MOV-01: Truthful, joinable forced-position row (high).**
File: `crates/base-world-entry/src/base/world_entry/teleport.rs`, `handle_teleport_player`.
Change: bind the `BundleSendOutcome` from `send_bundle_to_witness_reliable` (`:118`). Read `player_class_id` from the `ConnectedClientState` in the existing lock at `:68`. Add these fields to `wire.out.forced_position` (DEBUG):
- `send` = `sent` | the outcome's not-sent variant name
- `base_seq` and `packets` when sent
- `client_class` = `SGWPlayer` | `SGWGmPlayer` | `unknown`
- `streaming_hint = "method_116"`
- `player_id`, `player_name`

When not sent, emit WARN `reason="forced_position_not_sent"` with the full identity, and skip the "sent" row. Merge the INFO "snapping avatar" row into the DEBUG row only if the reviewer agrees. Otherwise leave it.
Test: a LogCapture unit test beside the existing `teleport_early_returns_*` tests. A mapped and connected GM-class client asserts `client_class=SGWGmPlayer` and `send=sent` on the forced-position row. A second test, with `entity_to_addr` mapped but the client in `connected` gone, asserts the WARN and no "sent" row. Reverting either change fails the test: the field goes missing, or the "sent" row reappears.
OTEL: none (`wire.out.forced_position=debug` is pinned).

**TG-MOV-02: `movement.teleport` row for every server-authoritative move (high).**
Files: `crates/cell-world/src/cell/space_manager/movement_telemetry/mod.rs` (new `log_authorized_teleport(&self, entity_id)`), `client_move.rs::note_authorized_teleport` (one call line; the file is 671 lines, so no logic there), `crates/server/src/logging/filters.rs`.
Every caller writes the position before calling `note_authorized_teleport` (verified at all 9 sites).
Event: `target: "movement.teleport"`, `event = "authorized_teleport"`, DEBUG. Fields:
- full identity (Rule 5/6) and `entity_name`
- `space_id`, `world`, `x`/`y`/`z`
- `navmesh_mode` (`enforce` | `advisory` | `none`)
- `dest_on_mesh` (`Option<bool>` from `diagnose_point`, absent when meshless)
- `is_player`

When `is_player && enforces_navmesh_containment(space) && dest_on_mesh == Some(false)`, emit WARN `reason = "teleport_dest_off_mesh"` and increment `movement_teleport_dest_off_mesh_total{world}`.
Test: a LogCapture unit test in `movement_telemetry/tests.rs`, using the existing fixture mesh. Put a player off-mesh in an enforce space, call `note_authorized_teleport`, and assert the WARN. Then put it on-mesh and assert DEBUG only. Reverting the call line fails both.
OTEL: add `movement.teleport=debug` to `OTEL_FILTER`; `target_scan_tests` requires it. Add a catalog row in `docs/architecture/observability-target-catalog.md`.

**TG-MOV-03: Speed warn bunching (med).**
File: `crates/cell-world/src/cell/space_manager/client_move.rs`, the `kin.speed_warn` arm (`:382`).
Change: when `sample.dt_secs <= 1e-4` (or `!implied_speed.is_finite()`), emit DEBUG `reason="speed_bunched"` (same target, same fields, `ratio` omitted) and count `movement_validation_warns_total{reason="speed_bunched"}` instead of the WARN. The validator's decision does not change.
Test: a LogCapture test with the time-injected `apply_client_position_update_at`, two packets at the same `Instant` 0.3 u apart. Assert no WARN and one DEBUG `speed_bunched`. A third packet at +0.1 s and 5 u away still WARNs with a finite ratio. Revert, and the WARN returns.
OTEL: none.

**TG-MOV-04: Ring passenger release row (med).**
File: `crates/cell-world/src/cell/ring_transport/runtime/teardown.rs::dispatch_release_effects`.
Event: in the `ShowPlayer` arm, emit INFO `event = "ring.passenger_released"` with full identity via `space_mgr.player_identity`, plus `world`. Target stays the module path.
Test: a LogCapture unit test feeding `[ShowPlayer{id}, UnlockMovement{id}]` for a seeded player. Assert the row carries `account_id`. Revert, and the test fails.
OTEL: none.

**TG-MOV-05: Ring FSM transitions (med).**
File: `crates/cell-content/src/cell/ring_transport/runtime/tick.rs::run_one_deadline`.
Event: after the effects are computed, emit DEBUG `event = "ring.transition"` with:
- `region_id`, `region_name`
- `deadline` (`hide` | `warmup` | `remote_warmup` | `cooldown` | `stall`)
- `state_before`, `state_after`
- `passengers` (count)

Also log the three `return false` arms at `:268,:275,:278` as DEBUG `reason = "deadline_unapplied"`.
Test: a LogCapture test driving one hide deadline through the existing ring test fixtures (`ring_transport/tests`). Assert one `ring.transition deadline=hide` row.
OTEL: none (the crate row is debug).

**TG-MOV-06: Split `gm/travel.rs` (low, refactor).**
`crates/cell-console/src/cell/console/gm/travel.rs` is 704 lines, over the hard cap, and TG-MOV-07 has to add lines to it. Promote it to `gm/travel/{goto_xyz.rs, goto_location.rs, dhd.rs, goto_summon.rs}` with `mod.rs` re-exports, and make no behaviour change.
Test: the existing `gm/tests/travel.rs` passes unchanged.

**TG-MOV-07: Rule 5 on GM travel rows (med, after 06).**
Files: the split `gm/travel/*.rs`.
Change: every `tracing::` call gets `account_id`, `account_name`, `player_id`, `player_name` from `space_mgr.player_identity(entity_id)`, matching `console/travel/mod.rs:144`. Also add the missing WARN in gmGoto's "caller entity not found" branch (`reason="caller_missing"`).
Test: a LogCapture test that runs gmGotoXYZ for a seeded GM and asserts `account_id` on the "teleporting GM" row.

**TG-MOV-08: Content teleport identity (med).**
Files: `cell-content/src/cell/content/executor/transport.rs`, `content/executor/world/movement.rs`.
Change: add Rule 5 fields to the four rows in `transport.rs`. In `world/movement.rs:181`, log DEBUG `reason="target_missing"` before the `return`.
Test: a LogCapture test that runs `teleport` for a seeded player and asserts `account_id`.

**TG-MOV-09: Method 116 on `SGWGmPlayer` (med, needs-domain-agent: movement-teleport-advisor + game-archaeology-specialist).**
The ordinary `SGWPlayer` class shows no drops in 7 days. There were also no non-GM teleports to compare against, apart from 6 snap-backs.
- RE in Ghidra: why the client's `SGWGmPlayer` dispatch drops `onPlayerTeleport` at index 116 (`play_character.rs:124` says GM methods only append), and whether the drop loses the streaming pre-load.
- Decide: either skip 116 for class 0x03 (with a forced-position row field `streaming_hint="skipped_gm_class"`), or route the hint differently.
- Until then, TG-MOV-01's `client_class` field classifies the drop.

**TG-MOV-10: GM navmesh bypass row (low).**
File: `client_move.rs:287-318`.
Change: INFO instead of WARN. Throttle through `movement_telemetry` admit (5 s per entity) with `suppressed`. The counter keeps firing on every occurrence.
Test: a LogCapture test with 3 off-mesh GM packets within 1 s gets one INFO row, `suppressed` absent; a 4th packet at +6 s gets a row with `suppressed=2`.

**TG-MOV-11: Drop per-packet `EntityMove` TRACE (low).**
File: `cell/src/cell/service/base_messages/movement.rs:54-59`. Delete the row. `movement.player` and `position_sample` supersede it.
Test: a LogCapture test that one `handle_entity_move` call emits no event with message `EntityMove`. It fails on revert.

**TG-MOV-12: `navmesh_missing` counter (low).**
File: `cell-world/src/cell/space_manager/lifecycle.rs:46,59`.
Add `movement_navmesh_missing_total{world, reason}`, where reason is `missing` | `load_failed`. Add the metric to the observability.md movement counter table.
Test: create a space for a world with no `.nav` in a temp space-data dir, and assert the WARN plus the counter via the existing metrics test recorder.

**TG-MOV-13: Respawn silent send (low).**
File: `cell-interactions/src/cell/respawn/mod.rs`.
Replace `let _ =` at `:155` with a WARN `reason="end_aid_wait_send_failed"` carrying identity, and add identity to the `:221` INFO row.
Test: a LogCapture test with a closed `tx`, asserting the WARN.

**TG-MOV-14: Dial-hub GM grant level (low).**
File: `cell-interactions/src/cell/gate_travel/dial_hub.rs`.
Change the two "granting a GM every enterable gate" rows from WARN to INFO. Keep the gmDHD outbound-only refusal at WARN.
Test: a LogCapture level assertion on the grant row.
