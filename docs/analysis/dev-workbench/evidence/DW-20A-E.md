# DW-20A-E: dot travel and spawn evidence

This is the source contract for DW-20A, not proof that a workbench form or a live client has been tested. The inventory is exactly the eight `Spec` names in each of `crates/cell-console/src/cell/console/registry/commands/travel.rs` and `spawn.rs` (16 total). Command names below include the `.` prefix; the registry stores the part after it.

## Shared execution and readback rules

The cell chat gate checks server-side `access_level >= GameMaster` before `handle_console_command`; `crates/cell-console/src/cell/console/mod.rs` and `crates/cell-console/src/cell/console/dispatch.rs::handle_console_command` are the authority sources. The handler splits on whitespace, with **no quote-aware parsing**, and checks registry arity and `crates/cell-console/src/cell/console/dispatch.rs::resolve_target` before routing. A name or world with spaces is therefore not expressible as one argument through this channel. `Target::None` does not mean an existing selection is ignored: `resolve_target` passes a selected entity only if it is in the caller's space and view. `Target::Mob`/`Spawnable` require a selected entity in that space and view. `MobOrId` uses an explicit first-argument ID instead of the selection. `crates/cell-console/src/cell/console/parse.rs::{parse_i32,parse_f32,parse_bool}` give the actual scalar syntax; `parse_f32` rejects non-finite values. The palette must not treat its own preview as the authority check.

For movement verification, `server_entity_get {entity_id}` returns `entity.space_id`, `world_name`, `position`, `direction`, and `is_player` (`crates/lab-mcp/src/tools/entities.rs::entity_get`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`). Read the subject before and after, by identity and instance, not merely by name. A `null` result while a cross-space handoff is in flight is **unknown**, not failure. For a player same-space snap, `crates/cell-console/src/cell/console/travel/mod.rs::snap_in_current_space` updates the cell spatial grid and queues `CellToBaseMsg::TeleportPlayer`; `crates/base-world-entry/src/base/world_entry/teleport.rs::handle_teleport_player` sends `BASEMSG_FORCED_POSITION` (and method 116, which is unbound in the stock client according to `docs/reverse-engineering/findings/method-116-on-teleport.md`). The `wire.out.forced_position` log reports send status, but a server snapshot alone does not prove client application. For cross-space travel, `crates/cell-interactions/src/cell/space_transfer/mod.rs::execute_transfer` validates, queues `GateTravel`, then tears down the origin player; its `Transferred` return proves enqueue, **not** destination re-entry. Verify the re-entered player with a fresh `server_entity_get`/`server_sessions` sample in the destination. An NPC cannot cross spaces through this path. Travel commands do not write a spawnlist row. Same-space player snaps also attempt `sgw_player` position persistence in `crates/base-world-entry/src/base/world_entry/teleport.rs`; the cell feedback does not prove that DB write succeeded.

For spawn verification, `server_entity_get {entity_id}` gives `template_id`, `spawn_id`, `space_id`, `position`, `health_cur/max`, and `ai_state`. `server_entity_query {space_id,template_id}` can enumerate candidates, but it can be capped and cannot identify a particular newly spawned NPC without the returned entity ID. The base template source is `resources.entity_templates` (`crates/base-session/src/base/gm_spawn.rs::load_spawn_record_for_template`); a GM-created `SpawnRecord` has no spawnlist row. The only spawnlist writer here is the queue-then-confirm path in `crates/cell-console/src/cell/console/{spawn/authoring.rs,seed.rs}`. `.seedconfirm` requests a live DB write and emits SQL for a developer to merge into `db/resources/Worlds/Seed/spawnlist.sql`; a success line is not proof of a committed seed edit, and live DB failure can be reported separately. Read-only SQL can verify a confirmed row by `spawn_id` or the emitted row fields. No command in this packet edits that seed file directly.

## .gotoxyz

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/travel/mod.rs::{goto_xyz,snap_in_current_space}`; `crates/cell-console/src/cell/console/placement.rs::location` shares the snap; `docs/commands.md` Travel row.
- **Arguments:** Required `x y z`: three finite `f32` coordinates, whitespace-delimited, no quote syntax or navmesh bound.
- **Target/scope:** Optional current selection if in caller's space/view, otherwise caller; subject stays in its current loaded space.
- **Preconditions and authority:** Cell GM gate, three parsed coordinates, subject still present. No navmesh check.
- **Effects and impact:** Writes authoritative position and AoI grid, preserves facing, reseeds movement validator. Player subject queues `TeleportPlayer`/`BASEMSG_FORCED_POSITION` and moves its pet; NPC subject has no owner snap and witnesses update by AoI. Travel hook may end a duel. Player position persistence is attempted by the base snap handler; no spawnlist/seed write.
- **Verification:** `server_entity_get` before/after for subject `space_id`, `position`, `direction`; for player, inspect `wire.out.forced_position` send status and client position separately if a client observation is available. The success feedback follows the queue send; it does not prove client application or DB persistence.
- **Compensation:** A second `.gotoxyz` to the captured old position is a manual compensating move, with new side effects; it is not atomic undo.
- **Availability:** Runnable with fresh subject/space check. Unknown client-applied state remains explicit.

## .goto

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/travel/named_destination.rs::{goto,resolve_named_player}`; `crates/cell-console/src/cell/console/travel/mod.rs::move_subject`; `crates/cell-interactions/src/cell/space_transfer/mod.rs::execute_transfer`; `docs/commands.md` Travel row.
- **Arguments:** Required single online player name token; exact player lookup. Embedded spaces cannot be quoted through `split_whitespace`.
- **Target/scope:** Move selected in-view entity or caller to the named player's exact position **and exact instance**. The named player supplies destination, not subject.
- **Preconditions and authority:** Cell GM gate; unique, online, reachable named player; present subject. Ambiguous/missing/in-transition names refuse. A selected NPC may move within its own space but cross-space NPC transfer refuses.
- **Effects and impact:** Same instance uses the `.gotoxyz` snap path. Different instance/world uses player-only `GateTravel` teardown/re-entry, preserving facing; may cancel trade, duel and pet state. No spawnlist write.
- **Verification:** `server_entity_get` for named player supplies destination `space_id`/`position`; compare subject's fresh `space_id`, `world_name`, `position` after move. On cross-space, wait for subject re-entry; feedback is enqueue evidence only.
- **Compensation:** Manually move subject back to captured origin with `.gotospace <oldSpaceId> x y z` while it remains loaded; otherwise another appropriate travel command. No proven inverse for trade/duel/pet effects.
- **Availability:** Runnable, with destination name and transfer result checks; a transfer pending re-entry has unknown final outcome.

## .summon

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/travel/named_destination.rs::{summon,resolve_named_player}`; `crates/cell-console/src/cell/console/travel/mod.rs::move_subject`; `docs/commands.md` Travel row.
- **Arguments:** Required one online player name token; no quote-aware multiword name.
- **Target/scope:** The named player is the subject; destination is the caller's server-known position and exact instance. Any selected entity is ignored.
- **Preconditions and authority:** Cell GM gate; unique reachable named player, caller in a live space. Missing/in-transition/ambiguous name or missing caller entity refuses.
- **Effects and impact:** Same-space player snap or cross-space `GateTravel` and re-entry; may end duel, cancel trade and dismiss/reposition pets according to path. No spawnlist write.
- **Verification:** Capture both players' `server_entity_get` before; after, inspect named player's `space_id`, `world_name`, `position` against caller, plus forced-position send for same-space or fresh re-entry for cross-space.
- **Compensation:** Manual return to recorded origin via loaded-space/world travel; no automatic undo of associated gameplay state.
- **Availability:** Runnable; final cross-space state stays unknown until read back.

## .gotolocation

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/travel/named_destination.rs::goto_location`; `crates/cell-console/src/cell/console/travel/world_entry_point.rs::world_entry_point`; `crates/cell-console/src/cell/console/travel/mod.rs::move_subject`; `docs/commands.md` Travel row.
- **Arguments:** Required one world-name token; optional complete finite `x y z` triple (1 or 4 total args, although registry min/max is 1/4). No quoted multiword world. With only world, entry point is selected from start profile, story ring pad, stargate arrival or first authored respawner; absent entry point refuses.
- **Target/scope:** Always caller, never selection. World names canonicalize case-insensitively against the world table. In the caller's current world, retain exact current instance; another world resolves default/first loaded instance or creates one through transfer.
- **Preconditions and authority:** Cell GM gate, known world, valid entry point or coordinates, caller present. Non-finite/partial tuple refuses; no navmesh check for explicit coordinates.
- **Effects and impact:** Same-instance snap or cross-space/world `GateTravel` teardown and re-entry, with the player side effects described above. No spawnlist write.
- **Verification:** `server_entity_get` before/after for caller `space_id`, `world_name`, `position`, plus forced-position send or re-entry observation as appropriate. Entry-point feedback labels which source was used; it does not prove client application.
- **Compensation:** Manual travel to captured origin world/instance/position if still reachable; no general inverse for associated state changes.
- **Availability:** Runnable for a world resolvable by the server. A world with no known entry point requires explicit coordinates.

## .gotospace

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/travel/mod.rs::{goto_space,move_subject}`; `crates/cell-console/src/cell/console/travel/world_entry_point.rs::world_entry_point`; `crates/cell-interactions/src/cell/space_transfer/mod.rs::transfer_player_to_loaded_space`; `docs/commands.md` Travel row.
- **Arguments:** Positive `i32` loaded `spaceId`; optional complete finite `x y z` triple (1 or 4 args). No coordinate-only partial form. With ID only, resolves that instance world's entry point.
- **Target/scope:** Selected in-view entity or caller; destination is the exact loaded instance. World name is derived from the instance, so no world-table lookup is required for the by-ID path.
- **Preconditions and authority:** Cell GM gate, loaded positive instance, valid coordinates/entry point and live subject. Cross-space subject must be a player; NPC same-space move is allowed. Transfer primitive rechecks destination instance.
- **Effects and impact:** Same-instance authoritative snap or cross-instance/world `GateTravel`, teardown and re-entry, preserving facing. No spawnlist write.
- **Verification:** `server_entity_get` for subject before/after, including exact `space_id`, `world_name`, `position`; forced-position send for snap, fresh player re-entry for transfer. Enqueue feedback is not final arrival proof.
- **Compensation:** Captured origin `space_id` and coordinates can support a manual `.gotospace` return while that origin instance remains loaded; otherwise no guaranteed inverse.
- **Availability:** Runnable only while destination instance is loaded; refuse a stale ID.

## .location

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/placement.rs::{location,parse_triple,settle_npc}`; `crates/cell-console/src/cell/console/spawn/authoring.rs::save_spawn`; `docs/commands.md` Placement row.
- **Arguments:** Zero args reports; exactly finite `x y z` sets. One/two args refuse despite registry accepting 0–3 by count.
- **Target/scope:** Required selected in-view spawnable in caller's space. Set keeps subject in that space and preserves facing; unlike `.gotoxyz`, it settles a non-player as its home.
- **Preconditions and authority:** Cell GM gate, live selected target, finite complete triple for set. A player target is allowed by `Target::Spawnable` and receives a player snap.
- **Effects and impact:** Report is read-only. Set writes position/grid, reseeds movement validator and, for a player, queues `TeleportPlayer`; for an NPC, updates home and stops movement. If caller enabled `.autosavespawn`, NPC set also queues `.savespawn` SQL, still pending `.seedconfirm`.
- **Verification:** Handler itself rereads actual `position` and prints it. Independently use `server_entity_get` for `position`, `space_id`, `direction`; for player, inspect forced-position send; for NPC autosave, inspect `.seedpending`, then confirmed DB/seed separately.
- **Compensation:** Manual set to captured old coordinates; queued autosave may be discarded with `.seedcancel` before confirmation, but that does not restore runtime position/home.
- **Availability:** Runnable for report or set; surface autosave state/seed-pending impact before set.

## .rotation

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/placement.rs::{rotation,parse_triple,settle_npc}`; `docs/commands.md` Placement row.
- **Arguments:** Zero args reports; exactly finite `pitch yaw roll` `f32` radians sets. One/two args refuse. The response also converts yaw to degrees.
- **Target/scope:** Required selected in-view spawnable in caller's space, including player. No position or space change.
- **Preconditions and authority:** Cell GM gate, live target, complete finite triple for set.
- **Effects and impact:** Report is read-only. Set writes `direction`; NPC set also updates home/stops movement and, when autosave is on, queues `.savespawn`. AoI movement update carries direction; no forced-position message or spawnlist write unless later confirmed.
- **Verification:** Handler rereads and prints orientation; `server_entity_get.direction` independently verifies. `.seedpending`/confirmed DB check is needed for autosave persistence.
- **Compensation:** Manual set to captured old orientation; `.seedcancel` can remove unconfirmed SQL but does not restore runtime direction/home.
- **Availability:** Runnable; distinguish report from set and show autosave impact.

## .movehere

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/travel.rs::SPECS`; `crates/cell-console/src/cell/console/placement.rs::{move_here,settle_npc}`; `docs/commands.md` Placement row.
- **Arguments:** None.
- **Target/scope:** Required selected in-view non-player spawnable in caller's **same space**; takes caller's server-known position and yaw. Registry `Target::Mob` is broader than an NPC class check, so the handler's non-player/same-space guards are relevant.
- **Preconditions and authority:** Cell GM gate, present caller and target, target not player, same space, finite caller position/yaw. Cross-space move refuses.
- **Effects and impact:** Updates target position/grid and facing, makes both its home, stops movement; observers see AoI update. With `.autosavespawn`, queues `.savespawn` for later confirmation. No cross-world transfer.
- **Verification:** `server_entity_get` before/after for target `space_id`, `position`, `direction` against caller; feedback says whether SQL was queued. `spawn_position` is not in `LabEntitySnapshot`, so home/stop-path readback requires behavioral observation or a new endpoint; do not infer it from position alone.
- **Compensation:** Manual `.location`/`.rotation` to captured state; any queued seed change can be canceled before confirmation. No proven automatic restoration of prior AI path/home.
- **Availability:** Runnable with same-space target; mark home/AI-path verification gap explicitly.

## .spawn

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/mod.rs::{spawn_entity,caller_placement}`; `crates/base-session/src/base/gm_spawn.rs::{handle_gm_spawn_npc,load_spawn_record_for_template}`; `crates/cell/src/cell/service/base_messages/gm_spawn.rs::handle_gm_spawn_npc_ready`; `docs/commands.md` Spawn authoring row.
- **Arguments:** One positive `i32` `templateId` (cell rejects `<=0`); no quoted text.
- **Target/scope:** No selection. One NPC at caller's exact position/facing in caller's space, based on `resources.entity_templates`.
- **Preconditions and authority:** Cell GM gate, caller in a space, finite position/yaw, base template lookup and cell placement success. Unknown template, DB query or channel/placement failure does not create a confirmed NPC.
- **Effects and impact:** Sends `GmSpawnNpc` to base, then cell allocates runtime NPC ID on `GmSpawnNpcReady`; does **not** add `resources.spawnlist` row, even with `.autosavespawn` on. Other observers see new entity via AoI.
- **Verification:** Definitive completion line is `spawned npc <id> (template <n>)` from `handle_gm_spawn_npc_ready`, not the initial send. Then `server_entity_get {entity_id:id}` must match `template_id`, caller `space_id`, and expected `position`; `spawn_id` is absent. If base has no DB pool, single-spawn failure can be silent: no completion means outcome unknown, not success.
- **Compensation:** `.despawn <returnedId>` removes runtime NPC if still in caller's space; no spawnlist change to undo.
- **Availability:** Runnable with completion-ID readback; disable success affordance until cell completion arrives.

## .despawn

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/dispatch.rs::resolve_target`; `crates/cell-console/src/cell/console/spawn/despawn_by_id.rs::resolve_npc_id`; `crates/cell-console/src/cell/console/spawn/mod.rs::despawn_entity`; `crates/cell-world/src/cell/space_manager/entities.rs::despawn_npc`; `docs/commands.md` Spawn authoring row.
- **Arguments:** Zero args uses selection; optional `u32` `entityId` names an NPC without selection. An explicit ID is limited to the caller's space and NPC/being class, excluding player, pet and prop; no-ID selection can still remove a selected pet per the handler comment.
- **Target/scope:** One runtime non-player entity in caller's space. ID form need not be in view; selection form must be in view.
- **Preconditions and authority:** Cell GM gate and target resolution; `despawn_npc` rechecks player refusal. Unknown or cross-space ID gets the same `no NPC with id ... in your space` reason.
- **Effects and impact:** Releases related combat, removes runtime entity and notifies witnesses. Existing `resources.spawnlist` row remains, so a seeded NPC can return on later respawn/reload.
- **Verification:** Completion line identifies ID and witness count; `server_entity_get {entity_id}` should return `null` after removal, but pair with prior `space_id`/identity to avoid ID-reuse confusion. Query/SQL can separately show spawnlist row remained.
- **Compensation:** A new `.spawn <templateId>` creates a different runtime ID and may not restore authored state. No exact inverse; `.delspawn` is a separate persistent operation.
- **Availability:** Runnable after explicit selection/ID policy; show the stricter ID-form pet/prop refusal.

## .savespawn

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/authoring.rs::save_spawn`; `crates/cell-console/src/cell/console/seed.rs::{record_spawn,confirm}`; `crates/cell-console/src/cell/console/tests/spawn_authoring.rs::spawn_authoring_savespawn_queues_without_touching_the_db`; `docs/commands.md` Spawn authoring table.
- **Arguments:** None.
- **Target/scope:** Required selected in-view spawnable with `template_id`, non-player; its current position, yaw, tag and world become an `UPDATE` by existing `spawn_id` or `INSERT` using template/world lookup.
- **Preconditions and authority:** Cell GM gate, live target, template/world available. A newly inserted spawn already confirmed in this server session but lacking a memory `spawn_id` is refused to prevent duplicate insert.
- **Effects and impact:** Immediately changes NPC home in memory and queues one SQL/structured row in caller's authoring buffer; repeated saves replace that NPC's pending change. `.seedconfirm` asks base to write live DB and emits SQL for human seed merge. Queueing alone is not durable.
- **Verification:** `.seedpending` reports pending statement count; `server_entity_get` checks current `position`/`direction`/`spawn_id` but does not expose home. After `.seedconfirm`, read-only SQL checks the row by `spawn_id` or unique emitted fields and inspect live-write failure log; inspect committed `db/resources/Worlds/Seed/spawnlist.sql` separately for durable seed status. A new insert's generated `spawn_id` is not returned to the NPC in memory by this path.
- **Compensation:** `.seedcancel` discards unconfirmed SQL; restore prior home/position manually. Once confirmed, a compensating authoring change and seed edit are required; no automatic undo.
- **Availability:** Runnable with pending/confirmed/seed states shown separately.

## .delspawn

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/authoring.rs::del_spawn`; `crates/cell-console/src/cell/console/seed.rs::{record_spawn,confirm}`; `crates/cell-console/src/cell/console/tests/spawn_authoring.rs`; `docs/commands.md` Spawn authoring table.
- **Arguments:** None.
- **Target/scope:** Required selected in-view spawnable with a real `spawn_id` and `template_id`; SQL targets that exact `resources.spawnlist.spawn_id`.
- **Preconditions and authority:** Cell GM gate, target still exists, has spawnlist row/template. Command-spawned entity and player with no `spawn_id` refuse.
- **Effects and impact:** Queues `DELETE` SQL for `.seedconfirm`, then immediately despawns target in memory with witness/combat cleanup. Live DB delete and seed-file removal are later, distinct events. A target can disappear even if the SQL is canceled or later live write fails.
- **Verification:** Completion line and `server_entity_get {oldId}` null prove runtime removal only. `.seedpending` shows queue; after confirmation, read-only SQL by captured `spawn_id` checks live row deletion and emitted SQL/seed diff checks durable source. A null entity alone does not prove DB deletion.
- **Compensation:** Before confirm, `.seedcancel` can discard SQL but does not respawn runtime entity. After confirm, manually restore row from captured data and merge seed correction; no built-in inverse.
- **Availability:** Runnable only with captured `spawn_id` and conspicuous runtime-now/DB-later impact.

## .autosavespawn

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/authoring.rs::autosave`; `crates/cell-console/src/cell/console/placement.rs::settle_npc`; `crates/cell-console/src/cell/console/tests/spawn_authoring.rs::spawn_authoring_autosave_queues_a_savespawn_after_movehere`; `docs/commands.md` Spawn authoring row.
- **Arguments:** One boolean token: `1`, `0`, `true`, `false` (case-insensitive). Registry accepts one argument, parser rejects other values.
- **Target/scope:** Caller-specific in-memory preference (`space_mgr.autosave_spawns`), no selection. It applies to later `.movehere`, `.lookat`, `.location`, `.rotation` on non-player targets; not `.spawn` or `.spawnrandom`.
- **Preconditions and authority:** Cell GM gate and valid boolean.
- **Effects and impact:** Inserts/removes caller ID from autosave set; future placement queues `.savespawn` SQL, still requiring `.seedconfirm`. Toggle itself writes no DB/seed row.
- **Verification:** Feedback says on/off. There is no existing read-only lab field for `autosave_spawns`; a safe behavioral check is a later deliberate NPC placement plus `.seedpending`, but this mutates state. Until a read endpoint exists, independent preference readback is unavailable.
- **Compensation:** Reissue with the prior boolean; this does not cancel SQL already queued while on.
- **Availability:** Runnable, but label preference readback as feedback-only/unknown independently.

## .respawnall

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/mod.rs::respawn_all`; `docs/commands.md` Spawn authoring row.
- **Arguments:** None.
- **Target/scope:** Every NPC returned by `all_npc_entity_ids()` whose `space_id` matches caller's space; not a selected target or another space.
- **Preconditions and authority:** Cell GM gate and caller in a space.
- **Effects and impact:** Releases players from combat with each NPC, restores its `spawn_position` when present, fills health, clears threat/respawn timer and sets AI Idle. Runtime-only; no spawnlist or seed write. It directly sets `e.position` rather than calling `update_position_preserving_facing`; source does not establish immediate spatial-grid/AoI refresh for moved NPCs, so visible position is a gap.
- **Verification:** Feedback gives reset count. Capture `server_entity_query {space_id}` before/after, then `server_entity_get` for each prior NPC ID to compare `position`, `health_cur/max`, `ai_state`; snapshot lacks threat/home/respawn-timer fields, so those cannot be independently read back through existing lab endpoint. Query cap can prevent full enumeration.
- **Compensation:** None proven; restoring health, threat, AI and combat links exactly is not available.
- **Availability:** Runnable as a high-impact whole-space reset; display incomplete field/visual readback and require operator confirmation in the later palette.

## .spawnset

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/sets.rs::{spawn_set,list,clear}`; `crates/cell-content/src/cell/content/spawn_sets.rs::{show_spawn_set,hide_spawn_set,SpawnSetSwitch}`; `crates/cell-world/src/cell/space_manager/spawn_sets.rs`; `crates/cell-console/src/cell/console/gm/tests/spawn_sets.rs::spawn_set_dot_console_on_list_clear`; `docs/commands.md` Spawn sets row.
- **Arguments:** Zero args defaults to `list`. Explicit `list`, `clear`, `on <setId>`, `off <setId>`; `setId` is `i32`. Registry allows 0–2 args, but handler branches decide shape; e.g. an extra token after `list`/`clear` is ignored by those branches, so typed form must emit only the canonical shape.
- **Target/scope:** `list` reports all loaded sets; `on`/`off` act on named set's world for everyone there, and `on` first clears active peers of the same kind/world. `clear` hides active sets in caller's world. Selection ignored.
- **Preconditions and authority:** Cell GM gate. `on` requires a known nonempty set whose world has running space; unknown/already-on/off cases return explicit `SpawnSetSwitch` outcomes. `clear` with none active is a no-op.
- **Effects and impact:** Runtime activation spawns set members; deactivation despawns through combat/witness cleanup. Shared-world effect, no DB or seed write. `list` is read-only; `on`/`off`/`clear` can change many entities.
- **Verification:** Handler's `list` feedback gives set IDs, on/off state and live count; `SpawnSetSwitch::line` describes one switch result. `server_entity_query {space_id}` can inspect members only when their tags/templates are known, but `LabEntitySnapshot` has no spawn-set ID field. There is no existing structured read-only set-state tool, so full independent membership proof is a gap.
- **Compensation:** `off <setId>` reverses an isolated `on`; restoring peers cleared by `on` needs their prior IDs/state and separate `on` actions, which can clear one another. No general atomic inverse.
- **Availability:** Runnable with `list` read-only and mutation actions marked shared-world/high impact; independent set-state readback gap remains explicit.

## .spawnrandom

- **Sources:** `crates/cell-console/src/cell/console/registry/commands/spawn.rs::SPECS`; `crates/cell-console/src/cell/console/spawn/mod.rs::spawn_random`; `crates/cell-console/src/cell/console/spawn/batch.rs::{SpawnBatches,deliver}`; `crates/base-session/src/base/gm_spawn.rs::handle_gm_spawn_npc`; `crates/cell/src/cell/service/base_messages/gm_spawn.rs::{handle_gm_spawn_npc_ready,late_copy}`; `crates/cell-console/src/cell/console/tests/ch13_spawn_batch.rs`; `docs/commands.md` Spawn authoring row.
- **Arguments:** Required `templateId` parsed as `i32`, finite `xRange`/`zRange` as `f32`, optional `count` parsed as `i32` then clamped to 1–50. **Current handler quirk:** malformed count silently defaults to 1; negative ranges are accepted, and arithmetic can overflow finite inputs into non-finite output positions. A typed palette must reject malformed count and non-finite derived ring coordinates before dispatch, while DW-20A should add server-side bounds/refusal for the same conditions.
- **Target/scope:** No selection; 1–50 runtime copies of `resources.entity_templates` template, placed on a deterministic x/z ring around caller in caller's space, sharing caller yaw.
- **Preconditions and authority:** Cell GM gate and caller placement; template existence checked by base for each copy. Unlike `.spawn`, handler does not reject non-positive template ID before enqueue. Cell/base channel failures can leave a partial batch.
- **Effects and impact:** Queues one `GmSpawnNpc` per copy; runtime-only, no `resources.spawnlist` write. Immediate `requested` feedback is **submission only**. Cell receives each real ID, and batch summary reports spawned/failed/timed-out copies; late lab copies are despawned, late in-world copies can remain with a later line.
- **Verification:** Wait for final `spawnrandom: spawned n/count ... ids ...` summary, then `server_entity_get` every returned ID for `template_id`, `space_id`, `position` and absent `spawn_id`. Timed-out copies have unknown outcome at summary time; query by template/space is only a candidate list. A missing final summary is not success.
- **Compensation:** `.despawn <id>` each returned live NPC in same space; unknown/timed-out or late copies need follow-up discovery. No exact batch undo.
- **Availability:** **Disabled pending DW-20A server-side validation** of count, template ID, ranges and derived positions; a UI-only guard would leave another submission path unsafe. The current command's observed behavior remains documented above.
