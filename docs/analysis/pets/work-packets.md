# Pets Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-27. Companions: [README and decisions](README.md), [evidence audit](audit.md), [session resume](handoffs/session-resume.md), [testing playbook](../../../TESTING.md), [ability-tree ledger](../ability-trees/work-packets.md) (same dispatch rules).

## Dispatch rules

- One worktree per worker under `.claude/worktrees/`, created by the Agent tool's `isolation: "worktree"` or by `bash tools/build-lane/mk-worktree.sh pets/<packet>-<slug> <name>`. Branches are `pets/<packet>-<slug>`.
- Every compiling cargo call goes through `bash tools/build-lane/lane.sh cargo <cmd> -p <crate>`. Never use `--exclusive` or `--workspace` while other campaigns build. Live-DB tests use `bash tools/build-lane/live-db-test.sh <filter>` against the worktree's own `sgw_<worktree>` database. Never reload another campaign's database.
- The toolchain is pinned (1.98.1), so local clippy is CI's clippy. After any dependency change, run `cargo hakari generate && cargo hakari manage-deps --yes` and `python tools/crate-graph/crate_graph.py --check`.
- Remove a worktree's `external` junction non-recursively (`cmd //c rmdir external`) before removing the worktree.
- Squash-merge after green CI **and** a Copilot review whose comments are all fixed or answered (D-PT14). When a PR's CI predates the latest `main`, the coordinator rebases and re-tests first.
- Shared seed files (`entity_templates.sql`, `spawnlist.sql`) and the entity/AoI creation paths are also edited by crafting (cimmeria-af, templates 310-329) and guilds (cimmeria-fa, 330-349). Pets stay inside **templates 350-369 and spawns 450-469**, and the coordinator messages both before a PR touches the AoI create path.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` is the default writer. Advisors:

- `aoi-witness-broadcast`: the pet entity lifecycle, owner-only sends, AoI intro and teardown (PT-01, PT-02).
- `npc-ai-spawn-advisor`: PT-05 and the PT-S templates.
- `combat-systems-advisor`: PT-03, PT-06, PT-08.
- `server-authority-enforcer` reviews PT-03, PT-04 and PT-07.
- `game-archaeology-specialist` owns PT-E1.
- `documentation-writer` reviews the docs each packet owes (see the [CLAUDE.md doc map](../../../CLAUDE.md)).

## Contract fixed by this ledger

Parallel packets build against these names. A worker who needs to change one raises it with the coordinator instead of renaming locally.

**Entity** (`cimmeria-entity`, PT-01): `crates/entity/src/cell_entity/pet.rs` holds `PetState` and `PetStance`, and `CellEntity` gets one new field, `pet: Option<Box<PetState>>` (`entity_struct.rs` is already over the cap, so nothing else is added there).

- `PetState` fields: `owner_id: u32`, `stance: PetStance`, `ability_list: Vec<i32>`, `toggled_off: Vec<i32>`, `transfer_xp: f32` (1.0), `stance_mask: u8` (bit per allowed stance, from `NoPassive`/`NoDefensive`/`NoAggressive`), `summon_ability_id: i32`, `last_teleport_at: Option<Instant>`, `despawn_at: Option<Instant>`.
- `PetStance { Passive = 0, Defensive = 1, Aggressive = 2 }` with `TryFrom<i8>` that rejects anything else.

**Wire** (`cimmeria-wire`, PT-01):

- `crates/wire/src/cell/client_methods/pet.rs`: `ON_PET_ABILITY_LIST`, `ON_PET_STANCE_LIST`, `ON_PET_STANCE_UPDATE` (values from PT-E1) and `build_pet_ability_list(&[i32])`, `build_pet_stance_list(&[i8])`, `build_pet_stance_update(i8)`.
- `SGWPET_CLASS_ID = 0x05` beside `SGWPLAYER_CLASS_ID`.
- `GENERICPROPERTY_PET_OWNER_ID = 5`.
- `ENTITYFLAG_PET` and the other pet flag constants.
- `NpcAoIData.pet_owner_id: Option<u32>`.

**Pets module** (`cimmeria-cell-world`, PT-01): `crates/cell-world/src/cell/pets/` with:

- `registry.rs`: `PetRegistry` on `SpaceManager` with the owner↔pet maps; `owned_pet(caller, claimed) -> Result<u32, PetReject>`; `pets_of(owner)`; `credit_recipient(attacker) -> Option<u32>` (pet → owner, player → self, NPC → None).
- `spawn.rs`: `spawn_pet_from_template(owner, template_id, summon_ability_id) -> Result<u32, PetSpawnError>`.
- `create_on_client.rs`: `pet_create_on_client_events(witness, &entity) -> Vec<CellToBaseMsg>`, owner-only.
- `teardown.rs`: `despawn_pet`, `forget_owner`, and the self-healing owner check.

**Seed** (PT-S): a new resource table `resources.pet_summons (ability_id int PRIMARY KEY, template_id int NOT NULL, max_active int NOT NULL DEFAULT 1)` in `db/resources/` (a seed file, not a `db/scripts` migration). Pet templates are 350-369: `class = 'pet'` (new `class_id_for_class` arm → 0x05), `ENTITYFLAG_Pet` set, `loot_table_id NULL`, never placed in `spawnlist`.

**Log targets**: `pets.lifecycle` (summon, despawn and reason), `pets.command` (commands and refusals), `pets.ai` (AI decisions) and `pets.credit` (kill credit and XP). The `pets=debug` `OTEL_FILTER` row covers all four. Each is added to `OTEL_FILTER` with its pinning assertion, and its `decision_outcome` values go in `docs/architecture/observability.md`.

## Dependency graph and waves

```text
Wave 0 (now, parallel)        Wave 1 (after PT-01, parallel)                Wave 2                     Owner
PT-E1 client evidence ──┐
PT-01 foundation ───────┼──► PT-02 lifecycle hooks ───────────────┐
PT-S  seed + templates ─┘    PT-03 summon via ability (needs PT-S)─┤
                             PT-04 commands + ownership guard ────┼──► PT-08 owner pet buffs ──► PT-13 close-out ─► PT-UAT
                             PT-05 pet AI: follow/stance/defend ──┤    PT-11 rest of roster
                             PT-06 kill credit + XP ──────────────┤    (Prime, Lo'taur, Straegis)
                             PT-07 UAT tooling (needs PT-S) ──────┘
Blocked:  PT-10 persistence (D-PT01)   PT-12 turrets (no client model)
```

PT-01 is the bottleneck. It is kept to "a pet exists, is introduced to its owner, and is torn down safely", with no summon, commands or AI. PT-E1 and PT-S touch no Rust that PT-01 owns. **Gate:** PT-01 may land its structure early, but its wire indices and owner-binding message stay behind PT-E1's confirmation. If PT-E1 is late, PT-01 lands with the indices under a `// PT-E1 pending` test that pins the derived values, and PT-E1 fixes them.

**Contended files** (the coordinator merges one packet at a time):

- `crates/cell-combat/src/cell/service/npc_ai/dispatch.rs`: PT-05 (the pet pre-pass hook) only. Other packets call into `npc_ai/pet/`.
- `crates/cell-combat/src/cell/abilities/damage_apply/mod.rs`: PT-06 (the `credit_recipient` seam) and PT-03 (the summon branch sits ahead of the damage pipeline, in `use_ability/`, not here). Merge PT-06 first.
- `crates/cell-world/src/cell/space_manager/aoi.rs` and `request_entity_update.rs`: PT-01 only.
- `crates/cell-methods/src/cell/cell_methods/player/social.rs` and `dispatch.rs`: PT-04 only.
- `db/resources/Entities/Seed/entity_templates.sql`: PT-S, then PT-07 (the trainer NPC), then PT-11. Pets rows only.

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** ([TESTING.md](../../../TESTING.md)):
  - wire output → byte-exact tests;
  - owner-only routing → a second-witness negative;
  - the ownership guard → a `LogCapture` negative-log test that fails when the check is removed;
  - AI → fixture tests on the log or state, not `nav_path` (the fixtures have no navmesh);
  - seed → live-DB guards (`require_db_or_skip!`, exact-sentinel cleanup).
- Every button press gets visible feedback on the first press: a rejected pet command sends `onErrorCode` or a visible refresh.
- **Telemetry (D-PT15).** A support question like "player X did Y at time T and it failed" is answerable from SigNoz alone:
  - an info span on each command/GM dispatch entrypoint (`pets.command`), and no per-handler spans inside AI ticks;
  - a debug event with `event = "..."` on every pet state transition (summon, replace, despawn with `reason`, corpse, owner-left/teleported, stance, toggle, follow/fight, leash-teleport with from/to/distance);
  - the owner's `account_id` + `player_id` (via `SpaceManager::player_identity`) and the correlators `pet_id`, `owner_id`, `template_id`, `ability_id`, `target_id` on every event;
  - `reason = "..."` on every refusal, each with a `LogCapture` test;
  - before/after values on kill credit (`xp_granted`, `transfer_xp`);
  - every target in `OTEL_FILTER` with its pinning assertion, and its values listed in `observability.md`;
  - the worknote lists every new log line.
- Each packet updates the docs it owes:
  - `docs/gameplay/pet-system.md`;
  - `docs/protocol/client-method-dispatch-table.md` (a new SGWPet table) and `docs/protocol/message-catalog.md` for wire changes;
  - `docs/architecture/abilities-and-effects-system.md` for the summon path;
  - `docs/gap-analysis.md` §28;
  - `docs/content/debug-hub.md` for the hub NPC.

## Wave 0

### PT-E1

**Status:** Integrated (#863). **Scope title:** pet client contract, closing the static gaps. **Agent:** game-archaeology-specialist (Ghidra static; any live check uses non-freezing log breakpoints only).
**Entries:** [client static RE](research/client-static-re.md) §A, §E, §H; `docs/reverse-engineering/findings/pet-restoration.md`, `pet-wire-formats.md`.
**Scope:**

1. Confirm the SGWPet client method wire indices (EntityDescription for type 5, or the method table order).
2. Find what fills `Unit.Pet1..4`: trace the unit-slot setter (the same family as the DialogSpeaker slot-17 mapping `FUN_00c67bd0`) for the pet slots, and the handler for `onEntityProperty(GENERICPROPERTY_PetOwnerId)` and the `ownerID` property. Name the exact message the server must send.
3. The `SpeedPet` consumer, if static analysis finds it.
4. Write `docs/reverse-engineering/findings/pet-client-contract.md` from the static RE plus these answers, fix the two INT8 errors in `pet-wire-formats.md` and the "idx 0/1/2" claim in `pet-restoration.md`, and add both README index rows.

**Acceptance:** each answer has an address and a confidence. If (2) cannot be closed statically, list the non-freezing capture recipe that PT-01's UAT build can use: summon through `.pet summon`, watch `Unit.Pet1` fill in the pet window.

### PT-01

**Status:** Integrated (#870). **Scope title:** the pet exists: entity, wire, registry, spawn, AoI intro, safe teardown. **Agent:** rust-gameserver-dev; advisor aoi-witness-broadcast.
**Entries:** the contract above; audit A-02..A-05, A-20..A-24, A-31, A-34.
**Scope:**

- the `PetState` / `PetStance` entity file;
- wire constants and the three builders;
- the `NpcAoIData.pet_owner_id` field, and the owner binding in the create cascade (per PT-E1);
- `PetRegistry` on `SpaceManager`;
- `spawn_pet_from_template`: class 0x05, `ENTITYFLAG_Pet`, owner faction (D-PT06), owner's level (D-PT02), stance Defensive, and `ability_list` from the template's ability set;
- the owner-only createOnClient replay at both EnteredAoI sites: `onPetAbilityList`, `onPetStanceList` (filtered by `stance_mask`), `onPetStanceUpdate`;
- the `queries.rs` class filters per A-22, each with a comment;
- `despawn_pet`, `forget_owner` called from `disconnect_entity`, and a per-tick self-healing sweep that despawns any pet whose owner is gone, dead (D-PT08) or in another space.

No summon ability, commands or AI.

**Acceptance:**

- byte-exact tests for the three pet methods and for the class-0x05 CREATE_ENTITY with the owner binding;
- a replay test: the owner gets the lists and a second witness does not (a negative);
- class-filter tests: the AI tick sees the pet, while AoE/cone/assist/respawn do not;
- teardown tests: disconnect, instanced-space teardown, and owner in another space → `LeftAoI` plus an empty registry;
- the SGWPet table in `client-method-dispatch-table.md`.

### PT-S

**Status:** Integrated (#865). **Scope title:** pet seed: the summon table, pet templates, summon VFX. **Agent:** rust-gameserver-dev; advisor npc-ai-spawn-advisor.
**Entries:** audit A-26, A-40..A-45; [content inventory](research/content-inventory.md) §1, §3.
**Scope:**

- the `resources.pet_summons` table and its loader into the startup cache (the `spawn_templates` pattern);
- template 350 "Straegis Fighter" pet: a clone of 78 (`MOB_StraegisFighter`), class `pet`, `ENTITYFLAG_Pet` and `NoPetLeveling`, a non-hostile faction, no loot, and a kit built from the Straegis mob abilities (1156 Disengage, 2847 Dissonance; 1240 Explode only if it does not kill the caster). Name 28894 if it has text, otherwise 27377 "Summoned Straegis Fighter";
- the row `2826 → 350` (the duplicates 3491/3493/3495-3497 only if they are the same summon);
- event sets 1121/1122 (Goa'uld summon source/target) on ability 2826;
- rows for Jaffa, Prime and Lo'taur (351-353) are added by PT-11.

**Acceptance:** live-DB seed guards: every `pet_summons` row points at a pet-flagged template in 350-369, every pet template is absent from `spawnlist`, and 2826 carries the event set.

## Wave 1

### PT-02

**Status:** Integrated (#892). **Scope title:** owner lifecycle hooks. **Agent:** rust-gameserver-dev; advisor aoi-witness-broadcast.
**Scope:** explicit, immediate handling on every owner path in audit A-31, on top of PT-01's self-healing sweep:

- **Despawn** on logout, death (D-PT08), cross-world respawn, gate travel, space transfer, GM travel, content transport and cross-world ring.
- **Teleport** the pet beside the owner on a same-space move (reanchor, `.goto`, content teleport, same-world ring), using `update_entity_position` plus the NPC movement fan-out and never `onPlayerTeleport`.
- **Pet death** leaves a corpse that despawns after 10 s.

**Acceptance:** one test per path class (despawn, teleport, corpse). Each checks the registry is empty or updated and that witnesses got `LeftAoI` or a position update.

### PT-03

**Status:** Review (PR #890; its dependencies PT-01, PT-S, PT-02 and PT-06 are merged). **Scope title:** summon via ability. **Agent:** rust-gameserver-dev; advisor combat-systems-advisor; review server-authority-enforcer.
**Scope:**

- A summon branch in `use_ability/`, ahead of the damage pipeline and the #444 gate, for abilities with a `pet_summons` row.
- The ability's normal warmup is the spawn timer (D-PT10: scaled by `speedPet`).
- On completion, `spawn_pet_from_template` beside the owner. A second summon replaces the current pet (D-PT04).
- The source and target VFX sequences fire through the existing ability sequence path.
- Cooldown as seeded.
- The #444 gate is not widened: a summon ability that has no `pet_summons` row still fails as today.

**Acceptance:**

- casting 2826 spawns template 350, owned by the caster;
- a second cast replaces the first pet;
- an interrupted warmup spawns nothing;
- a non-summon self-cast is still rejected (a revert guard on the gate).

### PT-04

**Status:** BlockedDependency (PT-01). **Scope title:** pet commands and the ownership guard (CAT-C-11 / #462). **Agent:** rust-gameserver-dev; review server-authority-enforcer.
**Scope:** move the three stubs into `crates/cell-methods/src/cell/cell_methods/player/pet/{mod,invoke,toggle,stance}.rs`, keeping the dispatch arm-order test.

- **Every handler** resolves `owned_pet(caller, claimed)` first. On a mismatch it sends a WARN `pets.command` with `reason`, plus visible `onErrorCode` feedback.
- **CM 88:** the ability must be in the pet's list and not toggled off. Then `handle_use_ability(pet, ability, target)` through the kill-credit wrapper, and the pet's threat is seeded on the target.
- **CM 89:** update `toggled_off` and re-send `onPetAbilityList`.
- **CM 90:** accept a stance id that is in the pet's stance list. If the id is outside `EPetStance`, treat it as a 1-based slot index into the list that was sent (A-07). Otherwise reject with feedback. On success, set the stance and send `onPetStanceUpdate`.

**Acceptance:**

- a spoof test: a command naming another player's pet, and one naming an NPC id, are both rejected with the negative log, and the test fails when the check is removed;
- a slot-index stance test;
- a toggle round trip;
- an invoke that is out of range, on cooldown, or names an unknown ability gives feedback.

### PT-05

**Status:** BlockedDependency (PT-01). **Scope title:** pet AI: follow, teleport, stances, defend, owner-relative leash. **Agent:** rust-gameserver-dev; advisors npc-ai-spawn-advisor, combat-systems-advisor.
**Scope:** `crates/cell-combat/src/cell/service/npc_ai/pet/` (`mod.rs` is the pre-pass hook from `dispatch.rs`; `owner_follow.rs`, `stance.rs`, `defend.rs`).

- **Follow the owner** (band 2-5 u). Teleport past 40 u or across a floor band, rate-limited to once per 5 s (D-PT07).
- **Re-arm Follow** after a fight, instead of going Idle.
- **Leash to the owner, never to `spawn_position`:** add a pet branch at `fight_target.rs:92,206`, or a pet case in `leash/policy.rs`.
- **Stance rules** per D-PT09. Defensive seeds threat from whatever damages the owner or the pet. Aggressive adds a pet-centred scan for hostile NPCs within 15 u, plus the owner's target.
- **Pet threat** puts the owner in combat (D-PT06).
- **Ability choice** goes through the existing selector, filtered by `toggled_off`.

**Acceptance:**

- fixture tests on state and log for follow, teleport, each stance, defend-owner and the post-fight re-arm;
- a negative test: a Passive pet does not engage when hit;
- leash never targets `spawn_position`.

### PT-06

**Status:** Integrated (#889). **Scope title:** kill credit and XP. **Agent:** rust-gameserver-dev; advisor combat-systems-advisor.
**Scope:**

- One seam: `credit_recipient`, used where `grant_kill_xp` is called and in `kill_credit.rs`.
- A pet kill credits the owner with XP × `transfer_xp` (D-PT02) and fires the owner's `EntityDeath` content events, so KillCount missions advance.
- A mob killing a pet sends no `GrantXP`, and an NPC attacker never gets `GrantXP`.
- Loot from a pet kill is owned by the owner.

**Acceptance:**

- death tests capturing `GrantXP`: owner credited, mob not credited;
- a chain-replay test where a pet kill advances a KillCount objective.

### PT-07

**Status:** BlockedDependency (PT-01; the hub NPC also needs PT-S and PT-03). **Scope title:** UAT tooling. **Agent:** rust-gameserver-dev; review server-authority-enforcer (GM gating).
**Scope:**

- **Dot console, GM-gated:**
  - `.pet summon <templateId|abilityId>`, which skips the warmup;
  - `.pet dismiss`, `.pet stance <0-2>`, `.pet info` (owner, stance, lists, AI state, distance, last teleport);
  - `.pet list`;
  - `.giveability <id>`, which reuses the `AbilityGranted` mirror and is marked non-persistent in its feedback.
- **Debug hub pet trainer** in the Castle Cellblock stasis room: template 350-369 range (e.g. 360 "Pet Trainer"), spawn 450, trainer list offering 1643-1645, 1652, 1654 and 2826. Follow the authoring traps note. Its dialog ids go above 100100, clear of #846's.
- Update `docs/content/debug-hub.md`, including its "cannot test" row.

**Acceptance:**

- GM-gating tests: a non-GM `.pet` or `.giveability` is refused with feedback;
- live-DB seed guards for the trainer NPC and its list;
- the UAT checklist in [handoffs/session-resume.md](handoffs/session-resume.md) is filled in.

## Wave 2

### PT-08

**Status:** Review (PR #920; [worknote](worknotes/pt-08.md), D-PT17 proposed). **Scope title:** owner abilities that act on pets. **Agent:** rust-gameserver-dev; advisor combat-systems-advisor.
**Scope:**

- **Holy Warrior** (2824): pet +Accuracy / −Defense.
- **To The Death** (2839): +400 Accuracy for 60 s, then the pet dies.
- **Heed Our Calling** (2852): instant summon through `speedPet`.
- **Lord's Concentration** (1650): pet interrupt resistance (needs an effect, since it has none).
- **Heal-pet** effects that target the owner's pet.

Each is an effect script or a target redirect ("the owner's pet") and gets its own test.

### PT-11

**Status:** Review (PR #918; worknote [pt-11](worknotes/pt-11.md)). **Scope title:** the rest of the Servant Lord roster (D-PT13 order). **Agent:** rust-gameserver-dev.
**Scope:**

- templates 351-353 and `pet_summons` rows for Jaffa (1643 → 351, a clone of 160, name 8087), Prime (1645, Praxis Jaffa Lieutenant look, name 28892) and Lo'taur (1644, Goa'uld servant dress, name 28891);
- their kits: ability set 4 for the Jaffa; 1654 for Prime; 1653 and 3326-3329 for Lo'taur;
- 1652 Double Blast on the Jaffa (PetTrained, via `knownPetAbilities`);
- per-template stance masks;
- a client render check of each look in UAT.

### PT-13

**Status:** BlockedDependency (all merged packets). **Scope title:** close-out. Final docs pass:

- `pet-system.md` status;
- gap-analysis §28;
- `docs/project-status.md`;
- #570 checklist, closing it;
- session-resume state.

Then comment `/release` on the last merged PR (D-PT00; from PowerShell, or with `MSYS_NO_PATHCONV=1`).

### PT-UAT

**Status:** BlockedDependency (PT-13 release). Owner UAT on the colo with the checklist in [handoffs/session-resume.md](handoffs/session-resume.md).

## Blocked

- **PT-10 persistence:** BlockedDecision (D-PT01). Only needed if the owner wants pets saved: a `db/sgw/` table, `saveToDB` on logout, re-summon on login.
- **PT-12 turrets:** blocked. The client ships no turret body or mesh (A-46). It needs a model decision, e.g. `MOB_CA_DroneTank`, or a static-mesh turret prop.
