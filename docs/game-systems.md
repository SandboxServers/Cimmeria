# Game Systems

Every major game system identified in Stargate Worlds, what it does, and how far along it is in the emulator.

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

**Data:** QR formulas, 5 damage types, armor/absorption/mitigation math all coded in Python (AbilityManager.py, 1,090 lines). **Server:** Confirmed working in-game — players fight enemies, deal and receive damage, and kill NPCs in Castle Cellblock.

### Cover System

SGW has a cover-based combat mechanic with adjustable cover weights and stances. Cover links define where players can take cover in each zone. The game has 1,332 cover node positions across its maps.

**Data:** 1,332 cover mesh entries in covernodes_nikols.pak (9,161 individual node positions). Binary format is parsed (22 bytes/node: position, facing, height, quality, slot angle). **Server:** Not implemented — CoverSet entity is a stub. Cover nodes need to be loaded and associated with map chunks at runtime.

## Abilities

Players have ability trees with training points. Abilities can be:
- **Targeted** (single enemy/ally)
- **AoE** (area of effect — radius or cone)
- **Ground-targeted** (click on the ground)
- **Auto-cycle** (automatically repeat)

Each ability has warmup time, channeling time, cooldowns, ammo costs, weapon requirements, and position requirements (front, flank, rear, above, below).

**Data:** 1,887 abilities in the database. **Code:** Full ability pipeline in Python (cooldowns, warmup, channeling, targeting, ammo, position checks). **Server:** Not yet tested end-to-end with a client — ability use, effects, and cooldowns need in-game verification.

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

**Data:** 29 stargates with addresses in the database. **Code:** Dialing sequence with timers and state validation coded in SGWPlayer.py; GateTravel.py is a stub. **Server:** Not yet tested — zone transitions via stargate need end-to-end verification.

## Inventory

Multiple container types:
- **Personal inventory** (main bag)
- **Equipment** (worn gear with visual components)
- **Mission items** (quest-related)
- **Crafting materials**
- **Vault** (bank storage)
- **Team/Command/Org vaults** (shared storage)

Items have stacking, charges, durability, and can trigger abilities when used. NPC stores support buy, sell, buyback, repair, and recharge.

**Data:** 6,060 items in the database. **Server:** Confirmed working in-game — items are given to players during quest progression and appear in inventory. Stores (buy/sell/repair) not yet tested.

## Missions

Multi-step quest system with:
- Step-based progression with objectives and tasks
- Mission sharing between team members
- Reward selection (choose your reward)
- Mission history tracking

**Data:** 1,041 missions in the database. 17 compiled mission scripts for 4 zones. **Server:** Confirmed working in-game — FindAmbernol quest in Castle Cellblock runs end-to-end (region enter, interact, kill, use-item objectives all advance). Other zones' missions not yet tested. Known issue: some quest entities missing `INT_MissionWorldObject` interaction type flag (bit 30) for visual outline glow.

## Crafting

- **Blueprints** — Recipes for creating items from components
- **Disciplines** — Crafting specializations (79 total across 5 applied sciences)
- **Racial Paradigms** — Race-specific crafting bonuses (6 types)
- **Research** — Learn new recipes
- **Reverse Engineer** — Deconstruct items for knowledge
- **Naqahdah** — Primary crafting resource (from Stargate lore)

**Data:** 499 blueprints with component requirements in the database. **Code:** Full crafting logic in Python (Crafter.py, 532 lines) — blueprint management, discipline leveling, research, reverse engineering, alloying. **Server:** Not yet tested in-game.

## Organizations (Guilds)

Three tiers of player organizations:
- **Squad** — Small group (5-6 players)
- **Team** — Mid-size group
- **Command** — Large organization (guild equivalent)

Features include: rank system with customizable names and permissions, MOTD, member/officer notes, organization bank and XP, and PvP organization support.

**Data:** Entity definitions complete (23KB of properties). **Code:** Not implemented — all 15 organization RPC methods in SGWPlayer.py are empty stubs. No persistence layer.

## Black Market (Auction House)

Player-to-player auction system for buying and selling items. Supports creating auctions, bidding, searching, and canceling.

**Data:** Entity definitions only. **Code:** Not implemented — SGWBlackMarket.py is an empty stub (both base and cell).

## Mail System

In-game mail with:
- Attachments (items)
- Cash on Delivery (COD)
- Return to sender
- Archive

**Data:** Database table exists. **Code:** Mail header retrieval works; sending, archiving, and deleting are stubs.

## Chat & Communication

- Local say, yell, and emote
- Private messages (tell)
- Multiple channel types: Squad, Team, Command, Officer, Platoon
- Channel management: join, leave, kick, ban, mute, password
- AFK and DND status messages

**Data:** 11 system channels defined. **Code:** Full channel management in Python (Chat.py, 365 lines) — channel creation, passwords, operators, private messaging, DND/AFK. **Server:** Not yet tested in-game.

## Pets

Companion pets with their own abilities and stances. Players can command pets to use abilities, change stance, and toggle ability auto-use.

**Data:** Entity defined. **Code:** Cell-side pet logic exists (SGWPet.py, 383 lines — spawning, abilities, stances); base-side is a stub. Player pet commands are stubs.

## Minigames

An extensive minigame framework with 10 types:
- Activate, Analyze, Bypass, Converse, Hack, Livewire, Goauld Crystals, Alignment, and more

Features matchmaking, spectating, and helper systems.

**Code:** 9 minigame handlers in Python (1,787 lines total). Alignment, Goauld Crystals, and Livewire have full logic; 6 others have placeholder handlers that accept any input. **Server:** Not yet tested in-game.

## Dueling

PvP duel system with challenge/accept/decline, forfeit, and duel area management.

**Data:** Entity defined. **Code:** Not implemented — duel methods in SGWPlayer.py are empty stubs. SGWDuelMarker.py is a stub.

## Trading

Direct player-to-player trading with a request, propose, lock, confirm flow.

**Code:** Full trade transaction logic in Python (Trade.py, 273 lines) — proposal/lock/confirm flow, inventory validation, cash transfer. **Server:** Not yet tested in-game.

## Contact Lists

Friend and ignore lists with multiple named lists per player, online notifications, and list management.

**Data:** Entity properties defined. **Code:** Not implemented — all 6 contact list methods in SGWPlayer.py are empty stubs.

## Space Queue

Instanced content queue system (think: dungeon finder). Queue, ready check, enter flow with strike team integration.

**Data:** Entity defined. **Code:** Not implemented — queue methods are stubs.

## Spawn System

NPCs and monsters spawn from configurable spawn sets:
- Weighted random selection from spawn tables
- Population caps and respawn cooldowns
- Spawn regions for organizing groups of spawn points
- 153 entity templates defining different NPC types

**Data:** 153 NPC templates. **Code:** Spawn logic, population tracking, and cooldowns coded in Python. **Server:** Confirmed working in-game — NPCs and world objects spawn visibly in Castle Cellblock and are interactable.

## Dialog & Interactions

5,406 dialog trees with screens and buttons, linked to NPCs via 4,662 interaction set maps. Dialog options can change based on mission state. Interaction types include vendors, ability trainers, loot, and DHD (Stargate dialing).

**Data:** 5,406 dialog trees, 4,662 interaction set maps. **Server:** Confirmed working in-game — dialog trees display correctly, NPC right-click triggers interaction scripts.

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

**Data:** 23 character definitions with customization data in the database. **Code:** Data loading only (CharacterCreation.py). **Server:** Barely implemented — character creation flow is not fully functional. Customization options are not properly served to the client.
