---
title: "Pet System"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Pet System

> **Last updated**: 2026-07-25
> **Status**: ~10% — engine support, content, and client are complete; the server-side summon/command/despawn lifecycle is unimplemented (tracked in #570). Findings: [`reverse-engineering/findings/pet-restoration.md`](../reverse-engineering/findings/pet-restoration.md).

## Overview

The pet system allows players to summon and control companion NPCs that fight alongside them. Pets extend the `SGWMob` entity with owner tracking, ability management (including toggling abilities on/off), stance control, leveling, and despawn timers. Pets respond to owner events (death, leash, respawn) and can resolve abilities on spawn.

The `SGWPet` entity is defined in `entities/defs/SGWPet.def` (parent: `SGWMob`). The Python script `deprecated/python/cell/SGWPet.py` contains only stub initialization for ability and stance lists.

## Implementation Status

Nothing in `crates/` implements the pet lifecycle — there is no pet module, and the `SGWPet` cell/client methods below have no Rust handlers. The table records what the *entity definitions* provide versus what any server has ever done with them.

| Feature | Status | Notes |
|---------|--------|-------|
| Pet entity definition | DONE | Full property and method set defined |
| Owner tracking | DEFINED | `ownerID`, `ownerBase` properties |
| Ability list | STUB | `onPetAbilityList` sends list to client |
| Stance list | STUB | `onPetStanceList` sends list to client |
| Ability toggling | STUB | `toggleAbility` with on/off flag |
| Stance changing | STUB | `changePetStance` with `onPetStanceUpdate` |
| Pet leveling | STUB | `setPetLevel` defined |
| Owner death response | STUB | `onOwnerDeath` cell method |
| Owner leash response | STUB | `onOwnerLeash` cell method |
| Owner respawn response | STUB | `onOwnerRespawn` with despawn flag |
| Despawn timer | DEFINED | `petDespawnTimerId` property |
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
