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

**Status: Confirmed working in-game.** The combat formulas, damage types, and hit resolution are all coded in Python and have been tested in live gameplay. Players can engage enemies, take cover, deal and receive damage, and kill NPCs in the Castle Cellblock zone.

### Cover System

SGW has a cover-based combat mechanic with adjustable cover weights and stances. Cover links define where players can take cover in each zone. The game has 1,332 cover node positions across its maps.

**Status: Data exists (binary format), logic defined but cover nodes need parsing.**

## Abilities

Players have ability trees with training points. Abilities can be:
- **Targeted** (single enemy/ally)
- **AoE** (area of effect — radius or cone)
- **Ground-targeted** (click on the ground)
- **Auto-cycle** (automatically repeat)

Each ability has warmup time, channeling time, cooldowns, ammo costs, weapon requirements, and position requirements (front, flank, rear, above, below).

**Status: Fully implemented.** 1,887 abilities defined in the database. Full ability pipeline coded including cooldowns, warmup, channeling, and targeting.

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

**Status: Fully implemented.** 29 stargates with addresses defined. Travel logic coded.

## Inventory

Multiple container types:
- **Personal inventory** (main bag)
- **Equipment** (worn gear with visual components)
- **Mission items** (quest-related)
- **Crafting materials**
- **Vault** (bank storage)
- **Team/Command/Org vaults** (shared storage)

Items have stacking, charges, durability, and can trigger abilities when used. NPC stores support buy, sell, buyback, repair, and recharge.

**Status: Confirmed working in-game.** 6,060 items in the database. Items are successfully given to players during quest progression and appear in inventory.

## Missions

Multi-step quest system with:
- Step-based progression with objectives and tasks
- Mission sharing between team members
- Reward selection (choose your reward)
- Mission history tracking

**Status: Confirmed working in-game.** 1,041 missions defined. 17 mission scripts for 4 zones. Mission accept/complete/fail/abandon lifecycle is coded. The FindAmbernol quest in Castle Cellblock runs end-to-end: region enter, interact, kill, and use-item objectives all advance correctly. One known issue: some quest entities are missing their `INT_MissionWorldObject` interaction type flag (bit 30, value 1073741824), which controls the visual quest outline glow. Scripts need to call `setInteractionType()` to set this dynamically.

## Crafting

- **Blueprints** — Recipes for creating items from components
- **Disciplines** — Crafting specializations (79 total across 5 applied sciences)
- **Racial Paradigms** — Race-specific crafting bonuses (6 types)
- **Research** — Learn new recipes
- **Reverse Engineer** — Deconstruct items for knowledge
- **Naqahdah** — Primary crafting resource (from Stargate lore)

**Status: Fully implemented.** 499 blueprints with component requirements. Full crafting logic coded.

## Organizations (Guilds)

Three tiers of player organizations:
- **Squad** — Small group (5-6 players)
- **Team** — Mid-size group
- **Command** — Large organization (guild equivalent)

Features include: rank system with customizable names and permissions, MOTD, member/officer notes, organization bank and XP, and PvP organization support.

**Status: Entity definitions complete (23KB of properties defined). No server-side logic implemented yet.**

## Black Market (Auction House)

Player-to-player auction system for buying and selling items. Supports creating auctions, bidding, searching, and canceling.

**Status: Entity definitions only. No implementation.**

## Mail System

In-game mail with:
- Attachments (items)
- Cash on Delivery (COD)
- Return to sender
- Archive

**Status: Database table exists. No server-side logic.**

## Chat & Communication

- Local say, yell, and emote
- Private messages (tell)
- Multiple channel types: Squad, Team, Command, Officer, Platoon
- Channel management: join, leave, kick, ban, mute, password
- AFK and DND status messages

**Status: Fully implemented.**

## Pets

Companion pets with their own abilities and stances. Players can command pets to use abilities, change stance, and toggle ability auto-use.

**Status: Entity defined. Basic framework in place.**

## Minigames

An extensive minigame framework with 10 types:
- Activate, Analyze, Bypass, Converse, Hack, Livewire, Goauld Crystals, Alignment, and more

Features matchmaking, spectating, and helper systems.

**Status: 3 of 10 fully implemented (Alignment, Goauld Crystals, Livewire). 7 have placeholder stubs.**

## Dueling

PvP duel system with challenge/accept/decline, forfeit, and duel area management.

**Status: Entity defined. Basic framework.**

## Trading

Direct player-to-player trading with a request, propose, lock, confirm flow.

**Status: Defined in entity system.**

## Contact Lists

Friend and ignore lists with multiple named lists per player, online notifications, and list management.

**Status: Entity defined and interface implemented.**

## Space Queue

Instanced content queue system (think: dungeon finder). Queue, ready check, enter flow with strike team integration.

**Status: Defined in entity system.**

## Spawn System

NPCs and monsters spawn from configurable spawn sets:
- Weighted random selection from spawn tables
- Population caps and respawn cooldowns
- Spawn regions for organizing groups of spawn points
- 153 entity templates defining different NPC types

**Status: Confirmed working in-game.** Spawn logic, population tracking, and cooldowns all coded. NPCs and world objects spawn visibly in Castle Cellblock and are interactable.

## Dialog & Interactions

5,406 dialog trees with screens and buttons, linked to NPCs via 4,662 interaction set maps. Dialog options can change based on mission state. Interaction types include vendors, ability trainers, loot, and DHD (Stargate dialing).

**Status: Confirmed working in-game.** Dialog trees display correctly during NPC interactions and mission sequences. Right-clicking world objects and NPCs triggers the correct interaction scripts.

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

**Status: Fully implemented.**
