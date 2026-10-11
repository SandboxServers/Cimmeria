# AoI and witness telemetry review

- The introduction path is well instrumented on the happy side (`aoi.entity_enter`, `aoi.introduce`, `aoi.create_emit`, `aoi.cinematic_hold`), and every send-failure WARN is pinned or at WARN. None of those WARNs fired in 7 days, so they are unproven in production, not proven quiet.
- The real holes are the drops that never reach a WARN: the deferred-buffer cap drop names no entity, a leave that fails to send names no AoI seam, appearance rebroadcast has no positive row and its skips are DEBUG, and the one server row that could pair with T2 (`not_in_witness_aoi`, 145 in 7 days) cannot tell "raced a leave" from "never introduced".
- There is no AoI metric at all (no introduction/leave counters, no witness-set size), so a witness leak or a flush storm is invisible outside the lab-only `server_witnesses` tool.

## 1. Inventory

Colo SigNoz, `cimmeria-server`, last 7 days (`signoz_logs.distributed_logs_v2`, grouped by scope and severity).

| Target / scope | Level | Pinned in OTEL_FILTER | Fired (7d) | Source |
|---|---|---|---|---|
| `aoi.create_emit` | DEBUG | yes (filters.rs:340) | 23,640 | cell_dispatch/aoi.rs:88, deferred_flush.rs:46 |
| `aoi.entity_enter` | DEBUG | yes (:339) | 13,166 | cell-world space_manager/aoi.rs:117 |
| `aoi.entity_leave` | DEBUG | yes (:339) | 7,894 | space_manager/aoi.rs:333 |
| `aoi.introduce` | DEBUG | yes (:341) | 2,816 | aoi_dispatch.rs:244, deferred_flush.rs:283 |
| `aoi.cinematic_hold` | INFO | n/a (INFO, own target) | 124 (62 start, 62 release) | cinematic_aoi_hold/mod.rs:92, :217 |
| `reliable_fit` "AoI bundle: flushed" | INFO | module path | 7,791 | reliable_fit.rs:439 (T9 noise, see 4) |
| `deferred_flush` module rows | INFO and DEBUG | module path | 62 and 124 | deferred_flush.rs:214, :283-:372 |
| `aoi.create_send_failed` | WARN | n/a | 0 | aoi.rs:111, deferred_flush.rs:65 |
| `aoi.entered_no_witness_addr` | WARN | n/a | 0 | aoi_dispatch.rs:284 |
| `aoi.player_ghost_incomplete` | WARN | n/a | 0 | player_ghost.rs:88, :100 |
| `aoi.cascade_appearance_missing` | WARN | n/a | 0 | wire create.rs:473 |
| `aoi.witness_broadcast_failed` | WARN | n/a | 0 | witness_broadcast.rs:43 |
| `request_entity_update` (0x07) | WARN / DEBUG | `cimmeria_cell::cell=debug` | 145 / 13,024 | request_entity_update.rs:102, :122 |
| `abilities.wire` `wire_npc_no_witnesses` | WARN | n/a | 65 | cell-combat messaging.rs:246 |

DEBUG-only and invisible if the pin were lost: all four `aoi.*` DEBUG targets (pinned, with a test per observability.md), the per-message `Broadcast entity method to witness` and `EntityMoved while pre-ready` rows (aoi_dispatch.rs:374, trace), the appearance-refresh skips (appearance.rs:53, :68), and the deferred teardown discard (deferred_aoi.rs:280).

## 2. Positive gaps

- **Appearance rebroadcast has no success row.** `refresh_player_appearance` (base-methods inventory/appearance.rs:39) builds, caches (:103), sends to self (:107) and calls `broadcast_to_witnesses` (:130). Nothing says "rebroadcast to N witnesses". The cell arm (cell base_messages/mod.rs:331) discards the `usize` that `send_entity_method_to_witnesses` returns (messaging.rs:171). With zero witnesses the only trace is a DEBUG `abilities.wire` row (messaging.rs:267). Holster toggles and equips are the failure-mode-3 shape, and there is no INFO evidence either way.
- **Deferred `LeftAoI` buffering is silent.** `entered_aoi` logs `deferred_not_ready` (aoi_dispatch.rs:244); `left_aoi` buffers with no row (:335-:342). An enter that is later cancelled by a leave in `lifecycle_segments` shows one buffer row and no pairing row.
- **No flush-source label.** `reliable_fit` rows carry `send_kind = "witness_bundle"` for every bundle (helpers/mod.rs:582 hardcodes it), so a deferred-flush phase-1/phase-2 bundle cannot be told from a live batch. This is the prerequisite for T1's per-message detail.
- **Entity enter/leave rows name only the witness.** `aoi.entity_enter` and `aoi.create_emit` carry `witness_id` (+ `witness_name` on the cell side) but no `account_id`/`player_id` (hot-path rule is annotated `nt:id-only`, acceptable). Unverified: whether `entity_names(eid)` on `aoi.entity_leave` (space_manager/aoi.rs:332) still resolves a destroyed entity after `destroy_entity`; a leave for a despawned NPC may log with null names.
- **No AoI metrics.** `grep` for counters/gauges named aoi/witness/introduction finds none. Compare `npc_respawns_total` (npc_respawn/mod.rs:423).

## 3. Negative gaps

| Site | What is silent or weak |
|---|---|
| deferred_aoi.rs:215-221 | Cap-drop WARN has only `addr` and `buffered`. No `entity_id`, message kind or witness name. The dropped message is usually an `EnteredAoI`; the cell already marked the witness set, so the entity stays invisible until relog (same class as `aoi.entered_no_witness_addr`). |
| aoi_dispatch.rs:253, :336 | `push_deferred` returns `DeferOutcome`; both callers discard it (`BufferFull`, `SessionGone`). |
| aoi.rs:241-248 (`left_aoi`), :470-477 (`entity_invisible`) | `send_to_witness_reliable` outcome discarded. `entered_aoi` has `log_create_emit`; leave and invisible have no AoI-specific failure row (the helper logs its own generic line, no entity id). A lost leave leaves a ghost on the client. |
| cinematic_aoi_hold/mod.rs:168-198 | Six silent `return`s in `release`: lock poisoned, session gone, hold absent, already releasing, token mismatch. A `hold_started` with no `hold_released` has no explanation row. |
| deferred_aoi.rs:275-280 | `log_discarded_on_teardown` is DEBUG. A player who quits during the intro movie drops up to 512 buffered introductions with nothing at INFO+. |
| inventory/appearance.rs:49-76 | Both early returns are DEBUG "skipping". A skip while the player is connected (state missing for addr) means no recomposite, no cache update, no witness fan-out. Should WARN unless the witness recently departed. |
| appearance.rs:97-100 | Second `None => return` after the cache write has no row at all. |
| space_manager/aoi.rs:75-88, :104-109, :266, :281 | `introduction_events`/`push_introduction`/`compute_player_aoi` return empty silently. Practically unreachable inside the tick, but `introduction_events` backs the NPC respawn re-create (npc_respawn/mod.rs:371-374, `continue` on empty). That one is covered by `recreated` vs `witness_count` on the INFO row. |
| request_entity_update.rs:102-115 | WARN `not_in_witness_aoi` does not say whether the entity exists, is in another space, or just left. See 6.C. |

Positive finding: `witness_broadcast.rs:32` (`cell_tx` None) is silent by design and documented; the closed-channel WARN is tested (:63).

## 4. Noise

- `reliable_fit.rs:439` INFO "AoI bundle: flushed N messages in 1 packet(s)": 7,791 rows, 7,477 of them the 2-message single-packet case. Already T9; the proposed demotion should be to DEBUG unless `packets > 1` or `fragmented` (7791 rows minus the ~330 multi-packet ones). Not repeated as a packet.
- `request_entity_update.rs:122` DEBUG "acknowledged": 13,024 in 7 days, once per NPC entry. Acceptable at DEBUG; do not promote.
- `abilities.wire` `wire_npc_no_witnesses` WARN (65) and `abilities.sequence` (34): gated on `player_present`, not noise.
- `deferred_flush.rs:214` INFO "Flushing deferred-AoI buffer" and the two phase DEBUG rows duplicate `log_deferred_flush` (method_delivery); 62 rows, harmless.

## 5. Seams (two hops)

| Neighbour | Hand-off | Do both sides log enough to tell which side dropped it? |
|---|---|---|
| Mercury bundle encoder / client receive (T1, T2, #1341) | `send_bundle_reliable_to_addr` -> client | Server logs a flush with `packet_bytes` only. Client side logs the BigWorld error, not the entity. Which entity a dropped bundle lost is unrecoverable (T1). Add the flush source label (TG-AOI-04) so deferred flushes are separable. Two hops: the client's `svidFollow ... unknown` (T2) can be joined only to `aoi.introduce`/`create_emit` by entity id, which holds. |
| Movement and teleport | position snap -> cell `EntityMoved` / forced position; AoI refresh via `compute_aoi_changes_for_player` | `aoi.entity_enter/leave` fire at the diff, but no row ties a snap to "AoI refreshed for N witnesses". A teleport that moves the entity out of view logs a leave, nothing says why. Neighbour movement agent should own the snap row; AoI side logs correctly. |
| NPC spawn / despawn | `CreateEntity` -> introduction at next tick (periodic 100 ms), `destroy_entity` -> leave via diff | Spawn: `aoi.entity_enter` DEBUG fires. Despawn: `destroy_entity` sends no explicit leave; leave arrives next tick via the diff. If the witness's addr is gone the helper WARN has no entity id. Two hops: respawn re-create logs `npc_respawn_recreate` with `recreated`/`witness_count` (good). |
| Combat (death, effects) | `send_entity_method_to_self_and_witnesses` -> `WitnessEntityMethod` | Strong on the cell side (`abilities.wire` rows, `wire_npc_no_witnesses`). Base side `method_delivery` logs outcomes. Seam is covered; the appearance path (equip/holster) uses a different helper that is not. |
| World entry / gate travel | `onClientReady` -> deferred flush; cinematic hold arms/releases | Hold start/release paired and matched 62/62. Gap: abandonment (player quits mid-hold) and cap-drop have no INFO+ row (TG-AOI-01, -06). |
| Inventory/equip -> appearance (neighbour of combat and items) | `refresh_player_appearance` -> self send + `BroadcastToWitnesses` | Self send outcome is discarded (appearance.rs:107, no check); witness side covered only by the closed-channel WARN. Gap in TG-AOI-03 and -05. |
| Two hops: DB (player load for appearance) | `query_player_load_data` | If it returns defaults on error the appearance is rebuilt from empty data and rebroadcast to everyone; not examined here, for the persistence review. |

## 6. Adversarial scenarios

**A. A player enters a crowded world and half the NPCs never appear (cap drop).** A session pre-`onClientReady` holds more than 512 introductions (many NPCs, or leaves plus enters). Today SigNoz shows one WARN per dropped message: `Deferred-AoI buffer at cap; dropping message`, with `addr` and `buffered=512`. No entity, no witness name, no kind. Should show: `event=deferred_dropped`, `witness_id`, `witness_name`, `entity_id`, `kind=entered|left|method`, `reason=buffer_full`. (TG-AOI-01)

**B. Another player's gear or holster never updates for observers (failure mode 3).** Today: a successful equip leaves no INFO row, a skipped refresh is DEBUG-only (invisible on the colo), and a rebroadcast to zero witnesses is a DEBUG `abilities.wire` row. A developer cannot tell "rebroadcast went to 3" from "never fired". Should show one INFO per rebroadcast, `event=appearance_rebroadcast`, `entity_id`, `player_name`, `witness_count`, and a WARN when the refresh is skipped for a connected player. (TG-AOI-02, -03)

**C. A player sees an NPC that is not there, or the client asks about an entity the server says is not in its AoI.** Seen in production: 145 `not_in_witness_aoi` WARNs in 7 days. The row names witness, entity, template, account and player, but not whether the entity still exists, shares the space, or just left. The two causes need opposite action: a race with a leave is benign, a never-introduced entity is the T2 smoking gun. Should show `cause = entity_gone | other_space | not_introduced_yet | outside_aoi`. (TG-AOI-05)

**D. A player quits during the first-login movie.** Today: `hold_started` INFO, then nothing; `log_discarded_on_teardown` is DEBUG. 62/62 matched so far, so this has not happened in the sampled week, but nothing would show it. Should show an INFO `hold_abandoned` with `reason=session_ended`, `held_ms`, `discarded`. (TG-AOI-06)

**E. Witness set grows or stops being refreshed (failure mode 1).** Today there is no signal: no gauge, no log of set sizes; only the lab `server_witnesses` call. Should show `aoi_introductions_total`/`aoi_leaves_total` by world, plus a witness-set size gauge so a mismatch (enters minus leaves drifting) alerts. 7d ratio is 13,166 enters to 7,894 leaves, which is expected churn, but there is nothing to assert against. (TG-AOI-07)

## Candidate packets

| ID | Title | Severity | Files | needs-domain-agent |
|---|---|---|---|---|
| TG-AOI-01 | Name the entity and kind on a deferred-buffer cap drop | high | deferred_aoi.rs, aoi_dispatch.rs | no |
| TG-AOI-02 | Positive `appearance_rebroadcast` row on the cell side | med | cell base_messages/mod.rs | no |
| TG-AOI-03 | WARN when appearance refresh is skipped for a connected player | med | inventory/appearance.rs | no |
| TG-AOI-04 | Separate `send_kind` for deferred-flush bundles | med | helpers/mod.rs, deferred_flush.rs | no |
| TG-AOI-05 | Classify `not_in_witness_aoi` (raced vs never introduced) | med | request_entity_update.rs | no |
| TG-AOI-06 | Close the cinematic-hold lifecycle (abandon, early returns) | low | cinematic_aoi_hold/mod.rs, deferred_aoi.rs | no |
| TG-AOI-07 | AoI introduction/leave counters and witness-set gauge | med | space_manager/aoi.rs, cell tick | no |
| TG-AOI-08 | AoI-specific failure row for leave and invisible sends | low | cell_dispatch/aoi.rs | no |

### TG-AOI-01: cap drop names the entity (high)
- Files: `crates/base-session/src/base/deferred_aoi.rs` (`push_deferred`, :215-221), `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi_dispatch.rs` (`entered_aoi` :253, `left_aoi` :336).
- Change: in the cap branch, add `event = "deferred_dropped"`, `reason = "buffer_full"`, `kind` (`entered|left|method|witness_method|invisible`) and `entity_id` taken from the `DeferredAoiMsg` variant, and `witness_name` via `known_names`/`known_names::player_name` if available in `ConnectedClientState`. Keep WARN. In both callers, also stop discarding the outcome: on `SessionGone` log DEBUG `reason=session_gone` (race). No OTEL_FILTER change (WARN reaches SigNoz).
- Test: unit, `LogCapture` (TESTING.md: negative-logging seam). Extend `push_deferred_drops_at_cap` (deferred_aoi.rs:516) to assert a WARN with `event=deferred_dropped`, `reason=buffer_full`, `entity_id=<the pushed id>`. Reverting leaves the old WARN without `entity_id` and the assertion fails.

### TG-AOI-02: appearance rebroadcast success row (med)
- Files: `crates/cell/src/cell/service/base_messages/mod.rs` (arm at :331).
- Change: capture the returned `usize`, emit INFO `target: "aoi.appearance"`, `event = "appearance_rebroadcast"`, `entity_id`, `entity_name = space_mgr.entity_label`, `method_index`, `witness_count`. Use `space_mgr.player_identity` for `account_id`/`player_id`. Level INFO is fine: one per equip/holster action. Add `aoi.appearance` to nothing: INFO targets export without a pin (confirm against `filters.rs` the INFO default; if not, add `aoi.appearance=info`).
- Test: unit, `LogCapture`, drive the `BroadcastToWitnesses` arm with one witness and with zero; assert the INFO row has `witness_count=1` and `=0`. Removing the log fails the find.

### TG-AOI-03: skipped appearance refresh warns (med)
- Files: `crates/base-methods/src/base/world_entry/methods/inventory/appearance.rs` (:49-76, :97-100).
- Change: the entity_to_addr miss uses `helpers::witness_recently_departed(entity_id)` to pick DEBUG `reason=witness_session_ended` versus WARN `reason=entity_to_addr_miss`; the `clients.get` miss is WARN `reason=client_state_missing`; give the second `None => return` (:99) the same WARN (`reason=entity_gone_mid_refresh`). Fields: `entity_id`, `player_id`, `player_name`.
- Test: unit, `LogCapture`: call with unmapped entity not recently departed -> WARN; with `note_witness_departed` first -> DEBUG only. Reverting to DEBUG-only fails the first.

### TG-AOI-04: flush-source `send_kind` (med)
- Files: `crates/base-session/src/base/helpers/mod.rs` (:551-585), `crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs` (:349, :374).
- Change: add `send_bundle_to_witness_reliable_kind(..., kind: &'static str)`; the existing fn delegates with `"witness_bundle"`. `deferred_flush` passes `"aoi_flush_create_base"` and `"aoi_flush_cascade"`. The `reliable_fit` INFO row then carries the new `send_kind`. No new fields elsewhere.
- Test: unit, `LogCapture`, in `deferred_flush` tests: flush two buffered `EnteredAoI`, assert the INFO "AoI bundle: flushed" row has `send_kind=aoi_flush_create_base`. Fails if the hardcoded kind returns.
- Note: prerequisite for T1, which wants per-message offsets in these bundles.

### TG-AOI-05: classify `not_in_witness_aoi` (med)
- Files: `crates/cell/src/cell/service/base_messages/request_entity_update.rs` (:90-117).
- Change: compute `entity_exists = space_mgr.get_entity(entity_id).is_some()`, `same_space = space_mgr.entity_space(entity_id) == space_mgr.entity_space(witness_id)` (use the existing accessor, check name), `introducible`; add `cause` = `entity_gone | other_space | not_introduced_yet | outside_aoi`. Keep WARN and the reason token (tests pin it).
- Test: unit, `LogCapture`, extend the existing refuse test: witness with no such entity -> `cause=entity_gone`; a live NPC in another space -> `cause=other_space`. Fails if the field is removed.

### TG-AOI-06: close the cinematic-hold lifecycle (low)
- Files: `crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs` (:168-198), `crates/base-session/src/base/deferred_aoi.rs` (:275-280).
- Change: replace the five silent `return`s in `release` with a DEBUG `event=hold_release_skipped` and `reason` (`session_gone | already_releasing | token_mismatch | no_hold`); in `log_discarded_on_teardown`, when the buffer is non-empty and a hold was active, log INFO `event=hold_abandoned`, `discarded`, `held_ms`.
- Test: unit, `LogCapture`: call `release` against a map with no session (expect `session_gone`); and teardown with a non-empty buffer (expect INFO). Reverting fails both.

### TG-AOI-07: AoI counters and witness-set gauge (med)
- Files: `crates/cell-world/src/cell/space_manager/aoi.rs` (`push_introduction` :96, leave loop :329), cell service AoI tick (where `compute_aoi_changes` is called).
- Change: `counter!("aoi_introductions_total", "world")` at `push_introduction`, `counter!("aoi_leaves_total", "world")` in the leave loop, and a per-tick gauge `aoi_witness_set_size` (sum over player entities, label `world`). Low cardinality (about 30 worlds). Register in the metric catalog doc if one exists.
- Test: unit using the existing metrics test recorder (`npc_respawns_total` precedent, find via `rg "npc_respawns_total" crates`); assert the counter increments by the number of events from `compute_aoi_changes`. Removing the `counter!` fails.
- OTEL: confirm the metric reaches SigNoz through the metrics exporter; no `OTEL_FILTER` change expected.

### TG-AOI-08: leave and invisible failure row (low)
- Files: `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi.rs` (`left_aoi` :229-249, `entity_invisible` :456-478).
- Change: keep the outcome, and on non-`Sent` log WARN `target: "aoi.create_send_failed"` reuse with `phase = "leave"` / `"invisible"`, same fields as `log_create_emit` (the downstream `departed_witnesses` DEBUG downgrade applies via `failure_reason`). Use the existing `AoiNames`.
- Test: unit, `LogCapture`, next to `left_aoi_fans_out_one_packet_per_witness_to_each_addr` (:497): call with an unmapped witness, assert WARN `phase=leave` and `entity_id`. Removing the outcome check fails it.
