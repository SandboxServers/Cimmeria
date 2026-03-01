# Commands Reference

All 256 slash commands available in Stargate Worlds. Player commands work for everyone; GM/Debug commands require elevated permissions.

## Player Commands

### Movement

| Command | What It Does |
|---------|-------------|
| `/run` | Switch to running |
| `/walk` | Switch to walking |
| `/location` | Show your current position |
| `/unstuck` | Attempt to unstick your character |
| `/exit` | Exit the game |

### Chat & Communication

| Command | What It Does |
|---------|-------------|
| `/say` | Say something in local chat |
| `/yell` | Yell (wider range than say) |
| `/tell <player>` | Send a private message |
| `/emote` | Perform an emote |
| `/saysquad` | Talk in squad chat |
| `/sayteam` | Talk in team chat |
| `/saycommand` | Talk in command/guild chat |
| `/sayofficer` | Talk in officer chat |
| `/chatjoin` | Join a chat channel |
| `/chatleave` | Leave a chat channel |
| `/chatlist` | List available channels |
| `/chatsetafkmessage` | Set your AFK auto-reply |
| `/chatsetdndmessage` | Set your Do Not Disturb auto-reply |
| `/chatignore` | Ignore a player |
| `/chatmute` | Mute a player |
| `/chatunmute` | Unmute a player |

### Squad (Small Group)

| Command | What It Does |
|---------|-------------|
| `/squadinvite` | Invite a player to your squad |
| `/squadinviteaccept` | Accept a squad invite |
| `/squadinvitedecline` | Decline a squad invite |
| `/squadkick` | Kick a squad member |
| `/squadleave` | Leave your squad |
| `/squadpromote` | Promote a squad member to leader |

### Team (Mid-Size Group)

| Command | What It Does |
|---------|-------------|
| `/teaminvite` | Invite to team |
| `/teaminviteaccept` | Accept team invite |
| `/teaminvitedecline` | Decline team invite |
| `/teamkick` | Kick from team |
| `/teamleave` | Leave team |
| `/teampromote` | Promote in team |
| `/teamdemote` | Demote in team |
| `/teammotd` | Set team message of the day |

### Command (Guild)

| Command | What It Does |
|---------|-------------|
| `/commandinvite` | Invite to guild |
| `/commandinviteaccept` | Accept guild invite |
| `/commandinvitedecline` | Decline guild invite |
| `/commandkick` | Kick from guild |
| `/commandleave` | Leave guild |
| `/commandpromote` | Promote in guild |
| `/commanddemote` | Demote in guild |
| `/commandmotd` | Set guild MOTD |
| `/chooseorgname` | Choose organization name |

### Combat & Abilities

| Command | What It Does |
|---------|-------------|
| `/invokeability` | Use an ability |
| `/activatebandolierslot` | Switch equipment loadout |
| `/changeammo` | Switch ammo type |
| `/trainability` | Learn a new ability |
| `/resetabilities` | Reset all abilities |
| `/respec` | Full ability respec |

### Pets

| Command | What It Does |
|---------|-------------|
| `/petinvokeability` | Command pet to use an ability |
| `/petinvokecommand` | Give pet a command |
| `/petabilitytoggle` | Toggle pet auto-ability |
| `/petchangestance` | Change pet stance |

### Items & Inventory

| Command | What It Does |
|---------|-------------|
| `/equipitem` | Equip an item |
| `/unequipitem` | Unequip an item |
| `/useitem` | Use an item |
| `/deleteitem` | Delete an item |
| `/moveitem` | Move an item between containers |
| `/purchaseitem` | Buy from a vendor |
| `/repairitem` | Repair an item |
| `/rechargeitem` | Recharge an item |

### Missions

| Command | What It Does |
|---------|-------------|
| `/missionassign` | Accept a mission |
| `/missionabandon` | Abandon a mission |
| `/missiondetails` | View mission details |
| `/missionlist` | List your missions |
| `/sharemission` | Share a mission with your team |
| `/sharemissionaccept` | Accept a shared mission |
| `/sharemissiondecline` | Decline a shared mission |

### Stargate

| Command | What It Does |
|---------|-------------|
| `/dhd` | Interact with a Dial Home Device |
| `/setringtransporterdestination` | Set ring transporter destination |

### Dueling

| Command | What It Does |
|---------|-------------|
| `/duelchallenge` | Challenge another player to a duel |
| `/duelresponse` | Accept or decline a duel challenge |
| `/duelforfeit` | Forfeit an active duel |

### Crafting

| Command | What It Does |
|---------|-------------|
| `/craft` | Create an item from a blueprint |
| `/alloy` | Combine crafting materials |
| `/research` | Research a new recipe |
| `/reverseengineer` | Deconstruct an item for knowledge |
| `/respeccraft` | Respec crafting skills |

### Other

| Command | What It Does |
|---------|-------------|
| `/who` | List online players |
| `/users` | Show user count |
| `/help` | Show command help |
| `/helpfull` | Show all available commands |
| `/petition` | File a support petition |
| `/logoff` | Log off |
| `/respawn` | Respawn after death |

## GM Commands

These commands require Game Master permissions.

### Teleportation

| Command | What It Does |
|---------|-------------|
| `/goto` | Teleport to a player |
| `/gotolocation` | Teleport to a named location |
| `/gotoxyz` | Teleport to specific coordinates |
| `/summon` | Teleport a player to you |

### Character Modification

| Command | What It Does |
|---------|-------------|
| `/setlevel` | Set a character's level |
| `/sethealth` | Set current health |
| `/sethealthmax` | Set maximum health |
| `/setfocus` | Set current focus |
| `/setspeed` | Set movement speed |
| `/setgodmode` | Toggle invincibility |
| `/setinvulnerable` | Toggle damage immunity |
| `/setinfiniteammo` | Toggle unlimited ammo |
| `/setnoaggro` | Toggle NPC aggro |
| `/setnotarget` | Toggle untargetable |
| `/invisible` | Toggle invisibility |
| `/sethidegm` | Toggle GM visibility |
| `/setarchetype` | Change class/archetype |
| `/setfaction` | Change faction |
| `/setpvp` | Toggle PvP flag |
| `/setfly` | Toggle flight |
| `/setghost` | Toggle ghost mode (walk through walls) |
| `/setspectator` | Toggle spectator mode |

### Giving Things

| Command | What It Does |
|---------|-------------|
| `/giveability` | Give a specific ability |
| `/giveallabilities` | Give every ability |
| `/giveitem` | Give an item |
| `/givegearset` | Give a full gear set |
| `/givexp` | Give experience points |
| `/givetrainingpoints` | Give training points |
| `/givenaqahdah` | Give naqahdah (crafting currency) |
| `/givestargateaddress` | Give a stargate address |
| `/giveblueprint` | Give a crafting blueprint |
| `/giveammo` | Give ammunition |

### Entity Control

| Command | What It Does |
|---------|-------------|
| `/spawn` | Spawn an NPC/entity |
| `/despawn` | Remove an NPC/entity |
| `/kill` | Kill target entity |
| `/interact` | Force an NPC interaction |

### Mission GM Commands

| Command | What It Does |
|---------|-------------|
| `/missionadvance` | Advance a mission step |
| `/missioncomplete` | Complete a mission |
| `/missionreset` | Reset a mission |
| `/missionclear` | Clear a mission |
| `/missionclearactive` | Clear all active missions |
| `/missionclearhistory` | Clear mission history |
| `/missionsetavailable` | Make a mission available |

## Debug Commands

These are developer/debug tools for investigating game behavior.

### Visual Debug

| Command | What It Does |
|---------|-------------|
| `/showfps` | Show frames per second |
| `/showmemory` | Show memory usage |
| `/showlog` | Show log output |
| `/showcover` | Visualize cover links |
| `/shownavmesh` | Show navigation mesh |
| `/showspawns` | Show spawn points |
| `/showmobpaths` | Show NPC patrol paths |
| `/showwaypoints` | Show waypoints |
| `/showtriggers` | Show trigger volumes |
| `/showposition` | Show exact position |
| `/showrotation` | Show rotation |

### Combat Debug

| Command | What It Does |
|---------|-------------|
| `/combatdebug` | Toggle combat debug info |
| `/combatdebugverbose` | Toggle verbose combat debug |
| `/abilitydebug` | Toggle ability debug info |
| `/healdebug` | Toggle healing debug info |
| `/testlos` | Test line of sight to target |
| `/togglecombatlos` | Toggle combat line of sight |

### NPC/AI Debug

| Command | What It Does |
|---------|-------------|
| `/mobdata` | Dump all data for target NPC |
| `/getmobattribute` | Get a specific NPC stat |
| `/setmobattribute` | Set a specific NPC stat |
| `/debugbehaviorsonmob` | Show NPC behavior tree |
| `/debugpathsonmob` | Show NPC pathfinding |

### Hot-Loading

These commands reload data without restarting the server:

| Command | What It Does |
|---------|-------------|
| `/loadability` | Reload an ability definition |
| `/loaditem` | Reload an item definition |
| `/loadmission` | Reload a mission |
| `/loadmob` | Reload an NPC definition |
| `/loadbehavior` | Reload an AI behavior |
| `/loadconstants` | Reload game constants |
| `/reload` | General data reload |

### Misc Debug

| Command | What It Does |
|---------|-------------|
| `/forceclientcrash` | Intentionally crash the client (testing) |
| `/timeofday` | Set the in-game time of day |
| `/worldinstancereset` | Reset a world instance |
| `/xrayeyes` | See through walls |
| `/physics` | Toggle physics debug visualization |
