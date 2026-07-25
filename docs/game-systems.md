---
title: "Game Systems"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# Game Systems

Every major game system identified in Stargate Worlds, what it does, and how far along it is in the emulator.

> **Where the implementation lives.** Active development is Rust, under `crates/`. The `python/` scripts referenced in older revisions of this page are the *original* server, retained under `deprecated/python/` as evidence of intent only — they are not running code and line counts from them are not a measure of progress. Each system below reports the Rust status.

## Combat

SGW uses a **Quality Rating (QR)** system for combat resolution. Every attack rolls a quality value between 0 and 1, which determines the outcome:

| QR Range | Result |
|----------|--------|
| 0.00 - 0.07 | Miss |
| 0.07 - 0.20 | Glancing blow (reduced damage) |
| 0.20 - 0.80 | Normal hit |
| 0.80 - 0.93 | Critical hit |
| 0.93 - 1.00 | Double critical |

Damage is calculated as: base damage, modified by the QR roll, multiplied by stat resistance, armor factor, mitigation, and absorption. There are 5 damage types: Untyped, Energy, Hazmat, Physical, and Psionic — each with its own armor and absorption stats.

**Server:** Confirmed working in-game — players fight enemies, deal and receive damage, and kill NPCs in Castle Cellblock. The QR formulas, 5 damage types, and armor/absorption/mitigation math are implemented in `crates/services/src/cell/combat/` and `cell/abilities/`. Channeled abilities, cone AoE, pulsing DoT/HoT, absorption shields, stun/suppression, and threat/aggro all work. **Known gap:** no combat *visuals* — nothing under `cell/effects/` emits `onSequence`, so hit, crit, pulse, and effect-application VFX never play (see [cinematic-system.md](gameplay/cinematic-system.md)).

### Cover System

SGW has a cover-based combat mechanic with adjustable cover weights and stances. Cover links define where players can take cover in each zone.

**Data:** 1,380 cover sets / 9,346 cover nodes seeded in `db/resources/AI/Seed/`, extracted from the union of `covernodes_nikols.pak` and `covernodes_sdeiter.pak` via `tools/ue3_extract_cover_nodes.py`. **Server:** Implemented — `crates/services/src/cell/cover/` loads the nodes at cell startup, indexes them in a uniform grid, and provides slot reservation and scoring. Wired into NPC combat: when an NPC has `use_cover` set and is not stationary, `maintain_cover_for_npc` substitutes the chosen cover slot for the threat's position so the NPC paths to cover rather than charging. Cover is released on combat end and on leash. Players get a separate 1 Hz proximity-detection sweep (`COVER_PROXIMITY_RADIUS = 5.0`) that fires `OnPlayerEnteredCover` / `OnPlayerLeftCover` / `OnPlayerInCoverDuration` content-engine triggers, so chain authors can gate quest steps on cover state. `setCrouched` toggles the `BSF_CROUCHING` state flag and echoes `onStateFieldUpdate` to the caller (not yet to witnesses). What is still missing is any *combat* consequence of player cover — the QR pipeline expects crouch/cover to arrive as stat modifiers, and nothing applies them.

## Abilities

Players have ability trees with training points. Abilities can be:
- **Targeted** (single enemy/ally)
- **AoE** (area of effect — radius or cone)
- **Ground-targeted** (click on the ground)
- **Auto-cycle** (automatically repeat)

Each ability has warmup time, channeling time, cooldowns, ammo costs, weapon requirements, and position requirements (front, flank, rear, above, below).

**Data:** 1,886 abilities seeded in `db/resources/Abilities/`. **Server:** Working — ability activation for direct-target, cone, and AoE abilities, plus channeled abilities with movement-interrupt. Cooldowns, warmup, ammo gating, and per-ability range all enforced. Remaining gaps: chain targeting, the combo system, and diminishing returns.

## Stargates

The signature feature — functional Stargates for traveling between zones.

The flow works like this:
1. Server sends the player their list of known gate addresses
2. Player approaches a Stargate and the DHD (Dial Home Device) interface appears
3. Player dials a 7-symbol address
4. Chevron lock animations play
5. On success: player travels through the gate to the destination zone
6. On failure: error notification

There are also **Ring Transporters** for shorter-range travel within a zone.

**Data:** 29 stargates with addresses in the database. **Server:** Zone transition is implemented — `base/world_entry/gate_travel/` tears down the client's view with RESET_ENTITIES, persists the destination world and position, and replays the world-entry flow against the new space. Ring transport is implemented in `cell/ring_transport/`. **Known gap:** no gate *animations* — neither `Stargate_MakeGate` nor `Stargate_CrossGate` is ever emitted, and the DHD chevron-lock sequences are never triggered.

## Inventory

Multiple container types:
- **Personal inventory** (main bag)
- **Equipment** (worn gear with visual components)
- **Mission items** (quest-related)
- **Crafting materials**
- **Vault** (bank storage)
- **Team/Command/Org vaults** (shared storage)

Items have stacking, charges, durability, and can trigger abilities when used. NPC stores support buy, sell, buyback, repair, and recharge.

**Data:** 6,059 items seeded in `db/resources/Items/`. **Server:** Confirmed working in-game — items are given to players during quest progression and appear in inventory. The full vendor stack is implemented in `base/world_entry/methods/vendor/`: purchase, sell, buyback, repair, recharge, plus the paid-repair and paid-recharge variants. Vendor operations are restricted to a fixed bag allowlist (`VENDOR_FILTER_BAGS`) so they cannot reach into the bank, mail attachments, or loot bags. **Known gap:** the client-initiated `repairItemRequest` cell method is still a stub — repair currently only works through the vendor store path.

## Missions

Multi-step quest system with:
- Step-based progression with objectives and tasks
- Mission sharing between team members
- Reward selection (choose your reward)
- Mission history tracking

**Data:** 1,040 missions seeded in `db/resources/Missions/`. **Server:** Confirmed working in-game — FindAmbernol quest in Castle Cellblock runs end-to-end (region enter, interact, kill, use-item objectives all advance). Mission state persists to `sgw_mission`. Other zones' missions not yet tested. Known issue: some quest entities missing `INT_MissionWorldObject` interaction type flag (bit 30) for visual outline glow.

## Crafting

- **Blueprints** — Recipes for creating items from components
- **Disciplines** — Crafting specializations (79 total across 5 applied sciences)
- **Racial Paradigms** — Race-specific crafting bonuses (6 types)
- **Research** — Learn new recipes
- **Reverse Engineer** — Deconstruct items for knowledge
- **Naqahdah** — Primary crafting resource (from Stargate lore)

**Data:** 499 blueprints with component requirements in the database. **Server:** Phase 1 only. The crafting state model and its persistence work (`sgw_player` columns plus the normalised `sgw_player_discipline_expertise` table), and GM expertise / applied-science grants push `onUpdateDiscipline` to the client. All six player-facing activities — craft, research, reverse engineer, alloy, spend applied-science points, respec — are stubs that decode their arguments and log `UNIMPLEMENTED`. Tracked in #567. See [crafting-system.md](gameplay/crafting-system.md).

## Organizations (Guilds)

Three tiers of player organizations:
- **Squad** — Small group (5-6 players)
- **Team** — Mid-size group
- **Command** — Large organization (guild equivalent)

Features include: rank system with customizable names and permissions, MOTD, member/officer notes, organization bank and XP, and PvP organization support.

**Data:** Entity definitions complete (23KB of properties). **Server:** Not implemented. Twelve inbound cell methods (indices 8–19) decode their payloads and log `UNIMPLEMENTED`; there is no base-side handler, no organization table in `db/sgw/`, and none of the eighteen `onOrganization*` client methods is ever sent. Blocked on the group system, which is definition-only. See [organization-system.md](gameplay/organization-system.md).

## Black Market (Auction House)

Player-to-player auction system for buying and selling items. Supports creating auctions, bidding, searching, and canceling.

**Server:** Phase 1 implemented **on the unmerged branch `feat/571-black-market-phase1` — not on `main`.** On `main` the black market is still 94 lines of handler stubs (`cell/cell_methods/black_market.rs` + `cell/client_methods/black_market.rs`), and the [Gap Analysis](gap-analysis.md) counts it as missing until the branch lands.

On that branch: search, create, bid, and cancel work end-to-end, with item escrow on listing, cash refunds to outbid players, and a 30-second expiry sweep that settles auctions by mailing the winning cash to the seller and the item to the buyer (or returning the item on an unsold listing). Persisted in `sgw_auction` + `sgw_auction_bid`. **Remaining:** item watching is a stub, immediate buyout settlement is deferred to the sweep, and three wire constants (error ordinals, duration→hours mapping, next-min-bid formula) are placeholders pending a debugger capture. See [black-market.md](gameplay/black-market.md).

## Mail System

In-game mail with:
- Attachments (items)
- Cash on Delivery (COD)
- Return to sender
- Archive

**Data:** `sgw_gate_mail` table. **Server:** Read side works — headers, body (with read-time stamping), delete, and archive, all ownership-checked by `character_id`. Server-generated mail also works and is in production use by the Black Market settlement path. **Not implemented:** player-composed sending, return-to-sender, attachment claim, and COD payment. The header query also ignores the `bArchive` flag, so archived mail still shows in the inbox listing. See [mail-system.md](gameplay/mail-system.md).

## Chat & Communication

- Local say, yell, and emote
- Private messages (tell)
- Multiple channel types: Squad, Team, Command, Officer, Platoon
- Channel management: join, leave, kick, ban, mute, password
- AFK and DND status messages

**Server:** Spatial chat only. Say, emote, and yell fan out to AoI witnesses; eight channels are registered with the client at world entry; the DND flag and GM speaker flag are computed. Everything else — tells, team/squad/command delivery, user channels, moderation, ignore, petitions — is unimplemented. Only five chat base methods are dispatched at all, and two of those are acknowledge-only. See [chat-system.md](gameplay/chat-system.md).

## Pets

Companion pets with their own abilities and stances. Players can command pets to use abilities, change stance, and toggle ability auto-use.

**Data:** Entity, entity flags (`ENTITYFLAG_Pet` and friends), `EPetStance` enum, and ~65 summon/command/buff abilities are all authored. The 2009 client has a complete `GamePet` class and pet UI. **Server:** Not implemented — there is no pet module in `crates/`. The original Python server only ever sent the ability and stance lists on spawn, so the summon/command/despawn lifecycle is greenfield. Tracked in #570. See [pet-system.md](gameplay/pet-system.md).

## Minigames

An extensive minigame framework with 10 types:
- Activate, Analyze, Bypass, Converse, Hack, Livewire, Goauld Crystals, Alignment, and more

Features matchmaking, spectating, and helper systems.

**Server:** The SmartFoxServer 1.x host the Flash minigame SWFs connect to is reimplemented in-process (`crates/services/src/minigame/`), along with the session-ticket handshake. Content chains launch minigames via `Action::StartMinigame` and receive a victory callback that runs follow-on chains. Livewire is fully ported; six game types (Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid) use an auto-win placeholder, matching the original server. Alignment and GoauldCrystals are not yet ported — and an unrecognised game name falls back to the auto-win placeholder, so a missing port looks like a win. The player-facing `MinigamePlayer` cell methods (manual start, spectating, helper calls) are all stubs. See [minigame-system.md](gameplay/minigame-system.md).

## Dueling

PvP duel system with challenge/accept/decline, forfeit, and duel area management.

**Data:** Entity defined. **Server:** Not implemented — `sendDuelResponse` (CM 102) and `duelForfeit` (CM 103) are dispatched but only log and drop. No challenge method is dispatched at all.

## Trading

Direct player-to-player trading with a request, propose, lock, confirm flow.

**Server:** Implemented and wired end-to-end. Cell methods 104–107 drive the session; `onTradeState` (144) and `onTradeResults` (145) go back to the clients. The lock state machine, version tracking, a 5.0-unit range gate, and disconnect teardown all work, and the final swap is a single base-side sqlx transaction. Not yet verified with two live clients. See [trade-system.md](gameplay/trade-system.md).

## Contact Lists

Friend and ignore lists with multiple named lists per player, online notifications, and list management.

**Server:** Implemented and confirmed working in-game. All six cell methods (55–60) and five client methods (85–89) are wired; lists and members persist to `sgw_contact_list` / `sgw_contact_list_member`; every character gets Friends and Ignore on first login; and all four `EContactListEvent` types (login status, level gain, death, gate travel) fire from real game-state changes. **Gap:** nothing consults the `Ignore` list to actually suppress anything. See [contact-list.md](gameplay/contact-list.md).

## Space Queue

Instanced content queue system (think: dungeon finder). Queue, ready check, enter flow with strike team integration.

**Data:** Entity defined. **Server:** Not implemented.

## Spawn System

NPCs and monsters spawn from configurable spawn sets:
- Weighted random selection from spawn tables
- Population caps and respawn cooldowns
- Spawn regions for organizing groups of spawn points
- 154 entity templates defining different NPC and world-object types

**Data:** 154 entity templates seeded in `db/resources/Entities/Seed/entity_templates.sql`. **Server:** Confirmed working in-game — NPCs and world objects spawn visibly in Castle Cellblock and are interactable. Spawn logic lives in `crates/services/src/cell/spawner/`, with respawn handled by a 1 Hz `npc_respawn_tick` that reads `respawn_secs` and promotes Dead NPCs back to Idle.

## Dialog & Interactions

5,412 dialog trees with screens and buttons, linked to NPCs via 4,671 dialog set maps. Dialog options can change based on mission state. Interaction types include vendors, ability trainers, loot, and DHD (Stargate dialing).

**Data:** 5,412 dialogs, 13,467 dialog screens, 4,350 screen buttons, 4,671 dialog set maps, 1,178 dialog sets, 602 speakers. **Server:** Confirmed working in-game — dialog trees display correctly, NPC right-click triggers interaction scripts.

### Interaction Type System

Entity interaction types are controlled by a UINT64 bitmask (`EInteractionNotificationType`) that determines visual indicators shown to the player:

- **Bits 1-21**: NPC types (banker, vendor, trainer, minigames, etc.)
- **Bits 22-25**: A-Story mission states (pending, available, active, turn-in)
- **Bits 26-29**: Non-A-Story mission states
- **Bit 30**: `INT_MissionWorldObject` — quest item outline glow
- **Bit 31**: `INT_MissionWaypoint`
- **Bit 32**: `INT_DrossPile`

**Known issue:** Some entity templates have `interaction_type=0` and rely on mission scripts to set the correct flags dynamically via `setInteractionType()`. If a script omits this call, the entity will lack its quest visual indicator even though it is functionally interactable.

## Character Creation

23 character definitions across archetypes and genders with visual customization (body sets, component choices) and starting ability assignments.

**Data:** 23 character definitions with customization data in `db/resources/Archetypes/Seed/char_creation.sql`. **Server:** Implemented — `createCharacter` (0xC4) parses the payload, validates the visual choices, and inserts into `sgw_player`, resolving alignment / archetype / gender / body set / starting world and coordinates from the CharDefId via `chardef_lookup`. Starting bags are allocated in a fixed fill order. Failures return `charCreateFailed` to the client. Covered by live-DB tests in `character_create_live_db_tests.rs`.
