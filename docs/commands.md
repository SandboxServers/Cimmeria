---
title: "Commands Reference"
type: reference
audience: players, GMs, operators
last_updated: 2026-06-17
---

# Commands Reference

Every slash command Stargate Worlds exposes, with — for each one — what it does,
whether **this** server actually handles it, whether there's an automated test
guarding it, the access level you need, its parameters, and a sample.

Player commands work for everyone. GM and debug commands require an elevated
account.

## How to read the status columns

| Column | Meaning |
|--------|---------|
| **Handled?** | Does this server implement a handler for the command? ✅ implemented · 🚧 partial · ❌ not server-handled (the client may still act on it) |
| **Tested?** | Is there an explicit automated test that exercises the handler? ✅ yes · — none |
| **Access** | Minimum account level (User / GM). GM commands also work for Admin and Developer accounts. |

> [!IMPORTANT]
> **The server side is tested; the client round-trip is not yet confirmed.** Every
> command marked ✅ Handled has server-side handler code and (where ✅ Tested) an
> automated test that drives that handler with byte-accurate arguments. What has
> **not** yet been verified by a manual UAT is that the real game client's console
> actually fires each command end-to-end and renders the result. Treat ✅ as
> "the server does the right thing when the call arrives", not "confirmed working
> in a live client". A live-client pass is still pending.

<!-- -->

> [!NOTE]
> **Why some player commands show "not server-handled".** Many player-facing
> console commands are handled entirely on the client, or map to game flows the
> server has not yet implemented. Where the server has no handler for a command,
> it's marked ❌ and the parameters/sample reflect the client's expected usage,
> not a confirmed server contract. Commands marked ❌ are documented for
> completeness — they are **not** evidence of a server feature.

<!-- -->

> [!NOTE]
> **How GM access is enforced.** Every GM/debug command is gated server-side by a
> single rule: a caller whose account access level is below **Game Master** is
> rejected before any handler runs. You do not need to be Admin or Developer for
> any command below — Game Master is sufficient for all of them. No command in
> this reference has been found to require Admin or Developer specifically.

---

## Command count and the "256 / 266" reconciliation

This catalog enumerates **168 distinct commands** across **179 documented rows**
— the human-readable set a player, GM, or operator would actually type. (A handful
of commands — `/dhd`, `/respawn`, `/missionassign`, `/missionabandon`,
`/missiondetails`, `/missionlist`, `/who`, `/users` — appear twice because they
have a distinct player form and a more capable GM form; those account for the
179-vs-168 difference.)

The larger figures you may have seen come from counting the underlying protocol
surface, not the typed-command list:

- The client's player entity exposes **109** server-callable methods, and the GM
  entity adds **117** more (the `gm*`/debug tail), for **226** exposed protocol
  methods. Not all of these have a typed `/command`; many are UI-driven or
  internal.
- Older copies of this page asserted "All 256 slash commands". That number was
  never substantiated by an enumerable list — the actual typed-command catalog
  below is **168 distinct commands**. The "256" (and the sometimes-quoted "266")
  appear to be rounded references to the protocol-method surface plus client-only
  commands, not a count of distinct typed commands.

Rather than pad the list to hit 256 or 266, this reference documents the 168
commands that are actually enumerable from the sources, and is honest about the
gap. The authoritative protocol-method inventory (every exposed index and its
wire arguments) lives in the protocol docs, not here.

---

## Player Commands

### Movement

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/run` | Switch to running gait | ❌ not server-handled | — | User | none | `/run` |
| `/walk` | Switch to walking gait | ❌ not server-handled | — | User | none | `/walk` |
| `/location` | Show your current position | ❌ not server-handled | — | User | none | `/location` |
| `/unstuck` | Attempt to free a stuck character | 🚧 partial (handler present) | — | User | none | `/unstuck` |
| `/exit` | Exit the game | ❌ client-side | — | User | none | `/exit` |
| `/respawn` | Respawn after death | ✅ implemented | ✅ | User | none | `/respawn` |

> `/respawn` drives the full health/focus reset, state-flag clear, and pawn
> re-anchor on the server. `/unstuck` is wired but its server effect is limited.

### Chat & Communication

Spatial channels (**say / emote / yell**) are broadcast to everyone in your Area
of Interest by the server. The remaining channels (team, squad, command/guild,
officer, private tells) are routed elsewhere and are **not** handled by the
in-world cell service.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/say` | Say something in local (spatial) chat | ✅ implemented | ✅ | User | `<text>` — the message | `/say hello there` |
| `/emote` | Perform an emote (spatial) | ✅ implemented | ✅ | User | `<text>` — emote text | `/emote waves` |
| `/yell` | Yell — wider spatial range than say | ✅ implemented | ✅ | User | `<text>` — the message | `/yell incoming!` |
| `/tell <player>` | Send a private message | ❌ not server-handled (cell) | — | User | `<player> <text>` | `/tell Jack on my way` |
| `/saysquad` | Talk in squad chat | ❌ not server-handled (cell) | — | User | `<text>` | `/saysquad regroup` |
| `/sayteam` | Talk in team chat | ❌ not server-handled (cell) | — | User | `<text>` | `/sayteam push left` |
| `/saycommand` | Talk in command/guild chat | ❌ not server-handled (cell) | — | User | `<text>` | `/saycommand event tonight` |
| `/sayofficer` | Talk in officer chat | ❌ not server-handled (cell) | — | User | `<text>` | `/sayofficer ranks updated` |
| `/chatjoin` | Join a chat channel | ❌ not server-handled (cell) | — | User | `<channel>` | `/chatjoin trade` |
| `/chatleave` | Leave a chat channel | ❌ not server-handled (cell) | — | User | `<channel>` | `/chatleave trade` |
| `/chatlist` | List available channels | ❌ not server-handled (cell) | — | User | none | `/chatlist` |
| `/chatsetafkmessage` | Set your AFK auto-reply | ❌ not server-handled (cell) | — | User | `<text>` | `/chatsetafkmessage afk, back soon` |
| `/chatsetdndmessage` | Set your Do-Not-Disturb auto-reply | ❌ not server-handled (cell) | — | User | `<text>` | `/chatsetdndmessage in a mission` |
| `/chatignore` | Ignore a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatignore Spammer` |
| `/chatmute` | Mute a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatmute Loud` |
| `/chatunmute` | Unmute a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatunmute Loud` |

### Squad (Small Group)

These map to the organization-invite protocol; the in-world cell service does not
implement the group state machine yet.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/squadinvite` | Invite a player to your squad | ❌ not server-handled | — | User | `<player>` | `/squadinvite Sam` |
| `/squadinviteaccept` | Accept a squad invite | ❌ not server-handled | — | User | none | `/squadinviteaccept` |
| `/squadinvitedecline` | Decline a squad invite | ❌ not server-handled | — | User | none | `/squadinvitedecline` |
| `/squadkick` | Kick a squad member | ❌ not server-handled | — | User | `<player>` | `/squadkick Sam` |
| `/squadleave` | Leave your squad | ❌ not server-handled | — | User | none | `/squadleave` |
| `/squadpromote` | Promote a member to leader | ❌ not server-handled | — | User | `<player>` | `/squadpromote Sam` |

### Team (Mid-Size Group)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/teaminvite` | Invite to team | ❌ not server-handled | — | User | `<player>` | `/teaminvite Sam` |
| `/teaminviteaccept` | Accept team invite | ❌ not server-handled | — | User | none | `/teaminviteaccept` |
| `/teaminvitedecline` | Decline team invite | ❌ not server-handled | — | User | none | `/teaminvitedecline` |
| `/teamkick` | Kick from team | ❌ not server-handled | — | User | `<player>` | `/teamkick Sam` |
| `/teamleave` | Leave team | ❌ not server-handled | — | User | none | `/teamleave` |
| `/teampromote` | Promote in team | ❌ not server-handled | — | User | `<player>` | `/teampromote Sam` |
| `/teamdemote` | Demote in team | ❌ not server-handled | — | User | `<player>` | `/teamdemote Sam` |
| `/teammotd` | Set team message of the day | ❌ not server-handled | — | User | `<text>` | `/teammotd raid at 8` |

### Command (Guild)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/commandinvite` | Invite to guild | ❌ not server-handled | — | User | `<player>` | `/commandinvite Sam` |
| `/commandinviteaccept` | Accept guild invite | ❌ not server-handled | — | User | none | `/commandinviteaccept` |
| `/commandinvitedecline` | Decline guild invite | ❌ not server-handled | — | User | none | `/commandinvitedecline` |
| `/commandkick` | Kick from guild | ❌ not server-handled | — | User | `<player>` | `/commandkick Sam` |
| `/commandleave` | Leave guild | ❌ not server-handled | — | User | none | `/commandleave` |
| `/commandpromote` | Promote in guild | ❌ not server-handled | — | User | `<player>` | `/commandpromote Sam` |
| `/commanddemote` | Demote in guild | ❌ not server-handled | — | User | `<player>` | `/commanddemote Sam` |
| `/commandmotd` | Set guild MOTD | ❌ not server-handled | — | User | `<text>` | `/commandmotd welcome!` |
| `/chooseorgname` | Choose organization name | 🚧 partial (route only) | — | User | `<name>` | `/chooseorgname SG-Alpha` |

### Combat & Abilities

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/invokeability` | Use an ability on a target | ✅ implemented | ✅ | User | `<abilityId> <targetId>` | `/invokeability 4201 5310` |
| `/activatebandolierslot` | Switch equipment loadout | ✅ implemented | — | User | `<bagId> <slotId>` | `/activatebandolierslot 2 1` |
| `/changeammo` | Switch ammo type | ✅ implemented | — | User | `<itemId> <ammoType>` | `/changeammo 8800 3` |
| `/trainability` | Learn a new ability (spends a point) | ✅ implemented | — | User | `<abilityId>` | `/trainability 4202` |
| `/resetabilities` | Reset all abilities | 🚧 partial | — | User | none | `/resetabilities` |
| `/respec` | Full ability respec | ❌ not server-handled | — | User | none | `/respec` |

> `/invokeability` is the typed form of the in-world ability cast; it runs the
> full damage/kill-credit pipeline. `/activatebandolierslot` and `/changeammo`
> reach the vendor/inventory layer. `/trainability` debits a training point on
> the server.

### Pets

The pet command protocol exists but the in-world handlers are stubs (they log and
acknowledge, but don't yet drive pet behavior).

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/petinvokeability` | Command pet to use an ability | 🚧 partial (stub) | — | User | `<petId> <abilityId> <targetId>` | `/petinvokeability 9001 4300 5310` |
| `/petinvokecommand` | Give pet a command | ❌ not server-handled | — | User | `<petId> <command>` | `/petinvokecommand 9001 attack` |
| `/petabilitytoggle` | Toggle pet auto-ability | 🚧 partial (stub) | — | User | `<petId> <abilityId> <toggle>` | `/petabilitytoggle 9001 4300 1` |
| `/petchangestance` | Change pet stance | 🚧 partial (stub) | — | User | `<petId> <stance>` | `/petchangestance 9001 2` |

### Items & Inventory

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/equipitem` | Equip an item | 🚧 partial | — | User | `<itemId>` | `/equipitem 8800` |
| `/unequipitem` | Unequip an item | 🚧 partial | — | User | `<itemId>` | `/unequipitem 8800` |
| `/useitem` | Use an item | ✅ implemented | — | User | `<itemId> <targetId>` | `/useitem 7700 0` |
| `/deleteitem` | Delete an item | ✅ implemented | — | User | `<itemId> <quantity>` | `/deleteitem 7700 1` |
| `/moveitem` | Move an item between containers | ✅ implemented | — | User | `<itemId> <targetBag> <targetSlot> <quantity>` | `/moveitem 7700 1 4 1` |
| `/purchaseitem` | Buy from a vendor | ✅ implemented | — | User | `<itemIndex...> <quantity...>` | `/purchaseitem 12 1` |
| `/repairitem` | Repair an item | ✅ implemented | — | User | `<itemId...>` | `/repairitem 8800` |
| `/rechargeitem` | Recharge an item | ✅ implemented | — | User | `<itemId...>` | `/rechargeitem 8800` |

> Buy/sell/repair/recharge run through the server's vendor flow with server-side
> validation. Equip/unequip are partially wired.

### Missions

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/missionassign` | Accept a mission | 🚧 partial | — | User | `<missionId>` | `/missionassign 1001` |
| `/missionabandon` | Abandon a mission | ✅ implemented | — | User | `<missionId>` | `/missionabandon 1001` |
| `/missiondetails` | View mission details | ❌ not server-handled | — | User | `<missionId>` | `/missiondetails 1001` |
| `/missionlist` | List your missions | ❌ not server-handled | — | User | none | `/missionlist` |
| `/sharemission` | Share a mission with your team | ✅ implemented | — | User | `<missionId>` | `/sharemission 1001` |
| `/sharemissionaccept` | Accept a shared mission | ✅ implemented | — | User | none | `/sharemissionaccept` |
| `/sharemissiondecline` | Decline a shared mission | ✅ implemented | — | User | none | `/sharemissiondecline` |

> Player mission accept/abandon/share reach the server's per-player mission
> state. The GM mission tools (below) are the more fully featured path.

### Stargate

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/dhd` | Dial a Dial-Home Device | ✅ implemented | — | User | `<targetAddressId> <sourceAddressId>` | `/dhd 14 0` |
| `/setringtransporterdestination` | Set ring transporter destination | ✅ implemented | — | User | `<regionId> <destinationId>` | `/setringtransporterdestination 3 7` |

### Dueling

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/duelchallenge` | Challenge a player to a duel | ❌ not server-handled | — | User | `<player>` | `/duelchallenge Rival` |
| `/duelresponse` | Accept or decline a duel challenge | ✅ implemented | — | User | `<response>` (accept/decline) | `/duelresponse 1` |
| `/duelforfeit` | Forfeit an active duel | ✅ implemented | — | User | none | `/duelforfeit` |

### Crafting

The crafting command protocol is routed but the actual crafting logic is not yet
implemented — these reach the server but currently log and acknowledge only.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/craft` | Create an item from a blueprint | 🚧 partial (route only) | — | User | `<craftId> <items...> <quantity>` | `/craft 500 8800 1` |
| `/alloy` | Combine crafting materials | 🚧 partial (route only) | — | User | `<craftId> <currentTierItemId> <lowerTierItems...>` | `/alloy 510 8801 8800` |
| `/research` | Research a new recipe | 🚧 partial (route only) | — | User | `<itemId> <kickers...>` | `/research 8800` |
| `/reverseengineer` | Deconstruct an item for knowledge | 🚧 partial (route only) | — | User | `<itemId>` | `/reverseengineer 8800` |
| `/respeccraft` | Respec crafting skills | 🚧 partial (route only) | — | User | none | `/respeccraft` |

### Other

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/who` | List online players | 🚧 partial (stub) | — | User | none | `/who` |
| `/users` | Show user count | ❌ not server-handled (player form) | — | User | none | `/users` |
| `/help` | Show command help | ❌ client-side | — | User | none | `/help` |
| `/helpfull` | Show all available commands | ❌ client-side | — | User | none | `/helpfull` |
| `/petition` | File a support petition | ❌ not server-handled | — | User | `<text>` | `/petition stuck in geometry` |
| `/logoff` | Log off | ❌ client-side | — | User | none | `/logoff` |

---

## GM Commands

All GM commands require a **Game Master** account (or higher). Authorization is
enforced server-side before any handler runs.

> [!NOTE]
> **Numeric ids only.** The in-world GM handlers accept **numeric** ids for
> design ids, mission ids, and entity targets. Name-to-id resolution (e.g.
> `/Goto SomePlayerName`) is **not** wired into the in-world handlers — pass the
> numeric entity id instead. A non-numeric argument is rejected with a feedback
> message.
>
> **Same-space only.** Teleport, kill, summon, despawn, and inspect commands act
> only on entities in the GM's own space instance. A cross-space target is
> refused.

### Teleportation & Travel

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/gotoxyz` | Teleport yourself to coordinates in your space | ✅ implemented | ✅ | GM | `<x> <y> <z>` (floats; must be finite) | `/gotoxyz 1200.5 64.0 -880.0` |
| `/gotolocation` | Teleport yourself to a named world + coordinates (full reload) | ✅ implemented | ✅ | GM | `<worldName> <x> <y> <z>` | `/gotolocation Abydos 100 5 200` |
| `/goto` | Teleport yourself to a target entity | ✅ implemented | ✅ | GM | `<entityId>` (numeric) | `/goto 5310` |
| `/summon` | Move a target entity to you | ✅ implemented | ✅ | GM | `<entityId>` (numeric; not yourself) | `/summon 5310` |
| `/dhd` (GM) | Dial a stargate by numeric address | ✅ implemented | ✅ | GM | `<gateAddress>` (positive int) | `/dhd 14` |

> `/gotoxyz` and `/goto` are same-space snaps. `/gotolocation` does a full
> cross-world reload. `/summon` snaps players via a forced-position update; NPCs
> move on the spatial grid and witnesses pick them up on the next refresh.

### Giving Things

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/givexp` | Give yourself experience | ✅ implemented | ✅ | GM | `<amount>` (positive int) | `/givexp 5000` |
| `/giveitem` | Give an item to your inventory | ✅ implemented | ✅ | GM | `<designId> <quantity>` (numeric id; qty clamped 1–1000) | `/giveitem 8800 5` |
| `/givenaqahdah` | Give yourself naquadah (currency) | ✅ implemented | ✅ | GM | `<amount>` (positive int) | `/givenaqahdah 10000` |
| `/giveexpertise` | Give yourself crafting expertise in a discipline | ✅ implemented | ✅ | GM | `<disciplineId> <amount>` (both positive) | `/giveexpertise 3 50` |
| `/giveappliedsciencepoints` | Give yourself applied-science points | ✅ implemented | ✅ | GM | `<points>` (positive int) | `/giveappliedsciencepoints 25` |
| `/removeitem` (GM) | Remove a quantity of an inventory item from yourself | ✅ implemented | ✅ | GM | `<itemId> <quantity>` (both positive) | `/removeitem 8800 1` |

> `/giveitem` and `/giveexpertise` and the rest persist through the same base-side
> sinks the normal game flows use, so they fire the proper client updates.

The following give-commands are recognized by the client but **not yet
implemented** server-side:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/giveability` | Give a specific ability | ❌ not server-handled | GM |
| `/giveallabilities` | Give every ability | ❌ not server-handled | GM |
| `/givegearset` | Give a full gear set | ❌ not server-handled | GM |
| `/givetrainingpoints` | Give training points | ❌ not server-handled | GM |
| `/givestargateaddress` | Give a stargate address | ❌ not server-handled | GM |
| `/giveblueprint` | Give a crafting blueprint | ❌ not server-handled | GM |
| `/giveammo` | Give ammunition | ❌ not server-handled | GM |
| `/giverespawner` | Give a player respawner | ❌ not server-handled | GM |
| `/giveracialparadigmlevels` | Give racial paradigm levels | ❌ not server-handled | GM |

### Character / Stat Modification

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/sethealth` | Set current health on yourself or a target | ✅ implemented | ✅ | GM | `<amount> <targetId>` (amount ≥ 0; target 0 = self) | `/sethealth 500 0` |
| `/sethealthmax` | Set maximum health | ✅ implemented | ✅ | GM | `<amount> <targetId>` | `/sethealthmax 1000 0` |
| `/setfocus` | Set current focus | ✅ implemented | ✅ | GM | `<amount> <targetId>` | `/setfocus 300 0` |
| `/setfocusmax` | Set maximum focus | ✅ implemented | ✅ | GM | `<amount> <targetId>` | `/setfocusmax 400 0` |
| `/settarget` | Set your current target | ✅ implemented | ✅ | GM | `<entityId>` (numeric; 0 = clear) | `/settarget 5310` |

The following character-modification commands are recognized by the client but
**not yet implemented** server-side:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/setlevel` | Set a character's level | ❌ not server-handled | GM |
| `/setspeed` | Set movement speed | ❌ not server-handled | GM |
| `/setgodmode` | Toggle invincibility | ❌ not server-handled | GM |
| `/setnodamage` | Toggle damage immunity | ❌ not server-handled | GM |
| `/setnoxp` | Toggle XP immunity | ❌ not server-handled | GM |
| `/setnoaggro` | Toggle NPC aggro | ❌ not server-handled | GM |
| `/sethidegm` | Toggle GM visibility | ❌ not server-handled | GM |
| `/setflag` | Force-set a state flag | ❌ not server-handled | GM |
| `/setmobstance` | Set an NPC's stance | ❌ not server-handled | GM |
| `/resetabilities` (GM) | Reset abilities | ❌ not server-handled | GM |
| `/giveallabilities` | Grant the full ability tree | ❌ not server-handled | GM |
| `/respec` (GM) | Full respec | ❌ not server-handled | GM |

### Entity Control (Spawn / World)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/spawn` | Spawn an NPC by template id at your position + offset | ✅ implemented | ✅ | GM | `<designId> <xOffset> <zOffset>` (numeric template id; float offsets) | `/spawn 3402 5.0 0.0` |
| `/despawn` | Remove an NPC from the space (NPC-only) | ✅ implemented | ✅ | GM | `<entityId>` (numeric) | `/despawn 5400` |
| `/kill` | Kill an NPC via the canonical death sequence (NPC-only) | ✅ implemented | ✅ | GM | `<entityId>` (numeric) | `/kill 5400` |
| `/respawn` (GM) | Respawn yourself | ✅ implemented | ✅ | GM | none | `/respawn` |

> `/spawn` does a base round-trip: the server looks up the template, builds the
> spawn record, and places the NPC. `/kill` and `/despawn` refuse player targets
> by design.

The following entity-control command is recognized but **not implemented**:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/interact` (GM force) | Force an NPC interaction | ❌ not server-handled | GM |

### Inspection / Query

These report text back to you through the GM feedback channel.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/showplayer` | Dump id / name / kind / faction / level / health / position for an entity (`.info`) | ✅ implemented | ✅ | GM | `<targetId>` (0 = current target, then self) | `/showplayer 5310` |
| `/showtargetlocation` | Report the target's (or your) position (`.location`) | ✅ implemented | ✅ | GM | none | `/showtargetlocation` |
| `/showrotation` | Report the target's (or your) heading (`.rotation`) | ✅ implemented | ✅ | GM | none | `/showrotation` |
| `/listabilities` | List the known ability ids of the target (or you) | ✅ implemented | ✅ | GM | none | `/listabilities` |
| `/showflag` | Report whether a state-flag bit is set | ✅ implemented | ✅ | GM | `<flagId>` (bit index 0–31) | `/showflag 4` |
| `/getmobattribute` | Report one attribute of an NPC | ✅ implemented | ✅ | GM | `<targetId> <attribute>` (health/focus/level/faction/alignment/aistate/name/template/pos) | `/getmobattribute 5400 health` |
| `/showmobcount` | Count NPCs in a space | ✅ implemented | ✅ | GM | `<spaceId>` (0 = your current space) | `/showmobcount 0` |
| `/users` (GM) / `/who` (GM) | List players in your space | ✅ implemented | ✅ | GM | none | `/users` |
| `/testlos` | Report navmesh line-of-sight between two entities | ✅ implemented | ✅ | GM | `<sourceId> <targetId>` (numeric, same space) | `/testlos 5310 5400` |

The following inspection commands are recognized but **not implemented**:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/showinventory` | Show a player's inventory | ❌ not server-handled | GM |
| `/showip` | Show a player's IP | ❌ not server-handled | GM |
| `/listinteractions` | List available interactions | ❌ not server-handled | GM |
| `/showpointset` | Show a cover/nav point set | ❌ not server-handled | GM |

### Mission (GM)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/missionassign` (GM) | Assign a mission to yourself by numeric id | ✅ implemented | ✅ | GM | `<missionId> <popup>` (popup is a UI hint) | `/missionassign 1001 1` |
| `/missionclear` | Abandon one mission by numeric id | ✅ implemented | ✅ | GM | `<missionId>` | `/missionclear 1001` |
| `/missionabandon` (GM) | Alias of `/missionclear` | ✅ implemented | ✅ | GM | `<missionId>` | `/missionabandon 1001` |
| `/missionadvance` | Jump a mission to a specific step | ✅ implemented | ✅ | GM | `<missionId> <step>` (step positive) | `/missionadvance 1001 3` |
| `/missionlist` (GM) | List your active missions | ✅ implemented | ✅ | GM | none | `/missionlist` |
| `/missionlistfull` | List all your missions (incl. completed/hidden) | ✅ implemented | ✅ | GM | none | `/missionlistfull` |
| `/missiondetails` (GM) | Show one mission's detail by numeric id | ✅ implemented | ✅ | GM | `<missionId>` | `/missiondetails 1001` |

The following mission GM commands are recognized but **not implemented**:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/missioncomplete` | Complete a mission | ❌ not server-handled | GM |
| `/missionreset` | Revert a mission to a step | ❌ not server-handled | GM |
| `/missionclearactive` | Clear all active missions | ❌ not server-handled | GM |
| `/missionclearhistory` | Clear mission history | ❌ not server-handled | GM |
| `/missionsetavailable` | Make a mission available | ❌ not server-handled | GM |

### Debug (GM)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/mobdata` | Dump an NPC's debug data (template, AI state, faction, health, threat) | ✅ implemented | ✅ | GM | `<spaceId> <targetId>` | `/mobdata 0 5400` |

Toggle-style debug commands map to the inherited combat/heal-debug methods. They
are GM-gated but currently log-only stubs:

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/combatdebug` | Toggle combat debug info | 🚧 partial (log-only stub) | GM |
| `/combatdebugverbose` | Toggle verbose combat debug | 🚧 partial (log-only stub) | GM |
| `/healdebug` | Toggle healing debug info | 🚧 partial (log-only stub) | GM |
| `/worldinstancereset` | Reset the world instance | 🚧 gated, no handler (destructive) | GM |

The remaining debug commands are recognized by the client but **not
implemented** server-side: `/abilitydebug`, `/getmobattribute` (set form
`/setmobattribute`), `/debugbehaviorsonmob`, `/debugpathsonmob`,
`/forceclientcrash`, `/timeofday`, `/xrayeyes`, `/physics`, `/invisible`,
`/togglecombatlos`.

### Visual / Client Debug

These render entirely in the client (overlays, FPS counters, visualizers). The
server holds no state for them, so they are **not server-handled**.

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/showfps` | Show frames per second | ❌ client-side | GM |
| `/showmemory` | Show memory usage | ❌ client-side | GM |
| `/showlog` | Show log output | ❌ client-side | GM |
| `/showcover` | Visualize cover links | ❌ client-side | GM |
| `/shownavmesh` | Show navigation mesh | ❌ not server-handled | GM |
| `/showspawns` | Show spawn points | ❌ not server-handled | GM |
| `/showmobpaths` | Show NPC patrol paths | ❌ not server-handled | GM |
| `/showwaypoints` | Show waypoints | ❌ client-side | GM |
| `/showtriggers` | Show trigger volumes | ❌ client-side | GM |
| `/showposition` | Show exact position | ❌ client-side | GM |
| `/showrotation` (client) | Show rotation overlay | ❌ client-side | GM |

### Hot-Loading / Content Reload (GM)

These ask the server to reload a data definition without restarting. None are
implemented as targeted reloads today.

| Command | What It Does | Handled? | Access |
|---------|--------------|----------|--------|
| `/loadability` | Reload an ability definition | ❌ not server-handled | GM |
| `/loaditem` | Reload an item definition | ❌ not server-handled | GM |
| `/loadmission` | Reload a mission | ❌ not server-handled | GM |
| `/loadmob` | Reload an NPC definition | ❌ not server-handled | GM |
| `/loadbehavior` | Reload an AI behavior | ❌ not server-handled | GM |
| `/loadconstants` | Reload game constants | ❌ not server-handled | GM |
| `/loaddialogset` | Reload a dialog set | ❌ not server-handled | GM |
| `/reload` | General data reload | ❌ not server-handled | GM |
| `/reloadorganizations` | Reload organization defs | ❌ not server-handled | GM |
| `/reloadinventory` | Reload inventory defs | ❌ not server-handled | GM |

---

## Summary

| Group | Rows documented | Fully implemented (✅) | Partial (🚧) | Not server-handled (❌) | Tested (✅) |
|-------|-----------------|------------------------|--------------|--------------------------|-------------|
| Player | 86 | 22 | ~12 | ~52 | 4 |
| GM / Debug | 93 | 37 | ~7 | ~49 | 38 |
| **Total** | **179** | **59** | **19** | **101** | **42** |

- **59 rows** have a complete server-side handler (✅ Handled). These cover the
  168 distinct commands once player/GM duplicate forms are collapsed.
- **42 rows** have an explicit automated test (✅ Tested) — all 38 implemented GM
  commands plus the 4 spatial-chat / ability player commands (`/say`, `/emote`,
  `/yell`, `/invokeability`).
- **19 rows** are partial (🚧): route-only stubs that acknowledge the call but
  don't yet drive the full game effect.
- **101 rows** are ❌ — client-side only, or recognized by the client but not yet
  implemented server-side. They are documented for completeness, not as evidence
  of a server feature.

> Counts above reflect the evidence in this server's handler code and tests as of
> the `last_updated` date. The client-side round-trip for every command is still
> pending a manual UAT.

## See also

- [Chat system](gameplay/chat-system.md) — channels, whispers, emotes.
- [Pet system](gameplay/pet-system.md) — pet commands and abilities.
- [Service architecture](architecture/service-architecture.md) — how console
  commands reach the cell/base services.
</content>

</invoke>
