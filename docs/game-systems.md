---
title: "Game Systems"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Game Systems

Every major game system identified in Stargate Worlds, what it does, and how far along it is in the emulator.

> **Where the implementation lives.** Active development is Rust, under `crates/`. Any `deprecated/python/…` path cited below is the *original* 2009-era server, kept as evidence of design intent only — it is not running code, and line counts from it are not a measure of progress. There is no longer a `python/` tree at the repo root. Each system below reports the Rust status.

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

**Server:** Confirmed working in-game — players fight enemies, deal and receive damage, and kill NPCs in Castle Cellblock. The QR formulas, 5 damage types, and armor/absorption/mitigation math are implemented in `crates/cell-combat/src/cell/combat/` and `cell/abilities/`. Channeled abilities, cone AoE, pulsing DoT/HoT, absorption shields, stun/suppression, and threat/aggro all work. **Known gap:** no combat *visuals* — nothing under `cell/effects/` emits `onSequence`, so hit, crit, pulse, and effect-application VFX never play (see [cinematic-system.md](gameplay/cinematic-system.md)).

**Leash and reset (NA12):** an NPC gives up a fight when it is itself more than its leash radius from its spawn (`entity_templates.leash_distance`, default 50 u, with a 5 u hysteresis band), or when its last target dies, disconnects or stays out of its AoI for 5 s. It then walks home on the navmesh, ignoring damage and threat while it does. On arrival it heals to full, faces its spawn heading, clears its cooldowns and ignores players for 5 s. It snaps home only when no route exists or the walk takes more than 20 s. Every player it was fighting leaves combat, so their regen resumes. Details: [npc-ai.md](gameplay/npc-ai.md#leash-and-reset-na12).

**Proximity aggro (NA13):** an Idle NPC that is hostile to players attacks the closest player within 18 u horizontally (`entity_templates.aggro_radius`), within 4 u of its height, and in line of sight. Line of sight comes from the world's collision occluder (`data/spaces/<world>.occ`, #797), which all 23 game worlds ship (every navmesh except `sandbox`); the navmesh ray is the fallback only where no occluder exists. Hostility is the spawn's aggression override (`spawnlist.aggression_override`, or a chain's `set_aggression`), otherwise the 2009 faction reaction table: faction-10 mobs are hostile, faction 1 and 3 are friendly. The Cellblock mobs a chain arms (the first guard, chain 1008, and the PRU, chain 1032) are seeded neutral so their chain still starts the fight. GMs are aggroed like players unless they type `.aggro off`. Details: [npc-ai.md](gameplay/npc-ai.md#aggression-system).

**Same-room assist (NA14):** when an NPC engages from damage or proximity, hostile NPCs of its faction within 10 u of it (`entity_templates.assist_radius`), on its floor and in line of sight, that are Idle, patrolling or wandering, join on the same target. Assist does not chain, and a content chain's threat does not recruit. This is a deliberate deviation from the 2009 server, which had no assist. Details: [npc-ai.md](gameplay/npc-ai.md#same-room-assist-na14-d-na04).

### Cover System

SGW has a cover-based combat mechanic with adjustable cover weights and stances. Cover links define where players can take cover in each zone.

**Data:** world-space cover seeds in `db/resources/AI/Seed/`, extracted from the cooked `.umap` chunks by the `cover_extract` tool in `crates/navmesh-extractor` (NA21, #780): 236 nodes in 58 sets for Castle_CellBlock and 3,788 nodes in 481 sets for Castle. No other world has cover data. The earlier 1,380 sets / 9,346 nodes from `covernodes_*.pak` were dropped by #780. **Server:** Implemented — `crates/cell-cover/src/cell/cover/` loads the nodes at cell startup, indexes them in a uniform grid, and provides slot reservation and scoring. Wired into NPC combat: when an NPC has `use_cover` set and is not stationary, `maintain_cover_for_npc` treats cover as a firing position (NA22): the NPC takes or holds a slot only if it is within attack range of the target and gives it a shot, and releases it when flanked or out of reach. Cover is released on combat end and on leash. Players get a separate 1 Hz proximity-detection sweep (`COVER_PROXIMITY_RADIUS = 5.0`) that fires `OnPlayerEnteredCover` / `OnPlayerLeftCover` / `OnPlayerInCoverDuration` content-engine triggers, so chain authors can gate quest steps on cover state. `setCrouched` toggles the `BSF_CROUCHING` state flag and echoes `onStateFieldUpdate` to the caller (not yet to witnesses). What is still missing is any *combat* consequence of player cover — the QR pipeline expects crouch/cover to arrive as stat modifiers, and nothing applies them.

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

**Data:** 29 stargates with addresses in the database. **Server:** Zone transition is implemented — `base/world_entry/gate_travel/` tears down the client's view with RESET_ENTITIES, persists the destination world and position, and replays the world-entry flow against the new space. Ring transport is implemented in `cell/ring_transport/`. Gate animations match the 2009 server since #663 (CA10): `Stargate_MakeGate` (6100) fires 4 s after a successful dial and `Stargate_CrossGate` (6113) on the crossing, both fanned out to witnesses (`cell/gate_travel/sequences.rs`). The seven chevron events (6106-6112) and `Stargate_DestroyGate` are not sent, as in 2009. **Known gap:** the multi-player fan-out has had no two-client run.

## Inventory

Multiple container types:
- **Personal inventory** (main bag)
- **Equipment** (worn gear with visual components)
- **Mission items** (quest-related)
- **Crafting materials**
- **Vault** (bank storage)
- **Team/Command/Org vaults** (shared storage)

Items have stacking, charges, durability, and can trigger abilities when used. NPC stores support buy, sell, buyback, repair, and recharge.

**Data:** 6,059 items seeded in `db/resources/Items/`. **Server:** Confirmed working in-game — items are given to players during quest progression and appear in inventory. The full vendor stack is implemented in `base/world_entry/methods/vendor/`: purchase, sell, buyback, repair, recharge, plus the paid-repair and paid-recharge variants. Vendor operations are restricted to a fixed bag allowlist (`VENDOR_FILTER_BAGS`) so they cannot reach into the bank, mail attachments, or loot bags. **Known gaps:** the store window could not open in the client until #609 moved `onStoreOpen`/`onStoreUpdate` to the correct indices, and no client has tested a vendor since. No world spawns a vendor: template 25 is the only vendor template and has no spawn row, so `.spawn 25` is the only route in. The client-initiated `repairItemRequest` cell method is still a stub, so repair only works through the vendor store path.

## Missions

Multi-step quest system with:
- Step-based progression with objectives and tasks
- Mission sharing between team members
- Reward selection (choose your reward)
- Mission history tracking

**Data:** 1,040 missions seeded in `db/resources/Missions/`. **Server:** Confirmed working in-game — FindAmbernol quest in Castle Cellblock runs end-to-end (region enter, interact, kill, use-item objectives all advance). The 2026-09-18 colo playtest also completed Castle missions 701-704 and 706 in the client ([playtest report](analysis/playtests/2026-09-18-colo-castle/)). Mission state persists to `sgw_mission`. Missions outside Castle Cellblock and Castle are not yet client-tested. Completion grants no reward: `reward_xp` and `reward_naq` are 0 in every seed row and nothing dispatches cash or item rewards (#310). Known issue: some quest entities missing `INT_MissionWorldObject` interaction type flag (bit 30) for visual outline glow.

## Crafting

- **Blueprints** — Recipes for creating items from components, learned from Blueprint items and research
- **Disciplines** — Crafting specializations (78 across 4 applied sciences: Biomedical, Materials, Power Systems, Electronic), learned with applied-science points
- **Racial Paradigms** — Five tech trees (Common, Human, Goa'uld, Asgard, Ancient) whose levels gate disciplines; raised by Racial Paradigm Guides
- **Research** — Use up an item for a chance at expertise and the blueprint that makes it
- **Reverse Engineer** — Break an item down into some of its components
- **Alloying** — Combine a component with lower-tier elementary components into a higher-tier material

**Data:** 498 blueprints (40 alloys) with component requirements, 78 disciplines. **Server:** restored server-side by the crafting campaign (2026-09-27), not yet run in a client: learning disciplines with applied-science points (earned at one per level), craft, research, reverse engineering and alloying on a 3-second induction that consumes only when it completes, a free two-step respec, crafting stations and Field Crafting Tools, Blueprint items and Racial Paradigm Guides, and the login sync. The owner's CR-14 UAT is next. See [crafting-system.md](gameplay/crafting-system.md) and the [crafting ledger](analysis/crafting/README.md).

## Organizations (Guilds)

Three tiers of player organizations:
- **Squad** — Small group (5-6 players)
- **Team** — Mid-size group
- **Command** — Large organization (guild equivalent)

Features include: rank system with customizable names and permissions, MOTD, member/officer notes, organization bank and XP, and PvP organization support.

**Data:** Entity definitions complete (23KB of properties). **Server:** implemented by the organizations campaign (2026-09-27, [ledger](analysis/organizations/README.md)), and **not yet client-verified**; the owner's two-client [UAT](guides/organizations-uat.md) is next. Squads live on the cell, never persisted: invite, accept, leave, leader-only kick and loot mode, disconnect, gate travel and squad chat (ORG-03, ORG-04). Teams and Commands live on the base, backed by `db/sgw/Organizations/`: founding at a registrar NPC, login restore with online and offline presence, invite, kick and rank change under the organization lock, leave and disband, MOTD, member and officer notes, the rank editor, and team, command and officer chat (ORG-02, ORG-05 to ORG-09), with GM `.squad_*` and `.org_*` commands (ORG-10). The Team and Command vaults and the treasury deposits and withdrawals come from the Bank and Vault campaign (BV-07, BV-08). Not there yet: organization experience (always 0), strike teams, and applying the squad loot mode to loot. See [organization-system.md](gameplay/organization-system.md) and [group-system.md](gameplay/group-system.md).

## Black Market (Auction House)

Player-to-player auction system for buying and selling items. Supports creating auctions, bidding, searching, and canceling.

**Server:** Phase 1 implemented on `main` (ported from PR #586 by packet BM-01, 2026-09-27; base side in `crates/base-methods/src/base/world_entry/methods/black_market/`), **not player-visible**: the client drops every `onBM*` method until the client patch ships (#587).

Search, create, bid, buyout and cancel work server-side and follow the client's wire contract (packet BM-02, through the codec crate the client patch shares). A listed item moves into the seller's server-held container 18; outbid players are refunded; a buyout settles at once; and a 30-second expiry sweep moves the item to the buyer (or back to the seller when unsold) and mails the seller the cash. Create, bid and cancel are honoured only at an open auctioneer. Persisted in `sgw_auction` + `sgw_auction_bid`. **Remaining** (see the [restoration plan](analysis/black-market/README.md)): the client patch, settlement on the social-systems mail API (BM-02b), the auctioneer content (BM-07) and the watch list (deferred, D4). See [black-market.md](gameplay/black-market.md).

## Mail System

In-game mail with:
- Attachments (items)
- Cash on Delivery (COD)
- Return to sender
- Archive

**Data:** `sgw_gate_mail` and the escrow table `sgw_gate_mail_item`. **Server:** Read side works — headers (inbox and archive listed separately), body (with read-time stamping), delete, and archive, all ownership-checked by `character_id`. Text-only player sending works (social-systems SS-M1): up to 10 recipients, offline ones included, with a flood limit, a 100-message mailbox cap and a reason for every refusal. Sending gift cash, an item or COD to one recipient works (SS-M2): 25 naquadah postage, the item held in escrow, and a mail holding an attachment cannot be deleted. Taking the cash and the item, paying COD (the price is mailed to the sender) and return-to-sender work (SS-M3). New-mail notification (a feedback line and a live header push to an online recipient) and the 30-day expiry (return, delete or quarantine, D-SS04) work (SS-M4). Server mail goes through one system-mail writer, which GMs reach with `.mail` / `.mailbox` / `.mail_expire` (SS-U1) and content chains with the `send_system_mail` action, as the debug hub's Gate Mail Clerk does (SS-U3). The Black Market expiry sweep still writes its settlement mail through its own `send_mail_to_player` (on `main` since BM-01) and is to move to that writer (BM-02b). Nothing is client-tested yet; the owner's [SS-UAT](analysis/social-systems/work-packets.md#ss-uat-owner-uat-colo-after-the-release) covers it. See [mail-system.md](gameplay/mail-system.md).

## Chat & Communication

- Local say, yell, and emote
- Private messages (tell)
- Multiple channel types: Squad, Team, Command, Officer, Platoon
- Channel management: join, leave, kick, ban, mute, password
- AFK and DND status messages

**Server:** Say, emote, and yell fan out to AoI witnesses, and since #737 (players in a shared world see each other) those witnesses include other players. The social-systems campaign (2026-09-27) added tells with AFK and DND replies, `chatIgnore` and a one-way Ignore filter (SS-C1), a flood limit and text rules (SS-00), GM broadcast through `/gmshout` and `.announce` (SS-C2), a channel allowlist, GM `.mute` / `.unmute` and a feedback line for every unimplemented Communicator method (SS-C3), and channel ids that match the client's, with no channel registration at login (SS-C4). Squad chat (ORG-04) and team, command and officer chat (ORG-09) work. Still missing: user channels, channel moderation and petitions. Nothing here has been checked with two real clients yet. See [chat-system.md](gameplay/chat-system.md).

## Deployables

Stationary objects a player places with a Scientist "Deployable:" ability; each pulses an effect around itself for a fixed lifetime.

**Data:** The cooked effects describe the shape (1012's 5065 "Pulser, 30 pulses x1 Second, Despawn Target on Finish" and 5066 "Medium Radius AE, -100F"); no row names the object. The "Kit:" items are crafting components. **Server:** Phase 0 (2026-09-28): 1012 Microwave Emitter works server-side through `resources.deployables`, an owned `SGWBeing` whose pulses damage hostile NPCs as its owner, removed on expiry, re-cast, or the owner's death, logout or zone change. Not client-tested. See [deployables.md](gameplay/deployables.md).

## Pets

Companion pets with their own abilities and stances. Players can command pets to use abilities, change stance, and toggle ability auto-use.

**Data:** Entity, entity flags (`ENTITYFLAG_Pet` and friends), `EPetStance` enum, and ~65 summon/command/buff abilities are all authored. The 2009 client has a complete `GamePet` class and pet UI. **Server:** Not implemented — there is no pet module in `crates/`. The original Python server only ever sent the ability and stance lists on spawn, so the summon/command/despawn lifecycle is greenfield. Tracked in #570. See [pet-system.md](gameplay/pet-system.md).

## Minigames

An extensive minigame framework with 10 types:
- Activate, Analyze, Bypass, Converse, Hack, Livewire, Goauld Crystals, Alignment, and more

Features matchmaking, spectating, and helper systems.

**Server:** The SmartFoxServer 1.x host the Flash minigame SWFs connect to is reimplemented in-process (`crates/minigame/src/minigame/`), along with the session-ticket handshake. Content chains launch minigames via `Action::StartMinigame` and receive a victory callback that runs follow-on chains. Livewire is fully ported; six game types (Hack, Activate, Analyze, Bypass, Converse, ConverseBasicHumanoid) use an auto-win placeholder, matching the original server. Alignment and GoauldCrystals are not yet ported — and an unrecognised game name falls back to the auto-win placeholder, so a missing port looks like a win. The player-facing `MinigamePlayer` cell methods (manual start, spectating, helper calls) are all stubs. See [minigame-system.md](gameplay/minigame-system.md).

## Dueling

PvP duel system with challenge/accept/decline, forfeit, and a duel area.

**Data:** `SGWDuelMarker` defined, not needed (D-SS24). **Server:** Challenge and response implemented (SS-D1): `sendDuelChallenge` (base 0xD9) prompts the target with `onDuelChallenge` [143], and `sendDuelResponse` (CM 102) accepts or declines. The accept starts a 5-second countdown on both clients; then the duel is engaged (SS-D2): both duelists are PvP-flagged for themselves and everyone around them, are in combat with each other, and may damage each other and nobody else. The duel ends (SS-D3) on `duelForfeit` (CM 103), on partner damage that would kill (the duelist is held at 1 HP instead, D-SS20, so duels are non-lethal), on death from anyone else, disconnect, teleport or gate travel, or 5 seconds outside the 40-unit arena. The winner hears "You won the duel" (879) and the loser a feedback line; nothing is awarded (D-SS22). A solo tester can duel `sparbot`, a wireclient partner, and GMs have `.duel_status` / `.duel_end` (SS-U2). Squad duels are refused, and nothing is client-tested yet. See [gameplay/duel-system.md](gameplay/duel-system.md).

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

NPCs and monsters spawn from fixed rows in `resources.spawnlist`, one entity per row, using 154 entity templates that define the NPC and world-object types. A dead NPC respawns at its row after `respawn_secs` (the spawnlist row's value, else the template's).

The 2009 design also had a population-control layer (`SGWSpawnRegion` / `SGWSpawnSet`: weighted random selection from spawn tables, population caps, set cooldowns, spawn regions grouping spawn points). **None of it is implemented** (#62): the Python server had empty stubs, `spawn_sets.sql` and `spawn_points.sql` are empty, and the shipped content does not need it. See [spawn-system.md](gameplay/spawn-system.md).

**Data:** 154 entity templates seeded in `db/resources/Entities/Seed/entity_templates.sql`. **Server:** Confirmed working in-game — NPCs and world objects spawn visibly in Castle Cellblock and are interactable. The spawn loaders live in `crates/cell-catalog/src/cell/spawner/` and populating spaces from them in `crates/cell-world/src/cell/space_manager/`, with respawn handled by a 1 Hz `npc_respawn_tick` that reads `respawn_secs` and promotes Dead NPCs back to Idle.

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
