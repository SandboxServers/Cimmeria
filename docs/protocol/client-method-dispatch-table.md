---
title: "SGWPlayer Client Method Dispatch Table (Server → Client)"
type: reference
audience: engineers
last_updated: 2026-07-25
---

# SGWPlayer Client Method Dispatch Table (Server → Client)

> **Last updated**: 2026-07-25
> **Verified**: 2026-07-25 — all 157 index/name pairs re-derived from
> `entities/defs/` by replaying the BigWorld flattening rule, and diffed
> against both this table and the constants in
> `crates/services/src/cell/client_methods/`. Zero mismatches in either
> direction.
> **Total methods**: 157 (indices 0–156)
> **Encoding**: Methods 0–60 use direct wire encoding (`msg_id = 0x80 + index`);
> methods 61+ use extended encoding (`msg_id = 0xBD`, sub-byte = `index - 61`).
> **Entity type**: SGWPlayer (class_id = 0x02)

---

## BigWorld Flattening Rule

The flat client method index is computed by the BigWorld entity definition parser
(`entity_description.cpp:parseInterface()`). For each entity in the inheritance
chain (root → leaf), the parser processes:

1. **`<Implements>` interfaces first** (recursively, in document order)
2. **The entity's own `<ClientMethods>`** second

This means interface methods always come before the entity's own methods at each
level. The full parse order for SGWPlayer is:

```
SGWEntity (0 own, 0 interfaces with client methods)
  └─ SGWSpawnableEntity (12 own methods)
       └─ SGWBeing (1 own method)
            ├─ Implements: SGWBeing-interface (8), SGWAbilityManager (0), SGWCombatant (6)
            └─ SGWPlayer (59 own methods)
                 └─ Implements: Communicator (7), OrganizationMember (18),
                    MinigamePlayer (13), GateTravel (4), SGWInventoryManager (7),
                    SGWMailManager (4), Missionary (5), SGWPoller (0),
                    ContactListManager (5), SGWBlackMarketManager (6), ClientCache (2)
```

---

## Summary

| Range | Source | Count | Rust constants |
|-------|--------|-------|----------------|
| 0–11 | SGWSpawnableEntity own | 12 | — |
| 12–19 | SGWBeing interface | 8 | — |
| 20–25 | SGWCombatant interface | 6 | — |
| 26 | SGWBeing own | 1 | — |
| 27–33 | Communicator | 7 | — |
| 34–51 | OrganizationMember | 18 | — |
| 52–64 | MinigamePlayer | 13 | `CLIENT_MG_*` |
| 65–68 | GateTravel | 4 | — |
| 69–75 | SGWInventoryManager | 7 | — |
| 76–79 | SGWMailManager | 4 | — |
| 80–84 | Missionary | 5 | — |
| 85–89 | ContactListManager | 5 | — |
| 90–95 | SGWBlackMarketManager | 6 | — |
| 96–97 | ClientCache | 2 | — |
| 98–156 | SGWPlayer own | 59 | — |

---

## Complete Method Table

### SGWSpawnableEntity (entity own) — 12 methods, indices 0–11

| Index | Method | Args |
|-------|--------|------|
| 0 | `onStaticMeshNameUpdate` | `WSTRING StaticMeshName, WSTRING BodySetName` |
| 1 | `onSequence` | `INT32 KismetEventSetSeqID, INT32 SourceID, INT32 TargetID, INT8 PrimaryTarget, FLOAT ImpactTime, ARRAY<NameValuePair> NameValuePairs, INT8 ViewType, INT32 InstanceId` |
| 2 | `onEntityMove` | `FLOAT locationX/Y/Z, FLOAT velocityX/Y/Z, FLOAT yaw, FLOAT pitch, FLOAT roll` |
| 3 | `InteractionType` | `UINT64 TypeId` |
| 4 | `onEntityFlags` | `UINT64 aFlags` |
| 5 | `getInteractions` | `MAILBOX aEntity` |
| 6 | `toggleInteractionDebugging` | `INT32 playerId` |
| 7 | `onEntityProperty` | `INT32 type, INT32 value` |
| 8 | `onVisible` | `INT8 visible` |
| 9 | `onKismetEventSetUpdate` | `INT32 kismetEventSetId` |
| 10 | `onEntityTint` | `UINT32 primaryColorId, UINT32 secondaryColorId, UINT32 skinColorId` |
| 11 | `onBeingNameIDUpdate` | `INT32 BeingNameID` |

### SGWBeing (interface) — 8 methods, indices 12–19

| Index | Method | Args |
|-------|--------|------|
| 12 | `onTimerUpdate` | `INT32 ID, INT8 Type, INT32 SourceID, FLOAT TotalTime, FLOAT BigWorldTimeComplete` |
| 13 | `onEffectUserData` | `INT32 InstanceId, ARRAY<WSTRING> UserDataNames, ARRAY<WSTRING> UserDataValues` |
| 14 | `onEffectResults` | `INT32 SourceID, INT32 AbilityID, INT32 EffectID, INT32 TargetID, UINT8 ResultCode, ClientEffectResultList` |
| 15 | `onLevelUpdate` | `INT32 Level` |
| 16 | `onTargetUpdate` | `INT32 TargetId` |
| 17 | `onBeingNameUpdate` | `WSTRING BeingName` |
| 18 | `onTopSpeedUpdate` | `FLOAT TopSpeed` |
| 19 | `onStateFieldUpdate` | `INT32 bStateField` |

### SGWCombatant (interface) — 6 methods, indices 20–25

| Index | Method | Args |
|-------|--------|------|
| 20 | `onStatUpdate` | `StatUpdateList Stats` |
| 21 | `onStatBaseUpdate` | `StatUpdateList Stats` |
| 22 | `onMeleeRangeUpdate` | `INT32 range` |
| 23 | `onArchetypeUpdate` | `INT32 archetype` |
| 24 | `onAlignmentUpdate` | `INT8 alignment` |
| 25 | `onFactionUpdate` | `INT8 faction` |

### SGWBeing (entity own) — 1 method, index 26

| Index | Method | Args |
|-------|--------|------|
| 26 | `BeingAppearance` | `WSTRING BodySet, ARRAY<WSTRING> Components` |

### Communicator (interface) — 7 methods, indices 27–33

| Index | Method | Args |
|-------|--------|------|
| 27 | `onSystemCommunication` | `INT32 TextType, INT32 StringId, WSTRING Speaker, ARRAY<StringToken> tokenList` |
| 28 | `onPlayerCommunication` | `WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel, WSTRING Text` |
| 29 | `onLocalizedCommunication` | `WSTRING Speaker, UINT8 SpeakerFlags, UINT8 Channel, WSTRING Text, ARRAY<StringToken> tokenList` |
| 30 | `onTellSent` | `WSTRING aTarget, WSTRING aText` |
| 31 | `onChatJoined` | `WSTRING ChannelName, UINT8 ChannelID` |
| 32 | `onChatLeft` | `WSTRING ChannelName` |
| 33 | `onNickChanged` | `WSTRING aPlayerName, WSTRING aPlayerNickname, UINT8 aAddRemoveFlag` |

### OrganizationMember (interface) — 18 methods, indices 34–51

| Index | Method | Args |
|-------|--------|------|
| 34 | `onOrganizationInvite` | `WSTRING aInviterName, UINT8 aOrganizationType, INT32 aRequestID, WSTRING aName, UINT8 aIsStrikeTeam` |
| 35 | `onOrganizationJoined` | `INT32 aOrganizationId, UINT8 aOrganizationType, UINT8 aRank, UINT8 aNewMember` |
| 36 | `onOrganizationLeft` | `UINT8 aReason, INT32 aOrganizationId` |
| 37 | `onMemberJoinedOrganization` | `WSTRING aMemberName, INT32 aMember, INT32 aOrganizationId, UINT8 aRank, UINT8 aNewMember` |
| 38 | `onOrganizationRosterInfo` | `INT32 aOrganizationId, ARRAY<RosterInfo> aRosterInfo` |
| 39 | `onMemberLeftOrganization` | `INT32 aMember, UINT8 aReason, INT32 aOrganizationId, WSTRING aMemberName` |
| 40 | `onMemberRankChangedOrganization` | `INT32 aMember, UINT8 aRank, INT32 aOrganizationId, WSTRING aMemberName` |
| 41 | `onStrikeTeamUpdate` | `INT32 aOrganizationId, UINT8 aPvPValue` |
| 42 | `onPvPOrganizationLeaveRequest` | `INT32 aOrganizationId, UINT8 aPvPValue` |
| 43 | `onOrganizationNameUpdate` | `INT32 aOrganizationId, WSTRING aName` |
| 44 | `onOrganizationExperienceUpdate` | `INT32 aOrganizationId, UINT64 aExperience` |
| 45 | `onOrganizationMOTDUpdate` | `INT32 aOrganizationId, WSTRING aMOTD` |
| 46 | `onOrganizationNoteUpdate` | `INT32 aOrganizationId, WSTRING aName, WSTRING aNote` |
| 47 | `onOrganizationOfficerNoteUpdate` | `INT32 aOrganizationId, WSTRING aName, WSTRING aNote` |
| 48 | `onOrganizationCashUpdate` | `INT32 aOrganizationId, UINT64 aCash` |
| 49 | `onOrganizationRankUpdate` | `INT32 aOrganizationId, ARRAY<INT32> aRankIds, ARRAY<INT32> aRankFlags` |
| 50 | `onOrganizationRankNameUpdate` | `INT32 aOrganizationId, ARRAY<INT32> aRankIds, ARRAY<WSTRING> aRankNames` |
| 51 | `onSquadLootType` | `INT32 aOrganizationId, INT32 aLootType` |

### MinigamePlayer (interface) — 13 methods, indices 52–64

| Index | Method | Args |
|-------|--------|------|
| 52 | `onStartMinigame` | `WSTRING URL` |
| 53 | `onStartMinigameDialog` | `WSTRING Name, WSTRING Difficulty, INT32 TCLevel, WSTRING Verb, INT32 ArchetypeBitfield, UINT8 CanPlay, UINT8 CanCall, UINT8 CanSpectate` |
| 54 | `onStartMinigameDialogClose` | *(none)* |
| 55 | `onEndMinigame` | *(none)* |
| 56 | `onSpectateList` | `ARRAY<INT32> playerIds, ARRAY<WSTRING> playerNames` |
| 57 | `onMinigameRegistrationPrompt` | `INT32 Cost` |
| 58 | `minigameRegistrationInfo` | `UINT8 Registered, UINT8 InRangeOnly, UINT8 WantsRequests, WSTRING Note` |
| 59 | `addOrUpdateMinigameHelper` | `INT32 PlayerId, WSTRING Name, WSTRING Note, UINT8 Level, UINT8 Archetype, UINT8 Friend` |
| 60 | `removeMinigameHelper` | `INT32 PlayerId` |
| 61 | `minigameCallDisplay` | `INT32 CallingPlayerId, WSTRING Name, INT32 Archetype, INT32 Level, INT32 TipAmount, INT32 ExpiresAt, WSTRING GameName, WSTRING GameDifficulty, WSTRING GameVerb, INT32 GameTC, WSTRING NPCTitle` |
| 62 | `minigameCallResult` | `INT32 ResultCode, FLOAT StartTime` |
| 63 | `minigameCallAbort` | `INT32 CallingPlayerId` |
| 64 | `showMinigameContact` | `INT32 Id, WSTRING Name, WSTRING Title, WSTRING Icon, INT32 Time, INT32 Success, INT32 Cost` |

### GateTravel (interface) — 4 methods, indices 65–68

| Index | Method | Args |
|-------|--------|------|
| 65 | `setupStargateInfo` | `ARRAY<INT32> worldStargateList, ARRAY<INT32> knownStargateList, ARRAY<INT32> hiddenStargateList` |
| 66 | `updateStargateAddress` | `INT32 addressId, UINT8 hasAddress, UINT8 hidden` |
| 67 | `stargateRotationOverride` | `FLOAT yaw` |
| 68 | `onStargatePassage` | `INT32 addressId` |

### SGWInventoryManager (interface) — 7 methods, indices 69–75

| Index | Method | Args |
|-------|--------|------|
| 69 | `onBagInfo` | `ARRAY<BagInfo> BagInfo` |
| 70 | `onActiveSlotUpdate` | `INT32 BagId, INT32 SlotId` |
| 71 | `onRemoveItem` | `ARRAY<INT32> ItemIdList` |
| 72 | `onUpdateItem` | `ARRAY<InvItem> ItemUpdates` |
| 73 | `onRefreshItem` | `INT32 ItemId` |
| 74 | `onClearOrgVaultInventory` | `INT32 OrganizationId` |
| 75 | `onCashChanged` | `INT32 cash` |

### SGWMailManager (interface) — 4 methods, indices 76–79

| Index | Method | Args |
|-------|--------|------|
| 76 | `onMailHeaderInfo` | `UINT8 ResetCategory, UINT8 bArchive, ARRAY<MessageHeader> MessageHeaders, ARRAY<MessageAttachment> MessageAttachments` |
| 77 | `onMailHeaderRemove` | `INT32 MailId` |
| 78 | `onMailRead` | `INT32 MailId, WSTRING BodyText, INT32 BodyId, WSTRING ToText` |
| 79 | `sendMailResult` | `UINT8 ResultCode, ARRAY<WSTRING> FailedRecipients, INT32 FailedRecipientFlags` |

### Missionary (interface) — 5 methods, indices 80–84

| Index | Method | Args |
|-------|--------|------|
| 80 | `onMissionUpdate` | `INT32 MissionID, INT8 Status, INT32 MissionGiverName` |
| 81 | `onStepUpdate` | `INT32 StepID, INT8 Status` |
| 82 | `onObjectiveUpdate` | `INT32 ObjectiveID, INT8 Status, INT8 Hidden, INT8 Optional` |
| 83 | `onTaskUpdate` | `INT32 TaskID, INT8 Status, INT32 Count` |
| 84 | `offerSharedMission` | `INT32 MissionId` |

### ContactListManager (interface) — 5 methods, indices 85–89

| Index | Method | Args |
|-------|--------|------|
| 85 | `onContactListUpdate` | `INT32 aListId, WSTRING aName, UINT32 aFlags` |
| 86 | `onContactListDelete` | `INT32 aListId` |
| 87 | `onContactListAddMembers` | `INT32 aListId, ARRAY<WSTRING> aPlayerNames` |
| 88 | `onContactListRemoveMembers` | `INT32 aListId, ARRAY<WSTRING> aPlayerNames` |
| 89 | `onContactListEvent` | `WSTRING aPlayerName, UINT32 aEventId, INT32 aDataValue` |

### SGWBlackMarketManager (interface) — 6 methods, indices 90–95

| Index | Method | Args |
|-------|--------|------|
| 90 | `onBMOpen` | `INT32 entityId` |
| 91 | `onBMError` | `INT32 errorId` |
| 92 | `onBMAuctions` | `ARRAY<AuctionItem> auctionItems, INT32 totalResults, INT32 clientKey` |
| 93 | `onBMAuctionRemove` | `INT32 sequenceId` |
| 94 | `onBMAuctionUpdate` | `AuctionItem auctionItem` |
| 95 | `onBMWatchedItemsUpdate` | `ARRAY<INT32> itemList` |

### ClientCache (interface) — 2 methods, indices 96–97

| Index | Method | Args |
|-------|--------|------|
| 96 | `onVersionInfo` | `INT32 CategoryId, INT32 Version, INT32 RequiredUpdates, INT8 InvalidateAll, ARRAY<INT32> InvalidKeys` |
| 97 | `onCookedDataError` | `INT32 categoryID, INT32 elementKey` |

### SGWPlayer (entity own) — 59 methods, indices 98–156

| Index | Method | Args |
|-------|--------|------|
| 98 | `onBeginAidWait` | `INT32 TimeToAid, ARRAY<Respawner> respawners` |
| 99 | `onEndAidWait` | *(none)* |
| 100 | `onDHDReply` | `WSTRING aMessage` |
| 101 | `onKnownAbilitiesUpdate` | `ARRAY<INT32> AbilityData` |
| 102 | `onTimeofDay` | `FLOAT Time, FLOAT Wind, INT8 Weather` |
| 103 | `onOverridePerfStatsRate` | `INT32 NewIntervalMS` |
| 104 | `onInitialInteraction` | `ARRAY<DialogChoices> Choices` |
| 105 | `onDialogDisplay` | `INT32 EntityId, INT32 DialogID, INT32 MissionFlags, UINT8 IsImmediate, INT32 aMissionId` |
| 106 | `onVaultOpen` | `INT32 EntityId, VECTOR3 Position` |
| 107 | `onTeamVaultOpen` | `INT32 EntityId, VECTOR3 Position` |
| 108 | `onCommandVaultOpen` | `INT32 EntityId, VECTOR3 Position` |
| 109 | `onStoreOpen` | `INT32 EntityId, INT32 VendorType, ARRAY<StoreItem> Items, ...` |
| 110 | `onStoreUpdate` | `ARRAY<ItemCostUpdate> ItemCostUpdates` |
| 111 | `onStoreClose` | *(none)* |
| 112 | `onCraftingRespecPrompt` | `INT32 CostToRespec` |
| 113 | `onTrainerOpen` | `INT32 TrainerID, ARRAY<TrainerAbility> Abilities, INT32 CostToRespec` |
| 114 | `onLootDisplay` | `INT32 EntityID, LootItemQuantityList ItemList, INT8 Initial` |
| 115 | `onPlayerDataLoaded` | *(none)* |
| 116 | `onPlayerTeleport` | `VECTOR3 Location, VECTOR3 Direction` |
| 117 | `onClientMapLoad` | `WSTRING areaName, WSTRING mapPath, INT32 WorldID, VECTOR3 Location, VECTOR3 Direction` |
| 118 | `giveAbility` | `INT32 abilityId, INT8 persist` |
| 119 | `giveXPForLevel` | `INT32 level` |
| 120 | `onDisplayDHD` | `UINT8 PointOfOrigin` |
| 121 | `onErrorCode` | `UINT8 SystemID, INT32 InstanceID, UINT16 ErrorCodeID` |
| 122 | `setupWorldParameters` | `INT32 worldId, INT32 weatherSetId, INT32 minToRealMinutes, INT32 minutesPerDay, INT32 currentTimeInSeconds, FLOAT gravity, FLOAT runSpeed, ... (22 args total)` |
| 123 | `onMapInfo` | `UINT32 SysTypeID, UINT32 SysID, UINT32 KeyID, INT32 WorldID, VECTOR3 Location, UINT32 Lifetime, UINT8 Delete` |
| 124 | `clearClientHintedGenericRegions` | *(none)* |
| 125 | `addClientHintedGenericRegion` | `INT32 regionId, FLOAT height, FLOAT radius, INT32 flags, ARRAY<VECTOR3> points` |
| 126 | `onResetMapInfo` | *(none)* |
| 127 | `onMissionRewardsDisplay` | `Rewards Rewards, INT32 aMissionId` |
| 128 | `onMissionOfferDisplay` | `INT32 aDialogId, Rewards Rewards, INT32 aMissionId` |
| 129 | `stargateTriggerFailed` | *(none)* |
| 130 | `onExtraNameUpdate` | `WSTRING ExtraName` |
| 131 | `onExpUpdate` | `INT32 Exp` |
| 132 | `onMaxExpUpdate` | `INT32 MaxExp` |
| 133 | `onRingTransporterList` | `RegionInfo aRegion, ARRAY<RegionInfo> aRegionList` |
| 134 | `onOrganizationCreationResult` | `UINT8 Result, UINT8 RetCode` |
| 135 | `launchOrganizationCreation` | `UINT8 aOrgType` |
| 136 | `onUpdateDiscipline` | `INT32 aDisciplineSeqId, INT32 aExpertise` |
| 137 | `onDisciplineRespec` | *(none)* |
| 138 | `onUpdateRacialParadigmLevel` | `INT32 aRacialParadigmId, INT8 aLevel` |
| 139 | `onUpdateKnownCrafts` | `ARRAY<INT32> aCraftList` |
| 140 | `onUpdateCraftingOptions` | `CraftingOptions aOptions` |
| 141 | `onAbilityTreeInfo` | `ARRAY<ARRAY<INT32>> AbilityLists` |
| 142 | `onClientChallenge` | `INT32 aClientChallenge, INT32 aChallengeType, WSTRING aChallengeObject, INT32 aChallengeID1, INT32 aChallengeID2` |
| 143 | `onDuelChallenge` | `INT32 aEntityId, ARRAY<INT32> aSquadList` |
| 144 | `onTradeState` | `INT32 EntityId, LocalTradeProposal LocalProposal, RemoteTradeProposal RemoteProposal` |
| 145 | `onTradeResults` | `INT32 EntityId, INT32 Result` |
| 146 | `onSpaceQueued` | `WSTRING aSpaceName` |
| 147 | `onSpaceQueueReady` | `WSTRING aSpaceName` |
| 148 | `onRemoteEntityCreate` | `INT32 aEntityId, WSTRING aEntityType, VECTOR3 aPosition, INT32 aWorldId, INT32 aSpaceId` |
| 149 | `onRemoteEntityMove` | `INT32 aEntityId, VECTOR3 aPosition, INT32 aWorldId, INT32 aSpaceId` |
| 150 | `onRemoteEntityRemove` | `INT32 aEntityId` |
| 151 | `onDuelEntitiesSet` | `ARRAY<INT32> aEntityList` |
| 152 | `onDuelEntitiesRemove` | `INT32 aEntityId` |
| 153 | `onDuelEntitiesClear` | *(none)* |
| 154 | `onThreatenedMobsUpdate` | `INT32 EntityId, UINT8 HasThreat` |
| 155 | `onPlayMovie` | `WSTRING MovieName, UINT8 FullScreen` |
| 156 | `onCancelMovie` | `WSTRING MovieName, INT32 EntityId` |

---

## Wire Encoding

Methods 0–60 are encoded as **direct** calls: `msg_id = 0x80 + method_index`.

Methods 61+ are encoded as **extended** calls: `msg_id = 0xBD`, followed by a
sub-index byte = `method_index - 61`. This is because BigWorld reserves message
IDs 0x80–0xBC for direct method calls (61 slots), then uses 0xBD as an
overflow marker.

The boundary at 61 is computed by the engine from the total method count:
`numSubSlots = ceil((157 - 63) / 255) = 1`, `begSubSlot = 62 - numSubSlots = 61`.

---

## Derivation

Generated by parsing all `.def` files in `entities/defs/` following BigWorld's
`entity_description.cpp:parseInterface()` parse order. Verified against 14
empirically confirmed indices from the running server codebase.

Source script and verification anchors are in the commit that added this file.
