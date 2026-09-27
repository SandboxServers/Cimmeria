---
title: "Pet System"
type: reference
audience: engineers
last_updated: 2026-09-26
---

# Pet System

> **Last updated**: 2026-09-27
> **Status**: ~30%. Engine support, content and client are complete. A player can summon an owned pet by casting its summon ability (pets campaign PT-03: 2826 Summon Straegis spawns template 350). The server introduces the pet to its owner, tears it down, and keeps it tied to its owner on every owner lifecycle path (PT-01, PT-02). The owner commands the pet (PT-04), the pet AI follows and fights (PT-05), and the owner's own pet abilities (Holy Warrior, To The Death, Heed Our Calling, Lord's Concentration, the pet heals) act on it (PT-08). Tracked in #570, ledger `docs/analysis/pets/`. Findings: [`reverse-engineering/findings/pet-restoration.md`](../reverse-engineering/findings/pet-restoration.md).

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
- **Kill credit (PT-06).** Kill XP and mission kill credit resolve the attacker through one seam: `SpaceManager::credit_recipient` on the XP path, which logs a refusal once per kill, and its non-logging twin `credit_recipient_quiet` for mission credit and the per-cast AI and warmup gates. A pet's kill pays its owner `kill_xp × transfer_xp` (1.0, D-PT02) as one `GrantXP` to the owner, and raises `EntityDeath` on the owner, so KillCount chains bump the owner's counters and read the owner's mission state. A mob or any other NPC attacker gets no `GrantXP`, including a mob that kills a pet. A pet whose owner's entity id now belongs to another player, or whose owner is gone, credits nobody (`credit_refused` on `pets.credit`). A payout that rounds to zero, or exceeds `i32::MAX`, pays nothing and is logged on `pets.credit` (`zero_xp`, `xp_overflow`). A paid kill logs `pet_kill_credited` only once the owner's `GrantXP` has been sent; if the send fails it logs `pet_kill_credit_undelivered` instead. A pet's NPC AI attack, its warmed-up casts and its DoTs all credit the owner. A corpse a pet killed rolls its loot as usual. The server has no per-corpse loot owner (any player in interact range may open a corpse), so the owner loots it like any other kill.
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

- **AI (PT-05).** Follow, teleport, stances, defend-owner, the owner-anchored leash and the owner's combat state. See [Pet AI](#pet-ai-pt-05).

**Summoning (PT-03).** A player ability with a `resources.pet_summons` row summons a pet. The seed has four rows (see [Roster](#roster-pt-s-pt-11)), each with one pet out at a time. The code is `crates/cell-combat/src/cell/abilities/use_ability/summon.rs`, and the design is decision 23 of [`abilities-and-effects-system.md`](../architecture/abilities-and-effects-system.md).

- The cast is an ordinary cast. It has the 6 s warmup, which `speedPet` shortens for `SpeedPet` abilities (D-PT10). The cooldown is charged at launch. Moving, dying or changing space during the warmup cancels it, and a cancelled warmup spawns nothing.
- The client's target is ignored. Any other self-targeted ability is still refused by the #444 gate.
- When the warmup ends, the new pet appears 2 u behind the owner and the caster plays the summon effect (2292). A second summon then despawns the previous pet (D-PT04). If the spawn fails, the cast plays as interrupted and the previous pet stays.
- The ground effect (2293) plays at the pet once the owner's client has created the pet.
- A summon that cannot spawn gets an `onErrorCode` and a chat line: "Your pet could not be summoned." (or "You have not trained that summon."). No cooldown is charged when this happens at the press.
- The `.pet` console is PT-07.

### Roster (PT-S, PT-11)

The Goa'uld Servant Lord summons, in the owner's order (D-PT13). Every row has `max_active` 1, and every summon carries event set 1121 (the Goa'uld summon cast effect). Pet templates 350-359 are summoned creatures only and are never placed in `spawnlist` (D-PT16).

| Summon | Tree level | Template | Name (moniker) | Look | Kit (lowest id is the primary) |
|---|---|---|---|---|---|
| 1643 Summon Jaffa | L1 root | 351 | "Jaffa Soldier" (8087) | A copy of 160 Praxis Jaffa Guard: `BS_JaffaMale` in `AR_J_Praxis` | 584 Staff Auto Attack, 710 Staff Melee AA, 1652 Jaffa: Double Blast |
| 1644 Summon Lo'taur | L10 | 353 | "Lo'Taur Servant" (28891) | Composed: the bare `BS_GoauldMale` of 211 in the `AR_G_Underlings` servant dress (body armour, dress, feet, bracers) and slave headwrap. No seeded template wore that dress | 1653 Heal Health, 3326-3329 (focus heal, focus-regen buff, defense buff, defense debuff) |
| 1645 Summon Prime | L15 | 352 | "Jaffa Prime" (28892) | A copy of 159 Praxis Jaffa Lieutenant | 584, 710, 1654 Prime: Focus Degeneration. **Deviation:** the packet named only 1654; the Prime keeps 159's staff pair (set 4) so it has a working attack (coordinator-approved) |
| 2826 Summon Straegis | L50 capstone | 350 | "Summoned Straegis Fighter" (27377) | A copy of 78 Straegis Fighter | 221 Energy Shock, 1156 Straegis: Disengage |

- **Level.** Templates 350-353 carry `ENTITYFLAG_Pet` alone, so the summon gives the pet its owner's level (D-PT02). `ENTITYFLAG_NoPetLeveling` would make the spawn keep the template's level 1; template 350 carried it until PT-11, so the L50 capstone Straegis spawned at level 1.
- **Stances.** No roster template sets `NoPassive`, `NoDefensive` or `NoAggressive`, and no data says any pet lacks a stance, so every pet is offered all three.
- **What the kits do today.** Only 584 (effect 646) and the Straegis's 221 deal damage. 1652, 1654, 1653 and 3326-3329 have effects with no damage values and no script, so they resolve as empty hits. 1652 plays the staff shot (event set 3, the set of 584; its description is "Staff: Ranged Single Target Attack"). 1654 and the Lo'taur abilities have no event set in the data and play nothing; the NA43 animation linter allowlists them with a guard that they still deal no damage.
- **Abilities that do nothing are refused and skipped.** An ability with no event set and no effect that deals damage or runs a script (`cimmeria_entity::abilities::ability_is_unimplemented`) has no visible result. A CM 88 order for one is refused before the pet casts (`onErrorCode` 167, the chat line "Your pet can't use that ability yet.", DEBUG `pets.command reason=ability_not_implemented`), and the pet AI never picks one for a pet. Today that is 1654 and all five Lo'taur abilities, so **the Lo'taur holds fire** and the Prime fights with its staff only.
- **Known gap: the Lo'taur cannot heal (friendly-target pet abilities are unsupported).** The pet AI and a CM 88 order both aim a pet's ability at an enemy. Binding the `HealHealth` / `HealFocus` scripts to the Lo'taur effects needs an ally-target behaviour first, or the Lo'taur would heal what it fights.
- **Pet-trained abilities.** 1652 and 1654 are `PetTrained`: in the original the owner trains them (Servant Lord L20) and `SGWPlayer.knownPetAbilities` carries them to the pet. The server has no `knownPetAbilities` path, so they ride on the pet's own ability set and every Jaffa or Prime has them from level 1.
- **Range.** 1652's `max_range` is 3000 and 1653's is 800. The server reads those as world units, so the pet AI picks 1652 at any distance while 584 cools and never walks in for it. Most seeded ranges look like centimetres (3000 = 30 m); that is a server-wide question, not a pet one.
- **Renders.** The Jaffa and Prime looks are placed today (160 and 159 in Harset and Castle). The Lo'taur composite has never been rendered and needs an in-game check.

The owner's commands are PT-04 (see [Owner commands](#owner-commands-pt-04)). The table records what the entity definitions provide and what the server does with them.

| Feature | Status | Notes |
|---------|--------|-------|
| Pet entity definition | DONE | Full property and method set defined |
| Owner tracking | DONE (PT-01) | `PetRegistry` on the cell. `ownerID` reaches the client as `onEntityProperty(GENERICPROPERTY_PetOwnerId)` in the create cascade |
| Ability list | DONE (PT-01) | `onPetAbilityList` to the owner only, on AoI entry |
| Stance list | DONE (PT-01) | `onPetStanceList` to the owner only, filtered by `ENTITYFLAG_NoPassive` / `NoDefensive` / `NoAggressive` |
| Summon by ability | DONE (PT-03) | `pet_summons` row → warmup → spawn beside the owner; one pet per owner (D-PT04); source and target VFX |
| Spawn and teardown | DONE (PT-01, PT-02) | `spawn_pet_from_template`. Despawn on every owner departure, move beside the owner on a same-space teleport; see [Owner lifecycle](#owner-lifecycle) |
| Ability toggling | DONE (PT-04) | CM 89 `petAbilityToggle` updates `toggled_off` and re-sends `onPetAbilityList` to the owner. CM 88 refuses an OFF ability |
| Stance changing | DONE (PT-04) | CM 90 `petChangeStance`: a listed stance id or a 1-based slot (A-07), then `onPetStanceUpdate` to the owner only |
| Owner ability orders | DONE (PT-04) | CM 88 `petInvokeAbility` behind the ownership guard; the pet casts and engages the target |
| Pet leveling | STUB | `setPetLevel` defined |
| Owner death response | DONE (PT-02) | The pet despawns when its owner dies (D-PT08), from `resolve_death`; the `onOwnerDeath` cell method itself is unused |
| Owner leash response | STUB | `onOwnerLeash` cell method. The AI's own teleport back (PT-05) does not go through it |
| Owner respawn response | DONE (PT-02) | Cross-world respawn despawns the pet; a same-world respawn moves it beside the owner. `onOwnerRespawn` itself is unused |
| Despawn timer | DONE (PT-02) | `PetState::despawn_at`: a dead pet's corpse despawns after 10 s |
| Ability on spawn | DEFINED | `abilityToResolve`, `abilityInformation` |
| XP transfer | DONE (PT-06) | Owner gets `kill_xp × transfer_xp` for the pet's kills; a zero, negative or non-finite value pays nothing, and so does any payout above `i32::MAX` (the width of `sgw_player.exp`) |
| Kill credit | DONE (PT-06) | A pet's kill raises the owner's `EntityDeath` (KillCount missions advance); NPC attackers are never credited |
| Position tracking | DEFINED | `ownerLastPosition`, `petLastPosition`, `lastOwnerPositionCheck` |
| Pet AI | DONE (PT-05) | Follow, teleport, stances, defend-owner, owner-anchored leash, owner combat state. See [Pet AI](#pet-ai-pt-05) |
| Owner abilities on the pet | DONE (PT-08) | Holy Warrior, To The Death, Heed Our Calling, Lord's Concentration and the Repair Turret heals act on the owner's pet. See [Owner abilities on pets](#owner-abilities-on-pets-pt-08) |
| Pet persistence | STUB | `saveToDB` defined but no save logic |

## Owner commands (PT-04)

The owner commands the pet through three SGWPlayer cell methods. The handlers are in `crates/cell-methods/src/cell/cell_methods/player/pet/`, and their log target is `pets.command`.

Every command checks ownership first (CAT-C-11 / #462). The pet id in the packet goes through `SpaceManager::owned_pet(caller, claimed)`. If the id names another player's pet, an NPC, a player or nothing, or the caller holds the owner's entity id but did not summon the pet (the id was reused), the command is refused. `owned_pet` logs the refusal once at DEBUG (`event = ownership_rejected`, `reason` = `not_owner`, `not_a_pet`, `pet_gone` or `owner_identity_mismatch`): a client can name any id at will, so it is not a WARN. The caller gets `onErrorCode` (`IsNotPetOwner` 236, or `DoesNotHavePet` 190 for `pet_gone`). A dead owner is refused with `NotLiving` 14, and a pet that is not in its owner's space (the teardown sweep has not run yet) with 190. Every other refusal also gets an answer: an `onErrorCode` to the **owner**, or a re-send of the pet bar or stance.

| Method | What it does |
|---|---|
| CM 88 `petInvokeAbility(petId, abilityId, targetId)` | The ability must be on the pet's bar and not toggled off. The pet must be alive, not warming up another ability, and off cooldown. An explicit target must be something the pet may fight, by the pet AI's own rule (`npc_ai::pet::fight_refusal`, PT-05): a combatant SGWMob, so never a player, a pet or an SGWBeing (`target_not_combatant`), that its owner could attack itself (`target_not_hostile`). It must be in the pet's space, engageable (not dead, not walking home or leaving, `target_resetting`; a surrendered NPC is still a target, as for a player's own attack), within the ability's range (`max_range`, or 30 u by default), and in line of sight (the fight tick's `attack_line_of_sight`; `fire_los` skips NPC shooters). The pet casts through `handle_use_ability_with_kill_credit`, then engages the target through the pet AI's `engage_pet_target` with `PetEngagement::OwnerOrder`: the target lists the pet and fights back, the pet puts the target on top of its own threat list and goes Fighting (`npc_ai.transition reason = pet_engage`), and the pet AI's next pass puts the owner in combat. For a cast with a warmup that engagement waits until the cast fires (the warmup tick's `pet_order`). An interrupted warmup engages nothing: the order is dropped and the owner is told why. Switching the pet to Passive also drops an order still warming up. An order's threat on the pet is capped at 1,000,000 (`OWNER_ORDER_THREAT_CAP`). `targetId <= 0` casts untargeted and engages nothing. Refusal codes: 167 (not on the bar, toggled off, or an ability that does nothing yet: `ability_not_implemented`, PT-11), 14 (the pet or the target is dead), 99 (cooldown or busy), 37 (a target the pet may not fight, including any pet or being), 42 (out of range), 39 (no line of sight), 0 (the target is gone, in another space or resetting). Every `onErrorCode` is paired with a `CHAN_FEEDBACK` chat line saying why (`pets::order_feedback_text`), because the shipped client has no Lua consumer for `onErrorCode` (AT-E1). A pet ability with a warmup is re-checked when it fires: the warmup tick applies the same `fight_refusal` rule, the engagement's state rule (a target that started walking home is not hit) and line of sight to a pet caster, before any damage. |
| CM 89 `petAbilityToggle(petId, abilityId, toggle)` | `toggle` 1 turns the ability on (removes it from `toggled_off`) and 0 turns it off. Either way the owner is sent `onPetAbilityList` again. Any other value changes nothing and still re-sends the bar. An ability that is not on the bar is refused with 167. |
| CM 90 `petChangeStance(petId, stance)` | A stance id from the pet's stance list is taken as sent. Any other value is read as a 1-based slot into the list the owner was sent. The small pet bar sends slot numbers (A-07, a bug in the 2009 client). A value that is neither is refused, and the current stance is sent again. On success the stance is set and `onPetStanceUpdate` goes to the owner only. |

The owner may not aim the pet at anything the owner could not attack. `handle_use_ability` applies the #444 target rule to player casters only, so the pet handler applies it for the owner. Every pet-bar click in the shipped UI arrives as CM 88; nothing in the client Lua calls CM 89 (A-06). The stance rules for autonomous engagement belong to PT-05, and an explicit CM 88 order is obeyed whatever the stance.

## Owner abilities on pets (PT-08)

Some of the owner's own abilities act on the owner's pet. The code is in
`crates/cell-combat/src/cell/abilities/use_ability/owner_pet/`, the effect scripts are in
`crates/cell-world/src/cell/effects/pet_scripts.rs`, and the design is decision 25 of
[`abilities-and-effects-system.md`](../architecture/abilities-and-effects-system.md).

| Ability | What the server does |
|---|---|
| 2824 Holy Warrior (Toggled, Battle Lord) | The first press gives the pet +100 Accuracy and -100 Defense (effect 4220). The next press takes it off. The owner reads "Holy Warrior is on." or "... off.". Its other effect, 4087 "Stance Removal", would remove the owner's other stance; no player stance exists on this server, so it removes nothing |
| 2839 To The Death | After its 2 s warmup the pet gets +400 Accuracy for 60 s (4121), and the owner reads "Your pet fights to the death: it dies in 60 seconds.". When the 60 s run out (4119), the Accuracy comes off and the pet dies (4122). The death goes through the normal death resolver: the pet becomes a corpse and is despawned 10 s later (D-PT08). Nobody gets XP or kill credit for it. Casting it again while it runs is refused (`onErrorCode` 133 and "Your pet is already fighting to the death."), so the timer can never be restarted |
| 2852 Heed Our Calling (passive) | While the owner knows it, the owner's `speedPet` is 100, so a `SpeedPet` summon has no warmup: the summon is instant (D-PT10). It is applied at login (which covers a new character's first entry), when the ability is trained, and when a GM grants it (`.giveability 2852`: the next summon is instant at once), and removed by a respec |
| 1650 Lord's Concentration | After its 2 s warmup every pet the owner has out gets +50 Interrupt Resistance for 30 s. The 2009 data gave this ability no effect; effect 350 and its values are greenfield (D-PT17, PROPOSED, adopted at its default unless the owner objects). **Interrupt Resistance has no reader yet:** the server has no damage-driven warmup interrupt, so the buff changes nothing in play until one lands |
| 967 / 968 / 1207 Repair Turret: Percentage / Regenerate / Full | Heal the owner's pet: 20% of its max health at once (3211), 5% a second for 15 s (3230), or 10% every 0.5 s for 5 s (3350). These are Robotics turret abilities, so until turrets exist (PT-12) they heal whatever pet the owner has |

How the target is chosen:

- **Never the client's target.** The cast is redirected to the caster's own pet. The pet is looked up in the registry and kept only when the summon-time identity says the caster summoned it (`summoner_matches`), it is alive, and it is in the caster's space. A player given a destroyed owner's entity id cannot act on that owner's pet.
- **No pet, no cooldown.** With no such pet the press is refused before the cooldown is charged, with `onErrorCode` and a chat line: 190 and "You have no pet to use that on." (no pet, or a reused owner id), 190 and "Your pet is not here." (another space), 14 and "Your pet is dead.". A pet that dies or leaves during a warmup gets the same answer at the fire, after `Ability_Interrupt`, and the cooldown stays charged.
- **The state is on the pet.** Buffs are kept on the pet's `PetState::buffs`, with the exact stat change each made, so removing one restores the stat. Toggles have no expiry; timed buffs are removed by the owner-pet tick. A despawn, a new summon or the owner's death clears all of it with the pet. `Defense` and `Interrupt Resistance` start at `[0, 0]`; the pet's bound widens so the buff applies (python clamped it away).
- **Log target `pets.buff`.** Every step is logged there; see [observability.md](../architecture/observability.md).

Not done: Repair Turret: Restoration (1214, revive), and 1646 "Health Heal", which stays a heal on the caster's target (it is also the universal starter, D-AT09). 1647 / 1651 (focus heals) and 1648 / 2831 (Defend Your God) are not wired.

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

The `toggledAbilities` array tracks abilities that the player has turned OFF. The AI's ability selector skips any ability in `PetState::toggled_off` (PT-05). With every ability toggled off the pet holds fire, as an NPC with every ability cooling does.

## Pet AI (PT-05)

A pet is ticked by the NPC AI like a mob (`npc_ai_tick`, every 2 s). An owner-relative pre-pass in `crates/cell-combat/src/cell/service/npc_ai/pet/` runs before the state handler. The numbers are the greenfield values of D-PT07 and D-PT09 (`docs/analysis/pets/README.md`); the client has no footprint for them.

| Behaviour | What the server does |
|---|---|
| Follow | Out of a fight a pet is always in `Follow` with `follow_target_id` = owner and a 2-5 u band. The ordinary follow handler walks it. An Idle pet is always admitted to the tick, so a freshly summoned pet starts following on its first turn |
| Teleport | More than 40 u from the owner horizontally, or more than 4 u above or below (another floor): the pet is brought back through PT-02's owner-teleport move (`on_owner_teleported`, path `pet_left_behind`), so it lands on the same grounded spot behind the owner and its witnesses get an immediate `EntityMoved`. At most once every 5 s (`PetState::last_teleport_at`, which an owner teleport also stamps). It is a same-space position write, not `onPlayerTeleport`. An owner with several pets gets all of them moved |
| Passive | Never engages. `generate_threat` refuses all threat to a Passive pet, so even a hit leaves it following. A pet switched to Passive mid-fight drops the fight on its next turn |
| Defensive (default) | Engages a mob that is fighting its owner, or one fighting the pet itself, within 40 u of the owner. A hit preempts it into Fighting, as it does any NPC. A pet that is itself left behind engages nothing until it is back |
| Aggressive | Defensive, plus the owner's current target once the owner has `BSF_InCombat` (within 40 u of the owner; any target the owner may attack, even one content set to Neutral: it is a fight the owner chose), plus an NPC hostile to players within 15 u of the pet, on its floor and not behind a wall |
| Leash | Measured from the owner's position, never from `spawn_position`: the ordinary leash radius (50 u, or the template's) with its hysteresis and vertical cap |
| End of a fight | Straight back to `Follow` on the owner (the `begin_leash` pet branch). No walk home, no evade, no heal, no cooldown reset |
| The mob it leaves | Released (the pet leaves the mob's threat list, and the owner's combat entry for it goes the same turn) when the target stopped being fightable (not a combatant, not hostile, resetting, just reset) or the owner called the pet off (switched to Passive, or content forced Leashing). Kept when the pet is only pulled back by distance (the owner-anchored leash, or a target left behind by an owner teleport) or lost its target: the mob keeps chasing the pet as it would a fleeing player, until its own leash resets it and takes the owner out of combat. A dismissed pet needs nothing: the mob prunes a vanished target |
| Targets it lets go | A fighting pet drops a target that is walking home (it evades), leaving or dead, one that has just finished its reset, and one more than 40 u from the owner or off the owner's floor. So after an owner teleport the pet follows instead of running back to the fight, and it never re-pulls a mob that has reset |
| Engaging | Every engagement goes through `pet::engage_pet_target` (stance picks, and PT-04's owner orders): the target lists the pet and fights back, and the pet lists the target and fights. The owner is mirrored into the fight in the same turn. A refusal (not a combatant mob its owner could attack, dead, resetting) changes nothing. The pet never engages a surrendered (`Submit`) NPC on its own, and drops one that surrenders mid-fight; an owner's order still reaches it, as a player's own attack does |
| Owner's combat state | Every mob with the pet on its threat list enters the owner's `threatened_mobs`, which sets `BSF_InCombat`. The entry goes when the mob dies (the dead-NPC sweep walks every player whose set names it), leashes, or no longer has the owner or a pet of the owner on its threat list. The owner is sent `onStateFieldUpdate` on each edge |
| Hostility | A pet fights only combatant mobs (`SGWMob`; never an `SGWBeing`, even one with the hostile faction) that its owner could attack, by the same rule as the player's own single-target gate (#444): `combat::player_may_attack`, today a hostile-faction NPC that is not a pet. Every stance pick, every target it keeps, every threat it accepts and every fight mirrored into the owner's combat state goes through it, so a pet never turns on a player, another pet, a vendor or a neutral NPC, even one a content chain set fighting its owner. When duels land (SS-D2) they widen that one function and pets follow |
| Line of sight | The Aggressive scan uses the NPC acquisition gate (floor band, radius, navmesh line of sight failing closed on `Unknown`, D-NA08). A fight's shots go through the NPC fight handler's attack line-of-sight check, like any NPC's |
| Players | A pet is never hostile to players (`is_hostile_to_players` is false for any pet), so it never runs the player proximity scan and is never recruited as an assister. Its targets are always mobs (`SGWMob`), never a player, another pet or a being |
| Owner id reused | The owner is the player who summoned the pet, checked against the summon-time identity, not the owner's entity id. If the owner is destroyed and its id handed to another player before the sweep, the pet holds (`owner_missing reason=owner_identity_mismatch`): it does not follow, teleport to, leash to or put that player in combat, and the sweep despawns it |

Every decision logs on the `pets.ai` target; the rows are listed in [observability.md](../architecture/observability.md). A content `generate_threat` aimed at a Passive pet is refused like a hit, so a chain cannot force a Passive pet to fight.

Known limits: the owner enters combat on the pet's next AI turn (up to 2 s after the pet's first hit), and hostile mobs still engage a pet only once it has hit them (they do not proximity-aggro or assist on a pet, audit A-30). An owner entry the pet's fight put in `threatened_mobs` is also cleared by the next pet turn; if the pet is dismissed first, it stays until that mob dies or leashes.

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

Cimmeria does not wait for that recovery. The binding is its own seed table,
`resources.pet_summons` (ability → template, `max_active`; PT-S). The summon keys on the ability
id, not on an effect script (PT-03). The Jaffa, Prime and Lo'taur templates the 2009 seed lacked
are Cimmeria templates 351-353 (PT-11), named with the surviving `DN_Pet_*_Tier_1` monikers; see
[Roster](#roster-pt-s-pt-11).

## Related Docs

- [combat-system.md](combat-system.md) - Pet uses mob combat system
- [ability-system.md](ability-system.md) - Pet abilities
- [stat-system.md](stat-system.md) - Pet stats (inherited from SGWMob)
