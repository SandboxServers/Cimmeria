---
type: reference
audience: engineers writing the `cimmeria-client-telemetry` ability hooks (AB-C1 to AB-C5) and anyone asking why a hotbar press never reached the server
last_updated: 2026-10-04
companion_docs:
  - client-wire-emit-suppression.md
  - ability-resolution-pipeline.md
  - client-instrumentation-hookpoints.md
  - client-entity-lifecycle.md
  - cme-event-signal.md
  - native-combat-debug.md
  - ../../analysis/ability-mechanics/lab-uat-and-telemetry.md
---

# Ability client hook anchors (AB-C0)

> **Diátaxis type**: reference
> **Packet**: AB-C0 of the ability-mechanics telemetry plan
> **Binary**: the QA `SGW.exe` (ASLR off, so every address is the live address)
> **Method**: headless Ghidra 12.0.4 (`tools/re/ghidra-headless/Probe.java`), decompile plus raw disassembly, 2026-10-04. No debugger was attached to a live client; everything here is static.
> **Confidence legend**: **VERIFIED** = read in a decompile or disassembly in this pass. **INFERRED** = follows from verified code plus naming or a neighbouring structure. **UNRESOLVED** = not found; the next action is named.

## Summary

The client has one outgoing path and one incoming path for ability traffic, and both are narrow enough to hook once each.

- **Out.** Every ability method the client can send (`useAbility`, `useAbilityOnGroundTarget`, `petInvokeAbility`, `petAbilityToggle`, `confirmationResponse`, `trainAbility`, `resetMyAbilities`) is a `Event_NetOut_*` CME event. A shared member callback, `0x00d43dc0`, turns each one into a call to `RouteOutgoingEntityRpc` (`0x00c6fc40`). The arguments are not a struct: they are a **name-keyed property bag** carried by the event, and the names are the `.def` argument names.
- **In.** Every inbound entity method is read off a `MemoryIStream` by `Client_NetIn_EntityMethodDispatch` into the same kind of property bag, wrapped in an `Event_NetIn_*` event, and later delivered to subscribers by one function, `0x00a372f0`. The `Event_UI_*` events that drive the Lua UI (combat text, effect bar, cooldown overlays) go through the same function.
- **Press.** Hotbar click and hotkey both end in Lua `useAction`. The press has **no cooldown, range, dead, target or resource check on the client**. It drops silently in only three places: the action slot is empty, the ability is not in the client's `AbilitySet`, or the send path finds no connection or no matching entity class. The "in-flight queue gate at `0x00d2b020`" recorded in two earlier documents is not a gate (see [Corrections](#corrections-to-earlier-documents)).

## Seam table

| Packet | Seam | Address | Status |
|---|---|---|---|
| AB-C1 | `RouteOutgoingEntityRpc`, arguments by name from the event bag | `0x00c6fc40`, callback `0x00d43dc0` | VERIFIED |
| AB-C1 | Mercury packet sequence of the carrying bundle | `0x0158bb40` return value, inside `Nub::send` `0x01582160` (network thread) | VERIFIED code, join design INFERRED |
| AB-C1 | Local player's current target | `GameBeing+0xfc` of `[[EM+0x8c]+0x4]` | VERIFIED field, INFERRED object identity |
| AB-C2 | Press chain and every drop branch | `0x00aa94e0`, `0x00ad9580`, `0x00e3cd90`, `0x00d2afc0`, `0x00c6fc40` | VERIFIED |
| AB-C3 | Inbound arguments: wire bytes at `onEntityMethod`, or the event at `0x00a372f0` | `0x00dd2b80` / `0x00a372f0` | VERIFIED for scalars; arrays UNRESOLVED in the bag |
| AB-C4 | Effect bar, cooldown, stat, state flag | `0x00e09160`, `0x00ea6af0`, `0x00ea62b0`, `0x00e004e0` / `0x00e005b0`, `0x00e01c90` | VERIFIED addresses, INFERRED roles for two |
| AB-C5 | Sequence drop branch | `0x00d05790` at `0x00d0585c` | VERIFIED |
| AB-C5 | Combat text and feedback line | Lua only (`SCTMod.createEventWindow`, `ChatMod.onMessageReceived`) | VERIFIED as Lua; native producers UNRESOLVED |

## The two shared structures

### The event bag

An event object is `{vtable, +4 pointer to a std::map<std::string, PropertyBase*>}`. The MSVC 2008 `std::string` is 0x1c bytes: `+0` proxy, `+4` 16-byte buffer or heap pointer, `+0x14` size, `+0x18` capacity (the small-string test in `RouteOutgoingEntityRpc` reads `[+0x18] < 0x10`). A map node holds the key at `+0x0c` and the `PropertyBase*` at `+0x28`; `Property<int>` keeps its value at `+8` (`FUN_005783b0`, `0x005783b0`).

The game's own typed readers take the event as `this`, so a hook does not need to walk the map:

| Reader | Address | Signature | Reads |
|---|---|---|---|
| `GetInt` | `0x00e3cba0` | `bool __thiscall (event*, const std::string* name, int* out)`, `ret 8` | INT32 |
| `GetFloat` | `0x00e3cc20` | same shape | FLOAT |
| `GetByte` | `0x00d434d0` | same shape | INT8 / UINT8 |

Each builds a temporary copy of the tree (`0x004410d0`), looks the name up, and returns success in `AL` (default 0 on a miss). These are the same functions the stock handlers call (`FUN_00ea6af0` reads `"SourceID"`, `"Type"`, `"ID"`, `"TotalTime"`, `"BigWorldTimeComplete"` this way). Array arguments are stored as a `BasicPropertyList` under the argument name (`FUN_00e01f40` reads `"Stats"` through `0x00dfe8d0`); that layout is **UNRESOLVED**, so array-bearing methods are decoded from wire bytes instead (AB-C3).

### The event system

| Function | Address | Shape | Role |
|---|---|---|---|
| event-system accessor | `0x0054c980` (thunk to `0x0054c900`) | returns the singleton | `this` of every call below |
| post (NetOut) | `0x00cacd50` | `thiscall(sys, subject, event, flag)` | queues a 0x18-byte record; `Event_NetOut_*` are posted here |
| post (per event class, vfunc 2) | `0x004414d0` is `NetworkEvent`'s; every `Event_*` class has its own copy of the same body | `thiscall(sys, subject, event, flag)` | same queue; the inbound dispatcher calls it at `0x00c6fc03` on the network thread |
| **dispatch** | **`0x00a372f0`** | `thiscall(sys, subjectId, event, baseTypeDescriptor, classTypeDescriptor)`, `ret 0x10` | looks the subscriber list up by `classTypeDescriptor`, then by `subjectId` and the wildcard 0, and invokes each callback; reached from every event class's vfunc 3 on the **main thread** |
| subscribe | `0x00a374a0` | `thiscall(sys, subjectId, &callback)`, `ret 8` | `callback` is the 12-byte `{vtable, object, method}`; wrapped by per-type helpers such as `0x00dfac80` |

The event name at dispatch time is the type descriptor's name at `classTypeDescriptor+8` (for example `.?AVEvent_NetIn_TimerUpdate@@`). `subjectId` is the entity id for per-entity subscribers (`GameBeing`) and 0 for `NoSubject` ones.

## AB-C1: outbound methods

### The send chain

```text
Lua press ─► native emit ─► Event_NetOut_<X> built with fields named like the .def args
          ─► 0x00cacd50 posts it ─► main-thread pump ─► 0x00a372f0 ─► EventHandler<Event_NetOut_<X>>
          ─► member callback 0x00d43dc0 ─► RouteOutgoingEntityRpc 0x00c6fc40
          ─► startEntityMessage 0x00dd6a60 (cell) / startProxyMessage 0x00dd6980 (base) / 0x00dd8010 (local player cell)
          ─► DataType vfunc 8 (+0x20) per argument writes the bytes ─► Channel bundle ─► Channel::send 0x01576F90
          ─► Nub::send 0x01582160 on the network thread
```

**Single exit (VERIFIED).** `0x00c6fc40` has exactly one direct caller, `0x00d43dd9` inside the shared callback `0x00d43dc0`, and `startEntityMessage` (`0x00dd6a60`), `startProxyMessage` (`0x00dd6980`) and `0x00dd8010` have no caller except `0x00c6fc40`. The callback is stored in the vtable slot area of every `SGWNetworkManager::EventHandler<Event_NetOut_*>`: the data-xref list of `0x00d43dc0` runs to well over a hundred constructors. A method sent by stock code therefore always passes `0x00c6fc40`. (The Black Market patch calls `startEntityMessage` directly on purpose and bypasses it.)

### Signatures

```text
0x00d43dc0  EventHandler callback   thiscall(this = handler, event*, subject*)   ret 8
              handler+0x04 = EntityDescription* (the class: SGWPlayer or SGWGmPlayer)
              handler+0x08 = MethodDescription*
            body: RouteOutgoingEntityRpc(subject, handler+4, handler+8, event)

0x00c6fc40  RouteOutgoingEntityRpc  stdcall(Entity* entity, EntityDescription* desc,
                                            MethodDescription* method, Event* args)   ret 0x10
              entity == NULL means the local player
```

Argument slots were read from the stack offsets in the disassembly (`entity` `[esp+0x240]`, `desc` `[esp+0x250]`, `method` `[esp+0x254]`, `args` `[esp+0x258]` after the pushes). The decompiler shows three parameters; the fourth is real.

`MethodDescription` fields used: `+0x00` method name (`std::string`), `+0x1c & 3` route (`2` = base), `+0x20` vector of `DataType*` (begin `+0x24`, end `+0x28`, stride 4), `+0x30` vector of **argument names** (`std::string`, begin `+0x34`, end `+0x38`, stride 0x1c), `+0x44` wire message id, `+0x48` extended sub-index (negative when unused).

### Decoding the arguments at this seam

For argument `i` the name is the `std::string` at `names_begin + i*0x1c`, usable directly as the `name` argument of `GetInt` / `GetFloat` / `GetByte` with `args` as `this`. The `.def` supplies the type. The bag keys are the `.def` `ArgName`s (VERIFIED for `useAbility`: `FUN_00d2ae40` stores `"AbilityID"` and `"TargetID"`, strings `0x019bb5d8` and `0x019bb5e4`, and the def names are the same).

| Method (cell index) | `Event_NetOut_*` (handler ctor) | Bag keys and types | Status |
|---|---|---|---|
| `useAbility` (68) | `UseAbility` (`0x00cb7d30` builds it) | `AbilityID` int, `TargetID` int | VERIFIED |
| `useAbilityOnGroundTarget` (69) | `useAbilityOnGroundTarget` | `AbilityID` int, `LocationX/Y/Z` float | names from the `.def`; event class VERIFIED, field names INFERRED |
| `petInvokeAbility` (88) | `PetInvokeAbility` (ctor `0x00d66030`) | `aEntityId`, `aAbilityId`, `aTargetId` int | ctor VERIFIED, keys from the `.def` |
| `petAbilityToggle` (89) | `PetAbilityToggle` | `aEntityId`, `aAbilityId` int, `aToggle` byte | event VERIFIED, keys from the `.def` |
| `confirmationResponse` (4) | `ConfirmEffect` (ctor `0x00d672f0`) | `aEffectId` int, `aAccepted` byte | VERIFIED; the dispatch table row `INT8 choice` was wrong and is fixed |
| `trainAbility` (77) | `TrainAbility` (ctor `0x00d5a730`) | `AbilityID` int | ctor VERIFIED |
| `resetMyAbilities` (72) | **`RespecAbility`** (ctor `0x00d5c8f0`) | none | VERIFIED; the event is not named after the method |
| `toggleCombatDebug` (2), `toggleCombatVerboseDebug` (3) | **none** | n/a | The client has no string `toggleCombatDebug` or `toggleCombatVerboseDebug`, so no event binds them. The client cannot send cell 2 or 3. VERIFIED by negative string search |

The existing `client.net.out` event already carries the method name, route, message id and extended sub-index. AB-C1 adds the decoded arguments for this table.

### The client's current target (`client_target_id`)

- **Local player, as the hotbar reads it.** `AbilityAction::execute` (`0x00e3cd90`) reads `GameBeing+0xfc` of `[[EM+0x8c]+0x4]`, with `EM = [0x01ef244c]` (the `GameEntityManager`), `[EM+0x8c]` the player context (`GameProxyPlayer`) and `+0x4` of it the local player entity. The field is written by `GameBeing::setTargetId` (`0x00e003c0`, `thiscall(this, uint)`, `ret 4`: `[this+0xfc] = id`), called from `GameProxyPlayer::setTarget` (`0x00de9ae0`). For beings other than the local player, the `onTargetUpdate` handler `0x00e018b0` (field `"TargetId"`) performs the same store inline; for the local player it calls `setTarget` instead. VERIFIED. That `[[EM+0x8c]+0x4]` is the same pointer as the `GameBeing` after `__RTDynamicCast` (no adjustment) is INFERRED: **first live check** is to read `+0xfc` with a target selected and compare it with the server's `setTargetID`.
- **Lua `Unit.Target`.** A slot, not an entity id. `GameEntityManager::unitSlotToEntityId` is `0x00c67410` (a thunk to `0x00c67120`, `thiscall(this, slot)`, `ret 4`) and reads the `std::map<int,int>` at `EM+0x130`. The writer is `0x00c67bd0` (`thiscall(this, slot, entityId)`), which also raises `Event_UI_UnitMappingChanged` through `0x00a372f0`.
- **What each path sends as `TargetID`.**
  - Hotbar (`useAction`): `[GameBeing+0xfc]` of the local player; with the second `useAction` argument true, the local player's own id (`[entity+0xc]`). VERIFIED (`0x00e3cdb2` / `0x00e3cdcd`).
  - Lua `useAbility(id, Unit.Target)` (`Ability.lua:181`, the trainer UI): `0x00ad78e0` maps the slot to an entity, casts it to `GameBeing` and sends **that entity's** `+0xfc`, with 0 when the cast fails. If `+0xfc` is the being's *own* target, this path sends the target's target; that is how the code reads, and it is rarely exercised. INFERRED meaning, VERIFIED code.

### Getting the Mercury packet sequence

`Channel::send` (`0x01576F90`, `fastcall(Channel*)`) is **too early**. It only detaches the bundle (`*(channel+0x28)`) and hands it to `Nub::send`'s front end `0x0157ec70`, which pushes a send record onto a TBB queue (`nub+0x150`). No sequence number exists yet.

The sequence is assigned on the **network thread** in `Nub::send` (`0x01582160`, `thiscall(Nub*, addr, Bundle*, Channel*)`), once per packet, in order:

- reliable channel send: `seq = FUN_0158bb40(channelInternal)` (`0x0158bb40`: `eax = [ecx+0x4c]; [ecx+0x4c] = (eax+1) & 0x0fffffff; return eax`). It has one caller, the `Nub::send` finalise branch at `0x0158258c`.
- otherwise: the Nub-level counter at `nub+0x98`.
- the value is stored at `packet+0x44` and appended as the 4-byte footer when the packet flag `0x40` is set. This is the same field the server logs as the inbound packet sequence (`client-mercury-receive-path.md` records `+0x44` for the receive side).

Join recipe (INFERRED, to be proven by the primitive test):

1. In the `Route` hook (main thread), append `{method, decoded args}` to a thread-local pending list.
2. In a `Channel::send` hook, take the list and tag it with the bundle pointer `*(channel+0x28)`.
3. In a `Nub::send` hook, set a thread-local current bundle from argument 2; a hook on `0x0158bb40` records each returned sequence into that bundle's list. At `Nub::send` exit, emit `client.ability.sent` with `first_seq` and `last_seq`.

A bundle can span several packets, so the join key is a **range** (`first_seq..last_seq`), not an exact packet; the server's receipt row is joined by `seq in range`. The counter is 28 bits (`& 0x0fffffff` at `0x0158bb4c`), so a range can wrap: test membership with modular arithmetic, `((seq - first_seq) & 0x0fffffff) <= ((last_seq - first_seq) & 0x0fffffff)`, never `first <= seq <= last`. An ability message is small and is almost always in the first packet. The hook on `0x0158bb40` avoids reading the bundle after `Nub::send` returns, when it may be freed.

## AB-C2: the press path and its drop branches

```text
ActionButtonMod.onActionPress (ActionButtons.lua:157)         click or hotkey (Actions['ActionButton'..n])
  └─ useAction(actionId, false)  Lua thunk 0x00aa94e0
       └─ FUN_00ad9580(actionId, selfFlag)                     ActionBar = [[EM+0x8c]+0x4c]; slot = FUN_00e3d190(actionId-1)
            └─ action->vtable[9](selfFlag)                     AbilityAction 0x00e3cd90 / PetAbilityAction 0x00e3cf40
                 └─ FUN_00d2afc0(abilitySet, abilityId, targetId)      AbilitySet = [[EM+0x8c]+0x3c]
                      └─ FUN_00d2a000(abilityId)               map find at abilitySet+0x28
                      └─ FUN_00d2ae40(abilitySet, rec, targetId)
                           ├─ rec+0x48 == 3 ─► ground reticle (0x00dea330); useAbilityOnGroundTarget is sent when it ends
                           └─ else ─► Event_NetOut_UseAbility {AbilityID = rec+0xc, TargetID}  ─► post 0x00cacd50
```

**Where the press can be dropped** (all VERIFIED unless marked):

| # | Place | Branch | Condition | Visible to the player |
|---|---|---|---|---|
| 1 | `ActionButtons.lua:99,160-164` | Lua | `actionsEnabled` false, or the button has no action (`actionId` nil or 0) | no |
| 2 | `useAction` thunk `0x00aa94e0` | `JZ` at `0x00aa94fc`, `0x00aa9512`, `0x00aa9526` to `0x00aa9569` | argument 1 not a number (`lua_isnumber`), argument 2 not a boolean, or **a third argument is present** (`0x00403280` is `tolua_isnoobj`) | Lua error `#ferror in function 'useAction'.` (`0x01945108`) |
| 3 | `FUN_00ad9580` | `JZ` at `0x00ad959e` to `0x00ad95ae` | `actionId-1 >= 200` (`FUN_00e3d190` returns 0) or an empty slot | no |
| 4 | `PetAbilityAction` `0x00e3cf40` | `JZ` at `0x00e3cfb1` | the pet entity (`this+0x10`) does not resolve to a `GamePet` | no |
| 5 | **`FUN_00d2afc0`** | **`JZ` at `0x00d2afcf` to `0x00d2afde`** | **the ability id is not in the client's `AbilitySet` map** | **no** |
| 6 | `RouteOutgoingEntityRpc` | `JZ` at `0x00c6fc68` | no `ServerConnection` (`[EM+8]` null) | no |
| 7 | same | `JZ` at `0x00c6fc77` | not connected (`0x00dd6130`: `[conn+0x30c] != 0`) | no |
| 8 | same | `JZ` at `0x00c6fca8` | the local player entity is not in the world map (`0x00dd0de0` returns 0) | no |
| 9 | same | `JZ` at `0x00c6fcd2` | the entity's type has no description mapping (`0x0158e710` returns `0xffff`) | no |
| 10 | same | `JZ` at `0x00c6fd1d` / `0x00c6fd41` | **the entity's class chain does not contain the method's class** (the walk uses `0x0158eca0` to climb parents) | no |

Row 5 is the finding that matters for the unexplained missing heals (audit B-15): a press for an ability the server never taught the client (`onKnownAbilitiesUpdate`, `onAbilityTreeInfo`, a cooldown timer) is discarded at `0x00d2afcf` with no log, no error and no wire message. The `AbilitySet` is filled by `FUN_00d2b020` (`addAbilityIfAbsent(id)`, `thiscall(set, id)`, `ret 4`), whose callers are the known-abilities handler `0x00dec500` (field `"AbilityData"`), the ability-tree handler `0x00e19a30` (field `"AbilityLists"`), the `GameEntityManager` handlers `0x00c68110` (timer update: `"Type"`, `"SourceID"`, `"ID"`, `"BigWorldTimeComplete"`) and `0x00c68460` (field `"AbilityID"`), `0x00d211c0`, and `0x00d39eb0` (field `"aAbilityList"`).

Row 10 is how the client refuses a method it should not send. A non-GM player entity is an `SGWPlayer`; its class chain never contains `SGWGmPlayer`, so the `gmDebug*` methods fall out here ([native-combat-debug.md](native-combat-debug.md)).

**Not checked by the client on this path** (VERIFIED by reading `0x00ad9580`, `0x00e3cd90`, `0x00d2afc0`, `0x00d2ae40` in full): cooldown, global cooldown, range, line of sight, target hostility or presence, caster dead, resources, and any in-flight cast. Those refusals are the server's alone, and the client shows them only through `onErrorCode` and the cooldown overlay.

`client.ability.press_dropped` reasons that follow from this: `no_action` (rows 1, 3), `bad_args` (row 2), `not_known` (row 5), `pet_missing` (row 4), `not_connected` (rows 6 to 9), `class_mismatch` (row 10). The GamePet send behind row 4 has three more drops, found while implementing AB-C2 ([addendum](#addendum-reads-made-while-implementing-ab-c1-and-ab-c2)). As implemented, rows 9 and 10 are reported together as `class_mismatch`: both return before a `start*Message`, and only game code tells them apart. `on_cooldown`, `no_target`, `out_of_range` and `dead` **do not exist on the client** and should not be offered as reasons.

## AB-C3: inbound payloads

Two seams, used for different jobs.

**Seam A: wire bytes at `EntityManager::onEntityMethod` (`0x00dd2b80`).** Already hooked by `client.mercury.entity_method`. Its third argument is a `MemoryIStream` (`vtable 0x01b18e38`): `+4` error flag, `+8` read cursor, `+0xc` end. Slot 1 (`0x0157aff0`) advances the cursor and returns the old one, slot 2 (`0x0157af60`) is `end - cursor`, slot 3 (`0x0157af70`) skips to the end, slot 4 (`0x0157b030`) peeks a byte. A hook can read `[cursor, end)` without consuming it. The bytes are the arguments in `.def` order (the existing byte-exact decoders in `cimmeria-wire` apply), after the message id.

The flat method index comes from `FUN_01590bb0(msg_id, N, stream)`, where `N` is the receiving class's method count: `k = 0x3e - (N + 0xc0) / 0xff`; an id below `k` is the index itself, otherwise the index is `k + 0x100*(id - k) + stream.peekThenConsumeByte()`. For `SGWPlayer` (157 methods) `k = 61`, which reproduces the `0xBD`-plus-sub-byte rule in [client-method-dispatch-table.md](../../protocol/client-method-dispatch-table.md). `SGWMob` and `SGWPet` have fewer than 63 methods, so their `k` is 62: a decoder must use the class's own `k`.

Threading: the dispatcher runs on a network thread (`0x00c6f8f0` header comment). A message for an entity not yet in the world is **queued** and delivered later by the replay (`0x00dd1e40`), so the receive time and the apply time differ for non-local entities.

**Seam B: the event at `0x00a372f0` (main thread).** Gives the event name from the class type descriptor, the subject entity id, and the bag. It also sees the `Event_UI_*` events that mean "the client applied it". Scalar fields are read with `GetInt` / `GetFloat` / `GetByte`. Use it for the apply side and for scalar-only events; use Seam A for arrays.

| Inbound | Event class (name in the binary) | Subscriber and handler | Fields (bag keys) | Notes |
|---|---|---|---|---|
| `onEffectResults` (14) | `Event_NetIn_onEffectResults` | `GameEntityManager`, `SequenceManager`, `CombatQueue` (`CombatQueue.cpp`, one function `0x00eb1630` holds all its asserts) | `SourceID`, `AbilityID`, `EffectID`, `TargetID`, `ResultCode`, `ClientEffectResultList` | the effect id is the server's `cast_id`; the list is an array, so decode from bytes |
| `onTimerUpdate` (12) | `Event_NetIn_TimerUpdate` | `GameEntityManager` `0x00c68110`, `MissionSet`, `DialogController`, `GameProxyPlayer`, `GameBeing`, `EffectSet` `0x00e09160`, `SGW::Crafting`, `CooldownManager` `0x00ea6af0` | `ID`, `Type`, `SourceID`, `SecondaryId`, `TotalTime`, `BigWorldTimeComplete` | all scalar; eight subscribers (type map in `ability-resolution-pipeline.md`) |
| `onErrorCode` (121) | `Event_NetIn_onErrorCode` | a `FreeCallback` bound to `Communicator`, handler `0x00cf33e0` (the function is **not defined in the Ghidra project**; prologue verified from bytes) | `SystemID`, `InstanceID`, `ErrorCodeID` (the assert string `0x019b81d8` names `ErrorCodeID`) | scalar |
| `onStatUpdate` (20) | `Event_NetIn_onStatUpdate` | `GameBeing`, per entity, handler `0x00e01f40`, per-stat functor `0x00e004e0` | `Stats` (array of `{StatId, Min, Current, Max}`) | array; INFERRED `{statId, Min, Current, Max}` maps to the functor's four arguments |
| `onStatBaseUpdate` (21) | `Event_NetIn_onStatBaseUpdate` | `GameBeing`, handler `0x00e02060`, functor `0x00e005b0` | `Stats` | same shape; stores the base values |
| `onStateFieldUpdate` (19) | `Event_NetIn_onStateFieldUpdate` | `GameBeing` handler `0x00e01c90` | `bStateField` | scalar |
| `onTargetUpdate` (16) | `Event_NetIn_onTargetUpdate` | `GameBeing` handler `0x00e018b0` | `TargetId` | the local player's goes through `GameProxyPlayer::setTarget` |
| `onSequence` (1) | `Event_NetIn_onSequence` | `SequenceManager` handler `0x00d05790` | `KismetEventSetSeqID`, `SourceID`, `TargetID`, `PrimaryTarget`, `ImpactTime`, `NameValuePairs`, `ViewType`, `InstanceId` | scalars plus one array |
| `onKnownAbilitiesUpdate` (101) | `Event_NetIn_KnownAbilitiesUpdate` | `GameProxyPlayer` handler `0x00dec500` | `AbilityData` (array of int) | array |
| `onAbilityTreeInfo` (141) | `Event_NetIn_AbilityTreeInfo` | handler `0x00e19a30` | `AbilityLists` | array of arrays |
| `Ability_Interrupt` | not a method | it is an `onSequence` whose event id is **1002** (`EVENT_ABILITY_INTERRUPT`, `crates/cell-catalog/src/cell/spawner/abilities.rs:12`) | n/a | filter `onSequence` on `KismetEventSetSeqID` or the cooked event set |
| `onSendCombatDebug`, `onSendEventDebug` | **not client methods** | n/a | n/a | see [native-combat-debug.md](native-combat-debug.md) |

The per-entity `GameBeing` handlers are registered by `FUN_00e02ab0` (`GameBeing::registerCallbacks`; the matching unregister is `FUN_00e02c00`), in this order: appearance `0x00e01360`, level `0x00e016b0`, target `0x00e018b0`, being name `0x00e01a20`, name id `0x00e02980`, melee range `0x00e01b80`, state field `0x00e01c90`, stats `0x00e01f40`, base stats `0x00e02060`, alignment `0x00e02180`, faction `0x00e02280`, archetype `0x00e01240`, reload timer `0x00e02380`. The mapping of each address to its event is VERIFIED from the field names each handler reads, except the two stat handlers, which are told apart by what their functors write.

## AB-C4: what the client applied

| Kind | Seam | Address and shape | What a hook reads | Status |
|---|---|---|---|---|
| Effect bar add and refresh | `EffectSet` timer handler | `0x00e09160`, `thiscall(this = EffectSet, event*)` | `Type` (only type **5** continues), `SecondaryId`, `BigWorldTimeComplete`, then on a new entry `TotalTime`, `SourceID`, `ID`. A refresh is the `FUN_00e08570(this, SecondaryId)` hit that extends the interval at `this+0x38` (`FUN_00c6d1c0`). A new entry is a 0x20-byte `FUN_00d2d740` object pushed with `FUN_015fbd50` and announced by `FUN_00e0a9e0` | VERIFIED |
| Effect bar notify | `FUN_00e0a9e0` | `thiscall(this, int* entry)`; posts a type-9 record through `FUN_00e0a2d0` (`FUN_004366b0(DAT_01ea56d4, ...)`) | the entry pointer | addresses VERIFIED, "this raises `Event_UI_UnitEffectsUpdate`" INFERRED |
| Effect bar remove | `FUN_00e0a810` | `thiscall(this, int* secondaryId)`; builds an event with `CategoryId` = 9 and `Key` = the id and posts it with `FUN_00cfa190`. Its only caller is `FUN_00e0a9e0` (at `0x00e0aa43`), which `FUN_00e09160` calls once, at `0x00e094b6`, after a new entry; `FUN_00e0a9e0` takes this branch when its lookup `FUN_00e0a6f0` returns null and otherwise posts the add through `FUN_00e0a2d0` | the id | VERIFIED. No other native call removes an entry, so expiry on the bar is clock-driven (`getEffectInfo` computes the remaining time): INFERRED |
| Cooldown on a hotbar slot | `CooldownManager` timer handler | `0x00ea6af0`, `thiscall(this, event*)`; gate `SourceID == *(this)`; calls `FUN_00ea6120` (interval query), updates the interval tree at `this+0x14`, then `FUN_00ea62b0` | `SourceID`, `Type`, `ID`, `TotalTime`, `BigWorldTimeComplete` | VERIFIED |
| Cooldown UI callback | `FUN_00ea62b0` | `thiscall(this, type, id, float remaining, uint)`; finds a callback with `FUN_00adff40(this+4, type)` and calls it as `(owner, id, &remaining)` | `type`, `id`, `remaining` | address VERIFIED, "this is what drives `Event_UI_ActionCooldown`" INFERRED |
| Stat on the local player or target | `GameBeing` per-stat functors | `0x00e004e0` (current), `0x00e005b0` (base); `thiscall(this = GameBeing, statId, a, b, c)`; they store into the map at `this+0x160` (current at `+0..+8`, base at `+0xc..+0x14`) | `[this+0xc]` is the entity id; the four arguments | VERIFIED code; INFERRED argument order `{statId, Min, Current, Max}` from `alias.xml` |
| State flag | `GameBeing::onStateFieldUpdate` | `0x00e01c90`, `thiscall(this = GameBeing, event*, u32)`; already hooked | entity id `[this+0xc]`; new state `GetInt(event, "bStateField")`; the old state is `[this+0x158]` **at entry**, so the changed bits are `old ^ new` | the layout is VERIFIED in `state-flag-broadcast.md`; reading the old value at entry is INFERRED |
| Ability known to the client | `AbilitySet` insert | `0x00d2b020`, `thiscall(set, id)`, `ret 4` | the ability id | VERIFIED |

## AB-C5: what the client showed

**Combat text and chat lines are Lua.** Floating combat text is `SCTMod.onUnitCombat` (`SCT.lua:83`, subscribed to `Events.UnitCombat`) calling `SCTMod.createEventWindow` (`SCT.lua:151`). The combat chat line is `CHAT_onUnitCombat` (`ChatEvents.lua:204`) calling `CHAT_dispatch`, which calls `ChatMod.onMessageReceived(window, '', UISpeakerFlags.None, UIChannel.Combat, 'Combat', message)`. A feedback line is `ChatMod.onMessageReceived(..., UIChannel.Feedback, ...)`. There is no native "add SCT" or "add chat line" function to hook; the seams are:

- **Native to Lua event.** The CME `Event_UI_UnitCombat`, `Event_UI_UnitEffectsUpdate`, `Event_UI_ActionCooldown`, `Event_UI_AbilityCooldown` (class names VERIFIED by string search) are dispatched through `0x00a372f0` like any event (`0x00c67bd0` shows the call shape for `Event_UI_UnitMappingChanged`), and reach Lua through `SGWScriptedWindow::GameEventHandler<Event>`. For a UI event the third argument of `0x00a372f0` is a plain struct, not a bag (`Event_UI_UnitMappingChanged` passes `{int slot}` with `NoSubject` as the base descriptor, `0x00c67bd0`). The field layout of `Event_UI_UnitCombat` and the other UI events is **UNRESOLVED**.
- **Lua level.** The lab's `client_combat_log` and `client_chat_log` already wrap `SCTMod.onUnitCombat` and the chat window. The player-shipped hook should use the `lua_pcall` IAT seam that exists, with an allowlist of the three Lua functions above.
- **The `onErrorCode` feedback line.** The handler is `0x00cf33e0` (see AB-C3). The Lua binding `writeLocalFeedback` is a thunk at `0x00ac67e0` (prologue verified from bytes; error string `0x0194f3b8`). That `onErrorCode` ends in a `writeLocalFeedback` call is INFERRED; **next action**: create a function at `0x00cf33e0` in the project and read its tail.

**The `onSequence` drop branch.** `SequenceManager::onSequence` is `0x00d05790` (`thiscall(this, event*)`). It reads `"SourceID"` with `GetInt` (string `0x019ba3c4`), resolves it with `0x00dd0de0(EM, id, 0)`, and at `0x00d0585a`/`0x00d0585c` (`CMP EAX,EBX` / `JZ 0x00d05951`) **returns silently when no client entity exists for the source**. VERIFIED. A hook at the entry that reads `KismetEventSetSeqID`, `SourceID` and `InstanceId` and calls `0x00dd0de0` itself can emit `client.sequence.dropped` with `reason = no_source_entity` before the original runs. The later drop branches (`0x00d06f30` cache-ready, `0x00d06dd0` culling) are in `npc-attack-presentation.md` and are already hooked.

## Hook gate fingerprints

The first 16 bytes of each function, read from the QA image (`0x` addresses are not relocated). A function whose prologue is a relative `call` or `jmp` should be pinned on the opcode and the target.

| Address | Bytes |
|---|---|
| `0x00c6fc40` Route | `6a ff 68 ea 50 6f 01 64 a1 00 00 00 00 50 64 89` |
| `0x00d43dc0` NetOut callback | `8b 44 24 04 8b 51 08 50 8b 41 04 8b 4c 24 0c 52` |
| `0x00a372f0` dispatch | `51 53 8b 5c 24 18 55 56 57 8b f1 8d 44 24 24 50` |
| `0x00a374a0` subscribe | `83 ec 08 53 8b 5c 24 14 8b 03 8b 50 08 55 56 8b` |
| `0x00aa94e0` `useAction` thunk | `83 ec 10 56 8b 74 24 18 8d 44 24 08 50 6a 00 6a` |
| `0x00aa2910` `useAbility` thunk | `83 ec 0c 56 8b 74 24 14 8d 44 24 04 50 6a 00 6a` |
| `0x00ad9580` | `e8 4b d5 18 00 8b 54 24 04 05 8c 00 00 00 8b 00` |
| `0x00ad78e0` | `8b 44 24 08 50 e8 e6 f1 18 00 8b c8 e8 1f fb 18` |
| `0x00e3cd90` `AbilityAction::execute` | `56 57 8b f9 e8 37 9d e2 ff 05 8c 00 00 00 80 7c` |
| `0x00e3cf40` `PetAbilityAction::execute` | `56 57 8b f9 e8 87 9b e2 ff 05 8c 00 00 00 80 7c` |
| `0x00d2afc0` | `8b 44 24 04 56 50 8b f1 e8 33 f0 ff ff 85 c0 74` |
| `0x00d2a000` | `83 ec 08 53 55 56 57 8d 71 28 8d 44 24 1c 50 8d` |
| `0x00d2b020` | `83 ec 0c 56 57 8b 7c 24 18 57 8b f1 e8 cf ef ff` |
| `0x00d2ae40` | `64 a1 00 00 00 00 6a ff 68 9f cc 6f 01 50 64 89` |
| `0x00e3cba0` / `0x00e3cc20` / `0x00d434d0` | `6a ff 68 50 82 70 01 64 a1 00 00 00 00 50 64 89` (all three: tell them apart by address) |
| `0x00dd2b80` `onEntityMethod` | `6a ff 68 e4 52 70 01 64 a1 00 00 00 00 50 64 89` |
| `0x00c6f8f0` dispatcher | `6a ff 68 bf 50 6f 01 64 a1 00 00 00 00 50 64 89` |
| `0x01576f90` `Channel::send` | `6a ff 68 f8 2f 79 01 64 a1 00 00 00 00 50 64 89` |
| `0x01582160` `Nub::send` | `6a ff 68 0b 3f 79 01 64 a1 00 00 00 00 50 64 89` |
| `0x0158bb40` sequence counter | `8b 41 4c 8d 50 01 81 e2 ff ff ff 0f 89 51 4c c3` |
| `0x00e09160` `EffectSet` timer | `6a ff 68 ab 88 70 01 64 a1 00 00 00 00 50 64 89` |
| `0x00ea6af0` `CooldownManager` timer | `6a ff 68 4f 15 71 01 64 a1 00 00 00 00 50 64 89` |
| `0x00ea62b0` | `83 ec 14 8b 44 24 18 8b 54 24 1c 53 55 56 57 89` |
| `0x00d05790` `SequenceManager::onSequence` | `6a ff 68 23 a8 6f 01 64 a1 00 00 00 00 50 64 89` |
| `0x00e018b0` target handler | `6a ff 68 f3 84 70 01 64 a1 00 00 00 00 50 64 89` |
| `0x00e01f40` / `0x00e02060` stat handlers | `6a ff 68 74 85 70 01 ...` / `6a ff 68 96 85 70 01 ...` |
| `0x00e003c0` `setTargetId` | `56 8b f1 8b 46 08 33 c9 85 c0 57 74 06 8d 88 ac` |
| `0x00de9ae0` `GameProxyPlayer::setTarget` | `64 a1 00 00 00 00 6a ff 68 98 71 70 01 50 64 89` |
| `0x00cf33e0` `onErrorCode` handler | `64 a1 00 00 00 00 6a ff 68 fc 9e 6f 01 50 64 89` |
| `0x00ac67e0` `writeLocalFeedback` thunk | `64 a1 00 00 00 00 6a ff 68 38 8c 6d 01 50 64 89` |
| `0x00dd0de0` `findEntity` | `83 ec 08 53 55 56 57 8b f9 8d 44 24 1c 50 8d 4c` |
| `0x00c67410` slot to entity id | `e9 0b fd ff ff cc cc cc cc cc cc cc cc cc cc cc` (a `jmp` to `0x00c67120`) |
| `0x00dd6130` `isConnected` | `33 c0 39 81 0c 03 00 00 0f 95 c0 c3 cc cc cc cc` |

## Corrections to earlier documents

- The signatures of every function AB-C1 and AB-C2 hook or call were re-read from the image, with each `ret N` and the `ECX` evidence, in the [addendum](#addendum-reads-made-while-implementing-ab-c1-and-ab-c2). All of them match this document.
- [client-wire-emit-suppression.md](client-wire-emit-suppression.md) names `0x00d2b020` the **in-flight queue gate** and says `FUN_00d2a000` "returns the head of an in-flight ability queue at `this+0x228`". Both are wrong. `0x00d2a000` is a map lookup by ability id (`std::map<int, AbilityData*>` at `AbilitySet+0x28`, `thiscall(set, id)`, `ret 4`), and `0x00d2b020` is `addAbilityIfAbsent`. The real silent drop is the not-found branch at `0x00d2afcf`. The same document says the `useAbility` thunk requires the third argument to **exist**; `0x00403280` is `tolua_isnoobj`, so the call fails when a third argument is **present**. And the hotbar does not go through the `useAbility` thunk at all; it goes through `useAction`.
- [ability-resolution-pipeline.md](ability-resolution-pipeline.md) calls `0x00d2a000` `AbilitySet_GetSlotByIndex`; it is a find by id.
- The `client-instrumentation-hookpoints.md` statement that `0x00c6fc40` is the single exit is **confirmed** with the caller evidence above.
- The cell dispatch table gave `confirmationResponse` the arguments `INT8 choice`. The `.def`, the client event (`aEffectId` int, `aAccepted` byte) and the server decoder agree on `INT32 aEffectId, UINT8 aAccepted`; the table is fixed in the same pull request.

## Addendum: reads made while implementing AB-C1 and AB-C2

Added 2026-10-04 by the AB-C1/AB-C2 implementation. Method: the QA `SGW.exe` image on disk, disassembled with capstone (no debugger, no live client). Every fingerprint in the [table above](#hook-gate-fingerprints) matched the image byte for byte.

- **The GamePet send has three more silent drops (VERIFIED code, meanings UNRESOLVED).** Row 4's `PetAbilityAction::execute` calls `0x00d3a820` (`thiscall(GamePet*, abilityId, targetId)`, `ret 8`; callers `0x00e3cfba` and `0x00ad7bfe`), which returns without sending when: `[pet+0x38] & 0x400` is clear (`JE` at `0x00d3a84a`); the ability is not in the pet's own `AbilitySet` at `[pet+0x174]` (`FUN_00d2a000`, `JE` at `0x00d3a862`); or the ability record has bit `0x8` of `+0x98` set (`JNE` at `0x00d3a875`). Otherwise it builds the event with the keys `aAbilityId`, `aEntityId`, `aTargetId` (strings at `0x019bbe34`) and posts it through `0x00cb0350`, not `0x00cacd50`. The telemetry reports these as `not_known` (the pet's set) and as `pet_state_flag` / `pet_ability_flag`. **Next action:** find the writers of `GamePet+0x38` bit `0x400` and of `AbilityData+0x98` bit `0x8`.
- **The router's third exit is a wrapper.** `0x00dd8010` is 15 bytes: `startEntityMessage(msgId, 0)` with the same `this`, then `ret 4`. So observing `startEntityMessage` (`0x00dd6a60`, `thiscall(conn, msgId, entityId)`, `ret 8`) and `startProxyMessage` (`0x00dd6980`, `thiscall(conn, msgId)`, `ret 4`) covers every send; both return the message stream. Prologues: `64 a1 00 00 00 00 6a ff 68 38 55 70 01 50 64 89` and `64 a1 00 00 00 00 6a ff 68 20 55 70 01 50 64 89`.
- **A base-route branch the drop table lacks.** For a base method (`flags & 3 == 2`) whose entity is not the local player (the id from `[EM+0x14]` against `[entity+0xc]`, `0x00c6fd62`), the router logs `"The subject of an outgoing base rpc '%s..."` (wide string `0x019abae8`, through `0x00482ff0`) and goes on to the argument loop with a null stream. Every method in the AB-C1 allowlist is a cell method, so the telemetry does not need it; what the argument writers do with a null stream is UNRESOLVED.
- **Row 2 is a Lua error, not a silent return.** The `useAction` thunk's failure target `0x00aa9569` calls `tolua_error` (`0x00402f40`) with `#ferror in function 'useAction'.`, which raises through the C++-compiled `lua51.dll`. The `useAbility` thunk (`0x00aa2910`) has the same shape (failure target `0x00aa2997`), checks its second argument with `tolua_isnumber` (`0x00403330`), and calls `0x00ad78e0` as `cdecl(abilityId, unitSlot)`.
- **`Channel::send` can return without detaching the bundle.** `0x01576f90` is `thiscall(Channel*) -> int` with no stack arguments; it returns 0 at `0x01576fd6` when the bundle is empty and not forced, leaving it at `channel+0x28`. A join must tag the bundle before the call (the network thread may send it before the caller regains control) and undo the tag on this path.
- **`Nub::send` frees the bundle before it returns.** Its epilogue calls a virtual on the bundle (`0x01582d96`) just before `ret 0xc`. The join keys on the pointer value only and never reads the bundle after the call.

- **Calling conventions re-verified for every AB-C1/AB-C2 hook (2026-10-04).** A sibling packet found three handlers in this document listed with one argument where they take two (`ret 8`), so every function the AB-C1/AB-C2 DLL hooks or calls was checked against the image rather than this document. Method: follow every reachable branch from the entry and collect each `ret`; `this` is shown by the first read of `ECX` before any write. All of them match the signatures above and in `hooks/inline_hooks/ability/mod.rs`. The cdecl argument counts come from the `[esp+N]` reads (two for `0x00ad9580`, one `lua_State*` for each thunk).

  | Function | Address | Every `ret` observed | `this` (ECX) evidence | Convention used by the detour |
  |---|---|---|---|---|
  | `useAction` thunk | `0x00aa94e0` | `0x00aa9568 ret`, `0x00aa9582 ret` | none (cdecl) | `cdecl int(lua_State*)` |
  | `useAbility` thunk | `0x00aa2910` | `0x00aa2996 ret`, `0x00aa29b0 ret` | none (cdecl) | `cdecl int(lua_State*)` |
  | `FUN_00ad9580` | `0x00ad9580` | `0x00ad95ae ret` | none (cdecl; reads `[esp+4]`, `[esp+8]`) | `cdecl (actionId, self)` |
  | `FUN_00d2afc0` | `0x00d2afc0` | `0x00d2afdf ret 8` | `mov esi, ecx` at `0x00d2afc6` | `thiscall`, 2 stack args |
  | `FUN_00d2ae40` | `0x00d2ae40` | `0x00d2aec2 ret 8`, `0x00d2afb5 ret 8` | `mov esi, ecx` at `0x00d2ae60` | `thiscall`, 2 stack args |
  | `PetAbilityAction::execute` | `0x00e3cf40` | `0x00e3cfc1 ret 4` | `mov edi, ecx` at `0x00e3cf42` | `thiscall`, 1 stack arg |
  | GamePet send | `0x00d3a820` | `0x00d3a97b ret 8` | `mov edi, ecx` at `0x00d3a83a` | `thiscall`, 2 stack args |
  | `startEntityMessage` | `0x00dd6a60` | `0x00dd6b4a ret 8` | `mov esi, ecx` at `0x00dd6a79` | `thiscall`, 2 stack args, returns the stream |
  | `startProxyMessage` | `0x00dd6980` | `0x00dd6a52 ret 4` | `mov esi, ecx` at `0x00dd6999` | `thiscall`, 1 stack arg, returns the stream |
  | `Channel::send` | `0x01576f90` | `0x01576fe8 ret`, `0x0157704c ret` | `mov esi, ecx` at `0x01576fa7` | `thiscall`, no stack args, returns int |
  | `Nub::send` | `0x01582160` | `0x01582db6 ret 0xc` | `mov [esp+0x60], ecx` at `0x01582192` | `thiscall`, 3 stack args |
  | sequence counter | `0x0158bb40` | `0x0158bb4f ret` | `mov eax, [ecx+0x4c]` at `0x0158bb40` | `thiscall`, no stack args, returns the seq |
  | `RouteOutgoingEntityRpc` | `0x00c6fc40` | `0x00c6ffa5 ret 0x10` | none (stdcall) | `stdcall`, 4 args |
  | `GetInt` / `GetFloat` / `GetByte` (called) | `0x00e3cba0` / `0x00e3cc20` / `0x00d434d0` | `0x00e3cc16` / `0x00e3cc9d` / `0x00d43546`, each `ret 8` | `push ecx` at `0x00e3cbb9` / `0x00e3cc39` / `0x00d434e9` | `thiscall(event, name, out)`, result in `AL` |

  The functions the press chain hooks have no meaningful return value, but each leaves something in `EAX` (`FUN_00d2afc0` leaves `FUN_00d2a000`'s result). The detours return the original's `EAX` unchanged instead of clobbering it.

## Open questions

| Question | Evidence that resolves it | Next action |
|---|---|---|
| Is `[[EM+0x8c]+0x4]+0xfc` the local player's current target, with no cast adjustment? | a live read with a target selected | first lab run after the hook lands, compare with `setTargetID` on the server |
| How are array arguments stored in the bag (`BasicPropertyList`)? | the reader `0x00dfe8d0` and the list element type | decode arrays from wire bytes (Seam A); read `0x00dfe8d0` only if a bag-side array read is wanted |
| What do the `Event_UI_UnitCombat` and `Event_UI_ActionCooldown` objects carry, and which function emits them? | the emitter inside `CombatQueue::onEffectResults` `0x00eb1630` and the `CooldownManager` callback table at `this+4` | decompile `0x00eb1630` and the callback registered for each timer type |
| Does `onErrorCode` end in `writeLocalFeedback`? | the body of `0x00cf33e0` | create a function there (the region is undefined in the project) and read the tail |
