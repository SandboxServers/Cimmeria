---
title: "Slash Commands"
type: reference
audience: players, GMs, operators
last_updated: 2026-09-27
---

# Slash Commands

This is the complete list of every `/command` you can type in Stargate Worlds -- all **266** of them, captured straight from the game's own command list.

## How to use a command

1. Press **Enter** (or click the chat box) to start typing.
2. Type a slash, the command, then any extra info it needs -- for example `/say hello` or `/yell incoming!`
3. Press **Enter** again to send it.

Type `/help` in-game any time to see the list inside the game.

## Will it actually do something?

This is a *fan-made* server, so not every command the game knows about is wired up on our end yet. The **"Works now?"** column tells you what to expect:

| You'll see | What it means for you |
|---|---|
| ✅ Yes | Works on our server right now. |
| ✅ Yes *(game handles it)* | Works -- your game handles it locally, no server needed. |
| 🚧 Partly | The server hears it but only does part of the job so far. |
| ❌ Not yet | The game accepts it, but our server doesn't do anything with it yet. |

> **Two kinds of commands.** Most commands are for everyone. Commands that start with **`/gm`** are **Game Master** tools -- they only work if you're signed in on a GM (or higher) account. If you're a normal player, you can ignore the whole "Game Master Commands" section near the bottom.

---

## Player Commands

These work for everyone.

### Getting Around

Move, find yourself, and get unstuck.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/exit` | Exit the game | ✅ Yes *(game handles it)* | none | `/exit` |
| `/location` | Show your current position | ✅ Yes *(game handles it)* | none | `/location` |
| `/logoff` | Log off | ✅ Yes *(game handles it)* | none | `/logoff` |
| `/logout` | Log off | ✅ Yes *(game handles it)* | none | `/logout` |
| `/quit` | Exit the game | ✅ Yes *(game handles it)* | none | `/quit` |
| `/run` | Switch to running gait | ✅ Yes *(game handles it)* | none | `/run` |
| `/showlocation` | Show your current position | ✅ Yes *(game handles it)* | none | `/showlocation` |
| `/unstuck` | Attempt to free a stuck character | 🚧 Partly | none | `/unstuck` |
| `/walk` | Switch to walking gait | ✅ Yes *(game handles it)* | none | `/walk` |

### Talking to People

Chat channels, emotes, friends, and private messages.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/afk` | Set your AFK auto-reply | 🚧 Server side (social-systems, 2026-09-27), not client-tested. A tell to you while you are away is answered with it | `<text>` | `/afk afk, back soon` |
| `/ban` | Ban a player from a channel | ❌ Not yet | `<channel> <player>` | `/ban trade Spammer` |
| `/chat` | Join a chat channel | ❌ Not yet | `<channel>` | `/chat trade` |
| `/chatjoin` | Join a chat channel | ❌ Not yet | `<channel>` | `/chatjoin trade` |
| `/chatleave` | Leave a chat channel | ❌ Not yet | `<channel>` | `/chatleave trade` |
| `/chatlist` | List available channels | ❌ Not yet | none | `/chatlist` |
| `/chatwho` | See who is in a chat channel | ❌ Not yet | none | `/chatwho trade` |
| `/command` | Talk in command/guild chat | ❌ Not yet | `<text>` | `/command event tonight` |
| `/csay` | Talk in your current chat channel | ❌ Not yet | <text> | `/csay hi all` |
| `/dnd` | Set your Do-Not-Disturb auto-reply | 🚧 Server side (social-systems, 2026-09-27), not client-tested. Your lines carry the DND flag, and a tell to you is answered with the message | `<text>` | `/dnd in a mission` |
| `/emote` | Perform an emote (spatial) | ✅ Yes | `<text>` — emote text | `/emote waves` |
| `/friend` | Add a player as a friend | ❌ Not yet | `<player>` | `/friend Sam` |
| `/ignore` | Ignore a player | 🚧 Server side (social-systems, 2026-09-27), not client-tested. Their tells, say, emote, yell, mail and duel challenges stop reaching you; they can still see you | `<player>` | `/ignore Spammer` |
| `/kick` | Kick a player from a channel you own | ❌ Not yet | `<channel> <player>` | `/kick trade Spammer` |
| `/me` | Perform an emote (spatial) | ✅ Yes | `<text>` — emote text | `/me waves` |
| `/moderator` | Grant channel moderator status | ❌ Not yet | <channel> <player> | `/moderator trade Sam` |
| `/mute` | Mute a player | ❌ Not yet | `<player>` | `/mute Loud` |
| `/ntell` | Send a private message by name (numeric form) | ❌ Not yet | `<player> <text>` | `/ntell Jack on my way` |
| `/nwhisper` | Send a private message by name (numeric form) | ❌ Not yet | `<player> <text>` | `/nwhisper Jack on my way` |
| `/officer` | Talk in officer chat | ❌ Not yet | `<text>` | `/officer ranks updated` |
| `/password` | Set or clear a channel password | ❌ Not yet | `<channel> <password>` | `/password trade s3cret` |
| `/petition` | File a support petition | ❌ Not yet | `<text>` | `/petition stuck in geometry` |
| `/say` | Say something in local (spatial) chat | ✅ Yes | `<text>` — the message | `/say hello there` |
| `/squad` | Talk in squad chat | 🚧 Server side (ORG-04), not client-tested. Reaches every squad member in any space; outside a squad you get "You are not in a squad." | `<text>` | `/squad regroup` |
| `/team` | Talk in team chat | ❌ Not yet | `<text>` | `/team push left` |
| `/tell` | Send a private message | 🚧 Server side (social-systems, 2026-09-27), not client-tested. Only to a player who is online; you are told when they are not | `<player> <text>` | `/tell Jack on my way` |
| `/unban` | Lift a channel ban | ❌ Not yet | `<channel> <player>` | `/unban trade Spammer` |
| `/unfriend` | Remove a friend | ❌ Not yet | `<player>` | `/unfriend Sam` |
| `/unmute` | Unmute a player | ❌ Not yet | `<player>` | `/unmute Loud` |
| `/whisper` | Send a private message | ❌ Not yet | `<player> <text>` | `/whisper Jack on my way` |
| `/yell` | Yell — wider spatial range than say | ✅ Yes | `<text>` — the message | `/yell incoming!` |

### Squads, Teams & Guilds

Group up. Most grouping is still being built on our server.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/chooseorgname` | Choose organization name | 🚧 Partly | `<name>` | `/chooseorgname SG-Alpha` |
| `/commanddemote` | Demote in guild | ❌ Not yet | `<player>` | `/commanddemote Sam` |
| `/commandinvite` | Invite to guild | ❌ Not yet | `<player>` | `/commandinvite Sam` |
| `/commandinviteaccept` | Accept guild invite | ❌ Not yet | none | `/commandinviteaccept` |
| `/commandinvitedecline` | Decline guild invite | ❌ Not yet | none | `/commandinvitedecline` |
| `/commandkick` | Kick from guild | ❌ Not yet | `<player>` | `/commandkick Sam` |
| `/commandleave` | Leave guild | ❌ Not yet | none | `/commandleave` |
| `/commandmotd` | Set guild MOTD | ❌ Not yet | `<text>` | `/commandmotd welcome!` |
| `/commandpromote` | Promote in guild | ❌ Not yet | `<player>` | `/commandpromote Sam` |
| `/setcommandnote` | Set a guild member note | ❌ Not yet | `<player> <text>` | `/setcommandnote Sam recruit` |
| `/setcommandofficernote` | Set a guild officer note | ❌ Not yet | `<player> <text>` | `/setcommandofficernote Sam vouched` |
| `/setteamnote` | Set a team member note | ❌ Not yet | `<player> <text>` | `/setteamnote Sam main tank` |
| `/setteamofficernote` | Set a team officer note | ❌ Not yet | `<player> <text>` | `/setteamofficernote Sam promote soon` |
| `/squadinvite` | Invite a player to your squad | ❌ Not yet | `<player>` | `/squadinvite Sam` |
| `/squadinviteaccept` | Accept a squad invite | ❌ Not yet | none | `/squadinviteaccept` |
| `/squadinvitedecline` | Decline a squad invite | ❌ Not yet | none | `/squadinvitedecline` |
| `/squadkick` | Kick a squad member | ❌ Not yet | `<player>` | `/squadkick Sam` |
| `/squadleave` | Leave your squad | ❌ Not yet | none | `/squadleave` |
| `/squadpromote` | Promote a member to leader | ❌ Not yet | `<player>` | `/squadpromote Sam` |
| `/teamdemote` | Demote in team | ❌ Not yet | `<player>` | `/teamdemote Sam` |
| `/teaminvite` | Invite to team | ❌ Not yet | `<player>` | `/teaminvite Sam` |
| `/teaminviteaccept` | Accept team invite | ❌ Not yet | none | `/teaminviteaccept` |
| `/teaminvitedecline` | Decline team invite | ❌ Not yet | none | `/teaminvitedecline` |
| `/teamkick` | Kick from team | ❌ Not yet | `<player>` | `/teamkick Sam` |
| `/teamleave` | Leave team | ❌ Not yet | none | `/teamleave` |
| `/teammotd` | Set team message of the day | ❌ Not yet | `<text>` | `/teammotd raid at 8` |
| `/teampromote` | Promote in team | ❌ Not yet | `<player>` | `/teampromote Sam` |

### Fighting & Abilities

Use abilities, swap ammo, train skills.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/activatebandolierslot` | Switch equipment loadout | ✅ Yes | `<bagId> <slotId>` | `/activatebandolierslot 2 1` |
| `/respecability` | Respec a single ability | ❌ Not yet | `<abilityId>` | `/respecability 4202` |
| `/toggleautocycle` | Toggle ability auto-cycling | 🚧 Partly | `<enabled>` (0/1) | `/toggleautocycle 1` |
| `/trainability` | Learn a new ability (spends a point) | ✅ Yes | `<abilityId>` | `/trainability 4202` |

### Items & Gear

Equip, use, move, buy, and repair gear.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/deleteitem` | Delete an item | ✅ Yes | `<itemId> <quantity>` | `/deleteitem 7700 1` |
| `/equip` | Equip an item | 🚧 Partly | `<itemId>` | `/equip 8800` |
| `/getiteminfo` | Show detail for one item | ❌ Not yet | `<itemId>` | `/getiteminfo 8800` |
| `/listitems` | List your inventory items | ✅ Yes | none | `/listitems` |
| `/lootitem` | Take an item from a loot container | ✅ Yes | `<index>` | `/lootitem 0` |
| `/moveitem` | Move an item between containers | ✅ Yes | `<itemId> <targetBag> <targetSlot> <quantity>` | `/moveitem 7700 1 4 1` |
| `/purchaseitem` | Buy from a vendor | ✅ Yes | `<itemIndex...> <quantity...>` | `/purchaseitem 12 1` |
| `/unequip` | Unequip an item | 🚧 Partly | `<itemId>` | `/unequip 8800` |
| `/useitem` | Use an item | ✅ Yes | `<itemId> <targetId>` | `/useitem 7700 0` |

### Talking to NPCs & Objects

Interact with the world and answer dialogs.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/dialogbuttonchoice` | Pick a dialog button | ✅ Yes | `<dialogId> <buttonId>` | `/dialogbuttonchoice 12 1` |
| `/initialresponse` | Open the initial dialog for an NPC | ✅ Yes | `<dialogSetMapId>` | `/initialresponse 12` |
| `/interact` | Interact with the targeted object/NPC | ✅ Yes | `<overrideTarget>` (0 = current target) | `/interact 0` |

### Missions

Accept, abandon, and share missions.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/abandonmission` | Abandon a mission (alternate form) | ✅ Yes | `<missionId>` | `/abandonmission 1001` |
| `/sharemission` | Share a mission with your team | ✅ Yes | `<missionId>` | `/sharemission 1001` |
| `/sharemissionaccept` | Accept a shared mission | ✅ Yes | none | `/sharemissionaccept` |
| `/sharemissiondecline` | Decline a shared mission | ✅ Yes | none | `/sharemissiondecline` |

### Dueling

Challenge other players.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/duel` | Challenge a player to a duel | 🚧 Server side (social-systems, 2026-09-27), not client-tested. The target must be within 20 units, in your world and not already dueling | `<player>` | `/duel Rival` |
| `/duelforfeit` | Forfeit an active duel | 🚧 Server side (social-systems, 2026-09-27), not client-tested. Only once the countdown has finished | none | `/duelforfeit` |
| `/duelresponse` | Accept or decline a duel challenge | 🚧 Server side (social-systems, 2026-09-27), not client-tested. | `<response>` (accept/decline) | `/duelresponse 1` |

> **Duels (2026-09-27).** 1v1 duels are implemented server-side (social-systems
> SS-D1 to SS-D3): a duel counts down for 5 seconds, then only the two duelists
> can damage each other. It ends on a forfeit, when one would drop to 0 HP (held
> at 1 HP instead: duels never kill), on death from anyone else, on logging off,
> on travel, or after 5 seconds outside the 40-unit duel area. Nothing is won but
> the line "You won the duel". Squad duels are refused. See
> [duel-system.md](gameplay/duel-system.md).

### Crafting

Crafting is mostly driven by the UI; little of it is typed.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `.respeccraft` | Unlearn every crafting discipline and get back the applied science points you spent on them, free. Blueprints and paradigm levels stay. Answer Yes to the prompt within 60 seconds | ✅ Yes | none | `.respeccraft` |
| `/respeccraft` | The client's own slash command; what it sends is not known. Use `.respeccraft` | ❓ Unknown | none | `/respeccraft` |

### Minigames

Start and report minigames.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/debugminigamecomplete` | Report a minigame result | 🚧 Partly | `<gameId> <winnerId> <loserId>` | `/debugminigamecomplete 1 5310 0` |
| `/startminigame` | Start a minigame at a host object | 🚧 Partly | `<hostEntityId> <gameDefId>` | `/startminigame 5400 3` |

### Other Handy Commands

Help, who is online, and odds and ends.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/debugperformance` | Toggle performance debug | ❌ Not yet | none | `/debugperformance` |
| `/help` | Show command help | ✅ Yes *(game handles it)* | none | `/help` |
| `/pvp` | Toggle your PvP flag | ❌ Not yet | `<on>` (0/1) | `/pvp` |
| `/reload` | General data reload | ❌ Not yet | none | `/reload` |
| `/reloadui` | Reload the in-game UI | ✅ Yes *(game handles it)* | none | `/reloadui` |
| `/showfacing` | Report the target's (or your) heading | ✅ Yes | none | `/showfacing` |
| `/showfps` | Show frames per second | ✅ Yes *(game handles it)* | none | `/showfps` |
| `/showposition` | Report your exact position | ✅ Yes | none | `/showposition` |
| `/showracialparadigmlevels` | Show your racial paradigm levels | ❌ Not yet | none | `/showracialparadigmlevels` |
| `/showrotation` | Report the target's (or your) heading | ✅ Yes | none | `/showrotation` |
| `/who` | List online players | 🚧 Partly | none | `/who` |
| `/worldreset` | Reset the world instance | 🚧 Partly | none | `/worldreset` |

---

## Game Master Commands

**You need a Game Master account for everything below.** Normal players can't use these -- the server checks your account level and refuses the command otherwise.

> **Tip for GMs:** these commands take **numbers**, not names. Use a target's numeric id (not a player's name), and they only affect things in your own area.

### Teleporting

Jump yourself or pull others to you.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmgoto` | Teleport yourself to a target entity | ✅ Yes | `<entityId>` (numeric) | `/gmgoto 5310` |
| `/gmgotolocation` | Teleport yourself to a named world + coordinates (full reload). Coordinates `0 0 0` mean the world's entry point (new-character start, else story ring pad, else stargate arrival, else first respawner) | ✅ Yes | `<worldName> <x> <y> <z>` | `/gmgotolocation Abydos 100 5 200`, `/gmgotolocation Harset 0 0 0` |
| `/gmgotoxyz` | Teleport yourself to coordinates in your space | ✅ Yes | `<x> <y> <z>` (floats; must be finite) | `/gmgotoxyz 1200.5 64.0 -880.0` |
| `/gmsummon` | Move a target entity to you | ✅ Yes | `<entityId>` (numeric; not yourself) | `/gmsummon 5310` |

### Giving Yourself Things

XP, money, items, abilities, and more.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmgiveability` | Give yourself one ability without spending a training point. It is saved to your character, survives relog and a trainer respec, and your ability window updates at once. Refused, with a line saying why, for an unknown id or one you already know | ✅ Yes | `<abilityId>` | `/gmgiveability 2826` |
| `/gmgiveallabilities` | Give yourself every ability in your archetype's tree (all three branches) in one go. Saved, no training points spent, one ability-window refresh, and a line with the count | ✅ Yes | none | `/gmgiveallabilities` |
| `/gmgiveammo` | Give special-ammo rounds | ✅ Yes, as `.giveammo` | `<ammo type> <rounds>` | `.giveammo hollowpoint 500` |
| `/gmgiveappliedsciencepoints` | Give yourself applied-science points | ✅ Yes | `<points>` (positive int) | `/gmgiveappliedsciencepoints 25` |
| `/gmgiveblueprint` | Give a crafting blueprint | ❌ Not yet | `<blueprintId>` | `/gmgiveblueprint` |
| `/gmgivecash` | Give yourself naquadah (currency) | ✅ Yes | `<amount>` (positive int) | `/gmgivecash 10000` |
| `/gmgiveexpertise` | Give yourself crafting expertise in a discipline | ✅ Yes | `<disciplineId> <amount>` (both positive) | `/gmgiveexpertise 3 50` |
| `/gmgivefaction` | Give faction standing | ❌ Not yet | `<factionId> <amount>` | `/gmgivefaction` |
| `/gmgivegearset` | Give a full gear set | ❌ Not yet | `<gearsetId>` | `/gmgivegearset` |
| `/gmgiveinventory` | Give a full inventory loadout | ❌ Not yet | `<inventoryId>` | `/gmgiveinventory` |
| `/gmgiveitem` | Give an item to your inventory | ✅ Yes | `<designId> <quantity>` (numeric id; qty clamped 1–1000) | `/gmgiveitem 8800 5` |
| `/gmgiveracialparadigmlevels` | Give racial paradigm levels | ❌ Not yet | `<id> <levels>` | `/gmgiveracialparadigmlevels` |
| `/gmgiverespawner` | Give a player respawner | ❌ Not yet | `<mobId>` | `/gmgiverespawner` |
| `/gmgivestargateaddress` | Give a stargate address | ❌ Not yet | `<address> <target> <hidden>` | `/gmgivestargateaddress` |
| `/gmgivetrainingpoints` | Give yourself ability-tree training points (the counter updates at once, and an open trainer refreshes) | ✅ Yes | `<count>` (positive int; refused if your total would pass 2147483647) | `/gmgivetrainingpoints 5` |
| `/gmgivexp` | Give yourself experience | ✅ Yes | `<amount>` (positive int) | `/gmgivexp 5000` |
| `/gmrechargeitem` | Recharge an item | ✅ Yes | `<itemId...>` | `/gmrechargeitem 8800` |
| `/gmremoveitem` | Remove a quantity of an inventory item from yourself | ✅ Yes | `<itemId> <quantity>` (both positive) | `/gmremoveitem 8800 1` |
| `/gmremovestargateaddress` | Remove a stargate address | ❌ Not yet | `<address> <target>` | `/gmremovestargateaddress` |
| `/gmrepairitem` | Repair an item | ✅ Yes | `<itemId...>` | `/gmrepairitem 8800` |
| `/gmsetfaction` | Set faction standing | ❌ Not yet | `<factionId> <value>` | `/gmsetfaction` |
| `/gmsettechskill` | Set a tech (crafting) skill | ❌ Not yet | `<skillId> <value>` | `/gmsettechskill` |

### Changing Stats & Toggles

Set health/focus, target, and debug toggles.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmchangeammo` | Switch ammo type | ✅ Yes | `<itemId> <ammoType>` | `/gmchangeammo 8800 3` |
| `/gminvokeability` | Use an ability on a target. No exposed cell method exists for it (`SGWAbilityManager.invokeAbility` is server-only), so the server never receives it; earlier versions of this page wrongly said it worked | ❌ No | `<abilityId> <targetId>` | `/gminvokeability 4201 5310` |
| `/gmresetabilities` | Reset your abilities to your archetype's starters: trained, quest and GM-given abilities are removed and your spent tree points come back. No trainer needed and no naquadah charged (unlike the trainer's Respec) | ✅ Yes | none | `/gmresetabilities` |
| `/gmrespec` | Full respec (GM) | ❌ Not yet | none | `/gmrespec` |
| `/gmsetarchetype` | Set a character's archetype | ❌ Not yet | `<archetypeId>` | `/gmsetarchetype` |
| `/gmsetflag` | Force-set a state flag | ❌ Not yet | `<flagId> <force>` | `/gmsetflag` |
| `/gmsetfly` | Toggle fly mode | ❌ Not yet | `<on>` (0/1) | `/gmsetfly` |
| `/gmsetfocus` | Set current focus | ✅ Yes | `<amount> <targetId>` | `/gmsetfocus 300 0` |
| `/gmsetfocusmax` | Set maximum focus | ✅ Yes | `<amount> <targetId>` | `/gmsetfocusmax 400 0` |
| `/gmsetghost` | Toggle ghost (no-collision) mode | ❌ Not yet | `<on>` (0/1) | `/gmsetghost` |
| `/gmsetgodmode` | Take no Health or Focus damage from ability hits and damage over time; hits show "Absorbed". Heals, buffs and debuffs still land. Only ever on yourself, and off again when you relog | ✅ Yes | `<on>` (0/1) | `/gmsetgodmode 1` |
| `/gmsethealth` | Set current health on yourself or a target | ✅ Yes | `<amount> <targetId>` (amount ≥ 0; target 0 = self) | `/gmsethealth 500 0` |
| `/gmsethealthmax` | Set maximum health | ✅ Yes | `<amount> <targetId>` | `/gmsethealthmax 1000 0` |
| `/gmsethidegm` | Toggle GM visibility | ❌ Not yet | `<on>` (0/1) | `/gmsethidegm` |
| `/gmsetignorefocus` | Ignore focus costs | ❌ Not yet | `<on>` (0/1) | `/gmsetignorefocus` |
| `/gmsetignorehealth` | Ignore health damage | ❌ Not yet | `<on>` (0/1) | `/gmsetignorehealth` |
| `/gmsetinfiniteammo` | Special-ammo reloads take nothing from your bags | ✅ Yes, as `.infiniteammo` | `[on\|off]` | `.infiniteammo on` |
| `/gmsetinvulnerable` | Toggle invulnerability | ❌ Not yet | `<on>` (0/1) | `/gmsetinvulnerable` |
| `/gmsetlevel` | Set a character's level | ❌ Not yet | `<level>` | `/gmsetlevel` |
| `/gmsetmobabilityset` | Give your selected mob the abilities of an NPC ability set (`resources.ability_set_abilities`) until it respawns. Refused for a player, a mob in another area, or an unknown set | ✅ Yes | `<setId>` | `/gmsetmobabilityset 350` |
| `/gmsetmobattribute` | Set an NPC attribute | ❌ Not yet | `<target> <attr> <type> <value>` | `/gmsetmobattribute` |
| `/gmsetmobstance` | Set an NPC's stance | ❌ Not yet | `<stance>` | `/gmsetmobstance` |
| `/gmsetmobvariable` | Set a generic NPC variable | ❌ Not yet | `<var> <value>` | `/gmsetmobvariable` |
| `/gmsetnoaggro` | Toggle NPC aggro | ❌ Not yet | `<on>` (0/1) | `/gmsetnoaggro` |
| `/gmsetnodamage` | Toggle timed damage immunity | ❌ Not yet | `<on>` (0/1) | `/gmsetnodamage` |
| `/gmsetnotarget` | Make yourself untargetable | ❌ Not yet | `<on>` (0/1) | `/gmsetnotarget` |
| `/gmsetnoxp` | Toggle XP gain off | ❌ Not yet | `<on>` (0/1) | `/gmsetnoxp` |
| `/gmsetomnipotent` | Toggle all-powerful debug mode | ❌ Not yet | `<on>` (0/1) | `/gmsetomnipotent` |
| `/gmsetpvp` | Toggle your PvP flag | ❌ Not yet | `<on>` (0/1) | `/gmsetpvp` |
| `/gmsetspectator` | Toggle spectator mode | ❌ Not yet | `<on>` (0/1) | `/gmsetspectator` |
| `/gmsetspeed` | Set movement speed | ❌ Not yet | `<multiplier>` | `/gmsetspeed` |
| `/gmsettarget` | Set your current target | ✅ Yes | `<entityId>` (numeric; 0 = clear) | `/gmsettarget 5310` |
| `/gmtoggleautocycleability` | Toggle ability auto-cycling | 🚧 Partly | `<enabled>` (0/1) | `/gmtoggleautocycleability 1` |

### Spawning & World

Spawn, kill, and despawn NPCs.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmdespawn` | Remove an NPC from the space (NPC-only) | ✅ Yes | `<entityId>` (numeric) | `/gmdespawn 5400` |
| `/gmdumpobjects` | Dump the object list to the log | ❌ Not yet | none | `/gmdumpobjects` |
| `/gmkilltarget` | Kill an NPC via the canonical death sequence (NPC-only) | ✅ Yes | `<entityId>` (numeric) | `/gmkilltarget 5400` |
| `/gmrespawn` | Respawn after death | ✅ Yes | none | `/gmrespawn` |
| `/gmspawnbycmd` | Spawn an NPC by template id at your position + offset | ✅ Yes | `<designId> <xOffset> <zOffset>` (numeric template id; float offsets) | `/gmspawnbycmd 3402 5.0 0.0` |
| `/gmspawnentityloot` | Force-drop loot from an entity | ❌ Not yet | `<entity> <lootTableId>` | `/gmspawnentityloot` |

### GM Stargate

Dial a gate by raw address.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmdhd` | Dial a stargate by numeric address | ✅ Yes | `<gateAddress>` (positive int) | `/gmdhd 14` |

### Looking Things Up

Inspect players, NPCs, positions, and flags. These report back to you.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmgetmobattribute` | Report one attribute of an NPC | ✅ Yes | `<targetId> <attribute>` (health/focus/level/faction/alignment/aistate/name/template/pos) | `/gmgetmobattribute 5400 health` |
| `/gmhelpfull` | Show all available commands | ✅ Yes *(game handles it)* | none | `/gmhelpfull` |
| `/gmlistabilities` | List the known ability ids of the target (or you) | ✅ Yes | none | `/gmlistabilities` |
| `/gmlistinteractions` | List interactions available on the target | ❌ Not yet | none | `/gmlistinteractions` |
| `/gmprintstats` | Print a named server statistic | ❌ Not yet | `<stat>` | `/gmprintstats` |
| `/gmshowflag` | Report whether a state-flag bit is set | ✅ Yes | `<flagId>` (bit index 0–31) | `/gmshowflag 4` |
| `/gmshowinventory` | Show a player's inventory | ❌ Not yet | `<target>` | `/gmshowinventory` |
| `/gmshowip` | Show a player's IP | ❌ Not yet | `<target>` | `/gmshowip` |
| `/gmshowmobcount` | Count NPCs in a space | ✅ Yes | `<spaceId>` (0 = your current space) | `/gmshowmobcount 0` |
| `/gmshowplayer` | Dump id / name / kind / faction / level / health / position for an entity | ✅ Yes | `<targetId>` (0 = current target, then self) | `/gmshowplayer 5310` |
| `/gmshowtargetlocation` | Report the target's (or your) position | ✅ Yes | none | `/gmshowtargetlocation` |
| `/gmtestlos` | Report navmesh line-of-sight between two entities | ✅ Yes | `<sourceId> <targetId>` (numeric, same space) | `/gmtestlos 5310 5400` |
| `/gmtestsequence` | Play a test animation sequence | ✅ Yes *(game handles it)* | `<sequenceName>` | `/gmtestsequence idle` |
| `/gmusers` | List players in your space | ✅ Yes | none | `/gmusers` |

### GM Mission Tools

Assign, advance, and inspect missions on yourself.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmmissionabandon` | Alias of `/missionclear` | ✅ Yes | `<missionId>` | `/gmmissionabandon 1001` |
| `/gmmissionadvance` | Jump a mission to a specific step | ✅ Yes | `<missionId> <step>` (step positive) | `/gmmissionadvance 1001 3` |
| `/gmmissionassign` | Assign a mission to yourself by numeric id | ✅ Yes | `<missionId> <popup>` (popup is a UI hint) | `/gmmissionassign 1001 1` |
| `/gmmissionclear` | Abandon one mission by numeric id | ✅ Yes | `<missionId>` | `/gmmissionclear 1001` |
| `/gmmissionclearactive` | Clear all active missions | ❌ Not yet | none | `/gmmissionclearactive` |
| `/gmmissionclearhistory` | Clear mission history | ❌ Not yet | none | `/gmmissionclearhistory` |
| `/gmmissioncomplete` | Complete a mission | ❌ Not yet | `<missionId> <turnIn>` | `/gmmissioncomplete` |
| `/gmmissiondetails` | Show one mission's detail by numeric id | ✅ Yes | `<missionId>` | `/gmmissiondetails 1001` |
| `/gmmissionlist` | List your active missions | ✅ Yes | none | `/gmmissionlist` |
| `/gmmissionlistfull` | List all your missions (incl. completed/hidden) | ✅ Yes | none | `/gmmissionlistfull` |
| `/gmmissionreset` | Revert a mission to a step | ❌ Not yet | `<missionId> <step>` | `/gmmissionreset` |
| `/gmmissionsetavailable` | Make a mission available | ❌ Not yet | `<missionId>` | `/gmmissionsetavailable` |

### Debugging

Combat, AI, and minigame debug switches.

The five ability and combat debug commands (`/gmdebugability`, `/gmdebugabilityonmob`, `/gmdebugcombat`, `/gmdebugcombatverbose`, `/gmdebugheal`; the heal one's keyword is name-derived, like the others) are sent by the game only from a Game Master character; from a normal character the game drops them silently before they leave your computer. The game has no combat-debug window: any text a server returns for them arrives as an ordinary chat line. Details: [native-combat-debug.md](reverse-engineering/findings/native-combat-debug.md).

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmaddbehavioreventset` | Add a behavior event set | ❌ Not yet | `<id>` | `/gmaddbehavioreventset` |
| `/gmchat` | Send a GM chat message | ❌ Not yet | `<text>` | `/gmchat` |
| `/gmconfirmeffect` | Respond to an effect-confirmation prompt | ✅ Yes | `<choice>` | `/gmconfirmeffect 1` |
| `/gmdebugability` | Toggle ability debug | ❌ Not yet | `<abilityId>` | `/gmdebugability` |
| `/gmdebugabilityonmob` | Run an ability on an NPC for debug | ❌ Not yet | `<abilityId>` | `/gmdebugabilityonmob` |
| `/gmdebugbehaviorsonmob` | Stream an NPC's behavior state | ❌ Not yet | none | `/gmdebugbehaviorsonmob` |
| `/gmdebugcombat` | Toggle combat debug info | 🚧 Partly | none | `/gmdebugcombat` |
| `/gmdebugcombatverbose` | Toggle verbose combat debug | 🚧 Partly | none | `/gmdebugcombatverbose` |
| `/gmdebugevents` | Toggle event debug | ❌ Not yet | `<target> <level>` | `/gmdebugevents` |
| `/gmdebugflash` | Toggle Flash UI debug | ❌ Not yet | none | `/gmdebugflash` |
| `/gmdebugheal` | Toggle healing debug info | 🚧 Partly | none | `/gmdebugheal` |
| `/gmdebuginteract` | Force an interaction for debug | ❌ Not yet | none | `/gmdebuginteract` |
| `/gmdebugjoinminigame` | Join a minigame for debug | ❌ Not yet | `<gameId>` | `/gmdebugjoinminigame` |
| `/gmdebugminigameinstance` | Inspect a minigame instance | ❌ Not yet | `<instanceId>` | `/gmdebugminigameinstance` |
| `/gmdebugpathsonmob` | Stream an NPC's nav path | ❌ Not yet | none | `/gmdebugpathsonmob` |
| `/gmdebugspectateminigame` | Spectate a minigame for debug | ❌ Not yet | `<gameId>` | `/gmdebugspectateminigame` |
| `/gmdebugstartminigame` | Start a minigame for debug | ❌ Not yet | `<gameId>` | `/gmdebugstartminigame` |
| `/gmdebugtarget` | Dump debug data for your target | ❌ Not yet | `<target>` | `/gmdebugtarget` |
| `/gmemitbehavioreventonmob` | Emit a behavior event on an NPC | ❌ Not yet | `<id>` | `/gmemitbehavioreventonmob` |
| `/gmentererroraistate` | Force an NPC into the error AI state | ❌ Not yet | none | `/gmentererroraistate` |
| `/gmexiterroraistate` | Clear an NPC's error AI state | ❌ Not yet | none | `/gmexiterroraistate` |
| `/gmforceclientcrash` | Force the client to crash (debug) | ✅ Yes *(game handles it)* | none | `/gmforceclientcrash` |
| `/gmforcerendercrash` | Force the render thread to crash (debug) | ✅ Yes *(game handles it)* | none | `/gmforcerendercrash` |
| `/gmgiveminigamecontact` | Grant a minigame contact | ❌ Not yet | `<contactId> <target>` | `/gmgiveminigamecontact` |
| `/gmmobdata` | Dump an NPC's debug data (template, AI state, faction, health, threat) | ✅ Yes | `<spaceId> <targetId>` | `/gmmobdata 0 5400` |
| `/gmonphysics` | Toggle physics debug | ❌ Not yet | `<on>` (0/1) | `/gmonphysics` |
| `/gmperfstatsbychannel` | Toggle per-channel perf stats | ❌ Not yet | `<onOff>` | `/gmperfstatsbychannel` |
| `/gmpetabilitytoggle` | Toggle pet auto-ability | 🚧 Partly | `<petId> <abilityId> <toggle>` | `/gmpetabilitytoggle 9001 4300 1` |
| `/gmpetinvokeability` | Command pet to use an ability | 🚧 Partly | `<petId> <abilityId> <targetId>` | `/gmpetinvokeability 9001 4300 5310` |
| `/gmpetinvokecommand` | Give pet a command | ❌ Not yet | `<petId> <command>` | `/gmpetinvokecommand 9001 attack` |
| `/gmremovebehavioreventset` | Remove a behavior event set | ❌ Not yet | `<id>` | `/gmremovebehavioreventset` |
| `/gmremoveminigamecontact` | Remove a minigame contact | ❌ Not yet | `<contactId> <target>` | `/gmremoveminigamecontact` |
| `/gmshout` | Send a server-channel line to your space or to every online player (`.announce` does the same) | ✅ Yes | `<global> <text>` (how the client splits the typed line is not recovered) | `/gmshout` |
| `/gmsetringdestination` | Set ring transporter destination | ✅ Yes | `<regionId> <destinationId>` | `/gmsetringdestination 3 7` |
| `/gmspacequeuedresponse` | Respond to a queued-for-space prompt | ❌ Not yet | `<response>` | `/gmspacequeuedresponse 1` |
| `/gmspacequeuereadyresponse` | Respond to a space-ready prompt | ❌ Not yet | `<response>` | `/gmspacequeuereadyresponse 1` |
| `/gmspacequeuestatus` | Query your space-instance queue status | ❌ Not yet | none | `/gmspacequeuestatus` |
| `/gmtimeofday` | Set the time of day | ❌ Not yet | `<hour>` | `/gmtimeofday` |
| `/gmtogglecombatlos` | Toggle combat line-of-sight enforcement | ❌ Not yet | none | `/gmtogglecombatlos` |

### Visual Overlays

On-screen overlays: FPS, navmesh, cover, and more.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmoninvisible` | Toggle invisibility | ❌ Not yet | `<on>` (0/1) | `/gmoninvisible` |
| `/gmonxrayeyes` | Toggle x-ray vision | ❌ Not yet | `<on>` (0/1) | `/gmonxrayeyes` |
| `/gmsetinstanceflag` | Set a space-instance flag | ❌ Not yet | `<flag> <value>` | `/gmsetinstanceflag` |
| `/gmshowarea` | Show area bounds | ✅ Yes *(game handles it)* | none | `/gmshowarea` |
| `/gmshowcommandwaypoints` | Show command-group waypoints | ✅ Yes *(game handles it)* | none | `/gmshowcommandwaypoints` |
| `/gmshowcover` | Visualize cover links | ✅ Yes *(game handles it)* | none | `/gmshowcover` |
| `/gmshowinstanceflag` | Show a space-instance flag | ❌ Not yet | `<flag>` | `/gmshowinstanceflag` |
| `/gmshowlog` | Show log output | ✅ Yes *(game handles it)* | none | `/gmshowlog` |
| `/gmshowmemory` | Show memory usage | ✅ Yes *(game handles it)* | none | `/gmshowmemory` |
| `/gmshownavmesh` | Show navigation mesh | ❌ Not yet | none | `/gmshownavmesh` |
| `/gmshowregion` | Show region bounds | ✅ Yes *(game handles it)* | none | `/gmshowregion` |
| `/gmshowspawnset` | Show a spawn set | ❌ Not yet | `<setId>` | `/gmshowspawnset` |
| `/gmshowtriggers` | Show trigger volumes | ✅ Yes *(game handles it)* | none | `/gmshowtriggers` |
| `/gmshowwaypoints` | Show waypoints | ✅ Yes *(game handles it)* | none | `/gmshowwaypoints` |

### Cover System

Tune the NPC cover system.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmchangecoverstanceweight` | Adjust per-stance cover weights | ❌ Not yet | `<stance> <6 values>` | `/gmchangecoverstanceweight` |
| `/gmchangecoverweight` | Adjust cover scoring weights | ❌ Not yet | `<6 floats>` | `/gmchangecoverweight` |
| `/gmregeneratecoverlinks` | Recompute cover links | ❌ Not yet | `<normLimit> <maxLinks> <maxDist>` | `/gmregeneratecoverlinks` |
| `/gmtrackmob` | Toggle a debug track on an NPC | ❌ Not yet | none | `/gmtrackmob` |

### Reloading Content

Reload data without a restart.

| Command | What it does | Works now? | Parameters | Example |
|---|---|---|---|---|
| `/gmloadability` | Reload an ability definition | ❌ Not yet | `<id>` | `/gmloadability` |
| `/gmloadabilityset` | Reload an ability set | ❌ Not yet | `<id>` | `/gmloadabilityset` |
| `/gmloadbehavior` | Reload an AI behavior | ❌ Not yet | `<id>` | `/gmloadbehavior` |
| `/gmloadconstants` | Reload game constants | ❌ Not yet | none | `/gmloadconstants` |
| `/gmloaddialogset` | Hot-reload a dialog-set definition | ❌ Not yet | <id> | `/gmloaddialogset 12` |
| `/gmloadinteractionset` | Reload an interaction set | ❌ Not yet | `<id>` | `/gmloadinteractionset` |
| `/gmloaditem` | Reload an item definition | ❌ Not yet | `<id>` | `/gmloaditem` |
| `/gmloadmission` | Reload a mission | ❌ Not yet | `<id>` | `/gmloadmission` |
| `/gmloadmob` | Reload an NPC definition | ❌ Not yet | `<id>` | `/gmloadmob` |
| `/gmreloadinventory` | Reload inventory defs | ❌ Not yet | none | `/gmreloadinventory` |
| `/gmreloadorganizations` | Reload organization defs | ❌ Not yet | none | `/gmreloadorganizations` |
| `/gmreloadscripts` | Hot-reload content/mission scripts | ❌ Not yet | none | `/gmreloadscripts` |

---

## Dev console (`.`-commands)

The 266 `/`-commands above are baked into the game client and can't be added to.
The legacy server and the FanMMORPG fork shipped a second set of **dev/authoring
commands** that the client never exposed as slash commands. Those use a separate
**`.`-prefixed console**: the client doesn't intercept `.`-text — it sends it as
an ordinary chat message, and the server intercepts it. (This is why `.` works
where a new `/command` can't: the client *eats* unknown `/`-input but *forwards*
`.`-input.)

**Game-Master only.** A GM's `.`-command is consumed by the server and never
appears in anyone else's chat. A normal player who types a registered command
(`.pet`, `.giveability`, ...) gets "is a GM command" back and nobody else sees
it. Any other `.`-message is ordinary chat. Auth is checked against your account
level server-side. Type `.help` (or `.help <word>`) in-game for the live list.

### Database persistence & the seed-commit flow

Some `.`-commands change **persistent game data** (spawns, patrol paths). These
are the only commands in the console that touch the database, and how they
persist is the most important thing to understand before you use them.

> **⚠️ Nothing is saved until you `.seedconfirm`, and even then only until the next deploy.**
>
> A persistence command changes the world in memory at once and **queues** its
> SQL. It touches no database. `.seedconfirm` writes your queued changes to the
> **running database**, so they survive reconnects and server restarts
> **within the current deploy**, and sends them to SigNoz for a developer to
> merge. `.seedcancel` throws the queue away and nothing is written.
>
> **The next deploy rebuilds the database from the seed files in
> `db/resources/` and erases anything that isn't committed there.** A live DB
> write that you never commit to a seed file **is lost on the next deploy.**
>
> To preserve authored content deploy-over-deploy you **must** copy the
> generated seed SQL into the right `db/resources/…` file and commit it to git
> (steps below). This is deliberate: the seed files are the single source of
> truth, and we never hand-write `db/scripts/*.sql` migrations.

**Exactly which commands queue a database change, and the seed file each one
must be committed into:**

| Command | DB operation | Table | Commit the SQL into |
|---|---|---|---|
| `.savespawn` | INSERT (new) or UPDATE (existing row) | `resources.spawnlist` | `db/resources/Worlds/Seed/spawnlist.sql` |
| `.delspawn` | DELETE one row by `spawn_id` | `resources.spawnlist` | `db/resources/Worlds/Seed/spawnlist.sql` |
| `.path_add` | INSERT a waypoint (+ a `point_sets` header row on the first waypoint) | `resources.point_set_points` (+ `resources.point_sets`) | `db/resources/Events/Seed/point_set_points.sql` (+ `point_sets.sql`) |
| `.path_clear` | DELETE all waypoints + the header | `resources.point_set_points` + `resources.point_sets` | `db/resources/Events/Seed/point_set_points.sql` + `point_sets.sql` |
| `.path_assign` | UPDATE the spawn's patrol override (`patrol_path_id`, `patrol_point_delay`) | `resources.spawnlist` | `db/resources/Worlds/Seed/spawnlist.sql` |
| `.path_unassign` | UPDATE — clear the spawn's patrol override | `resources.spawnlist` | `db/resources/Worlds/Seed/spawnlist.sql` |
| `.path_set_seq` · `.path_clear_seq` · `.path_set_tp` · `.path_clear_tp` · `.path_set_tp_seq` · `.path_set_tp_delay` | UPDATE one waypoint (sequence / teleport fields) | `resources.point_set_points` | `db/resources/Events/Seed/point_set_points.sql` |

**Commands that do NOT persist (in-memory only — lost on the next server
restart, never written to the DB):**

- **All entity-authoring commands** (`.tag`, `.name`, `.alignment`, `.nameid`,
  `.staticmesh`, `.bodyset`, `.eventset`, `.interactiontype`, `.lookat`,
  `.visible`, `.setcombatant`, `.unsetcombatant`, `.addcomponent`,
  `.delcomponent`, `.adddialog`, `.removedialog`, `.dynamicupdate`). To make an
  edited entity stick, place/configure it, then `.savespawn` it. `.savespawn`
  saves position, heading and tag only.
- The placement commands `.movehere`, `.location` and `.rotation`. On an NPC,
  they and `.lookat` also make the new spot and facing the NPC's home, so it
  does not walk or turn back to where it spawned.
- `.respawnall` — a live reset of every NPC in your space; no DB write.
- `.autosavespawn 1|0` — a per-session toggle. While it is on, `.movehere`,
  `.lookat`, `.location` and `.rotation` on an NPC queue a `.savespawn` for
  you.

**The queue → confirm → merge workflow:**

1. **Author in-game.** Run the persistence commands (`.savespawn`, `.path_add`,
   …). Each one applies in memory and **queues** the exact SQL (`savespawn:
   queued -- 3 pending`). Saving the same NPC again before you confirm replaces
   its queued change instead of adding a second one. The raw SQL is **not**
   shown in-game: the client's chat can't be copy-pasted, so it goes
   out-of-band instead.
2. **It's logged on the server host.** Every queued statement is appended as it
   happens to a per-session file: **`logs/seed-authoring-<session>.sql`**
   (override the directory with the `CIMMERIA_AUTHORING_LOG_DIR` env var).
   Confirmed batches are marked `CONFIRMED batch <id>` in the same file.
3. **Confirm when satisfied.** Run **`.seedconfirm`**. It writes each queued
   change to the running database and emits the batch to SigNoz under the
   `authoring` target, all stamped with one `batch` id that the confirm line
   shows you (`Sent for merge as batch 1-1790...`): one `seed spawn confirmed`
   event per spawn, with every column as its own field, and one `seed
   authoring confirmed` event per seed file with the statements to append. Use
   `.seedpending` to see what's queued and `.seedcancel` to discard it.
4. **A developer merges the batch (required to survive a deploy).** Hand the
   batch id to a developer. [Placing NPCs and objects](guides/placing-npcs-and-objects.md#for-developers-merging-a-batch)
   says how to turn it into a seed commit. **Until that commit lands, the
   change lives only in the running DB and the next deploy will wipe it.**

A new NPC (one you `.spawn`ed) has no spawn id until the server restarts, so
once its row is confirmed, `.savespawn` on it again is refused rather than
inserting a duplicate row. Place it, save it as often as you like, and confirm
once at the end.

### Command families

| Family | Commands | Works now? |
|---|---|---|
| Search | `.searchitem` `.searchmission` `.searchtemplate` `.players` | ✅ Yes |
| Stat readouts | `.primarystats` `.speedstats` `.armorstats` `.qrstats` `.absorbstats` `.stealthstats` | ✅ Yes |
| Entity authoring | `.tag` `.name` `.alignment` `.nameid` `.staticmesh` `.bodyset` `.eventset` `.interactiontype` `.lookat` `.visible` `.setcombatant` `.unsetcombatant` `.addcomponent` `.delcomponent` `.adddialog` `.removedialog` `.dynamicupdate` | 🚧 In-memory (pair with `.savespawn`) |
| Net / AI debug | `.net_seq` `.net_seqto` `.net_seqfrom` `.net_timer` `.net_mapinfo` `.net_speak` `.net_dialog` `.net_challenge` `.debug_velocity` `.debug_controller` `.debug_follow` `.threaten` `.aggression` `.aggro` | ✅ Yes. `.aggro off` stops idle mobs noticing you, `.aggro on` restores it, and `.aggro` alone shows it. `.aggression <1-5|0|clear>` sets the selected mob's aggression override (1 hostile, 3 neutral; 0 means passive; `clear` goes back to its faction) |
| Crafting | `.learndiscipline` `.forgetdiscipline` `.allcraft` `.craftkit` `.learnblueprint` | ✅ Yes. `.allcraft` on a targeted player sets every paradigm to 7, every discipline to 100 and grants every blueprint (saved), and lets that player craft anywhere until logout. `.craftkit <blueprintId> [count]` grants the target the items of the blueprint's component set 1, `count` (1-10, default 1) times over, into the bags the items allow (crafting components land in the crafting bag). `.learnblueprint <blueprintId>` teaches the target one blueprint. Both answer the GM with one line, refusals included |
| Mission gaps | `.missionfail` `.missionrewards` | ✅ / 🚧 (preview) |
| Spawn authoring | `.spawn` `.despawn` `.savespawn` `.delspawn` `.autosavespawn` `.respawnall` `.spawnrandom` | ✅ Yes — `.savespawn`/`.delspawn` **queue a DB change** (`.seedconfirm` writes it; a developer merges the seed). Walkthrough: [Placing NPCs and objects](guides/placing-npcs-and-objects.md) |
| Placement | `.movehere` `.location` `.rotation` `.lookat` | ✅ Yes — `.movehere` puts the selected NPC or object where you stand, facing the way you face. `.location` and `.rotation` with no arguments report the target's position and facing (in degrees too); with `x y z` or `pitch yaw roll` (radians) they set it. `.lookat` turns the target to face you. On an NPC each one also becomes its home. Pair with `.savespawn` |
| Patrol authoring | `.path_add` `.path_show` `.path_clear` `.path_assign` `.path_unassign` `.path_set_seq` `.path_clear_seq` `.path_set_tp` `.path_clear_tp` `.path_set_tp_seq` `.path_set_tp_delay` | ✅ Yes — all except `.path_show` **queue a DB change** (`.seedconfirm` writes it; commit the seed) |
| Server / maint | `.save` `.reloadmap` `.reloadres` `.removerespawner` `.loglevel` `.logclient` | ❌ Differs (see `.help`) |
| Seed commit | `.seedconfirm` `.seedpending` `.seedcancel` | ✅ Yes |
| Broadcast | `.announce` | ✅ Yes. `.announce <text>` reaches every online player, `.announce space <text>` only your space instance. Same line as `/gmshout`: your name, the GM flag, the server channel |
| Chat mutes | `.mute` `.unmute` | ✅ Yes (server side). `.mute <name> <minutes> [reason]` (1 to 10080) stops an online player's chat and tells; they are told for how long, and each refused line says how much is left. `.unmute <name>` lifts it. A mute holds across relog and ends at a server restart. A GM cannot be muted |
| Ammo | `.giveammo` `.infiniteammo` (aliases `.gmgiveammo` `.gmsetinfiniteammo`) | ✅ Yes — `.giveammo <type> <rounds>` gives the selected player (else you) rounds of a special ammo type as bag stacks, filled to the 500-round cap. The type can be its number (`3`), its name in any case (`Bullet_Hollow_Point`, `hollowpoint`), `hp`/`ap`, or the ammo item id (`9001`); `emp` is ambiguous, so write `bullet_emp` or `dart_emp`. Default ammo is refused, since it is free. Counts above 5000 are cut to 5000, and rounds that do not fit in the bags are reported, never lost. `.infiniteammo on\|off` makes special-ammo reloads take nothing from the bags; the clip still empties and still needs a reload. It lasts until the server restarts, and `.infiniteammo` alone shows it. The native `/gmgiveammo` and `/gmsetinfiniteammo` do nothing: the client never sends them to the server, so use the `.` commands |
| Grants | `.givecash` `.givexp` `.giveability` | ✅ Yes — `.giveability <abilityId>` gives the selected player (else you) the ability and **saves it to the character**: it survives relog and a trainer respec, costs no points, and refreshes the ability window at once. Pets UAT uses it for Summon Straegis (`.giveability 2826`) |
| Pets | `.pet summon <templateId\|abilityId>` `.pet dismiss` `.pet stance <0-2>` `.pet info` `.pet list` | ✅ Yes (server side) — `summon` spawns a pet beside you at once, with no warmup, replacing your current pet; an id with a `pet_summons` row (2826) is the summon ability, anything else a template id (350). `stance` is 0 passive, 1 defensive, 2 aggressive. `info` shows the selected pet, else yours: owner, stance, abilities, toggled-off abilities, AI state, distance to its owner, last teleport. `list` shows every pet in your space with its owner. Nothing is saved: pets are per session |
| Playtest bookmark | `.bug <note>` | ✅ Yes. Snapshots you, your target and every entity within 60 units into SigNoz with your note, so a tester's "this looks wrong" can be found later; see the [telemetry runbook](operations/npc-ai-telemetry-runbook.md#start-from-a-bug-bookmark) |
| Ability lab | `.effects` `.cooldowns [reset [abilityId]]` `.dummy [hostile\|friendly\|clear] [templateId]` `.cleareffects` | ✅ Yes (server side; no legacy counterpart, ability-mechanics AB-L2). `.effects` shows the ability state of your selected target (else you) in a few chat lines: Health, Focus, the state field and any warmup, then cooldowns, pulsing effects, effect-ledger entries (stat deltas, time left or held, absorb pools, cast id) and held state flags; the lab's `server_ability_state` returns the same snapshot in full, and every `.bug` writes it to SigNoz (`abilities.snapshot`). `.cooldowns` lists your running cooldowns; `.cooldowns reset` clears all of yours and `.cooldowns reset <abilityId>` one (with its moniker groups), and each sends your client the clear timer so the hotbar sweep stops too (the one-ability form sends it even when the server had none running). It never touches another player. `.dummy` (or `.dummy friendly`) places a target 3 m in front of you, facing you: template 34 (SGC Jaffa) unless you name one, 1,000,000 Health, its template's Defense and Accuracy (printed in the reply), **no AI at all** so it never attacks, chases or leashes, and no respawn. `hostile` makes it faction 10, so your attacks land; `friendly` makes it faction 9 (Friendly_Ambient), so attacks are refused and no NPC targets it, whatever the template's own faction. Taking a dummy away (clear, expiry, logout) also takes everyone who hit it out of combat with it. It goes after 10 minutes, when you log out, or on `.dummy clear`, which removes only your own; at most 4 each. `.cleareffects` strips every timed and pulsing effect from your selected target (else you), reason `cleansed`: stats, absorb pools and stun locks come back off and the client's effect icons clear. Each change also logs one `abilities.gm` row. `.qr` (forced QR outcome) waits on the owner (D-AU2) |
| Duels | `.duel_status` `.duel_end` | ✅ Yes. `.duel_status [name]` shows a player's duel or duel challenge (yours with no name): the other player, the duel number, the stage and the seconds left. `.duel_end <name>` ends that duel or challenge in any stage, sends both players "Duel aborted", and starts no challenge cooldown. For a second duelist when testing alone, see `sparbot` in [the wireclient doc](architecture/wireclient.md#sparbot-a-duel-partner-for-solo-testing) |
| Bank | `.bank` `.bankdump` `.bankexpand` `.orgvaultexpand` | ✅ Yes. `.bank` opens your own personal vault wherever you stand, with no Banker; any later right-click on an NPC closes the session. A player without GM access who types `.bank` is told it needs GM access, and the line is not said aloud. `.bankdump [name]` lists a character's personal vault (container 17) in chat, read-only: one line per item with its slot, name, type id, stack size and item id, and the vault size (`bank_slots`). With no name it lists your own. The name is matched exactly (case-sensitive) against every character, so an offline character works too. A row in a slot past the vault size is flagged `(beyond bank_slots)`, because the client's vault window cannot show it. `.bankexpand` buys one +10 expansion of your own vault at the seeded price (`resources.bank_expansion_price`, 100 naquadah per step), through the same purchase the Banker's Expand dialog uses: it needs your vault open (a `.bank` session, which skips the Banker distance, or a Banker in range), charges your own naquadah, stops at 100 slots, and answers in chat either way. It is the only way to expand a vault while the Banker's dialog is held back (#943). A player without GM access is told it needs GM access, and the line is not said aloud. `.orgvaultexpand [team\|command] [from_slots]` expands your Team vault by +10 at the same seeded price, paid from the Team treasury, and only for the Team's leader: with no size it tells you the current size, the price and the treasury, and `.orgvaultexpand 40` (the current size) buys the step, so repeating it buys nothing twice. It stops at 100 slots; a Command vault is fixed at 100 and is refused. A player without GM access is told it needs GM access |
| Mail | `.mail` `.mailbox` `.mail_expire` | ✅ Yes. `.mail [to <name>] [cash <n>] [item <typeId> [qty]] [cod <n>] [<subject>]` sends yourself (or `<name>`, online or not) a mail with minted cash and a minted item, no postage; the options come first in any order, the first other word starts the subject, and a number right after the type id is the quantity. With `cod <n>` the mail is a COD mail from your character, so the payment comes back to you; it needs an item and no cash. `.mailbox [name]` shows a mailbox's open and archived mail, its system and quarantined mail, what is in escrow and the next expiry. `.mail_expire <mailId>` makes a mail due now and expires it at once as the sweep would (returned, deleted or quarantined) and says which; archived and quarantined mail are refused |
| Black Market | `.bm_seed` `.bm_list` `.bm_expire` | ✅ Yes (server side; no legacy counterpart, BM-07). `.bm_seed [count]` lists `count` auctions (default 8, at most 60) from the system seller "Black Market", cycling through a test set (the SI 3 9mm Pistol at tech competency 1 to 20, Health Slappacks, two SMGs; every duration tier, bid-only and buyout listings) and names their ids; it refuses, listing nothing, if the reserved system seller ids hold another account or character. `.bm_list` shows the 10 newest active auctions with their ids, seller, price and time left (the client window shows no ids). `.bm_expire <auctionId>` makes an active auction due now and settles it at once as the sweep would, then says how: sold (to whom, for how much), returned to its seller, or a system listing that simply ends; a sold, cancelled or expired auction is refused. The window itself needs the client patch; see [the Black Market UAT checklist](analysis/black-market/uat.md) |
| Squads | `.squad_invite <name>` `.squad_join <name>` `.squad_info [name]` | ✅ Yes (server side) — `.squad_invite` is `/squadinvite` from you. `.squad_join` puts you straight into that player's squad with no invite, founding one they lead if they have none (you must not be in a squad). `.squad_info` lists your squad, or the named player's: id, size, loot mode, and each member's rank, level and entity. Squads are never saved |
| Teams and Commands | `.org_disband <orgId>` | ✅ Yes (server side). Disbands a Team or Command, memberless ones included: every online member is told it was disbanded, and you get a line with the member count. Refused while the organization's vault holds anything. The base checks your GM access itself |
| Teams and Commands | `.org_join <orgId> [player]` | ✅ Yes (saved). Adds an online player (default: you) to a Team or Command at its entry rank (Team Member, Command Initiate), or as Leader when it has no members. Skips the invite and permission checks, but still refuses a player already in an organization of that type. The player gets the organization window and the members see them join. The base checks your GM access itself |
| Teams and Commands | `.org_rank <player> <rank> [orgId]` | ✅ Yes (saved). Sets a member's rank (online or not), skipping the rank-authority checks; refuses Leader (8) and ranks the type does not use (a Team uses 2, 3 and 8). Without `orgId` it acts on the member's only Team or Command, and asks for the id when they are in both. Online members see the new rank |
| Teams and Commands | `.org_info [player]` | ✅ Yes (read only). Lists every Team and Command a character (default: you, online or not) belongs to, with the org id, rank and that rank's permission mask in hex. The base checks your GM access itself |
| Teams and Commands | `.org_list` | ✅ Yes (read only). Lists every Team and Command with its id, member count and leader, oldest first, at most 50 lines |
| Teams and Commands | `.org_set_perms <orgId> <rank> <mask>` | ✅ Yes (saved). Sets a rank's permission mask (decimal or `0x` hex). Only the bits the type's rank editor shows take your value (12 for a Team, 14 for a Command); the others keep what is stored, and the reply names the bits it ignored. Refuses the Leader rank (it always holds every bit), ranks the type does not use, and an edit that changes nothing. Online members get the new rank table |
| Organizations | `.org_create <team\|command> <name>` | ✅ Yes (saved) — founds a Team or Command that you lead, without the Organization Registrar. The name is every word after the type. The base re-checks your GM access and applies the same rules as the registrar: the name rules, one Team and one Command per character, and the creation cost (0 for now). You get the organization window and a confirmation line, or the reason it was refused |
| Travel | `.gotoxyz` `.goto` `.summon` `.gotolocation` `.gotospace` | ✅ Yes — world names match case-insensitively; `.gotospace` takes a loaded space id so it needs no world name at all. Coordinates are optional on both: `.gotolocation <world>` and `.gotospace <spaceId>` land on the world's entry point — the new-character start for `Castle_CellBlock` / `SGC_W1`, else the story ring pad (Castle: the Armory drop zone mission 688's ring ceremony lands on), else the stargate arrival, else the world's first authored respawner — and the feedback line says which one it used. `.summon <name>` always brings the player to **your** instance and current position; whatever you have selected is ignored (a deliberate departure from the original `target or player` rule) |

A few commands (`.debug_controller`, the server/maint family) report
an honest limitation in-game where the Rust server handles the concern
differently from the legacy Python (incremental persistence, startup resource
loading, env-based log level). See the
[dev-console-channel ADR](architecture/dev-console-channel.md) for the full
design and the per-command status.

---

## At a glance

- **266 commands** total -- **105** for everyone, **161** Game-Master only.
- **67** fully work on our server, **23** are handled by the game itself, **23** partly work or work only server-side so far, and **153** aren't wired up on our server yet.
- **47** have an automated test guarding the server behavior.

> The server side is tested where marked, but a full live-client pass (typing each one in the real game and watching the result) is still pending. Treat ✅ as "the server does the right thing when the command arrives."
>
> **Accuracy caveat (2026-07-25):** the ✅/🚧/❌ marks in the `/`-command tables above have **not** been audited handler-by-handler. Two were checked and both were wrong — `/duelforfeit` and `/duelresponse` were marked ✅ but are `UNIMPLEMENTED` stubs (see the Dueling section); the totals here reflect that correction. Expect other rows to be optimistic in the same way, because a command that reaches a dispatch arm can still do nothing. The `.`-console section below **was** verified in full against [console/registry.rs](../crates/cell-console/src/cell/console/registry.rs).

## See also

- [Chat system](gameplay/chat-system.md) -- channels, whispers, emotes.
- [Pet system](gameplay/pet-system.md) -- pet commands and abilities.
- [Service architecture](architecture/service-architecture.md) -- how console commands reach the server.
