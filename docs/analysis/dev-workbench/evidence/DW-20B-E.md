# DW-20B-E: dot patrol and entity authoring evidence

This is the source contract for DW-20B, not live-client acceptance. The inventory is the 11 `Spec` names in `registry/commands/patrol.rs` plus the 17 in `registry/commands/entity_authoring.rs` (28 total). Headings use the typed `.` prefix; the registry omits it. A command marked disabled below still exists in the raw GM console. Disabled means its proposed white-glove palette action must wait for the stated server-side guard or readback.

## Shared authority, syntax, and durability

`crates/cell-console/src/cell/console/mod.rs` documents the chat gate: server-side `access_level >= GameMaster`. `dispatch.rs::handle_console_command` splits on whitespace, checks registry arity and `resolve_target`, logs accepted names, then routes. There is no quote-aware parser. `Target::None` does not require a selection. `Mob` requires a same-space, in-view, non-player selection, but `registry/mod.rs::TargetPolicy::matches` does **not** prove an NPC spawn class: pets and props may pass. `Being` and `Spawnable` both currently match any entity; the latter is not a persistence or NPC guarantee. Recheck the subject at execution, not merely when drawing a form. `parse.rs` parses `i32`, finite `f32`, and boolean `0|1|true|false`. All commands below inherit this GM and target gate; command-specific refusals are stated per row.

`patrol.rs` uses `seed.rs::record`, which writes an authoring log and per-GM queue, not the DB. The `SpaceManager.patrol_authoring` waypoint buffer is keyed only by path ID and shared by GMs in that cell process; it is not a per-GM buffer. `.seedcancel` discards only that GM's SQL queue, not an in-memory edit or earlier on-disk log line. `.seedconfirm` removes the queue, sends each `ExecuteAuthoringSql` to base, and emits a batch to the `authoring` log/session file. Its immediate “saved” feedback proves only channel enqueue; `crates/base-session/src/base/console_authoring/mod.rs::handle_execute_authoring_sql` sends a later result distinguishing no pool, SQL error, zero rows, and positive `rows_affected`. A positive result is a **live DB** edit, wiped when the DB is rebuilt from `db/resources/`; emitted SQL is a developer merge artifact, never a committed seed. A zero-row `UPDATE` can still have a queued and emitted statement. No handler here directly edits a seed file. The only DB tables in this family are `resources.point_sets`, `resources.point_set_points`, and `resources.spawnlist`, after confirmation. `crates/cell-console/src/cell/console/tests/mod.rs::{path_add_buffers_and_records,path_clear_records_single_statements,negative_waypoint_index_is_rejected}` guard a positive queue path and two refusal/shape cases; `tests/spawn_authoring.rs` guards queue versus confirm and the `.lookat`/autosave seam. `docs/commands.md` and `docs/guides/placing-npcs-and-objects.md` describe the operator flow.

For readback, `server_entity_get {entity_id}` returns the live cell snapshot (`crates/lab-mcp/src/tools/entities.rs::entity_get`; `crates/wire/src/cell/messages/lab.rs::LabEntitySnapshot`). It includes identity, space, position, direction, name, alignment, tag, name ID, AI state, state field, interaction flags and component count, but not the full patrol waypoints, dialog list, mesh/body strings, event-set ID or appearance applied by the client. `.info` (`console/query.rs::info`) exposes some additional live fields. `server_db_query` may read exact `resources.*` rows after base feedback; compare a before/after exact key, not just an authoring status line. An absent snapshot or DB result is unknown. No live lab/client was used for this packet.

## Patrol commands

### .path_add

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::{path_add,parse_path_id}`; `seed.rs::record`; `tests/mod.rs::path_add_buffers_and_records`.
- **Arguments:** One signed `i32` `pathId`; no positive-ID bound in the handler. One whitespace token.
- **Target/scope:** Caller position and current world; no selected target. Path ID is global `point_sets.set_id`.
- **Preconditions and authority:** GM; caller entity and world must resolve. First session waypoint requires a world; no DB collision/world ownership check before recording.
- **Effects and impact:** Appends position to `patrol_authoring[pathId]` in memory. First point queues `point_sets` `INSERT ... ON CONFLICT DO NOTHING`, then every point queues a `point_set_points` `INSERT`. No live DB until `.seedconfirm`; neither confirmation nor emitted SQL commits either seed file.
- **Verification:** `.path_show <pathId>` shows the session buffer; `.seedpending` counts queued statements. After positive base write feedback, read `resources.point_sets` by `set_id` and ordered `resources.point_set_points` by `set_id`. Zero-row header result may mean an existing header, not correct world ownership.
- **Compensation:** `.path_clear` clears buffer and queues DELETEs, including preexisting DB rows; this is destructive, not undo. Before confirm, `.seedcancel` drops queued SQL but leaves the buffer.
- **Availability:** Disabled in typed palette until path-ID collision/world ownership is checked server-side. Raw console handler exists.

### .path_show

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_show`.
- **Arguments:** One signed `i32` `pathId`; one token.
- **Target/scope:** Cell process's in-memory path buffer keyed by path ID, shared across GMs; no selected target.
- **Preconditions and authority:** GM; parseable ID. Empty or unknown buffer gets explicit “no waypoints authored this session.”
- **Effects and impact:** Read-only feedback, no DB read, queue, or seed SQL; it can reveal another GM's unconfirmed path points with the same ID.
- **Verification:** The returned indexed coordinate lines are the readback; they do not describe a preexisting DB path.
- **Compensation:** None needed.
- **Availability:** Runnable, labelled “session buffer only.”

### .path_clear

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_clear`; `tests/mod.rs::path_clear_records_single_statements`.
- **Arguments:** One signed `i32` `pathId`.
- **Target/scope:** Global path ID and shared cell-process buffer, no selected target.
- **Preconditions and authority:** GM and parseable ID; no ownership or existence check.
- **Effects and impact:** Removes the session buffer immediately; queues two separate SQL DELETEs, child `point_set_points` then parent `point_sets`. `.seedconfirm` may delete live DB rows; emitted DELETEs remain only proposed seed edits until merged. Existing NPC patrol lists are not cleared by this handler.
- **Verification:** `.path_show` proves buffer removal. After confirm, inspect each base result and `server_db_query` counts by exact `set_id` for both tables; queued status alone proves no deletion.
- **Compensation:** Recreate the exact captured header/points manually; `.path_add` cannot restore metadata, ordering, sequences and teleport fields. No automatic inverse.
- **Availability:** Disabled until a server-side ownership/existence precondition and explicit destructive preview exist.

### .path_assign

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_assign`; `npc_ai::set_ai_state_on`.
- **Arguments:** Required signed `i32` `pathId`, optional finite `f32` `delay` in seconds (default `3.0`); no nonnegative bound.
- **Target/scope:** Selected same-space/in-view non-player. Registry `Mob` also admits non-NPC entities.
- **Preconditions and authority:** GM; selected target; at least one session-authored waypoint. Bad delay and empty path refuse before mutation; no world match or actual NPC class check.
- **Effects and impact:** Copies session points into target `patrol_path`, sets delay/index and AI state `Patrol` immediately. If `spawn_id` exists, queues `UPDATE resources.spawnlist SET patrol_path_id, patrol_point_delay WHERE spawn_id = ...`; without it, memory only. Live DB needs confirm and positive row result; seed SQL remains uncommitted.
- **Verification:** `server_entity_get` AI state and `spawn_id` before/after; it omits patrol path/delay, so full in-memory readback is unavailable. After confirm, query the exact `spawn_id` override columns and compare; verify `point_set_points` separately. AI state alone is insufficient.
- **Compensation:** `.path_unassign` stops the live patrol and queues NULL override, losing the earlier assignment; capture old values for manual restoration.
- **Availability:** Disabled until target class, path world, delay bounds and exact patrol readback are enforced.

### .path_unassign

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_unassign`.
- **Arguments:** None.
- **Target/scope:** Selected same-space/in-view non-player; `Mob` is broader than NPC.
- **Preconditions and authority:** GM and target selection; no actual NPC class check.
- **Effects and impact:** Clears target's in-memory patrol list/index, sets AI `Idle`; if a `spawn_id` exists, queues a `spawnlist` UPDATE setting both override columns NULL. Confirm may change live DB, never the committed seed.
- **Verification:** `server_entity_get.ai_state`/`spawn_id`, then exact `spawnlist` override columns after positive base result. Snapshot lacks the full path list; an already-idle target can make the apparent change a no-op.
- **Compensation:** Manually restore captured path/delay and AI state; `.path_assign` requires session points and cannot reconstruct a DB-only path automatically.
- **Availability:** Disabled until true NPC class and before/after patrol readback are available.

### .path_set_seq

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::{waypoint_set,parse_index}`; `tests/mod.rs::negative_waypoint_index_is_rejected` (shared index guard).
- **Arguments:** Signed `i32` `pathId`, nonnegative `i32` zero-based `index`, signed `i32` `seqId`; exactly three tokens.
- **Target/scope:** Nth DB waypoint ordered by `point_id`; no selected target.
- **Preconditions and authority:** GM; parsable values; negative index refuses. Missing index/path is not checked before queue.
- **Effects and impact:** Queues `UPDATE point_set_points SET sequence_id = seqId` for the offset row. No immediate patrol-memory change; confirm can write live DB, emitted SQL is not a seed commit.
- **Verification:** Positive base row feedback then `server_db_query` ordered `point_id` row's `sequence_id`; zero rows means no live change.
- **Compensation:** Queue old captured sequence ID manually, or `.path_clear_seq` if old value was NULL; no automatic inverse.
- **Availability:** Disabled until path ownership and selected row existence are checked before queuing.

### .path_clear_seq

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::waypoint_set`.
- **Arguments:** Signed `i32` `pathId`, nonnegative zero-based `i32` `index`; exactly two tokens.
- **Target/scope:** Nth ordered DB waypoint, no selected target.
- **Preconditions and authority:** GM; valid index syntax; no row/world ownership check.
- **Effects and impact:** Queues `sequence_id = NULL`; no immediate memory edit. Confirm can change live DB; SQL is emitted, not committed to seed.
- **Verification:** Exact `point_id`/`sequence_id` after positive base result; zero rows is no-op.
- **Compensation:** Restore captured sequence ID manually with `.path_set_seq`.
- **Availability:** Disabled until path/row ownership and existence are checked.

### .path_set_tp

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_set_tp`; `tests/mod.rs::negative_waypoint_index_is_rejected`.
- **Arguments:** Signed `i32` `pathId`, nonnegative zero-based `i32` `index`.
- **Target/scope:** Nth ordered DB waypoint; destination is caller's current position, not a selected target.
- **Preconditions and authority:** GM; caller entity, valid index; no waypoint/world match check.
- **Effects and impact:** Queues UPDATE of `teleport_x/y/z`; no immediate runtime path mutation or DB write. Confirm may write live DB; seed file remains unchanged.
- **Verification:** Capture caller position and exact `point_id`; after positive base result, compare `teleport_x/y/z` via `server_db_query`. No client teleport occurs from this authoring command itself.
- **Compensation:** `.path_clear_tp` also clears arrival sequence/delay, so restore all captured fields manually if needed.
- **Availability:** Disabled until row/world ownership and destination validity are checked.

### .path_clear_tp

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_clear_tp`.
- **Arguments:** Signed `i32` `pathId`, nonnegative zero-based `i32` `index`.
- **Target/scope:** Nth ordered DB waypoint; no selected target.
- **Preconditions and authority:** GM; valid index syntax; no row existence check.
- **Effects and impact:** Queues one UPDATE clearing `teleport_x/y/z`, `teleport_sequence_id`, and `teleport_delay`; no immediate memory effect. Confirm can change live DB; SQL remains uncommitted seed proposal.
- **Verification:** Exact ordered `point_id` and all five NULL columns after positive base result; zero rows is no-op.
- **Compensation:** Restore all five captured fields manually; `.path_set_tp` alone is incomplete.
- **Availability:** Disabled until exact affected row and all five old values can be previewed.

### .path_set_tp_seq

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::waypoint_set`.
- **Arguments:** Signed `i32` `pathId`, nonnegative zero-based `i32` `index`, signed `i32` `seqId`.
- **Target/scope:** Nth ordered DB waypoint; no selected target.
- **Preconditions and authority:** GM; parsed args; no row/world check or teleport-destination prerequisite.
- **Effects and impact:** Queues `teleport_sequence_id = seqId`; no immediate memory edit. Confirm may update live DB; emitted statement is not a seed commit.
- **Verification:** Exact `point_id`/`teleport_sequence_id` via `server_db_query` after positive base result; zero rows is no-op.
- **Compensation:** Restore captured value manually; no dedicated clear command except `.path_clear_tp`, which clears other fields too.
- **Availability:** Disabled until affected row and prior value are read and checked.

### .path_set_tp_delay

- **Sources:** `registry/commands/patrol.rs::SPECS`; `patrol.rs::path_set_tp_delay`; `parse.rs::parse_f32`.
- **Arguments:** Signed `i32` `pathId`, nonnegative zero-based `i32` `index`, finite `f32` `delay`; no nonnegative bound.
- **Target/scope:** Nth ordered DB waypoint; no selected target.
- **Preconditions and authority:** GM; finite delay and nonnegative index; no row/world check or teleport-destination prerequisite.
- **Effects and impact:** Queues `teleport_delay = delay`; no immediate memory edit. Confirm may write live DB; emitted SQL is uncommitted.
- **Verification:** Exact `point_id`/`teleport_delay` via `server_db_query` after positive base result; zero rows is no-op.
- **Compensation:** Restore captured old value manually; clearing with `.path_clear_tp` also destroys destination/sequence fields.
- **Availability:** Disabled until affected row exists and delay bounds/old value are checked.

## Entity authoring commands

All entity entries below are immediate cell-memory edits unless explicitly stated. They do not call `seed::record` or `ExecuteAuthoringSql`, and they emit no seed SQL. `spawn/authoring.rs::save_spawn` persists only the spawnlist placement/heading/tag fields; it is not a generic save for the fields below. `placement.rs::settle_npc` is the exception: `.lookat` on **any non-player spawnable**, including a prop or pet admitted by `Target::Spawnable`, also changes home facing and stops movement; if `.autosavespawn` is enabled it calls `.savespawn`. That save may refuse for missing template/world or an already confirmed new spawn, or may queue placement SQL even for an inappropriate non-player class. A queued record still needs confirmation, positive DB feedback, and developer seed merge. `tests/spawn_authoring.rs::{spawn_authoring_lookat_writes_yaw_not_a_vector,spawn_authoring_lookat_then_savespawn_saves_the_heading,spawn_authoring_autosave_queues_a_savespawn_after_movehere}` prove the NPC positive seam, not behavior for every non-player class or a live-client appearance. For other entity commands, a successful feedback line does not establish persistence or client rendering.

### .tag

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_tag`; `spawn/authoring.rs::save_spawn` for separate persistence.
- **Arguments:** One token `tag`; exact lowercase `none` clears; no quoting/space or length bound.
- **Target/scope:** Selected same-space/in-view `Spawnable` (any entity).
- **Preconditions and authority:** GM/selection; no tag namespace or target class validation.
- **Effects and impact:** Sets `CellEntity.tag` to a string or None immediately. No SQL/DB/seed action here; only a later explicit `.savespawn` can queue a spawnlist tag for a valid spawn target.
- **Verification:** `server_entity_get.tag` before/after by entity ID; for separate save, exact `spawnlist.tag` after confirm/positive base feedback.
- **Compensation:** Reapply captured old tag or `none`; no atomic undo.
- **Availability:** Runnable as transient edit with fresh target check; persistent-save claim disabled without separate `.savespawn` proof.

### .name

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_name`.
- **Arguments:** One or more whitespace tokens joined with single spaces; no quote preservation or length bound.
- **Target/scope:** Selected same-space/in-view `Being` (registry currently matches any entity).
- **Preconditions and authority:** GM/selection; no NPC-only check despite handler help saying NPC.
- **Effects and impact:** Sets `npc_name` in memory even for a player target; may appear on an NPC's next AoI entry. No DB/SQL/seed write; `.savespawn` does not store this name.
- **Verification:** For an NPC, compare `server_entity_get.name`/`entity_name` before/after. `crates/cell-world/src/cell/space_manager/lab_snapshots.rs::{snapshot_entity,live_entity_name}` prefers `character_name`, so a player's snapshot name/entity_name does not show the `npc_name` edit; player readback is unavailable. Client display on AoI entry remains unverified.
- **Compensation:** Reapply captured prior name if present; no clear command for a prior None value.
- **Availability:** Disabled for typed palette until NPC class and clear/length semantics are defined.

### .alignment

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_alignment`; `console/query.rs::info`.
- **Arguments:** One case-insensitive enum token `undefined|praxis|sgu` mapped to `0|1|2`.
- **Target/scope:** Selected same-space/in-view `Being` (currently any entity).
- **Preconditions and authority:** GM/selection; invalid enum refuses without mutation.
- **Effects and impact:** Replaces in-memory `alignment`; no DB/SQL/seed write.
- **Verification:** `server_entity_get.alignment` or `.info` before/after; client display is separate.
- **Compensation:** Reapply captured enum if value was 0–2; an unrecognized old numeric value cannot be restored through this command.
- **Availability:** Runnable only when old value is representable and subject is a verified being; otherwise disabled for no safe compensation.

### .nameid

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_i32_field`.
- **Arguments:** One signed `i32` `nameId` token; no resource existence check.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection and parseable number.
- **Effects and impact:** Sets in-memory `name_id = Some(value)`; no DB/SQL/seed write, and `.savespawn` does not persist it.
- **Verification:** `server_entity_get.name_id` or `.info` before/after; localized client text unverified.
- **Compensation:** Reapply captured numeric ID; no command clears to None.
- **Availability:** Disabled until name resource lookup/clear semantics are available.

### .staticmesh

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_str_field`; `console/query.rs::info`.
- **Arguments:** One string token `mesh`; spaces cannot be quoted, no asset existence check.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection; no mesh validation.
- **Effects and impact:** Sets in-memory `static_mesh = Some(token)`; no DB/SQL/seed write or immediate NPC appearance broadcast.
- **Verification:** `.info` reports static mesh; `server_entity_get.has_static_mesh` is only a boolean, not the string. Client render awaits AoI entry and is unverified.
- **Compensation:** Reapply old mesh token if expressible; no clear-to-None command.
- **Availability:** Disabled until asset lookup and safe clear/readback are defined.

### .bodyset

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_str_field`; `console/query.rs::info`.
- **Arguments:** One string token `bodySet`; no quoting or resource validation.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection.
- **Effects and impact:** Sets in-memory `body_set = Some(token)`; no DB/SQL/seed write or immediate NPC appearance broadcast.
- **Verification:** `.info` reports body set; `server_entity_get` omits the string. Client AoI appearance is not proved.
- **Compensation:** Reapply captured old token if expressible; no clear-to-None command.
- **Availability:** Disabled until asset lookup and clear/readback are defined.

### .eventset

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_i32_field`; `console/query.rs::info`.
- **Arguments:** One signed `i32` `eventSetId`; no seeded-key validation.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection and parseable number.
- **Effects and impact:** Sets in-memory `event_set_id = Some(value)`; no DB/SQL/seed write. A later content event may use it, but this handler does not execute one.
- **Verification:** `.info` reports nonzero event set; `server_entity_get` omits it, so zero versus None needs a new readback. Event execution is unverified.
- **Compensation:** Reapply old nonzero ID if known; no clear-to-None command.
- **Availability:** Disabled until seeded ID lookup and complete readback exist.

### .interactiontype

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_interaction_type`; `console/query.rs::info`.
- **Arguments:** One signed `i32` raw flags token, stored as `i64`; no mask validation.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection and parseable number.
- **Effects and impact:** Replaces in-memory `interaction_type_flags`; no DB/SQL/seed write or forced witness refresh.
- **Verification:** `server_entity_get.interaction_type_flags` before/after; actual client prompt behavior remains unverified.
- **Compensation:** Reapply captured flags if in signed `i32` range; wider old values cannot be restored here.
- **Availability:** Disabled until valid flag mask and readback/client semantics are reviewed.

### .lookat

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::{look_at,yaw_towards}`; `placement.rs::settle_npc`; `tests/spawn_authoring.rs::{spawn_authoring_lookat_writes_yaw_not_a_vector,spawn_authoring_lookat_then_savespawn_saves_the_heading}`.
- **Arguments:** None.
- **Target/scope:** Selected same-space/in-view `Spawnable`; yaw toward caller's current x/z position.
- **Preconditions and authority:** GM/selection/caller position. Coincident x/z refuses with feedback and no facing change.
- **Effects and impact:** Sets in-memory direction `[0,yaw,0]`. `settle_npc` gives **every non-player target** home facing and stops movement; `.autosavespawn` then calls `.savespawn`, which may refuse or queue spawnlist heading/placement SQL depending on template/world/confirmed-spawn state. With autosave off, no SQL/DB/seed action. A queue still needs confirm, positive base feedback, and seed merge.
- **Verification:** `server_entity_get.direction` before/after; for any non-player target with autosave on, inspect refusal or `.seedpending`, then exact spawnlist heading only after positive base result. Client facing/witness application is unverified.
- **Compensation:** `.rotation` with captured pitch/yaw/roll is a manual compensating action; it may itself autosave a non-player target and cannot undo stopped movement.
- **Availability:** Runnable for verified NPC with explicit autosave state and old facing captured; player, prop, pet and other unverified classes disabled pending wire and side-effect proof.

### .visible

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_visible`; `parse.rs::parse_bool`.
- **Arguments:** One boolean `0|1|false|true` token.
- **Target/scope:** Selected same-space/in-view `Spawnable` and its current witnesses.
- **Preconditions and authority:** GM/selection; malformed bool refuses.
- **Effects and impact:** False sends `CellToBaseMsg::EntityInvisible` to each current witness; it does not set a stored visibility flag. True only sends feedback “re-appears on next AoI refresh” and changes nothing. No DB/SQL/seed write.
- **Verification:** `server_witnesses` can identify current recipients, but neither `server_entity_get` nor this handler proves each client's hidden/shown state; true is a no-op. Observe per-client AoI to verify actual rendering.
- **Compensation:** No proven inverse; `visible true` does not actively show the entity.
- **Availability:** Disabled until a real show/readback path exists; must not label true a completed show.

### .setcombatant

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_combatant`.
- **Arguments:** One `i32` bit index `0..31`; outside range refuses.
- **Target/scope:** Selected same-space/in-view `Being` (currently any entity).
- **Preconditions and authority:** GM/selection and bounded bit.
- **Effects and impact:** ORs `1 << bit` into in-memory `state_field`; no DB/SQL/seed write. Does not update state-field ownership/refcounts in this handler.
- **Verification:** `server_entity_get.state_field` before/after; combat behavior and client flag display are not proved by that snapshot.
- **Compensation:** `.unsetcombatant <bit>` clears the bit, but may erase a state held by another system; not a safe automatic inverse.
- **Availability:** Disabled until state ownership/refcount rules are enforced.

### .unsetcombatant

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::set_combatant`.
- **Arguments:** One `i32` bit index `0..31`.
- **Target/scope:** Selected same-space/in-view `Being` (currently any entity).
- **Preconditions and authority:** GM/selection and bounded bit; out-of-range refuses.
- **Effects and impact:** AND-clears the bit in in-memory `state_field`; no DB/SQL/seed write, and no refcount reconciliation.
- **Verification:** `server_entity_get.state_field` before/after; downstream state-owner effects remain unknown.
- **Compensation:** `.setcombatant <bit>` may restore a bit but not its owner/refcount; no safe inverse.
- **Availability:** Disabled until state ownership/refcount rules are enforced.

### .addcomponent

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::component`.
- **Arguments:** One component string token; no quote/asset lookup. Duplicate is an in-memory no-op but feedback still says add.
- **Target/scope:** Selected same-space/in-view `Being` (currently any entity).
- **Preconditions and authority:** GM/selection; no component validity check.
- **Effects and impact:** Pushes component into in-memory list if absent. For player with `player_id`, sends `RefreshAppearance`; NPC relies on next AoI entry. No DB/SQL/seed write.
- **Verification:** `server_entity_get.component_count` only gives count, not identity; client `BeingAppearance` application is not proved. Need full component readback before claiming exact effect.
- **Compensation:** `.delcomponent` is manual and may remove a component installed by another system; no automatic inverse.
- **Availability:** Disabled until component lookup and exact list readback exist.

### .delcomponent

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::component`.
- **Arguments:** One component string token; no quoting.
- **Target/scope:** Selected same-space/in-view `Being` (currently any entity).
- **Preconditions and authority:** GM/selection; absent component is a no-op with success-shaped feedback.
- **Effects and impact:** Retains every in-memory component except matching string. Player with `player_id` gets `RefreshAppearance`; NPC waits for AoI entry. No DB/SQL/seed write.
- **Verification:** Component count alone cannot prove which item was removed; exact list/client appearance readback is missing.
- **Compensation:** `.addcomponent` can re-add a captured component but not restore original ordering or duplicate state.
- **Availability:** Disabled until exact list readback and component ownership are known.

### .adddialog

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::dialog`.
- **Arguments:** Two signed `i32` tokens, `templateId setMapId`; no template-ID validity check.
- **Target/scope:** Selected same-space/in-view `Spawnable` (any entity), although handler comment describes a player.
- **Preconditions and authority:** GM/selection; `setMapId` must exist in loaded `dialog_set_maps`, else refusal. It may map to no dialog ID (interaction-only row), which feedback says explicitly.
- **Effects and impact:** Inserts `(setMapId,dialogId,interactionFlags)` into the in-memory `available_interactions[templateId]` list if absent. No DB/SQL/seed write or explicit client dialog refresh.
- **Verification:** Feedback exposes resolved dialog ID, but `server_entity_get` omits the interaction list; no exact post-state readback or client prompt proof.
- **Compensation:** `.removedialog` removes by set-map ID in that template bucket; manual only and may erase an existing tuple.
- **Availability:** Disabled until target class, template relation and exact interaction-list readback are proven.

### .removedialog

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::dialog`.
- **Arguments:** Two signed `i32` tokens, `templateId setMapId`.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection; `setMapId` must still resolve in `dialog_set_maps` even to remove an old tuple. Absent tuple still produces success-shaped feedback.
- **Effects and impact:** Removes matching set-map tuples from in-memory `available_interactions[templateId]`; no DB/SQL/seed write or explicit client refresh.
- **Verification:** Feedback is insufficient; snapshot omits list, so exact readback is unavailable.
- **Compensation:** `.adddialog` reconstructs the tuple from the **current** map, which may differ from the removed one; no proven inverse.
- **Availability:** Disabled until exact list readback and map-change semantics are proven.

### .dynamicupdate

- **Sources:** `registry/commands/entity_authoring.rs::SPECS`; `entity.rs::dynamic_update`.
- **Arguments:** None.
- **Target/scope:** Selected same-space/in-view `Spawnable`.
- **Preconditions and authority:** GM/selection. Player needs a `player_id` for a `RefreshAppearance` message; NPC sends none.
- **Effects and impact:** Player queues `RefreshAppearance`; NPC only gets success-shaped feedback and waits for future AoI entry. No cell field, DB, SQL or seed mutation.
- **Verification:** A queued message is not client application; use packet/client witness evidence for a player. No immediate NPC readback exists.
- **Compensation:** None; it is a rebroadcast request.
- **Availability:** Player request can be offered only as “refresh requested,” outcome unverified; NPC action disabled because handler sends no refresh.
