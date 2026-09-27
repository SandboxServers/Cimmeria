---
title: "Pet System"
type: reference
audience: engineers
last_updated: 2026-09-26
---

# Pet System

> **Last updated**: 2026-09-27
> **Status**: ~25%. Engine support, content and client are complete. The server can now spawn an owned pet, introduce it to its owner, tear it down, and keep it tied to its owner on every owner lifecycle path (pets campaign PT-01, PT-02); summoning by ability, pet commands and pet AI are still missing (tracked in #570, ledger `docs/analysis/pets/`). Findings: [`reverse-engineering/findings/pet-restoration.md`](../reverse-engineering/findings/pet-restoration.md).

## Overview

The pet system allows players to summon and control companion NPCs that fight alongside them. Pets extend the `SGWMob` entity with owner tracking, ability management (including toggling abilities on/off), stance control, leveling, and despawn timers. Pets respond to owner events (death, leash, respawn) and can resolve abilities on spawn.

The `SGWPet` entity is defined in `entities/defs/SGWPet.def` (parent: `SGWMob`). The Python script `deprecated/python/cell/SGWPet.py` contains only stub initialization for ability and stance lists.

## Implementation Status

The foundation is in (PT-01). The pieces:

- **Pet state.** `crates/entity/src/cell_entity/pet.rs` holds `PetState`: owner, stance, ability list, toggled-off list, `transfer_xp` and stance mask. It also holds `PetStance`. A pet is an ordinary NPC `CellEntity` with `pet: Some(..)` and wire class `SGWPet` (0x05).
- **Wire contract.** `crates/wire/src/cell/client_methods/pet.rs` has client methods 29/30/31, `GENERICPROPERTY_PET_OWNER_ID = 5`, the pet `EEntityFlags` bits and the three argument builders. The indices and the owner binding are confirmed against the client in [`pet-client-contract.md`](../reverse-engineering/findings/pet-client-contract.md).
- **Pets module.** `crates/cell-world/src/cell/pets/` has four parts:
  - `PetRegistry`, the ownership source of truth, with `SpaceManager::owned_pet` and `SpaceManager::credit_recipient`;
  - `SpaceManager::spawn_pet_from_template`;
  - the owner-only list replay on AoI entry;
  - teardown.

How a spawned pet behaves:

- **Introduction.** Every witness gets the pet's CREATE_ENTITY (class 0x05) and a cascade that carries `ENTITYFLAG_Pet` and `onEntityProperty(PetOwnerId, owner)`. Only the owner gets `onPetAbilityList` and `onPetStanceList`, plus `onPetStanceUpdate` when the stance is not the default. The owner's client binds the pet into `Unit.Pet1..4` from the flag, the owner property and the stance list. The replay runs on the AoI tick and on the client's `requestEntityUpdate`.
- **Spawn values.** The pet takes the owner's faction (D-PT06) and the owner's level (D-PT02, unless the template sets `ENTITYFLAG_NoPetLeveling`). It starts Defensive. It has no loot, respawn, patrol, wander, cover or tag.
- **Queries.** The AI and movement ticks include pets. Player AoE, cone, the respawn tick and the NA14 assist fan-out leave them out.
- **Teardown and owner lifecycle.** See [Owner lifecycle](#owner-lifecycle) below. Log target: `pets.lifecycle`.

### Owner lifecycle

A pet lives exactly as long as its owner holds it in one space (D-PT01: pets are per session and are re-summoned after any trip). Two calls in `crates/cell-world/src/cell/pets/owner_hooks.rs` carry the rules, and every owner path calls one of them (PT-02, audit A-31):

| Owner event | Call site | What happens to the pet |
|---|---|---|
| Logout / client disconnect | `SpaceManager::disconnect_entity` (via `forget_owner`) | Despawned at once |
| Base `DestroyEntity` | `service/base_messages/lifecycle.rs` `flush_and_destroy` | Despawned before the owner's destroy |
| Owner death | `abilities::death::resolve_death` (player branch) | Despawned at once (D-PT08); the owner sees it go |
| Cross-world respawn | `respawn/mod.rs` | Despawned before the owner's destroy, once the `GateTravel` send is confirmed |
| Stargate travel | `gate_travel/mod.rs` | Despawned after the `GateTravel` send is confirmed |
| GM / console transfer to another space | `space_transfer/mod.rs`, `gm/travel.rs` (`gmGotoLocation`) | Despawned; a rejected transfer keeps it |
| Content cross-world teleport | `content/executor/transport.rs` | Despawned once the `GateTravel` send is confirmed |
| Cross-world ring | `ring_transport/dispatch.rs` (`TeleportCrossWorld`) | Despawned once the `GateTravel` send is confirmed |
| Same-world respawn (a GM respawn of a living owner) | `respawn/mod.rs`, after `ReanchorPlayer` | Moved beside the owner, only once the reanchor is sent |
| `.goto` / `.summon` / `.gotolocation` in the same space, `.location`, `gmGoto`, `gmGotoXYZ`, `gmSummon` | `console/travel`, `console/placement.rs`, `gm/travel.rs`, after `TeleportPlayer` | Moved beside the owner |
| Content `teleport` action | `content/executor/transport.rs`, after `TeleportPlayer` | Moved beside the owner, only once the owner's snap is sent |
| Same-world ring | `ring_transport/dispatch.rs`, at `ShowPlayer` | Moved beside the owner when the owner reappears at the destination, not while the owner is still hidden. Only if the ring's `TeleportPlayer` really went out: an aborted or failed trip leaves the pet where it is, and an abort after the move still brings it |
| Instanced space torn down | `destroy_space` | Removed with the space; the registry is scrubbed |

- **Despawn** (`on_owner_left`) goes through `despawn_npc`, so every witness gets `LeftAoI` and the witness sets are scrubbed. On travel, disconnect and base destroy the owner itself gets no `LeftAoI`: its client is about to be reset (`RESET_ENTITIES`) or is closing, and a leave queued behind the `GateTravel` would reach the new world's view. On owner death the owner does get it.
- **Move** (`on_owner_teleported`) puts each live pet 2 u behind the owner's new spot, walked there along the navmesh from the owner's feet so it stops at walls and stands on the floor. The pet is stopped and faces the owner's heading, `last_teleport_at` is stamped (the clock the PT-05 follow teleport rate-limits on), and each witness gets an `EntityMoved`. A pet is an NPC, so it never gets `TeleportPlayer` / `onPlayerTeleport`. A dead pet is left where it fell.
- **Only after the owner's move is sent.** Every hook runs after the owner's own `TeleportPlayer`, `ReanchorPlayer` or `GateTravel` has been sent. If that send fails, the owner stays where it is (a cross-world path does not tear it out of its space) and so does its pet.
- **Ownership is the summoner, not the id.** Entity ids are reused, so the hooks decide ownership with the identity captured when the pet was summoned (`PetRegistry::summoner_matches`), not the bare owner id. A player given a destroyed owner's id is not that pet's owner: a teleport never pulls such a pet after it, and the pet is despawned instead (`teleport_skipped reason=owner_identity_mismatch`).
- **Pet death.** A pet that dies stays as a corpse, then despawns 10 s after the pet sweep first sees it dead (`PET_CORPSE_DESPAWN`, D-PT08). It never respawns as an NPC.
- **Backstop.** `pet_owner_sweep` runs every AoI tick and despawns any pet whose owner is gone, dead or in another space, so a future owner path that forgets the hooks costs at most one tick. A source-scan test (`every_owner_travel_site_calls_the_pet_hooks`) fails when a cell file sends `GateTravel` without `on_owner_left`, or `TeleportPlayer` without `on_owner_teleported`.
- The `SGWPet.def` cell methods `onOwnerDeath`, `onOwnerLeash` and `onOwnerRespawn` are not called: in our server the cell owns both the owner and the pet, so the hooks run directly.

Nothing spawns a pet yet except code and tests. Summon by ability is PT-03 and the `.pet` console is PT-07. The table records what the entity definitions provide and what the server does with them.

| Feature | Status | Notes |
|---------|--------|-------|
| Pet entity definition | DONE | Full property and method set defined |
| Owner tracking | DONE (PT-01) | `PetRegistry` on the cell. `ownerID` reaches the client as `onEntityProperty(GENERICPROPERTY_PetOwnerId)` in the create cascade |
| Ability list | DONE (PT-01) | `onPetAbilityList` to the owner only, on AoI entry |
| Stance list | DONE (PT-01) | `onPetStanceList` to the owner only, filtered by `ENTITYFLAG_NoPassive` / `NoDefensive` / `NoAggressive` |
| Spawn and teardown | DONE (PT-01, PT-02) | `spawn_pet_from_template`. Despawn on every owner departure, move beside the owner on a same-space teleport; see [Owner lifecycle](#owner-lifecycle) |
| Ability toggling | STUB | `toggleAbility` with on/off flag |
| Stance changing | STUB | `changePetStance` with `onPetStanceUpdate` |
| Pet leveling | STUB | `setPetLevel` defined |
| Owner death response | DONE (PT-02) | The pet despawns when its owner dies (D-PT08), from `resolve_death`; the `onOwnerDeath` cell method itself is unused |
| Owner leash response | STUB | `onOwnerLeash` cell method |
| Owner respawn response | DONE (PT-02) | Cross-world respawn despawns the pet; a same-world respawn moves it beside the owner. `onOwnerRespawn` itself is unused |
| Despawn timer | DONE (PT-02) | `PetState::despawn_at`: a dead pet's corpse despawns after 10 s |
| Ability on spawn | DEFINED | `abilityToResolve`, `abilityInformation` |
| XP transfer | DEFINED | `transferXP` float property |
| Position tracking | DEFINED | `ownerLastPosition`, `petLastPosition`, `lastOwnerPositionCheck` |
| Pet AI | NOT IMPL | No AI behavior scripts |
| Pet persistence | STUB | `saveToDB` defined but no save logic |

## Entity Definition (SGWPet.def)

**Parent**: `SGWMob` (inherits all mob properties, combat, ability manager, etc.)

### Properties

| Property | Type | Flags | Purpose |
|----------|------|-------|---------|
| `ownerID` | INT32 | CELL_PUBLIC | Entity ID of pet owner |
| `ownerBase` | MAILBOX | CELL_PUBLIC | Base mailbox of owner |
| `transferXP` | FLOAT | CELL_PRIVATE | XP transfer ratio (default 1.0) |
| `petDespawnTimerId` | CONTROLLER_ID | CELL_PRIVATE | Despawn countdown timer |
| `abilityToResolve` | INT32 | CELL_PRIVATE | Ability to use on spawn |
| `abilityInformation` | PYTHON | CELL_PRIVATE | Runtime params for spawn ability |
| `toggledAbilities` | ARRAY\<INT32\> | CELL_PRIVATE | Abilities toggled OFF |
| `lastOwnerPositionCheck` | FLOAT | CELL_PRIVATE | Last owner distance check time |
| `lastTeleportTime` | FLOAT | CELL_PRIVATE | Last teleport-to-owner time |
| `ownerLastPosition` | VECTOR3 | CELL_PRIVATE | Owner position cache |
| `petLastPosition` | VECTOR3 | CELL_PRIVATE | Pet position cache |
| `petStance` | INT8 | CELL_PRIVATE | Current stance (default 1) |

### Client Methods (Server -> Client)

| Method | Args | Purpose |
|--------|------|---------|
| `onPetAbilityList` | ARRAY\<INT32\> | Send pet's ability IDs to owner |
| `onPetStanceList` | ARRAY\<INT8\> | Send available stances to owner |
| `onPetStanceUpdate` | INT8 stance | Notify stance change |

### Cell Methods

| Method | Args | Purpose |
|--------|------|---------|
| `onOwnerDeath` | (none) | Owner died -- despawn or go passive |
| `onOwnerLeash` | (none) | Owner moved too far -- leash pet |
| `onOwnerRespawn` | shouldDespawn (INT8) | Owner respawned |
| `saveToDB` | playerDbId (INT32) | Persist pet state |
| `toggleAbility` | abilityId (INT32), onOff (INT8) | Toggle ability active state |
| `changePetStance` | stance (INT8) | Change pet behavior stance |
| `setPetLevel` | level (INT8) | Set pet level |
| `sendPetInfoToOwner` | ownerMailbox (MAILBOX), ownerPetAbilities (ARRAY\<INT32\>) | Send abilities to owner |

## Pet Stance System

Stances control pet AI behavior mode. The `petStance` property defaults to 1.
Confirmed values (from `db/resources/AI/Types/EPetStance.sql`):

| Value | Stance | Notes |
|-------|--------|-------|
| 0 | Passive | Won't engage |
| 1 | Defensive | Default — fights when owner/itself is attacked |
| 2 | Aggressive | Engages on sight |

## Pet Ability Toggling

The `toggledAbilities` array tracks abilities that the player has turned OFF. When the pet AI selects abilities to use, it should skip any ability whose ID is in this list.

## Data References

- **Parent entity**: `SGWMob` (inherits all mob combat systems)
- **Enumerations**: `EPetStance` — `PET_STANCE_Passive` / `_Defensive` / `_Aggressive`, shipped in `db/resources/AI/Types/EPetStance.sql`
- **Entity flags**: `ENTITYFLAG_Pet`, `ENTITYFLAG_DetectionPet`, `ENTITYFLAG_PetUseOwnFaction`, `ENTITYFLAG_PetWaitToDespawn`, `ENTITYFLAG_NoPetLeveling`, `ENTITYFLAG_NoPetTargeting` in `db/resources/Entities/Types/EEntityFlags.sql`
- **Database**: no pet persistence table exists yet — `saveToDB` has no schema behind it

## RE Priorities

1. **Pet AI** - Behavior tree for pet combat (stance-driven)
2. **Pet summoning** - How pets are created (from items? abilities?)
3. **Pet persistence** - `saveToDB` schema and what is saved
4. **Leash distance** - How `lastOwnerPositionCheck` triggers `onOwnerLeash`
5. **Spawn ability** - How `abilityToResolve` is used when pet spawns

## What pets are (overview)

Pets are summoned combat companions you command. The roster is Goa'uld-themed, and
each pet type has its own authored ability kit:

- **Jaffa** — *Double Blast*
- **Lo'taur** — *Heal Health* (a healer pet)
- **Prime** (a First Prime) — *Focus Degeneration*
- **Ashrak** (Goa'uld assassin) — a full dagger move-set: *Back Slash, Onslaught,
  Paralyze, Crippling Slash, Dervish, Decimation Wound, Double Slash, Inevitable
  End, Prolong Agony, Assassin*
- **Straegis** (enemy line) — *Disengage, Dissonance, Explode*
- **Turret** (deployable) — *Burst, Cone Attack, AOE Attack, Enhance (Shield /
  Contamination Damage), Repair (Full / Restoration)*, plus *Dual Turrets* and
  *Prototype* summon variants
- **System Lord** summon

Player abilities also buff pets: *Lord's Concentration* (interrupt resist to **all**
pets), *Defend Your God*, *Holy Warrior*, *To The Death*, *Heed Our Calling*. About
**65** summon/command/buff abilities are authored in `db/resources/Abilities/Seed/abilities.sql`.

The client supports three stances and targeting up to **6 party members' pets** at once.

## Built vs. concept

Pets are a real, built-out system at every layer except the original server's lifecycle:

- **Engine — first-class.** The entity flag set (`db/resources/Entities/Types/EEntityFlags.sql`)
  includes `ENTITYFLAG_Pet`, `ENTITYFLAG_DetectionPet`, `ENTITYFLAG_PetUseOwnFaction`,
  `ENTITYFLAG_PetWaitToDespawn`, `ENTITYFLAG_NoPetLeveling`, `ENTITYFLAG_NoPetTargeting`.
  Ownership is a `GENERICPROPERTY_PetOwnerId` entity property, and there's a
  `RESOURCE_PetCommand` resource type. A mob *becomes* a pet by setting the Pet flag + owner id.
- **Content — authored.** Summon abilities and per-pet ability kits are present.
- **Client — complete.** The 2009 binary has a full `GamePet` class, the stance/command/
  ability UI, and party-pet targeting (Ghidra-confirmed).
- **Original server — stubbed.** Python `SGWPet` only sent the ability/stance lists on
  spawn; summon/despawn/command/follow logic was never finished. Our restoration (#570)
  is therefore greenfield on the server.

## Models

We have a large model library, in two styles:

- **Dedicated creature meshes** — e.g. the Straegis line (`MOB_StraegisBeacon`,
  `MOB_StraegisTitan`, `MOB_StraegisFighter`) and `MOB_AncientDrone`.
- **Jaffa "kit" models** — every Jaffa shares one base body (`BS_JaffaMale`) and gets
  its look from swappable **armor component sets**, so a few base meshes yield dozens of
  variants: Standard (`AR_J_Standard.*`), Eagle (`AR_J_Eagle.*`), Praxis (`AR_J_Praxis.*`),
  plus Bull/Cat/Cobra/Croc/Demon/Dragon/Falcon/Horse/Hyena/Jackal/Mayan/Morrigan/Naga/Ra/
  Svarog/Tiki/Viking — full **Female** sets — and the **Unas 1–6** beasts.

Model *references* live in `db/resources/Entities/Seed/entity_templates.sql` (133 mob
templates); the model *binaries* ship in the cooked client art (available via the game
cache; not in git).

## How summoning works — and the one real gap

Using a summon ability spawns a mob, flags it as a Pet, and stamps the caster's
`PetOwnerId`; the player then commands it via the stance/ability UI.

**The unresolved binding:** summoning runs through a generic **"Spawn Mob" effect** that
carries **no template id** in the data — *which* creature it spawns was decided by that
effect's *script*. Tellingly, the seed has **no dedicated `Lo'taur` / `Prime` / `Ashrak` /
`Turret` entity templates** by name, even though their summon + command abilities are fully
authored. (The **Straegis** line is the one fully-wired example: abilities **and** templates
**and** models all present.) So the *summon → specific creature/model* mapping for the
player-pet types still needs recovering from the effect scripts / a debugger capture — see
the dynamic-analysis list in [`pet-restoration.md`](../reverse-engineering/findings/pet-restoration.md).

## Related Docs

- [combat-system.md](combat-system.md) - Pet uses mob combat system
- [ability-system.md](ability-system.md) - Pet abilities
- [stat-system.md](stat-system.md) - Pet stats (inherited from SGWMob)
