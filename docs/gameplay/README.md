---
title: "Gameplay Systems Dashboard"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Gameplay Systems Dashboard

> **Last updated**: 2026-07-25

Overview of every game system in Stargate Worlds, with implementation status, key network events, and entity interfaces.

> **Reading this page.** Status reflects the **Rust** server under `crates/`. Some per-system sections below cite `deprecated/python/…` files: that is the *original* 2009-era server, kept as evidence of design intent, not running code. (There is no longer a `python/` tree at the repo root.) Where a section's Python reference and its status line disagree, the status line and the linked per-system doc are authoritative.

## System Status Overview

Status key: **CW** = Confirmed Working, **NT** = Needs Test, **IM** = Implemented (known gaps), **KM** = Known/Missing, **NU** = Needed/Unknown. See [gap-analysis.md](../gap-analysis.md) for per-feature detail.

| System | Status | Key Interface | Rust implementation | Priority |
|--------|--------|---------------|---------------------|----------|
| [Combat](#combat) | IM | SGWCombatant | `cell/combat/`, `cell/abilities/` | HIGH |
| [Abilities](#abilities) | IM | SGWAbilityManager | `cell/abilities/` | HIGH |
| [Weapon Ammo & Reload](#weapon-ammo--reload) | IM | SGWPlayer / SGWInventoryManager | `cell/cell_methods/player/world/reload.rs`, `cell/cell_methods/inventory/bandolier/` | HIGH |
| [Effects](#effects) | IM | SGWCombatant | `cell/effects/` | HIGH |
| [Stats](#stats) | IM | SGWCombatant | `crates/entity/src/stats/` | HIGH |
| [Inventory](#inventory) | IM | SGWInventoryManager | `cell/cell_methods/inventory/`, `base/world_entry/methods/inventory/` | HIGH |
| [Missions](#missions) | IM | Missionary | `cell/missions/` | HIGH |
| [NPC AI](#npc-ai) | IM | SGWMob (direct) | `cell/service/npc_ai/` | HIGH |
| [Spawn System](#spawn-system) | IM | SGWSpawnRegion | `cell/spawner/` | HIGH |
| [Loot](#loot) | IM | Lootable | `cell/abilities/loot_drop.rs` | HIGH |
| [Death & Respawn](#death--respawn) | IM | SGWCombatant / SGWPlayerRespawner | `cell/abilities/death.rs`, `cell/service/ticks/npc_respawn/` | HIGH |
| [XP & Leveling](#xp--leveling) | IM | SGWCombatant | `base/world_entry/methods/progression/` | HIGH |
| [Character Creation](#character-creation) | IM | Account | `base/character_create.rs`, `base/chardef.rs` | MEDIUM |
| [Gate Travel](#gate-travel) | IM | GateTravel | `base/world_entry/gate_travel/`, `cell/gate_travel.rs` | MEDIUM |
| [Cover](#combat) | IM | SGWCoverSet | `cell/cover/` | MEDIUM |
| [Chat](#chat) | KM | Communicator | `base/dispatch/chat.rs`, `cell/chat.rs` | MEDIUM |
| [Crafting](#crafting) | KM | (SGWPlayer direct) | `base/crafting/` (state only) | MEDIUM |
| [Vendors](#vendors) | IM | SGWInventoryManager | `base/world_entry/methods/vendor/` | MEDIUM |
| [Organizations](#organizations) | KM | OrganizationMember | `cell/cell_methods/organization.rs` (stubs) | MEDIUM |
| [Minigames](#minigames) | IM | MinigamePlayer | `minigame/` | LOW |
| [Mail](#mail) | IM | SGWMailManager | `cell/mail.rs`, `base/world_entry/methods/mail/` | MEDIUM |
| [Trading](#trading) | IM | SGWPlayer | `cell/cell_methods/player/trade/`, `base/world_entry/methods/trade/` | LOW |
| [Black Market](#black-market) | KM on `main` | SGWBlackMarketManager | stubs on `main`; Phase 1 on unmerged PR #586 | LOW |
| [Pets](#pets) | KM | (SGWPet entity) | — | LOW |
| [Dueling](#dueling) | KM | (SGWPlayer direct) | `cell/cell_methods/player/social.rs` (stubs) | LOW |
| [Groups](#groups) | KM | GroupAuthority | — | MEDIUM |
| [Contact Lists](#contact-lists) | CW | ContactListManager | `base/contact_list/`, `cell/cell_methods/contact_list/` | LOW |
| [Cinematics](#cinematics) | KM | SGWSpawnableEntity | seven `onSequence` emit sites; no NVP support | MEDIUM |
| [Ring Transport](#ring-transport) | IM | GateTravel | `cell/ring_transport/` | MEDIUM |

---

## Combat

**Status**: IM — Ability use, damage, death/respawn, cone AoE, channeled abilities (with movement-interrupt), pulsing DoT/HoT, absorption shields, stun/suppression debuffs, and threat/aggro all work (shipped in PR #420 — see [../architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)). Remaining gaps: diminishing returns and some targeting refinements.

**Key Events (NetOut → server)**:
- `UseAbility` — Activate an ability on a target
- `useAbilityOnGroundTarget` — AoE/cone ability at position
- `SetAutoCycle` — Toggle auto-attack
- `ConfirmEffect` — Client confirms effect application
- `SetCrouched` — Toggle cover/crouch stance

**Key Events (NetIn → client)**:
- `onEffectResults` — Damage/heal numbers, effect application
- `TimerUpdate` — Cooldown and warmup timers
- `onStatUpdate` / `onStatBaseUpdate` — HP, focus, stat changes
- `onMeleeRangeUpdate` — Melee range for current weapon
- `onThreatenedMobsUpdate` — Threat list

**Interface**: `SGWCombatant.def` — 44 properties, 15 cell methods, 4 client methods
**Original server reference**: `deprecated/python/cell/AbilityManager.py` (1,091 lines — it holds the effect lifecycle too; there is no separate `EffectManager.py`)
**RE doc**: [combat-system.md](combat-system.md)

---

## Abilities

**Status**: IM — Ability activation works for direct-target, cone, and AoE abilities, plus channeled abilities (shipped in PR #420 — see [../architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)). Remaining gaps: chain targeting and the combo system.

**Data**: 1,886 abilities seeded in `db/resources/Abilities/`
**Schema**: `Ability.xsd` defines ability structure
**Enums**: `TargetingMode`, `AbilityRange`, `AbilityType` in enumerations.xml

**RE doc**: [ability-system.md](ability-system.md)

---

## Weapon Ammo & Reload

**Status**: IM — Server-authoritative per-bandolier-slot ammo. Fire-gate validates `required_ammo` against `active_ammo()`, decrements via `set_slot_ammo`, mirrors to `Stat[AMMO_SLOT_1+slot]`. `requestReload(EReloadType)` (cell method 86 on SGWPlayer) sets a warmup deadline; a 100 ms tick refills the magazine. Persistence is batched (reload completion / slot swap / ammo change / logout / world transition).

**Key cell methods (NetOut)**:
- `requestReload(EReloadType)` — opcode 86 on SGWPlayer
- `requestActiveSlotChange(BagId, SlotId)` — opcode 41 on SGWInventoryManager
- `requestAmmoChange(ItemId, AmmoType)` — opcode 42 on SGWInventoryManager

**Key client method (NetIn)**: `onStatUpdate` (method 20) carrying the AmmoSlot{N} stat — the bandolier UI's only refresh trigger.

**RE doc**: [weapon-ammo-reload.md](weapon-ammo-reload.md)

---

## Effects

**Status**: IM — Effect application/removal, pulsing DoT/HoT effects, stacking semantics, and absorption shields all work (shipped in PR #420 — see [../architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)). Remaining gaps: diminishing returns, and **no effect visuals** — nothing under `cell/effects/` emits `onSequence`, so events 2000–2008 are never sent.

**Data**: 3,216 effects seeded in `db/resources/Effects/`
**Schema**: `Effect.xsd` defines effect structure
**Key concepts**: Duration, pulse interval, stat modifiers, conditions, cleanup

**RE doc**: [effect-system.md](effect-system.md)

---

## Stats

**Status**: 50% — Base stats and some derived stats work. Missing: full derived stat calculation, regen formulas, level scaling curves, stat caps.

**Key properties** (SGWCombatant.def): `health`, `healthMax`, `focus`, `focusMax`, `armor`, `meleeRange`, `level`
**Architecture**: 6-tier stat dictionary, ported to `crates/entity/src/stats/`
**Enums**: 70+ stat types in enumerations.xml (`StatType`)

**RE doc**: [stat-system.md](stat-system.md)

---

## Inventory

**Status**: IM — Item storage, equipping, moving, and the full vendor stack (purchase / sell / buyback / repair / recharge, plus paid-repair and paid-recharge) all work. Vendor operations are confined to a bag allowlist so they cannot touch the bank, mail attachments, or loot bags. Missing: stat recalculation on equip, org vault integration, and the client-initiated `repairItemRequest` cell method (still a stub — repair only works through the vendor store path).

**Key Events (NetOut)**:
- `MoveItem`, `RemoveItem`, `UseItem`, `LootItem`
- `PurchaseItems`, `SellItems`, `BuybackItems`
- `RepairItem`, `RechargeItem`

**Key Events (NetIn)**:
- `onContainerInfo` — Full container sync
- `onUpdateItem` — Single item update
- `onRemoveItem` — Item removed
- `onStoreOpen` / `onStoreClose` — Store UI

**Interface**: `SGWInventoryManager.def` — 9 properties, 13 cell methods, 6 client methods
**Data**: 6,059 items in `db/resources/Items/`
**RE doc**: [inventory-system.md](inventory-system.md)

---

## Missions

**Status**: IM — Mission accept, step advancement, and objective tracking work; the FindAmbernol chain in Castle Cellblock runs end-to-end with region-enter, interact, kill, and use-item objectives all advancing. State persists to `sgw_mission`. Missing: reward selection, mission sharing, and verification of chains outside Castle Cellblock.

**Key Events (NetOut)**: `MissionAssign`, `MissionAdvance`, `MissionComplete`, `ChosenRewards`, `ShareMission`
**Key Events (NetIn)**: `onMissionUpdate`, `onStepUpdate`, `onObjectiveUpdate`, `onTaskUpdate`, `MissionOffer`, `MissionRewards`

**Interface**: `Missionary.def` — 7 properties, 20 cell methods, 5 client methods
**Data**: 1,040 missions in `db/resources/Missions/`
**RE doc**: [mission-system.md](mission-system.md)

---

## Gate Travel

**Status**: IM — Zone transition works: `base/world_entry/gate_travel/` sends RESET_ENTITIES, persists the destination world and position, and replays the world-entry flow against the new space. Ring transport is implemented separately. Missing: all gate animation sequences — `Stargate_MakeGate` and `Stargate_CrossGate` are never emitted, and the DHD chevron-lock events (6106–6112) are never triggered.

**Key Events (NetOut)**: `onDialGate`, `DHD`, `SetRingTransporterDestination`
**Key Events (NetIn)**: `setupStargateInfo`, `onStargatePassage`, `onDisplayDHD`, `onDHDReply`, `onRingTransporterList`

**Interface**: `GateTravel.def` — 5 properties, 4 cell methods, 4 client methods, 2 base methods
**Data**: 29 stargate addresses defined
**RE doc**: [gate-travel.md](gate-travel.md)
**See also**: [ring-transport-system.md](ring-transport-system.md)

---

## Chat

**Status**: KM — Only spatial chat works. Say, emote, and yell fan out to AoI witnesses; eight channels are registered with the client on login; DND and GM speaker flags are computed. Tells, team/squad/command delivery, user channels, moderation, ignore, and petitions are all unimplemented — only five chat base methods are dispatched, two of them acknowledge-only.

**Interface**: `Communicator.def` — 5 properties, 11 base methods, 7 client methods, 1 cell method
**RE doc**: [chat-system.md](chat-system.md)

---

## Crafting

**Status**: KM — Phase 1 only. The crafting state model and its persistence work, and GM expertise / applied-science grants push `onUpdateDiscipline`. All six player-facing activities (craft, research, reverse engineer, alloy, spend ASP, respec) are stubs. Tracked in #567.

**Key Events (NetOut)**: `Craft`, `Research`, `ReverseEngineer`, `Alloy`, `RespecCraft`, `SpendAppliedSciencePoint`
**Key Events (NetIn)**: `onUpdateDiscipline`, `onUpdateCraftingOptions`, `onUpdateKnownCrafts`, `onCraftingRespecPrompt`, `onDisciplineRespec`

**Data**: 499 recipes, crafting disciplines, blueprints, paradigms
**Concepts**: Applied Science (tech trees), racial paradigms, material alloys, reverse engineering
**RE doc**: [crafting-system.md](crafting-system.md)

---

## Vendors

**Status**: IM — The full stack is implemented in `base/world_entry/methods/vendor/`: store open, purchase, sell, buyback, repair, recharge, plus the paid-repair and paid-recharge variants. Handled through the inventory manager rather than a dedicated vendor entity. Operations are restricted to `VENDOR_FILTER_BAGS` (main bag, bandolier, the eleven equipment slots, quick bar) so they cannot reach the bank, mail attachments, or loot bags.

**Key Events (NetOut)**: `PurchaseItems`, `SellItems`, `BuybackItems`, `RepairItem`, `RechargeItem`
**Key Events (NetIn)**: `onStoreOpen`, `onStoreClose`, `onUpdateItem`
**Interface**: `SGWInventoryManager` (vendor flows route through the inventory manager)
**See also**: [inventory-system.md](inventory-system.md)

---

## Organizations

**Status**: KM — Not functional. Cell methods 8–19 decode their payloads and log `UNIMPLEMENTED`; no base handler, no DB table, and none of the `onOrganization*` client methods (34–51) is ever sent. Blocked on the [group system](#groups).

**16 NetIn events** for organization state sync (roster, ranks, cash, XP, MOTD, notes)
**15 NetOut events** for organization actions (create, invite, kick, rank change, etc.)

**Interface**: `OrganizationMember.def` — 8 properties, 45+ cell methods, 16 client methods, 3 base methods
**Organization types**: Command (guild), Squad (group), Team
**RE doc**: [organization-system.md](organization-system.md)

---

## Minigames

**Status**: IM — The SmartFoxServer 1.x host, session-ticket handshake, and content-chain launch/victory callback all work. Livewire is fully ported; six game types use an auto-win placeholder (matching the original server); Alignment and GoauldCrystals are not yet ported, and an unrecognised name silently falls back to the placeholder. The player-facing `MinigamePlayer` cell methods (manual start, spectating, helper calls) are stubs. See [minigame-system.md](minigame-system.md).

**Known minigames**: 10 minigame types referenced in data
**Architecture**: MinigamePlayer interface (25 properties, 78 methods) — the largest interface by method count

**Key Events**: `StartMinigame`, `EndMinigame`, `MinigameCallRequest/Accept/Decline/Abort`
**RE doc**: [minigame-system.md](minigame-system.md)

---

## Mail

**Status**: IM — Read side works: headers, body (with read-time stamping), delete, and archive, all ownership-checked. Player-composed sending, return-to-sender, attachment claim, and COD are unimplemented. The header query also ignores `bArchive`, so archived mail still shows in the inbox. The one server-generated mail *sender* (`send_mail_to_player`, driven by the Black Market sweep) is on unmerged PR #586, so on `main` nothing writes to `sgw_gate_mail`.

**Features**: Send/receive mail, item attachments, currency attachments, COD (cash on delivery), return to sender
**9 NetOut events**, **4 NetIn events**
**Interface**: `SGWMailManager.def` — 4 properties, 9 cell methods, 3 client methods, 1 base method
**Data**: `sgw_gate_mail`
**RE doc**: [mail-system.md](mail-system.md)

---

## Trading

**Status**: IM — Wired end-to-end. Cell methods 104–107 drive the session; `onTradeState` (144) and `onTradeResults` (145) go back to both clients. Lock state machine, version tracking, a 5.0-unit range gate, and disconnect teardown all work; the final swap is a single base-side sqlx transaction. Not yet verified with two live clients, and the partner-facing proposal is padded with sentinel item stubs rather than real item detail.

**Features**: Player-to-player trade windows, item/currency proposals, lock and confirm
**Events**: `tradeRequest`, `tradeRequestCancel`, `tradeUpdateProposal`, `tradeLockState` (NetOut); `onTradeState`, `onTradeResults` (NetIn)
**RE doc**: [trade-system.md](trade-system.md)

---

## Black Market

**Status**: KM on `main` — 94 lines of handler stubs that log and drop (`cell/cell_methods/black_market.rs`, `cell/client_methods/black_market.rs`). No base handler, no `sgw_auction` table, no `onBM*` ever sent.

Phase 1 exists on the **unmerged** branch `feat/571-black-market-phase1` (PR #586): search, create, bid, and cancel work end-to-end with item escrow, outbid refunds, and a 30-second expiry sweep that settles by mail. Item watching is still a stub there; immediate buyout falls through to the sweep; three wire constants are placeholders pending a debugger capture.

**Features**: Auction house for player-listed items. Search, bid, buyout, create/cancel auctions.
**Events**: `BMSearch`, `BMCreateAuction`, `BMCancelAuction`, `BMPlaceBid` (NetOut, cell methods 61–66); `onBMOpen`, `onBMAuctions`, `onBMAuctionUpdate`, `onBMAuctionRemove`, `onBMError` (NetIn, client methods 90–95)
**Entity**: `SGWBlackMarket` — dedicated base entity for the auction system
**Interface**: `SGWBlackMarketManager.def` — 1 property, 7 cell methods, 5 client methods, 5 base methods
**RE doc**: [black-market.md](black-market.md)

---

## Pets

**Status**: KM — Not implemented; there is no pet module in `crates/`. Entity flags, `EPetStance`, ~65 summon/command/buff abilities, and a complete client-side `GamePet` all exist, but the server-side summon/command/despawn lifecycle is greenfield. Tracked in #570.

**Entity**: `SGWPet` (extends SGWMob) — 8 properties, 8 cell methods, 3 client methods
**Features**: Pet summoning, ability control, stance management, pet leveling
**Events**: `PetInvokeAbility`, `PetAbilityToggle`, `PetChangeStance` (NetOut), `PetAbilities`, `PetStances`, `PetStanceUpdate` (NetIn)
**RE doc**: [pet-system.md](pet-system.md)

---

## Dueling

**Status**: KM — Not implemented. `sendDuelResponse` (CM 102) and `duelForfeit` (CM 103) are dispatched but log `UNIMPLEMENTED` and drop; no challenge method is dispatched at all.

**Entity**: `SGWDuelMarker` — placed in world to define duel area
**Events**: `DuelChallenge`, `DuelResponse`, `DuelForfeit` (NetOut), `onDuelChallenge`, `onDuelEntitiesSet`, `onDuelEntitiesRemove`, `onDuelEntitiesClear` (NetIn)
**RE doc**: [duel-system.md](duel-system.md)

---

## Groups

**Status**: KM — Definition-only. No `SGWPlayerGroupAuthority` instance exists in the Rust server, no group registry, and no handler for any of the four base methods. The [organization system](#organizations) is blocked on this.

**Entity**: `SGWPlayerGroupAuthority` (extends SGWEntity, implements GroupAuthority)
**Interface**: `GroupAuthority.def` — 3 properties, 4 base methods
**Organization types** double as group types (Squad = party)
**RE doc**: [group-system.md](group-system.md)

---

## Contact Lists

**Status**: CW — Confirmed working in-game. All six cell methods (55–60) and five client methods (85–89) are wired; lists and members persist to `sgw_contact_list` / `sgw_contact_list_member`; every character gets Friends and Ignore on first login; and all four `EContactListEvent` types fire from real game-state changes. Gap: nothing consults the `Ignore` list to suppress anything.

**Features**: Friend lists, ignore lists, custom lists. Online notifications.
**6 NetOut events**, **5 NetIn events**
**Interface**: `ContactListManager.def` — 1 property, 6 cell methods, 5 client methods, 2 base methods
**RE doc**: [contact-list.md](contact-list.md)

---

## NPC AI

**Status**: IM — All 12 AI states are wired in `cell/service/npc_ai/`. Behavior states (Patrol, Wander, Investigating, Follow) run off the 2-second `npc_ai_tick`; terminal states (Despawning, Submit, Error) are reachable via the `SetNpcAiState` content action. Threat preempts any live state into Fighting with per-state scratch preserved. Cover selection is integrated into the Fighting state. Known gap: `move_speed` is a hardcoded 0.6 units/tick regardless of AI state, so the broadcast `EMobMovementType` changes but the actual traversal speed never does.

**Key concepts**: AI state machine (12 states), threat table, aggro radius, three-bucket ability selection, aggression system, `LEASH_DISTANCE = 50`
**RE doc**: [npc-ai.md](npc-ai.md)

---

## Spawn System

**Status**: IM — Confirmed working in-game: NPCs and world objects spawn visibly in Castle Cellblock and are interactable. Implemented in `cell/spawner/` (NPCs, dialogs, loot, missions, regions, respawners, stargates, abilities), with a 1 Hz `npc_respawn_tick` handling Dead → Idle promotion from `respawn_secs`.

**Key concepts**: Spawn regions (area-based), spawn sets (NPC templates), weighted spawn tables, population caps, respawn timers
**Data**: 154 entity templates
**RE doc**: [spawn-system.md](spawn-system.md)

---

## Loot

**Status**: IM — `cell/abilities/loot_drop.rs` rolls each entry in the NPC's loot table independently on death and, if anything drops, sets the interaction flag that makes the client render the loot cursor. Loot tables themselves remain sparsely populated.

**Key concepts**: Independent probability rolls per entry, eligibility checks, loot interaction handler
**RE doc**: [loot-system.md](loot-system.md)

---

## Death & Respawn

**Status**: IM — Death state, corpse lifecycle, and respawn placement work. On death the player enters a downed/dead state; respawn returns the player to a selected respawn point.

**Key concepts**: Death-state transition on zero health, corpse entity lifecycle, respawn-point selection, post-respawn AoI refresh
**Interface**: `SGWCombatant` (death state) / `SGWPlayerRespawner` (respawn placement)
**Original server reference**: `deprecated/python/cell/SGWPlayer.py`
**RE doc**: [death-respawn-system.md](death-respawn-system.md)
**See also**: [respawn-lifecycle (RE findings)](../reverse-engineering/findings/respawn-lifecycle.md)

---

## XP & Leveling

**Status**: IM — giveExperience() works, placeholder LEVEL_EXP table. Missing: real XP curve, stat scaling on level-up, training point awards.

**Key concepts**: XP thresholds, level cap (20), training points, applied science points, archetype stat growth
**Rust**: `base/world_entry/methods/progression/`
**RE doc**: [progression-system.md](progression-system.md)

---

## Character Creation

**Status**: IM — `createCharacter` (0xC4) parses the payload, validates the visual choices, and inserts into `sgw_player`, resolving alignment / archetype / gender / body set / starting world and coordinates from the CharDefId. Starting bags are allocated in a fixed fill order; failures return `charCreateFailed`. Covered by live-DB tests.

**Key concepts**: Archetype selection, visual choices, name validation, starting loadout, character deletion
**Rust**: `base/character_create.rs`, `base/chardef.rs`
**RE doc**: [character-creation.md](character-creation.md)

---

## Cinematics

**Status**: KM — Sequences fire for ability begin/end, entity death, item equip/unequip/reload/use, ring transport, the content-chain `play_sequence` action, and the debug console commands. Nothing fires for effect lifecycle (init / removal / pulse / per-QR hit), ability interrupt/failed, stargate dialing or crossing, DHD chevrons, designer slots, or entity spawn/despawn. **No emit site sends NVP parameters** — all seven hardcode `NameValuePairs count = 0`, and nothing reads `resources.sequences_nvp`, so voiceover sound banks and VFX parameters never reach the client.

**Key Events (NetIn → client)**:
- `onSequence` (client method 1) — Play a Kismet/Matinee sequence (8 args: seqId, source, target, primaryTarget, impactTime, NVPs, viewType, instanceId)
- `onKismetEventSetUpdate` (client method 9) — Update entity's default event set; sent on AoI create

**Interface**: `SGWSpawnableEntity.def` — `kismetEventSetId` property, `shouldSendKismet` flag
**Data**: 1,973 sequences, 675 event sets, 1,958 event-set↔sequence links, 2,042 NVPs (533 `SoundBankName`, 1,509 VFX/animation params), 2,767 item event sets, 66 event types
**RE doc**: [cinematic-system.md](cinematic-system.md)

---

## Ring Transport

**Status**: IM — Implemented in `cell/ring_transport/` (dispatch, regions, runtime, transporter, wire). Console activation triggers the teleport and the `Region_Teleport_Out` / `_In` sequences fire. Missing: multi-player sync (only the first player in the region gets the Matinee — playing it per-player corrupts the shared ring prop animation), ring platform visual effects, and a proper activation UI (currently a small console on the ground). The visibility-safety sequence the original played on show is also unported.

**Key Events (NetOut)**: `SetRingTransporterDestination`
**Key Events (NetIn)**: `onRingTransporterList`, `onSequence` (Kismet matinee)

**Architecture**: 8-state FSM: IDLE → SEND_WAIT → SEND_WARMUP → REMOTE_LOAD_WAIT → REMOTE_WARMUP → COOLDOWN. Timed transitions (3.5s hide, 4.0s teleport, 3.0s reveal, 2.5s unlock).

**Interface**: `GateTravel.def` — ring transport methods
**RE doc**: [ring-transport-system.md](ring-transport-system.md)

---

## Priority Matrix

### Must Have (blocks playability)

1. **Combat and effect visuals** — nothing under `cell/effects/` emits `onSequence`, so hits, crits, pulses, and effect application have no VFX. Combat resolves correctly but looks inert
2. **NVP parameter overrides** — no emit site sends name-value pairs, so dialog voiceover sound banks and weapon VFX parameters never reach the client
3. **Mission completion** — reward selection and the full completion flow
4. **Stat system** — derived stats, regen, level scaling
5. **XP & leveling** — real XP curve, stat scaling on level-up, training point awards (see [progression-system.md](progression-system.md))
6. **NPC movement speed** — a single hardcoded speed for every AI state; patrol and combat-advance look identical in motion

### Should Have (core MMO features)

7. **Chat beyond spatial** — tells, group/guild channels, ignore enforcement
8. **Groups** — the blocker under organizations; nothing exists yet
9. **Organizations** — guild creation, roster, ranks
10. **Mail send path** — receive works; player-composed sending, attachments, and COD do not
11. **Stargate animations** — zone transition works, but no gate opens or closes on screen

### Nice to Have (polish)

12. **Crafting activities** — the six player-facing operations, on top of the working state layer
13. **Black Market** — land PR #586 first (nothing is on `main`), then phase 2: item watching, immediate buyout, real error/duration constants
14. **Minigames** — port Alignment and GoauldCrystals; stop falling back to auto-win on an unknown name
15. **Trading** — two-client verification and real partner-side item detail
16. **Pets** — pet summoning and control
17. **Dueling** — PvP duels
18. **Player cover** — cover detection fires content triggers but has no combat effect

---

> For per-feature status tracking with dependency chains and implementation confidence levels, see the [Gap Analysis](../gap-analysis.md).
>
> For content-level data audit (what content populates each system, orphan rates, cross-reference integrity), see the [Content Data Audit](../content/README.md).
