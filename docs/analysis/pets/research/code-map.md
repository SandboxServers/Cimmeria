# Pets restoration: server code integration map

Research only. All paths and line numbers are on `origin/main` at `004bccb4` (PR #846 merged, crate split #825 in place). Nothing was edited, built or committed.

Docs read first: `docs/gameplay/pet-system.md`, `docs/reverse-engineering/findings/pet-restoration.md`, `pet-wire-formats.md`, `docs/protocol/client-method-dispatch-table.md` (SGWMob table), `docs/drafts/spec/entity-property-sync.md` App. B, `docs/architecture/services-crate-split.md`, `docs/content/debug-hub.md`, CAT-C-11, issue #570.

Crate layering (services-crate-split.md §1): `wire` <- `cell-catalog` <- `cell-world` <- `cell-combat` <- `cell-content` <- `cell-interactions` <- {`cell-methods`, `cell-console`} <- `cell`. Entity structs are in `cimmeria-entity`. Anything the AoI tick or `SpaceManager` needs must be in `cell-world` or lower.

---

## 0. Existing pet code (item 7)

There is almost none. Every hit:

| Where | What |
|---|---|
| `crates/wire/src/cell/cell_methods/player/constants.rs:26-28` | `PET_INVOKE_ABILITY=88`, `PET_ABILITY_TOGGLE=89`, `PET_CHANGE_STANCE=90` |
| `crates/wire/src/cell/dispatch/constants.rs:114-116` | `CM_PET_*` re-exports |
| `crates/cell-methods/src/cell/cell_methods/player/dispatch.rs:44` | `PET_INVOKE_ABILITY..=PET_CHANGE_STANCE => social::dispatch` (arm ordered before the `world` arm, pinned by the test at :115) |
| `crates/cell-methods/src/cell/cell_methods/player/social.rs:15-58` | Three stubs. They parse 12/9/5 bytes (matching `.def`) and log `UNIMPLEMENTED`. `_tx`/`_space_mgr` unused. The `pet_entity_id` is not validated (CAT-C-11). |
| `crates/wire/src/cell/spawn_record.rs:125-136` | `class_id_for_class` knows `spawnable`/`being`/`mob` only. Anything else maps to 0x04. The comment lists 5 = SGWPet. |
| `crates/wire/src/mercury/mod.rs:139` | Comment only (clientIndex table) |
| `crates/mercury/src/channel_bundle/idbase.rs:34-40` | `IDBASE_NPC_DEFAULT = 62` names SGWPet explicitly. SGWPet has 32 client methods, so its idbase is 62 (entity-property-sync App. B). |
| `crates/resources/src/base/resources/mod.rs:118,152` | Notes that the `pet_command` resource category (21) was never client-side |
| `db/resources/AI/Types/EPetStance.sql` | Enum type only. No table uses it. |
| `db/resources/Entities/Types/EEntityFlags.sql`, `EEntityPropertyType.sql` | Enum names only |
| `deprecated/python/cell/SGWPet.py` | `createOnClient` sends `onPetAbilityList` and `onPetStanceList`. The `onPetStanceUpdate` call is commented out. Nothing else. |
| db/sgw | No pet table, no `owner_id` column anywhere |

There is no `owner_id`, `PetOwnerId`, `ENTITYFLAG_Pet` or `SGWPET_CLASS_ID` in any Rust crate.

Values from `entities/defs/enumerations.xml`:

- `GENERICPROPERTY_PetOwnerId = 5` (line 1727)
- `ENTITYFLAG_NoPetLeveling=8`, `NoPetTargeting=16`, `DespawnOnOwnerLeash=32`, `NoPassive=64`, `NoDefensive=128`, `NoAggressive=256`, `DetectionPet=512`, `Pet=1024`, `DespawnOnLeashFromOwner=32768`, `PetUseOwnFaction=65536`, `PetWaitToDespawn=131072` (lines 1479-1493)
- `NoPassive`/`NoDefensive`/`NoAggressive` are likely the per-template stance mask that `onPetStanceList` should be built from. This is inferred, not RE-confirmed.

---

## 1. Entity classes, spawn, AoI create, method indices

### 1.1 How class/type is represented

- `CellEntity` is at `crates/entity/src/cell_entity/entity_struct.rs:32`, and the file is 747 lines (already over the 700 hard cap). Relevant fields: `class_id: u8` (:99), `is_player` (:89), `faction: u8` (:204), `entity_flags: u64` (:201), `template_id` (:175), `abilities: AbilityManager` (:107), `threat_list` (:450), `leash: LeashState` (:463), `aggro` (:492), `follow_target_id`/`follow_min_distance`/`follow_max_distance` (:591-599), `spawn_position` (:452), `respawn_secs` (:516), `loot_table_id` (:607), `witnesses` (:81, players only).
- There is no generic "owner" field. A pet needs one new field (see §9), and it should go in a new sibling file under `crates/entity/src/cell_entity/` (the directory already has `leash_state.rs`, `aggression.rs` and similar), not inline.

### 1.2 NPC spawning

- `crates/cell-world/src/cell/space_manager/spawn.rs`
  - `spawn_npc` (:19) is bare and hardcodes `class_id = 0x04` (:32).
  - `allocate_npc_id` (:57): one counter `next_npc_id`, starting at 100_000 (`space_manager/mod.rs:173,430`). Pets can share it. A separate range buys nothing, because ownership is enforced by a map, not by the id range (see §2.4).
  - `spawn_npc_from_template` (:118) builds from the `spawn_templates` cache (`mod.rs:277`), forces `respawn_secs=None`, allocates the id itself, and takes an aggression override. **This is the model for `spawn_pet_from_template`:** same shape, plus `class_id = 0x05`, `entity_flags |= ENTITYFLAG_Pet`, owner and stance, and faction from the owner or the template.
  - `spawn_npc_from_record_into` (:216) is where `e.class_id = class_id_for_class(&record.class)` is set (:225). The ability bucket falls back to `NPC_DEFAULT_ABILITY` when empty (:~305). HP comes from level (:~315).
- Stale doc: `crates/cell-console/src/cell/console/gm/spawn.rs:3-10` says "the cell has no template cache". `SpaceManager::spawn_templates` has existed since H03. A pet summon must not copy the base round-trip it describes.

### 1.3 Hardcoded `class_id == 0x04` filters (must change for 0x05)

| Site | Used by | Pet action |
|---|---|---|
| `crates/cell-world/src/cell/space_manager/queries.rs:224-233` `all_npc_entity_ids` (`== 0x04`, :228) | player AoE `abilities/dispatch/mod.rs:282`, cone `cone_aoe/geometry.rs:73`, respawn tick `cell/service/ticks/npc_respawn/mod.rs:102`, cover stance, detectors sweep, GM query | **Keep pets out.** Player AoE/cone must never hit an owned pet. The respawn tick must not respawn pets. Leaving 0x05 excluded here is correct, but say so in a comment. |
| `queries.rs:249-276` `ai_driven_npc_entity_ids` (`match class_id { 0x04 => true, 0x01 => ...}`, :258) | AI tick `npc_ai/dispatch.rs:108`, movement tick `cell/service/ticks/npc_movement.rs:94`, movement detector | **Must admit 0x05**, or the pet never thinks or moves. |
| `queries.rs:283-295` `npc_ids_in_space_of` (`== 0x04`, :294) | NA14 assist `npc_ai/assist.rs:75` | Keep pets out: a hostile mob must not recruit a pet. |
| `crates/cell-combat/src/cell/combat/threat/aggro.rs:115,209` `BEING_CLASS_ID` refusal | `generate_threat` | Not affected. Pets must take threat. |
| `crates/wire/src/mercury/aoi/create.rs:237` `class_id != 0x00` | cascade: level/target/align/faction/state/stats | 0x05 gets the combatant cascade. Correct. |

### 1.4 AoI create cascade (picking the client type, composing createOnClient)

1. Cell side: `crates/cell-world/src/cell/space_manager/aoi.rs` `compute_player_aoi` (:60)
   - It builds `NpcAoIData` from the `CellEntity` (:126-140) and pushes `CellToBaseMsg::EnteredAoI { class_id: other.class_id, npc_data, player_data, .. }` (:149).
   - `class_id` goes to the client unchanged, so setting `class_id = 0x05` on the cell entity is all that makes the client build a `GamePet`.
   - Right after, at :160-184, comes the **per-class createOnClient replay**, the SGWMob `onAggressionOverrideUpdate` push as a `WitnessEntityMethod { witness_id, entity_id, method_index, args, entity_is_player:false }`.
   - **This is the insertion point for the pet's owner-only createOnClient.** When `other.pet.owner_id == player_id`, push `onPetAbilityList` then `onPetStanceList` (Python order). Push `onPetStanceUpdate` too, because the Python stub commented it out and the client otherwise never learns a non-default stance.
2. Second EnteredAoI site, the client-driven re-emit: `crates/cell/src/cell/service/base_messages/request_entity_update.rs:95-119`.
   - It does **not** replay the aggression override today (latent gap).
   - A pet re-intro needs the same replay. Factor it as one `cell-world` helper, `pet_create_on_client_events(witness, &entity) -> Vec<CellToBaseMsg>`, and call it from both sites.
3. Base side
   - `crates/base-world-entry/src/base/world_entry/cell_dispatch/aoi_dispatch.rs` passes `class_id` through (:48,198-280).
   - `WitnessEntityMethod` rides `deferred_aoi` (`crates/base-session/src/base/deferred_aoi.rs:110`), so ordering after CREATE_ENTITY holds when both come from the same batch.
4. Wire
   - `crates/wire/src/mercury/aoi/create.rs` `compose_create_entity_base_body` (:77) writes `[eid][0xFF idAlias][class_id][0][0]`. **No BigWorld property stream is ever sent for NPCs** (and players get `propCount=0`).
   - The cascade `compose_create_entity_cascade_body` (:146) uses `cascade_idbase` = 62 for any `npc_data` (:26), which is correct for SGWPet.
   - `onEntityProperty(GENERICPROPERTY_*, value)` is already emitted for `DatabaseId` (:37, :158-175). **The owner id most likely rides `onEntityProperty(GENERICPROPERTY_PetOwnerId=5, ownerEntityId)` in this cascade.** That requires adding `pet_owner_id: Option<i32>` to `NpcAoIData` (`crates/wire/src/cell/messages/data.rs:24`).
   - This is also where issue #570 Phase 4 ("ownerID/ownerBase CELL_PUBLIC in the AoI-entry property payload") lands. Since no property payload exists, the `.def` CELL_PUBLIC `ownerID` cannot be sent the BigWorld way without new wire work. **RE question:** does `GamePet` read owner from the generic property or from a BW property?

### 1.5 Method indices for SGWPet

- The authoritative flattening rule is in `docs/protocol/client-method-dispatch-table.md:23-45`.
  - The SGWMob table (:321-358) is 0-26 shared prefix, then 27 `onAggressionOverrideUpdate` and 28 `onAggressionOverrideCleared`.
  - `SGWPet.def` has no `<Implements>` and 3 own ClientMethods, so the derived indices are **29 `onPetAbilityList(ARRAY<INT32>)`, 30 `onPetStanceList(ARRAY<INT8>)`, 31 `onPetStanceUpdate(INT8)`**.
  - That total of 32 matches entity-property-sync App. B (SGWPet = 32).
  - All three direct-encode as `0x80|idx` = `0x9D/0x9E/0x9F`.
- **Discrepancy to reconcile.** `pet-restoration.md` says "onPetAbilityList [client idx 1] / onPetStanceList [idx 0] / onPetStanceUpdate [idx 2]". That is the client's handler-registration order (`0x00d77720/0x00d779c0/0x00d77c60`), not the wire index. The work plan needs one verification step (Ghidra EntityDescription for type 5, or a live capture) and a doc fix in the same PR. Neither table lists SGWPet yet: add an "SGWPet Client Method Dispatch Table" section after the SGWMob one.
- Constants
  - SGWMob's live constants are in `crates/wire/src/mercury/mod.rs:252-253` (`method_idx::ON_AGGRESSION_OVERRIDE_*`). Agent memory flags `method_idx` as a drifted duplicate; `crates/wire/src/cell/client_methods/` is authoritative.
  - SGWPet constants belong in a new `crates/wire/src/cell/client_methods/pet.rs` (siblings: `being.rs`, `combatant.rs`, ...) with a doc comment warning that 27+ collides with SGWPlayer's Communicator range.
- ARRAY encoding is a `u32` LE count, then elements (e.g. `crates/cell-interactions/src/cell/respawn/resync.rs:151-155`). The byte shapes are:
  - `onPetAbilityList`: `[count u32][n*i32]`
  - `onPetStanceList`: `[count u32][n*i8]`
  - `onPetStanceUpdate`: `[i8]`, a 1-byte arg (2 bytes with the msg id). `pet-wire-formats.md` is wrong on both of the last two, as `pet-restoration.md` already notes, and needs fixing in the PR.
- Add `pub const SGWPET_CLASS_ID: u8 = 0x05` beside `SGWPLAYER_CLASS_ID` (`crates/wire/src/mercury/mod.rs:149`) and a `"pet" => 0x05` arm in `class_id_for_class` (`spawn_record.rs:130`). That arm matters only if pet templates are marked `class='pet'`. The summon path should set 0x05 explicitly anyway.
- Owner-only routing:
  - Use `CellToBaseMsg::WitnessEntityMethod { witness_id: owner, entity_id: pet, entity_is_player:false }`, the single-recipient form.
  - `send_entity_method` (`crates/cell-combat/src/cell/abilities/messaging.rs:39`) is wrong for this: for an NPC it goes to witnesses.
  - `send_entity_method_to_witnesses` (:98) would leak the pet's ability list to every observer.

### 1.6 SGWPet cell methods (`onOwnerDeath`, `onOwnerLeash`, `onOwnerRespawn`, `saveToDB`, `toggleAbility`, `changePetStance`, `setPetLevel`, `sendPetInfoToOwner`)

All are non-`<Exposed/>`, so they are server-internal with no wire surface. They should become plain Rust functions on the pet module, not dispatch-table entries.

---

## 2. Ability resolution, targeting, factions, threat, kill credit

### 2.1 Summoning via ability

- Effect scripts (sync layer, `cell-world`):
  - `crates/cell-world/src/cell/effects/mod.rs` has `EffectContext { source_id, target_id, effect, space_mgr }` (:74-86). **It has no `tx`.** `dispatch_by_name` is at :117.
  - The registry match is `crates/cell-world/src/cell/effects/registry.rs:4-18`, with 11 scripts and no summon.
  - `scripts.rs` is 1648 lines (over the cap), so a new script goes in its own file, following `cover_stance.rs`.
- Callers:
  - `crates/cell-combat/src/cell/abilities/damage_apply/mod.rs:349-392` runs scripts only for effects with a non-NULL `script_name`, and **only inside `apply_damage_to_target`**, i.e. after a QR roll against a target.
  - Other callers: the pulse tick `effects/pulsing/tick.rs:320`, content `cell-content/.../content/effect_apply.rs:178`, and `cover/stance.rs:75`.
- Data:
  - The summon effects exist by `name` only, with `script_name` NULL: "Spawn Mob" x8, "Spawn Jaffa" x6, "Turret Spawn" x4, "Spawn Turret Prototype" x4, "Dual Turret Spawn" x4, "Pet Death Timer", "Pet Death", "Heal Pet: Health", "Pet Accuracy: +400" (`db/resources/Effects/Seed/effects.sql`).
  - `effect_nvps.sql` has 47 lines total and no spawn/template NVPs.
  - Summon abilities by name include 1869/1955 MS019_Summon Jaffa, 2385/2832 SummonLotar, 2386/2833 SummonAshrakWarrior, 2388/2834 SummonStraegis/SystemLord, 3372/3379-3381 SummonTurret:*, and 2830 MS000_TEMPLATE_Summon.
  - #570 cites other ids (1644/1645/2825/1134/...), so a content researcher should reconcile the two lists.
- What a summon needs:
  1. A seed edit (not a migration): set `effects.script_name = 'SummonPet'` on the chosen summon effects, plus an `effect_nvps` row such as `TemplateId` (and optionally `DespawnSecs`).
  2. A `SummonPet` `EffectScript`. It can spawn synchronously into `SpaceManager` because the AoI tick introduces the entity, and the owner-only createOnClient replay (§1.4) emits the pet lists. So the missing `tx` is **not** a blocker.
  3. A call path that runs for a **self-targeted** cast. `handle_use_ability` (`use_ability/handle.rs:71`) funnels into `apply_damage_to_target`, whose target gate at :246 rejects any player-cast single-target whose target is a player (self included) or non-hostile. **Riskiest ability-side seam:** check how `target_type_id` = self casts are routed (the TODO at handle.rs:238-245 says no offensive/supportive split exists). Wire summon resolution ahead of that gate (e.g. a "summon" branch keyed on the effect's script before the damage pipeline) rather than widening the #444 gate.
- Alternative for UAT before the ability binding is recovered: a GM `.pet summon <templateId>` console command calling the same `spawn_pet_from_template`. See §5.

### 2.2 NPC ability use (pet attacking)

- NPC AI fight calls the same `handle_use_ability` entry. NPC attackers are deliberately not gated by #444 (handle.rs:233-236), and the warmup re-check has the same shape (`use_ability/warmup/tick.rs:164`). A pet attacking a faction-10 mob therefore passes.
- Ability choice is in `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs` (three-bucket model; memory `npc-range-gate-and-weapon-range-columns.md`). Pet `toggledAbilities` (the OFF list) is a filter layered on the selector. Memory notes a layered-selector pattern for adding one without touching existing tests.
- `petInvokeAbility(petId, abilityId, targetId)` should validate ownership, check that the ability is in the pet's list and not toggled off, then call `handle_use_ability(pet_id, ability_id, target_id, ...)`. Also seed the pet's threat/target so the AI keeps fighting.

### 2.3 Factions, target validity, aggro

- The combat model today is binary: players versus `faction == HOSTILE_FACTION (10)`.
  - Constant: `crates/cell-world/src/cell/combat/faction_reaction.rs:30`.
  - Gates: `use_ability/handle.rs:246-247`, `warmup/tick.rs:164`, `cone_aoe/geometry.rs:87`, `abilities/dispatch/mod.rs:290`.
- Consequences for pets:
  - A pet must **not** be faction 10, or players, including the owner, can damage it. Give it the owner's faction (`ENTITYFLAG_PetUseOwnFaction`) or 0/1, and the #444 gate then rejects owner friendly fire automatically.
  - `is_hostile_to_players` (`cell-world/src/cell/combat/aggression.rs:66`) must return false for a pet, or the Idle tick (`npc_ai/dispatch.rs:~178`) will run proximity aggro *against players*.
  - Hostile mobs will not proximity-aggro a pet. `npc_ai_idle_auto_aggro` scans the NPC's witnesses, which are players only (`npc_ai/idle_aggro.rs:46`).
  - NA14 assist recruits only when the target is a player (`npc_ai/assist.rs:69`).
  - Net effect: enemies engage a pet only after it damages them, through `generate_threat` (`combat/threat/aggro.rs:178`). That function already works for a non-player attacker: it adds the pet to the mob's `threat_list` and `enter_player_combat` no-ops for a non-player. Design decision: should pets draw proximity aggro and assist? The code does not do it today.
  - Owner combat state: when only the pet is fighting, the owner never gets `BSF_InCombat` (`threat/player_combat.rs:30,92` are player-only). Decide whether pet threat should mirror into the owner's `threatened_mobs`.
- Fight target pruning (`npc_ai/fight_target.rs`, `Dropped::{Gone,Dead,OutOfPerception}`) works on any entity id. Check that the "out of perception" test does not assume a player witness when a mob's top-threat target is a pet.

### 2.4 Ownership map (CAT-C-11 / #462)

- Required: a server-side `pet -> owner` and `owner -> pets` map. Put it on `SpaceManager`, beside `ring_transporters` (`space_manager/mod.rs:286`), plus `pet.owner_id` on the pet's `CellEntity`.
- Every 88/89/90 handler must resolve through `space_mgr.owned_pet(caller, claimed_pet_id)` and reject with a warn plus visible `onErrorCode` feedback (the button-press feedback rule) on mismatch.
- The spoofing guard needs a negative-log test (`docs/architecture/negative-logging-convention.md`).

### 2.5 Kill credit / XP (`transferXP`)

- `grant_kill_xp` (`crates/cell-combat/src/cell/abilities/death/side_effects.rs:76`) sends `GrantXP { entity_id: attacker_id }`.
  - `damage_apply/mod.rs:291-301` and `:421-430` pass `grant_xp = true` **unconditionally**.
  - **When a pet kills a mob, XP goes to the pet id**, and the base has no session for it, so the XP is lost and an error is logged.
  - **When a mob kills a pet, `GrantXP` is sent to the mob's id** (the target is not a player, so the no-op guard at :84 does not fire). This is new and wrong traffic.
  - Fix at one seam: a `SpaceManager::credit_recipient(attacker_id) -> Option<u32>` that maps pet to owner and returns `None` for NPC attackers. Apply `transferXP` (default 1.0) as the scale.
- Mission kill credit: `use_ability/kill_credit.rs:58` (`handle_use_ability_with_kill_credit`) fires `EntityDeath` content events only on player-driven paths. The NPC AI path calls bare `handle_use_ability`, so **pet kills never advance KillCount missions**. `petInvokeAbility` and the pet AI fight path need the kill-credit wrapper with the owner substituted as the credited player.
- Death of the pet itself goes through `resolve_death` (`death/mod.rs:320`), the NPC branch (`mark_npc_dead`, loot).
  - Pet templates must carry `loot_table_id = NULL`. The summon path must force `respawn_secs = None`, as `spawn_npc_from_template` does.
  - A dead pet stays a corpse until the respawn/despawn sweep. The pet module must `despawn_npc` it and clear the owner map.

---

## 3. AI: follow, leash, assist, stances

- Tick structure: `crates/cell-combat/src/cell/service/npc_ai/dispatch.rs` `npc_ai_tick` (:103).
  1. Snapshot `ai_driven_npc_entity_ids` (:108).
  2. Drop incapacitated NPCs (:109).
  3. Admit per `AiState` (:125-137).
  4. Dispatch per state (:171-183): Fighting / Leashing / Patrol / Wander / Investigating / Follow / Despawning / Submit / Error / Idle.
  - Idle priority is hostile scan, then patrol, then wander.
  - Transitions go through `set_ai_state`/`set_ai_state_on` with an `AiTransitionReason` (`crates/cell-world/src/cell/service/npc_ai/transition.rs`).
  - The `decision_outcome` vocabulary must be added to `docs/architecture/observability.md`.
- Follow: `crates/cell-combat/src/cell/service/npc_ai/follow.rs` (403 lines).
  - It keeps a distance band `follow_min/max_distance` to `follow_target_id`, paths with nav (`path_failure::UnroutedMove`), and drops to Idle when the target is gone.
  - Reusable as-is for "follow owner": set `follow_target_id = owner`.
  - Missing for pets: **teleport-to-owner** when distance exceeds the leash threshold (`onOwnerLeash`, `lastTeleportTime` rate limit; threshold unknown and blocked on x64dbg per #570).
  - Missing: **re-arm Follow after a fight** instead of Idle.
- Leash, the riskiest AI seam:
  - `npc_ai/fight_target.rs:92` and `:206` call `leash::begin_leash` (`npc_ai/leash/begin.rs:183`) when the threat list empties or the target leaves range. That walks the NPC back to `spawn_position` and sets it to evade (`generate_threat` refuses threat while Leashing, aggro.rs:195-199).
  - For a pet, "spawn position" is meaningless. It must go back to Follow(owner), and its leash distance should be measured from the owner, not the spawn.
  - Put a pet branch at those two call sites (or a `leash::policy` pet case in `crates/cell-world/src/cell/service/npc_ai/leash/policy.rs`).
- Stances (`EPetStance` 0 Passive, 1 Defensive default, 2 Aggressive). Mapping ideas:
  - Passive: never enter Fighting on its own. Ignore owner damage. Still follows. Its own threat from being hit is not acted on; decide whether a hit pet fights back. Legacy is unknown.
  - Defensive: on the owner being damaged, seed the pet's threat with the owner's attacker. The hook is `apply_damage_to_target` / `generate_threat` when the target is a player owning a pet. Also fight back when hit.
  - Aggressive: proximity scan around the **pet** for hostile (faction-10) NPCs. There is no existing NPC-vs-NPC scan to reuse (`idle_aggro` is witness/player-based). This needs a new pet-side scan.
  - Owner's target assist: if the owner has `current_target_id` (`entity_struct.rs:681`) and is in combat, engage it (Defensive/Aggressive).
- Where the pet plugs in: a `pet` pre-pass at the top of the per-NPC loop in `dispatch.rs` (after the snapshot, before `match ai_state`). It runs `pet::tick(npc_id, ...)` to handle owner checks, stance-driven engagement and teleport, then lets the ordinary Follow/Fighting handlers run. This avoids a new `AiState` variant, which would touch every exhaustive match in both crates.

---

## 4. Lifecycle hooks (despawn / move with owner)

Player teardown call sites on the cell (all async with `tx` in hand unless noted):

| Path | File:line | Pet action |
|---|---|---|
| Client disconnect / logout | `crates/cell/src/cell/service/base_messages/lifecycle.rs:283` -> `SpaceManager::disconnect_entity` (`crates/cell-world/src/cell/space_manager/entities.rs:362`) | despawn pets (tx available) |
| Base-requested destroy | `lifecycle.rs:269` `destroy_entity` | despawn |
| Owner death | `crates/cell-combat/src/cell/abilities/death/mod.rs:341` (player branch of `resolve_death`) and :231 | `onOwnerDeath`. Legacy semantics unknown: despawn, or go passive and wait for `onOwnerRespawn(aShouldDespawn)`. |
| Respawn, same world (reanchor) | `crates/cell-interactions/src/cell/respawn/mod.rs:100` `handle_respawn`, same-world branch (:117-145) | `onOwnerRespawn`: teleport the pet to the owner, or despawn |
| Respawn, cross world | `respawn/mod.rs:146-156` -> `destroy_entity` | despawn |
| Gate travel | `crates/cell-interactions/src/cell/gate_travel/mod.rs:530` | despawn (optionally resummon on arrival, needs persistence of "had pet") |
| GM / space transfer | `crates/cell-interactions/src/cell/space_transfer/mod.rs:450` | despawn, or move if same space |
| GM travel | `crates/cell-console/src/cell/console/gm/travel.rs:196` | same |
| Content transport | `crates/cell-content/src/cell/content/executor/transport.rs:137` (and `teleport` :34) | same |
| Ring transport cross-world | `crates/cell-content/src/cell/ring_transport/dispatch.rs:163` | same. A same-world ring trip moves the owner, and the follow tick must teleport the pet (distance > leash). |
| Instanced-space teardown | `entities.rs:~170-186`: the last player leaving an instanced space runs `destroy_space` | pets die with the space; the owner map must be scrubbed |

- `destroy_entity` (`entities.rs:110`) is **sync with no `tx`**. It already queues ring cleanup (`ring_transporters.note_player_gone`, :157) for exactly that reason.
- `despawn_npc` (`entities.rs:250`) is the correct observer-visible removal: immediate `LeftAoI` plus a witness scrub (memory `destroy-entity-vs-despawn-npc.md`).
- Recommended shape, one choke point instead of eleven call-site edits:
  - (a) In `destroy_entity`, when the entity is a player owning pets, record them in a `pets.pending_owner_gone` queue.
  - (b) In `disconnect_entity`, call an async `pets::forget_owner(owner, tx, self)` that despawns immediately, mirroring `ring_transport::forget_player` (:385).
  - (c) Make the pet tick self-healing: if `owner_id` is missing, in another space, or dead with `DespawnOnOwner*` semantics, `despawn_npc` the pet.
  - (c) alone covers every teardown path within one AI tick. (a)/(b) make the common paths immediate.
- Same-space teleports (`.goto`, content `teleport`, reanchor) do not destroy the owner. The follow tick's teleport-when-far rule handles them. Use `update_entity_position` (`entities.rs:461`) plus the NPC movement fan-out, never `onPlayerTeleport` (movement-teleport advisor territory).
- Deferred-state scrubs `destroy_entity` already performs on the pet id (`pending_content_actions`, `pending_health_below`, detectors, throttles) work for pets unchanged.

---

## 5. Commands / UAT tooling

- Native GM cell methods (`SGWGmPlayer`, index 109+): `crates/cell-console/src/cell/console/gm/` (`give.rs`, `spawn.rs` `gmSpawnByCmd`, `world.rs` `.despawn` at :225, `feedback.rs` `send_gm_feedback`). These only cover native indices. Pets have no native GM method.
- Dot console (GM-gated, #523): registry at `crates/cell-console/src/cell/console/registry/commands/*.rs` (spawn, stats, travel, progression, ...) with handlers in `console/*.rs`. **Add `registry/commands/pet.rs` and a handler (`console/pet.rs`)** for `.pet summon <templateId>`, `.pet dismiss`, `.pet stance <0-2>` and `.pet info`, calling the same `cell-world` primitives the ability path uses. Deviating from legacy is allowed (memory: GM intent over legacy parity; mark the deviation).
- PR #848 `/gmgivetrainingpoints` (cell method 137) is **OPEN, not merged** (`gm/give-training-points`).
- Granting an ability for testing:
  - No GM "give ability" command exists.
  - The only grant path is trainer purchase: `crates/cell-methods/src/cell/cell_methods/player/vendor/train.rs` -> base `crates/base-methods/src/base/world_entry/methods/progression/train_ability.rs:235` (`AbilityGranted`) -> cell `crates/cell/src/cell/service/base_messages/ability_granted.rs:37` (`abilities.add_ability` + `onKnownAbilitiesUpdate`).
  - There is no content-engine `grant_ability` action.
  - A trainer only offers ability-tree nodes of the player's archetype (`trainer_abilities`, `template_trainer_lists`; `space_manager/mod.rs:248,254`). Summon abilities must be tree nodes, or a new trainer list must be seeded, before a tester can buy them.
  - Cheaper UAT: `.pet summon` plus a `.giveability <id>` dot command that reuses the `AbilityGranted` mirror (it persists nothing unless routed through base).
- Debug hub (PR #846, merged)
  - `docs/content/debug-hub.md`: World 12 Castle_CellBlock, Region1. Templates 300-304, spawns 400-404, chains 7001-7005.
  - The "cannot test" table lists Pets (gap-analysis §28). **Update that row when the pet NPC lands.**
  - Traps (`.claude/agent-memory/rust-gameserver-dev/debug-hub-npc-authoring-traps.md`):
    - Vendor interaction derives from `INT_Vendor*` at spawn only.
    - Ability set 4 is not harmless; set 6 = `[710]`.
    - `system_message` is log-only; use `npc_bark`.
    - New dialogs need `DIALOG_OVERRIDES` plus pinned-id test edits, with screen ids >= 200000.
    - `cimmeria-cell-methods` has no `sqlx`.
    - Name monikers must be existing PAK ids.
  - Pet reservations: templates 350-369, spawns 450-469. A pet trainer NPC is a template with a trainer list containing the seeded summon abilities (`template_trainer_lists`). Pet *creature* templates (350+) should be `class='mob'` with `ENTITYFLAG_Pet` in `flags`, or a new `'pet'` class value. They are not placed in `spawnlist`; the summon spawns them.
  - The Straegis line is the only fully wired template/model set.

---

## 6. Persistence touchpoints (locate only)

- Player load (base): `crates/base-methods/src/base/world_entry/methods/player_load/core/player_data.rs:63` (`FROM sgw_player`), `player_load/core/inventory_items.rs`, `player_load/meta.rs`.
- Cell-side hydrate pattern: `crates/cell/src/cell/service/base_messages/player_init/` (mission_restore per memory `mission-persist-hydrate-roundtrip.md`).
- Mission persist precedent: cell `crates/cell-content/src/cell/missions/persist.rs:47,92` builds `CellToBaseMsg`; base `crates/base-methods/src/base/world_entry/methods/missions/mod.rs:32,110` (`sgw_mission` UPSERT).
- Logout position persist: `crates/base-world-entry/src/base/world_entry/cell_dispatch/position.rs:37` `persist_position`.
- Gate-travel arrival persist: `crates/base-world-entry/src/base/world_entry/gate_travel/persist_arrival.rs:68`.
- No pet table in `db/sgw/` or `db/database.sql` (only the enum include at database.sql:27). A new table would be a `db/sgw/` schema file (seed rule; ask before any `db/scripts` migration).

---

## 7. Test infrastructure

| Need | Existing harness | Anchor |
|---|---|---|
| Wire bytes of pet client methods + cascade with `PetOwnerId` | wire-format tests in `crates/wire/src/mercury/aoi/tests.rs`, create.rs `mod tests` (:~400) | assert `0x9D/0x9E/0x9F` + arg bytes, and CREATE_ENTITY class byte 0x05 |
| AoI replay to owner only (not other witnesses) | `crates/cell-world/src/cell/space_manager/tests/aoi.rs:372` `aoi_entry_replays_active_aggression_override` (spawn + `compute_aoi_changes` + filter `WitnessEntityMethod`) | the exact template; add a second-witness negative |
| Ownership spoof guard | cell-methods dispatch tests (`player/dispatch.rs:115` pattern) + `LogCapture` negative log | must fail when the check is removed |
| AI (follow owner, stance, leash-to-owner) | `crates/cell-combat/src/test_fixtures/` (`npc_detectors.rs`, `npc_surrender.rs`); memory `npc-ai-fight-test-fixtures.md`: `make_ai_fixture` has no navmesh, so assert the log, not `nav_path` | |
| Kill credit to owner / no XP to NPC | death tests `resolve_death_for_test` (`death/mod.rs:479`), capture `GrantXP` on the mpsc | |
| Lifecycle teardown | `space_manager/tests/*` + `aoi_churn_smoke.rs:175` (disconnect); `despawn_npc` observer-count return | |
| Summon via content/ability end to end | chain replay `crates/cell-content/src/cell/content/chain_replay_tests/` (e.g. `debug_hub.rs`) | |
| Seed guards (pet templates 350-369, trainer list, effect script_name/NVP) | live-DB `crates/cell-catalog/src/cell/spawner/tests/live_db_debug_hub.rs` pattern, `require_db_or_skip!` | |
| Real client-shaped session | `crates/wireclient` (`bundle.rs:180-217` decodes CREATE_ENTITY class_id), `docs/architecture/wireclient.md` | assert a class-5 create reaches the owner |

---

## 8. Recommended module layout (fits the crate split and file-organization rules)

- `cimmeria-entity`
  - `crates/entity/src/cell_entity/pet.rs`: `PetState { owner_id, stance: PetStance, ability_list, toggled_off, transfer_xp, despawn_at, last_teleport_at, ability_to_resolve }`, and `PetStance` with `TryFrom<i8>` (reject unknown values).
  - One field `pet: Option<Box<PetState>>` on `CellEntity`, keeping the addition to `entity_struct.rs` to one line.
- `cimmeria-wire`
  - `crates/wire/src/cell/client_methods/pet.rs`: indices 29/30/31 and the three arg serializers.
  - `SGWPET_CLASS_ID` in `mercury/mod.rs`.
  - `GENERICPROPERTY_PET_OWNER_ID` and `ENTITYFLAG_*` pet constants.
  - `NpcAoIData.pet_owner_id`.
  - The cascade emits `onEntityProperty(5, owner)`.
- `cimmeria-cell-world`: new directory `crates/cell-world/src/cell/pets/`
  - `registry.rs`: owner<->pet maps, `owned_pet(caller, claimed)`, `credit_recipient`.
  - `spawn.rs`: `spawn_pet_from_template`.
  - `create_on_client.rs`: owner-only replay events, called from `space_manager/aoi.rs:~184` and `request_entity_update.rs`.
  - `teardown.rs`: the `note_owner_gone` queue plus async `forget_owner`.
  - Also: `effects/pet_summon.rs` (`SummonPet` script) plus one registry arm, and the `ai_driven_npc_entity_ids` 0x05 admit.
- `cimmeria-cell-combat`: `crates/cell-combat/src/cell/service/npc_ai/pet/`
  - `mod.rs`: pre-pass hook from `dispatch.rs`.
  - `owner_follow.rs`: follow plus teleport.
  - `stance.rs`: engagement rules.
  - `defend.rs`: owner-attacked hook.
  - Plus: a pet case at `fight_target.rs:92/206`, pet->owner XP in `death/side_effects.rs:76`, and kill credit for pet kills.
- `cimmeria-cell-methods`: `crates/cell-methods/src/cell/cell_methods/player/pet/` (`mod.rs`, `invoke.rs`, `toggle.rs`, `stance.rs`). Remove the three stubs from `social.rs` and repoint the `dispatch.rs:44` arm. Keep the arm-ordering test.
- `cimmeria-cell-console`: `registry/commands/pet.rs` plus `console/pet.rs`.
- `cimmeria-cell`: optional `service/ticks/pet_despawn.rs` for timers and the owner-gone queue drain, if it is not folded into the AI pre-pass.
- Docs in the same PRs:
  - SGWPet table in `docs/protocol/client-method-dispatch-table.md`
  - fix `pet-wire-formats.md` and the idx claim in `pet-restoration.md`
  - `docs/gameplay/pet-system.md` status
  - `docs/architecture/abilities-and-effects-system.md` (new script)
  - observability outcome vocabulary
  - the debug-hub "cannot test" row
  - gap-analysis §28

---

## 9. Riskiest integration points (ranked)

1. **Kill credit / XP attribution.** `grant_xp = true` unconditionally (`damage_apply/mod.rs:298,426`). A pet kill sends XP to the pet id (lost). A mob killing a pet sends XP to the mob id. The kill-credit wrapper is skipped on NPC paths, so pet kills never advance missions. This needs a single `credit_recipient` seam, or it breaks silently.
2. **Hardcoded `class_id == 0x04`** (`queries.rs:228,258,294`). The AI and movement ticks must admit 0x05, while AoE, cone, respawn and assist must keep excluding it. A blanket "treat 0x05 like 0x04" change lets players AoE their own pets and lets the respawn tick resurrect pets.
3. **Binary faction model.** Pets must be non-hostile to players but valid targets for hostile NPCs. Hostile NPCs have no NPC-vs-NPC proximity aggro or assist (witness/player-only scans). Owner combat state does not mirror pet combat.
4. **Leash semantics.** `begin_leash` returns an NPC to `spawn_position` and makes it evade. For a pet this strands it at the summon point and makes it immune to threat.
5. **Teardown fan-out.** 11 owner-teardown paths (§4), and `destroy_entity` has no `tx`. Without the self-healing tick, a pet is orphaned in the world on any missed path.
6. **Wire uncertainty.**
   - Pet client-method indices (29/30/31 derived vs the "idx 0/1/2" in the findings doc).
   - How the client learns `ownerID`: there is no BW property stream, and `GENERICPROPERTY_PetOwnerId` via `onEntityProperty` is inferred.
   - Stance-list element meaning.
   - Leash threshold and poll interval.
   - All need Ghidra/x64dbg before the wire PR, per the bible rule.
7. **Summon binding and the self-target cast path.**
   - Summon effects have no `script_name` and no template NVP, so a seed edit is required.
   - Script dispatch only runs inside `apply_damage_to_target` behind the #444 hostile-target gate, which a self-cast summon fails.
   - `EffectContext` has no `tx`. That is fine if AoI does the intro.
8. **Ordering.** The owner-only lists must follow CREATE_ENTITY. Emitting them in the same AoI batch right after `EnteredAoI` (as the aggression replay does) is safe. Emitting them from the summon handler directly races the AoI tick and the base `deferred_aoi` gates.
