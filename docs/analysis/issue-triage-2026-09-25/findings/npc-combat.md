# Triage findings — batch `npc-combat`

Code of record: `main-ro` at 059d6038 (2026-09-25, NA22 close-out #790). Read-only research; nothing posted.

## #784 — Line of sight from collision geometry (per-space occluder sidecar)

- Verdict: NEEDS-OWNER
- Priority: P2
- Labels: remove `needs-triage` once the owner answers; add `enhancement`. Do not add `ready-for-agent` until the size budget is set.
- Summary: Filed 2026-09-25 as the NA16 follow-up. Every factual claim checks out: LoS is a navmesh raycast, NA16 measured 269/601 (45%) false `Blocked` and 5/3,399 false `Clear`, D-NA11 (`permits_stationary_attack`) is the stationary-only stopgap, and there is no fire-time LoS check. The body already names its blocking question: the per-world data-size budget for a new shipped artifact (`data/spaces/<world>.occ`). That is an owner call. The rest is ready.
- Evidence:
  - `crates/entity/src/navigation/line_of_sight.rs:129` `permits_stationary_attack`, applied via `SpaceManager::attack_line_of_sight` (`npc_ai/fight.rs:159`).
  - `docs/analysis/npc-ai-restoration/audit.md` row S15 (NA16 numbers); README D-NA11 ("Retired when the collision-geometry occluder lands (#784)"); `handoffs/session-resume.md` open-items table lists #784.
  - `crates/services/src/cell/space_manager/lifecycle.rs` exists (load site named in the proposal). `crates/navmesh-extractor/` exists with `obj`, `floor_probe`, `coverage` modules to build on.
  - `entities/defs/enumerations.xml:1250` `CONDITION_FEEDBACK_NoLOS = 40` matches the body.
- Related/duplicates: #46 (same extractor pipeline), D-NA08/D-NA11. Retires the "stationary NPCs shoot through same-floor walls" cost (~40 spawns, including Harset).

### Action text

Status comment (no body change):

> Triage 2026-09-25: the body matches main (059d6038). NA16 (#786) shipped the D-NA11 stopgap, and the audit S15 numbers are as quoted. One owner decision blocks this: **what per-world size budget is acceptable for a new `data/spaces/<world>.occ` artifact?** For scale, Cellblock collision is 1.53 M triangles (about 40 MB as an untrimmed 0.5 m grid), and Castle is several times larger. Options: (a) a fixed cap such as ≤5 MB per world after trimming to walkable area plus margin, with compression; (b) ship `.occ` only for worlds with stationary shooters (12, 8, 57, 68); (c) generate it at server start from the cooked maps rather than committing it. Once that's answered this is ready-for-agent (npc-ai-spawn-advisor plus game-archaeology-specialist).

## #407 — NPC AI: structured decision_outcome + zone field to surface no-path stationary mobs

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: All three asks are on main. Every AI tick runs inside an `npc_ai.decision` span with `space_id` and a `decision_outcome` field. The vocabulary (attack_in_place, stationary_holds, min_range_backup, no_ability, move_to_cover, no_path, and others) is documented. Path failures carry a separate reason (no_mesh vs no_path) keyed by space. A `stuck` detector covers the stretch goal. PR #410's description says "Closes #407", but GitHub did not auto-close the issue.
- Evidence:
  - `crates/services/src/cell/service/npc_ai/dispatch.rs:156-168` (span with `space_id`, `decision_outcome`); `fight.rs:194,269,314,328` outcomes.
  - `npc_ai/path_failure/mod.rs:80,114,233` (no_mesh/no_path split, space lookup).
  - `npc_ai/detectors/sweep.rs:8,21,117` (`npc_ai event=stuck`, `npc_off_mesh`).
  - `crates/server/src/logging.rs:49,531` (`npc_ai=debug` in the OTLP allowlist, test `otel_filter_prefix_matching_exports_npc_ai_children`).
  - `docs/architecture/observability.md:277` `npc_ai.decision_outcome` enum. Commit 8eeaee7a (#410) "Closes #408 and #407", extended by NA00/NA02 (#776, #781).
- Related/duplicates: #408 (closed).

### Action text

Closing comment (reason: completed):

> Done on main. #410 added the structured `npc_ai.decision` span with `decision_outcome`, and NA00/NA02 (#776, #781) extended it. The span carries `space_id` (`npc_ai/dispatch.rs:156`). Path failures report `no_mesh` vs `no_path` per space (`npc_ai/path_failure/`). The stationary/stuck signal is the `npc_ai event=stuck` detector (`npc_ai/detectors/sweep.rs`). `npc_ai=debug` is in the OTLP allowlist (`crates/server/src/logging.rs`), and the outcome vocabulary is in `docs/architecture/observability.md` §`npc_ai.decision_outcome`. The SigNoz "NPC AI health" dashboard from NA03 (#782) pivots on it. #410 said "Closes #407", but the issue never auto-closed.

## #406 — Ring transport: server-driven cinematic camera via onSequence (Ghidra-confirmed)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: The owner's own 2026-05-26 comments settled it: the server already sends `onSequence` with `KISMET_VIEW_EventInvoker = 3` on Teleport_Out/Teleport_In, the DB rows resolve, and the gap is on the client/content side. The side fix proposed there (decode `view_type` in the wire log) has landed. The client-side facts are now in `docs/analysis/ring-transport-cellblock-castle/` (#750): the client resolves sequence ids from its cooked PAK and not the DB, and rigs are reached by object path. New sequences are delivered per key (#755). Nothing server-side remains under this title.
- Evidence:
  - `crates/services/src/cell/ring_transport/wire_helpers.rs` `send_play_sequence` (EventInvoker = 3); `transporter/mod.rs` TeleportOut/In effects.
  - `crates/services/src/wire_log/decoders/outbound.rs:15-41` `decode_on_sequence` now emits `view_type`.
  - `docs/analysis/ring-transport-cellblock-castle/README.md` §"How the client plays a ring sequence" and the Phase 1 status (sequence 772 rig; `.net_seq 1951 3` as the region-3 control); PRs #750, #751, #753, #755.
- Related/duplicates: #750/#751/#753/#755 (ring-transport client-patch track, mission 688).

### Action text

Closing comment (reason: completed):

> Closing. The original ask (drive the ring cinematic from the server via `onSequence`) was already implemented, as the 2026-05-26 comments found: `ring_transport/wire_helpers.rs::send_play_sequence` sends ViewType 3 (EventInvoker) on Teleport_Out/In. The proposed decoder fix has also landed (`wire_log/decoders/outbound.rs` now reports `view_type`). The remaining questions are client/content ones: how the client resolves a sequence id (from its cooked `CookedDataKismetSeqEvent.pak`, not the DB) and how it finds a rig (by object path). They're answered in `docs/analysis/ring-transport-cellblock-castle/README.md` (#750), and new sequences reach clients per key since #755. If a stock ring (regions 1-3) still shows no animation in-client, please open a fresh bug with the `.net_seq 1951 3` control result from that doc's decision table.

## #330 — feat(cell): aggression-override server→client wire-out (onAggressionOverrideUpdate / Cleared)

- Verdict: REWRITE
- Priority: P3
- Labels: add `needs-triage` (the method index still needs binary verification before `ready-for-agent`)
- Summary: About half the body is now false. NA13 (#787) built the server side: `MobAggression` enum (1-5), a per-spawn `spawnlist.aggression_override` column with a CHECK constraint, runtime override via the `set_aggression` content action, spawn param and GM `.aggression`, and faction-reaction fallback. So "no aggression handling code today", "searching crates/ returns zero matches" and "add a column to entity_templates" no longer hold (the column went on `spawnlist`). The wire-out is still missing. `docs/gameplay/npc-ai.md:221` says the server sends neither `onAggressionOverrideUpdate` nor `GENERICPROPERTY_MobAggression`, because the SGWMob flat method index (27 by the flattening rule) is not binary-verified. That doc also says the value is display-only: the client derives friend or foe from faction. This undercuts the body's "blocks quest flows" motivation, so this is a P3 polish item.
- Evidence:
  - `crates/entity/src/cell_entity/aggression.rs` (enum + override/radius); `crates/services/src/cell/combat/faction_reaction.rs`.
  - `db/resources/Worlds/Tables/spawnlist.sql:33-41` (`aggression_override smallint` + CHECK 1..5).
  - `crates/services/src/cell/content/executor/world/mod.rs:91-94` ("does not yet… `onAggressionOverrideUpdate`… flat method index is not binary-verified").
  - `docs/gameplay/npc-ai.md:148-152, 221, 606` (PARTIAL; "No client broadcast yet and no timed revert").
  - `entities/defs/SGWMob.def:556-561` (both client methods declared); `docs/analysis/event-net-mapping.md:907-908` (registration addresses 0x019bdf64 / 0x019bdf8c).
  - `docs/analysis/npc-ai-restoration/handoffs/session-resume.md` open-items row "`onAggressionOverrideUpdate` client broadcast".
- Related/duplicates: NA13 (#787), D-NA01.

### Action text

Comment:

> Triage 2026-09-25: NA13 (#787) has built the server half of this. Main has the `MobAggression` enum (`crates/entity/src/cell_entity/aggression.rs`). The override is per spawn in `spawnlist.aggression_override`, not on `entity_templates`, and can be changed at runtime by `set_aggression`, the spawn parameter and the GM `.aggression` command. When no override is set, the faction-reaction table applies. What's still missing is the client wire-out. The SGWMob flat index for `onAggressionOverrideUpdate` (27 by the flattening rule) isn't binary-verified, so nothing is sent (`docs/gameplay/npc-ai.md` §"Wire: not broadcast yet"). The same doc notes the value is display-only, because the client derives friend or foe from faction. I've rewritten the body to that remaining scope and lowered the priority.

#### New body

## Problem

The server computes an NPC's effective aggression toward players (NA13: `spawnlist.aggression_override`, the `set_aggression` / spawn-param / `.aggression` overrides, else the faction reaction), but never tells the client. The 2009 server sent `onAggressionOverrideUpdate(INT8 aAggressionLevel)` from `createOnClient` and on every `setAggression`, and `onAggressionOverrideCleared()` on revert. Rust sends neither, so the client's `GameMob+0x16c` cache stays at DEFAULT (5) for every mob. This is display-only: friend or foe comes from faction. There is also no timed revert (python `overrideAggression(level, entityBase, seconds)` / `aggressionOverrideTimers`).

## Evidence

- Client handler `0x00d31bd0` stores the INT8 at `GameMob+0x16c` and is registered via `MemberCallback<GameMob, Event_NetIn_onAggressionOverrideUpdate>` (`docs/gameplay/npc-ai.md:221`). The Cleared handler is inline at `0x00d31770` and writes 5. Registration strings are at `0x019bdf64` / `0x019bdf8c` (`docs/analysis/event-net-mapping.md:907-908`).
- `entities/defs/SGWMob.def:556-561` declares both client methods. The flat index is 27 by the flattening rule (Lootable has no client methods), but that is **not binary-verified**.
- Server-side state: `crates/entity/src/cell_entity/aggression.rs`, `crates/services/src/cell/combat/faction_reaction.rs`, `crates/services/src/cell/content/executor/world/mod.rs:91-94` (the explicit "not yet" note).
- No client handler for `onEntityProperty` type 6 (`GENERICPROPERTY_MobAggression`) was found, so the method is the only known carrier.

## Acceptance criteria

- The SGWMob client-method index for `onAggressionOverrideUpdate` and `onAggressionOverrideCleared` is verified against `SGW.exe` and recorded in `docs/protocol/client-method-dispatch-table.md` and the `method_idx` constants.
- When an NPC with a non-DEFAULT effective level enters a player's AoI, that player receives `onAggressionOverrideUpdate` with the level byte.
- A runtime change (`set_aggression`, `.aggression`, surrender to NEUTRAL) sends Update to current witnesses. A revert to "no override" sends Cleared.
- Optional, if a content scenario needs it: a timed override that reverts after N seconds (python `overrideAggression`).
- An in-client check shows that nameplate or reticle colour changes for a FRIENDLY-override hostile-faction mob. If it doesn't, record that and close as display-inert.

## Test type

- Wire-format (byte-exact: method index + single INT8; Cleared has an empty payload).
- Fan-out byte (AoI-entry and runtime-change fan-out to witnesses only).
- Negative: no Update is sent for an NPC whose effective level equals the faction default. Decide whether that means "no override set" or "level equals faction reaction", and pin it.

## Docs to update

`docs/protocol/client-method-dispatch-table.md`, `docs/protocol/message-catalog.md`, `method_idx` in `crates/services/src/mercury/mod.rs`, `docs/gameplay/npc-ai.md` §"Wire: not broadcast yet" (flip to DONE).

## Client impact

Free: an existing client method, no new opcode.

## Domain advisor

npc-ai-spawn-advisor (level resolution), aoi-witness-broadcast (fan-out), game-archaeology-specialist (index verification).

## Needs a human for

RE (confirm the flat index in Ghidra) and a short in-game UAT to see whether the client visibly reacts.

## #209 — NPC AI ignores cover system — guards path straight at the player

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: remove `ready-for-human`
- Summary: Cover is implemented end to end. NA20 (#777) located world-space cover. NA21 (#780) extracted 236 nodes / 58 sets for Cellblock and 3,788 / 481 for Castle into per-world seeds. NA22 (#790) built the behaviour: spawn-hold, seek within attack range, flank release, squad affinity, `entity_templates.use_cover`, and Cover Stance (ability 1451). Reservation, scoring, spatial index, player cover detection (`onEnterCoverSet`) and content cover triggers exist under `crates/services/src/cell/cover/`. Two assumptions in the issue's plan were disproved. `aMovementType=0` is not a server-to-client message (D-NA10; `0x00deb660` is the GM `onShowPath` visualiser). The prefab-local `covernodes_*.pak` rows were dropped. The leftovers (Cover Stance has no combat effect because `COVER_DEFENSE` isn't read by hit resolution; the crouch pose is an owner experiment) are listed as open items in the NA session-resume and belong in new, narrow tickets.
- Evidence:
  - `crates/services/src/cell/cover/{ai_integration,reservation,scoring,spatial,detection,stance,loader}.rs`; `npc_ai/fight_cover.rs:86,121,134,289` (use_cover gate, move_to_cover / stay_in_cover / no_cover outcomes).
  - `crates/services/src/cell/content/event_dispatch/cover.rs` (player cover triggers).
  - `docs/analysis/npc-ai-restoration/work-packets.md` NA20/NA21/NA22 status lines; `README.md` D-NA05, D-NA10; `docs/architecture/cover-system.md`.
  - `crates/services/src/cell/effects/cover_stance.rs:22` ("What the stat does today: nothing in combat").
- Related/duplicates: #48 (closed-candidate below). Follow-ups to file: Cover Stance → `COVER_DEFENSE` in hit resolution (+100 vs +200 magnitude needs an owner call); the cover-pose owner experiment (`findings/cover-world-placement.md` Q4).

### Action text

Closing comment (reason: completed):

> Closing as done by the NPC AI restoration campaign. NA20 (#777) found where world-space cover lives. NA21 (#780) extracted it per world: 236 nodes / 58 sets for Castle_CellBlock and 3,788 / 481 for Castle. NA22 (#790) added the behaviour. Guards spawned in cover hold it. NPCs seek the best free slot within attack range and release it when flanked, with squad spreading. `entity_templates.use_cover` gates it, and Cover Stance (ability 1451) is granted in cover. Code: `crates/services/src/cell/cover/` and `npc_ai/fight_cover.rs`. Design: `docs/architecture/cover-system.md`.
>
> Two premises in this thread turned out wrong. There is no server-to-client movement-type message (D-NA10: `0x00deb660` is the GM `onShowPath` visualiser, not an NPC-movement FSM). The `covernodes_*.pak` rows are prefab-local and were replaced by per-map extraction. Two items are still open and will get their own tickets: Cover Stance has no combat effect yet (`COVER_DEFENSE` isn't read by hit resolution, and the magnitude, +100 or +200, is undecided), and whether the client crouches at a slot is pending the owner's in-client experiment (`docs/reverse-engineering/findings/cover-world-placement.md` Q4). UAT-3 is in `docs/analysis/npc-ai-restoration/handoffs/session-resume.md`.

## #191 — test(cell/service): npc_ai_leash should pin its onStatUpdate + onStateFieldUpdate wire packets

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: remove `ready-for-human`
- Summary: The requested guard exists. It adds a player witness, drains `rx`, and asserts method 20 before method 19 to the witness. The 2026-09-19 agent comment says the same; I re-verified on 059d6038. NA12 (#785) since rewrote the leash as a walk-home, and the test still exists on main at the path below.
- Evidence: `crates/services/src/cell/service/tests/npc_ai/state_machine.rs:496` `npc_ai_leash_emits_stat_update_then_state_field_to_witnesses`; code under test moved to `crates/services/src/cell/service/npc_ai/leash/`.
- Related/duplicates: #190, #123, #181.

### Action text

Closing comment (reason: completed):

> Covered on main (re-checked at 059d6038). `npc_ai_leash_emits_stat_update_then_state_field_to_witnesses` (`crates/services/src/cell/service/tests/npc_ai/state_machine.rs:496`) adds a Castle player witness and asserts `WitnessEntityMethod` 20 (stat update) before 19 (onStateFieldUpdate). It pins presence and ordering, which is what this issue asked for.

## #190 — test(cell/service): npc_ai_fight should pin top-threat selection across multiple live targets

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: remove `ready-for-human`
- Summary: All three requested guards exist on main: the multi-target top-threat test and both NaN tests. The 2026-09-19 agent comment is accurate. Re-verified.
- Evidence: `crates/services/src/cell/service/tests/npc_ai/state_machine.rs:354` `npc_ai_fight_picks_top_threat_among_multiple_live_targets`, `:417` `npc_ai_fight_single_nan_target_does_not_panic`, `:455` `npc_ai_fight_nan_in_threat_list_with_other_targets_does_not_panic`. Target selection now lives in `npc_ai/fight_target.rs`.
- Related/duplicates: #191, #123, #181.

### Action text

Closing comment (reason: completed):

> Covered on main (re-checked at 059d6038). `crates/services/src/cell/service/tests/npc_ai/state_machine.rs` has `npc_ai_fight_picks_top_threat_among_multiple_live_targets` (three live threats, with the highest not first) plus `npc_ai_fight_single_nan_target_does_not_panic` and `npc_ai_fight_nan_in_threat_list_with_other_targets_does_not_panic`. That's all three shapes this issue asked for.

## #165 — update_bandolier_ammo TOCTOU guard uses type_id, not item_id — same-type weapon swap silently overwrites ammo

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: Fixed by #445 (option 1 of the issue). The WHERE clause now keys on `sgw_inventory.item_id` (the instance PK) via `expected_instance_id`, and `BandolierItem.item_id` is documented as the instance id. There is a live-DB guard for the same-design, different-instance swap.
- Evidence: `crates/services/src/base/world_entry/methods/inventory/ammo.rs:11-36` (`AND item_id = $5`, docstring on the old same-type-swap window); test `update_no_op_for_same_type_swap_different_instance` at `ammo.rs:260`; `crates/entity/src/cell_entity/mod.rs:107-115`; commit ebb6a4786 (#445).
- Related/duplicates: #158, #79. Out of domain for this batch (items), but it came in here.

### Action text

Closing comment (reason: completed):

> Fixed by #445. `update_bandolier_ammo` now guards on the row instance id (`WHERE … AND item_id = $5`, bound from `expected_instance_id`) instead of `type_id`, and `BandolierItem.item_id` carries the `sgw_inventory` PK (`crates/entity/src/cell_entity/mod.rs:107`). The same-design, different-instance swap is pinned by the live-DB test `update_no_op_for_same_type_swap_different_instance` (`base/world_entry/methods/inventory/ammo.rs:260`).

## #63 — Movement validation: speed checks, navmesh containment, anti-cheat

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: All four layers from the issue are live: bounds, speed, teleport and navmesh containment, plus snap-back via FORCED_POSITION with a snap-target sanity check and a correction cap. A per-world `navmesh_mode` (enforce/advisory) handles the partial-mesh false-positive risk the issue warned about. The only part deliberately held back is **speed enforcement**, which is warn-only pending tolerance calibration from telemetry. #443 already tracks that residual. The old `movement.rs:59` TODO is gone.
- Evidence:
  - `crates/entity/src/movement_validation/mod.rs:11-35` (four layers, "all four layers live"), constants at `:173-185` (`SPEED_WARN_TOLERANCE = 1.5`, `TELEPORT_JUMP_UNITS = 50`, `TELEPORT_SPEED_FACTOR = 10`), `MAX_SNAP_BACK_CORRECTIONS`.
  - `crates/services/src/cell/space_manager/client_move.rs`, `navmesh_mode.rs`; tests `space_manager/tests/movement_validation/{bounds,kinematics,navmesh,recovery,advisory,...}.rs`.
  - Commits 95e441fa (#437, "#63 PR1 of 4") and f8c3a51e (#478).
- Related/duplicates: #443 (speed-cap enforcement residual; its 2026-09-19 comment proposes re-scoping it to exactly that). Out-of-scope note: `0x02/0x04/0x05` avatar-update variants are length-parsed but still not dispatched (`base/connect_loop/encrypted/mod.rs:497-503`).

### Action text

Closing comment (reason: completed):

> Done. #437 (bounds and snap-back) and #478 (speed, teleport, navmesh containment and spaceId) put all four layers on the `AVATAR_UPDATE_EXPLICIT` path (`crates/entity/src/movement_validation/`, `cell/space_manager/client_move.rs`). Corrections snap back via FORCED_POSITION, with a snap-target sanity check and a per-entity correction cap. The partial-mesh false-positive risk is handled by the per-world `navmesh_mode` (`space_manager/navmesh_mode.rs`; Harset and Castle are advisory). Regression guards are under `cell/space_manager/tests/movement_validation/`. The one deliberate gap is that speed is warn-only until the tolerance is calibrated from SigNoz. That residual is tracked in #443, which should be re-scoped to "speed-cap enforcement".

## #62 — Respawn timer, corpse lifecycle, and NPC repopulation

- Verdict: REWRITE
- Priority: P3
- Labels: add `needs-triage` (two small owner confirmations below)
- Summary: Most of the body is done or disproved. NPC respawn is live (#423). `respawn_secs` is now seeded: Castle world 8 guards and PRUs at 120 s, Harset rows at 30 s (H13). So the body's headline claim that no seed row sets `respawn_secs` is stale. The deep dive already disproved the respawn-button cooldown, spirit/ghost mechanics and death penalty (SGW had none). Corpse-despawn is "respawn_secs is the corpse lifespan". Three real items remain. (1) `EF_ClearOnDeath` (enum value 4) is **not** wired: no Rust reference exists, and `docs/gap-analysis.md:268` wrongly says "EF_ClearOnDeath wired". (2) `TimeToAid` is still a hardcoded `30` (`death/side_effects.rs:164`). The python reference comment in the same file says 100. (3) The SGWSpawnRegion/SpawnSet population caps are unimplemented, and `docs/gap-analysis.md:436-438` still misreports them as CW/IM. The missing is-dead guard on `callForAid`/`respawn` is a real server-authority hole, but #462 (CAT-C-01/02) already tracks it. It is still unfixed on main (`player/combat/mod.rs:35-57,160-163` call `handle_respawn` without a BSF_DEAD check).
- Evidence:
  - `crates/services/src/cell/service/ticks/npc_respawn.rs`; `db/resources/Worlds/Seed/spawnlist.sql:8-40` (H13 note), rows at `:187-201` (`respawn_secs` 120 for Castle_PRU/NidGuard).
  - `crates/services/src/cell/abilities/death/side_effects.rs:117` (python used `onBeginAidWait(100, …)`), `:164` (`30i32`).
  - `grep -ri clear_on_death crates/` finds nothing; `entities/defs/enumerations.xml:1099` `EF_ClearOnDeath = 4`; `docs/gap-analysis.md:268` claims wired.
  - `grep MaxActiveSets crates/` finds nothing; `docs/gap-analysis.md:436,438` rows are wrong.
  - `crates/services/src/cell/cell_methods/player/combat/respawn.rs:77-90` (no dead check); #462 CAT-C-01/02.
- Related/duplicates: #462 (dead-guard / arbitrary respawner teleport), #233 (per-player respawner unlocks), #48.

### Action text

Comment:

> Triage 2026-09-25: I've rewritten the body to what is still true on main (059d6038). NPC respawn is live (#423) and seeded: Castle world-8 guards and PRUs at 120 s, Harset at 30 s (H13). The respawn-button cooldown, ghost mechanics and death penalty were disproved by the May deep dive, since SGW had none, so they're dropped. Three items remain: wire `EF_ClearOnDeath`, which no Rust code references, even though `docs/gap-analysis.md` claims it's wired; decide the `TimeToAid` value (hardcoded 30, python used 100); and correct the gap-analysis rows that say spawn-region population caps are implemented. The missing "is the player dead?" guard on `callForAid`/`respawn` is a security gap and is tracked in #462, not here.

#### New body

## Problem

Player and NPC death and respawn work end to end, but three pieces are still missing or misdocumented:

1. **`EF_ClearOnDeath` effects survive death.** Effects flagged `EF_ClearOnDeath` (4) should be removed when the bearer dies. No Rust code reads the flag.
2. **`TimeToAid` is a hardcoded 30 s.** It's the auto-release countdown in `onBeginAidWait`, not a button cooldown. The python reference used 100 for its only emitter (unstuck). The value has no Ghidra anchor and isn't configurable.
3. **Docs claim spawn-region population control exists.** `SGWSpawnRegion` / `SGWSpawnSet` (MaxActiveSets, CurrentPopulation, set cooldowns) have no Rust implementation, but `docs/gap-analysis.md` lists them as CW/IM.

Out of scope (decided by the 2026-05-27 evidence, SGW shipped none of these): a respawn-button cooldown, spirit/ghost form, XP-loss death penalty, and a separate corpse-despawn timer (`respawn_secs` is the corpse lifespan; NULL means permanent).

## Evidence

- `entities/defs/enumerations.xml:1099` `EF_ClearOnDeath = 4`; `grep -ri clear_on_death crates/` finds nothing; `docs/gap-analysis.md:268` says "EF_ClearOnDeath wired" (wrong).
- `crates/services/src/cell/abilities/death/side_effects.rs:164` `30i32`; `:117` cites python `onBeginAidWait(100, respawnerList)`.
- `grep MaxActiveSets crates/` finds nothing; `docs/gap-analysis.md:436,438`; `docs/gameplay/spawn-system.md` ("none are implemented").
- Already done: `ticks/npc_respawn.rs` (#423); seeds in `db/resources/Worlds/Seed/spawnlist.sql` (Castle 120 s, Harset 30 s).

## Acceptance criteria

- A player or NPC that dies with an active `EF_ClearOnDeath` effect loses it, and witnesses get the effect-removal packet. Effects without the flag survive death.
- `TimeToAid` comes from one named constant or config value, with the chosen default recorded in `docs/gameplay/death-respawn-system.md`.
- `docs/gap-analysis.md` §Spawn System rows for MaxActiveSets / population / set cooldowns read KM (not implemented), and the Clear-on-death row matches the code.
- Spawn-region population control is either filed as its own issue or explicitly left as KM.

## Test type

- Unit / effects: a flagged effect is removed on death, and an unflagged effect survives (the guard must fail if the sweep clears everything or nothing).
- Wire-format: the effect-removal packet is sent to witnesses on death.
- Unit: the `onBeginAidWait` payload carries the configured `TimeToAid`.

## Docs to update

`docs/gameplay/death-respawn-system.md`, `docs/gap-analysis.md`, `docs/architecture/abilities-and-effects-system.md` (death-clear rule); `crates/server/src/main.rs` env table if `TimeToAid` becomes an env var.

## Client impact

Free.

## Domain advisor

combat-systems-advisor (effects/death), npc-ai-spawn-advisor (spawn regions).

## Needs a human for

Owner confirmation of the `TimeToAid` default (keep 30 or use 100). Nothing else.

## #48 — NPC AI state machine (10 missing behavior states)

- Verdict: CLOSE (completed)
- Priority: P3
- Labels: no change
- Summary: All 12 Atrea `AiState` variants exist, and every non-trivial one has a handler: Investigating, Leashing, Dead+respawn, Despawning, Follow, Patrol, Wander, Submit and Error. `docs/gameplay/npc-ai.md` marks them DONE. #423 landed the expansion. The NPC AI restoration campaign (NA00-NA22, #774-#790) then fixed aggro, leash walk-home, assist, chase, grounding and cover. "Flee" isn't a state in the client enum. The `crates/game/src/mob.rs` `todo!()` stub is gone. One part of #423 was reversed: `setMovementType` was suppressed by NA10 because no server-to-client movement-type message exists (D-NA10). Separate doc drift: `docs/gameplay/npc-ai.md:626` still describes the movement-type byte broadcast as live.
- Evidence:
  - `crates/entity/src/cell_entity/mod.rs:172-185` (`AiState` 0-11); `crates/services/src/cell/service/npc_ai/{patrol,wander,investigate,follow,leash/,lifecycle/,dispatch}.rs`; `ticks/npc_respawn.rs`.
  - `docs/gameplay/npc-ai.md:32-41, 617-624`; commit 3e4c6f84 (#423 "#48 + #270"); NA work-packets status lines.
  - `ls crates/game/src/` has no `mob.rs`.
- Related/duplicates: #209, #62, #270 (closed).

### Action text

Closing comment (reason: completed):

> Done. #423 expanded `AiState` to the full 0-11 Atrea enum with Patrol, Wander, Investigating, Follow, Despawning, Submit, Error and NPC respawn (`crates/services/src/cell/service/npc_ai/`, `ticks/npc_respawn.rs`). The NPC AI restoration campaign (NA00-NA22, PRs #774-#790, released) then reworked Fighting and Leashing: faction-derived proximity aggro with radius, LoS and floor gates; same-room assist; walk-home leash with heal and reset; and cover. `docs/gameplay/npc-ai.md` lists every state as DONE. "Flee" isn't in the client enum, and the orphaned `mob.rs` stub is gone. One correction to the plan in this thread: there is no server-to-client movement-type message, so the `setMovementType` broadcast from #423 was suppressed in NA10 (D-NA10). Anything new should be a narrow ticket against the campaign's UAT findings.

## #46 — Extract collision geometry from UE3 .umap chunks and regenerate navmeshes

- Verdict: REWRITE
- Priority: P2
- Labels: add `enhancement`
- Summary: The epic's core deliverable has shipped. #683 added `crates/navmesh-extractor` (Rust, not the `upk_parser.py` path the body describes) with StaticMesh, Terrain and BSP extraction plus a tunable NavBuilder. #694 rebuilt `castle_cellblock.nav` (17 components, down from 50). #709 shipped a new `castle.nav` (world 8, advisory). The NavBuilder-in-Rust question was settled: NavBuilder stays C++ as a build tool. What remains: `harset.nav` (1,939 disconnected components, world 57 runs advisory until "GH1"), `harset_storagerm.nav` (104 components), `sgc_w1.nav` and `agnos.nav` are all still the 2012-2014 meshes. Worlds 68/69 and roughly 17 other maps have no mesh. Castle's mesh is 549 components, so it's advisory and not a containment gate. The body's pipeline description, Phase 1/1.3 decoder plans and "5 of 24" table are obsolete.
- Evidence:
  - `data/spaces/README.md` (build table: Cellblock + Castle rebuilt 2026-09-19; agnos/harset/harset_storagerm/sgc_w1 "Not rebuilt yet"); `ls data/spaces/` shows 6 meshes.
  - `crates/navmesh-extractor/src/{terrain.rs,bsp,staticmesh,nav_components,coverage,floor_probe}`; PRs #683, #694, #709 (merged 2026-09-19); #726 (header-first `.nav` load).
  - `docs/analysis/harset-rebuild/audit.md:53` (H-B9: 1,939 components, 3 of 12 spawns on-mesh, GH1); `db/resources/Worlds/Seed/worlds.sql:192` (Harset advisory); `crates/services/src/cell/space_manager/navmesh_mode.rs`.
- Related/duplicates: #784 (collision-geometry occluder, reuses the extractor), Harset GH1 (U15), CA14 (done).

### Action text

Comment:

> Triage 2026-09-25: the pipeline this epic asked for exists. #683 added `crates/navmesh-extractor` (StaticMesh, Terrain, BSP) and a tunable NavBuilder, #694 rebuilt Cellblock, and #709 shipped `castle.nav`. The Python `upk_parser.py` plan and the Terrain/Brush decoder deep dives below are superseded. I've rewritten the body as the remaining regeneration backlog: Harset first, because world 57 is advisory with 1,939 mesh islands and NPC chase freezes at island boundaries, then Harset_StorageRm, SGC_W1, Agnos and the playable maps that have no mesh. The original body and deep dive stay in the issue history.

#### New body

## Problem

Only two worlds run on navmeshes rebuilt from the client maps: Castle_CellBlock (world 12) and Castle (world 8, advisory). The rest still use 2012-2014 meshes with a 0.6 / 0.9 / 0.6 agent, or have none:

- `harset.nav` (world 57): 1,939 connected components, the largest island only 16% of polys, and 3 of 12 checked spawn/arrival points on-mesh. World 57 is seeded `navmesh_mode = 'advisory'`, and its sentries are forced `is_stationary` because chase freezes at island boundaries.
- `harset_storagerm.nav`: 104 components.
- `sgc_w1.nav`, `agnos.nav`: not rebuilt, quality unmeasured.
- Worlds 68/69 (Harset interiors) and the other cooked maps have no mesh, so pathing is straight-line and containment fails open.
- `castle.nav` has 549 components (exterior and interior disconnected), so it stays advisory.

## Evidence

- `data/spaces/README.md` build table; `crates/navmesh-extractor/` (PR #683), rebuilt meshes in PRs #694 and #709.
- `docs/analysis/harset-rebuild/audit.md` H-B9 (component counts, GH1/U15); `db/resources/Worlds/Seed/worlds.sql` Harset advisory row; `crates/services/src/cell/space_manager/navmesh_mode.rs`.
- `crates/services/src/cell/service/npc_ai/path_failure/` reports `no_mesh` / `no_path` per space. Use the SigNoz `npc_ai.path_fail` rows to rank worlds.

## Acceptance criteria

- `harset.nav` rebuilt with `crates/navmesh-extractor` + NavBuilder: the component count and the on-mesh share of seeded spawns/arrivals are recorded in `data/spaces/README.md`; the placement tripwire test `world57_placements_match_their_recorded_navmesh_verdict` is updated; the owner decides whether world 57 returns to `enforce` and whether the H13 `is_stationary` rows are reverted.
- The same for `harset_storagerm.nav`, and new meshes for worlds 68/69.
- `sgc_w1.nav` and `agnos.nav` rebuilt or measured and recorded as acceptable.
- Every mesh shipped has a README row: build date, agent, NavBuilder params, verts/polys/components.

## Test type

- Asset-independent extractor tests on synthetic fixtures (PR #683 lesson). Live-DB / placement tests that pin on-mesh verdicts for seeded spawns per rebuilt world. Negative-log check that `npc_ai.path_fail reason=no_mesh` disappears for rebuilt worlds.

## Docs to update

`data/spaces/README.md`, `crates/navmesh-extractor/README.md`, `docs/analysis/harset-rebuild/` (GH1 status), `docs/gap-analysis.md` navmesh rows.

## Client impact

Free (server data only; the colo DB and `data/spaces` ship with the next release).

## Domain advisor

movement-teleport-advisor (containment modes), npc-ai-spawn-advisor (chase/stationary), game-archaeology-specialist (map extraction edge cases).

## Needs a human for

A local client copy of the cooked maps (not in git), and in-game UAT walking each rebuilt world as a non-GM.

## Batch summary

| # | verdict | priority | one-line reason |
|---|---|---|---|
| 784 | NEEDS-OWNER | P2 | Accurate; blocked only on the per-world `.occ` data-size budget |
| 407 | CLOSE (completed) | P3 | `npc_ai.decision` span + `decision_outcome` + `space_id` + stuck detector all shipped (#410, NA00/NA02) |
| 406 | CLOSE (completed) | P3 | Server already sends ViewType 3; decoder fix landed; client-side rig resolution covered by #750/#755 |
| 330 | REWRITE | P3 | NA13 built the server-side aggression; only the unverified-index `onAggressionOverrideUpdate` wire-out remains |
| 209 | CLOSE (completed) | P3 | Cover shipped end to end by NA20-NA22 (#777, #780, #790); stance effect and pose are new narrow follow-ups |
| 191 | CLOSE (completed) | P3 | Leash witness wire-order test exists (`state_machine.rs:496`) |
| 190 | CLOSE (completed) | P3 | Top-threat + NaN tests exist (`state_machine.rs:354/417/455`) |
| 165 | CLOSE (completed) | P3 | Fixed by #445: guard keys on instance PK, same-type-swap test pinned |
| 63 | CLOSE (completed) | P3 | All four validation layers live (#437, #478); speed-enforce residual is #443 |
| 62 | REWRITE | P3 | NPC respawn live and seeded; left: EF_ClearOnDeath unwired, TimeToAid value, gap-analysis corrections |
| 48 | CLOSE (completed) | P3 | All 12 AiStates implemented (#423) plus NA campaign; mob.rs stub gone |
| 46 | REWRITE | P2 | Pipeline + Cellblock/Castle meshes shipped (#683/#694/#709); remaining is Harset/StorageRm/SGC_W1/Agnos/68/69 rebuild |
