---
name: reference-gm-command-wire-spec
description: Authoritative wire shapes for SGW GM commands; SGWGmPlayer.def + Ghidra Event_NetOut_* anchors
metadata:
  type: reference
---

## SGW GM-command wire spec — extraction points

The SGW client emits ~125 distinct `Event_NetOut_*` GM/debug events from
`SGWTextCommandMgr` (slash command source). Each maps to a `gm*`-prefixed
cell method or base method on `entities/defs/SGWGmPlayer.def` (parent:
SGWPlayer). A small subset (`onWorldInstanceReset`, `resetMyAbilities`,
`toggleCombatDebug`, `toggleCombatVerboseDebug`, `toggleHealDebug`,
`perfStats`) is exposed on the regular SGWPlayer/SGWAbilityManager/SGWCombatant
interfaces too — i.e., the wire indices for those land in the regular
flat-index table.

### File anchors

- `entities/defs/SGWGmPlayer.def` — the GM entity class declaration.
  Parent SGWPlayer. Adds 6 ClientMethods + 80+ CellMethods + 5
  BaseMethods. Every GM-prefixed method shape (args, types) is
  authoritative here.
- `entities/defs/SGWPlayer.def` — has `<onWorldInstanceReset>` line
  868, `<resetMyAbilities>` line 602, `<perfStats>` line 529 all
  `<Exposed/>` — i.e., GM-shaped methods exposed on the regular
  player flat-index table.
- `entities/defs/interfaces/SGWAbilityManager.def:103-113` —
  `bCombatDebug` and `bCombatVerboseDebug` properties + the
  exposed `toggleCombatDebug` / `toggleCombatVerboseDebug` methods.
- `entities/defs/interfaces/SGWCombatant.def` — `toggleHealDebug`
  is here.

### Ghidra anchors (SGW.exe)

The full RTTI string scan for `Event_NetOut_*` GM commands lives
in the binary at ~0x019b3xxx and 0x019bexxx (the NetOut event class
RTTI block). The typed RTTI descriptors live at 0x01df3xxx-0x01df5xxx.

Specific examples:
- `019b373c` Event_NetOut_GiveItem
- `019b3848` Event_NetOut_SetGodMode
- `019b3820` Event_NetOut_Kill
- `019b4340` Event_NetOut_WorldInstanceReset
- `019b3bd8` Event_NetOut_Spawn / `019b3c00` Event_NetOut_Despawn
- `019b3818` Event_NetOut_SetSpeed
- `019b3794` Event_NetOut_GiveAbility / `019b36b0` GiveAllAbilities
- `019b370c` Event_NetOut_GiveNaqahdah / `019b362c` GiveXp
- `019b2ec8` Event_NetOut_SetHideGM
- `019c33b0` gmGiveXp (method-name string)
- `019c3498` gmSetGodMode (method-name string)

The corresponding method-name strings ("gmGiveXp", "gmSetGodMode",
etc.) live in a separate block near 0x019c3xxx — these are the
flat-method-name lookup table used by the BigWorld dispatcher
client-side.

### Wire payload reference for hot ones

(All args are little-endian; WSTRING is `u32 char_count + N×u16 LE`.)

- `gmSetGodMode(UINT8 bTurnOn)`
- `gmSetHealth(INT32 Amount, INT64 TargetId)` — same shape for
  `gmSetHealthMax`, `gmSetFocus`, `gmSetFocusMax`.
- `gmSetSpeed(FLOAT Multiplier)` — unbounded float
- `gmSetLevel(INT32 aLevel)` — self only, no TargetId
- `gmGiveXp(INT32 XpAmount)` / `gmGiveCash(INT32 Amount)` —
  both signed; binary has the error string "Amount to be given
  (or taken away) cannot be 0." confirming negatives are
  intended in the Python reference.
- `gmGiveItem(WSTRING DesignId, INT32 Quantity)`
- `gmGiveRespawner(INT32 aRespawnerMobID)`
- `gmGiveAbility(INT32 aAbilityID)` / `gmGiveAllAbilities()`
- `gmGiveStargateAddress(WSTRING AddressId, INT64 TargetId,
  UINT8 Hidden)`
- `gmKillTarget(INT64 TargetId)`
- `gmSpawnByCmd(WSTRING DesignId, FLOAT XOffset, FLOAT ZOffset)`
- `gmDespawnByCmd(INT32 TargetID)`
- `gmGoto(WSTRING aNameOrID)` / `gmGotoXYZ(FLOAT, FLOAT, FLOAT)`
  / `gmGotoLocation(WSTRING WorldName, FLOAT X, Y, Z)`
- `gmSummon(WSTRING aNameOrID)`
- `gmShowIP(INT32 TargetID)` / `gmShowInventory(INT32 TargetID)`
  / `gmShowPlayer(INT32 TargetID)`
- `gmSetFlag(INT32 aFlagId, UINT8 aForceVal)`
- `gmSetInstanceFlag(INT32 aFlagNumber, INT8 aFlagValue)`
- `gmSetMobAttribute(INT32 TargetID, WSTRING Attribute,
  WSTRING AttributeType, INT32 Value)` — reflection-style setter
- `gmEmitBehaviorEventOnMob(INT32 aBehaviorEventId)`
- `gmAddBehaviorEventSet(INT32)` / `gmRemoveBehaviorEventSet(INT32)`
- `gmSetCallback(PYTHON aChangeList, UINT8 bSuccess, UINT32 aEntityId)`
  — **PYTHON wire type is pickle deserialization**; today the Rust
  cell-method decoder does not handle PYTHON at all so this is
  unreachable, but future implementers must NEVER deserialize
  pickled Python from client.
- `gmGetMobAttribute(INT32 TargetID, WSTRING Attribute)`
- `gmShowMobCount(INT32 SpaceID)` (cell) / `(WSTRING aAreaKey)` (base)
- `gmDebugEvents(INT32 TargetId, INT32 InformLevel)`
- `regenerateCoverLinks(FLOAT NormalLimit, UINT32 MaxLinks,
  FLOAT MaxDistance)` — UINT32 MaxLinks is unclamped
- `changeCoverWeight(FLOAT × 6)` / `changeCoverStanceWeight(WSTRING × 7)`
- `loadConstants()` / `loadAbility(INT32)` / `loadAbilitySet(INT32)`
  / `loadNACSI(INT32)` / `loadBehavior(INT32)` / `loadMOB(INT32)`
  / `loadDialogSet(INT32)` / `loadItem(INT32)` / `loadMission(INT32)`
- `testLOS(INT32 aSourceEntityID, INT32 aTargetEntityID)`
- `toggleCombatLOS()` / `toggleCombatDebug()` / `toggleCombatVerboseDebug()`
  / `toggleHealDebug()`
- `onInvisible(UINT8 bTurnOn)` / `onXRayEyes(UINT8 bTurnOn)`
  / `onPhysics(UINT8 bTurnOn)`
- `sendGMShout(UINT8 isGlobal, WSTRING Text)` (cell variant) /
  `sendGMShout(UINT8 ChannelID, UINT8 BroadcastScope, INT32 SpaceID,
  WSTRING Text)` (base variant)
- `setMobVariable(INT32 aVariable, INT32 aValue)`
- `enterErrorAIState()` / `exitErrorAIState()`
- `gmSetMobAbilitySet(INT32 aAbilitySetId)` /
  `gmSetMobStance(INT32 aNewStance)`
- `gmDHD(INT8 aGateAddress)`
- `gmReloadOrganizations()` / `gmReloadInventory()` / `gmUsers()`
- `gmSetHideGM(UINT8 bTurnOn)` — both cell and base variants
- `gmPerfStatsByChannel(INT8 aOnOff)`
- `gmShowInstanceFlag(INT32 aFlagNumber)` /
  `gmShowNavigation(INT8 aOnOff)`
- `gmRechargeItem(INT32 ItemId)`
- `gmRemoveItem(ItemID itemID, INT16 quantity)` — INT16 signed
  quantity is a footgun
- `gmRemoveStargateAddress(WSTRING AddressId, INT64 TargetId)`
- `gmMission*` family (Assign / Clear / ClearActive / ClearHistory /
  List / ListFull / Details / Advance / Reset / Complete /
  SetAvailable / Abandon)
- `gmRespec()` / `Event_NetOut_RespecAbility` /
  `Event_NetOut_ResetAbilities`

### Mission GM variants on SGWGmPlayer.def lines 65-123

`gmMissionAssign`, `gmMissionClear`, `gmMissionClearActive`,
`gmMissionClearHistory`, `gmMissionList`, `gmMissionListFull`,
`gmMissionDetails`, `gmMissionAdvance`, `gmMissionReset`,
`gmMissionComplete`, `gmMissionSetAvailable`, `gmMissionAbandon` —
all `Exposed`, all take a `WSTRING DesignID` and possibly
additional integer args.

### What's NOT in SGWGmPlayer.def but IS a GM event

Some events appear in the Ghidra NetOut RTTI scan but not in the
SGWGmPlayer.def excerpt — likely because they go through the
SGWTextCommandMgr Python-side dispatch only (no exposed cell
method on the entity). Examples: `Event_NetOut_GiveGearset`,
`Event_NetOut_GiveInventory`, `Event_NetOut_GiveBlueprint`,
`Event_NetOut_GiveTrainingPoints` (declared in .def but on
SGWGmPlayer), `Event_NetOut_SetFaction`, `Event_NetOut_SetTechSkill`.
The Python reference would be the next place to look for
these shapes; needs x64dbg confirmation.
