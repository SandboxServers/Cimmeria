---
title: "Gap Analysis: Core Gameplay Systems (§5-§15)"
type: explanation
audience: engineers
last_updated: 2026-10-03
companion_docs:
  - ../gap-analysis.md
  - ../project-status.md
---

# Gap Analysis: Core Gameplay Systems (§5-§15)

> Part of the [Gap Analysis](../gap-analysis.md), split out of it on 2026-10-03 with no change to any row. The status taxonomy, the evidence bar and the Summary Completion Matrix are in the main file; each matrix row counts the feature rows of its section here, so change both together.

## Core Gameplay Systems

### 5. Character Creation --- CW (core create-and-enter flow; was NT)

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/character-creation.md](../gameplay/character-creation.md)
- **Rust code**: [`crates/base/src/base/character_create.rs`](../../crates/base/src/base/character_create.rs) (636), [`crates/base-world-entry/src/base/character/`](../../crates/base-world-entry/src/base/character/) (1,068 across `mod.rs`, `delete_live_db_tests.rs`, `request_visuals_live_db_tests.rs`), [`crates/resources/src/base/chardef.rs`](../../crates/resources/src/base/chardef.rs) (333), plus `character_create_live_db_tests.rs` (209). About 2,250 lines in total. No code change since 2026-07-25.
- **Recent PRs**:
  - #473 / #516 / #518: SGWGmPlayer.
  - **#704**: the Account typeID is pinned at `0x07` (its clientIndex) with a guard. The owner re-verified this against the binary: `0x08` would break character select.
- **In-client record (new 2026-09-25)**: the 2026-09-18 colo playtest ran the full flow with a real client. It covered the character list (count 1 → 2 → 3) and two creations: player 71, a Human Soldier (archetype 1), and player 72, a Jaffa (archetype 8). The creation-time boot item (3438, from `char_creation_choices`) showed in the character-select preview ("jaffa had the boot in main menu after creation"). Both characters entered Castle_CellBlock and played through to Castle. Source: [analysis/playtests/2026-09-18-colo-castle/README.md](../analysis/playtests/2026-09-18-colo-castle/README.md) §3, rows 6:58 PM and 8:04 PM.
- **Path forward**:
  - Confirm that the client shows the name-reject feedback. P1 logged two `createCharacter` rejects for a surname with trailing whitespace; the tester retried and succeeded, but no one recorded what the client displayed.
  - Exercise **Delete**. P1's delete never reached the server.
  - Confirm the starting hotbar on a fresh character.
  - Name filtering and a per-account slot limit are still missing.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Character list display | CW | -- | base/character/ | **Promoted 2026-09-25.** SELECT from sgw_player. In-client: P1 list count 1 → 2 → 3 across two creations, each character selected and played ([playtest README](../analysis/playtests/2026-09-18-colo-castle/README.md) §3, 6:58 PM). The typeID `0x07` guard (#704) protects this path. Re-verified 2026-09-25 |
| Character visual preview | CW | -- | base/character/request_visuals_live_db_tests.rs | **Promoted 2026-09-25.** Lazy-load from sgw_inventory. In-client: P1 8:04 PM, the Jaffa's creation-time boot (item 3438) rendered in the character-select preview, `Sending character visuals component_count=14` (playtest README §3). Re-verified 2026-09-25 |
| Name validation | NT | -- | base/character_create.rs:549 | `validate_character_name` (3–20 chars, no leading/trailing or doubled spaces, `[A-Za-z0-9 '-]`) plus SQL uniqueness. The reject path fired in-client: P1 logged two rejects for surname `"will "`, sent as `send_char_create_failed(.., 2)` at character_create.rs:62. What the client displayed is unrecorded (playtest README §7, tester question 1) |
| Visual choice validation | NT | -- | base/chardef.rs | char_creation_choices table. Valid choices were accepted and rendered in P1. The reject path has not been exercised in a client |
| Archetype selection | CW | -- | base/character_create.rs | **Promoted 2026-09-25.** 8 archetypes from resources. In-client: P1 created archetype 1 (Human Soldier) and archetype 8 (Jaffa). Both played with archetype-correct content, for example chain 1098 vs chain 1099 crate loot (playtest README §3, 7:18 PM and 8:10 PM). Re-verified 2026-09-25 |
| Starting equipment | CW | -- | base/character_create.rs | **Promoted 2026-09-25.** BagFillOrder insertion. In-client: the item attached to the chosen visual (3438) was inserted and showed equipped in the preview and in world (P1 8:04 PM; playtest README §5 "boot-lock movement gate"). Re-verified 2026-09-25 |
| Starting abilities | NT | -- | base/character_create.rs | From the charDef ability list (known-issues KI-10 resolved). Not specifically observed in P1 |
| Character deletion | NT | -- | base/character/delete_live_db_tests.rs | CASCADE to inventory and missions. In P1 the delete **never reached the server**; it is unknown whether the tester pressed it (playtest README §5) |
| GM character creation | IM | -- | mercury/world_data/phases.rs:46 | **Corrected 2026-07-25.** SGWGmPlayer is ported: seed accounts get `access_level`, and a GM enters the world as entity class `0x03` instead of `0x02` (PRs #473 / #516 / #518, merged 2026-06-17). There is no GM-only *creation* UI |
| Name filtering | KM | -- | -- | No profanity or reserved-name check. `validate_character_name` is format-only |
| Character slot limit | KM | -- | -- | No per-account limit |

### 6. World Entry and Spaces --- CW

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [protocol/world-entry-phases.md](../protocol/world-entry-phases.md), [engine/space-management.md](../engine/space-management.md), [connection-flow.md](../connection-flow.md), [gameplay/death-respawn-system.md](../gameplay/death-respawn-system.md)
- **Rust code**: [`crates/base-world-entry/src/base/world_entry/`](../../crates/base-world-entry/src/base/world_entry/) — **95 files, 32,526 lines**. It covers `cell_dispatch/` (now including `position.rs` and `player_ghost.rs`), `gate_travel/`, `methods/{inventory, mail, player_load, progression, vendor}`, `reanchor_player.rs` (590) and `teleport.rs`. The post-reanchor replay is `cell/respawn/resync.rs:89` (`resync_after_pawn_recreate`).
- **Recent PRs**:
  - #410, #414 and #422: earlier edition.
  - **#756**: the reanchor replays the hotbar, active slot, journal and `state_field`, and logout persists position.
  - **#682**: region hints are replayed after the reanchor. This was playtest finding H8.
  - **#662 / H01** and **#795**: stargate arrival is validated, and Harset gate-row arrival is restored.
  - **#640 / P45**: a cross-space transfer primitive.
  - **#644**: GM world-name lookup is case-insensitive, and the snap-back loop is fixed.
  - **#747**: first-login cinematic AoI hold.
- **In-client record**: P1 ran Castle_CellBlock world entry for two fresh characters. The Cellblock → Castle cross-world hop was clean: 7:21:11 PM teleport, 7:21:14 PM Castle entry (6,608 B / 6 packets), 16 missions reloaded, no errors (playtest README §3; `appendix-session-timeline.md` rows 00:21:11 and 00:21:14). There were 19 same-world respawns, measured at 120.2–120.9 s, "position snapped + state cleared".

> **Open defect — cold-client direct login into a non-Cellblock world.** Four colo sessions went silent within about 15 s of `onClientReady` and hit the 60 s inactivity reap. Each was a freshly started client logging straight into Castle (at a GM-teleported spot) or SGC_W1. The server sequence was identical to healthy entries, with no WARN or ERROR, and the same worlds load fine through gate travel. Recorded in PR #756 "Not in this PR"; no issue is filed. It now matters more because #756 makes logout persist position, so more returning characters will log straight into Castle or Harset. It needs the client's own log or crash dump. The *Map load sequence* row stays `CW`: the server-side sequence is the part that row claims, and the record says that part was correct.

- **Path forward**:
  - File and root-cause the cold-client direct-login hang.
  - Run a respawn UAT for #756: the hotbar, journal, region hints and auto-cycle should survive a death.
  - Log out and back in inside Castle.
  - Verify the other published spaces end to end. Castle_CellBlock and Castle are now routinely played; Harset has placements but no written in-client pass.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Space loading | CW | -- | services/cell/space_manager | NavMesh + entity loading. A navmesh now loads for every world (#794). Castle has one since #709 |
| Player entity creation | CW | -- | base/world_entry/methods/player_load | Creates the SGWPlayer entity, two-stage base + cell |
| Map load sequence | CW | -- | base/world_entry/ | All 30+ client setup messages. See the cold-client direct-login callout above: the server sequence matches healthy entries, and the cause is unknown |
| Stat sync to client | CW | -- | base/world_entry/methods/player_load | All stats sent on entry |
| Ability tree sync | CW | -- | ability_tree/tree_info.rs, base/world_entry/methods/player_load | 3 trees per archetype, built from the shared `AbilityTreeCatalog` (AT-02); no Rust fallback tree. **Seed replaced 2026-09-26 (AT-05):** the Soldier/Commando stub became the FINAL v2 import, 439 nodes across 7 archetypes (Jaffa has none, D-AT04), pinned by `ability_tree::tests::seed_live_db`. No in-client pass with the new trees yet: branches now reach 25 nodes and levels up to 50 |
| Ability-tree purchase (spend gate, per-node cost) | NT | -- | ability_tree/gates/spend.rs, base/world_entry/methods/progression/train_ability.rs | AT-03 (2026-09-26): archetype-wide spend gate, per-node `skill_point_cost`, one guarded `UPDATE` (live-DB replay guard), point counter refreshed after each purchase. The cell's player level is now loaded at world entry and follows level-ups; before this, every player trained as level 1. No client run yet |
| Ability respec (trainer) | NT | -- | cell/cell_methods/player/vendor/respec.rs, base/world_entry/methods/progression/respec.rs, cell/service/base_messages/respec.rs | **New 2026-09-26 (AT-08).** `resetMyAbilities` (CM 72) was an `UNIMPLEMENTED` log. It now works only at a pinned trainer in range. One guarded `UPDATE` removes the trainer-bought abilities, refunds `tree_points_spent`, resets the spend and charges 1000 naquadah. A replay is free, and too little naquadah changes nothing (live-DB guards). Every refusal sends `onErrorCode` and the trainer re-send. Known gap: the client action bar has no server hook. Its bindings are a client-side saved variable (`GActionProfiles`), so buttons for refunded abilities stay until the player clears them. Pressing one now gets `onErrorCode` 167 instead of silence. A client Lua patch that clears the bar would be an owner decision. No client run yet |
| Zone transition | NT | -- | base/world_entry/gate_travel/ | **Changed 2026-09-25 (was IM).** The single-player hop is proven in-client: P1's Cellblock → Castle cross-world teleport was "CONFIRMED clean", with 16 missions reloaded and no errors (playtest README §3, 7:23 PM). The earlier IM reason, "multi-player sync incomplete", is now addressed in code: #737 introduces arriving players with the `SGWPlayer` ghost cascade once their client has loaded, and #640 (P45) adds a cross-space transfer primitive. Neither has a two-client run, hence NT rather than CW. Re-verified 2026-09-25 |
| Forced position handling | CW | -- | services/cell/cell_methods | BASEMSG_FORCED_POSITION authoritative move. #644 bounded the snap-back recovery: nearest navmesh point, then respawner, then AABB clamp, with a 5-correction budget |
| World-entry observability | CW | -- | base/world_entry/ | OTLP spans across the whole pipeline |
| Cell dispatch arms | IM | -- | base/world_entry/cell_dispatch/ | tests_dispatch_arms/ has live-DB coverage. New arms `position.rs` (logout persist) and `player_ghost.rs` are not client-validated |
| Same-world respawn client resync | NT | -- | base/world_entry/reanchor_player.rs; cell/respawn/resync.rs:89 | **New 2026-09-25.** The reanchor's `CREATE_BASE_PLAYER` wipes the client's per-entity caches. P1 finding H8: after respawning, the client sent zero region hints for 28 minutes. Main now replays inventory, region hints (#682), hotbar, active slot, journal and `state_field`, and keeps the auto-cycle bit (#756). Regression guards are `combat::tests::respawn_resync` and `base_messages::tests::disconnect_persist_position`. There is no in-client respawn record after the fix |

### 7. Movement and Navigation --- IM

- **Confidence**: HIGH (re-read 2026-09-25). Detour is live everywhere; every world now ships a mesh. Two defects from the 2026-09-18 playtest are still open: the speed-layer arithmetic and Castle's split mesh.
- **Documentation**:
  - [protocol/position-updates.md](../protocol/position-updates.md) and [drafts/spec/position-updates.md](../drafts/spec/position-updates.md) (the canonical bible draft).
  - [architecture/movement-validation.md](../architecture/movement-validation.md), [architecture/navmesh-containment-modes.md](../architecture/navmesh-containment-modes.md) and [architecture/movement-telemetry.md](../architecture/movement-telemetry.md).
  - [engine/navmesh-build-pipeline.md](../engine/navmesh-build-pipeline.md), [engine/navbuilder-recast-limits.md](../engine/navbuilder-recast-limits.md) and [engine/castle-navmesh-connectivity.md](../engine/castle-navmesh-connectivity.md).
  - [`data/spaces/README.md`](../../data/spaces/README.md), the per-world mesh provenance.
- **Rust code**:
  - Entity crate:
    - [`crates/entity/src/movement.rs`](../../crates/entity/src/movement.rs) (351).
    - [`crates/entity/src/navigation/`](../../crates/entity/src/navigation/) (4,571): `mod.rs`, `load.rs`, `load_tiled.rs`, `path.rs`, `surface.rs`, `line_of_sight.rs`, `verdict.rs`, `fingerprint.rs`, `poly_block.rs`, `xrc.rs`, tests.
    - [`crates/entity/src/movement_validation/`](../../crates/entity/src/movement_validation/) (870).
    - [`crates/entity/src/detour_ffi.rs`](../../crates/entity/src/detour_ffi.rs) (138, now with the tiled-mesh entry points). Detour is compiled from source by `crates/entity/build.rs`.
  - Cell seam:
    - [`cell/space_manager/client_move.rs`](../../crates/cell-world/src/cell/space_manager/client_move.rs) (664).
    - [`navmesh_containment.rs`](../../crates/cell-world/src/cell/space_manager/navmesh_containment.rs) and the `NavmeshMode` enum in [`crates/cell-catalog/src/cell/spawner/navmesh_mode.rs`](../../crates/cell-catalog/src/cell/spawner/navmesh_mode.rs) (329 together).
    - `movement_telemetry/` (999).
  - NPC stepping: `cell/service/ticks/npc_movement.rs` (682) and `npc_ground.rs` (102).
  - Build tooling: [`crates/navmesh-extractor/`](../../crates/navmesh-extractor/). The meshes are 24 `.nav` files under `data/spaces/`, with per-world `.occ` occluders.
- **Recent PRs**:
  - Earlier edition: #437, #478, #428.
  - Validator fixes:
    - **#643**: a jump no longer trips the navmesh reject.
    - **#644**: the snap-back rubber-band loop is fixed, with a recovery ladder and a correction budget.
    - **#645**: facing-preserving position primitive.
    - **#639**: `onPhysics` 221 GM fly/ghost bypass.
  - Mesh tooling and data:
    - **#683 / #710**: UE3 Terrain + BSP extraction pipeline.
    - **#694**: rebuilt `castle_cellblock.nav`.
    - **#709**: `castle.nav`, with Castle seeded advisory.
    - **#682**: per-world `navmesh_mode`.
    - **#794**: a mesh for every world; the new ones are seeded advisory.
    - **#796**: tiled meshes for the seven large exteriors.
    - **#700 / #726**: navmesh telemetry.
  - NPC stepping:
    - **#677**: `pack_angle` wrap and attacker re-face.
    - **#774**: storey-aware ground height.
    - **#779**: zero velocity when stopped.
    - **#783**: grounded every step.
    - **#788**: island edges and off-mesh starts.
    - **#707**: `move_waypoint` witness snap.
- **In-client record**: P1 is the source for these, with the fixes now shipped:
  - **H1**, backwards facing: #677.
  - **H2 / H4**, floating and the Y sawtooth at leg boundaries: #783, #774.
  - **H3**, no Castle mesh: #709.
  - **H4b**, attackers could not turn: #677.
  - **Rubber-band snap-backs**: 212 in Castle_CellBlock against the 2013 mesh. The rebuilt mesh accepts 195 of those 212 positions (#694).

  Still open from P1: the speed validator divides by per-packet wall-clock (35–60 ms windows, `Inf` reaching the metric, 756 warnings in one session; playtest README §5). U1 confirmed in play that no guard aggroed across floors and that guards held their cover slots. That is NPC AI evidence; it does not establish movement correctness.
- **Path forward**:
  - Window the speed layer over game ticks, *then* calibrate `SPEED_WARN_TOLERANCE` and switch it to enforce (#461 CAT-B-01 residual).
  - Join Castle's exterior and interior meshes with off-mesh links or per-region handling ([engine/castle-navmesh-connectivity.md](../engine/castle-navmesh-connectivity.md) §4).
  - Walk Cellblock as a non-GM to confirm the rebuilt mesh.
  - Promote more worlds from advisory to enforce as coverage is verified.
  - `unstuck` is still a stub (#461 CAT-B-08).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Client position updates | CW | -- | services/cell/cell_methods; cell/space_manager/client_move.rs | playerUpdate accepted. P1 ran two characters for about 90 minutes with no position-sync complaint beyond the snap-backs covered below |
| Dead reckoning / interpolation | IM | -- | entity/movement.rs | Extrapolates between updates. No change since 2026-07-25 |
| Space bounds check | IM | -- | entity/movement_validation/bounds.rs | AABB + NaN/infinity + Z-floor-clip. **Re-pointed 2026-09-25:** now wired at cell/space_manager/client_move.rs:260 (`entities.rs:288` is stale) |
| NavMesh / Detour FFI | NT | -- | entity/detour_ffi.rs; entity/navigation/ | **Changed 2026-09-25 (was IM).** The FFI now also loads tiled meshes (`detour_create_tiled_navmesh` / `detour_add_tile`, #796). Load is header-first with size caps (#726), and ground height is storey-aware (#774). `find_path` is at navigation/path.rs:149 and `raycast` at line_of_sight.rs:217. NPC line of sight now prefers per-world collision occluders (`.occ`, #797). Exercised in two colo sessions (P1 Cellblock paths, U1 cover-slot routing), but neither record judges the query results themselves. Re-verified 2026-09-25 |
| NPC pathfinding | IM | -- | entity/navigation/path.rs | Path query, off-mesh start snap and island-edge hold (#788). **Known gap:** Castle's mesh is still split exterior ↔ interior with no stair or ramp geometry between them (549 components; [engine/castle-navmesh-connectivity.md](../engine/castle-navmesh-connectivity.md) §4). There are no off-mesh links, so an NPC cannot route between storeys joined by scripted lifts or rings |
| NPC waypoint movement | NT | -- | cell/service/ticks/npc_movement.rs, npc_ground.rs | **Changed 2026-09-25 (was IM).** Every P1 stepping defect has a merged fix: facing wrap (#677), per-step navmesh grounding including ramps and stairs (#783, #774), zero velocity when stopped (#779), witness snap for `move_waypoint` (#707). The NPC AI ledger lists each as UATPending ([session-resume](../analysis/npc-ai-restoration/handoffs/session-resume.md) "Owner UAT checklist" item 2). Re-verified 2026-09-25 |
| NPC patrol | IM | -- | cell/service/npc_ai/patrol.rs | 262 lines. `AiState::Patrol` is set by the GM `.path_*` authoring commands (cell/console/patrol.rs). Few seeded routes exist, and there is no in-client patrol record |
| Server-side speed validation | IM | -- | entity/movement_validation/mod.rs:279 | Still **warn-only**. **Defect on record (P1, playtest README §5):** `implied_speed = distance / dt` per packet on 35–60 ms windows, with `dt < 1e-4` mapped to `f32::INFINITY` (mod.rs:308-312). That produced 756 warnings in one session, with `Inf`/`NaN` reaching the metric. The code is unchanged. #644 added the `movementSpeedMod` stat to the budget. Tracked in #461 (CAT-B-01 residual). Re-verified 2026-09-25 |
| Teleport detection | NT | -- | entity/movement_validation/mod.rs:314; cell/space_manager/client_move.rs:490-530 | **Changed 2026-09-25 (was IM).** Dual gate: distance > `TELEPORT_JUMP_UNITS` (50) **and** implied speed > `top_speed × TELEPORT_SPEED_FACTOR`, then snap-back. The live-incident rubber-band loop (`.goto harset`) is fixed by #644: `Rejected` / `Recovered` / `CorrectionSuppressed`, a budget of 5, and `note_authorized_teleport` clearing strikes at every authorized move. Jumping no longer false-rejects (#643). Not re-tested in-client after the fixes |
| Player navmesh containment (per-world mode) | IM | -- | cell/space_manager/client_move.rs:275; cell/space_manager/navmesh_containment.rs | **New 2026-09-25.** Layer 4 of the validator, gated per world by `resources.worlds.navmesh_mode` (#682). Castle_CellBlock is the only `enforce` world; every other world is `advisory`: accepted, logged at TRACE (#709, #794). GMs bypass it (`navmesh_gm_bypass`, `onPhysics` fly/ghost #639). **Defect on record:** P1 logged 212 Cellblock snap-backs against the 2013 mesh. The rebuilt mesh (#694) accepts 195 of 212. A spot at (-196.2, 55.5, -139.1) is still uncovered and "needs an in-client look" (PR #694) |
| Navmesh coverage (a mesh per world) | IM | -- | data/spaces/*.nav; crates/navmesh-extractor/ | **New 2026-09-25.** 24 meshes built from the cooked client maps (StaticMesh + Terrain + BSP), with tiled builds for the seven big exteriors (#683, #709, #794, #796). Seeded-point probe hit rates vary. Castle is 62/78, Harset 51/67, Cellblock 46/92 ([data/spaces/README.md](../../data/spaces/README.md); these are prefab and area corners, not only standable points). No mesh except Cellblock's has been walked in-client under enforcement |

### 8. Entity Lifecycle (AoI) --- IM (open entity-introduction defect; see project-status Known Issues)

- **Confidence**: MEDIUM (re-read 2026-09-25). Downgraded 2026-07-25 and still MEDIUM. The invisible-entity defect below has no validated fix; the #747 experiment is shipped but unobserved. Do not plan against "AoI is done". Note that [project-status.md](../project-status.md) lists this system as `IM`; the two docs disagree on the heading.
- **Documentation**: [engine/entity-lod-system.md](../engine/entity-lod-system.md), [engine/entity-type-catalog.md](../engine/entity-type-catalog.md), [architecture/first-login-cinematic-aoi-hold.md](../architecture/first-login-cinematic-aoi-hold.md), [architecture/player-ghost-aoi-cascade.md](../architecture/player-ghost-aoi-cascade.md)
- **Rust code**:
  - [`crates/entity/src/cell_entity/`](../../crates/entity/src/cell_entity/): bandolier, state_flags, system_options, `witness_aoi.rs` (with `is_introducible` at :55), tests, mod.
  - [`crates/entity/src/world_grid.rs`](../../crates/entity/src/world_grid.rs) and [`crates/entity/src/space.rs`](../../crates/entity/src/space.rs).
  - [`base/world_entry/cell_dispatch/aoi.rs`](../../crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi.rs) (599), [`cell_dispatch/player_ghost.rs`](../../crates/base-world-entry/src/base/world_entry/cell_dispatch/player_ghost.rs) (369) and [`cell_dispatch/deferred_flush.rs`](../../crates/base-world-entry/src/base/world_entry/cell_dispatch/deferred_flush.rs) (455).
  - [`base/deferred_aoi_lifecycle.rs`](../../crates/base-session/src/base/deferred_aoi_lifecycle.rs) (205).
  - [`base/world_entry_appearance/cinematic_aoi_hold/`](../../crates/base-world-entry/src/base/world_entry_appearance/cinematic_aoi_hold/mod.rs) (551).
  - [`mercury/aoi/`](../../crates/wire/src/mercury/aoi/): `create.rs`, `leave.rs`, `method.rs`, `update.rs`, `player_ghost.rs` (337).
  - Witness fan-out helpers in `cell/abilities/messaging.rs:98,153`.
- **Recent PRs**:
  - Earlier: #279 (BeingAppearance recomposite broadcast), #418, #408/#410, #580 (player combat and death state fanned out to witnesses; closes #232), #582 (`aoi.create_emit` / `aoi.create_send_failed` seams).
  - **#737**: players in a shared world see each other.
  - **#747**: first-login cinematic AoI hold, plus the `aoi.create_emit` OTLP export.
  - **#707**: `move_waypoint` fans `EntityMoved` to witnesses immediately (closes #616).
  - **#779**: the malformed `onSequence` "movement type" broadcast is no longer sent.
  - **#688 / #695**: lab witness queries and packet taps (tooling).

> **Open defect — invisible entity until relog.** In Castle Cellblock a GuardBody corpse (a `class_id 0` static mesh) is not visible to a first-login player until they relog. The 2026-06-20 repro disproved the address-gate hypothesis. The 2026-09-19 repro retired Mercury delivery: every create was ACKed on the first try, so the drop is client-side. #747 (merged 2026-09-21) ships a first-login cinematic AoI hold as the experiment on the one n=1 differential ([architecture/first-login-cinematic-aoi-hold.md](../architecture/first-login-cinematic-aoi-hold.md)). **No in-game observation of the hold exists as of 2026-09-25.** U1 (2026-09-25, a build that includes #747) does not mention the corpse either way. The Cellblock UAT guide's results table is still blank ([uat-guide.md](../analysis/castle-cellblock-rebuild/uat-guide.md) "Results", T03 and T28). Treat entity-introduction *rendering* as unproven.

- **Path forward**:
  - Run the #747 check: one fresh character watches the first-login movie to the end, another presses Esc, and both look at the GuardBody corpse.
  - Run the two-client checklist in [architecture/player-ghost-aoi-cascade.md](../architecture/player-ghost-aoi-cascade.md).
  - #278 (the witness-fanout consolidation) was closed as done on 2026-09-25.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Entity creation | CW | -- | entity/manager.rs | From template or dynamic |
| Entity destruction | CW | -- | entity/manager.rs | Cleanup + witness notification |
| Grid-based AoI | CW | -- | entity/world_grid.rs | Chunk-based witness management |
| Witness enter/leave | IM | -- | entity/cell_entity/mod.rs; base/world_entry/cell_dispatch/aoi.rs | **Downgraded 2026-07-25; unchanged 2026-09-25.** onEnter/onLeave fire, but a witness can still fail to *render* an entity it was correctly introduced to (see the callout above). The #747 hold buffers `EnteredAoI` / `LeftAoI` / witness methods / `EntityInvisible` until `cancelMovie` or 16 s. `deferred_aoi_lifecycle.rs` keeps leave-before-create order on flush. Not observed in game. Re-verified 2026-09-25 |
| Property synchronization | CW | -- | entity/properties.rs | Per-distribution-flag write paths |
| State flag conventions | CW | -- | entity/cell_entity/state_flags.rs | bStateField, BSF_InCombat lifecycle |
| Bandolier state | CW | -- | entity/cell_entity/bandolier.rs | Slot lifecycle, type_id vs item_id discipline |
| LOD system | KM | -- | -- | No entity detail levels |
| Witness-fanout helper | NT | -- | cell/abilities/messaging.rs:98,153 | **Changed 2026-09-25 (was IM).** `send_entity_method_to_witnesses` and `send_entity_method_to_self_and_witnesses` carry combat, effects, stats, death and corpse state (#336, #580). All five child issues are closed, and parent #278 was closed as complete on 2026-09-25 (owner triage comment). Fan-out to *other players* has no two-client record, hence not CW |
| Player-to-player introduction | NT | -- | mercury/aoi/player_ghost.rs; base/world_entry/cell_dispatch/player_ghost.rs; entity/cell_entity/witness_aoi.rs:55 | **New 2026-09-25.** Players in a shared world (Castle, Harset) are introduced with a dedicated `SGWPlayer` / `SGWGmPlayer` ghost cascade. It carries appearance, name, level, alignment and live combat/death state. `is_introducible` holds a loading player out of AoI until it is initialised, and level-ups fan out (#737). Pinned by wire-format, fan-out byte and negative-log tests. **Not run with two real clients**; the UAT checklist is in [architecture/player-ghost-aoi-cascade.md](../architecture/player-ghost-aoi-cascade.md) |

### 9. Combat and Abilities --- IM

- **Confidence**: HIGH for primitives, MEDIUM for end-to-end coverage (re-read 2026-09-25). Two rows were corrected against the code, and one is promoted on in-client playtest records.
- **Documentation**: [gameplay/combat-system.md](../gameplay/combat-system.md), [gameplay/ability-system.md](../gameplay/ability-system.md), [architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md) (decisions 17-19: health-threshold drain, surrendered-NPC floor, single `resolve_death`), [reverse-engineering/findings/combat-wire-formats.md](../reverse-engineering/findings/combat-wire-formats.md), [reverse-engineering/findings/combat-formulas-status.md](../reverse-engineering/findings/combat-formulas-status.md) (#673: which formulas can be recovered and which are fan guesses)
- **Rust code**: [`crates/cell-combat/src/cell/combat/`](../../crates/cell-combat/src/cell/combat/) (3,400 lines, 13 files: `damage/{pipeline,qr}.rs`, `threat/{aggro,player_combat}.rs`, `auto_cycle.rs`, `state.rs`, `health_threshold.rs`, `faction_reaction.rs`, ...), [`crates/cell-combat/src/cell/abilities/`](../../crates/cell-combat/src/cell/abilities/) (7,994 lines, 31 files, including `death/`, `damage_apply/`, `cone_aoe/`, `use_ability/`). That makes **11,394 lines in production combat and 194 tests** (90 in combat, 104 in abilities). [`crates/game/src/combat/`](../../crates/game/src/combat/) (463 lines, 10 tests) has **no consumer outside `cimmeria-game`**. It is a dead parallel model and the live pipeline does not use it.
- **Recent PRs**: #420 (the complete ability and effect system) is still the base. Since 2026-07-25:
  - **#747**: an effect-script bleed to 0 HP kills on the same shot, for NPC and player victims. Every death goes through `abilities::death::resolve_death`, and a DoT kill is a real kill.
  - **#785**: leash drains the NPC from player combat.
  - **#787 / #789**: proximity aggro and same-room assist aggro feed the threat list.
  - **#791**: a dying player leaves every threat list, and `useItem` is refused while `BSF_DEAD`.
  - **#786 / #793 / #797**: NPC attack line of sight (stationary-relaxed policy, cover peek point, collision-geometry occluders).
  - **#734 / #744**: `onTimerUpdate` SecondaryId.
  - **#722 / #725**: docs-only verification of the cooldown SourceID and the PAK layout.
  - **#673**: the combat-formulas evidence ledger.
  - **#677**: attackers re-face their target.
  - **#635-#637**: `.combatinfo`, `.stats` and `.listabilities`.
  - Harset H04 (`560a8bd5`, via #662/#682): `entity_health_below` fires from every damage path.
  - **Ammo campaign (#1026, 2026-09-28)**, [ledger](../analysis/ammo/README.md): the loaded special ammo type's `ammo_modifiers` row modifies every player weapon shot in `damage_apply` (#1047), with an on-hit effect per family: Incendiary burn (#1053), EMP drain (#1063), Explosive 5 m splash with line of sight (#1063), crowd-control, tech and support darts (#1054, #1063). Support darts heal or cleanse allies and the shooter and are refused at hostiles (#1069). On by default since AM-12 (`ammo.finite_special`, D-AM11).
- **Path forward**:
  - Enforce LOS on player `useAbility`.
  - Add a min-range check.
  - Add positional (front/flank/rear) checks.
  - Enforce prerequisite monikers.
  - Fix the two divergences #673 found: `EF_DONT_USE_QR` is 32 and is never read (the original is 16), and the `EDamageType` wire values are 0-4 (the original is 13-18; check a client capture before changing them).
  - Add threat decay.
  - Fix the combat animation (the shoot animation does not play, UAT-1 finding 10).
  - Model deploy abilities.
  - Delete or re-home the dead `crates/game/src/combat/`.
  - Special ammo's limits ([ammo ledger, known issues](../analysis/ammo/README.md#known-issues-and-follow-ups)): populate `MITIGATION` so `penetration_mult` does something, an EMP interrupt, support darts on friendly NPCs and pets, and client-safe effect ids for the pulsing on-hit effects.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| QR calculation | IM | -- | cell/combat/damage/qr.rs:50 | Hit/miss/crit roll via `calculate_qr` / `calculate_result`. **Re-verified 2026-09-25.** Per #673 the QR-to-outcome mapping is a FAN-GUESS. `EF_DONT_USE_QR` (entity/abilities/defs.rs:58) is 32, but the original is 16, and it is never read, so the 754 no-QR effects still roll |
| Damage calculation | IM | -- | cell/combat/damage/pipeline.rs:33 | Resist → AF → absorb pipeline. The wire `damage_code` sends 0-4, but the original `EDamageType` is 13-18 (#673, unverified against a capture) |
| Stat resistance | IM | -- | cell/combat/damage/pipeline.rs:167 | **Path corrected 2026-09-25** (was game/combat/stats.rs, which is dead code). Flat-percent model. #673 shows that the original resistances were gating QR rolls, not flat reductions |
| Armor factor | IM | -- | cell/combat/damage/pipeline.rs:181 | Per-damage-type AF. **Path corrected 2026-09-25** |
| Absorption | IM | -- | cell/combat/damage/pipeline.rs:195 | 15 absorption stats. **Path corrected 2026-09-25** (was game/combat/damage.rs, which is dead code) |
| Auto-cycle (auto-fire) | CW | -- | cell/combat/auto_cycle.rs | Re-fires when the cooldown completes |
| In-combat state lifecycle | CW | -- | cell/combat/state.rs, threat/player_combat.rs | BSF_InCombat per-player threat tracking. Leash now drains player combat (#785) |
| Threat list | IM | -- | cell/combat/threat/aggro.rs:147 | `generate_threat` is exercised in-client: 50 player-initiated aggros in the [2026-09-18 playtest](../analysis/playtests/2026-09-18-colo-castle/README.md). Proximity and assist sources were added (#787/#789), and dead players are dropped (#791, UAT-1 finding 4). **Still no threat decay**, and #791 has not been re-UAT'd. Re-verified 2026-09-25 |
| Single-target abilities | IM | -- | cell/abilities/use_ability/handle.rs | TCM_Single. Kills in-client are recorded both ways (playtest README 7:37, UAT-1), but the shoot animation does not play ([UAT-1](../analysis/npc-ai-restoration/worknotes/uat-1.md) finding 10, the pre-existing combat-animation issue) |
| AoE abilities (radius) | IM | -- | cell/abilities/dispatch/mod.rs | PR #420. A `TCM_AERadius` effect on a single-target ability still does not fan out (ADR follow-up) |
| AoE abilities (cone) | IM | -- | cell/abilities/cone_aoe/ | PR #420 |
| Group targeting | KM | Groups | -- | TCM_Group. No code, and **no seeded effect uses it**: the seed has only TCM_Single, TCM_AERadius and TCM_AECone (entity/abilities/defs.rs:40-48) |
| Aura targeting | KM | -- | -- | TCM_Aura. No code, and no seeded effect uses it |
| Ability warmup | IM | -- | cell/abilities/use_ability/warmup/ | **Corrected 2026-09-26 (AT-10).** Before AT-10 only the `Ability_Begin` animation existed: the cast fired and dealt its damage in the launch pass, and the speed stats were never read. Now a warmup ability parks a pending cast and fires from the 100 ms tick after the warmup, with the speed-stat modifiers. Interrupts on death, slot change, movement and fire-time target/range/LoS/ammo checks, with `Ability_Interrupt` and a cooldown refund. Unit and wire tested; not client-exercised |
| Ability cooldowns | CW | -- | entity/abilities/manager.rs:303 | Per-ability and per-moniker timers. The SourceID emit paths were verified (#722). Cooldown, warmup, reload and effect timers send an absolute `BigWorldTimeComplete` on the one server game clock (CR-02, closes the #271 gap). Category (type 8) timers are still not sent |
| Position/facing checks | KM | -- | -- | **Corrected 2026-09-25 (was IM).** `use_ability/handle.rs` has no front/flank/rear or facing test, and neither does any file under `cell/abilities/` or `cell/combat/`. "Flank" exists only as the cover mission trigger `player_flanked_npc` (#671). Re-verified 2026-09-25 |
| Weapon range checks | IM | -- | cell/abilities/use_ability/handle.rs:239 | Only max range is enforced (default 30 u, error code 42). **No min-range check** on the player path. Re-verified 2026-09-25 |
| Ammo consumption | CW | -- | cell/abilities/use_ability/handle.rs:366 | Decrements under the bandolier discipline |
| Special ammo modifiers and on-hit effects | IM | -- | cell/abilities/damage_apply/, cell/effects/ammo_*.rs | **New 2026-09-28 (ammo campaign, [ledger](../analysis/ammo/README.md)).** Every player weapon shot applies the loaded type's `ammo_modifiers` row: damage and penetration multipliers, damage type, on-hit effect (Hollow Point, Armor Piercing, Incendiary burn, EMP drain, Explosive splash, seven dart effects). IM, not NT: `MITIGATION` is capped at 0, so penetration does nothing; EMP has no interrupt; Nanites has no effect; the pulsing on-hit effects send effect ids the client does not know. Unit, pipeline and live-DB seed tests; not client-exercised |
| Support-dart ally shots | NT | -- | cell/abilities/use_ability/support_shot.rs | **New 2026-09-28 (AM-11d, #1069).** A Stim, Adrenaline, Antidote or Coagulant dart heals or cleanses another player or the shooter with no damage, threat or combat state, and is refused at a hostile with "Support rounds only affect allies." Friendly NPCs and pets stay refused. No floating heal number. Whether the client emits a shot at a friend is statically traced, not seen (UAT AMMO-03) |
| Auto-reload | CW | -- | cell/abilities/use_ability/auto_reload.rs | PR #394. Open verify-only issue #720 (reload timer type 2 against the binary's types 12/13) |
| Damage application | CW | -- | cell/abilities/damage_apply/mod.rs, death/mod.rs | **Promoted 2026-09-25 (was IM).** In-client record, [2026-09-18 playtest](../analysis/playtests/2026-09-18-colo-castle/README.md): 26 lootable kills, 47 kill-XP grants, 19 player deaths to NPCs with 120 s respawns. [UAT-1 (2026-09-25)](../analysis/npc-ai-restoration/worknotes/uat-1.md): a guard killed the player. The 2026-09-19/21 bleed-to-0 defects (NPC and player) are fixed by #747 with regression guards |
| LOS checks | IM | -- | cell/space_manager/spatial.rs:22,41,125; cover_sight.rs:113 | NPC side: navmesh ray, a stationary-relaxed policy (#786), a cover peek point (#793), per-world collision-geometry occluders (#797), and LoS gates on aggro and assist (#787/#789). UAT-1 found guards shooting through walls (fixed by #793, not re-UAT'd). **Player `useAbility` still has no LOS check**: no LoS call exists under `cell/abilities/`. Re-verified 2026-09-25 |
| Prerequisite monikers | KM | -- | -- | `AbilityManager::can_use_ability` (entity/abilities/manager.rs:303) checks known/cooldown/moniker-cooldown only. **No `canUseWithMonikers` exists in `crates/`**: the old "loaded, not checked" note was wrong. Re-verified 2026-09-25 |
| Deploy abilities | IM | -- | entity/abilities/defs.rs:16 | Only `AF_SPEED_DEPLOY` is used, as a speed-modifier category. There are no deploy semantics |
| Health/focus regen tick | IM | -- | cell/service/ticks/regen.rs | **Path corrected 2026-09-25.** 1 Hz out-of-combat regen keyed on an empty `threatened_mobs`, with a floor of 1 per pool. Effect-driven HoT goes through `cell/effects/pulsing/tick.rs`. No in-client record |

### 10. Effects and Buffs --- IM

- **Confidence**: HIGH for the framework, MEDIUM for content coverage (re-read 2026-09-25). Four rows were corrected after the code showed that the `EF_ClearOn*` / "permanent" machinery the old notes cite does not exist.
- **Documentation**: [gameplay/effect-system.md](../gameplay/effect-system.md), [architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)
- **Rust code**: [`crates/cell-world/src/cell/effects/`](../../crates/cell-world/src/cell/effects/) (the scripts) and [`crates/cell-combat/src/cell/effects/`](../../crates/cell-combat/src/cell/effects/) (the pulsing): 3,742 lines across 9 files and **57 tests**:
  - `registry.rs` (63 lines);
  - `pulsing/{register,tick,channel_cancel,mod}.rs` (1,041 lines, plus a 649-line `tests.rs`);
  - `scripts.rs` (1,648 lines, of which tests start at line 696);
  - `cover_stance.rs` (173 lines, new in #790);
  - `mod.rs` (168 lines).

  **11 registered scripts**: HealHealth, HealFocus, MeleeDamage, MeleePhysicalDamage, AbsorbShield, Stun, Suppression, RangedPhysicalDamage, RangedEnergyDamage, CoverStance and RemoveCoverStance. **17 of 3,216 seeded effect rows** name a registered script (a further row names the unregistered `Reload`). The rest flow through the NVP `HealthDamage`/`FocusDamage` path.
- **Recent PRs**: #420 is the headline. Since 2026-07-25:
  - **#744** (with #734): the `onTimerUpdate` DurationEffect packet now sends the effect ID as SecondaryId. The client keys its active-effect timer lookup on that field, and it was 0 before.
  - **#747**: DoT and effect-script kills route through `resolve_death`.
  - **#619**: server-authoritative `launch_ability` / `apply_effect` content actions (`cell/content/effect_apply.rs`).
  - **#790**: Cover Stance buff and unbuff scripts.
  - ADR decision 18: surrendered NPCs are floored at 1 HP by pulses.
- **Path forward**:
  - Honour the rest of the effect-clear flags (`EEffectFlag`). `EF_ClearOnDeath` is `EF_CLEAR_ON_DEATH` (4) and only the timed stat-buff ledger honours it; pulsing `active_effects` ignore it. `EF_ClearOnDamage`, `EF_ClearOnRez` and `EF_RemoveOnBandolierSlotChange` have no Rust constant.
  - Send a wire packet for single-shot scriptless effects: `register_active_effect` returns early (pulsing/register.rs:50), so Stasis Sickness, the Prison Boot and similar effects are server no-ops.
  - Dispatch `effect_*` content triggers (#610; open PR #745).
  - Persist effects across logout.
  - Add script coverage for the long tail.
  - Wire `COVER_DEFENSE` into QR, because Cover Stance changes a stat that combat never reads.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Effect application | CW | -- | cell/effects/pulsing/register.rs:42 | A same-source re-apply refreshes (and never shortens), and different sources stack. UAT-1 recorded Cover Stance granted and removed in balance. Caveat: single-shot scriptless effects register nothing and send no packet (register.rs:50) |
| Effect pulse/tick | CW | -- | cell/effects/pulsing/tick.rs | Timer-driven at 100 ms. A DoT kill is now a real kill (#747) |
| Effect removal | CW | -- | cell/effects/pulsing/tick.rs:141 | Expiry sweep, then the script's `on_remove` (Stun, AbsorbShield, RemoveCoverStance), then an `onTimerUpdate` clear carrying SecondaryId (#744). Note corrected 2026-09-25: there is no general "revert non-permanent stat changes" |
| Stat change tracking | IM | -- | cell/effects/scripts.rs:390,454; cover_stance.rs | **Corrected 2026-09-25 (was CW).** No permanent/non-permanent distinction exists anywhere in `crates/` (a grep for "permanent" in cell/effects and entity finds nothing). Stat changes are reverted only by an individual script's `on_remove` (AbsorbShield, Stun, CoverStance). Direct HEALTH/FOCUS writes are one-way. Re-verified 2026-09-25 |
| Shared QR per pulse | IM | -- | cell/effects/pulsing/tick.rs:325 | Pulses hold QR neutral; the cast's roll is authoritative |
| Clear on death | IM | -- | cell/effects/stat_buffs/mod.rs; abilities/death/mod.rs; cell/effects/pulsing/tick.rs | **Corrected 2026-09-28 (#804).** `EF_ClearOnDeath` is `EF_CLEAR_ON_DEATH` (4, `entity/abilities/defs.rs`). `resolve_death` calls `clear_stat_buffs_on_death`, which ends the dead entity's timed stat buffs whose effect carries it (no stimpack row does). Pulsing `active_effects` ignore the flag: pulses on a dead target are skipped (the instances stay and age out), and a dying channeller's channels are cancelled |
| Clear on damage | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_ClearOnDamage` constant, and no code removes an effect when damage is taken. The `EF_*` constants are in `entity/abilities/defs.rs`; none is ClearOnDamage. Re-verified 2026-09-28 |
| Clear on revive | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_ClearOnRez`, and the respawn/revive paths never touch `active_effects` (the only writers are `cell/effects/pulsing/*`). Re-verified 2026-09-25 |
| Clear on bandolier swap | KM | -- | -- | **Corrected 2026-09-25 (was IM).** No `EF_RemoveOnBandolierSlotChange`, and the bandolier handlers never touch `active_effects`. Re-verified 2026-09-25 |
| Effect scripts (registry) | IM | -- | cell/effects/registry.rs:21-33 | 11 scripts, bound by 17/3,216 effect rows. The `effect_*` content triggers are inert (#610, open) |
| Effect persistence | KM | -- | -- | EF_AlwaysPersist is not honoured across logout |
| Channeled effects | IM | -- | cell/effects/pulsing/channel_cancel.rs | Four cancel triggers (new ability, death, movement over 0.5 m, safety cap), with the `AF_CHANNEL_ALLOWS_MOVEMENT` override |
| Stealth-related flags | KM | -- | -- | EF_RemoveOnStealthZeroed and the others are not handled |

### 11. Stats --- IM

- **Confidence**: HIGH (infrastructure), MEDIUM (formula coverage), re-read 2026-09-25. No status changes, only path corrections.
- **Documentation**: [gameplay/stat-system.md](../gameplay/stat-system.md) (**stale**: "Item stat bonuses PARTIAL, `inventoryAdjustments` exists", but no Rust code references it), [gameplay/progression-system.md](../gameplay/progression-system.md), [reverse-engineering/findings/combat-formulas-status.md](../reverse-engineering/findings/combat-formulas-status.md) (stat units verified from `alias.xml`; curves unknown)
- **Rust code**: [`crates/entity/src/stats/`](../../crates/entity/src/stats/): 1,137 lines, 6 files, 23 tests. [`crates/game/src/combat/stats.rs`](../../crates/game/src/combat/stats.rs) (159 lines) has **no consumer** and is dead code.
- **Recent PRs**: 29d46a65 (`StatList::scale_for_level()`, full heal on level-up). Since 2026-07-25: #636 (`.stats` plus the six stat-group dumps, verified field by field against legacy), #790 (the first effect to write `COVER_DEFENSE`), #756 (the reanchor replays client caches).
- **Path forward**:
  - Equipment stat bonuses (still no code).
  - Derived-stat formulas: none of them can be recovered from shipped data (#673), so they are a design decision parameterized by the verified 0.01-QR units.
  - Read `COVER_DEFENSE` and `coverAccuracy` in combat.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Stat class (min/cur/max) | CW | -- | entity/stats/stat.rs | 6 values plus dirty flags |
| Dirty stat sync | CW | -- | entity/stats/stat_list.rs | Incremental updates |
| Public/private split | CW | -- | entity/stats/stat_ids.rs | 11 public, the rest private |
| Archetype base stats | CW | -- | entity/stats/archetype.rs | Applied on first load |
| Per-level stat growth | CW | -- | entity/stats/stat_list.rs:324 | **Path corrected 2026-09-25** (was game/player.rs). `scale_for_level()` applies health/focus per level |
| Derived stat formulas | KM | -- | -- | No general stat derivation system. #673: the original curves are not in any shipped artifact |
| Stat soft caps | NU | -- | -- | No diminishing returns. #673: `EEffectFlag` has no cap or DR vocabulary |
| Item stat bonuses | KM | Inventory | -- | Equipment still does not modify stats. No `inventoryAdjustments` or item-stat code in `crates/`. #673: no shipped item field is a combat stat |

### 12. Inventory and Items --- IM

- **Confidence**: HIGH (re-read 2026-09-25)
- **Documentation**: [gameplay/inventory-system.md](../gameplay/inventory-system.md), [reverse-engineering/findings/inventory-wire-formats.md](../reverse-engineering/findings/inventory-wire-formats.md), [reverse-engineering/findings/inventory-state-machine.md](../reverse-engineering/findings/inventory-state-machine.md), [content/equip-from-inventory-pattern.md](../content/equip-from-inventory-pattern.md), [content/consumable-via-onitemuse-pattern.md](../content/consumable-via-onitemuse-pattern.md)
- **Rust code**: [`crates/base-methods/src/base/world_entry/methods/inventory/`](../../crates/base-methods/src/base/world_entry/methods/inventory/) — **5,585 lines** across `core/`, `grant/`, `move_/`, `ammo.rs`, `appearance.rs` + live-DB regression guards; [`crates/cell-methods/src/cell/cell_methods/inventory/`](../../crates/cell-methods/src/cell/cell_methods/inventory/) (cell-side item ops + bandolier / active slot); [`crates/game/src/inventory/`](../../crates/game/src/inventory/) (370 lines); [`crates/entity/src/inventory.rs`](../../crates/entity/src/inventory.rs) (318 lines)
- **Recent PRs**: #405 (server-side stacking + Slappack PAK override), #399 (Slappack stacks to 10), #214 (bandolier + content + UI sync), #250 (equip-from-inventory pattern), #409 (full inventory re-init bundle on respawn); since 2026-07-25: #756 (reanchor also replays hotbar, active slot, journal and `state_field` after the inventory snapshot), #791 (`useItem` refused while dead with `onErrorCode(NotLiving)`), #731 (`OnItemUse` / `remove_item` pairing lint for consumable chains), #743 (bandolier guards exercise production helpers), #697 (bandolier ammo doc correction), #609 (store methods moved to 109/110 — see §15; voids pre-2026-07-26 buyback testing); the Bank and Vault campaign's personal bank, 2026-09-27: #872 (one capacity table, `bank_slots`, the player-movable allowlist, closes #798), #921 (Banker open path and GM `.bank`), #927 (the `move_/` split) and #935 (vault moves, use and removal), #931 (debug-hub Banker and `.bankdump`), #947 (vault expansion); its organization vaults: #948 and #949 (Team and Command vault storage, open and moves), #960 (debug-hub Team and Command Bankers), #963 (org treasury), #966 (Team vault expansion)
- **Personal bank and organization vaults**: server-side done, the Bank and Vault campaign complete ([ledger](../analysis/bank-vault/README.md)). The personal bank shipped in release 1 and the organization vaults in release 2; both await the owner's UAT on the colo ([UAT checklist](../analysis/bank-vault/handoffs/session-resume.md#uat-checklist), steps 1-14 and 15-25). A Banker click or GM `.bank` opens container 17 with a vault session; every move into or out of it re-checks the session and the Banker's range; the vault starts at 40 slots and grows to 100 in +10 steps. The Team (19) and Command (20) vaults open at their own Bankers, take moves under the organization lock with the `DepositBank` and `WithdrawBank` bits, and fan every move out to the other online members; the Team vault grows from 40 to 100, paid from the treasury by its leader. Players cannot buy a step of either yet: the Expand dialog is quarantined after the #943 dialog-override crash, so only GM `.bankexpand` and `.orgvaultexpand` buy (the player-facing button is pending #967, the #943 quarantine). BV-03 also turned stack merging back on for every container (D-BV25). Mechanics: [inventory-system.md § The personal bank](../gameplay/inventory-system.md#the-personal-bank-vault) and [§ Opening a Team or Command vault](../gameplay/inventory-system.md#opening-a-team-or-command-vault); the treasury is §23.
- **In-client evidence**: 2026-09-18 colo playtest ([appendix-session-timeline.md](../analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) rows 00:03:14, 00:09:21, 00:24:38, 00:37:25) — item grant, equip-from-inventory (mission 622 completed on `item_equipped 55`), SMG grant + equip, Slappack use + consume, full inventory resync on reanchor.
- **Special ammo**: the ammo campaign ([ledger](../analysis/ammo/README.md), PRs #1040 to #1069) made special ammo a finite bag resource; §9 has its damage rows and §14 its drops. The GM tools are `.giveammo` and `.infiniteammo`. Over-cap loot and GM `GrantItem` stacks are #1045.
- **Path forward**: Durability wear (nothing lowers `durability`; only vendor repair raises it); bind-on-pickup / bind-on-equip triggers (the `bound` flag is honored but only ever set by character-creation seed rows); client smoke of the vendor → buyback loop after #609; the bank's UAT (personal and organization vaults), serving the Expand dialog once #943's crashing field is known, and the vault mail aliases (bank D-BV29).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Bag system (20 types) | CW | -- | base/world_entry/methods/inventory/core | Main, mission, equipment, bank |
| Item add/remove | CW | -- | base/world_entry/methods/inventory/grant | Live-DB regression guards; 2026-09-18 playtest grants + Slappack consume |
| Item stacking (server-side) | CW | -- | PR #405 | PR landed full server-side stack semantics |
| Equipment slots | NT | -- | base/world_entry/methods/inventory/ | Head through Artifact2. Weapon-slot equips are recorded in the 2026-09-18 playtest; no written record of armor-slot equips |
| Bandolier (4 weapon sets) | CW | -- | entity/cell_entity/bandolier.rs | Slot type_id/item_id discipline; #743 guards now call production helpers |
| Special ammo reserve | NT | -- | base-methods/.../inventory/ammo_reserve/, cell-combat/.../reload_reserve.rs | **New 2026-09-28 (ammo campaign AM-02, #1056, [ledger](../analysis/ammo/README.md)).** Special ammo is stackable bag items 9000-9014 (one per type, 500 rounds). A special reload draws `clip_size - current` rounds before the warmup (a short stack loads what is there, an empty one is refused with a line); switching type returns the unfired rounds, and a full bag keeps them loaded as the old type. Default ammo stays free. On by default (`ammo.finite_special`, D-AM11). Live-DB, concurrency and unit tested; the 15 item definitions reach the client through a cooked-data push that is not yet seen in a client for a wholly new id (AM-07) |
| Ammo-type validation (`requestAmmoChange`) | NT | -- | cell-combat/.../bandolier/ammo_change.rs | **New 2026-09-28 (AM-03, #1051).** The slot is found by the weapon's instance id (#534), the type is checked against the weapon's `ammo_types`, a missing `WeaponDef` fails closed (#602), and every refusal sends a line. Standard Pistol, Standard SMG and High Capacity SMG take the five bullet specials, 19 dart guns the ten dart specials |
| Buyback bag | NT | Vendors | base/world_entry/methods/vendor/buyback | **Demoted 2026-09-25 (CW → NT).** 12-slot bag, filled only by vendor sells. PR #609 found the store payload had been emitted on the Missionary indices 80/81 instead of 109/110, so "the vendor UI could never have worked" and earlier manual vendor testing "is void". No vendor client test since #609. Re-verified 2026-09-25 |
| Cash (naquadah) | CW | -- | base/world_entry/methods/inventory/ | addCash/removeCash; `onCashChanged` pushed by `GrantCash` (base/world_entry/methods/progression/mod.rs:427) |
| Equip-from-inventory pattern | CW | -- | docs/content/equip-from-inventory-pattern.md | Mission 622/641 worked examples (PR #250); 622 completed on equip in the 2026-09-18 playtest |
| Visual sync | NT | -- | base/world_entry_appearance/ | Equipment visual updates; other-player visibility (#737) not two-client validated |
| DB persistence | CW | -- | sqlx live-DB tests | sgw_inventory + bandolier tables |
| Item durability | KM | -- | Column exists | Value is loaded, carried on the wire (entity/inventory.rs:79) and restored by vendor repair; nothing ever wears it down |
| Item binding | IM | -- | trade/execute/swap.rs:299; vendor/data | **Corrected 2026-09-25 (KM → IM).** The `bound` column is not unused: trade aborts with `TradeAbort::BoundItemOffered` (base/world_entry/methods/trade/execute/swap.rs:299), vendor sell-price load excludes `bound = true` rows (vendor/data, test at vendor/data/tests.rs:296), stack merge skips bound rows (inventory/grant/grant_item.rs:144), and `isBound` ships on the wire. Only character-creation seed rows ever set it (base/character_create.rs:239); no bind-on-pickup/equip. Re-verified 2026-09-25 |
| Respawn re-init bundle | CW | -- | PR #409, #756 | Full inventory bundle on reanchor, recorded in the 2026-09-18 playtest (00:37:25); #756 adds hotbar / active slot / journal replay |

### 13. Missions --- IM

- **Confidence**: HIGH for framework (re-read 2026-09-25), MEDIUM for content coverage
- **Documentation**: [gameplay/mission-system.md](../gameplay/mission-system.md), [reverse-engineering/findings/mission-wire-formats.md](../reverse-engineering/findings/mission-wire-formats.md), [content/mission-chains.md](../content/mission-chains.md), [architecture/mission-pak-overrides.md](../architecture/mission-pak-overrides.md), [content/dialog-ui-client-contract.md](../content/dialog-ui-client-contract.md), [analysis/playtests/2026-09-18-colo-castle/](../analysis/playtests/2026-09-18-colo-castle/README.md)
- **Rust code**: [`crates/cell-content/src/cell/missions/`](../../crates/cell-content/src/cell/missions/mod.rs) (1,654: `lifecycle.rs` 541, `progression.rs` 713, `persist.rs` 280, `resend.rs` 87), [`cell/content/executor/mission.rs`](../../crates/cell-content/src/cell/content/executor/mission.rs) (528), [`base/world_entry/methods/missions/`](../../crates/base-methods/src/base/world_entry/methods/missions/mod.rs) (839), [`crates/entity/src/missions.rs`](../../crates/entity/src/missions.rs) (572), [`crates/resources/src/base/mission_overrides.rs`](../../crates/resources/src/base/mission_overrides.rs) (549). That is about 4,500 lines of mission code. About 30 missions are chain-authored across 11 seed files: Cellblock 622-689 + 1360, Castle 701-708, Harset 567/742/1200/1324/1326/1360/1361, SGC_W1 1559/1561/1562.
- **Recent PRs**: #214 (marsh quest loop), #250 (equip-from-inventory PAK), **#646/#648/#649/#650/#653/#655/#671 (Castle Cellblock rebuild C00-C08, GC1)**, **#659/#660/#668 (Castle 701-708)**, **#682 (H50 per-objective state survives relog, fixes #657; H52 step-activation region replay; H54 `mission_abandoned` trigger)**, **#714 (advance_step reports implicitly completed objectives, fixes #656)**, **#748 (replay `player_entered_cover` on step activation)**, **#756 (mission journal resent after same-world respawn)**, #767/#770/#771/#773 (dialog overrides with buttons and the offered-dialog set, UATPending)
- **In-client record**: the 2026-09-18 colo playtest ran the Cellblock chain (622 → 688) on two characters and Castle 701, 702, 703, 704 and 706 to completion, then reached 708 step 4462 (timeline rows 00:05-01:28 UTC). "Mission state persisted correctly throughout", including a cross-world load of 16 saved missions.
- **Path forward**: Mission reward dispatch (#310: `chosenRewards` is `UNIMPLEMENTED` and nothing sends `onMissionRewardsDisplay`; `reward_xp` / `reward_naq` are 0 on all 1,041 mission rows). Delete the `sgw_mission` row on abandon and honour `can_abandon`. Failed-objective status needs RE first (#612). Decide on hidden-mission frame suppression (#715). Mission sharing for groups; mission-gated loot filtering. Client UAT of H50 relog restore and the DU dialog packets.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Mission accept | CW | -- | cell/missions/lifecycle.rs | From NPC dialog or chain (`accept_mission`). Re-seen in client in the 2026-09-18 colo playtest (622, 638, 639, 680, 701-708 accepted on two characters). Re-verified 2026-09-25 |
| Mission tracking | CW | -- | entity/missions.rs | Steps, objectives, status. Journal is now resent after a same-world respawn (#756). Open divergence: hidden missions still send client frames (#715, client effect unknown) |
| Objective completion | CW | -- | cell/missions/progression.rs, cell/content/executor/mission.rs | Content-engine action. #714 now sends `onObjectiveUpdate` for objectives that `advance_step` completes implicitly (closed #656) |
| Step advancement | CW | -- | cell/missions/progression.rs | Content-engine action. Playtest step advances confirmed in telemetry and client |
| Mission completion | CW | -- | cell/missions/progression.rs `complete_mission_direct` | **Note corrected 2026-09-25.** Completion works in the client (playtest: 641, 686, 688, 701-706 completed). The old note "Rewards XP + naquadah" was wrong: completion flips state only, no code reads `reward_xp`/`reward_naq` (both 0 in every seed row), and there is no reward window. Playtest: "missions don't give xp". See #310 |
| Mission failure | IM | RE | entity/missions.rs `fail()` | Mission-level `fail()` is reachable only from the GM `.`-console (cell/console/mission.rs:44). `FailObjective` has no executor arm, and there is no failed-objective status or wire value (#612) |
| Mission abandon | IM | -- | cell/missions/lifecycle.rs:186 | Removes the instance and sends the removal frame; H54 (#682) fires `mission_abandoned` from all four entry points. Known gaps: `can_abandon` is never checked, the `sgw_mission` row is not deleted, and the removal send is a silent `let _ =`. Re-verified 2026-09-25 |
| DB persistence | CW | -- | cell/missions/persist.rs, base/world_entry/methods/missions/ | `sgw_mission` round trip (playtest: 16 saved missions reloaded on Castle entry). H50 (#682) now persists per-objective state, fixing the step-id-as-objective relog bug (#657, closed). The relog restore itself is not yet client-verified |
| Mission PAK overrides | CW | -- | base/mission_overrides.rs | Mid-chain step injection |
| Repeatable missions | IM | -- | cell/missions/lifecycle.rs | Repeat-cap and `can_repeat_on_fail` offer guards with unit tests; no seeded repeatable content has been exercised |
| Mission sharing | KM | Groups | cell/cell_methods/missionary.rs | `shareMission` / `shareMissionResponse` log `UNIMPLEMENTED` |
| Mission-gated loot | KM | Loot | -- | No mission filter in loot generation |

### 14. Loot --- IM

- **Confidence**: MEDIUM-HIGH (re-read 2026-09-25; logic exists and has in-client evidence, content sparse)
- **Documentation**: [gameplay/loot-system.md](../gameplay/loot-system.md), [reverse-engineering/findings/loot-generation.md](../reverse-engineering/findings/loot-generation.md)
- **Rust code**: [`crates/cell-interactions/src/cell/interactions/loot/`](../../crates/cell-interactions/src/cell/interactions/loot/) (`lootItem`, and `restore.rs`, which puts an item back on its corpse when the base refuses the grant), [`crates/cell-combat/src/cell/abilities/loot_drop.rs`](../../crates/cell-combat/src/cell/abilities/loot_drop.rs) (311 lines), [`crates/game/src/inventory/loot.rs`](../../crates/game/src/inventory/loot.rs) (prototype; `instantiate_loot_drop` is still `todo!()` at :77 and unused by the live path)
- **Recent PRs**: #446 (looter distance re-validated per `lootItem`), #491; since 2026-07-25: #649 (mission 1360 Frost's Letter accepted from the Frost loot dialog — content, not loot mechanics), #638 (`GrantCash` feedback-recipient split touches the loot cash grant)
- **In-client evidence**: 2026-09-18 colo playtest, [appendix-session-timeline.md](../analysis/playtests/2026-09-18-colo-castle/appendix-session-timeline.md) line 121: 26 lootable kills; Health Slappack rolled on 26/26 (`probability = 1`), naquadah on 17/26 (`probability = 0.8`, 5–50); tester complaint "too many slap-packs" (README line 85) and a looted Slappack used at 00:24:38.
- **Path forward**: Loot table content (3 tables / 14 loot rows in `db/resources/Loot/Seed/`, table 1 marked DEPRECATED, table 3 the stasis-room debug crate's); per-roll debug logging (playtest gap G9); per-player eligibility and group-loot modes after Groups; mission-gated loot.

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Loot table definitions | CW | -- | db/resources/Loot/ | Schema present; table 2 "Cellblock NID guard default" drove the 2026-09-18 playtest |
| Loot generation algorithm | CW | -- | cell/abilities/loot_drop.rs | **Promoted 2026-09-25 (NT → CW).** Per-item probability roll observed in-client: 26/26 at p = 1, 17/26 at p = 0.8 across 26 kills; tester received and used the drops (2026-09-18 playtest, appendix-session-timeline.md:121, :144; README.md:85). Re-verified 2026-09-25 |
| Item drops | CW | -- | cell/abilities/loot_drop.rs | Drops + transfer to inventory |
| Cash drops | NT | -- | cell/interactions/loot/mod.rs | LOOT_Cash → `CellToBaseMsg::GrantCash`. Naquadah was rolled in the 2026-09-18 playtest, but no written record that it reached the wallet |
| Special ammo drops | NT | -- | db/resources/Loot/Seed/ammo_loot.sql, ammo_dart_loot.sql | **New 2026-09-28 (AM-05, #1050, [ledger](../analysis/ammo/README.md)).** Castle NID guards drop Hollow Point (5-10 %) and veterans Armor Piercing (3 %); the Castle pre-Romney chest gives 50-75 Hollow Point, which fits the SGHC 6 SMG it also gives (#1052); the debug-hub crate gives a full stack of every special and weapons that take them. A looted stack logs `ammo_loot_dropped` (AM-12). Seed-guarded; not seen in a client |
| Loot bag take-all | CW | -- | cell/interactions/loot/ | Castle Cellblock smoke verified. Since the crafting campaign (CR-16, #932) a refused item grant (a full bag, an item no carried bag takes) goes back on the corpse with a line naming why, instead of being lost; not yet seen in a client |
| Per-player eligibility | KM | Groups | -- | No eligibility list anywhere in `crates/` (re-checked 2026-09-25). Looting is gated on distance only (PR #446) |
| Group loot modes | KM | Groups | -- | No RoundRobin/FreeForAll logic in `crates/` |
| Mission-gated loot | KM | Missions | -- | No missionId filtering in loot_drop.rs / loot.rs |
| Loot table content | KM | -- | -- | 3 loot tables, 14 loot rows seeded; one is DEPRECATED. Table 3 (2026-09-26) is the stasis-room debug crate's: four rows at probability 1, for testing cash drops and Loot All, plus the five Racial Paradigm Guides and Blueprint: Steel Plating at 0.2 (CR-16) ([content/debug-hub.md](../content/debug-hub.md)). Castle mission items are explicit `add_item` grants by design |

### 15. Stores / Vendors --- NT

- **Confidence**: HIGH for code, re-read 2026-09-25. **No vendor has ever been driven in-client on working code** (see #609).
- **Documentation**: [gameplay/inventory-system.md](../gameplay/inventory-system.md)
- **Rust code**: [`crates/base-methods/src/base/world_entry/methods/vendor/`](../../crates/base-methods/src/base/world_entry/methods/vendor/) — **7,453 lines** across `buyback/`, `paid_recharge/`, `paid_repair/`, `purchase/`, `sell/`, `data/` submodules plus `store.rs`, `repair.rs`, `recharge.rs`, `serializers.rs`
- **End-to-end smoke**: [`tools/vendor_store_smoke.sql`](../../tools/vendor_store_smoke.sql) (server-side PL/pgSQL, no client)
- **Recent PRs**: #214 (vendor sync), live-DB regression guards across each operation; since 2026-07-25: **#609** (store open/update were emitted on SGWPlayer indices 80/81 — Missionary's `onMissionUpdate`/`onStepUpdate` — and now go out on the correct 109/110; the PR states "the vendor UI could never have worked" and prior manual vendor testing "is void"), #737 (vendor emit path touched by the shared-world AoI change)
- **Content state**: `item_lists.sql` holds exactly two test lists. Harset packet H13 removed template 25's only spawn ([harset-rebuild/worknotes/H13.md](../analysis/harset-rebuild/worknotes/H13.md) lines 182–190). **Update 2026-09-26 (debug hub):** the Castle_CellBlock stasis room now spawns a vendor-only NPC, template 300, with template 25's lists ([content/debug-hub.md](../content/debug-hub.md)). Building it found that nothing ever set `NpcInteractionType::Vendor`, so a template with vendor lists and no trainer list could never open a store. Template 25 only opened one because its trainer list answered the click first. `spawn_npc_from_record_into` now derives Vendor from any `INT_Vendor*` bit. No client run yet.
- **Path forward**: First in-client smoke on post-#609 code (the stasis-room debug vendor, or `.spawn 300`: open store, buy / sell / buyback / repair / recharge) would move most rows to CW; real vendor lists and placed vendor NPCs (Harset GH2); client-initiated `repairItemRequest` (CM 40) is still a log-only stub (cell/cell_methods/inventory/item_ops.rs:229).

| Feature | Status | Blocks | Code | Evidence / Notes |
|---------|--------|--------|------|------------------|
| Buy items | NT | -- | vendor/purchase/ | Validates cash, creates item, live-DB tests |
| Sell items | NT | -- | vendor/sell/ | Validates ownership, adds cash; bound items excluded |
| Repair items | NT | -- | vendor/paid_repair/ | Cost calculation. Client `repairItemRequest` path still a stub |
| Recharge items | NT | -- | vendor/paid_recharge/ | Ammo recharge |
| Buyback | NT | -- | vendor/buyback/ | 12-slot |
| Vendor stock from DB | NT | -- | vendor/data/ | **Demoted 2026-09-25 (CW → NT).** Lists load from DB, but the stock only reaches the client via `onStoreOpen`, which was misrouted to index 80 until PR #609 (2026-07-26); #609 voids earlier client observations. Only two test lists exist. Re-verified 2026-09-25 |
| Transactional safety | IM | -- | sqlx transactions | Live-DB tests verify atomicity; open security findings in #464 (CAT-E) |
| PL/pgSQL smoke | CW | -- | tools/vendor_store_smoke.sql | End-to-end *server-side* test; not an in-client check (see Notes) |
