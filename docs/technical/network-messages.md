# Network Messages

Complete catalog of all client-server network messages extracted from sgw.exe RTTI and string data. These are the CME::EventSignal events that correspond to Mercury protocol messages.

## Event_NetIn (Server → Client): 167 Messages

Messages the server sends to the client. Extracted from `CME::EventSignal::TypedEmitInfo` RTTI templates.

### Login & Account
| Message | Category |
|---------|----------|
| `Event_NetIn_AccountLoginSuccess` | Login success |
| `Event_NetIn_LoginFailure` | Login failed |
| `Event_NetIn_ServerSelectSuccess` | Shard selected |
| `Event_NetIn_CharacterList` | Character list for account |
| `Event_NetIn_CharacterCreateFailed` | Character creation error |
| `Event_NetIn_CharacterVisuals` | Character visual data |
| `Event_NetIn_onCharacterLoadFailed` | Character load error |
| `Event_NetIn_onClientChallenge` | Anti-cheat challenge |
| `Event_NetIn_onClientMapLoad` | Map load trigger |
| `Event_NetIn_onPlayerDataLoaded` | Player data ready |
| `Event_NetIn_onVersionInfo` | Protocol version info |
| `Event_NetIn_onCookedDataError` | Cooked data error |
| `Event_NetIn_onErrorCode` | Generic error code |
| `Event_NetIn_onOverridePerfStatsRate` | Performance stats override |

### World & Entity
| Message | Category |
|---------|----------|
| `Event_NetIn_setupWorldParameters` | World setup on login |
| `Event_NetIn_onTimeofDay` | Time of day sync |
| `Event_NetIn_MapInfo` | Map/zone info |
| `Event_NetIn_ResetMapInfo` | Map info reset |
| `Event_NetIn_onEntityMove` | Entity position update |
| `Event_NetIn_onVisible` | Entity enters visibility |
| `Event_NetIn_EntityFlags` | Entity state flags |
| `Event_NetIn_EntityProperty` | Entity property change |
| `Event_NetIn_EntityTint` | Entity color tint |
| `Event_NetIn_onRemoteEntityCreate` | Remote entity spawned |
| `Event_NetIn_onRemoteEntityMove` | Remote entity moved |
| `Event_NetIn_onRemoteEntityRemove` | Remote entity despawned |
| `Event_NetIn_onStaticMeshNameUpdate` | Static mesh change |

### Being / Character
| Message | Category |
|---------|----------|
| `Event_NetIn_BeingAppearance` | Full appearance data |
| `Event_NetIn_onBeingNameUpdate` | Name change |
| `Event_NetIn_onBeingNameIDUpdate` | Name ID change |
| `Event_NetIn_onExtraNameUpdate` | Extra name (title, etc.) |
| `Event_NetIn_onNickChanged` | Nickname change |
| `Event_NetIn_onArchetypeUpdate` | Archetype/class change |
| `Event_NetIn_onAlignmentUpdate` | Faction alignment |
| `Event_NetIn_onFactionUpdate` | Faction standing |
| `Event_NetIn_onSetTarget` | Target assignment |
| `Event_NetIn_onTargetUpdate` | Target state update |

### Stats & Progression
| Message | Category |
|---------|----------|
| `Event_NetIn_onLevelUpdate` | Level change |
| `Event_NetIn_onExpUpdate` | XP gained |
| `Event_NetIn_onMaxExpUpdate` | Max XP for level |
| `Event_NetIn_onStatUpdate` | Stat change |
| `Event_NetIn_onStatBaseUpdate` | Base stat change |
| `Event_NetIn_onStateFieldUpdate` | State field change |
| `Event_NetIn_onActiveSlotUpdate` | Active equipment slot |
| `Event_NetIn_onUpdateRacialParadigmLevel` | Racial paradigm |

### Combat
| Message | Category |
|---------|----------|
| `Event_NetIn_onEffectResults` | Ability/combat results |
| `Event_NetIn_onSequence` | Combat sequence |
| `Event_NetIn_onLOSResult` | Line of sight check result |
| `Event_NetIn_onMeleeRangeUpdate` | Melee range change |
| `Event_NetIn_onAggressionOverrideUpdate` | Aggression override set |
| `Event_NetIn_onAggressionOverrideCleared` | Aggression override cleared |
| `Event_NetIn_onThreatenedMobsUpdate` | Threat list update |
| `Event_NetIn_EffectUserData` | Effect user data |
| `Event_NetIn_onBeginAidWait` | Aid/help waiting |
| `Event_NetIn_onEndAidWait` | Aid/help ended |

### Abilities & Pets
| Message | Category |
|---------|----------|
| `Event_NetIn_KnownAbilitiesUpdate` | Ability list update |
| `Event_NetIn_AbilityTreeInfo` | Ability tree data |
| `Event_NetIn_PetAbilities` | Pet ability list |
| `Event_NetIn_PetStances` | Pet stance list |
| `Event_NetIn_PetStanceUpdate` | Pet stance change |

### Inventory & Items
| Message | Category |
|---------|----------|
| `Event_NetIn_onContainerInfo` | Container contents |
| `Event_NetIn_onUpdateItem` | Item updated |
| `Event_NetIn_onRemoveItem` | Item removed |
| `Event_NetIn_onRefreshItem` | Item refreshed |
| `Event_NetIn_CashChanged` | Currency change |
| `Event_NetIn_LootDisplay` | Loot window |

### Store & Trading
| Message | Category |
|---------|----------|
| `Event_NetIn_onStoreOpen` | Store opened |
| `Event_NetIn_onStoreClose` | Store closed |
| `Event_NetIn_onStoreUpdate` | Store inventory change |
| `Event_NetIn_TradeState` | Trade state change |
| `Event_NetIn_TradeResults` | Trade completed |

### Vault
| Message | Category |
|---------|----------|
| `Event_NetIn_onVaultOpen` | Personal vault opened |
| `Event_NetIn_onTeamVaultOpen` | Team vault opened |
| `Event_NetIn_onCommandVaultOpen` | Command vault opened |
| `Event_NetIn_onClearOrgVaultInventory` | Org vault cleared |

### Missions
| Message | Category |
|---------|----------|
| `Event_NetIn_onMissionUpdate` | Mission state change |
| `Event_NetIn_onStepUpdate` | Mission step update |
| `Event_NetIn_onObjectiveUpdate` | Objective update |
| `Event_NetIn_onTaskUpdate` | Task update |
| `Event_NetIn_MissionOffer` | Mission offered to player |
| `Event_NetIn_MissionSharedOffer` | Shared mission offer |
| `Event_NetIn_MissionRewards` | Reward selection |

### Chat & Communication
| Message | Category |
|---------|----------|
| `Event_NetIn_onChatJoined` | Joined chat channel |
| `Event_NetIn_onChatLeft` | Left chat channel |
| `Event_NetIn_onTellSent` | Private message sent |
| `Event_NetIn_onPlayerCommunication` | Player message |
| `Event_NetIn_onLocalizedCommunication` | Localized system message |
| `Event_NetIn_onSystemCommunication` | System message |
| `Event_NetIn_onSquadLootType` | Squad loot mode |

### Social / Organizations
| Message | Category |
|---------|----------|
| `Event_NetIn_onOrganizationInvite` | Org invite received |
| `Event_NetIn_onOrganizationJoined` | Joined organization |
| `Event_NetIn_onOrganizationLeft` | Left organization |
| `Event_NetIn_onOrganizationRosterInfo` | Org roster data |
| `Event_NetIn_onOrganizationNameUpdate` | Org name changed |
| `Event_NetIn_onOrganizationMOTDUpdate` | Org MOTD changed |
| `Event_NetIn_onOrganizationRankUpdate` | Rank structure change |
| `Event_NetIn_onOrganizationRankNameUpdate` | Rank name change |
| `Event_NetIn_onOrganizationNoteUpdate` | Org note change |
| `Event_NetIn_onOrganizationOfficerNoteUpdate` | Officer note change |
| `Event_NetIn_onOrganizationCashUpdate` | Org bank update |
| `Event_NetIn_onOrganizationExperienceUpdate` | Org XP update |
| `Event_NetIn_onMemberJoinedOrganization` | Member joined |
| `Event_NetIn_onMemberLeftOrganization` | Member left |
| `Event_NetIn_onMemberRankChangedOrganization` | Member rank changed |
| `Event_NetIn_onLaunchOrganizationCreation` | Org creation dialog |
| `Event_NetIn_PvPOrganizationLeaveRequest` | PvP org leave request |
| `Event_NetIn_onStrikeTeamUpdate` | Strike team update |

### Contact Lists
| Message | Category |
|---------|----------|
| `Event_NetIn_onContactListUpdate` | Contact list changed |
| `Event_NetIn_onContactListAddMembers` | Members added |
| `Event_NetIn_onContactListRemoveMembers` | Members removed |
| `Event_NetIn_onContactListDelete` | List deleted |
| `Event_NetIn_onContactListEvent` | Contact event |

### Mail
| Message | Category |
|---------|----------|
| `Event_NetIn_onMailHeaderInfo` | Mail headers |
| `Event_NetIn_onMailHeaderRemove` | Mail removed |
| `Event_NetIn_onMailRead` | Mail body |
| `Event_NetIn_onSendMailResult` | Send result |

### Stargate System
| Message | Category |
|---------|----------|
| `Event_NetIn_setupStargateInfo` | Initial gate data |
| `Event_NetIn_updateStargateAddress` | Gate address update |
| `Event_NetIn_StargateRotationOverride` | Gate rotation anim |
| `Event_NetIn_StargateTriggerFailed` | Gate activation failed |
| `Event_NetIn_onStargatePassage` | Gate travel event |

### Disciplines & Crafting
| Message | Category |
|---------|----------|
| `Event_NetIn_onUpdateDiscipline` | Discipline update |
| `Event_NetIn_onDisciplineRespec` | Discipline respec |
| `Event_NetIn_onUpdateCraftingOptions` | Crafting options |
| `Event_NetIn_onUpdateKnownCrafts` | Known recipes |
| `Event_NetIn_onCraftingRespecPrompt` | Crafting respec prompt |

### Black Market (Auction House)
| Message | Category |
|---------|----------|
| `Event_NetIn_BMAuctions` | Auction listings |
| `Event_NetIn_BMAuctionUpdate` | Auction updated |
| `Event_NetIn_BMAuctionRemove` | Auction removed |
| `Event_NetIn_BMError` | Black market error |
| `Event_NetIn_BMOpen` | Black market opened |

### Minigames
| Message | Category |
|---------|----------|
| `Event_NetIn_onStartMinigame` | Minigame started |
| `Event_NetIn_onEndMinigame` | Minigame ended |
| `Event_NetIn_onStartMinigameDialog` | Minigame dialog |
| `Event_NetIn_onStartMinigameDialogClose` | Dialog closed |
| `Event_NetIn_MinigameCallDisplay` | Call display |
| `Event_NetIn_MinigameCallResult` | Call result |
| `Event_NetIn_MinigameCallAbort` | Call aborted |
| `Event_NetIn_onMinigameRegistrationInfo` | Registration info |
| `Event_NetIn_onMinigameRegistrationPrompt` | Registration prompt |
| `Event_NetIn_AddOrUpdateMinigameHelper` | Helper added/updated |
| `Event_NetIn_RemoveMinigameHelper` | Helper removed |
| `Event_NetIn_ShowMinigameContact` | Contact shown |

### Dueling
| Message | Category |
|---------|----------|
| `Event_NetIn_onDuelChallenge` | Duel challenge received |
| `Event_NetIn_onDuelEntitiesSet` | Duel participants set |
| `Event_NetIn_onDuelEntitiesRemove` | Duel participants removed |
| `Event_NetIn_onDuelEntitiesClear` | Duel cleared |

### UI & Navigation
| Message | Category |
|---------|----------|
| `Event_NetIn_DialogDisplay` | Dialog window |
| `Event_NetIn_InitialInteraction` | Initial interact options |
| `Event_NetIn_InteractionType` | Interaction type |
| `Event_NetIn_ShowNavigation` | Navigation display |
| `Event_NetIn_onShowPath` | Path display |
| `Event_NetIn_onDisableShowPath` | Path hidden |
| `Event_NetIn_onShowCommandWaypoints` | Waypoints |
| `Event_NetIn_TimerUpdate` | Timer display |
| `Event_NetIn_onTrainerOpen` | Trainer opened |
| `Event_NetIn_onDisplayDHD` | DHD interface |
| `Event_NetIn_onDHDReply` | DHD response |
| `Event_NetIn_onRingTransporterList` | Ring transporter list |
| `Event_NetIn_onKismetEventSetUpdate` | Kismet event update |

### Media
| Message | Category |
|---------|----------|
| `Event_NetIn_PlayMovie` | Play cutscene |
| `Event_NetIn_StopMovie` | Stop cutscene |

### Misc
| Message | Category |
|---------|----------|
| `Event_NetIn_onSpaceQueueReady` | Space queue ready |
| `Event_NetIn_onSpaceQueued` | In space queue |
| `Event_NetIn_onSpectateList` | Spectator list |
| `Event_NetIn_AddClientHintedGenericRegion` | Client hint region |
| `Event_NetIn_ClearClientHintedGenericRegions` | Clear hint regions |

---

## Event_NetOut (Client → Server): 254 Messages

Messages the client sends to the server.

### Character Lifecycle
| Message | Purpose |
|---------|---------|
| `Event_NetOut_CreateCharacter` | Create new character |
| `Event_NetOut_DeleteCharacter` | Delete character |
| `Event_NetOut_PlayCharacter` | Enter world with character |
| `Event_NetOut_RequestCharacterVisuals` | Request appearance data |
| `Event_NetOut_Disconnect` | Disconnect from server |
| `Event_NetOut_LogOff` | Log off character |
| `Event_NetOut_ClientReady` | Client ready signal |
| `Event_NetOut_InitialResponse` | Initial response to server |
| `Event_NetOut_onClientVersion` | Client version report |
| `Event_NetOut_onClientChallengeResponse` | Anti-cheat response |
| `Event_NetOut_Respawn` | Respawn after death |

### Combat & Abilities
| Message | Purpose |
|---------|---------|
| `Event_NetOut_UseAbility` | Use an ability |
| `Event_NetOut_useAbilityOnGroundTarget` | AoE ability |
| `Event_NetOut_ConfirmEffect` | Confirm combat effect |
| `Event_NetOut_SetCrouched` | Toggle crouch |
| `Event_NetOut_SetTarget` | Set target |
| `Event_NetOut_SetTargetID` | Set target by ID |
| `Event_NetOut_TestLOS` | Test line of sight |
| `Event_NetOut_ToggleCombatLOS` | Toggle combat LOS |
| `Event_NetOut_SetMovementType` | Change movement type |
| `Event_NetOut_callForAid` | Call for aid |
| `Event_NetOut_ChangeWeaponState` | Change weapon |
| `Event_NetOut_RequestAmmoChange` | Change ammo type |
| `Event_NetOut_RequestReload` | Reload weapon |
| `Event_NetOut_SetAutoCycle` | Auto-cycle toggle |
| `Event_NetOut_RequestActiveSlotChange` | Change active slot |

### Pets
| Message | Purpose |
|---------|---------|
| `Event_NetOut_PetInvokeAbility` | Pet uses ability |
| `Event_NetOut_PetChangeStance` | Change pet stance |
| `Event_NetOut_PetAbilityToggle` | Toggle pet ability |

### Inventory & Items
| Message | Purpose |
|---------|---------|
| `Event_NetOut_UseItem` | Use an item |
| `Event_NetOut_MoveItem` | Move item in inventory |
| `Event_NetOut_RemoveItem` | Destroy item |
| `Event_NetOut_GMRemoveItem` | GM remove item |
| `Event_NetOut_LootItem` | Loot an item |
| `Event_NetOut_GetItemInfo` | Query item info |
| `Event_NetOut_requestItemData` | Request item data |
| `Event_NetOut_PurchaseItems` | Buy from store |
| `Event_NetOut_SellItems` | Sell to store |
| `Event_NetOut_BuybackItems` | Buyback items |
| `Event_NetOut_RepairItem` | Repair single item |
| `Event_NetOut_RepairItems` | Repair all items |
| `Event_NetOut_RechargeItem` | Recharge item |
| `Event_NetOut_RechargeItems` | Recharge all items |
| `Event_NetOut_ReloadInventory` | Reload inventory |
| `Event_NetOut_ShowInventory` | Show inventory (debug) |
| `Event_NetOut_ListItems` | List items (debug) |

### Missions
| Message | Purpose |
|---------|---------|
| `Event_NetOut_MissionAssign` | Accept mission |
| `Event_NetOut_MissionAbandon` | Abandon mission |
| `Event_NetOut_MissionAdvance` | Advance mission step |
| `Event_NetOut_MissionComplete` | Complete mission |
| `Event_NetOut_MissionReset` | Reset mission |
| `Event_NetOut_MissionClear` | Clear mission |
| `Event_NetOut_MissionClearActive` | Clear active missions |
| `Event_NetOut_MissionClearHistory` | Clear mission history |
| `Event_NetOut_MissionDetails` | Request mission details |
| `Event_NetOut_MissionList` | List missions |
| `Event_NetOut_MissionListFull` | List all missions |
| `Event_NetOut_MissionSetAvailable` | Set mission available |
| `Event_NetOut_ShareMission` | Share mission with team |
| `Event_NetOut_ShareMissionResponse` | Response to mission share |
| `Event_NetOut_ChosenRewards` | Choose mission rewards |
| `Event_NetOut_AbandonMission` | Abandon mission (alt) |

### Chat & Communication
| Message | Purpose |
|---------|---------|
| `Event_NetOut_sendPlayerCommunication` | Send chat/emote |
| `Event_NetOut_ChatJoin` | Join channel |
| `Event_NetOut_ChatLeave` | Leave channel |
| `Event_NetOut_ChatList` | List channels |
| `Event_NetOut_ChatIgnore` | Ignore player |
| `Event_NetOut_ChatFriend` | Add friend |
| `Event_NetOut_ChatMute` | Mute player |
| `Event_NetOut_ChatKick` | Kick from channel |
| `Event_NetOut_ChatOp` | Grant channel op |
| `Event_NetOut_ChatBan` | Ban from channel |
| `Event_NetOut_ChatPassword` | Set channel password |
| `Event_NetOut_ChatSetAFKMessage` | Set AFK message |
| `Event_NetOut_ChatSetDNDMessage` | Set DND message |
| `Event_NetOut_SendGMShout` | GM broadcast |

### Contact Lists
| Message | Purpose |
|---------|---------|
| `Event_NetOut_contactListCreate` | Create contact list |
| `Event_NetOut_contactListDelete` | Delete contact list |
| `Event_NetOut_contactListRename` | Rename contact list |
| `Event_NetOut_contactListAddMembers` | Add contacts |
| `Event_NetOut_contactListRemoveMembers` | Remove contacts |
| `Event_NetOut_contactListFlagsUpdate` | Update list flags |

### Organizations
| Message | Purpose |
|---------|---------|
| `Event_NetOut_OrganizationCreation` | Create organization |
| `Event_NetOut_OrganizationInvite` | Invite to org |
| `Event_NetOut_OrganizationInviteByType` | Invite by type |
| `Event_NetOut_OrganizationInviteResponse` | Respond to invite |
| `Event_NetOut_OrganizationLeave` | Leave org |
| `Event_NetOut_OrganizationKick` | Kick from org |
| `Event_NetOut_OrganizationRankChange` | Change member rank |
| `Event_NetOut_OrganizationSetRankName` | Rename rank |
| `Event_NetOut_OrganizationSetRankPermissions` | Set rank permissions |
| `Event_NetOut_OrganizationMOTD` | Set org MOTD |
| `Event_NetOut_OrganizationNote` | Set member note |
| `Event_NetOut_OrganizationOfficerNote` | Set officer note |
| `Event_NetOut_OrganizationTransferCash` | Transfer org funds |
| `Event_NetOut_ReloadOrganizations` | Reload org data |
| `Event_NetOut_PvPOrganizationLeaveResponse` | PvP org leave |

### Mail
| Message | Purpose |
|---------|---------|
| `Event_NetOut_SendMailMessage` | Send mail |
| `Event_NetOut_RequestMailHeaders` | Request headers |
| `Event_NetOut_RequestMailBody` | Read mail body |
| `Event_NetOut_DeleteMailMessage` | Delete mail |
| `Event_NetOut_ArchiveMailMessage` | Archive mail |
| `Event_NetOut_ReturnMailMessage` | Return to sender |
| `Event_NetOut_TakeItemFromMailMessage` | Take mail attachment |
| `Event_NetOut_TakeCashFromMailMessage` | Take mail currency |
| `Event_NetOut_PayCODForMailMessage` | Pay COD |

### Trading
| Message | Purpose |
|---------|---------|
| `Event_NetOut_TradeRequest` | Initiate trade |
| `Event_NetOut_TradeRequestCancel` | Cancel trade |
| `Event_NetOut_TradeProposal` | Propose trade |
| `Event_NetOut_TradeLockState` | Lock trade |

### Black Market (Auction House)
| Message | Purpose |
|---------|---------|
| `Event_NetOut_BMCreateAuction` | Create auction |
| `Event_NetOut_BMCancelAuction` | Cancel auction |
| `Event_NetOut_BMPlaceBid` | Place bid |
| `Event_NetOut_BMSearch` | Search auctions |

### Crafting & Research
| Message | Purpose |
|---------|---------|
| `Event_NetOut_Craft` | Craft item |
| `Event_NetOut_Alloy` | Alloy materials |
| `Event_NetOut_Research` | Research recipe |
| `Event_NetOut_ReverseEngineer` | Reverse engineer item |
| `Event_NetOut_RespecCraft` | Respec crafting |
| `Event_NetOut_SpendAppliedSciencePoint` | Spend science point |

### Abilities & Training
| Message | Purpose |
|---------|---------|
| `Event_NetOut_TrainAbility` | Train ability |
| `Event_NetOut_ResetAbilities` | Reset abilities |
| `Event_NetOut_RespecAbility` | Respec ability |
| `Event_NetOut_Respec` | Full respec |

### Stargate
| Message | Purpose |
|---------|---------|
| `Event_NetOut_GiveStargateAddress` | GM: give gate address |
| `Event_NetOut_RemoveStargateAddress` | GM: remove gate address |
| `Event_NetOut_onDialGate` | Dial a stargate |
| `Event_NetOut_SetRingTransporterDestination` | Set ring destination |
| `Event_NetOut_DHD` | DHD interaction |

### Minigames
| Message | Purpose |
|---------|---------|
| `Event_NetOut_StartMinigame` | Start minigame |
| `Event_NetOut_EndMinigame` | End minigame |
| `Event_NetOut_MinigameComplete` | Minigame completed |
| `Event_NetOut_MinigameCallRequest` | Call request |
| `Event_NetOut_MinigameCallAccept` | Accept call |
| `Event_NetOut_MinigameCallDecline` | Decline call |
| `Event_NetOut_MinigameCallAbort` | Abort call |
| `Event_NetOut_MinigameContactRequest` | Contact request |
| `Event_NetOut_MinigameStartCancel` | Cancel start |
| `Event_NetOut_SpectateMinigame` | Spectate |
| `Event_NetOut_RegisterToMinigameHelp` | Register help |
| `Event_NetOut_UpdateRegisterToMinigameHelp` | Update help reg |
| `Event_NetOut_GiveMinigameContact` | GM: give contact |
| `Event_NetOut_RemoveMinigameContact` | GM: remove contact |

### Dueling
| Message | Purpose |
|---------|---------|
| `Event_NetOut_DuelChallenge` | Challenge player |
| `Event_NetOut_DuelResponse` | Accept/decline |
| `Event_NetOut_DuelForfeit` | Forfeit duel |

### GM Commands
| Message | Purpose |
|---------|---------|
| `Event_NetOut_Kill` | Kill target |
| `Event_NetOut_Spawn` | Spawn entity |
| `Event_NetOut_Despawn` | Despawn entity |
| `Event_NetOut_Summon` | Summon player |
| `Event_NetOut_SetGodMode` | Toggle god mode |
| `Event_NetOut_SetHealth` | Set health |
| `Event_NetOut_SetHealthMax` | Set max health |
| `Event_NetOut_SetFocus` | Set focus |
| `Event_NetOut_SetFocusMax` | Set max focus |
| `Event_NetOut_SetLevel` | Set level |
| `Event_NetOut_SetSpeed` | Set speed |
| `Event_NetOut_SetNoAggro` | Toggle no aggro |
| `Event_NetOut_SetNoXP` | Toggle no XP |
| `Event_NetOut_Invisible` | Toggle invisible |
| `Event_NetOut_SetHideGM` | Toggle GM hidden |
| `Event_NetOut_Goto` | Teleport to player |
| `Event_NetOut_GotoLocation` | Teleport to location |
| `Event_NetOut_GotoXYZ` | Teleport to coords |
| `Event_NetOut_SetFlag` | Set game flag |
| `Event_NetOut_ShowFlag` | Show game flag |
| `Event_NetOut_SetInstanceFlag` | Set instance flag |
| `Event_NetOut_ShowInstanceFlag` | Show instance flag |
| `Event_NetOut_SetFaction` | Set faction |
| `Event_NetOut_SetTarget` | Force target |
| `Event_NetOut_ShowPlayer` | Show player info |
| `Event_NetOut_ShowIP` | Show IP address |
| `Event_NetOut_ShowMobCount` | Show mob count |
| `Event_NetOut_ShowTargetLocation` | Show target loc |
| `Event_NetOut_ShowRotation` | Show rotation |
| `Event_NetOut_ShowNavigation` | Show nav mesh |
| `Event_NetOut_ShowPointSet` | Show point set |
| `Event_NetOut_Users` | List online users |
| `Event_NetOut_Who` | Who is online |

### GM Give Commands
| Message | Purpose |
|---------|---------|
| `Event_NetOut_GiveAbility` | Give ability |
| `Event_NetOut_GiveAllAbilities` | Give all abilities |
| `Event_NetOut_GiveAmmo` | Give ammo |
| `Event_NetOut_GiveItem` | Give item |
| `Event_NetOut_GiveInventory` | Give inventory set |
| `Event_NetOut_GiveGearset` | Give gear set |
| `Event_NetOut_GiveBlueprint` | Give blueprint |
| `Event_NetOut_GiveXp` | Give XP |
| `Event_NetOut_GiveExpertise` | Give expertise |
| `Event_NetOut_GiveFaction` | Give faction rep |
| `Event_NetOut_GiveTrainingPoints` | Give training points |
| `Event_NetOut_GiveNaqahdah` | Give naqahdah (currency) |
| `Event_NetOut_GiveAppliedSciencePoints` | Give science points |
| `Event_NetOut_GiveRacialParadigmLevels` | Give racial paradigm |
| `Event_NetOut_GiveRespawner` | Give respawn point |

### GM Mob/AI Control
| Message | Purpose |
|---------|---------|
| `Event_NetOut_GetMobAttribute` | Query mob attr |
| `Event_NetOut_SetMobAttribute` | Set mob attr |
| `Event_NetOut_SetMobVariable` | Set mob variable |
| `Event_NetOut_SetMobAbilitySet` | Set mob abilities |
| `Event_NetOut_SetMobStance` | Set mob stance |
| `Event_NetOut_MobData` | Dump mob data |
| `Event_NetOut_EnterErrorAIState` | Force AI error state |
| `Event_NetOut_ExitErrorAIState` | Exit AI error state |
| `Event_NetOut_EmitBehaviorEventOnMob` | Trigger behavior |
| `Event_NetOut_AddBehaviorEventSet` | Add behavior set |
| `Event_NetOut_RemoveBehaviorEventSet` | Remove behavior set |

### GM Data Loading
| Message | Purpose |
|---------|---------|
| `Event_NetOut_LoadAbility` | Hot-reload ability |
| `Event_NetOut_LoadAbilitySet` | Hot-reload ability set |
| `Event_NetOut_LoadBehavior` | Hot-reload behavior |
| `Event_NetOut_LoadConstants` | Hot-reload constants |
| `Event_NetOut_LoadInteractionSet` | Hot-reload interactions |
| `Event_NetOut_LoadItem` | Hot-reload item |
| `Event_NetOut_LoadMOB` | Hot-reload MOB def |
| `Event_NetOut_LoadMission` | Hot-reload mission |
| `Event_NetOut_LoadNACSI` | Hot-reload NACSI data |
| `Event_NetOut_ListAbilities` | List ability data |
| `Event_NetOut_ListInteractions` | List interactions |

### Debug
| Message | Purpose |
|---------|---------|
| `Event_NetOut_CombatDebug` | Combat debug |
| `Event_NetOut_CombatDebugVerbose` | Verbose combat debug |
| `Event_NetOut_AbilityDebug` | Ability debug |
| `Event_NetOut_HealDebug` | Heal debug |
| `Event_NetOut_DebugAbilityOnMob` | Debug ability on mob |
| `Event_NetOut_DebugBehaviorsOnMob` | Debug behaviors |
| `Event_NetOut_DebugPathsOnMob` | Debug pathing |
| `Event_NetOut_DebugEvents` | Debug events |
| `Event_NetOut_DebugInteract` | Debug interactions |
| `Event_NetOut_Physics` | Physics debug |
| `Event_NetOut_PrintStats` | Print stats |
| `Event_NetOut_PerfStats` | Performance stats |
| `Event_NetOut_PerfStatsByChannel` | Per-channel perf |
| `Event_NetOut_RegenerateCoverLinks` | Regen cover |
| `Event_NetOut_SystemOptions` | System options |
| `Event_NetOut_BroadcastMinimapPing` | Minimap ping |
| `Event_NetOut_XRayEyes` | X-ray vision |
| `Event_NetOut_Petition` | File petition |
| `Event_NetOut_Unstuck` | Unstuck self |
| `Event_NetOut_WorldInstanceReset` | Reset instance |
| `Event_NetOut_SetTechSkill` | Set tech skill |
| `Event_NetOut_CancelMovie` | Cancel movie |
| `Event_NetOut_DialogButtonChoice` | Dialog button |
| `Event_NetOut_Interact` | Interact with target |
| `Event_NetOut_elementDataRequest` | Request element data |
| `Event_NetOut_versionInfoRequest` | Request version info |
| `Event_NetOut_TriggerClientHintedGenericRegion` | Trigger hint region |
| `Event_NetOut_ChangeCoverStanceWeight` | Cover stance weight |
| `Event_NetOut_ChangeCoverWeight` | Cover weight |
| `Event_NetOut_SquadSetLootMode` | Set squad loot |
| `Event_NetOut_DebugAbilityOnMob` | Debug ability |

### Space Queue
| Message | Purpose |
|---------|---------|
| `Event_NetOut_onSpaceQueueStatus` | Queue status |
| `Event_NetOut_onSpaceQueueReadyResponse` | Queue ready response |
| `Event_NetOut_onSpaceQueuedResponse` | Queued response |
| `Event_NetOut_onStrikeTeamResponse` | Strike team response |

### Debug Minigames
| Message | Purpose |
|---------|---------|
| `Event_NetOut_debugJoinMinigame` | Debug join |
| `Event_NetOut_debugMinigameInstance` | Debug instance |
| `Event_NetOut_debugSpectateMinigame` | Debug spectate |
| `Event_NetOut_debugStartMinigame` | Debug start |

---

## SGWNetworkManager

The `SGWNetworkManager` class (source: `SGWNetworkManager.cpp` at 0x019abbc4) is the central hub that:
1. Subscribes to all `Event_NetOut_*` events via `CME::EventSignal::MemberCallback`
2. Serializes them into Mercury messages
3. Sends them to the server via Mercury::Nub

It also handles `Event_Sys_FrameStart` for per-frame network processing.

RTTI: `.?AUEventHandlerBase@SGWNetworkManager@@` at 0x01e29bf8
