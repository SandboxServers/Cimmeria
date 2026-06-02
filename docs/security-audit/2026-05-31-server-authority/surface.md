# Client → Server outbound message surface (Event_NetOut_*)

Extracted from SGW.exe via Ghidra string search for `Event_NetOut_*`.
~250 distinct C++ class names emit through `SGWNetworkManager`.

## How the client emits these

Pattern (from Ghidra strings):
- C++ class `Event_NetOut_X` is the typed payload
- `EventSignal<Event_NetOut_X>` dispatches via `SGWNetworkManager::EventHandler<Event_NetOut_X>`
- The handler serialises the typed payload into the Mercury bundle on the wire

So every `Event_NetOut_X` is one specific client → server message type.

## Inventory grouped by audit category

### CAT-A — Auth / Session / Character lifecycle
- `versionInfoRequest` / `onClientVersion`
- `elementDataRequest`
- `onClientChallengeResponse`
- `ClientReady`
- `Disconnect` / `LogOff`
- `PlayCharacter` / `CreateCharacter` / `DeleteCharacter` / `RequestCharacterVisuals`

### CAT-B — Movement / Teleport / Position
- `GotoXYZ` (debug teleport — CHEAT class)
- `Goto` / `GotoLocation` (debug)
- `Summon` (debug)
- `Unstuck`
- `SetRingTransporterDestination`
- `SetMovementType`
- `SetCrouched`
- `ChangeWeaponState`
- `Physics` (debug?)

### CAT-C — Combat / Abilities
- `UseAbility`
- `useAbilityOnGroundTarget`
- `ListAbilities`
- `TrainAbility`
- `SetTarget` / `SetTargetID`
- `RequestAmmoChange`
- `RechargeItem`
- `Respawn`
- `CombatDebug` / `CombatDebugVerbose` / `HealDebug` / `AbilityDebug`
- `PetInvokeAbility` / `PetAbilityToggle` / `PetChangeStance`
- `DebugAbilityOnMob` / `DebugBehaviorsOnMob` / `DebugPathsOnMob`
- `EnterErrorAIState` / `ExitErrorAIState`
- `ConfirmEffect`
- `callForAid`

### CAT-D — Inventory / Items
- `UseItem`
- `MoveItem`
- `RemoveItem`
- `RepairItem`
- `LootItem`
- `RequestActiveSlotChange`
- `ListItems`
- `GetItemInfo`
- `requestItemData`
- `RequestAmmoChange`

### CAT-E — Vendor
- `PurchaseItems` / `SellItems` / `BuybackItems` / `RepairItems` / `RechargeItems`

### CAT-F — Crafting / R&D
- `Craft` / `Alloy` / `Research` / `ReverseEngineer`
- `SpendAppliedSciencePoint`
- `TrainAbility`

### CAT-G — Mail
- `RequestMailHeaders`, `SendMailMessage`, `RequestMailBody`
- `ArchiveMailMessage`, `DeleteMailMessage`, `ReturnMailMessage`
- `TakeCashFromMailMessage`, `TakeItemFromMailMessage`
- `PayCODForMailMessage`

### CAT-H — Trade (P2P)
- `TradeProposal`, `TradeLockState`, `TradeRequestCancel`, `TradeRequest`

### CAT-I — Black Market / Auction
- `BMCreateAuction`, `BMCancelAuction`, `BMPlaceBid`, `BMSearch`

### CAT-J — Mission / Dialog / Interaction
- `MissionAbandon`, `AbandonMission`, `ChosenRewards`
- `ShareMission`, `ShareMissionResponse`
- `MissionAssign`, `MissionClear`, `MissionClearActive`, `MissionClearHistory`
- `MissionList`, `MissionListFull`, `MissionDetails`, `MissionAdvance`,
  `MissionReset`, `MissionComplete`, `MissionSetAvailable`
- `InitialResponse`, `DialogButtonChoice`
- `Interact`, `DebugInteract`

### CAT-K — Minigame
- `StartMinigame`, `EndMinigame`, `MinigameComplete`
- `SpectateMinigame`, `RequestSpectateList`, `MinigameStartCancel`
- `RegisterToMinigameHelp`, `UpdateRegisterToMinigameHelp`
- `GiveMinigameContact`, `RemoveMinigameContact`, `MinigameContactRequest`
- `MinigameCallRequest`, `MinigameCallAbort`, `MinigameCallAccept`, `MinigameCallDecline`
- `debugStartMinigame`, `debugSpectateMinigame`, `debugJoinMinigame`, `debugMinigameInstance`

### CAT-L — Chat / Contact list / Communication
- `sendPlayerCommunication`, `Petition`, `Who`, `ChatFriend`
- `ChatJoin`, `ChatLeave`, `ChatList`, `ChatMute`, `ChatKick`, `ChatBan`,
  `ChatOp`, `ChatPassword`, `ChatIgnore`, `ChatSetAFKMessage`, `ChatSetDNDMessage`
- `contactListCreate`, `contactListRename`, `contactListDelete`,
  `contactListFlagsUpdate`, `contactListAddMembers`, `contactListRemoveMembers`
- `SendGMShout`
- `BroadcastMinimapPing`

### CAT-M — Organization / Squad / Duel
- `OrganizationCreation`, `OrganizationInvite`, `OrganizationInviteByType`,
  `OrganizationInviteResponse`, `OrganizationLeave`, `OrganizationKick`,
  `OrganizationRankChange`, `OrganizationMOTD`, `OrganizationNote`,
  `OrganizationOfficerNote`, `OrganizationSetRankPermissions`,
  `OrganizationSetRankName`, `OrganizationTransferCash`, `ReloadOrganizations`
- `SquadSetLootMode`
- `PvPOrganizationLeaveResponse`
- `DuelChallenge`, `DuelResponse`, `DuelForfeit`

### CAT-N — GM / Debug / Cheat (CRITICAL — must all be GM-gated)
- **Give**: `GiveXp`, `GiveNaqahdah`, `GiveItem`, `GiveAmmo`, `GiveAbility`,
  `GiveAllAbilities`, `GiveRespawner`, `GiveTrainingPoints`, `GiveExpertise`,
  `GiveAppliedSciencePoints`, `GiveRacialParadigmLevels`, `GiveFaction`,
  `GiveBlueprint`, `GiveGearset`, `GiveInventory`, `GiveStargateAddress`
- **Set**: `SetGodMode`, `SetNoXP`, `SetNoAggro`, `SetSpeed`, `SetHealth`,
  `SetHealthMax`, `SetFocus`, `SetFocusMax`, `SetFlag`, `SetTarget`, `SetLevel`,
  `SetMobStance`, `SetMobAttribute`, `SetMobAbilitySet`, `SetMobVariable`,
  `SetFaction`, `SetTechSkill`, `SetHideGM`, `SetInstanceFlag`, `SetAutoCycle`,
  `SetMovementType` (could be non-GM — verify), `Invisible`, `XRayEyes`
- **Show/Get**: `ShowIP`, `ShowInventory`, `ShowPlayer`, `ShowRotation`,
  `ShowMobCount`, `ShowPointSet`, `ShowFlag`, `ShowTargetLocation`,
  `ShowInstanceFlag`, `ShowNavigation`, `ShowVariable`, `ListInteractions`,
  `GetMobAttribute`
- **Spawn/Destroy**: `Spawn`, `Despawn`, `Kill`, `GMRemoveItem`,
  `RemoveStargateAddress`
- **Behavior**: `EmitBehaviorEventOnMob`, `AddBehaviorEventSet`,
  `RemoveBehaviorEventSet`
- **Reload/Load**: `RequestReload`, `LoadConstants`, `LoadAbility`,
  `LoadAbilitySet`, `LoadNACSI`, `LoadBehavior`, `LoadMOB`,
  `LoadInteractionSet`, `LoadItem`, `LoadMission`, `RegenerateCoverLinks`,
  `ChangeCoverWeight`, `ChangeCoverStanceWeight`
- **Respec**: `Respec`, `RespecAbility`, `RespecCraft`, `ResetAbilities`
- **Misc**: `Users`, `WorldInstanceReset`, `PerfStats`, `PerfStatsByChannel`,
  `TestLOS`, `ToggleCombatLOS`, `PrintStats`, `DebugEvents`

### CAT-O — World / Space / Gate
- `WorldInstanceReset`
- `onSpaceQueuedResponse`, `onSpaceQueueReadyResponse`, `onSpaceQueueStatus`
- `onDialGate`, `DHD`
- `TriggerClientHintedGenericRegion`
- `onStrikeTeamResponse`
- `SetRingTransporterDestination`
- `CancelMovie`
- `SystemOptions`
