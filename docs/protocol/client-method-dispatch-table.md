---
title: "Client Method Dispatch Table (Server → Client): SGWPlayer, SGWMob, SGWPet"
type: reference
audience: engineers
last_updated: 2026-09-27
---

# Client Method Dispatch Table (Server → Client): SGWPlayer, SGWMob, SGWPet

> This reference covers three entity types' server→client method dispatch: the full 157-method
> **SGWPlayer** table below is the primary/reference table (and the one
> [`tools/wire_decoder_codegen.py`](../../tools/wire_decoder_codegen.py) parses — see its own
> section-boundary note); the **SGWMob** and **SGWPet** tables further down are each a much
> smaller, separate index space layered on top of the shared `SGWSpawnableEntity`/`SGWBeing`/
> `SGWCombatant` prefix (their own method indices reuse small numbers that collide with
> SGWPlayer's at the same index — they are a different entity type's dispatch table, not an
> extension of SGWPlayer's).
>
> **Last updated**: 2026-09-29 — added [Client handler bindings](#client-handler-bindings) (which class dispatches which method)
> **Previously**: 2026-09-27 — added the SGWPet table (pets campaign PT-E1); 2026-09-25 — added the SGWMob table (NA33)
> **Verified**: 2026-07-25 — all 157 index/name pairs re-derived from
> `entities/defs/` by replaying the BigWorld flattening rule, and diffed
> against both this table and the constants in
> `crates/wire/src/cell/client_methods/`. Zero mismatches in either
> direction.
> **Verified continuously** by `cimmeria-wire`'s `mercury::def_conformance` ([crates/wire/src/mercury/def_conformance/](../../crates/wire/src/mercury/def_conformance/), #801): it replays the flattening rule
> below over `entities/defs/` in CI and fails on any constant that drifts from it
> (`method_idx`, the per-interface tables, the SGWMob and SGWPet indices, and the
> `wire-log` name table).
> **Total methods**: 157 (indices 0–156)
> **Encoding**: Methods 0–60 use direct wire encoding (`msg_id = 0x80 + index`);
> methods 61+ use extended encoding (`msg_id = 0xBD`, sub-byte = `index - 61`).
> **Entity type**: SGWPlayer (class_id = 0x02) — see the SGWMob and SGWPet sections below for
> their own entity types and class_ids.

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
| 12 | `onTimerUpdate` | `INT32 ID, INT8 Type, INT32 SourceID, INT32 SecondaryId, FLOAT TotalTime, FLOAT BigWorldTimeComplete` |
| 13 | `onEffectUserData` | `INT32 InstanceId, ARRAY<WSTRING> UserDataNames, ARRAY<WSTRING> UserDataValues` |
| 14 | `onEffectResults` | `INT32 SourceID, INT32 AbilityID, INT32 EffectID, INT32 TargetID, UINT8 ResultCode, ClientEffectResultList` |
| 15 | `onLevelUpdate` | `INT32 Level` |
| 16 | `onTargetUpdate` | `INT32 TargetId` |
| 17 | `onBeingNameUpdate` | `WSTRING BeingName` |
| 18 | `onTopSpeedUpdate` | `FLOAT TopSpeed` |
| 19 | `onStateFieldUpdate` | `INT32 bStateField` |

`onTimerUpdate.BigWorldTimeComplete` is an absolute time on the client's game clock, `TICK_SYNC.gameTime / hertz` seconds ([system-protocol-wire-formats.md](../reverse-engineering/findings/system-protocol-wire-formats.md#the-client-game-clock)). Senders use `game_clock::game_time_secs() + duration` (`crates/wire/src/mercury/game_clock/`). The client shows `complete - clock`, clamped to 0, so `0.0` clears a timer, and the effect handler (type 5) creates no icon for an expiry already in the past.

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

Every method in this block has an argument serializer, `build_on_<method>`, in [`crates/wire/src/cell/client_methods/organization/builders.rs`](../../crates/wire/src/cell/client_methods/organization/builders.rs), typed on the `cimmeria_entity::organization` models and pinned byte for byte in `organization/tests.rs` (organizations campaign ORG-01). `RosterInfo` is `entities/defs/alias.xml:27-36`: `WSTRING name, UINT8 level, UINT8 archetype, UINT8 rank, WSTRING note, WSTRING officerNote`, with no online flag. `ARRAY` is a `u32` count and the elements. The indices are also in `mercury::method_idx`, re-exported from `client_methods::organization` so the two tables cannot drift. Sent today: 34-40 and 51 for squads (ORG-03, ORG-04); for Teams and Commands, 35, 37, 38, 43-45 and 48-50 in the state push (ORG-05, ORG-06), 36 and 39 on a leave or disband (ORG-06), 34, 36, 37, 39 and 40 on an invite, join, kick or rank change (ORG-07), and 45-47, 49 and 50 when a MOTD, note, officer note, rank mask or rank name changes (ORG-08; 47 only to members whose rank holds `OfficerNotes`, and as a visibility sync when that changes). 41 and 42 are never sent (no strike-team feature).

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
| 112 | `onCraftingRespecPrompt` | `INT32 CostToRespec` — sent with 0 when a player's `.respeccraft` opens a crafting respec (`base/crafting/respec/`) |
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
| 137 | `onDisciplineRespec` | *(none)* — sent after a confirmed crafting respec, then 139 and the ASP total |
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

`onErrorCode` (121), `onOrganizationCreationResult` (134) and `launchOrganizationCreation` (135) have serializers in [`crates/wire/src/cell/client_methods/player.rs`](../../crates/wire/src/cell/client_methods/player.rs) (`build_on_error_code`, `build_on_organization_creation_result`, `build_launch_organization_creation`), each with a byte test. 134 and 135 go out with the extended encoding (sub-slot 73 and 74). Since ORG-05 the cell sends 135 when an eligible player right-clicks an organization registrar, and the base sends 134 for every named creation: `(1, 0)` on success, or `(0, RetCode)` on a refusal. The `Result` and `RetCode` values are project policy, not recovered data (`org_creation_ret_code` in `player.rs`; [organization-system.md § Creation](../gameplay/organization-system.md#creation-org-05)).

#### Names that look like client methods but are not

`onSendCombatDebug` and `onSendEventDebug` are declared in `SGWPlayer.def` under `<CellMethods>` without `<Exposed/>` (lines 685 and 690), so they are server-internal cell methods with no client method index. The client has no handler, event or string for either, so a server cannot show a debug line through them. Debug text reaches a player only as a chat line, `onPlayerCommunication` (28) on the feedback channel. Evidence: [native-combat-debug.md](../reverse-engineering/findings/native-combat-debug.md).

#### Crafting payloads (112, 136-140)

The server-side serializers are in `crates/wire/src/crafting/client_methods.rs`, each byte-exact tested. Integers are little-endian; an `ARRAY` is a `u32` element count followed by the elements.

| Index | Argument bytes |
|-------|----------------|
| 112 | `i32 CostToRespec` (4 bytes) |
| 136 | `i32 aDisciplineSeqId, i32 aExpertise` (8 bytes) |
| 137 | none (0 bytes) |
| 138 | `i32 aRacialParadigmId, i8 aLevel` (5 bytes) |
| 139 | `u32 count, count × i32 blueprint id` |
| 140 | `CraftingOptions`: four `CraftingInfo`, each `u32 n, n × i32 items` then `u32 m, m × i32 entities` |
| 7 (ASP) | `onEntityProperty(i32 2, i32 total)`: `GENERICPROPERTY_AppliedSciencePoints`, always the unspent total |

Where the server sends them: the `mapLoaded` bundle carries 139 and the ASP property from the `sgw_player` row. After the `onClientReady` burst, `base/crafting/sync/` sends one reliable bundle with 136 per known discipline, 138 per racial paradigm (all five), 139 and the ASP property ([crafting CR-03](../analysis/crafting/work-packets.md#cr-03)). Learning a discipline (95) answers with 136 and the ASP property; the GM ASP grant answers with the ASP property.

`CraftingOptions` is a `FIXED_DICT`, which goes on the wire as its fields in declaration order with no header. The order in [`entities/defs/alias.xml`](../../entities/defs/alias.xml) (`CraftingOptions`, `CraftingInfo`) is:

| Offset (all lists empty) | Field |
|---|---|
| 0 | `crafting.items` |
| 4 | `crafting.entities` |
| 8 | `research.items` |
| 12 | `research.entities` |
| 16 | `reverseEngineering.items` |
| 20 | `reverseEngineering.entities` |
| 24 | `alloying.items` |
| 28 | `alloying.entities` |

The server sends one tool id (`items`, an inventory instance id) and one machine id (`entities`, a station entity id, or the player's own id under `.allcraft`'s "craft anywhere") per section at most, because the client keeps only the last id of each array (CR-E1 Q2). It is sent after every `onClientReady` and then on change; see [gameplay/crafting-system.md](../gameplay/crafting-system.md) "Stations, tools and crafting options".

With every list empty the payload is 32 zero bytes, which disables every crafting tab. `items` names usable tools (item ids) and `entities` usable machines (entity ids); the client keeps only the last id of each list ([crafting audit C-35](../analysis/crafting/audit.md)). The order above comes from the def file; the client unpacker (`0x00e49180` → `0x00e47250`) has not yet been checked against it (crafting packet CR-E1, question 2).

---

## Client handler bindings

> **Added**: 2026-09-29, from the colo drop-oracle report (191 dropped `onTimerUpdate` in one session).

A method index existing in an entity's table does not mean the client handles it for that
entity. The client binds a handler (a `(clientIndex, methodIndex)` node naming an
`Event_NetIn_*` signal) only for the class named at registration, and it registers every
NetIn handler in one startup sweep: `FUN_00c6f1e0(class, method, event)`, called 162 times
from `0x00db3390` and from nowhere else. `FUN_00c6f1e0` resolves the class's
`EntityDescription`, reads its clientIndex (`desc+0x1e`) and the method's index
(`MethodDescription+0x44`), and inserts the node. The dispatcher
`Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) looks the method up under the receiving
entity's clientIndex, then under each parent class's (`FUN_0158eca0`), and on a miss calls
`0x01590f30` and returns: the method is skipped whole, nothing else in the bundle is lost,
and the telemetry DLL reports `client.dispatch.method_dropped`.

So a method bound under `SGWBeing` works on every being (`SGWBeing` 1, `SGWPlayer` 2,
`SGWGmPlayer` 3, `SGWMob` 4, `SGWPet` 5), a method bound under `SGWPlayer` works on player
entities only, and nothing bound under `SGWBeing` reaches a plain `SGWSpawnableEntity` (0) or
`SGWDuelMarker` (6).

| Bound under | Methods (this table's indices) |
|---|---|
| `SGWSpawnableEntity` | `onStaticMeshNameUpdate`, `onEntityMove`, `InteractionType`, `onEntityFlags`, `onEntityProperty`, `onVisible`, `onKismetEventSetUpdate`, `onEntityTint` |
| `SGWBeing` | `onSequence`, `onBeingNameIDUpdate`, `onEffectResults`, `onLevelUpdate`, `onTargetUpdate`, `onBeingNameUpdate`, `onStateFieldUpdate`, `onStatUpdate`, `onStatBaseUpdate`, `onMeleeRangeUpdate`, `onAlignmentUpdate`, `onFactionUpdate`, `BeingAppearance` |
| `SGWPlayer` | every other method in the SGWPlayer table except those below, **including `onTimerUpdate` (12), `onEffectUserData` (13) and `onArchetypeUpdate` (23)** |
| `SGWMob` | `onAggressionOverrideUpdate`, `onAggressionOverrideCleared` |
| `SGWPet` | `onPetAbilityList`, `onPetStanceList`, `onPetStanceUpdate` |
| `SGWGmPlayer` | `onLOSResult`, `onShowWaypoints`, `onShowPath`, `onDisableShowPath`, `onSetTarget`, `onShowNavigation` |
| *(bound nowhere)* | `getInteractions` (5), `toggleInteractionDebugging` (6), `onTopSpeedUpdate` (18), the six `onBM*` (90-95), `onPlayerTeleport` (116), `giveAbility` (118), `giveXPForLevel` (119), `onOrganizationCreationResult` (134) |

Consequences for the server:

- **`onTimerUpdate` goes to the owning player's own client only.** Sent about an NPC or pet
  (an NPC's cooldown, a duration timer on an NPC target) it is dropped by every witness. The
  cell routes every timer through `send_timer_update`
  ([`timer_update.rs`](../../crates/cell-combat/src/cell/abilities/timer_update.rs)), and the
  witness fan-out helpers refuse method 12 for a non-player with a WARN
  (`reason = no_client_binding`, target `cimmeria_cell_combat::cell::abilities::messaging`). An NPC's attack is shown by its
  `onSequence`, which `SGWBeing` binds.
- **`onBeingNameIDUpdate` goes to beings only.** The AoI cascade skips it for a class-0 prop or
  corpse (`class_binds_being_methods` in `crates/wire/src/mercury/aoi/create.rs`).
- A method in the *bound nowhere* row always shows up in the drop oracle. Seen on the colo on
  2026-09-29: `onPlayerTeleport` (116) and `giveXPForLevel` (119). Sending them does nothing on
  a stock client; the six `onBM*` need the client patch in
  [black-market-client-window-patch.md](../reverse-engineering/findings/black-market-client-window-patch.md).

The table was read from the decompiled sweep in
`docs/reverse-engineering/decompiled/14_standalone_named.c` (the function Ghidra once named
`register_NetOut_onStrikeTeamResponse`), and `FUN_00c6f1e0` and the dispatcher were
re-decompiled on 2026-09-29.

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

## SGWMob Client Method Dispatch Table

> **Added**: 2026-09-25 (NA33). **Entity type**: SGWMob (`class_id = 0x04`).
> **Total methods**: 29 (indices 0-28), all under any plausible idbase (62),
> so every method uses **direct** wire encoding: `msg_id = 0x80 + index`.
> Full evidence trail: [`docs/reverse-engineering/findings/npc-aggression-broadcast.md`](../reverse-engineering/findings/npc-aggression-broadcast.md).

SGWMob's inheritance chain is `SGWEntity → SGWSpawnableEntity → SGWBeing →
SGWMob`, with `SGWMob` implementing `Lootable`:

```
SGWSpawnableEntity (12 own methods)
  └─ SGWBeing (1 own method)
       ├─ Implements: SGWBeing-interface (8), SGWAbilityManager (0), SGWCombatant (6)
       └─ SGWMob (2 own methods)
            └─ Implements: Lootable (0 client methods — entities/defs/interfaces/Lootable.def
                            has an empty <ClientMethods/> block)
```

Indices 0-26 are **identical** to the SGWPlayer table above (see the
"SGWSpawnableEntity own", "SGWBeing (interface)", "SGWCombatant (interface)"
and "SGWBeing (entity own)" sections — same tables apply verbatim, since
that prefix is a property of the shared ancestor classes, not the leaf
entity). SGWMob's own methods begin at index 27, immediately after the
shared prefix, because `Lootable` contributes no client methods:

| Index | Method | Args |
|-------|--------|------|
| 0-11 | *(SGWSpawnableEntity own — see SGWPlayer table above)* | — |
| 12-19 | *(SGWBeing interface — see SGWPlayer table above)* | — |
| 20-25 | *(SGWCombatant interface — see SGWPlayer table above)* | — |
| 26 | *(SGWBeing own: `BeingAppearance` — see SGWPlayer table above)* | — |
| 27 | `onAggressionOverrideUpdate` | `INT8 aAggressionLevel` |
| 28 | `onAggressionOverrideCleared` | *(none)* |

**Do not reuse the SGWPlayer method_idx constants for SGWMob traffic past
index 26** — from 27 the two entity types diverge (SGWPlayer continues into
`Communicator`/`OrganizationMember`/etc.), and the numeric ranges collide
(SGWPlayer's `Communicator` interface is also indices 27-33). Always pair a
method index with the correct entity's `class_id` at the call site; see
`crate::mercury::method_idx::ON_AGGRESSION_OVERRIDE_UPDATE`/
`ON_AGGRESSION_OVERRIDE_CLEARED` in `crates/wire/src/mercury/mod.rs`.

Ghidra evidence: the client registers both handlers as a pair through
`MemberCallback<GameMob, Event_NetIn_onAggressionOverrideUpdate>` /
`...Cleared` at `0x00d31cd0`; the Update handler at `0x00d31bd0` reads the
`aAggressionLevel` INT8 argument and stores it at `GameMob + 0x16c`.

## SGWPet Client Method Dispatch Table

> **Added**: 2026-09-27 (pets campaign packet PT-E1). **Entity type**: SGWPet (`class_id = 0x05`).
> **Total methods**: 32 (indices 0-31), all under `IDBASE_NPC_DEFAULT` (62), so every method
> uses **direct** wire encoding: `msg_id = 0x80 + index`.
> Full evidence trail: [`docs/reverse-engineering/findings/pet-client-contract.md`](../reverse-engineering/findings/pet-client-contract.md).

SGWPet's inheritance chain is `SGWEntity → SGWSpawnableEntity → SGWBeing → SGWMob → SGWPet`.
`SGWPet.def` has no `<Implements>` block, so its own 3 `<ClientMethods>` are appended directly
after the SGWMob prefix documented above (indices 0-28):

| Index | Method | Args |
|-------|--------|------|
| 0-28 | *(SGWMob prefix — see the SGWMob table above)* | — |
| 29 | `onPetAbilityList` | `ARRAY<INT32> aAbilityList` |
| 30 | `onPetStanceList` | `ARRAY<INT8> aStanceList` |
| 31 | `onPetStanceUpdate` | `INT8 aStance` |

Direct encoding: `0x9D` (29), `0x9E` (30), `0x9F` (31).

The indices (29/30/31) rest on the `.def` parse order plus the BigWorld flattening rule (see
"BigWorld Flattening Rule" above), not on anything decoded from the handlers themselves.
Separately, Ghidra evidence confirms *handler identity*, not the index values: each handler's
internal property-list lookup key matches its `.def` `<ArgName>` string exactly —
`GamePet__OnPetAbilityListChanged` (`0x00d39eb0`) keys on `"aAbilityList"`,
`GamePet__OnPetStanceListChanged` (`0x00d3a070`) keys on `"aStanceList"`,
`GamePet__OnPetStanceUpdateChanged` (`0x00d3a260`) keys on `"aStance"` — i.e. this confirms
which method each decompiled handler implements, not that the flattening rule assigned it the
number claimed above.

## Derivation

Generated by parsing all `.def` files in `entities/defs/` following BigWorld's
`entity_description.cpp:parseInterface()` parse order. Verified against 14
empirically confirmed indices from the running server codebase.

Source script and verification anchors are in the commit that added this file.
