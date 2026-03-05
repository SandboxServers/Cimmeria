# Gameplay Systems Dashboard

> **Last updated**: 2026-03-05

Overview of every game system in Stargate Worlds, with implementation status, key network events, entity interfaces, and Python scripts.

## System Status Overview

Status key: **CW** = Confirmed Working, **NT** = Needs Test, **IM** = Implemented (known gaps), **KM** = Known/Missing, **NU** = Needed/Unknown. See [gap-analysis.md](../gap-analysis.md) for per-feature detail.

| System | Status | Key Interface | Key Python Files | Priority |
|--------|--------|---------------|------------------|----------|
| [Combat](#combat) | IM | SGWCombatant | AbilityManager.py, EffectManager.py | HIGH |
| [Abilities](#abilities) | IM | SGWAbilityManager | AbilityManager.py | HIGH |
| [Effects](#effects) | IM | SGWCombatant | EffectManager.py | HIGH |
| [Stats](#stats) | IM | SGWCombatant | Stat.py | HIGH |
| [Inventory](#inventory) | NT | SGWInventoryManager | Inventory.py | HIGH |
| [Missions](#missions) | IM | Missionary | MissionManager.py | HIGH |
| [NPC AI](#npc-ai) | IM | SGWMob (direct) | SGWMob.py | HIGH |
| [Spawn System](#spawn-system) | KM | SGWSpawnRegion | SGWSpawnRegion.py | HIGH |
| [Loot](#loot) | NT | Lootable | Lootable.py | HIGH |
| [XP & Leveling](#xp--leveling) | IM | SGWCombatant | SGWPlayer.py | HIGH |
| [Character Creation](#character-creation) | NT | Account | Account.py | MEDIUM |
| [Gate Travel](#gate-travel) | IM | GateTravel | GateTravel.py | MEDIUM |
| [Chat](#chat) | NT | Communicator | SGWChannelManager.py | LOW (done) |
| [Crafting](#crafting) | NT | (SGWPlayer direct) | Crafter.py | MEDIUM |
| [Vendors](#vendors) | NT | SGWInventoryManager | Vendor.py | MEDIUM |
| [Organizations](#organizations) | KM | OrganizationMember | OrganizationManager.py | MEDIUM |
| [Minigames](#minigames) | IM | MinigamePlayer | MinigameManager.py | LOW |
| [Mail](#mail) | KM | SGWMailManager | MailManager.py | MEDIUM |
| [Trading](#trading) | NT | SGWInventoryManager | Trade.py | LOW |
| [Black Market](#black-market) | KM | SGWBlackMarketManager | SGWBlackMarket.py | LOW |
| [Pets](#pets) | KM | (SGWPet entity) | SGWPet.py | LOW |
| [Dueling](#dueling) | KM | (SGWPlayer direct) | — | LOW |
| [Groups](#groups) | KM | GroupAuthority | — | MEDIUM |
| [Contact Lists](#contact-lists) | KM | ContactListManager | — | LOW |
| [Cinematics](#cinematics) | IM | SGWSpawnableEntity | SGWSpawnableEntity.py, RingTransporter.py | MEDIUM |
| [Ring Transport](#ring-transport) | IM | GateTravel | RingTransporter.py | MEDIUM |

---

## Combat

**Status**: 70% — Basic ability use, damage, death/respawn work. Missing: AoE/cone targeting, channeled abilities, threat/aggro, diminishing returns.

**Key Events (NetOut → server)**:
- `UseAbility` — Activate an ability on a target
- `useAbilityOnGroundTarget` — AoE ability at position (NOT IMPLEMENTED)
- `SetAutoCycle` — Toggle auto-attack
- `ConfirmEffect` — Client confirms effect application (NOT IMPLEMENTED)
- `SetCrouched` — Toggle cover/crouch stance (NOT IMPLEMENTED)

**Key Events (NetIn → client)**:
- `onEffectResults` — Damage/heal numbers, effect application
- `TimerUpdate` — Cooldown and warmup timers
- `onStatUpdate` / `onStatBaseUpdate` — HP, focus, stat changes
- `onMeleeRangeUpdate` — Melee range for current weapon
- `onThreatenedMobsUpdate` — Threat list (NOT IMPLEMENTED)

**Interface**: `SGWCombatant.def` — 44 properties, 15 cell methods, 4 client methods
**Python**: `python/cell/AbilityManager.py` (1091 lines), `python/cell/EffectManager.py`
**RE doc**: [combat-system.md](combat-system.md)

---

## Abilities

**Status**: 60% — Ability activation works for direct-target abilities. Missing: targeting modes (cone, AoE, chain), channeled abilities, combo system.

**Data**: 1,887 abilities defined in `db/resources.sql`
**Schema**: `Ability.xsd` defines ability structure
**Enums**: `TargetingMode`, `AbilityRange`, `AbilityType` in enumerations.xml

**RE doc**: [ability-system.md](ability-system.md)

---

## Effects

**Status**: 60% — Basic effect application/removal works. Missing: pulsing effects, diminishing returns, effect stacking rules, absorption shields.

**Data**: 3,217 effects defined in `db/resources.sql`
**Schema**: `Effect.xsd` defines effect structure
**Key concepts**: Duration, pulse interval, stat modifiers, conditions, cleanup

**RE doc**: [effect-system.md](effect-system.md)

---

## Stats

**Status**: 50% — Base stats and some derived stats work. Missing: full derived stat calculation, regen formulas, level scaling curves, stat caps.

**Key properties** (SGWCombatant.def): `health`, `healthMax`, `focus`, `focusMax`, `armor`, `meleeRange`, `level`
**Architecture**: 6-tier stat dictionary system in Python
**Enums**: 70+ stat types in enumerations.xml (`StatType`)

**RE doc**: [stat-system.md](stat-system.md)

---

## Inventory

**Status**: 80% — Item storage, equipping, moving, basic store buy/sell work. Missing: stat recalculation on equip, item repair, item recharge, buyback, org vault integration.

**Key Events (NetOut)**:
- `MoveItem`, `RemoveItem`, `UseItem`, `LootItem`
- `PurchaseItems`, `SellItems`, `BuybackItems` (NOT IMPL)
- `RepairItem`, `RechargeItem` (NOT IMPL)

**Key Events (NetIn)**:
- `onContainerInfo` — Full container sync
- `onUpdateItem` — Single item update
- `onRemoveItem` — Item removed
- `onStoreOpen` / `onStoreClose` — Store UI

**Interface**: `SGWInventoryManager.def` — 9 properties, 13 cell methods, 6 client methods
**Data**: 6,060 items in `db/resources.sql`
**RE doc**: [inventory-system.md](inventory-system.md)

---

## Missions

**Status**: 40% — Mission accept and basic step advancement work. Missing: objective tracking, task completion detection, reward selection, mission sharing, full mission completion flow.

**Key Events (NetOut)**: `MissionAssign`, `MissionAdvance`, `MissionComplete`, `ChosenRewards`, `ShareMission`
**Key Events (NetIn)**: `onMissionUpdate`, `onStepUpdate`, `onObjectiveUpdate`, `onTaskUpdate`, `MissionOffer`, `MissionRewards`

**Interface**: `Missionary.def` — 7 properties, 20 cell methods, 5 client methods
**Data**: 1,041 missions in `db/resources.sql`
**RE doc**: [mission-system.md](mission-system.md)

---

## Gate Travel

**Status**: 30% — DHD UI opens, some gate addresses are known, ring transport partially implemented. Missing: actual gate travel (zone transition), gate animation sequences.

**Key Events (NetOut)**: `onDialGate`, `DHD`, `SetRingTransporterDestination`
**Key Events (NetIn)**: `setupStargateInfo`, `onStargatePassage`, `onDisplayDHD`, `onDHDReply`, `onRingTransporterList`

**Interface**: `GateTravel.def` — 5 properties, 4 cell methods, 4 client methods, 2 base methods
**Data**: 29 stargate addresses defined
**RE doc**: [gate-travel.md](gate-travel.md)
**See also**: [ring-transport-system.md](ring-transport-system.md)

---

## Chat

**Status**: NT — Chat.py (352 lines) implements channels, tells, emotes, AFK/DND. 11 admin methods are stubs. Not tested with live client.

**Interface**: `Communicator.def` — 5 properties, 11 base methods, 7 client methods, 1 cell method
**Python**: `python/base/SGWChannelManager.py`
**RE doc**: [chat-system.md](chat-system.md)

---

## Crafting

**Status**: NT — Crafter.py (575 lines) implements disciplines, crafting, research, reverse engineering, alloys. Untested with live client.

**Key Events (NetOut)**: `Craft`, `Research`, `ReverseEngineer`, `Alloy`, `RespecCraft`, `SpendAppliedSciencePoint`
**Key Events (NetIn)**: `onUpdateDiscipline`, `onUpdateCraftingOptions`, `onUpdateKnownCrafts`, `onCraftingRespecPrompt`, `onDisciplineRespec`

**Data**: 499 recipes, crafting disciplines, blueprints, paradigms
**Concepts**: Applied Science (tech trees), racial paradigms, material alloys, reverse engineering
**RE doc**: [crafting-system.md](crafting-system.md)

---

## Organizations

**Status**: 5% — Basic stub exists. Not functional.

**16 NetIn events** for organization state sync (roster, ranks, cash, XP, MOTD, notes)
**15 NetOut events** for organization actions (create, invite, kick, rank change, etc.)

**Interface**: `OrganizationMember.def` — 8 properties, 45+ cell methods, 16 client methods, 3 base methods
**Organization types**: Command (guild), Squad (group), Team
**RE doc**: [organization-system.md](organization-system.md)

---

## Minigames

**Status**: 0% — Framework scaffolded but no minigames functional.

**Known minigames**: At least 5 minigame types referenced in data
**Architecture**: MinigamePlayer interface (25 properties, 78 methods) — the largest interface by method count

**Key Events**: `StartMinigame`, `EndMinigame`, `MinigameCallRequest/Accept/Decline/Abort`
**RE doc**: [minigame-system.md](minigame-system.md)

---

## Mail

**Status**: 0% — Not implemented.

**Features**: Send/receive mail, item attachments, currency attachments, COD (cash on delivery), return to sender
**9 NetOut events**, **4 NetIn events**
**Interface**: `SGWMailManager.def` — 4 properties, 9 cell methods, 3 client methods, 1 base method
**RE doc**: [mail-system.md](mail-system.md)

---

## Trading

**Status**: NT — Trade.py (244 lines) implements trade proposals, slot management, lock/confirm flow. Untested with live client.

**Features**: Player-to-player trade windows, item/currency proposals, lock and confirm
**Events**: `TradeProposal`, `TradeLockState`, `TradeRequestCancel` (NetOut), `TradeState`, `TradeResults` (NetIn)
**RE doc**: [trade-system.md](trade-system.md)

---

## Black Market

**Status**: 0% — Not implemented.

**Features**: Auction house for player-listed items. Search, bid, buyout, create/cancel auctions.
**Events**: `BMSearch`, `BMCreateAuction`, `BMCancelAuction`, `BMPlaceBid` (NetOut), `BMOpen`, `BMAuctions`, `BMAuctionUpdate`, `BMAuctionRemove`, `BMError` (NetIn)
**Entity**: `SGWBlackMarket` — dedicated base entity for the auction system
**Interface**: `SGWBlackMarketManager.def` — 1 property, 7 cell methods, 5 client methods, 5 base methods
**RE doc**: [black-market.md](black-market.md)

---

## Pets

**Status**: 0% — Not implemented.

**Entity**: `SGWPet` (extends SGWMob) — 8 properties, 8 cell methods, 3 client methods
**Features**: Pet summoning, ability control, stance management, pet leveling
**Events**: `PetInvokeAbility`, `PetAbilityToggle`, `PetChangeStance` (NetOut), `PetAbilities`, `PetStances`, `PetStanceUpdate` (NetIn)
**RE doc**: [pet-system.md](pet-system.md)

---

## Dueling

**Status**: 0% — Not implemented.

**Entity**: `SGWDuelMarker` — placed in world to define duel area
**Events**: `DuelChallenge`, `DuelResponse`, `DuelForfeit` (NetOut), `onDuelChallenge`, `onDuelEntitiesSet`, `onDuelEntitiesRemove`, `onDuelEntitiesClear` (NetIn)
**RE doc**: [duel-system.md](duel-system.md)

---

## Groups

**Status**: 10% — Basic entity exists. Group formation and management not functional.

**Entity**: `SGWPlayerGroupAuthority` (extends SGWEntity, implements GroupAuthority)
**Interface**: `GroupAuthority.def` — 3 properties, 4 base methods
**Organization types** double as group types (Squad = party)
**RE doc**: [group-system.md](group-system.md)

---

## Contact Lists

**Status**: 0% — Not implemented.

**Features**: Friend lists, ignore lists, custom lists. Online notifications.
**6 NetOut events**, **5 NetIn events**
**Interface**: `ContactListManager.def` — 1 property, 6 cell methods, 5 client methods, 2 base methods
**RE doc**: [contact-list.md](contact-list.md)

---

## NPC AI

**Status**: IM — SGWMob.py (397 lines) implements combat AI (Fighting state), threat management, ability selection. Missing: 10 of 12 AI states (Patrol, Wander, Leash, Investigate, Flee, Follow, Summon, Idle, Inactive, Dead).

**Key concepts**: AI state machine (12 states), threat table, aggro radius, ability selection, aggression system (3 levels)
**Python**: `python/cell/SGWMob.py` (397 lines)
**RE doc**: [npc-ai.md](npc-ai.md)

---

## Spawn System

**Status**: KM — SGWSpawnRegion.py and SGWSpawnSet.py are empty shells. All 26+22 .def properties exist but no Python implementation.

**Key concepts**: Spawn regions (area-based), spawn sets (NPC templates), weighted spawn tables, population caps, respawn timers
**Python**: `python/base/SGWSpawnRegion.py`, `python/base/SGWSpawnSet.py` (both empty)
**RE doc**: [spawn-system.md](spawn-system.md)

---

## Loot

**Status**: NT — Lootable.py (221 lines) implements loot generation algorithm. Code works but loot tables are nearly empty.

**Key concepts**: Independent probability rolls per entry, eligibility checks, loot interaction handler
**Python**: `python/cell/Lootable.py` (221 lines)
**RE doc**: [loot-system.md](loot-system.md)

---

## XP & Leveling

**Status**: IM — giveExperience() works, placeholder LEVEL_EXP table. Missing: real XP curve, stat scaling on level-up, training point awards.

**Key concepts**: XP thresholds, level cap (20), training points, applied science points, archetype stat growth
**Python**: `python/cell/SGWPlayer.py`
**RE doc**: [progression-system.md](progression-system.md)

---

## Character Creation

**Status**: NT — Account.py (~300 lines) has complete creation flow: validation, DB insert, 8 archetypes, starting equipment/abilities.

**Key concepts**: Archetype selection, visual choices, name validation, starting loadout, character deletion
**Python**: `python/base/Account.py`
**RE doc**: [character-creation.md](character-creation.md)

---

## Cinematics

**Status**: 60% — Core `playSequence()` works for abilities, effects, stargates, and ring transport. 1,973 sequences and 675 event sets in DB. Missing: DHD chevron animations, stargate witness visibility, item equip/unequip VFX, multi-player ring sync, entity spawn/despawn sequences.

**Key Events (NetIn → client)**:
- `onSequence` — Play a Kismet/Matinee sequence (8 args: seqId, source, target, primaryTarget, impactTime, NVPs, viewType, instanceId)
- `onKismetEventSetUpdate` — Update entity's default event set

**Key Events (Cell methods)**:
- `PlayAllClientKismetSeq` — Original BigWorld sequence trigger (bypassed by Python implementation)

**Interface**: `SGWSpawnableEntity.def` — `kismetEventSetId` property, `shouldSendKismet` flag
**Python**: `python/cell/SGWSpawnableEntity.py` (playSequence), `python/cell/AbilityManager.py` (combat sequences), `python/cell/RingTransporter.py` (ring transport), `python/cell/SGWPlayer.py` (stargates)
**Data**: 1,973 sequences, 675 event sets, 2,042 NVPs, 2,767 item event sets, 66 event types
**RE doc**: [cinematic-system.md](cinematic-system.md)

---

## Ring Transport

**Status**: 40% — Ring transport Python implementation exists (RingTransporter.py). 8-state FSM documented. Console activation triggers teleport. Missing: full Kismet matinee animation sequence, multi-player sync (only first player gets animation), ring platform visual effects, proper activation UI (currently uses small console on ground).

**Key Events (NetOut)**: `SetRingTransporterDestination`
**Key Events (NetIn)**: `onRingTransporterList`, `onSequence` (Kismet matinee)

**Architecture**: 8-state FSM: IDLE → SEND_WAIT → SEND_WARMUP → REMOTE_LOAD_WAIT → REMOTE_WARMUP → COOLDOWN. Timed transitions (3.5s hide, 4.0s teleport, 3.0s reveal, 2.5s unlock).

**Interface**: `GateTravel.def` — ring transport methods
**Python**: `python/cell/RingTransporter.py`
**RE doc**: [ring-transport-system.md](ring-transport-system.md)

---

## Priority Matrix

### Must Have (blocks playability)
1. **Combat completion** — AoE targeting, channeled abilities, threat/aggro
2. **Mission completion** — Objective tracking, reward selection, full mission flow
3. **Stat system** — Derived stats, regen, level scaling
4. **XP & leveling** — Real XP curve, stat scaling on level-up, training point awards (see [progression-system.md](progression-system.md))
5. **NPC AI completion** — 10 of 12 AI states unimplemented (Patrol, Wander, Leash, Investigate, Flee, Follow, Summon, Idle, Inactive, Dead)
6. **Spawn system** — NPC population management (currently 100% empty)

### Should Have (core MMO features)
7. **Gate travel** — Zone transitions via stargates
8. **Organizations** — Guild creation, roster, ranks
9. **Mail** — Basic send/receive
10. **Inventory completion** — Repair, recharge, buyback

### Nice to Have (polish)
11. **Crafting** — Disciplines, recipes, alloys
12. **Black Market** — Auction house
13. **Minigames** — At least 1-2 functional
14. **Trading** — Player trade windows
15. **Pets** — Pet summoning and control
16. **Dueling** — PvP duels
17. **Contact lists** — Friends/ignore

---

> For per-feature status tracking with dependency chains and implementation confidence levels, see the [Gap Analysis](../gap-analysis.md).
>
> For content-level data audit (what content populates each system, orphan rates, cross-reference integrity), see the [Content Data Audit](../content/README.md).
