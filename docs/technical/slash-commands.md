# Slash Commands

Complete catalog of 256 slash commands extracted from sgw.exe RTTI (`Event_SlashCmd_*` pattern). These are player-typed commands that the client processes locally and/or sends to the server.

## Player Commands

### Movement & Position
| Command | Description |
|---------|-------------|
| `/run` | Switch to running |
| `/walk` | Switch to walking |
| `/location` | Show current position |
| `/unstuck` | Attempt to unstick character |
| `/exit` | Exit game |

### Combat
| Command | Description |
|---------|-------------|
| `/invokeability` | Use an ability |
| `/activatebandolierslot` | Switch equipment loadout |
| `/changeammo` | Switch ammo type |
| `/confirmeffect` | Confirm combat effect |

### Communication
| Command | Description |
|---------|-------------|
| `/say` | Say in local chat |
| `/yell` | Yell (wider range) |
| `/tell` | Private message |
| `/ntell` | Named tell |
| `/saychannel` | Say in specific channel |
| `/saycommand` | Command channel |
| `/sayofficer` | Officer channel |
| `/sayplatoon` | Platoon channel |
| `/saysquad` | Squad channel |
| `/sayteam` | Team channel |
| `/emote` | Perform emote |

### Chat Management
| Command | Description |
|---------|-------------|
| `/chatjoin` | Join chat channel |
| `/chatleave` | Leave chat channel |
| `/chatlist` | List chat channels |
| `/chatignore` | Ignore player |
| `/chatfriend` | Add friend |
| `/chatunfriend` | Remove friend |
| `/chatmute` | Mute player |
| `/chatunmute` | Unmute player |
| `/chatkick` | Kick from channel |
| `/chatop` | Grant channel operator |
| `/chatban` | Ban from channel |
| `/chatunban` | Unban from channel |
| `/chatpassword` | Set channel password |
| `/chatsetafkmessage` | Set AFK message |
| `/chatsetdndmessage` | Set DND message |

### Squad
| Command | Description |
|---------|-------------|
| `/squadinvite` | Invite to squad |
| `/squadinviteaccept` | Accept squad invite |
| `/squadinvitedecline` | Decline squad invite |
| `/squadkick` | Kick from squad |
| `/squadleave` | Leave squad |
| `/squadpromote` | Promote squad member |

### Team
| Command | Description |
|---------|-------------|
| `/teaminvite` | Invite to team |
| `/teaminviteaccept` | Accept team invite |
| `/teaminvitedecline` | Decline team invite |
| `/teamkick` | Kick from team |
| `/teamleave` | Leave team |
| `/teampromote` | Promote team member |
| `/teamdemote` | Demote team member |
| `/teammotd` | Set team MOTD |
| `/setteamnote` | Set team note |
| `/setteamofficernote` | Set officer note |

### Command (Organization)
| Command | Description |
|---------|-------------|
| `/commandinvite` | Invite to command |
| `/commandinviteaccept` | Accept command invite |
| `/commandinvitedecline` | Decline command invite |
| `/commandkick` | Kick from command |
| `/commandleave` | Leave command |
| `/commandpromote` | Promote in command |
| `/commanddemote` | Demote in command |
| `/commandmotd` | Set command MOTD |
| `/chooseorgname` | Choose org name |
| `/setcommandnote` | Set command note |
| `/setcommandofficernote` | Set officer note |

### Inventory & Items
| Command | Description |
|---------|-------------|
| `/equipitem` | Equip item |
| `/unequipitem` | Unequip item |
| `/useitem` | Use item |
| `/deleteitem` | Delete item |
| `/moveitem` | Move item |
| `/lootitem` | Loot item |
| `/purchaseitem` | Purchase item |
| `/repairitem` | Repair item |
| `/rechargeitem` | Recharge item |

### Missions
| Command | Description |
|---------|-------------|
| `/missionassign` | Accept mission |
| `/missionabandon` | Abandon mission |
| `/abandonmission` | Abandon mission (alt) |
| `/missionadvance` | Advance step |
| `/missioncomplete` | Complete mission |
| `/missionreset` | Reset mission |
| `/missionclear` | Clear mission |
| `/missionclearactive` | Clear active missions |
| `/missionclearhistory` | Clear history |
| `/missiondetails` | Show mission details |
| `/missionlist` | List missions |
| `/missionlistfull` | Full mission list |
| `/missionsetavailable` | Set available |
| `/sharemission` | Share with team |
| `/sharemissionaccept` | Accept shared |
| `/sharemissiondecline` | Decline shared |

### Dueling
| Command | Description |
|---------|-------------|
| `/duelchallenge` | Challenge to duel |
| `/duelresponse` | Respond to challenge |
| `/duelforfeit` | Forfeit duel |

### Stargate
| Command | Description |
|---------|-------------|
| `/givestargateaddress` | GM: give gate address |
| `/removestargateaddress` | GM: remove gate address |
| `/dhd` | Interact with DHD |
| `/setringtransporterdestination` | Set ring destination |

### Pets
| Command | Description |
|---------|-------------|
| `/petinvokeability` | Pet use ability |
| `/petinvokecommand` | Pet execute command |
| `/petabilitytoggle` | Toggle pet ability |

### Training
| Command | Description |
|---------|-------------|
| `/trainability` | Train an ability |
| `/resetabilities` | Reset all abilities |
| `/respec` | Full respec |
| `/respecability` | Respec single ability |
| `/respeccraft` | Respec crafting |

### Minigames
| Command | Description |
|---------|-------------|
| `/startminigame` | Start minigame |
| `/minigamecomplete` | Complete minigame |
| `/debugjoinminigame` | Debug: join |
| `/debugstartminigame` | Debug: start |
| `/debugspectatemininame` | Debug: spectate |
| `/debugminigameinstance` | Debug: instance |

### Space Queue
| Command | Description |
|---------|-------------|
| `/spacequeuedresponse` | Queue response |
| `/spacequeuereadyresponse` | Queue ready |
| `/spacequeuestatus` | Queue status |

## GM / Debug Commands

### Entity Control
| Command | Description |
|---------|-------------|
| `/kill` | Kill target |
| `/spawn` | Spawn entity |
| `/despawn` | Despawn entity |
| `/summon` | Summon player |
| `/interact` | Force interact |
| `/dialogbuttonchoice` | Force dialog choice |

### Character Modification
| Command | Description |
|---------|-------------|
| `/setlevel` | Set level |
| `/sethealth` | Set health |
| `/sethealthmax` | Set max health |
| `/setfocus` | Set focus |
| `/setfocusmax` | Set max focus |
| `/setspeed` | Set movement speed |
| `/setgodmode` | Toggle god mode |
| `/setinvulnerable` | Toggle invulnerable |
| `/setinfiniteammo` | Toggle infinite ammo |
| `/setignorehealth` | Ignore health |
| `/setignorefocus` | Ignore focus |
| `/setnoaggro` | Toggle no aggro |
| `/setnoxp` / `/setnoxp` (x2) | Toggle no XP |
| `/setnodamagetimemode` | No damage timed mode |
| `/setnotarget` | Toggle no target |
| `/setomnipotent` | Toggle omnipotent |
| `/setpvp` | Toggle PvP |
| `/setspectator` | Toggle spectator |
| `/invisible` | Toggle invisible |
| `/sethidegm` | Toggle GM hidden |
| `/setarchetype` | Set class/archetype |
| `/setfaction` | Set faction |
| `/settechskill` | Set tech skill |

### Teleportation
| Command | Description |
|---------|-------------|
| `/goto` | Teleport to player |
| `/gotolocation` | Teleport to named loc |
| `/gotoxyz` | Teleport to coordinates |

### Giving Items/Resources
| Command | Description |
|---------|-------------|
| `/giveability` | Give ability |
| `/giveallabilities` | Give all abilities |
| `/giveammo` | Give ammo |
| `/giveitem` | Give item |
| `/giveinventory` | Give inventory set |
| `/givegearset` | Give gear set |
| `/giveblueprint` | Give blueprint |
| `/givexp` | Give XP |
| `/giveexpertise` | Give expertise |
| `/givefaction` | Give faction rep |
| `/givetrainingpoints` | Give training points |
| `/givenaqahdah` | Give naqahdah |
| `/giveappliedsciencepoints` | Give science points |
| `/giveracialparadigmlevels` | Give racial paradigm |
| `/giverespawner` | Give respawner |
| `/giveminigamecontact` | Give minigame contact |
| `/removeminigamecontact` | Remove minigame contact |

### MOB/AI Debugging
| Command | Description |
|---------|-------------|
| `/mobdata` | Dump mob data |
| `/getmobattribute` | Get mob attribute |
| `/setmobattribute` | Set mob attribute |
| `/setmobvariable` | Set mob variable |
| `/setmobabilityset` | Set mob ability set |
| `/setmobstance` | Set mob stance |
| `/debugabilityonmob` | Debug ability on mob |
| `/debugbehaviorsonmob` | Debug behaviors |
| `/debugpathsonmob` | Debug pathing |
| `/emitbehavioreventonmob` | Emit behavior event |
| `/addbehavioreventset` | Add behavior event set |
| `/removebehavioreventset` | Remove behavior set |
| `/entererroraistate` | Force AI error |
| `/exiterroraistate` | Exit AI error |

### Data Hot-Loading
| Command | Description |
|---------|-------------|
| `/loadability` | Reload ability data |
| `/loadabilityset` | Reload ability set |
| `/loadbehavior` | Reload behavior |
| `/loadconstants` | Reload constants |
| `/loadinteractionset` | Reload interactions |
| `/loaditem` | Reload item data |
| `/loadmob` | Reload MOB definition |
| `/loadmission` | Reload mission |
| `/loadnacsi` | Reload NACSI data |
| `/reload` | General reload |

### Display/Debug
| Command | Description |
|---------|-------------|
| `/showfps` | Show FPS |
| `/showmemory` | Show memory usage |
| `/showlog` | Show log output |
| `/showarea` | Show area info |
| `/showcover` | Show cover links |
| `/showregion` | Show region info |
| `/showspawns` | Show spawn points |
| `/showspawnset` | Show spawn set |
| `/showmobpaths` | Show mob paths |
| `/shownavmesh` | Show nav mesh |
| `/showwaypoints` | Show waypoints |
| `/showcommandwaypoints` | Show command waypoints |
| `/showtriggers` | Show triggers |
| `/showposition` | Show position |
| `/showrotation` | Show rotation |
| `/showtargetlocation` | Show target location |
| `/showmobcount` | Show mob count |
| `/showplayer` | Show player info |
| `/showip` | Show IP |
| `/showflag` | Show flag value |
| `/showinstanceflag` | Show instance flag |
| `/shownavigation` | Show navigation |
| `/showdialog` | Show dialog |
| `/showpointset` | Show point set |
| `/showinventory` | Show inventory |

### Misc Debug
| Command | Description |
|---------|-------------|
| `/combatdebug` | Toggle combat debug |
| `/combatdebugverbose` | Verbose combat debug |
| `/abilitydebug` | Ability debugging |
| `/healdebug` | Heal debugging |
| `/debugevents` | Debug events |
| `/debugflash` | Debug flash |
| `/debuginteract` | Debug interactions |
| `/debugperformance` | Performance debug |
| `/debugtarget` | Debug target |
| `/dumpobjects` | Dump object list |
| `/testlos` | Test line of sight |
| `/testsequence` | Test sequence |
| `/togglecombatlos` | Toggle combat LOS |
| `/perfstatsbychannel` | Per-channel perf |
| `/regeneratecoverlinks` | Regen cover links |
| `/timeofday` | Set time of day |
| `/setflag` | Set game flag |
| `/setinstanceflag` | Set instance flag |
| `/forceclientcrash` | Force client crash |
| `/forcerendertheadcrash` | Force render thread crash |
| `/physics` | Physics debug |
| `/printstats` | Print statistics |
| `/reloadinventory` | Reload inventory |
| `/reloadorganizations` | Reload organizations |
| `/worldinstancereset` | Reset world instance |
| `/xrayeyes` | X-ray vision |
| `/petition` | File petition |
| `/users` | List users |
| `/who` | Who's online |
| `/help` | Show command help |
| `/helpfull` | Full help listing |
| `/gmchat` | GM chat channel |
| `/gmshout` | GM broadcast |
| `/gmdeleteitem` | GM delete item |
| `/getiteminfo` | Get item info |
| `/listabilities` | List abilities |
| `/listinteractions` | List interactions |
| `/listitems` | List items |
| `/initialresponse` | Initial response |
| `/logoff` | Log off |
| `/respawn` | Respawn |
| `/setfly` | Toggle flying |
| `/setghost` | Toggle ghost mode |
| `/toggleautocycleability` | Auto-cycle ability |
