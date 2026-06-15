---
title: "SGWPlayer / SGWGmPlayer Exposed CellMethod Dispatch Table"
type: reference
audience: engineers
last_updated: 2026-06-15
---

# SGWPlayer Exposed CellMethod Dispatch Table

Client-to-server cell method calls. Only methods with `<Exposed/>` in the .def file
get a wire index. Non-exposed methods are server-internal and **skipped** in numbering.

## Wire Encoding

- **Direct** (index 0-60): `[msg_id = index | 0x80][word_len: u16][entity_id: u32][args...]`
- **Extended** (index 61+): `[0xBD][word_len: u16][entity_id: u32][sub_index: u8 = index - 61][args...]`

The 4-byte `entity_id` prefix is always present and must be stripped before reading args.

---

## Entity Hierarchy

```
SGWSpawnableEntity          (parent, 0 exposed CellMethods)
  -> SGWBeing               (parent of SGWPlayer)
       implements: SGWBeing (interface), SGWAbilityManager, SGWCombatant
     -> SGWPlayer
          implements: Communicator, OrganizationMember, MinigamePlayer,
                      GateTravel, SGWInventoryManager, SGWMailManager,
                      Missionary, SGWPoller, ContactListManager,
                      SGWBlackMarketManager, ClientCache
```

Interfaces are traversed in the order listed in `<Implements>`, depth-first.
SGWBeing's interfaces come first (from SGWBeing.def), then SGWPlayer's (from SGWPlayer.def).

---

## Interface CellMethods (Indices 0-66)

### SGWBeing (interface) -- 2 exposed / 14 total

Source: `entities/defs/interfaces/SGWBeing.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 0 | setTargetID | YES | INT32 targetId |
| 1 | setMovementType | YES | UINT8 aMovementType (`EMobMovementType`: Cover=0, CombatAdvance=1, Patrol=2, Follow=3, Wander=4, Leash=5, Avoid=6). Bidirectional via the BigWorld Exposed convention — the server fans the same method index out to AoI witnesses (and the entity's own client) on NPC AI state transitions. See `crate::cell::abilities::messaging::broadcast_movement_type` (#48 / #270). |
| - | onPetSpawn | no | |
| - | onPetDeath | no | |
| - | onPetDetection | no | |
| - | toggleStateField | no | |
| - | setStateField | no | |
| - | registerVisionChangeCallback | no | |
| - | unregisterVisionChangeCallback | no | |
| - | enableDisguise | no | |
| - | enableDisguiseByDef | no | |
| - | reduceDisguiseRating | no | |
| - | stopMovement | no | |
| - | restoreMovement | no | |

### SGWAbilityManager (interface) -- 3 exposed

Source: `entities/defs/interfaces/SGWAbilityManager.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 2 | toggleCombatDebug | YES | (none) |
| 3 | toggleCombatVerboseDebug | YES | (none) |
| 4 | confirmationResponse | YES | INT8 choice |
| - | onHealthZeroed | no | |
| - | invokeAbility | no | |
| - | resolveAbility | no | |
| - | resolveEffect | no | |
| - | onKillCredit | no | |
| - | (+ others) | no | |

### SGWCombatant (interface) -- 3 exposed

Source: `entities/defs/interfaces/SGWCombatant.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 5 | setCrouched | YES | INT8 crouched |
| 6 | toggleHealDebug | YES | (none) |
| 7 | requestHolsterWeapon | YES | INT8 holstered |
| - | onAttacked | no | |
| - | onAddedToThreatList | no | |
| - | (+ others) | no | |

### Communicator (interface) -- 0 exposed

Source: `entities/defs/interfaces/Communicator.def`

No exposed CellMethods (only BaseMethods). One non-exposed: `processPlayerCommunication`.

### OrganizationMember (interface) -- 12 exposed

Source: `entities/defs/interfaces/OrganizationMember.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| - | onOrganizationJoined | no | |
| - | organizationInvite | no | |
| - | organizationInviteByType | no | |
| - | organizationKick | no | |
| - | organizationRankChange | no | |
| - | onMemberRankChange | no | |
| - | onOrganizationMOTDUpdate | no | |
| - | onOrganizationNoteUpdate | no | |
| - | onOrganizationOfficerNoteUpdate | no | |
| - | onOrganizationRankPermissionsUpdate | no | |
| - | onOrganizationSingleRankNameUpdate | no | |
| - | onSquadLootModeUpdate | no | |
| - | onOrganizationInvite | no | |
| 8 | organizationInviteResponse | YES | INT32 requestId, UINT8 response |
| - | organizationInviteResults | no | |
| - | organizationInviteAccepted | no | |
| 9 | organizationLeave | YES | INT32 organizationId |
| - | onOrganizationMemberJoined | no | |
| - | initialOrganizationMemberInfo | no | |
| - | onOrganizationMemberInfoUpdate | no | |
| 10 | BroadcastMinimapPing | YES | INT32 orgId, VECTOR3 location |
| - | receivedMinimapPing | no | |
| - | onStrikeTeamUpdate | no | |
| 11 | strikeTeamResponse | YES | INT32 orgId, UINT8 response |
| 12 | pvpOrganizationLeaveResponse | YES | INT32 orgId, UINT8 response |
| - | onOrganizationHeaderUpdate | no | |
| - | onOrganizationCashUpdate | no | |
| - | orgUpdatePlayerCash | no | |
| - | onOrganizationRankUpdate | no | |
| - | onOrganizationRankNameUpdate | no | |
| 13 | organizationMOTD | YES | INT32 orgId, WSTRING motd |
| 14 | organizationNote | YES | INT32 orgId, WSTRING note |
| 15 | organizationOfficerNote | YES | INT32 orgId, WSTRING name, WSTRING note |
| 16 | organizationSetRankPermissions | YES | INT32 orgId, INT32 rank, INT32 permissions |
| 17 | organizationSetRankName | YES | INT32 orgId, INT32 rank, WSTRING name |
| 18 | squadSetLootMode | YES | INT32 lootMode |
| 19 | organizationTransferCash | YES | INT32 orgId, INT32 cash |

### MinigamePlayer (interface) -- 15 exposed

Source: `entities/defs/interfaces/MinigamePlayer.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 20 | debugStartMinigame | YES | INT32 gameId |
| 21 | debugSpectateMinigame | YES | INT32 gameId |
| 22 | debugJoinMinigame | YES | INT32 gameId |
| 23 | debugMinigameInstance | YES | INT32 instanceId |
| 24 | startMinigame | YES | INT32 hostEntityId, INT32 gameDefId |
| 25 | endCurrentMinigame | YES | INT32 gameId, INT32 winnerId, INT32 loserId |
| 26 | requestSpectateList | YES | INT32 gameId |
| 27 | spectateMinigame | YES | INT32 gameId |
| 28 | registerToMinigameHelp | YES | INT32 gameDefId, INT32 helpLevel |
| 29 | updateRegisterToMinigameHelp | YES | INT32 gameDefId, INT32 helpLevel |
| 30 | minigameStartCancel | YES | INT32 gameId |
| 31 | minigameCallAccept | YES | INT32 gameId |
| 32 | minigameCallDecline | YES | INT32 gameId |
| 33 | minigameCallAbort | YES | INT32 gameId |
| 34 | minigameContactRequest | YES | INT32 targetEntityId, INT32 gameDefId |

### GateTravel (interface) -- 1 exposed

Source: `entities/defs/interfaces/GateTravel.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 35 | onDialGate | YES | INT32 targetAddressId, INT32 sourceAddressId |
| - | giveStargateAddressStr | no | |
| - | removeStargateAddressStr | no | |
| - | closeGatesTo | no | |
| - | processGateTravel | no | |

### SGWInventoryManager (interface) -- 7 exposed

Source: `entities/defs/interfaces/SGWInventoryManager.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 36 | removeItem | YES | ItemID itemId, INT16 quantity |
| 37 | listItems | YES | (none) |
| 38 | moveItem | YES | INT32 itemId, INT32 targetBag, INT32 targetSlot, INT32 quantity |
| 39 | useItem | YES | INT32 itemId, INT32 targetId |
| 40 | repairItemRequest | YES | INT32 itemId, FLOAT repairRatio |
| 41 | requestActiveSlotChange | YES | INT32 bagId, INT32 slotId |
| 42 | requestAmmoChange | YES | INT32 itemId, INT32 ammoType |
| - | giveCash | no | |
| - | requestGiveItem | no | |
| - | requestRemoveItem | no | |
| - | (+ others) | no | |

### SGWMailManager (interface) -- 9 exposed

Source: `entities/defs/interfaces/SGWMailManager.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 43 | requestMailHeaders | YES | UINT8 bArchive |
| 44 | sendMailMessage | YES | INT32 recipientFlags, ARRAY\<WSTRING\> recipients, WSTRING subject, WSTRING body, INT32 cash, UINT8 bCOD, INT32 itemId, INT32 itemQuantity |
| 45 | archiveMailMessage | YES | INT32 mailId |
| 46 | deleteMailMessage | YES | INT32 mailId |
| 47 | returnMailMessage | YES | INT32 mailId |
| 48 | requestMailBody | YES | INT32 mailId |
| 49 | takeCashFromMailMessage | YES | INT32 mailId |
| 50 | takeItemFromMailMessage | YES | INT32 mailId, INT32 containerId, INT32 slotId |
| 51 | payCODForMailMessage | YES | INT32 mailId |
| - | onNewMail | no | |

### Missionary (interface) -- 3 exposed

Source: `entities/defs/interfaces/Missionary.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 52 | abandonMission | YES | INT32 missionId |
| 53 | shareMission | YES | INT32 missionId |
| 54 | shareMissionResponse | YES | INT8 choice |
| - | shareMissionOffer | no | |
| - | (+ many server-internal methods) | no | |

### SGWPoller (interface) -- 0 exposed

Source: `entities/defs/interfaces/SGWPoller.def`

Empty CellMethods section.

### ContactListManager (interface) -- 6 exposed

Source: `entities/defs/interfaces/ContactListManager.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 55 | contactListCreate | YES | WSTRING listName, UINT32 flags |
| 56 | contactListDelete | YES | INT32 listId |
| 57 | contactListRename | YES | INT32 listId, WSTRING newName |
| 58 | contactListFlagsUpdate | YES | INT32 listId, UINT32 flags |
| 59 | contactListAddMembers | YES | INT32 listId, ARRAY\<WSTRING\> members |
| 60 | contactListRemoveMembers | YES | INT32 listId, ARRAY\<WSTRING\> members |

### SGWBlackMarketManager (interface) -- 6 exposed

Source: `entities/defs/interfaces/SGWBlackMarketManager.def`

| Index | Method | Exposed | Args |
|-------|--------|---------|------|
| 61 | BMSearch | YES | BMSearchOptions searchOptions |
| 62 | BMCreateAuction | YES | INT32 itemId, INT32 initialBid, INT32 buyoutPrice, INT32 durationDays |
| 63 | BMPlaceBid | YES | INT32 auctionId, INT32 bidAmount |
| 64 | BMCancelAuction | YES | INT32 auctionId |
| 65 | BMStartWatchingItem | YES | INT32 auctionId |
| 66 | BMStopWatchingItem | YES | INT32 auctionId |

### ClientCache (interface) -- 0 exposed CellMethods

Source: `entities/defs/interfaces/ClientCache.def`

No CellMethods. Only BaseMethods (versionInfoRequest, elementDataRequest) and
ClientMethods (onVersionInfo, onCookedDataError).

---

## SGWPlayer Own CellMethods (Indices 67-108)

Source: `entities/defs/SGWPlayer.def` lines 564-1109

42 exposed methods out of ~80 total.

| Index | Method | Exposed | Args | .def line |
|-------|--------|---------|------|-----------|
| 67 | callForAid | YES | INT32 respawnerID | 566 |
| - | awardSquadXP | no | | 572 |
| 68 | useAbility | YES | INT32 abilityId, INT32 targetId | 580 |
| 69 | useAbilityOnGroundTarget | YES | INT32 abilityId, FLOAT x, FLOAT y, FLOAT z | 586 |
| 70 | respawn | YES | (none) | 594 |
| 71 | unstuck | YES | (none) | 598 |
| 72 | resetMyAbilities | YES | (none) | 602 |
| 73 | who | YES | WSTRING name, WSTRING archetype, WSTRING alignment, WSTRING playerType | 606 |
| 74 | interact | YES | INT32 overrideTarget | 615 |
| 75 | dialogButtonChoice | YES | INT32 dialogId, INT32 buttonId | 621 |
| 76 | initialResponse | YES | INT32 dialogSetMapId | 629 |
| 77 | trainAbility | YES | INT32 abilityId | 635 |
| - | giveTrainingPoints | no | | 640 |
| 78 | purchaseItems | YES | ARRAY\<INT32\> itemIndices, ARRAY\<INT32\> quantities | 644 |
| 79 | sellItems | YES | ARRAY\<INT32\> itemIds, ARRAY\<INT32\> quantities | 650 |
| 80 | buybackItems | YES | ARRAY\<INT32\> itemIds, ARRAY\<INT32\> quantities | 656 |
| 81 | repairItems | YES | ARRAY\<INT32\> itemIds | 662 |
| 82 | rechargeItems | YES | ARRAY\<INT32\> itemIds | 667 |
| - | requestAdditionalLoot | no | | 672 |
| - | operateGateLoc | no | | 680 |
| - | onSendCombatDebug | no | | 685 |
| - | onSendEventDebug | no | | 690 |
| - | startAutoCycleAbility | no | | 694 |
| - | clearAbilities | no | | 698 |
| 83 | setAutoCycle | YES | INT8 enabled | 701 |
| - | stopAutoCycle | no | | 706 |
| - | sendGreetWindowToClient | no | | 710 |
| - | sendDialogDisplayToClient | no | | 717 |
| - | sendLootToClient | no | | 728 |
| - | closeLootWindow | no | | 734 |
| 84 | lootItem | YES | INT32 index | 738 |
| - | setDesignerFlag | no | | 743 |
| - | onClientReady (cell) | no | | 749 |
| - | setGateAddressLoc | no | | 752 |
| - | setGateAddressPoint | no | | 758 |
| 85 | triggerClientHintedGenericRegion | YES | INT32 id, UINT8 bEntering, VECTOR3 position | 766 |
| - | onTeleportStart | no | | 775 |
| - | onTeleportFinished | no | | 781 |
| - | gmRequestInfo | no | | 784 |
| - | onSetSpeed | no | | 790 |
| 86 | requestReload | YES | UINT8 reloadType | 794 |
| - | removeWaypoint | no | | 799 |
| - | onMissionRewardsDisplay | no | | 807 |
| - | onMissionOfferDisplay | no | | 814 |
| 87 | chosenRewards | YES | RewardChoices choices, INT32 missionId | 821 |
| 88 | petInvokeAbility | YES | INT32 entityId, INT32 abilityId, INT32 targetId | 827 |
| 89 | petAbilityToggle | YES | INT32 entityId, INT32 abilityId, INT8 toggle | 834 |
| 90 | petChangeStance | YES | INT32 entityId, INT8 stance | 841 |
| 91 | setRingTransporterDestination | YES | INT32 regionId, INT32 destinationId | 848 |
| - | onSquadMemberRingTransport | no | | 855 |
| - | onSquadMemberRingTransportFinished | no | | 861 |
| - | onReady | no | | 865 |
| 92 | onWorldInstanceReset | YES | (none) | 868 |
| 93 | updateSystemOptions | YES | ARRAY\<NameValuePair\> options | 872 |
| 94 | onOrganizationCreation | YES | WSTRING organizationName | 877 |
| - | callForAidFinish | no | | 882 |
| - | onRegisterSpawnRegionUpdates | no | | 886 |
| - | onDeregisterSpawnRegionUpdates | no | | 890 |
| - | gainRacialParadigmLevels | no | | 894 |
| - | gainExpertise | no | | 900 |
| - | gainAppliedSciencePoints | no | | 906 |
| - | updateCraftingFlags | no | | 911 |
| 95 | spendAppliedSciencePoints | YES | INT32 disciplineSeqId | 916 |
| 96 | craft | YES | INT32 craftId, ARRAY\<ItemID\> items, INT32 quantity | 921 |
| 97 | research | YES | ItemID itemId, ARRAY\<ItemID\> kickers | 928 |
| 98 | reverseEngineer | YES | ItemID itemId | 934 |
| 99 | alloying | YES | INT32 craftId, ItemID currentTierItemId, ARRAY\<ItemID\> lowerTierItems | 939 |
| 100 | respecCrafting | YES | (none) | 946 |
| - | gmGotoCallback | no | | 950 |
| 101 | onClientChallengeResponse | YES | INT32 challenge, WSTRING version, INT32 type, WSTRING object, INT32 id1, INT32 id2, WSTRING value | 955 |
| - | showPlayer | no | | 966 |
| - | sendDuelChallenge | no | | 970 |
| 102 | sendDuelResponse | YES | INT8 response | 975 |
| - | duelChallenge | no | | 980 |
| - | duelResponse | no | | 985 |
| - | duelEntityDefeat | no | | 991 |
| - | startSquadDuel | no | | 996 |
| - | duelAbort | no | | 1000 |
| - | onDuelDefeat | no | | 1003 |
| - | registerDuelMarker | no | | 1007 |
| - | startDuel | no | | 1011 |
| 103 | duelForfeit | YES | (none) | 1015 |
| - | perfStats | no | | 1019 |
| 104 | tradeRequest | YES | INT32 entityId, LocalTradeProposal proposal | 1034 |
| - | tradeRequestFromEntity | no | | 1041 |
| 105 | tradeRequestCancel | YES | INT32 entityId | 1047 |
| 106 | tradeUpdateProposal | YES | INT32 entityId, LocalTradeProposal proposal | 1053 |
| - | updateTradeState | no | | 1060 |
| 107 | tradeLockState | YES | INT32 localVersionId, INT32 remoteVersionId, INT8 lockState | 1067 |
| - | learnPlayerRespawner | no | | 1075 |
| - | updateTradeLockState | no | | 1080 |
| - | tradeCancel | no | | 1087 |
| - | setPvPFlag | no | | 1091 |
| - | startPvPTimer | no | | 1096 |
| - | cancelPvPTimer | no | | 1101 |
| 108 | cancelMovie | YES | WSTRING movieName | 1104 |

---

## Summary

| Range | Source | Exposed Count |
|-------|--------|---------------|
| 0-1 | SGWBeing (interface) | 2 |
| 2-4 | SGWAbilityManager | 3 |
| 5-7 | SGWCombatant | 3 |
| - | Communicator | 0 |
| 8-19 | OrganizationMember | 12 |
| 20-34 | MinigamePlayer | 15 |
| 35 | GateTravel | 1 |
| 36-42 | SGWInventoryManager | 7 |
| 43-51 | SGWMailManager | 9 |
| 52-54 | Missionary | 3 |
| - | SGWPoller | 0 |
| 55-60 | ContactListManager | 6 |
| 61-66 | SGWBlackMarketManager | 6 |
| - | ClientCache | 0 |
| 67-108 | SGWPlayer (own) | 42 |
| **Total** | | **109** |

## Key Methods for Server Implementation

| Index | Method | Wire | Notes |
|-------|--------|------|-------|
| 0 | setTargetID | 0x80 | Target selection |
| 1 | setMovementType | 0x81 | Walk/run/crouch |
| 5 | setCrouched | 0x85 | Crouch toggle |
| 35 | onDialGate | 0xBD+0 | Stargate travel (extended) |
| 52 | abandonMission | 0xB4 | Mission abandon |
| 68 | useAbility | 0xBD+7 | Combat ability use (extended) |
| 70 | respawn | 0xBD+9 | Death respawn (extended) |
| 74 | interact | 0xBD+13 | NPC interaction (extended) |
| 83 | setAutoCycle | 0xBD+22 | Auto-attack toggle (extended) |
| 108 | cancelMovie | 0xBD+47 | Cinematic finished (extended) |

## Validation

This table was derived by reading every `<CellMethods>` section across the entity
hierarchy .def files, counting only `<Exposed/>` methods in file order.
Triple-checked 2026-03-16. The OrganizationMember interface has **12** exposed
methods (not 11 -- `organizationLeave` at line 287 is easily missed).

---

## SGWGmPlayer extension (indices 109+) — #473 / CAT-N-04

GMs (account `access_level > 0`) enter the world as **SGWGmPlayer** (`class_id =
0x03`) instead of SGWPlayer (`0x02`). The single `class_id` byte in
CREATE_BASE_PLAYER is what the client uses to bind the entity method table.

`SGWGmPlayer.def` declares `<Parent>SGWPlayer</Parent>` with an empty
`<Implements>`, so its own `<Exposed/>` CellMethods **append at the end** of the
flattened table. The inherited SGWPlayer indices 0-108 do **not** renumber, and
the wire `idbase` stays 61 (the exposed-method-count staircase doesn't step
between 157 and 163). The first own SGWGmPlayer method (`gmMissionAssign`, def
line 65) is index **109**; counting forward in document order (skipping
`gmSetCallback` at def line 312, which has no `<Exposed/>`) runs to index **225**
(`changeCoverStanceWeight`, def line 673) — 117 gm*/debug methods total
(indices 109-225 inclusive).

### Authorization

The **entire** SGWGmPlayer tail (`index >= 109`) is GM-gated by
`crates/services/src/cell/dispatch/gm_gate.rs`: a caller whose
`CellEntity::access_level` is below `GameMaster` is rejected with an `onErrorCode`
(method 121) **before** any handler runs. One range rule (`index >= 109`) secures
every gm* method, implemented or not. See
[gm-cell-method-gating.md](../architecture/gm-cell-method-gating.md).

### Implemented handlers

A verified subset is wired into the cell router (the rest fall through to the
already-authorized "unhandled cell method" warn arm — harmless). Each index is
counted against `SGWGmPlayer.def` document order and asserted in
`cell::cell_methods::gm::tests::gm_indices_match_def_document_order`.

| Index | Method | Def line | Args | Behaviour |
|-------|--------|----------|------|-----------|
| 133 | `gmGiveItem` | 185 | `WSTRING DesignId, INT32 Quantity` | Grants the item to the GM's own inventory via `GrantItem`. DesignId resolved as a positive numeric design id (internal-name resolution not wired in the cell — rejected with a warn). Quantity clamped to `[1, 1000]`. |
| 163 | `gmGotoXYZ` | 348 | `FLOAT aX, FLOAT aY, FLOAT aZ` | Teleports the GM to the coordinate in their current space via `TeleportPlayer` (same-space FORCED_POSITION snap). Non-finite coordinates rejected. |
| 190 | `gmKillTarget` | 482 | `INT64 TargetId` | Kills an NPC via the canonical death sequence (`abilities::gm_kill_npc`). Refuses player targets and targets in a different space; INT64 ids out of `u32` range rejected. |

> The `class_id` flip and the per-method index counts are byte-verified server
> side (see the wire-format test
> `mercury::world_data::tests::player_creation::create_base_player_class_id_byte_reflects_gm_vs_player`).
> **Client-side GM-console verification still needs a manual UAT with a real
> client** — the server emits the correct class id and method indices, but only
> a live client confirms the GM method table binds and the console commands
> round-trip.

### Full gm* cell-method inventory + Cimmeria handler status

Every SGWGmPlayer own cell method (indices 109–225), its wire args, the stock
client console command that fires it (where one exists in the client's
`SGWTextCommandMgr`), and whether Cimmeria already has a server-side primitive to
service it. This is the implementation map for expanding the native GM surface
beyond the 3 verified handlers above.

**Status legend:**

- **DONE** — handler wired in `cell/cell_methods/gm.rs` (#518).
- **REUSE** — a direct primitive exists; a thin handler just calls it (low effort).
- **ADAPT** — a close primitive exists but needs a wrapper, a `pub(crate)`
  visibility widen, a param/target-vs-self tweak, or a client feedback callback
  (medium effort).
- **NEW** — no primitive; build from scratch (high effort).

**Tally (of 117):** 3 DONE · 18 REUSE · 52 ADAPT · 44 NEW.

> **Provenance.** Indices/args are byte-derived from `entities/defs/SGWGmPlayer.def`
> (document order; `gmSetCallback` at def line 312 excluded — no `<Exposed/>`) and
> match the client's baked method table. Stock console commands are from
> `SGWTextCommandMgr` / `Event_SlashCmd_*` / `Event_NetOut_*` strings in `SGW.exe`
> (QA build) — note the client emits cell calls **by name**, and a few `Event_NetOut`
> names differ from the cell-method name (`/Spawn`→`gmSpawnByCmd`,
> `/GiveNaqahdah`→`gmGiveCash`, `/Users`/`/Who`→`gmUsers`); those bindings are by
> semantics and should be confirmed against a live client / pcap. `file:line`
> primitives below were surveyed on `main`; the 3 **DONE** rows live in this branch's
> `gm.rs`. The bigger blocker for *any* new handler is per-call `access_level` at the
> cell-dispatch boundary — already solved on this branch by `gm_gate` (every index
> ≥109 is GM-gated), so handlers added here inherit authorization for free.

> **Note on "SHOW/LIST/PRINT" rows (ADAPT):** the *read* is trivial, but delivering
> text to the GM needs a client-facing callback. The cheapest path is the
> single-recipient feedback channel #517 already built (`onPlayerCommunication` on
> `CHAN_FEEDBACK`); the native `onShow*` client methods (SGWGmPlayer client tail,
> 157+) are the higher-fidelity option.

#### Missions (109–120)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 109 | `gmMissionAssign(WSTRING DesignID, UINT8 popup)` | `/MissionAssign` | `cell/missions.rs:177 accept_mission` (needs DesignID→id) | ADAPT |
| 110 | `gmMissionClear(WSTRING DesignID)` | `/MissionClear` | `cell/missions.rs:282 abandon_mission` | REUSE |
| 111 | `gmMissionClearActive()` | `/MissionClearActive` | loop `abandon_mission` over `active_missions()` | ADAPT |
| 112 | `gmMissionClearHistory()` | `/MissionClearHistory` | — (no history-clear) | NEW |
| 113 | `gmMissionList()` | `/MissionList` | `entity/missions.rs:155 active_missions` + feedback | ADAPT |
| 114 | `gmMissionListFull()` | `/MissionListFull` | `all_missions` + feedback | ADAPT |
| 115 | `gmMissionDetails(WSTRING DesignID)` | `/MissionDetails` | `entity/missions.rs:145 get_mission` + feedback | ADAPT |
| 116 | `gmMissionAdvance(WSTRING DesignID, INT32 step)` | `/MissionAdvance` | `cell/missions.rs:59 advance_step` | REUSE |
| 117 | `gmMissionReset(WSTRING DesignID, INT32 step)` | `/MissionReset` | — (no revert primitive) | NEW |
| 118 | `gmMissionComplete(WSTRING DesignID, INT8 turnIn)` | `/MissionComplete` | `cell/missions.rs:409 complete_mission_direct` (does NOT fire rewards) | ADAPT |
| 119 | `gmMissionSetAvailable(WSTRING DesignID)` | `/MissionSetAvailable` | — (availability not tracked in entity state) | NEW |
| 120 | `gmMissionAbandon(WSTRING DesignID)` | — | `cell/missions.rs:282 abandon_mission` | REUSE |

#### Show / query (121–131)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 121 | `gmShowTargetLocation()` | — | read `CellEntity.position` + feedback | ADAPT |
| 122 | `gmShowRotation()` | — | read `CellEntity.direction` + feedback | ADAPT |
| 123 | `listAbilities()` | — | `entity/abilities.rs:534 known_ability_ids` + feedback | ADAPT |
| 124 | `showPointSet(WSTRING Type)` | — | cover/nav point sets (partial) | ADAPT |
| 125 | `gmShowFlag(INT32 flagId)` | — | `state_field` (no `get_flag(id)` helper) | ADAPT |
| 126 | `gmListInteractions()` | — | read `available_interactions` + feedback | ADAPT |
| 127 | `gmGetMobAttribute(INT32 target, WSTRING attr)` | — | `queries.rs:42 get_entity` (no reflection; hand-map attrs) | ADAPT |
| 128 | `gmShowMobCount(INT32 spaceId)` | — | iterate space entities (no count fn) | ADAPT |
| 129 | `gmShowIP(INT32 target)` | — | SocketAddr in `connected_clients` (no eid→addr index) | ADAPT |
| 130 | `gmShowInventory(INT32 target)` | — | CellEntity has no inventory → base read / base→cell RPC | NEW |
| 131 | `gmShowPlayer(INT32 target)` | — | `service.rs:82 online_players` (+ eid index) | ADAPT |

#### Give / grant (132–141)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 132 | `gmGiveXp(INT32 amount)` | `/GiveXp` | `progression/mod.rs:41 handle_grant_xp` (`GrantXP`) | REUSE |
| 133 | `gmGiveItem(WSTRING DesignId, INT32 qty)` | `/GiveItem` | **`gm.rs` → `GrantItem`** | **DONE** |
| 134 | `gmGiveCash(INT32 amount)` | `/GiveNaqahdah` | `progression/mod.rs:295 handle_grant_cash` | REUSE |
| 135 | `gmRemoveItem(ItemID id, INT16 qty)` | — | `inventory/mod.rs handle_remove_inventory_item` | REUSE |
| 136 | `gmGiveAbility(INT32 abilityID)` | `/GiveAbility` | `progression/mod.rs:400 handle_train_ability` (debits a point; need no-debit variant) | ADAPT |
| 137 | `gmGiveTrainingPoints(INT32 n)` | — | — (no grant fn; XP path touches the field) | NEW |
| 138 | `gmGiveRespawner(INT32 mobID)` | `/GiveRespawner` | — (respawner persistence not implemented) | NEW |
| 139 | `gmGiveExpertise(INT32 disc, INT32 amt)` | — | `crafting/persistence.rs:208` (delete-all/insert-all; needs upsert) | ADAPT |
| 140 | `gmGiveAppliedSciencePoints(INT32 pts)` | — | crafting field exists (no incremental grant fn) | ADAPT |
| 141 | `gmGiveRacialParadigmLevels(INT32 id, INT32 lvls)` | — | `racial_paradigm_levels` array column (needs edit fn) | ADAPT |

#### Set player / target state (142–158)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 142 | `gmSetGodMode(UINT8 on)` | `/SetGodMode` | — (no godmode flag; gate would live in `cell/combat/damage.rs:161`) | NEW |
| 143 | `gmSetNoXP()` | `/SetNoXP` | — (no XP-immunity flag) | NEW |
| 144 | `gmSetNoDamage()` | `/SetNoDamageTimedMode` | — (no damage-immunity flag) | NEW |
| 145 | `gmSetNoAggro(UINT8 on)` | `/SetNoAggro` | — (NPC threat seeding has no gate) | NEW |
| 146 | `gmSetSpeed(FLOAT mult)` | — | `stats/stat.rs:51 set_current` on MOVEMENT_SPEED_MOD | ADAPT |
| 147 | `gmSetHealth(INT32 amt, INT64 target)` | — | `stats/stat.rs:51 set_current(HEALTH)` + serialize_dirty | REUSE |
| 148 | `gmSetHealthMax(INT32 amt, INT64 target)` | — | `stats/stat.rs:81 set_max` | REUSE |
| 149 | `gmSetFocus(INT32 amt, INT64 target)` | — | `set_current(FOCUS)` | REUSE |
| 150 | `gmSetFocusMax(INT32 amt, INT64 target)` | — | `set_max` | REUSE |
| 151 | `gmSetFlag(INT32 flagId, UINT8 force)` | — | `state_flags.rs:36 set_state_flag` (ref-counted; raw force-set caveat) | ADAPT |
| 152 | `gmSetLevel(INT32 level)` | — | `stat_list.rs:305 scale_for_level` + level write + recompute (no single fn) | ADAPT |
| 153 | `gmResetAbilities()` | — | — | NEW |
| 154 | `gmGiveAllAbilities()` | — | — (enumerate archetype tree + bulk insert + burst) | NEW |
| 155 | `gmRespec()` | — | — | NEW |
| 156 | `gmSetTarget(WSTRING nameOrID)` | — | `cell_entity/mod.rs:684 current_target_id` write + onTargetUpdate (name→id missing) | REUSE |
| 157 | `gmSetMobStance(INT32 stance)` | — | — (no stance field separate from `AiState`) | NEW |
| 158 | `gmSetMobAbilitySet(INT32 setId)` | — | `entity/abilities.rs` mutate `known_abilities` (player-oriented) | ADAPT |

#### Travel (159–163)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 159 | `gmDHD(INT8 gateAddr)` | — | `cell/gate_travel.rs:35 handle_dial_gate` | REUSE |
| 160 | `gmGoto(WSTRING nameOrID)` | `/Goto` | `ring_transport/dispatch.rs:276 same_world_teleport` + name/id resolve | ADAPT |
| 161 | `gmSummon(WSTRING nameOrID)` | — | `same_world_teleport` applied to the OTHER entity (no "move other" wrapper) | ADAPT |
| 162 | `gmGotoLocation(WSTRING world, FLOAT x,y,z)` | `/GotoLocation` | base `gate_travel/mod.rs:43 handle_gate_travel` | REUSE |
| 163 | `gmGotoXYZ(FLOAT x,y,z)` | `/GotoXYZ` | **`gm.rs` → `TeleportPlayer`** | **DONE** |

#### Admin / social (164–168)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 164 | `gmReloadOrganizations()` | `/ReloadOrganizations` | — (org methods are stubs; no def hot-reload) | NEW |
| 165 | `gmReloadInventory()` | `/ReloadInventory` | — (no inventory-def hot-reload) | NEW |
| 166 | `gmUsers()` | `/Users`, `/Who` | `service.rs:82 online_players` (already used by admin API) | REUSE |
| 167 | `gmSetHideGM(UINT8 on)` | `/SetHideGM` | — (`bHideGM` not implemented; `access_level` read-only at login) | NEW |
| 168 | `gmPrintStats(WSTRING stat)` | `/PrintStats` | per-entity `stat_list.rs` + feedback (server-wide stats: none) | ADAPT |

#### Debug (169–184)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 169 | `gmDebugAbility(INT32 abilityId)` | — | — | NEW |
| 170 | `gmDebugCombat()` | — | stub `cell_methods/ability_manager.rs:22` (also gated as in-range idx 2) | ADAPT |
| 171 | `gmDebugCombatVerbose()` | — | log-only stub (in-range idx 3) | ADAPT |
| 172 | `gmDebugHeal()` | — | stub `cell_methods/combatant.rs:59` (in-range idx 6) | ADAPT |
| 173 | `gmDebugStartMinigame(INT32 gameId)` | — | `minigame/session.rs:60 register` + cell dispatch stub | ADAPT |
| 174 | `gmDebugSpectateMinigame()` | — | cell stub `cell_methods/minigame.rs` | ADAPT |
| 175 | `gmDebugJoinMinigame()` | — | cell stub | ADAPT |
| 176 | `gmDebugAbilityOnMob(INT32 abilityID)` | — | — | NEW |
| 177 | `gmDebugBehaviorsOnMob()` | — | read `ai_state`/`threat_list` + stream callback | ADAPT |
| 178 | `gmDebugPathsOnMob()` | — | read `cell_entity/mod.rs:493 nav_path` + `onShowPath` callback | ADAPT |
| 179 | `gmDebugEvents(INT32 target, INT32 level)` | — | — | NEW |
| 180 | `gmDebugMobData(INT32 space, INT32 target)` | — | `queries.rs:42 get_entity` + feedback | ADAPT |
| 181 | `gmDebugInteract()` | — | — | NEW |
| 182 | `gmEmitBehaviorEventOnMob(INT32 id)` | — | — | NEW |
| 183 | `gmAddBehaviorEventSet(INT32 id)` | — | — | NEW |
| 184 | `gmRemoveBehaviorEventSet(INT32 id)` | — | — | NEW |

#### Spawn / mob (185–190)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 185 | `gmSpawnByCmd(WSTRING DesignId, FLOAT xOff, FLOAT zOff)` | `/Spawn` | `space_manager/spawn.rs:85 spawn_npc_from_record_in_space` (needs DesignId→`SpawnRecord`). Same primitive #517's chat `/spawn` used. | ADAPT |
| 186 | `gmDespawnByCmd(INT32 target)` | — | `space_manager/entities.rs:82 destroy_entity` | REUSE |
| 187 | `gmRechargeItem(INT32 itemId)` | — | vendor `recharge.rs` (base-scoped; need GM cell→base route) | ADAPT |
| 188 | `gmSetMobAttribute(INT32 target, WSTRING attr, WSTRING type, INT32 val)` | — | `queries.rs:35 get_entity_mut` (no reflection; hand-map attrs) | ADAPT |
| 189 | `gmRespawn()` | `/Respawn` | `cell_methods/player/combat/respawn.rs:73 handle_respawn` | REUSE |
| 190 | `gmKillTarget(INT64 target)` | `/Kill` | **`gm.rs` → `abilities::gm_kill_npc`** | **DONE** |

#### Minigame (191–193)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 191 | `debugMinigameComplete(INT32 resultCode)` | — | base `cell_dispatch/minigame.rs:90 minigame_result` | ADAPT |
| 192 | `gmGiveMinigameContact(WSTRING contactId, INT64 target)` | — | stub `cell_methods/minigame.rs:159` | NEW |
| 193 | `gmRemoveMinigameContact(WSTRING contactId, INT64 target)` | — | stub | NEW |

#### Stargate / content reload (194–207)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 194 | `gmGiveStargateAddress(WSTRING addr, INT64 target, UINT8 hidden)` | — | `spawner/stargates.rs:20 load_stargates` is startup-cache only; no per-player grant | NEW |
| 195 | `gmRemoveStargateAddress(WSTRING addr, INT64 target)` | — | — | NEW |
| 196 | `loadConstants()` | — | — (no constants module / hot-reload) | NEW |
| 197 | `loadAbility(INT32 id)` | — | `spawner/abilities.rs:85 load_ability_defs` (full reload only) | ADAPT |
| 198 | `loadNACSI(INT32 id)` | — | — (no NACSI table) | NEW |
| 199 | `loadAbilitySet(INT32 id)` | — | `spawner/abilities.rs:174 load_archetype_ability_trees` (full) | ADAPT |
| 200 | `loadBehavior(INT32 id)` | — | — (no loadable behavior defs) | NEW |
| 201 | `loadMOB(INT32 id)` | — | — (no per-mob-def reload) | NEW |
| 202 | `loadDialogSet(INT32 id)` | — | `spawner/dialogs.rs:28 load_dialog_set_maps` (full) | ADAPT |
| 203 | `loadItem(INT32 id)` | — | `spawner/loot.rs:173 load_item_defs` (full, weapons only) | ADAPT |
| 204 | `loadMission(INT32 id)` | — | `spawner/missions.rs:40 load_mission_defs` (full) | ADAPT |
| 205 | `setMobVariable(INT32 var, INT32 val)` | — | — (no generic KV store on mob) | NEW |
| 206 | `enterErrorAIState()` | — | `service/npc_ai.rs:1595` + `ai_state = AiState::Error` (also content `Action::SetNpcAiState`) | ADAPT |
| 207 | `exitErrorAIState()` | — | clear `AiState::Error` (no GM entrypoint) | ADAPT |

#### Instance / perf / nav (208–211)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 208 | `gmPerfStatsByChannel(INT8 onOff)` | — | — (no per-channel perf counters) | NEW |
| 209 | `gmShowInstanceFlag(INT32 flag)` | — | — (no `instance_flags` on SpaceInstance) | NEW |
| 210 | `gmSetInstanceFlag(INT32 flag, INT8 val)` | — | — | NEW |
| 211 | `gmShowNavigation(INT8 onOff)` | — | `space_manager/mod.rs:84 navmesh` readable; no overlay callback | ADAPT |

#### Test / loot / vision / cover (212–225)

| Idx | Method (args) | Stock cmd | Cimmeria primitive | Status |
|-----|---------------|-----------|--------------------|--------|
| 212 | `spawnEntityLoot(INT32 entity, LootTableID)` | — | `abilities/loot_drop.rs:29 generate_loot_on_death` (`pub(super)`) | ADAPT |
| 213 | `despawnMob(INT32 entityID)` | — | `space_manager/entities.rs:82 destroy_entity` | REUSE |
| 214 | `activateSpawnSet(INT32 id)` | — | — (no spawn-set runtime activation API) | NEW |
| 215 | `deactivateSpawnSet(INT32 id)` | — | — | NEW |
| 216 | `testLOS(INT32 source, INT32 target)` | — | `entity/navigation.rs:560 NavMesh::raycast` | REUSE |
| 217 | `toggleCombatLOS()` | — | `bCombatLOS` def-only; no Rust enforcement toggle | NEW |
| 218 | `trackMob()` | — | read `ai_state`; no debug-stream toggle | ADAPT |
| 219 | `onXRayEyes(UINT8 on)` | — | client-side presentation only; no server state | NEW |
| 220 | `onInvisible(UINT8 on)` | — | `mercury/aoi/leave.rs:18 build_entity_invisible` (wire only); no visibility-toggle state | NEW |
| 221 | `onPhysics(UINT8 on)` | — | — | NEW |
| 222 | `sendGMShout(UINT8 global, WSTRING text)` | — | `cell/chat.rs:101 broadcast_to_witnesses` (need space/all-shard variant) | ADAPT |
| 223 | `regenerateCoverLinks(FLOAT normLimit, UINT32 maxLinks, FLOAT maxDist)` | — | `cover/loader.rs:88` static-load only; no regen algorithm | NEW |
| 224 | `changeCoverWeight(6×FLOAT)` | — | `cover/scoring.rs:38 CoverWeights` (compile-time const; needs RwLock) | ADAPT |
| 225 | `changeCoverStanceWeight(WSTRING stance, 6×WSTRING)` | — | — (no stance-weight system) | NEW |

#### Also gated (in-range SGWPlayer indices, not in the 109+ tail)

These GM/debug methods share the inherited SGWPlayer interface range and are named
explicitly in `requires_gm` (see [gm-cell-method-gating.md](../architecture/gm-cell-method-gating.md)):
`2 toggleCombatDebug`, `3 toggleCombatVerboseDebug`, `6 toggleHealDebug` (log-only
stubs today), and `92 onWorldInstanceReset` (CAT-N-01, High — destroys + recreates
the space instance; **NEW**, keep gated, no handler).

#### How to add a handler

Each native command is `Event_SlashCmd_X` → `Event_NetOut_X` (client-side, already
in the binary) → the cell-method index above. Server-side you only implement the
cell handler: add an arm to `cell/cell_methods/gm::dispatch` keyed on the index,
parse the args per the def, call the primitive in the table (widening visibility /
adding a wrapper as the status notes), and — for SHOW/LIST rows — emit text via the
`CHAN_FEEDBACK` `onPlayerCommunication` path. The `gm_gate` already authorized the
caller, so handlers must **not** re-check access level. Pin the new index in
`gm_indices_match_def_document_order`.
