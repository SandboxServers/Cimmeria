# Network Messages

Every action in Stargate Worlds — swinging a weapon, opening a store, dialing a stargate — is a **network message** sent between the client and the server. This page catalogs all of them.

## How Messages Work

The client and server talk using the **Mercury protocol** (a custom UDP system). Each message has a name like `Event_NetOut_UseAbility` (client to server) or `Event_NetIn_onEffectResults` (server to client).

- **NetOut** = Client sends to Server (167 message types)
- **NetIn** = Server sends to Client (254 message types)
- **Total: 421 unique message types**

## Messages by Game System

### Login & Connection (14 messages)

Getting from the login screen to the game world. Includes account login, shard selection, character list, character creation/deletion, and the "play" command that enters the world.

### Combat (25 messages)

Everything combat-related: using abilities, hit/damage results, line-of-sight checks, threat lists, targeting, crouching, weapon changes, ammo, and reloading. Also includes the "call for aid" system.

### Inventory & Items (23 messages)

Moving items around, equipping gear, using consumables, looting, and managing your bags. Also covers buying from stores, selling, buyback, repairs, and recharging items.

### Missions (23 messages)

Accepting, abandoning, advancing, and completing missions. Mission sharing with teammates. Reward selection. Plus GM commands for mission debugging (advance, reset, clear, set available).

### Chat & Communication (21 messages)

Local chat, private messages (tells), channel management (join, leave, list), ignore/mute, AFK and DND status messages. GM broadcast (shout).

### Organizations (21 messages)

Creating, joining, and leaving guilds (called "Commands" in SGW). Inviting members, kicking, rank management, rank permissions, MOTD, notes, and organization bank/XP tracking. Covers squads, teams, and commands.

### Stargates (9 messages)

Dialing gates via the DHD (Dial Home Device), gate addresses, rotation animations, travel events, and ring transporters.

### Crafting & Disciplines (11 messages)

Crafting items, alloying materials, researching new recipes, reverse engineering, spending science points, discipline management, and respec options.

### Minigames (18 messages)

Starting, ending, and completing minigames. Matchmaking (call/accept/decline), spectating, helper registration, and contact management.

### Mail (9 messages)

Sending and receiving in-game mail. Reading, deleting, archiving, returning to sender. Taking item attachments and currency. Cash-on-delivery payments.

### Black Market / Auction House (9 messages)

Creating auctions, canceling them, placing bids, searching listings. Opening/closing the auction interface. Error handling.

### Dueling (6 messages)

Challenging another player, accepting or declining, and forfeiting active duels.

### Trading (4 messages)

Direct player-to-player trading: request, cancel, propose items, and lock/confirm the trade.

### Pets (6 messages)

Commanding pets to use abilities, changing pet stance, toggling auto-abilities.

### Contact Lists (11 messages)

Friend and ignore lists: creating, deleting, renaming lists, adding/removing contacts, and receiving online notifications.

### Space Queue (4 messages)

The instanced content queue (like a dungeon finder): checking status, ready checks, and strike team management.

### World & Entity (13 messages)

World setup, time-of-day sync, map info, and entity lifecycle: spawning, moving, and despawning of other players and NPCs in your area.

### Character Stats & Progression (8 messages)

Level changes, XP gains, stat updates, equipment slot changes, and racial paradigm progression.

### UI & Navigation (13 messages)

Dialog windows, NPC interaction menus, trainer interfaces, DHD (stargate dialing) UI, waypoint/path display, and timer updates.

### GM Commands (40+ messages)

Game Master tools: teleportation, god mode, invisibility, spawning/despawning entities, giving items/abilities/XP, setting levels and stats, and various debug toggles.

### Debug & Hot-Loading (30+ messages)

Developer tools for combat debugging, AI behavior inspection, physics visualization, performance stats, and hot-reloading game data (abilities, items, missions, behaviors) without restarting the server.

## What This Means for the Emulator

Every one of these 421 messages needs a handler on the server side. The current implementation handles the core messages (login, entity creation, resource data, chat). The remaining messages are implemented as the corresponding game systems come online — for example, the combat messages get wired up when combat is integrated, the mission messages when missions are connected, and so on.

The complete message list is valuable because it tells us exactly what the client expects to send and receive. No guessing needed — every possible client-server interaction is accounted for.

For the full technical message catalog with exact event names, see [technical/network-messages.md](technical/network-messages.md).
