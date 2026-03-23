---
name: SGWPlayer Client Method Index Table
description: Complete mapping of all 157 SGWPlayer client method indices, derived from BigWorld .def parse order (parent-first, implements-before-own-methods)
type: reference
---

## SGWPlayer Client Method Index Table (157 total)

BigWorld parses entity definitions recursively: parent first, then for each level it processes Implements (interfaces) before own ClientMethods. The order is deterministic and defines wire protocol method indices.

### Parse order for SGWPlayer:
1. SGWEntity (parent chain root) -> Implements: DistributionGroupMember(0), EventParticipant(0)
2. SGWSpawnableEntity -> own ClientMethods: 12 methods (0-11)
3. SGWBeing -> Implements: SGWBeing-iface(8), SGWAbilityManager(0), SGWCombatant(6) -> own: BeingAppearance(1)
4. SGWPlayer -> Implements: Communicator(7), OrganizationMember(18), MinigamePlayer(13), GateTravel(4), SGWInventoryManager(7), SGWMailManager(4), Missionary(5), SGWPoller(0), ContactListManager(5), SGWBlackMarketManager(6), ClientCache(2) -> own: 59 methods

### Full Index Table

| Range | Source | Count |
|-------|--------|-------|
| 0-11  | SGWSpawnableEntity own | 12 |
| 12-19 | SGWBeing interface | 8 |
| 20-25 | SGWCombatant interface | 6 |
| 26    | SGWBeing.def own (BeingAppearance) | 1 |
| 27-33 | Communicator interface | 7 |
| 34-51 | OrganizationMember interface | 18 |
| 52-64 | MinigamePlayer interface | 13 |
| 65-68 | GateTravel interface | 4 |
| 69-75 | SGWInventoryManager interface | 7 |
| 76-79 | SGWMailManager interface | 4 |
| 80-84 | Missionary interface | 5 |
| 85-89 | ContactListManager interface | 5 |
| 90-95 | SGWBlackMarketManager interface | 6 |
| 96-97 | ClientCache interface | 2 |
| 98-156 | SGWPlayer.def own | 59 |

### Key verified indices (all confirmed working):
- 20: onStatUpdate, 21: onStatBaseUpdate, 24: onAlignmentUpdate, 25: onFactionUpdate
- 26: BeingAppearance, 28: onPlayerCommunication, 31: onChatJoined
- 65: setupStargateInfo, 69: onBagInfo, 72: onUpdateItem, 75: onCashChanged
- 101: onKnownAbilitiesUpdate, 102: onTimeofDay, 105: onDialogDisplay
- 109: onStoreOpen, 115: onPlayerDataLoaded, 117: onClientMapLoad
- 122: setupWorldParameters, 125: addClientHintedGenericRegion
- 141: onAbilityTreeInfo, 152: onDuelEntitiesRemove, 155: onPlayMovie

### Computing method index from .def file
`entity_method_descriptions.cpp:init()` appends each method to `internalMethods_` in XML document order. For client methods, all are marked exposed so `exposedIndex == internalIndex`. The index is simply the sequential position across the entire inheritance + interface chain.

Source: `external/engines/BigWorld-Engine-2.0.1/src/lib/entitydef/entity_description.cpp` lines 240-268 and `base_user_data_object_description.cpp` lines 117-166.
