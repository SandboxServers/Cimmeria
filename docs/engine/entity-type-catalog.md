# Entity Type Catalog

Comprehensive reference for all 18 entity types and 18 interfaces in the Stargate Worlds entity system. Data sourced directly from the `.def` files in `entities/defs/` and `entities/defs/interfaces/`.

---

## 1. Entity Type Summary Table

| Entity | Parent | Interfaces | Properties | Cell Methods | Base Methods | Client Methods | Server Implementation |
|--------|--------|------------|------------|--------------|-------------|----------------|----------------------|
| Account | *(none)* | ClientCache | 2 | 0 | 7 | 4 | Cell+Base |
| SGWEntity | *(none)* | DistributionGroupMember, EventParticipant | 5 | 11 | 2 | 0 | Cell+Base |
| SGWSpawnableEntity | SGWEntity | *(none)* | 10 | 12 | 0 | 13 | Cell+Base |
| SGWBeing | SGWSpawnableEntity | SGWBeing, SGWAbilityManager, SGWCombatant | 0 | 0 | 0 | 1 | Cell+Base |
| SGWPlayer | SGWBeing | Communicator, OrganizationMember, MinigamePlayer, GateTravel, SGWInventoryManager, SGWMailManager, Missionary, SGWPoller, ContactListManager, SGWBlackMarketManager, ClientCache | 68 | 96 | 20 | 59 | Cell+Base |
| SGWGmPlayer | SGWPlayer | *(none)* | 2 | 93 | 11 | 6 | Cell+Base |
| SGWMob | SGWBeing | Lootable | 68 | 40 | 0 | 2 | Cell+Base |
| SGWPet | SGWMob | *(none)* | 14 | 9 | 0 | 3 | Cell+Base |
| SGWDuelMarker | SGWSpawnableEntity | *(none)* | 2 | 1 | 0 | 0 | Cell+Base |
| SGWBlackMarket | *(none)* | *(none)* | 1 | 0 | 7 | 0 | Cell+Base |
| SGWPlayerGroupAuthority | SGWEntity | GroupAuthority | 0 | 0 | 0 | 0 | Cell+Base |
| SGWSpaceCreator | SGWEntity | *(none)* | 10 | 3 | 24 | 0 | Cell+Base |
| SGWSpawnRegion | SGWEntity | *(none)* | 27 | 0 | 21 | 0 | Cell+Base |
| SGWSpawnSet | SGWEntity | GroupAuthority | 24 | 0 | 6 | 0 | Cell+Base |
| SGWPlayerRespawner | SGWEntity | *(none)* | 4 | 1 | 0 | 0 | Cell+Base |
| SGWCoverSet | SGWEntity | *(none)* | 6 | 2 | 0 | 0 | Cell+Base |
| SGWEscrow | SGWEntity | *(none)* | 2 | 3 | 0 | 0 | Cell+Base |
| SGWChannelManager | *(none)* | *(none)* | 0 | 0 | 14 | 0 | Base only |

**Notes:**
- Property, method, and interface counts are for the entity's **own** `.def` file only (not inherited).
- All entities except SGWChannelManager have both `python/cell/` and `python/base/` implementations.
- SGWChannelManager has a base implementation only (`python/base/SGWChannelManager.py`), with `python/cell/SGWChannelManager.py` also present but functionally server-only (no cell methods defined).

---

## 2. Entity Hierarchy Diagram

```
Account (standalone)
    Implements: ClientCache

SGWBlackMarket (standalone, server-only)

SGWChannelManager (standalone, server-only)

SGWEntity (server-only)
    Implements: DistributionGroupMember, EventParticipant
    |
    +-- SGWPlayerGroupAuthority (server-only)
    |       Implements: GroupAuthority
    |
    +-- SGWSpaceCreator (server-only)
    |
    +-- SGWSpawnRegion (server-only)
    |
    +-- SGWSpawnSet (server-only)
    |       Implements: GroupAuthority
    |
    +-- SGWPlayerRespawner (server-only)
    |
    +-- SGWCoverSet (server-only)
    |
    +-- SGWEscrow (server-only)
    |
    +-- SGWSpawnableEntity
            |
            +-- SGWDuelMarker
            |
            +-- SGWBeing
                    Implements: SGWBeing, SGWAbilityManager, SGWCombatant
                    |
                    +-- SGWMob
                    |       Implements: Lootable
                    |       |
                    |       +-- SGWPet
                    |
                    +-- SGWPlayer
                            Implements: Communicator, OrganizationMember,
                                MinigamePlayer, GateTravel, SGWInventoryManager,
                                SGWMailManager, Missionary, SGWPoller,
                                ContactListManager, SGWBlackMarketManager,
                                ClientCache
                            |
                            +-- SGWGmPlayer
```

---

## 3. Interface Summary Table

| Interface | Properties | Cell Methods | Base Methods | Client Methods | Used By |
|-----------|-----------|--------------|-------------|----------------|---------|
| ClientCache | 0 | 0 | 2 | 2 | Account, SGWPlayer |
| Communicator | 4 | 1 | 16 | 7 | SGWPlayer |
| ContactListManager | 1 | 6 | 2 | 5 | SGWPlayer |
| DistributionGroupMember | 3 | 5 | 0 | 0 | SGWEntity |
| EventParticipant | 2 | 1 | 0 | 0 | SGWEntity |
| GateTravel | 5 | 5 | 2 | 4 | SGWPlayer |
| GroupAuthority | 3 | 0 | 5 | 0 | SGWPlayerGroupAuthority, SGWSpawnSet |
| Lootable | 4 | 5 | 0 | 0 | SGWMob |
| MinigamePlayer | 28 | 23 | 18 | 14 | SGWPlayer |
| Missionary | 8 | 14 | 0 | 5 | SGWPlayer |
| OrganizationMember | 7 | 25 | 4 | 17 | SGWPlayer |
| SGWAbilityManager | 24 | 12 | 0 | 0 | SGWBeing |
| SGWBeing | 27 | 16 | 3 | 9 | SGWBeing |
| SGWBlackMarketManager | 1 | 7 | 6 | 6 | SGWPlayer |
| SGWCombatant | 22 | 11 | 0 | 4 | SGWBeing |
| SGWInventoryManager | 14 | 10 | 0 | 7 | SGWPlayer |
| SGWMailManager | 4 | 9 | 1 | 4 | SGWPlayer |
| SGWPoller | 2 | 0 | 0 | 0 | SGWPlayer |

---

## 4. Per-Entity Detail Sections

### Account

**Parent:** *(none)* | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** ClientCache

**Properties:**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| characterList | CharacterInfoList | BASE | [] |
| activePlayerID | INT32 | BASE | 0 |

**Client Methods:**
- `onCharacterList(CharacterInfoList CharacterList)`
- `onCharacterCreateFailed(INT32 ErrorID)`
- `onCharacterVisuals(INT32 PlayerId, WSTRING BodySet, ARRAY<WSTRING> Components, UINT32 primaryTint, UINT32 secondaryTint, UINT32 skinTint)`
- `onCharacterLoadFailed()`

**Base Methods:**
- `logOff()` [Exposed]
- `logOffInternal()`
- `createCharacter(WSTRING Name, WSTRING ExtraName, INT32 CharDefId, ARRAY<VisualChoices> VisualChoiceList, INT32 SkinTintColorID)` [Exposed]
- `playCharacter(INT32 PlayerId)` [Exposed]
- `deleteCharacter(INT32 PlayerId)` [Exposed]
- `requestCharacterVisuals(INT32 PlayerId)` [Exposed]
- `onPlayerFailedToLoad()`
- `onClientVersion(INT32 VersionSeed, WSTRING ClientVersion, INT32 LanguageId)` [Exposed]

**Cell Methods:** *(none)*

---

### SGWEntity

**Parent:** *(none)* | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** DistributionGroupMember, EventParticipant

**Volatile:** position, yaw, pitch, roll

**Properties:**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| dbID | DBID | CELL_PRIVATE | 0 |
| nextRequestID | INT32 | CELL_PRIVATE | 1 |
| pendingRequests | PYTHON | CELL_PRIVATE | {} |
| timers | PYTHON | CELL_PRIVATE | {} |
| createOnCell | MAILBOX | BASE | *(none)* |

**Cell Methods:**
- `DespawnMe()`
- `onDatabaseResponse(PYTHON)`
- `onShutdown(INT8, INT8)`
- `teleportTo(MAILBOX, VECTOR3, VECTOR3)`
- `onTargeted(INT32, MAILBOX)`
- `onWitnessAdded(MAILBOX, INT8)`
- `getInfo(PYTHON, PYTHON)`
- `logEvent(INT32, INT32, INT8, WSTRING, WSTRING, WSTRING, WSTRING, WSTRING)`
- `logPetition(WSTRING)`
- `logChatSent(UINT8, WSTRING)`
- `logChatRecievedPrivate(PYTHON, WSTRING)`
- `requestRunActions(PYTHON, PYTHON, INT32, PYTHON)`

**Base Methods:**
- `DespawnMe()`
- `DespawnWhenFree()`

---

### SGWSpawnableEntity

**Parent:** SGWEntity | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** *(none)*

**Properties:**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| kismetEventSetId | INT32 | CELL_PUBLIC | 0 |
| SpawnSetID | INT32 | BASE | *(none)* |
| staticMeshName | WSTRING | CELL_PUBLIC | *(none)* |
| bodySet | WSTRING | CELL_PUBLIC | *(none)* |
| mobId | INT32 | CELL_PUBLIC | 0 |
| baseMobId | INT32 | BASE | 0 |
| minigamePlayers | ARRAY<INT32> | CELL_PRIVATE | *(none)* |
| entityVariables | PYTHON | CELL_PRIVATE | {} |
| interactDebug | INT8 | CELL_PRIVATE | 0 |
| shouldSendKismet | INT8 | CELL_PRIVATE | 1 |
| craftingStationControllerID | CONTROLLER_ID | CELL_PRIVATE | 0 |
| spaceCreatorMailbox | MAILBOX | CELL_PRIVATE | *(none)* |

**Client Methods:**
- `onStaticMeshNameUpdate(WSTRING StaticMeshName, WSTRING BodySetName)`
- `onSequence(INT32 KismetEventSetSeqID, INT32 SourceID, INT32 TargetID, INT8 PrimaryTarget, FLOAT ImpactTime, ARRAY<NameValuePair> NameValuePairs, INT8 ViewType, INT32 InstanceId)`
- `onEntityMove(FLOAT locationX, ..locationY, ..locationZ, ..velocityX, ..velocityY, ..velocityZ, ..yaw, ..pitch, ..roll)`
- `InteractionType(UINT64 TypeId)`
- `onEntityFlags(UINT64 aFlags)`
- `getInteractions(MAILBOX aEntity)`
- `toggleInteractionDebugging(INT32)`
- `onEntityProperty(INT32 type, INT32 value)`
- `onVisible(INT8 visible)`
- `onKismetEventSetUpdate(INT32 kismetEventSetId)`
- `onEntityTint(UINT32 primaryColorId, UINT32 secondaryColorId, UINT32 skinColorId)`
- `onBeingNameIDUpdate(INT32 BeingNameID)`

**Cell Methods:**
- `updateSpaceCreatorMailbox(MAILBOX)`
- `PlayAllClientKismetSeq(INT32, INT32, INT32, INT32)`
- `onInitialResponse(INT32, MAILBOX, INT32, PYTHON, INT32)`
- `onButtonSelection(INT32, MAILBOX, INT32, INT32)`
- `onInteract(INT32, MAILBOX, UINT8)`
- `getInteractionType(MAILBOX)`
- `addMinigamePlayer(INT32 PlayerId, UINT8 SingleInstance)`
- `remMinigamePlayer(INT32 PlayerId)`
- `getMinigamePlayers(MAILBOX playerMailbox)`
- `sendPlayerFeedback(WSTRING)`
- `sendPlayerFeedbackToken(WSTRING, ARRAY<StringToken> tokenList)`

---

### SGWBeing

**Parent:** SGWSpawnableEntity | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** SGWBeing, SGWAbilityManager, SGWCombatant

This entity defines no properties or cell/base methods of its own. It serves as an aggregation point for its three interfaces (SGWBeing, SGWAbilityManager, SGWCombatant), which collectively add 73 properties and 39 cell methods.

**Client Methods (own):**
- `BeingAppearance(WSTRING BodySet, ARRAY<WSTRING> ComponentList)`

---

### SGWPlayer

**Parent:** SGWBeing | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** Communicator, OrganizationMember, MinigamePlayer, GateTravel, SGWInventoryManager, SGWMailManager, Missionary, SGWPoller, ContactListManager, SGWBlackMarketManager, ClientCache (11 total)

SGWPlayer is the largest entity type. The full property and method lists are summarized by category below.

**Properties (68 own) by category:**

*Identity & Naming (3):*
playerName (WSTRING, CELL_PUBLIC, Persistent, Identifier), extraName (WSTRING, CELL_PUBLIC), account (MAILBOX, BASE)

*Location & World (6):*
areaName, areaKey (WSTRING, CELL_PRIVATE), worldID (INT32, CELL_PUBLIC), spaceCreatorID (INT32, CELL_PRIVATE), worldinstance (PYTHON, CELL_PRIVATE), worldinstanceMapResetDate (FLOAT, CELL_PRIVATE)

*Progression (5):*
experience (INT32), trainingPoints (INT32), accessLevel (INT32), numRespecAbility (INT32), numRespecCrafting (INT32) -- all CELL_PRIVATE

*Abilities (4):*
knownAbilities (ARRAY<INT32>, CELL_PUBLIC), knownPetAbilities (ARRAY<INT32>, CELL_PUBLIC), queuedAbility (INT32, CELL_PRIVATE), playerAvatarSetID (INT32, CELL_PRIVATE)

*PvP & Dueling (8):*
pvpFlag (INT8, CELL_PUBLIC), duelState (INT8, CELL_PUBLIC), duelEntities (ARRAY<MAILBOX>, CELL_PUBLIC), duelEntitiesDefeated (ARRAY<INT32>, CELL_PRIVATE), duelMarker (MAILBOX), duelChallengeTimer (CONTROLLER_ID), pvpTimer (CONTROLLER_ID), pendingPlayerFlags (PYTHON)

*Timers & Controllers (7):*
respawnTimerID, unstuckTimerID, interactionTimer, logoutTimer, autoCycleTimerID, loadTimerID, craftTimer -- all CELL_PRIVATE

*Time Tracking (5):*
currentLoginTime, totalTimePlayed, timeLastLevelled, timeSpentThisLevel, lastNoiseTime -- all FLOAT, CELL_PRIVATE

*Crafting (4):*
craftingEntityFlags (PYTHON), craftingOptions (PYTHON), craftQueue (INT32) -- all CELL_PRIVATE

*UI/System (8):*
designerFlags (STRING, CELL_PUBLIC), playerReadyFlags, systemOptions, currentWaypoints, regions, availableDialogs -- all CELL_PRIVATE, plus clientVersion (WSTRING), languageId (INT32)

*Misc State (18):*
Various flags and state tracking (bXPOff, bInteractDebug, hasWitness, interactionList, isBankingOverride, etc.)

**Cell Methods (96 own):**
Major categories: Combat (useAbility, callForAid, etc.), Interaction/Dialog (interact, dialogButtonChoice, initialResponse), Shopping (purchaseItems, sellItems, buybackItems, repairItems, rechargeItems), Abilities (trainAbility, resetMyAbilities, clearAbilities), Crafting (craft, research, reverseEngineer, alloying), Pet Commands (petInvokeAbility, petAbilityToggle, petChangeStance), Dueling (sendDuelChallenge, sendDuelResponse, duelForfeit), Trade (tradeRequest, tradeUpdateProposal, tradeLockState), Movement (respawn, unstuck, onTeleportStart/Finished), Stargate (operateGateLoc, setGateAddressLoc), Misc GM relay commands, and more.

**Base Methods (20 own):**
logOff [Exposed], cancelLogOff [Exposed], onLogOffTimer, onSpaceReady, loadSpace, onPlayerReady, onPlayerFailed, onClientReady [Exposed], onSpaceEntered, sendRespawnersToPlayer, sendPlayerToRespawner, showIP, sendDuelChallenge [Exposed], space queue methods (3), perfStats [Exposed], onPerfStatsByChannel, onSpaceQueueUpdate, setClientVersion

**Client Methods (59 own):**
Major categories: HUD/UI (onStoreOpen/Close/Update, onVaultOpen, onTrainerOpen, onLootDisplay), Ability Updates (onKnownAbilitiesUpdate, giveAbility, giveXPForLevel), Map/World (onClientMapLoad, setupWorldParameters, onMapInfo, onResetMapInfo), Mission (onMissionRewardsDisplay, onMissionOfferDisplay), Combat (onBeginAidWait, onEndAidWait), Stargate (onDHDReply, onDisplayDHD, stargateTriggerFailed), Progression (onExpUpdate, onMaxExpUpdate, onUpdateDiscipline), Dueling/Trade (onDuelChallenge, onTradeState, onTradeResults), Remote Entity streaming (onRemoteEntityCreate/Move/Remove), Movie system (onPlayMovie, onCancelMovie)

---

### SGWGmPlayer

**Parent:** SGWPlayer | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** *(none)*

SGWGmPlayer adds GM/debug commands on top of the full SGWPlayer entity.

**Properties (2 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| bCombatLOS | UINT8 | CELL_PRIVATE | 0 |
| bHideGM | UINT8 | CELL_PRIVATE | 0 |

**Cell Methods (93 own):** All [Exposed] (callable from client). Organized by category:

*Mission Commands (11):* gmMissionAssign, gmMissionClear, gmMissionClearActive, gmMissionClearHistory, gmMissionList, gmMissionListFull, gmMissionDetails, gmMissionAdvance, gmMissionReset, gmMissionComplete, gmMissionSetAvailable, gmMissionAbandon

*Show/Info Commands (10):* gmShowTargetLocation, gmShowRotation, listAbilities, showPointSet, gmShowFlag, gmListInteractions, gmGetMobAttribute, gmShowMobCount, gmShowIP, gmShowInventory, gmShowPlayer

*Give/Remove Commands (10):* gmGiveXp, gmGiveItem, gmGiveCash, gmRemoveItem, gmGiveAbility, gmGiveTrainingPoints, gmGiveRespawner, gmGiveExpertise, gmGiveAppliedSciencePoints, gmGiveRacialParadigmLevels

*Set Commands (14):* gmSetGodMode, gmSetNoXP, gmSetNoDamage, gmSetNoAggro, gmSetSpeed, gmSetHealth, gmSetHealthMax, gmSetFocus, gmSetFocusMax, gmSetFlag, gmSetLevel, gmSetTarget, gmSetMobStance, gmSetMobAbilitySet, gmResetAbilities, gmGiveAllAbilities, gmRespec

*Travel Commands (4):* gmDHD, gmGoto, gmSummon, gmGotoLocation, gmGotoXYZ

*Reload Commands (9):* loadConstants, loadAbility, loadNACSI, loadAbilitySet, loadBehavior, loadMOB, loadDialogSet, loadItem, loadMission, gmReloadOrganizations, gmReloadInventory

*Debug Commands (17):* gmDebugAbility, gmDebugCombat, gmDebugCombatVerbose, gmDebugHeal, gmDebugStartMinigame, gmDebugSpectateMinigame, gmDebugJoinMinigame, gmDebugAbilityOnMob, gmDebugBehaviorsOnMob, gmDebugPathsOnMob, gmDebugEvents, gmDebugMobData, gmDebugInteract, gmEmitBehaviorEventOnMob, gmAddBehaviorEventSet, gmRemoveBehaviorEventSet

*Spawn/Despawn (4):* gmSpawnByCmd, gmDespawnByCmd, despawnMob, activateSpawnSet, deactivateSpawnSet

*Misc (14):* testLOS, toggleCombatLOS, trackMob, onXRayEyes, onInvisible, onPhysics, sendGMShout, regenerateCoverLinks, changeCoverWeight, changeCoverStanceWeight, spawnEntityLoot, gmSetCallback, gmPrintStats, gmShowNavigation, setMobVariable, enterErrorAIState, exitErrorAIState, gmPerfStatsByChannel, gmShowInstanceFlag, gmSetInstanceFlag

**Client Methods (6 own):**
- `onLOSResult(VECTOR3 aStart, VECTOR3 aEnd, INT8 aClear)`
- `onShowWaypoints(WSTRING pointSetName, FLOAT radius, WaypointList wayPoints)`
- `onShowPath(INT32 aEntityId, UINT8 aMovementType, ARRAY<VECTOR3> aPath)`
- `onDisableShowPath(INT32 aEntityId)`
- `onSetTarget(INT32 aEntityId)`
- `onShowNavigation(WSTRING aChunkName, ARRAY<NavigationPolygon> aNavPolyList)`

**Base Methods (11 own):**
spawnByCmd, sendGMShout, loadConstants, gmSummon, gmGoto, gmShowMobCount, gmUsers, gmSetHideGM

---

### SGWMob

**Parent:** SGWBeing | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** Lootable

SGWMob is the NPC/monster entity. Its 68 properties handle AI state, navigation, combat behavior, threat tracking, and spawning.

**Properties (68 own) by category:**

*Navigation (6):* navControllerID, visionID, yawID (CONTROLLER_ID), Home, POI (VECTOR3), lastNavigate (PYTHON)

*AI State (12):* AIState (INT32, CELL_PUBLIC), stateLock, errorAIState, errorStateReason, errorStateDescription, stateChanges, navigateFailures, lastErrorStateCheck, errorTime, stateHistory, disableBehaviorSystem, lastCombatAction

*Combat (12):* CombatStance, priorStance, MyAbilitySetID (CELL_PUBLIC), Aggression (CELL_PUBLIC), minIdealRange, maxIdealRange, bCoverFromTarget, lastCoverMove, lastCoverCheck, cullLOSBlockers, abilitySelection, nextAbility

*Threat (9):* threatList, entityDamageList, squadDamageList, totalDamageTaken, tappedEntity (CELL_PUBLIC), tappedSquad (CELL_PUBLIC), tappedSquadMembers (CELL_PUBLIC), threatXferMultpr, threatSystemEnabled, threatenedMobsWhenDisabled

*Identity (4):* SecondaryDisplayName (WSTRING, CELL_PUBLIC), raceId (CELL_PUBLIC), mobTypeId (CELL_PUBLIC), roleId, nameIDOverride

*Spawning (6):* spawner (MAILBOX), spawnTime (CELL_PUBLIC), spawnPointId, despawnTimerID, despawnFlag, activationTime

*Movement/Follow (7):* currentlyFollowing, followTarget, followMinDistance, followMaxDistance, followAngle, followMovementType, patrolMovementType

*Behavior (7):* mobBehaviorEventSet, behaviorTimerID, behaviorDebug, investigateTimerID, mobGroup, lastPatrolHomeUpdate, patrolPaths, currentPatrolPath

*Debug (5):* bFeedbackDebug, feedbackDebugTarget, feedbackDebugPreviousState, pathDebug

*Misc (6):* LootTableID, hearingRadius, intelligenceId, useCover, isKillable, isTrackable, isWorthXP, reservedCoverNode, costObjects, lastCostObjectsUpdate, grenadeDetectorID, targetOverride, targetOverrideTimer, decayTimerID, aggressionOverrides (CELL_PUBLIC), aggressionOverrideTimers, nextWanderTime, _targetSelectionTickCount, _myLastPos, _lostSight, _targetLastPos, _targetThreshold, _acted, trackControllerID

**Cell Methods (40 own):**
Threat management (onGroupMateEnteredCombat, onGroupMateThreatTransfer, addDirectToThreatList, addBuffToThreatList, addHealToThreatList, addToThreatList, removeFromThreatList, addToDamageTracking, clearDamageTracking), Loot (onSquadLooterResponse, onDropAdditionalLoot, onDropAdditionalItem), State control (reqState, removeStateLock, enterErrorAIState, leaveErrorAIState), Behavior (overrideAggression, onOverrideAggressionToTappedEntities, addBehaviorSet, removeBehaviorSet, onNoise, mobJoinGroup), Navigation (setYawRotation, chooseDialogResponse), Cover (onReserveCoverSlot, changeCoverWeight), Debug (onGatherInformation, onGetAttribute, onSetAttribute, onTrackData, toggleBehaviorDebugging, togglePathDebugging), Despawn (endDespawnTimer, DespawnWhenFree, onTappedSquadUpdate), Misc (changeAbilitySet, changeStance, setNextAbility, reevaluateEntityHostility)

**Client Methods (2 own):**
- `onAggressionOverrideUpdate(INT8 aAggressionLevel)`
- `onAggressionOverrideCleared()`

---

### SGWPet

**Parent:** SGWMob | **Server-Only:** No | **Implementation:** Cell+Base

**Interfaces:** *(none)* (inherits Lootable from SGWMob)

**Properties (14 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| ownerID | INT32 | CELL_PUBLIC | 0 |
| ownerBase | MAILBOX | CELL_PUBLIC | *(none)* |
| transferXP | FLOAT | CELL_PRIVATE | 1.0 |
| petDespawnTimerId | CONTROLLER_ID | CELL_PRIVATE | 0 |
| abilityToResolve | INT32 | CELL_PRIVATE | 0 |
| abilityInformation | PYTHON | CELL_PRIVATE | {} |
| toggledAbilities | ARRAY<INT32> | CELL_PRIVATE | *(none)* |
| lastOwnerPositionCheck | FLOAT | CELL_PRIVATE | 0 |
| lastTeleportTime | FLOAT | CELL_PRIVATE | 0 |
| ownerLastPosition | VECTOR3 | CELL_PRIVATE | *(none)* |
| petLastPosition | VECTOR3 | CELL_PRIVATE | *(none)* |
| petStance | INT8 | CELL_PRIVATE | 1 |

**Cell Methods (9):**
onOwnerDeath, onOwnerLeash, onOwnerRespawn(INT8), saveToDB(INT32), toggleAbility(INT32, INT8), changePetStance(INT8), setPetLevel(INT8), sendPetInfoToOwner(MAILBOX, ARRAY<INT32>)

**Client Methods (3):**
- `onPetAbilityList(ARRAY<INT32> aAbilityList)`
- `onPetStanceList(ARRAY<INT8> aStanceList)`
- `onPetStanceUpdate(INT8 aStance)`

---

### SGWDuelMarker

**Parent:** SGWSpawnableEntity | **Server-Only:** No | **Implementation:** Cell+Base

**Properties (2 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| duelDetectorID | CONTROLLER_ID | CELL_PRIVATE | 0 |
| duelEntities | ARRAY<MAILBOX> | CELL_PRIVATE | *(none)* |

**Cell Methods (1):**
- `onEntityDefeated(INT32)` -- entity id of defeated entity

---

### SGWBlackMarket

**Parent:** *(none)* | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** *(none)*

**Properties (1 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| watchedItems | PYTHON | BASE | *(none)* |

**Base Methods (7):**
searchBlackMarket(MAILBOX, INT32, BMSearchOptions, INT32), placeBid(INT32, INT32), createAuction(MAILBOX, INT32, INT32, INT32, UINT8, INT32), cancelAuction(MAILBOX, INT32, INT32), registerWatchedItems(ARRAY<INT32>, MAILBOX), unregisterWatchedItems(ARRAY<INT32>, MAILBOX)

---

### SGWPlayerGroupAuthority

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** GroupAuthority

No additional properties or methods of its own. All functionality comes from the GroupAuthority interface.

---

### SGWSpaceCreator

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** *(none)*

**Properties (10 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| areaName | WSTRING | CELL_PUBLIC | *(none)* |
| areaKey | WSTRING | CELL_PUBLIC | *(none)* (Persistent, Identifier) |
| worldID | UINT32 | CELL_PUBLIC | *(none)* |
| expirationTimestamp | STRING | CELL_PRIVATE | *(none)* |
| loadData | PYTHON | CELL_PRIVATE | {} |
| worldinstanceID | UINT32 | CELL_PUBLIC | *(none)* |
| worldinstanceFlags | UINT32 | CELL_PRIVATE | *(none)* |
| CurrentPopulation | PYTHON | BASE | {} |
| MySpawnRegionEntityIDs | ARRAY<INT32> | BASE | *(none)* |

**Cell Methods (3):** tearDownSpace, setWorldinstanceFlag(UINT32, UINT8), extendExpiration(UINT32)

**Base Methods (24):** requestCell, onPlayerEnter, onPlayerExit, registerPlayerRespawner, deregisterPlayerRespawner, registerSpawnRegion, deregisterSpawnRegion, sendRespawnersToPlayer, sendPlayerToRespawner, onStartLoadingEntities, entityEnteredRegion, entityExitRegion, isEntityInRegion, getEntitiesInRegion, reportPopulation, displayMobCount, activateSpawnRegion, deactivateSpawnRegion, activateSpawnSet, deactivateSpawnSet

---

### SGWSpawnRegion

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** *(none)*

**Properties (27 own):**
MySpawnSetIDs (ARRAY<INT32>, BASE), MySpawnSetEntityIDs (ARRAY<INT32>, BASE), RegionSpawnTimer (INT32, BASE, default 10), Activated (INT8, BASE, default 1), SpawnRegionID (UINT32, BASE), WorldID (INT32, BASE), MaxActiveSets (INT32, BASE), MinRespawnSeconds (INT32, BASE), MaxRespawnSeconds (INT32, BASE), SpawnSetMinCooldown (INT32, BASE), SpawnSetMaxCooldown (INT32, BASE), RespawnFlag (UINT8, BASE), EventSetId (INT32, BASE), lastTimeCheck (FLOAT, CELL_PRIVATE), lastWorldTime (FLOAT, CELL_PRIVATE), spaceCreatorBaseMB (MAILBOX, BASE), detectionRadius (FLOAT, CELL_PRIVATE), detectionController (CONTROLLER_ID, CELL_PRIVATE), entitiesInRegion (PYTHON, BASE), missionData (PYTHON, BASE), missionEventTimer (CONTROLLER_ID, BASE), populationReportTimer (INT32, BASE), CurrentPopulation (PYTHON, BASE), LastReportedPopulation (UINT32, BASE), mobDeaths (UINT32, BASE), timerReduction (FLOAT, BASE), respawnTimers (PYTHON, BASE)

**Base Methods (21):** RegisterSpawnSetBase, ActivateMe, DeactivateMe, deactivateSet, activateSet, StartRespawnTimer, onTimeOfDayTick, onSpawnSetActivate, onSpawnSetDeactivate, onEntityEnter, onEntityExit, entityMissionStepUpdate, entityMissionComplete, reportPopulation, onMobDeath

---

### SGWSpawnSet

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Interfaces:** GroupAuthority

**Properties (24 own):**
SpawnSetID (UINT32, BASE), MySpawnPoints (PYTHON, BASE), SpawnPointLastUsed (ARRAY<FLOAT>, BASE), MyMobBaseIDList (PYTHON, BASE), CurrentPopulation (UINT32, BASE), LastReportedPopulation (UINT32, BASE), Activated (INT8, BASE), bRandomizeSpawnPoints (INT8, BASE), mySpawnRegionID (INT32, BASE), bLinked (INT8, BASE), lastCooldownStart (INT32, BASE), cooldownSeconds (INT32, BASE), minCooldownSeconds (INT32, BASE), maxCooldownSeconds (INT32, BASE), currentSpawnTableID (INT32, BASE), spawnTableIDs (PYTHON, BASE), minMOBLevel (INT32, BASE), maxMOBLevel (INT32, BASE), deadMOBs (ARRAY<INT32>, BASE), spawnPointReservation (PYTHON, BASE), cooldownTimer (INT32, BASE), populationReportTimer (INT32, BASE), spaceCreatorBaseMB (MAILBOX, BASE)

**Base Methods (6):** RegisterMobBase(INT32, INT32), MobFailedToSpawn(INT32), BeingDeath(INT32), BeingDespawn(INT32), ActivateMe, DeactivateMe(UINT8)

---

### SGWPlayerRespawner

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Properties (4 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| spaceCreatorBaseMB | MAILBOX | BASE | *(none)* |
| mobID | INT32 | CELL_PRIVATE | 0 |
| visibilityID | INT32 | CELL_PRIVATE | 0 |
| playerRespawnerDetection | INT32 | CELL_PRIVATE | 0 |

**Cell Methods (1):** registerPlayerRespawner(MAILBOX aSpaceCreatorBaseMB, INT32 aRespawnerMobID)

---

### SGWCoverSet

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Properties (6 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| chunkName | STRING | CELL_PRIVATE | *(none)* |
| chunkID | UINT32 | CELL_PUBLIC | 0 |
| proximityID | CONTROLLER_ID | CELL_PRIVATE | 0 |
| reservedCoverNodes | PYTHON | CELL_PRIVATE | {} |
| publicReservationData | ARRAY<PublicCoverNodeReservationData> | CELL_PUBLIC | *(none)* |
| proximityRetries | INT8 | CELL_PRIVATE | 10 |

**Cell Methods (2):** reserveCoverSlot(INT32, INT32, INT8), releaseCoverSlot(INT32, INT32, INT8)

---

### SGWEscrow

**Parent:** SGWEntity | **Server-Only:** Yes | **Implementation:** Cell+Base

**Properties (2 own):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| escrowRecordList | EscrowRecordList | CELL_PRIVATE | [] |
| escrowNumber | INT32 | CELL_PRIVATE | 0 |

**Cell Methods (3):**
- `lootItemTransfer(INT32 FromID, INT32 ToID, LootItemDefinition ItemToTransfer)`
- `lootItemTransferCallback(INT32 aLooterID, INT32 aQuantity, INT32 aEscrowNumber)`

---

### SGWChannelManager

**Parent:** *(none)* | **Server-Only:** Yes | **Implementation:** Base only

**Interfaces:** *(none)*

No properties. Chat channel management is handled entirely through base methods.

**Base Methods (14):**
addChannelManager(UINT32, MAILBOX), joinGlobal(WSTRING, WSTRING, MAILBOX), join(WSTRING, WSTRING, WSTRING, MAILBOX), leaveAll(WSTRING), leave(WSTRING, WSTRING), speak(WSTRING, UINT8, WSTRING, WSTRING, WSTRING), announce(WSTRING, UINT8, WSTRING, WSTRING), list(WSTRING, WSTRING), mute(WSTRING, WSTRING, WSTRING), unmute(WSTRING, WSTRING, WSTRING), kick(WSTRING, WSTRING, WSTRING), ban(WSTRING, WSTRING, WSTRING), unban(WSTRING, WSTRING, WSTRING), op(WSTRING, WSTRING, WSTRING), password(WSTRING, WSTRING, WSTRING)

---

## 5. Per-Interface Detail Sections

### ClientCache

**Used by:** Account, SGWPlayer

Handles cooked data caching -- the client requests version info for data categories and downloads updated elements as needed.

**Base Methods (2):**
- `versionInfoRequest(INT32 CategoryId, INT32 Version)` [Exposed]
- `elementDataRequest(INT32 CategoryId, INT32 Key)` [Exposed]

**Client Methods (2):**
- `onVersionInfo(INT32 CategoryId, INT32 Version, INT32 RequiredUpdates, INT8 InvalidateAll, ARRAY<INT32> InvalidKeys)`
- `onCookedDataError(INT32 categoryID, INT32 elementKey)`

---

### Communicator

**Used by:** SGWPlayer

Chat and communication system. Handles say, tell, yell, chat channels, ignoring, AFK/DND status, petitions, and GM shouts.

**Properties (4):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| ignoredList | ARRAY<WSTRING> | BASE | *(none)* |
| channels | ARRAY<PYTHON> | CELL_PRIVATE | *(none)* |
| AFK | UINT8 | BASE | *(none)* |
| DND | UINT8 | BASE | *(none)* |

**Cell Methods (1):** processPlayerCommunication(WSTRING, UINT8, WSTRING, UINT8, WSTRING)

**Base Methods (16):**
chatJoin [Exposed], chatJoinGlobal, chatJoinGlobalRaw, chatLeave [Exposed], sendPlayerCommunication [Exposed], chatSetAFKMessage [Exposed], chatSetDNDMessage [Exposed], chatIgnore [Exposed], chatFriend [Exposed], chatList [Exposed], chatMute [Exposed], chatKick [Exposed], chatOp [Exposed], chatBan [Exposed], chatPassword [Exposed], petition [Exposed], announcePetition [Exposed], sendCommunicationToClient, sendYellToClients, hearGMShout, commTell

**Client Methods (7):**
onSystemCommunication, onPlayerCommunication, onLocalizedCommunication, onTellSent, onChatJoined, onChatLeft, onNickChanged

---

### ContactListManager

**Used by:** SGWPlayer

Player contact list management (friends lists, custom lists). Supports creating, renaming, adding/removing members, and event notifications.

**Properties (1):** contactLists (PYTHON, CELL_PRIVATE, default {})

**Cell Methods (6):** contactListCreate [Exposed], contactListDelete [Exposed], contactListRename [Exposed], contactListFlagsUpdate [Exposed], contactListAddMembers [Exposed], contactListRemoveMembers [Exposed]

**Base Methods (2):** sendEventToPlayers, sendLoginStatusMessages

**Client Methods (5):** onContactListUpdate, onContactListDelete, onContactListAddMembers, onContactListRemoveMembers, onContactListEvent

---

### DistributionGroupMember

**Used by:** SGWEntity

Manages entity membership in distribution groups (squads, raid groups, etc.). Groups are coordinated by a GroupAuthority entity.

**Properties (3):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| groups | PYTHON | CELL_PRIVATE | {} |
| groupInfoUpdateTimers | PYTHON | CELL_PRIVATE | {} |
| pendingJoin | PYTHON | CELL_PRIVATE | {} |

**Cell Methods (5):** remoteJoinGroup, onGroupJoined, onGroupJoinFail, onGroupMemberJoined, onGroupMemberLeft, callMethodOnGroup

---

### EventParticipant

**Used by:** SGWEntity

Enables entities to subscribe to and receive gameplay events across cell boundaries.

**Properties (2):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| baseEvents | PYTHON | BASE | *(none)* |
| cellEvents | PYTHON | CELL_PRIVATE | *(none)* |

**Cell Methods (1):** onEvent(INT32, PYTHON, PYTHON)

---

### GateTravel

**Used by:** SGWPlayer

Stargate travel system -- managing known gate addresses, dialing, and passage between worlds.

**Properties (5):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| knownStargateAddresses | ARRAY<PYTHON> | CELL_PRIVATE | [] |
| oldWorldID | INT32 | CELL_PRIVATE | 0 |
| gateCounter | INT32 | CELL_PRIVATE | 0 |
| destinationGate | INT32 | CELL_PRIVATE | 0 |
| destinationGateArrivalTime | FLOAT | CELL_PRIVATE | 0.0 |

**Cell Methods (5):** onDialGate(INT32, INT32) [Exposed], giveStargateAddressStr(WSTRING, UINT8), removeStargateAddressStr(WSTRING), closeGatesTo(UINT32), processGateTravel(PYTHON)

**Base Methods (2):** processSquadLeaderGateTravel(INT32, PYTHON), processGateTravel(PYTHON)

**Client Methods (4):** setupStargateInfo, updateStargateAddress, stargateRotationOverride, onStargatePassage

---

### GroupAuthority

**Used by:** SGWPlayerGroupAuthority, SGWSpawnSet

Central authority for managing distribution groups (squads, guilds, etc.). Handles group creation, joining, leaving, and cross-entity method invocation.

**Properties (3):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| authGroups | PYTHON | BASE | {} |
| authorityID | INT8 | BASE | -1 |
| lastTempID | INT32 | BASE | *(none)* |

**Base Methods (5):** joinGroup(WSTRING, PYTHON, MAILBOX, PYTHON, PYTHON), leaveGroup(PYTHON, MAILBOX, PYTHON, MAILBOX, INT8), leaveGroupByName(PYTHON, WSTRING, PYTHON, MAILBOX, INT8), callMethodOnGroup(INT32, STRING, PYTHON)

---

### Lootable

**Used by:** SGWMob

Provides loot inventory and looting mechanics for defeated NPCs/mobs.

**Properties (4):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| hasLoot | INT8 | CELL_PRIVATE | 0 |
| lootableItems | LootItemDefinitionList | CELL_PRIVATE | *(none)* |
| lootableCash | INT16 | CELL_PRIVATE | *(none)* |
| activeLooters | ARRAY<INT32> | CELL_PRIVATE | *(none)* |

**Cell Methods (5):** listLoot(INT32), lootItem(INT32, INT32), lootItemCallback(INT32, INT32), spawnLoot(LootTableID, INT8, INT8, ARRAY<INT32>, ARRAY<INT32>), spawnItem(INT32, INT32, INT8, INT8, ARRAY<INT32>, ARRAY<INT32>)

---

### MinigamePlayer

**Used by:** SGWPlayer

The largest interface by method count. Manages the archaeology/puzzle minigame system, including starting games, spectating, cooperative play, registration for help, contact lists, and calling other players for assistance.

**Properties (28):**
minigame (PYTHON), pendingInstance (INT32), pendingMinigamePosition (VECTOR3), pendingItem (INT32), pendingMob (INT32), pendingSeed (INT32), pendingTC (INT32), minigameMobAttemptTracker (PYTHON), minigameItemAttemptTracker (PYTHON), minigameRegistrationCost (INT32, default -1), minigameRegistered (UINT8), minigameRegisteredWantsRequests (UINT8), minigameRegisteredNote (WSTRING), minigameRegisteredRange (UINT8), minigameRegistrationAvailable (UINT8), pendingHelper (INT32), pendingHelperBase (MAILBOX), pendingHelperExpires (FLOAT), pendingHelperTip (INT32), pendingHelperTicket (STRING), pendingMinigameRequests (PYTHON), currentMinigameRequest (PYTHON), minigameCallTracker (PYTHON), minigameWaitingOnCash (PYTHON), minigameSavedTimeInfo (FLOAT), minigameSavedRegistrationInfo (STRING), minigameSavedRegistrationNote (WSTRING), minigameContacts (PYTHON), minigameRequestTimer (INT32), minigameNextNPCRequest (FLOAT), pendingContactList (PYTHON) -- all CELL_PRIVATE

**Cell Methods (23):** debugStartMinigame [Exposed], debugSpectateMinigame [Exposed], debugJoinMinigame [Exposed], debugMinigameInstance [Exposed], handleMinigameResults, handleStartMinigameFailed, handleMinigameSystemShutdown, joinMinigameRequest, joinMinigameResponse, joinMinigameError, startMinigame [Exposed], endCurrentMinigame [Exposed], processStartMinigameAction, processForceMinigameAction, consumeItemByMinigame, addMinigamePlayerSuccess, addMinigamePlayerFail, handleMinigamePlayerList, requestSpectateList [Exposed], spectateMinigame [Exposed], registerToMinigameHelp [Exposed], updateRegisterToMinigameHelp [Exposed], minigameStartCancel [Exposed], minigameCallRequest (cell), minigameCallRequestPhaseTwo (cell), minigameCallAccept [Exposed], minigameCallDecline [Exposed], minigameCallAcceptPhaseTwo (cell), minigameCallDeclinePhaseTwo (cell), minigameCallAbort [Exposed], minigameEndCall (cell), minigameCallAbortPhaseTwo (cell), giveMinigameContactStr (cell), removeMinigameContactStr (cell), minigameContactRequest [Exposed]

**Base Methods (18):** startMinigame, joinMinigame, endMinigameForPlayer, addItemToMinigame, remItemFromMinigame, consumeItemByMinigame, updateMinigameItemCheats, minigameLevelUpdate, updateMinigameRegistrationState, updateMinigameAvailableState, updateMinigameWantsHelpState, minigameCallRequest [Exposed], minigameCallRequestPhaseTwo, minigameCallAcceptPhaseTwo, minigameCallDeclinePhaseTwo, minigameEndCall, minigameCallAbortPhaseTwo, giveMinigameContactStr, removeMinigameContactStr

**Client Methods (14):** onStartMinigame, onStartMinigameDialog, onStartMinigameDialogClose, onEndMinigame, onSpectateList, onMinigameRegistrationPrompt, minigameRegistrationInfo, addOrUpdateMinigameHelper, removeMinigameHelper, minigameCallDisplay, minigameCallResult, minigameCallAbort, showMinigameContact

---

### Missionary

**Used by:** SGWPlayer

Mission/quest system. Tracks active missions, completed missions, mission loot tables, and provides GM mission manipulation methods.

**Properties (8):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| missions | PYTHON | CELL_PRIVATE | {} |
| publicMissionData | ARRAY<PYTHON> | CELL_PUBLIC | [] |
| completedMissions | ARRAY<PYTHON> | CELL_PUBLIC | [] |
| currentMissionLoot | PYTHON | CELL_PRIVATE | {} |
| bShowMissionDebug | INT8 | CELL_PRIVATE | 0 |
| missionProcessQueue | PYTHON | CELL_PRIVATE | {} |
| pendingMissionAccepts | ARRAY<INT32> | CELL_PRIVATE | *(none)* |
| pendingMissionShares | PYTHON | CELL_PRIVATE | {} |

**Cell Methods (14):** abandonMission [Exposed], shareMission [Exposed], shareMissionOffer, shareMissionResponse [Exposed], requestAbandonMission, missionAssign, missionClear, missionClearActive, missionClearHistory, missionList, missionListFull, missionDetails, missionAdvance, missionReset, missionComplete, missionSetAvailable, remoteMissionEvent

**Client Methods (5):** onMissionUpdate, onStepUpdate, onObjectiveUpdate, onTaskUpdate, offerSharedMission

---

### OrganizationMember

**Used by:** SGWPlayer

Guild, squad, and organization membership management. Handles invites, joins, leaves, rank changes, MOTD, strike teams, PvP flags, loot modes, cash transfers, and minimap pings.

**Properties (7):** records, squad (INT32, CELL_PUBLIC), strikeTeamTimers, pendingPvPTimers, pendingGroups, pendingJoins, pendingInvitesByType -- most PYTHON/CELL_PRIVATE

**Cell Methods (25):** onOrganizationJoined, organizationInvite, organizationInviteByType, organizationKick, organizationRankChange, onMemberRankChange, onOrganizationMOTDUpdate, onOrganizationNoteUpdate, onOrganizationOfficerNoteUpdate, onOrganizationRankPermissionsUpdate, onOrganizationSingleRankNameUpdate, onSquadLootModeUpdate, onOrganizationInvite, organizationInviteResponse [Exposed], organizationInviteResults, organizationInviteAccepted, organizationLeave [Exposed], onOrganizationMemberJoined, initialOrganizationMemberInfo, onOrganizationMemberInfoUpdate, BroadcastMinimapPing [Exposed], receivedMinimapPing, onStrikeTeamUpdate, strikeTeamResponse [Exposed], pvpOrganizationLeaveResponse [Exposed], onOrganizationHeaderUpdate, onOrganizationCashUpdate, orgUpdatePlayerCash, onOrganizationRankUpdate, onOrganizationRankNameUpdate, organizationMOTD [Exposed], organizationNote [Exposed], organizationOfficerNote [Exposed], organizationSetRankPermissions [Exposed], organizationSetRankName [Exposed], squadSetLootMode [Exposed], organizationTransferCash [Exposed]

**Base Methods (4):** organizationInvite [Exposed], organizationInviteByType [Exposed], organizationKick [Exposed], organizationRankChange [Exposed]

**Client Methods (17):** onOrganizationInvite, onOrganizationJoined, onOrganizationLeft, onMemberJoinedOrganization, onOrganizationRosterInfo, onMemberLeftOrganization, onMemberRankChangedOrganization, onStrikeTeamUpdate, onPvPOrganizationLeaveRequest, onOrganizationNameUpdate, onOrganizationExperienceUpdate, onOrganizationMOTDUpdate, onOrganizationNoteUpdate, onOrganizationOfficerNoteUpdate, onOrganizationCashUpdate, onOrganizationRankUpdate, onOrganizationRankNameUpdate, onSquadLootType

---

### SGWAbilityManager

**Used by:** SGWBeing

Manages abilities, effects, cooldowns, channeled abilities, warmup timers, and debug tools. All combat actions flow through this interface.

**Properties (24):**
warmupTimer, bIsWarmingUp, lastWarmUpInterruptTime, warmUpRuntimeParams, pulsedEffects, durationEffects, abilityAdjustments, abilityCooldowns, categoryCooldowns, bDmgOff, bGodMode (CELL_PUBLIC), bInfiniteAmmo (CELL_PUBLIC), bIgnoreHealth, bIgnoreFocus, bNoAggro (CELL_PUBLIC), bCombatDebug, bCombatVerboseDebug, debugAbilityList (ARRAY<INT32>), debugEffectList (ARRAY<INT32>), debugAbilityLevel, debugAbilityTargetID, channeledAbilityData, channeledData, lastChannelInterruptTime, effectComponents, effectMonikers (CELL_PUBLIC), effectSequenceId, diminishingReturns, immuneToEffects, pendingAbilities -- most CELL_PRIVATE

**Cell Methods (12):** onHealthZeroed, invokeAbility, resolveAbility, resolveEffect, onKillCredit, toggleCombatDebug [Exposed], toggleCombatVerboseDebug [Exposed], toggleAbilityDebugging, setAbilityDebugTarget, clearAbilityDebug, onStealthChanged, pulseChanneledEffectOnTarget, setNoAggro, clearEffectsByMoniker, confirmationResponse [Exposed], removeEffectById

---

### SGWBeing (interface)

**Used by:** SGWBeing (entity)

Core being properties: name, level, targeting, state, appearance (colors, components, disguises), pets, movement, vision, and detectors.

**Properties (27):**
beingName (WSTRING, CELL_PUBLIC, default "An Individual"), level (INT8, CELL_PUBLIC, default 1), visibilityID, targetID (INT32, CELL_PUBLIC), bStateField (INT32, CELL_PUBLIC), primaryColorId, secondaryColorId, skinColorId (UINT32, CELL_PRIVATE), currentComponentList (ARRAY<WSTRING>, CELL_PUBLIC), disguiseEnabled (INT8, CELL_PUBLIC), disguiseStaticMeshName, disguiseBodySet (WSTRING, CELL_PUBLIC), disguiseComponentList (ARRAY<WSTRING>, CELL_PUBLIC), disguiseFaction (INT8, CELL_PUBLIC), disguiseTimerId, disguiseReduction, disguiseVisionId (CELL_PRIVATE), petList (ARRAY<MAILBOX>, CELL_PRIVATE), movementType (UINT8, CELL_PRIVATE), detectors (PYTHON), visionChangeCallbacks (ARRAY<PYTHON>), visionExceptions (PYTHON), deathAbilityId (INT32)

**Cell Methods (16):** setTargetID [Exposed], onPetSpawn, onPetDeath, onPetDetection, setMovementType [Exposed], toggleStateField, setStateField, registerVisionChangeCallback, unregisterVisionChangeCallback, enableDisguise, enableDisguiseByDef, reduceDisguiseRating, stopMovement, restoreMovement, addVisionExceptions, removeVisionExceptions

**Base Methods (3):** onDeath, onDespawn, gmSummonCallback

**Client Methods (9):** onTimerUpdate, onEffectUserData, onEffectResults, onLevelUpdate, onTargetUpdate, onBeingNameUpdate, onTopSpeedUpdate, onStateFieldUpdate

---

### SGWBlackMarketManager

**Used by:** SGWPlayer

Client-side interface for the auction house (Black Market). Players search, create, bid on, and cancel auctions. Also supports watching for specific items.

**Properties (1):** watchedItems (ARRAY<INT32>, CELL_PRIVATE, default [])

**Cell Methods (7):** BMSearch [Exposed], BMCreateAuction [Exposed], BMPlaceBid [Exposed], BMCancelAuction [Exposed], BMStartWatchingItem [Exposed], BMStopWatchingItem [Exposed]

**Base Methods (6):** BMSearch, BMCreateAuction, BMPlaceBid, BMCancelAuction, BMStartWatchingItem, BMStopWatchingItem

**Client Methods (6):** onBMOpen, onBMError, onBMAuctions, onBMAuctionRemove, onBMAuctionUpdate, onBMWatchedItemsUpdate

---

### SGWCombatant

**Used by:** SGWBeing

Combat stats, threat management, stealth/detection, cover mechanics, health/focus regeneration, successive shot tracking, and stat dictionaries.

**Properties (22):**
entitiesDetectedStealth, stealthTimer, stealthDefaultDetector, revealDefaultDetector (CELL_PRIVATE), Alignment (UINT8, CELL_PUBLIC), faction (UINT8, CELL_PUBLIC), Archetype (UINT8, CELL_PUBLIC), threatenedMobs (ARRAY<INT32>, CELL_PUBLIC), lastCombatTime, lastRegenTime (FLOAT), regenTimerID (INT32), NearCoverSetIDs (PYTHON), successiveShots (INT8), successiveShotsTarget (INT32), lastSuccessiveShotTime (FLOAT), bHealDebug (INT8), statsBaseMin/statsBaseCurrent/statsBaseMax/statsMin/statsCurrent/statsMax (StatList, CELL_PUBLIC), reloadTimerId (CONTROLLER_ID, CELL_PUBLIC), currentAmmoType (INT32, CELL_PRIVATE)

**Cell Methods (11):** onAttacked, onAddedToThreatList, onRemovedFromThreatList, invokeThreatFromAbility, setCrouched [Exposed], toggleHealDebug [Exposed], onStealthDetected, _addAoIException, _removeAoIException, adjustStat, adjustStats, onEnterCoverSet, onLeaveCoverSet, requestHolsterWeapon [Exposed]

**Client Methods (4):** onStatUpdate, onStatBaseUpdate, onMeleeRangeUpdate, onArchetypeUpdate, onAlignmentUpdate, onFactionUpdate

---

### SGWInventoryManager

**Used by:** SGWPlayer

Player inventory system: bags, items, equipment slots, cash, weapon activation, ammo types, racial paradigms, disciplines, and crafting knowledge.

**Properties (14):**
playerBags (PYTHON), activeSlots (PYTHON), inventoryAdjustments (PYTHON), pendingItemTransactions (PYTHON), cash (INT32, default 0), weaponActivationTimerID, weaponDeactivationTimerID (CONTROLLER_ID), weaponActivated (UINT8, default 1), inventoryComponents (ARRAY<WSTRING>, CELL_PUBLIC), knownAmmoTypes (ARRAY<INT32>), racialParadigmLevels (PYTHON), appliedSciencePoints (INT32), knownDisciplines (PYTHON), knownCrafts (ARRAY<INT32>) -- most CELL_PRIVATE

**Cell Methods (10):** giveCash, requestGiveItem, removeItem [Exposed], requestRemoveItem, listItems [Exposed], moveItem [Exposed], useItem [Exposed], repairItemRequest [Exposed], requestActiveSlotChange [Exposed], requestAmmoChange [Exposed], showInventory, swapItemsFinish, onOrgMoveItemResult

**Client Methods (7):** onBagInfo, onActiveSlotUpdate, onRemoveItem, onUpdateItem, onRefreshItem, onClearOrgVaultInventory, onCashChanged

---

### SGWMailManager

**Used by:** SGWPlayer

In-game mail system with support for text, cash attachments, item attachments, COD (Cash on Delivery), and archival.

**Properties (4):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| mailMessages | PYTHON | CELL_PRIVATE | {} |
| pendingMailMessages | PYTHON | CELL_PRIVATE | {} |
| lastMailGetTime | FLOAT | CELL_PRIVATE | 0.0 |
| haveMailMessages | UINT8 | CELL_PRIVATE | 0 |

**Cell Methods (9):** requestMailHeaders [Exposed], sendMailMessage [Exposed], archiveMailMessage [Exposed], deleteMailMessage [Exposed], returnMailMessage [Exposed], requestMailBody [Exposed], onNewMail, takeCashFromMailMessage [Exposed], takeItemFromMailMessage [Exposed], payCODForMailMessage [Exposed]

**Base Methods (1):** notifyPlayersOfNewMail(ARRAY<WSTRING> Recipients)

**Client Methods (4):** onMailHeaderInfo, onMailHeaderRemove, onMailRead, sendMailResult

---

### SGWPoller

**Used by:** SGWPlayer

Lightweight interaction polling state.

**Properties (2):**

| Name | Type | Flags | Default |
|------|------|-------|---------|
| interactFlag | INT8 | CELL_PRIVATE | 0 |
| lastPollTime | INT32 | CELL_PRIVATE | 0 |

No methods defined. State is managed externally.

---

## 6. Property Flag Reference

| Flag | Scope | Description |
|------|-------|-------------|
| `CELL_PRIVATE` | Cell only | Exists only on the cell entity. Not replicated to other cells or clients. Used for internal server state (AI data, timers, debug flags). The most common flag. |
| `CELL_PUBLIC` | Cell + other cells | Shared between cells. When an entity migrates or is witnessed by entities on other cells, these properties are available. Also sent to clients in Area of Interest (see `ALL_CLIENTS`). |
| `BASE` | Base only | Exists only on the base entity (persistent process). Not directly accessible from the cell without an RPC. Used for persistent data and base-only logic. |
| `BASE_AND_CLIENT` | Base + owning client | On the base entity and sent to the owning client. Useful for data the client needs that is managed by the base (e.g., character lists). |
| `ALL_CLIENTS` | Cell + all nearby clients | Replicated to all clients that have this entity in their Area of Interest. Used for data everyone can see (name, level, faction, appearance). Properties with `CELL_PUBLIC` flags are implicitly visible to all clients. |
| `OWN_CLIENT` | Cell + owning client | Sent only to the client that owns this entity. Used for private player data (inventory, missions, experience) that other players should not see. |
| `OTHER_CLIENTS` | Cell + other nearby clients | Sent to all clients in AoI EXCEPT the owning client. Rarely used; for data relevant only to observers. |
| `CELL_PUBLIC_AND_OWN` | Cell (public) + owning client | Shared between cells AND sent to the owning client. Combines cross-cell visibility with owner notification. |

**Additional property tags:**

| Tag | Description |
|-----|-------------|
| `<Persistent>true</Persistent>` | Property is saved to the database. Only a few properties use this (e.g., `playerName`, `areaKey`). |
| `<Identifier>true</Identifier>` | Property is used as the database identifier/key for this entity type. |
| `<DatabaseLength>N</DatabaseLength>` | Maximum string length in the database schema. |
| `<Default>value</Default>` | Default value assigned when the entity is created. |

**Method tags:**

| Tag | Description |
|-----|-------------|
| `<Exposed/>` | Method can be called by the client. Without this tag, only server-side entities can invoke the method. Base methods with `<Exposed/>` can be called from the client directly. Cell methods with `<Exposed/>` can be called from the client when the client has a cell entity. |

---

## Appendix: File Locations

| Category | Path |
|----------|------|
| Entity registry | `entities/entities.xml` |
| Entity definitions | `entities/defs/*.def` (18 files) |
| Interface definitions | `entities/defs/interfaces/*.def` (18 files) |
| Cell implementations | `python/cell/*.py` |
| Base implementations | `python/base/*.py` |
| Custom type aliases | `entities/defs/custom_alias.xml` |
