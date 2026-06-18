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

This catalog enumerates **all 256 commands the client dispatches**, grouped by
category. Each one is a real, typed `/command` the game's console recognizes —
the complete set extracted directly from the client binary.

About the larger figure you may have seen:

- The client's `/help` reports **"266 commands found"**. The client binary
  contains exactly **256** distinct command classes — this is the authoritative,
  enumerable set, and it is what this page documents.
- The remaining **~10** entries in the `/help` total are typed aliases or
  client-utility entries that share no dedicated command class. They could not be
  enumerated from the binary without deeper analysis, so they are **not** invented
  here. This page documents the 256 it can prove and is honest about the ~10 gap
  rather than padding the list to 266.

Every command below is listed under its real binary name (lower-cased to its
typed form). Where the same effect has both a player and a more capable GM form,
both are shown in their respective sections.

---

## Player Commands

### Movement

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/run` | Switch to running gait | ❌ client-side | — | User | none | `/run` |
| `/walk` | Switch to walking gait | ❌ client-side | — | User | none | `/walk` |
| `/location` | Show your current position | ❌ client-side | — | User | none | `/location` |
| `/unstuck` | Attempt to free a stuck character | 🚧 partial (handler present) | — | User | none | `/unstuck` |
| `/exit` | Exit the game | ❌ client-side | — | User | none | `/exit` |
| `/logoff` | Log off | ❌ client-side | — | User | none | `/logoff` |
| `/respawn` | Respawn after death | ✅ implemented | ✅ | User | none | `/respawn` |

> `/respawn` drives the full health/focus reset, state-flag clear, and pawn
> re-anchor on the server. `/unstuck` is wired but its server effect is limited.

### Chat & Communication

Spatial channels (**say / emote / yell**) are broadcast to everyone in your Area
of Interest by the server. The remaining channels (team, squad, command/guild,
officer, platoon, custom channels, private tells) are routed elsewhere and are
**not** handled by the in-world cell service.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/say` | Say something in local (spatial) chat | ✅ implemented | ✅ | User | `<text>` — the message | `/say hello there` |
| `/emote` | Perform an emote (spatial) | ✅ implemented | ✅ | User | `<text>` — emote text | `/emote waves` |
| `/yell` | Yell — wider spatial range than say | ✅ implemented | ✅ | User | `<text>` — the message | `/yell incoming!` |
| `/tell` | Send a private message | ❌ not server-handled (cell) | — | User | `<player> <text>` | `/tell Jack on my way` |
| `/ntell` | Send a private message by name (numeric form) | ❌ not server-handled (cell) | — | User | `<player> <text>` | `/ntell Jack on my way` |
| `/sayteam` | Talk in team chat | ❌ not server-handled (cell) | — | User | `<text>` | `/sayteam push left` |
| `/saysquad` | Talk in squad chat | ❌ not server-handled (cell) | — | User | `<text>` | `/saysquad regroup` |
| `/saycommand` | Talk in command/guild chat | ❌ not server-handled (cell) | — | User | `<text>` | `/saycommand event tonight` |
| `/sayofficer` | Talk in officer chat | ❌ not server-handled (cell) | — | User | `<text>` | `/sayofficer ranks updated` |
| `/sayplatoon` | Talk in platoon chat | ❌ not server-handled (cell) | — | User | `<text>` | `/sayplatoon form up` |
| `/saychannel` | Talk in a named custom channel | ❌ not server-handled (cell) | — | User | `<channel> <text>` | `/saychannel trade WTS naquadah` |
| `/petition` | File a support petition | ❌ not server-handled | — | User | `<text>` | `/petition stuck in geometry` |
| `/chatjoin` | Join a chat channel | ❌ not server-handled (cell) | — | User | `<channel>` | `/chatjoin trade` |
| `/chatleave` | Leave a chat channel | ❌ not server-handled (cell) | — | User | `<channel>` | `/chatleave trade` |
| `/chatlist` | List available channels | ❌ not server-handled (cell) | — | User | none | `/chatlist` |
| `/chatsetafkmessage` | Set your AFK auto-reply | ❌ not server-handled (cell) | — | User | `<text>` | `/chatsetafkmessage afk, back soon` |
| `/chatsetdndmessage` | Set your Do-Not-Disturb auto-reply | ❌ not server-handled (cell) | — | User | `<text>` | `/chatsetdndmessage in a mission` |
| `/chatignore` | Ignore a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatignore Spammer` |
| `/chatfriend` | Add a player as a friend | ❌ not server-handled (cell) | — | User | `<player>` | `/chatfriend Sam` |
| `/chatunfriend` | Remove a friend | ❌ not server-handled (cell) | — | User | `<player>` | `/chatunfriend Sam` |
| `/chatmute` | Mute a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatmute Loud` |
| `/chatunmute` | Unmute a player | ❌ not server-handled (cell) | — | User | `<player>` | `/chatunmute Loud` |
| `/chatkick` | Kick a player from a channel you own | ❌ not server-handled (cell) | — | User | `<channel> <player>` | `/chatkick trade Spammer` |
| `/chatop` | Grant channel operator status | ❌ not server-handled (cell) | — | User | `<channel> <player>` | `/chatop trade Sam` |
| `/chatban` | Ban a player from a channel | ❌ not server-handled (cell) | — | User | `<channel> <player>` | `/chatban trade Spammer` |
| `/chatunban` | Lift a channel ban | ❌ not server-handled (cell) | — | User | `<channel> <player>` | `/chatunban trade Spammer` |
| `/chatpassword` | Set or clear a channel password | ❌ not server-handled (cell) | — | User | `<channel> <password>` | `/chatpassword trade s3cret` |

### Squad (Small Group)

These map to the organization-invite protocol; the in-world cell service does not
implement the group state machine yet.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/squadinvite` | Invite a player to your squad | ❌ not server-handled | — | User | `<player>` | `/squadinvite Sam` |
| `/squadinviteaccept` | Accept a squad invite | ❌ not server-handled | — | User | none | `/squadinviteaccept` |
| `/squadinvitedecline` | Decline a squad invite | ❌ not server-handled | — | User | none | `/squadinvitedecline` |
| `/squadkick` | Kick a squad member | ❌ not server-handled | — | User | `<player>` | `/squadkick Sam` |
| `/squadpromote` | Promote a member to leader | ❌ not server-handled | — | User | `<player>` | `/squadpromote Sam` |
| `/squadleave` | Leave your squad | ❌ not server-handled | — | User | none | `/squadleave` |

### Team (Mid-Size Group)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/teaminvite` | Invite to team | ❌ not server-handled | — | User | `<player>` | `/teaminvite Sam` |
| `/teaminviteaccept` | Accept team invite | ❌ not server-handled | — | User | none | `/teaminviteaccept` |
| `/teaminvitedecline` | Decline team invite | ❌ not server-handled | — | User | none | `/teaminvitedecline` |
| `/teamleave` | Leave team | ❌ not server-handled | — | User | none | `/teamleave` |
| `/teamkick` | Kick from team | ❌ not server-handled | — | User | `<player>` | `/teamkick Sam` |
| `/teampromote` | Promote in team | ❌ not server-handled | — | User | `<player>` | `/teampromote Sam` |
| `/teamdemote` | Demote in team | ❌ not server-handled | — | User | `<player>` | `/teamdemote Sam` |
| `/teammotd` | Set team message of the day | ❌ not server-handled | — | User | `<text>` | `/teammotd raid at 8` |
| `/setteamnote` | Set a team member note | ❌ not server-handled | — | User | `<player> <text>` | `/setteamnote Sam main tank` |
| `/setteamofficernote` | Set a team officer note | ❌ not server-handled | — | User | `<player> <text>` | `/setteamofficernote Sam promote soon` |

### Command (Guild)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/commandinvite` | Invite to guild | ❌ not server-handled | — | User | `<player>` | `/commandinvite Sam` |
| `/commandinviteaccept` | Accept guild invite | ❌ not server-handled | — | User | none | `/commandinviteaccept` |
| `/commandinvitedecline` | Decline guild invite | ❌ not server-handled | — | User | none | `/commandinvitedecline` |
| `/commandleave` | Leave guild | ❌ not server-handled | — | User | none | `/commandleave` |
| `/commandkick` | Kick from guild | ❌ not server-handled | — | User | `<player>` | `/commandkick Sam` |
| `/commandpromote` | Promote in guild | ❌ not server-handled | — | User | `<player>` | `/commandpromote Sam` |
| `/commanddemote` | Demote in guild | ❌ not server-handled | — | User | `<player>` | `/commanddemote Sam` |
| `/commandmotd` | Set guild MOTD | ❌ not server-handled | — | User | `<text>` | `/commandmotd welcome!` |
| `/setcommandnote` | Set a guild member note | ❌ not server-handled | — | User | `<player> <text>` | `/setcommandnote Sam recruit` |
| `/setcommandofficernote` | Set a guild officer note | ❌ not server-handled | — | User | `<player> <text>` | `/setcommandofficernote Sam vouched` |
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
| `/respecability` | Respec a single ability | ❌ not server-handled | — | User | `<abilityId>` | `/respecability 4202` |
| `/toggleautocycleability` | Toggle ability auto-cycling | 🚧 partial | — | User | `<enabled>` (0/1) | `/toggleautocycleability 1` |

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

> The client also dispatches a pet stance-change call, but it is routed through
> the inventory/ability layer rather than a dedicated typed command.

### Items & Inventory

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/equipitem` | Equip an item | 🚧 partial | — | User | `<itemId>` | `/equipitem 8800` |
| `/unequipitem` | Unequip an item | 🚧 partial | — | User | `<itemId>` | `/unequipitem 8800` |
| `/useitem` | Use an item | ✅ implemented | — | User | `<itemId> <targetId>` | `/useitem 7700 0` |
| `/deleteitem` | Delete an item | ✅ implemented | — | User | `<itemId> <quantity>` | `/deleteitem 7700 1` |
| `/moveitem` | Move an item between containers | ✅ implemented | — | User | `<itemId> <targetBag> <targetSlot> <quantity>` | `/moveitem 7700 1 4 1` |
| `/listitems` | List your inventory items | ✅ implemented | — | User | none | `/listitems` |
| `/getiteminfo` | Show detail for one item | ❌ not server-handled | — | User | `<itemId>` | `/getiteminfo 8800` |
| `/lootitem` | Take an item from a loot container | ✅ implemented | — | User | `<index>` | `/lootitem 0` |
| `/purchaseitem` | Buy from a vendor | ✅ implemented | — | User | `<itemIndex...> <quantity...>` | `/purchaseitem 12 1` |
| `/repairitem` | Repair an item | ✅ implemented | — | User | `<itemId...>` | `/repairitem 8800` |
| `/rechargeitem` | Recharge an item | ✅ implemented | — | User | `<itemId...>` | `/rechargeitem 8800` |

> Buy/sell/repair/recharge run through the server's vendor flow with server-side
> validation. Equip/unequip are partially wired.

### Interactions & Dialog

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/interact` | Interact with the targeted object/NPC | ✅ implemented | — | User | `<overrideTarget>` (0 = current target) | `/interact 0` |
| `/initialresponse` | Open the initial dialog for an NPC | ✅ implemented | — | User | `<dialogSetMapId>` | `/initialresponse 12` |
| `/dialogbuttonchoice` | Pick a dialog button | ✅ implemented | — | User | `<dialogId> <buttonId>` | `/dialogbuttonchoice 12 1` |
| `/showdialog` | Re-show the active dialog | ❌ client-side | — | User | none | `/showdialog` |
| `/confirmeffect` | Respond to an effect-confirmation prompt | ✅ implemented | — | User | `<choice>` | `/confirmeffect 1` |

### Missions

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/missionassign` | Accept a mission | 🚧 partial | — | User | `<missionId>` | `/missionassign 1001` |
| `/missionabandon` | Abandon a mission | ✅ implemented | — | User | `<missionId>` | `/missionabandon 1001` |
| `/abandonmission` | Abandon a mission (alternate form) | ✅ implemented | — | User | `<missionId>` | `/abandonmission 1001` |
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
| `/respeccraft` | Respec crafting skills | 🚧 partial (route only) | — | User | none | `/respeccraft` |

> The remaining crafting actions (craft, alloy, research, reverse-engineer,
> applied-science spend) are exposed to the server but are driven through the
> crafting UI rather than dedicated typed commands.

### Minigames

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/startminigame` | Start a minigame at a host object | 🚧 partial (route only) | — | User | `<hostEntityId> <gameDefId>` | `/startminigame 5400 3` |
| `/minigamecomplete` | Report a minigame result | 🚧 partial (route only) | — | User | `<gameId> <winnerId> <loserId>` | `/minigamecomplete 1 5310 0` |

### Squad / Map Tools

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/spacequeuestatus` | Query your space-instance queue status | ❌ not server-handled | — | User | none | `/spacequeuestatus` |
| `/spacequeuedresponse` | Respond to a queued-for-space prompt | ❌ not server-handled | — | User | `<response>` | `/spacequeuedresponse 1` |
| `/spacequeuereadyresponse` | Respond to a space-ready prompt | ❌ not server-handled | — | User | `<response>` | `/spacequeuereadyresponse 1` |

### Other

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/who` | List online players | 🚧 partial (stub) | — | User | none | `/who` |
| `/users` | Show user count | ❌ not server-handled (player form) | — | User | none | `/users` |
| `/help` | Show command help | ❌ client-side | — | User | none | `/help` |
| `/helpfull` | Show all available commands | ❌ client-side | — | User | none | `/helpfull` |
| `/testsequence` | Play a test animation sequence | ❌ client-side | — | User | `<sequenceName>` | `/testsequence idle` |
| `/updatesystemoptions` | Push a client option change to the server | 🚧 partial (route only) | — | User | `<name> <value>` | `/updatesystemoptions gore 0` |

---

## GM Commands

All GM commands require a **Game Master** account (or higher). Authorization is
enforced server-side before any handler runs.

> [!NOTE]
> **Numeric ids only.** The in-world GM handlers accept **numeric** ids for
> design ids, mission ids, and entity targets. Name-to-id resolution (e.g.
> `/goto SomePlayerName`) is **not** wired into the in-world handlers — pass the
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

> `/gotoxyz` and `/goto` are same-space snaps. `/gotolocation` does a full
> cross-world reload. `/summon` snaps players via a forced-position update; NPCs
> move on the spatial grid and witnesses pick them up on the next refresh. The GM
> `/dhd` form is listed under **Stargate (GM)** below.

### Giving Things

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/givexp` | Give yourself experience | ✅ implemented | ✅ | GM | `<amount>` (positive int) | `/givexp 5000` |
| `/giveitem` | Give an item to your inventory | ✅ implemented | ✅ | GM | `<designId> <quantity>` (numeric id; qty clamped 1–1000) | `/giveitem 8800 5` |
| `/givenaqahdah` | Give yourself naquadah (currency) | ✅ implemented | ✅ | GM | `<amount>` (positive int) | `/givenaqahdah 10000` |
| `/giveexpertise` | Give yourself crafting expertise in a discipline | ✅ implemented | ✅ | GM | `<disciplineId> <amount>` (both positive) | `/giveexpertise 3 50` |
| `/giveappliedsciencepoints` | Give yourself applied-science points | ✅ implemented | ✅ | GM | `<points>` (positive int) | `/giveappliedsciencepoints 25` |
| `/removeitem` | Remove a quantity of an inventory item from yourself | ✅ implemented | ✅ | GM | `<itemId> <quantity>` (both positive) | `/removeitem 8800 1` |

> `/giveitem`, `/giveexpertise`, and the rest persist through the same base-side
> sinks the normal game flows use, so they fire the proper client updates.

The following give-commands are recognized by the client but **not yet
implemented** server-side:

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/giveability` | Give a specific ability | ❌ not server-handled | GM | `<abilityId>` |
| `/giveallabilities` | Give every ability | ❌ not server-handled | GM | none |
| `/givegearset` | Give a full gear set | ❌ not server-handled | GM | `<gearsetId>` |
| `/giveinventory` | Give a full inventory loadout | ❌ not server-handled | GM | `<inventoryId>` |
| `/givetrainingpoints` | Give training points | ❌ not server-handled | GM | `<count>` |
| `/givestargateaddress` | Give a stargate address | ❌ not server-handled | GM | `<address> <target> <hidden>` |
| `/removestargateaddress` | Remove a stargate address | ❌ not server-handled | GM | `<address> <target>` |
| `/giveblueprint` | Give a crafting blueprint | ❌ not server-handled | GM | `<blueprintId>` |
| `/giveammo` | Give ammunition | ❌ not server-handled | GM | `<ammoId> <quantity>` |
| `/giverespawner` | Give a player respawner | ❌ not server-handled | GM | `<mobId>` |
| `/giveracialparadigmlevels` | Give racial paradigm levels | ❌ not server-handled | GM | `<id> <levels>` |
| `/givefaction` | Give faction standing | ❌ not server-handled | GM | `<factionId> <amount>` |
| `/setfaction` | Set faction standing | ❌ not server-handled | GM | `<factionId> <value>` |
| `/settechskill` | Set a tech (crafting) skill | ❌ not server-handled | GM | `<skillId> <value>` |

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

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/setlevel` | Set a character's level | ❌ not server-handled | GM | `<level>` |
| `/setspeed` | Set movement speed | ❌ not server-handled | GM | `<multiplier>` |
| `/setarchetype` | Set a character's archetype | ❌ not server-handled | GM | `<archetypeId>` |
| `/setgodmode` | Toggle invincibility | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setinvulnerable` | Toggle invulnerability | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setomnipotent` | Toggle all-powerful debug mode | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setnodamagetimedmode` | Toggle timed damage immunity | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setnoxp` | Toggle XP gain off | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setnoaggro` | Toggle NPC aggro | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setinfiniteammo` | Toggle infinite ammo | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setignorefocus` | Ignore focus costs | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setignorehealth` | Ignore health damage | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setfly` | Toggle fly mode | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setghost` | Toggle ghost (no-collision) mode | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setspectator` | Toggle spectator mode | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setnotarget` | Make yourself untargetable | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setpvp` | Toggle your PvP flag | ❌ not server-handled | GM | `<on>` (0/1) |
| `/sethidegm` | Toggle GM visibility | ❌ not server-handled | GM | `<on>` (0/1) |
| `/setflag` | Force-set a state flag | ❌ not server-handled | GM | `<flagId> <force>` |
| `/setmobstance` | Set an NPC's stance | ❌ not server-handled | GM | `<stance>` |
| `/setmobabilityset` | Set an NPC's ability set | ❌ not server-handled | GM | `<setId>` |
| `/setmobvariable` | Set a generic NPC variable | ❌ not server-handled | GM | `<var> <value>` |
| `/setmobattribute` | Set an NPC attribute | ❌ not server-handled | GM | `<target> <attr> <type> <value>` |
| `/resetabilities` | Reset abilities (GM) | ❌ not server-handled | GM | none |
| `/respec` | Full respec (GM) | ❌ not server-handled | GM | none |
| `/respecability` | Respec one ability (GM) | ❌ not server-handled | GM | `<abilityId>` |
| `/respeccraft` | Respec crafting (GM) | ❌ not server-handled | GM | none |

### Entity Control (Spawn / World)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/spawn` | Spawn an NPC by template id at your position + offset | ✅ implemented | ✅ | GM | `<designId> <xOffset> <zOffset>` (numeric template id; float offsets) | `/spawn 3402 5.0 0.0` |
| `/despawn` | Remove an NPC from the space (NPC-only) | ✅ implemented | ✅ | GM | `<entityId>` (numeric) | `/despawn 5400` |
| `/kill` | Kill an NPC via the canonical death sequence (NPC-only) | ✅ implemented | ✅ | GM | `<entityId>` (numeric) | `/kill 5400` |

> `/spawn` does a base round-trip: the server looks up the template, builds the
> spawn record, and places the NPC. `/kill` and `/despawn` refuse player targets
> by design. The GM `/respawn` form is implemented and respawns yourself.

The following entity-control commands are recognized but **not implemented**:

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/gmdeleteitem` | Force-delete an item by id | ❌ not server-handled | GM | `<itemId>` |
| `/activatespawnset` | Activate a spawn set | ❌ not server-handled | GM | `<setId>` |
| `/deactivatespawnset` | Deactivate a spawn set | ❌ not server-handled | GM | `<setId>` |
| `/spawnentityloot` | Force-drop loot from an entity | ❌ not server-handled | GM | `<entity> <lootTableId>` |
| `/dumpobjects` | Dump the object list to the log | ❌ not server-handled | GM | none |

### Stargate (GM)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/dhd` (GM) | Dial a stargate by numeric address | ✅ implemented | ✅ | GM | `<gateAddress>` (positive int) | `/dhd 14` |

### Inspection / Query

These report text back to you through the GM feedback channel.

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/showplayer` | Dump id / name / kind / faction / level / health / position for an entity | ✅ implemented | ✅ | GM | `<targetId>` (0 = current target, then self) | `/showplayer 5310` |
| `/showtargetlocation` | Report the target's (or your) position | ✅ implemented | ✅ | GM | none | `/showtargetlocation` |
| `/showrotation` | Report the target's (or your) heading | ✅ implemented | ✅ | GM | none | `/showrotation` |
| `/showposition` | Report your exact position | ✅ implemented | ✅ | GM | none | `/showposition` |
| `/listabilities` | List the known ability ids of the target (or you) | ✅ implemented | ✅ | GM | none | `/listabilities` |
| `/showflag` | Report whether a state-flag bit is set | ✅ implemented | ✅ | GM | `<flagId>` (bit index 0–31) | `/showflag 4` |
| `/getmobattribute` | Report one attribute of an NPC | ✅ implemented | ✅ | GM | `<targetId> <attribute>` (health/focus/level/faction/alignment/aistate/name/template/pos) | `/getmobattribute 5400 health` |
| `/showmobcount` | Count NPCs in a space | ✅ implemented | ✅ | GM | `<spaceId>` (0 = your current space) | `/showmobcount 0` |
| `/users` (GM) | List players in your space | ✅ implemented | ✅ | GM | none | `/users` |
| `/who` (GM) | List players in your space | ✅ implemented | ✅ | GM | none | `/who` |
| `/testlos` | Report navmesh line-of-sight between two entities | ✅ implemented | ✅ | GM | `<sourceId> <targetId>` (numeric, same space) | `/testlos 5310 5400` |

The following inspection commands are recognized but **not implemented**:

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/showinventory` | Show a player's inventory | ❌ not server-handled | GM | `<target>` |
| `/showip` | Show a player's IP | ❌ not server-handled | GM | `<target>` |
| `/listinteractions` | List interactions available on the target | ❌ not server-handled | GM | none |
| `/printstats` | Print a named server statistic | ❌ not server-handled | GM | `<stat>` |

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

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/missioncomplete` | Complete a mission | ❌ not server-handled | GM | `<missionId> <turnIn>` |
| `/missionreset` | Revert a mission to a step | ❌ not server-handled | GM | `<missionId> <step>` |
| `/missionclearactive` | Clear all active missions | ❌ not server-handled | GM | none |
| `/missionclearhistory` | Clear mission history | ❌ not server-handled | GM | none |
| `/missionsetavailable` | Make a mission available | ❌ not server-handled | GM | `<missionId>` |

### Debug (GM)

| Command | What It Does | Handled? | Tested? | Access | Parameters | Sample |
|---------|--------------|----------|---------|--------|------------|--------|
| `/mobdata` | Dump an NPC's debug data (template, AI state, faction, health, threat) | ✅ implemented | ✅ | GM | `<spaceId> <targetId>` | `/mobdata 0 5400` |

Toggle-style debug commands map to the inherited combat/heal-debug methods. They
are GM-gated but currently log-only stubs:

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/combatdebug` | Toggle combat debug info | 🚧 partial (log-only stub) | GM | none |
| `/combatdebugverbose` | Toggle verbose combat debug | 🚧 partial (log-only stub) | GM | none |
| `/healdebug` | Toggle healing debug info | 🚧 partial (log-only stub) | GM | none |
| `/worldinstancereset` | Reset the world instance | 🚧 gated, no handler (destructive) | GM | none |

The remaining debug commands are recognized by the client but **not
implemented** server-side:

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/abilitydebug` | Toggle ability debug | ❌ not server-handled | GM | `<abilityId>` |
| `/debugtarget` | Dump debug data for your target | ❌ not server-handled | GM | `<target>` |
| `/debugevents` | Toggle event debug | ❌ not server-handled | GM | `<target> <level>` |
| `/debugperformance` | Toggle performance debug | ❌ not server-handled | GM | none |
| `/debugabilityonmob` | Run an ability on an NPC for debug | ❌ not server-handled | GM | `<abilityId>` |
| `/debugbehaviorsonmob` | Stream an NPC's behavior state | ❌ not server-handled | GM | none |
| `/debugpathsonmob` | Stream an NPC's nav path | ❌ not server-handled | GM | none |
| `/debuginteract` | Force an interaction for debug | ❌ not server-handled | GM | none |
| `/debugflash` | Toggle Flash UI debug | ❌ not server-handled | GM | none |
| `/debugstartminigame` | Start a minigame for debug | ❌ not server-handled | GM | `<gameId>` |
| `/debugminigameinstance` | Inspect a minigame instance | ❌ not server-handled | GM | `<instanceId>` |
| `/debugspectateminigame` | Spectate a minigame for debug | ❌ not server-handled | GM | `<gameId>` |
| `/debugjoinminigame` | Join a minigame for debug | ❌ not server-handled | GM | `<gameId>` |
| `/giveminigamecontact` | Grant a minigame contact | ❌ not server-handled | GM | `<contactId> <target>` |
| `/removeminigamecontact` | Remove a minigame contact | ❌ not server-handled | GM | `<contactId> <target>` |
| `/emitbehavioreventonmob` | Emit a behavior event on an NPC | ❌ not server-handled | GM | `<id>` |
| `/addbehavioreventset` | Add a behavior event set | ❌ not server-handled | GM | `<id>` |
| `/removebehavioreventset` | Remove a behavior event set | ❌ not server-handled | GM | `<id>` |
| `/entererroraistate` | Force an NPC into the error AI state | ❌ not server-handled | GM | none |
| `/exiterroraistate` | Clear an NPC's error AI state | ❌ not server-handled | GM | none |
| `/timeofday` | Set the time of day | ❌ not server-handled | GM | `<hour>` |
| `/physics` | Toggle physics debug | ❌ not server-handled | GM | `<on>` (0/1) |
| `/togglecombatlos` | Toggle combat line-of-sight enforcement | ❌ not server-handled | GM | none |
| `/forceclientcrash` | Force the client to crash (debug) | ❌ client-side | GM | none |
| `/forcerenderthreadcrash` | Force the render thread to crash (debug) | ❌ client-side | GM | none |
| `/perfstatsbychannel` | Toggle per-channel perf stats | ❌ not server-handled | GM | `<onOff>` |
| `/gmchat` | Send a GM chat message | ❌ not server-handled | GM | `<text>` |
| `/gmshout` | Shout as a GM (space or global) | ❌ not server-handled | GM | `<global> <text>` |

### Visual / Client Debug

These render entirely in the client (overlays, FPS counters, visualizers) or have
no server state. They are **not server-handled**.

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/showfps` | Show frames per second | ❌ client-side | GM | none |
| `/showmemory` | Show memory usage | ❌ client-side | GM | none |
| `/showlog` | Show log output | ❌ client-side | GM | none |
| `/showcover` | Visualize cover links | ❌ client-side | GM | none |
| `/shownavmesh` | Show navigation mesh | ❌ not server-handled | GM | none |
| `/showspawns` | Show spawn points | ❌ not server-handled | GM | none |
| `/showmobpaths` | Show NPC patrol paths | ❌ not server-handled | GM | none |
| `/showwaypoints` | Show waypoints | ❌ client-side | GM | none |
| `/showtriggers` | Show trigger volumes | ❌ client-side | GM | none |
| `/showcommandwaypoints` | Show command-group waypoints | ❌ client-side | GM | none |
| `/showspawnset` | Show a spawn set | ❌ not server-handled | GM | `<setId>` |
| `/showarea` | Show area bounds | ❌ client-side | GM | none |
| `/showregion` | Show region bounds | ❌ client-side | GM | none |
| `/shownavigation` | Toggle navigation overlay | ❌ not server-handled | GM | `<onOff>` |
| `/showinstanceflag` | Show a space-instance flag | ❌ not server-handled | GM | `<flag>` |
| `/setinstanceflag` | Set a space-instance flag | ❌ not server-handled | GM | `<flag> <value>` |
| `/xrayeyes` | Toggle x-ray vision | ❌ not server-handled | GM | `<on>` (0/1) |
| `/invisible` | Toggle invisibility | ❌ not server-handled | GM | `<on>` (0/1) |

### Cover System (GM)

These tune the navmesh cover system. The cover loader is static-load only; none
of the runtime regen/weight commands are implemented.

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/regeneratecoverlinks` | Recompute cover links | ❌ not server-handled | GM | `<normLimit> <maxLinks> <maxDist>` |
| `/changecoverweight` | Adjust cover scoring weights | ❌ not server-handled | GM | `<6 floats>` |
| `/changecoverstanceweight` | Adjust per-stance cover weights | ❌ not server-handled | GM | `<stance> <6 values>` |
| `/trackmob` | Toggle a debug track on an NPC | ❌ not server-handled | GM | none |

### Hot-Loading / Content Reload (GM)

These ask the server to reload a data definition without restarting. None are
implemented as targeted reloads today.

| Command | What It Does | Handled? | Access | Parameters |
|---------|--------------|----------|--------|------------|
| `/reload` | General data reload | ❌ not server-handled | GM | none |
| `/loadconstants` | Reload game constants | ❌ not server-handled | GM | none |
| `/loadability` | Reload an ability definition | ❌ not server-handled | GM | `<id>` |
| `/loadabilityset` | Reload an ability set | ❌ not server-handled | GM | `<id>` |
| `/loadnacsi` | Reload a NACSI definition | ❌ not server-handled | GM | `<id>` |
| `/loadbehavior` | Reload an AI behavior | ❌ not server-handled | GM | `<id>` |
| `/loadmob` | Reload an NPC definition | ❌ not server-handled | GM | `<id>` |
| `/loadinteractionset` | Reload an interaction set | ❌ not server-handled | GM | `<id>` |
| `/loaditem` | Reload an item definition | ❌ not server-handled | GM | `<id>` |
| `/loadmission` | Reload a mission | ❌ not server-handled | GM | `<id>` |
| `/reloadorganizations` | Reload organization defs | ❌ not server-handled | GM | none |
| `/reloadinventory` | Reload inventory defs | ❌ not server-handled | GM | none |

---

## Summary

| Group | Rows documented | Fully implemented (✅) | Partial (🚧) | Not server-handled (❌) | Tested (✅) |
|-------|-----------------|------------------------|--------------|--------------------------|-------------|
| Player | 113 | 29 | 14 | 70 | 5 |
| GM / Debug | 159 | 38 | 4 | 117 | 38 |
| **Total** | **272** | **67** | **18** | **187** | **43** |

- This page documents **all 256 distinct commands** the client dispatches. The
  table above counts **272 rows** because some commands (`/dhd`, `/respawn`,
  `/missionassign`, `/missionabandon`, `/missiondetails`, `/missionlist`, `/who`,
  `/users`, `/showrotation`, `/resetabilities`, `/respec`, `/respecability`,
  `/respeccraft`, and others) have both a player and a more capable GM form, so
  they appear in both sections.
- **67 rows** have a complete server-side handler (✅ Handled).
- **43 rows** have an explicit automated test (✅ Tested) — the implemented GM
  commands plus the spatial-chat / ability / respawn player commands (`/say`,
  `/emote`, `/yell`, `/invokeability`, `/respawn`).
- **18 rows** are partial (🚧): route-only stubs that acknowledge the call but
  don't yet drive the full game effect.
- **187 rows** are ❌ — client-side only, or recognized by the client but not yet
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
