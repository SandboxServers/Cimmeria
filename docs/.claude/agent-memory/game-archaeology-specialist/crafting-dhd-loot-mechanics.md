---
name: crafting-dhd-loot-mechanics
description: Crafting state machine, Stargate DHD state machine, and loot generation pipeline — client class subscriber graph and key emitter addresses recovered in W-content-mech session 5
metadata:
  type: project
---

## Crafting (class_SGW::Crafting = VCrafting)

VCrafting is the client-side crafting controller. Key facts:
- 7 NetOut events: Craft, Alloy, Research, ReverseEngineer, SpendAppliedSciencePoint, RespecCraft, SetTechSkill
- 8 NetIn subscriptions including **onUpdateRacialParadigmLevel** (NOT in crafting-wire-formats.md — new finding)
- VCrafting subscribes to Cache_ElementReady<SGW::Blueprint> for blueprint loading
- VCrafting subscribes to TimerUpdate for craft induction countdown
- All registration stubs return literal strings — confirmed not callee-reachable (function pointer table pattern)
- SGWNetworkManager EventHandlers follow: vfunc_0 → scalar dtor → MemberCallback ctor → FUN_00a374a0 (wire send)

**Why:** Needed to document crafting mechanics beyond wire-format tables for Cimmeria server implementation guide.
**How to apply:** When investigating crafting bugs, check VCrafting subscriptions first. onUpdateRacialParadigmLevel must be added to crafting-wire-formats.md.

## Stargate DHD (class_GateTravel = VGateTravel)

VGateTravel is the client-side gate travel controller. Key facts:
- **onDialGate emitter** at `0x00e2e120`: sends TargetAddressId + SourceAddressId as INT32 (resolved from 6-glyph UINT8 array via FUN_00d2d8f0 glyph accessor)
- **StargateTriggerFailed** (Event_NetIn) is a new event not in gate-travel-wire-formats.md — registered at 0x00d88060
- **onDHDReply** is handled by VCommunicator (chat channel), NOT GateTravel — it's an NPC dialogue event
- VGateTravel subscribes to: Sys_FrameStart (animation tick), Cache_ElementReady<DBGateInfo>, World_StargateEvent, World_DialStargateAddress (Kismet), Effect applied/removed
- **Ring transporter**: EmitNetOut_SetRingTransporterDestination at 0x00aeab70 sends aRegionId + aDestinationId; VGameProxyPlayer handles onRingTransporterList
- onDialGate address resolution: GateTravel stores active vector at this+0x18/0x1c and pending vector at this+0x28/0x2c

**Why:** Needed for full DHD state machine documentation for cross-world travel implementation.
**How to apply:** When adding StargateTriggerFailed handling, wire fields are unknown — likely INT32 failure code.

## Loot (class_Lootables = VLootables, class_Squad = VSquad)

VLootables is the client-side loot window controller. Key facts:
- VLootables subscribes to: LootDisplay (wire), Cache_ElementReady<DBInvItem> (item data cache warm)
- LootDisplay payload confirmed: entityId UINT32 + ARRAY<InvItem FIXED_DICT> (from TypedEmitInfo plate at 0x00d805d0)
- LootItem NetOut wire: 5 bytes (1B header + 4B Index INT32) — confirmed from plate at 0x00d93680
- VSquad handles onSquadLootType for group loot mode changes
- No need/greed/pass roll NetOut event found — open question

**Why:** Needed to document downstream loot pipeline after right-click-routing-on-corpse.md established the upstream half.
**How to apply:** LootDisplay arrives with full item list inline (no separate container-open request). Cache_ElementReady<DBInvItem> subscription means loot window may delay rendering until item data loads.
