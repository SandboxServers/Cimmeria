# Client entity lifecycle: enterAoI, createEntity, enterWorld, the deferred-message queue and the appearance request

> **Date**: 2026-09-28
> **Method**: Ghidra (MCP, `SGW.exe` QA build) decompile and disassembly of the `EntityManager` message handlers; every `ret N` read from the function's epilogue bytes, so the argument counts below are exact even where the decompiler dropped a parameter.
> **Status**: statically verified. Nothing here has run in the live client yet. Items marked **inferred** are read from code shape, not from a trace.
> **Feeds**: the `client.entity.*`, `client.mercury.entity_*`, `client.net.out`, `client.lua.error` and `client.cme.catalog` events ([client-telemetry.md](../../architecture/client-telemetry.md)); the open invisible-guard bug (#838).
> **Related**: [request-entity-update-cache-stamp.md](request-entity-update-cache-stamp.md), [black-market-client-io.md](black-market-client-io.md) §2, [mercury-protocol-internals.md](mercury-protocol-internals.md) ("Receive Ordering"), [cme-event-signal.md](cme-event-signal.md).

## Summary

The client renders an entity only after `EntityManager::enterWorld` has run for it, and `enterWorld` is what asks for its appearance. Whether `enterWorld` runs is decided by two counters and two maps, not by the create message alone:

- `enterAoI(id, space, vehicle)` says the entity is in range. It bumps an **enter count**.
- `createEntity(id, type, space, vehicle, stream)` carries its state. It enters the world only if the enter count is at least 1 when it arrives; otherwise it **parks** the entity in a cache map.
- An `enterAoI` that arrives for an entity already parked in the cache map bumps the count but **does not enter the world**, unless the entity is the local player or a client-only entity (id `>= 0x40000000`).
- While an entity is not in the world map, every method and property the server sends it is **queued**, and the queue is flushed only when the entity enters the world.

So an NPC that is created while its enter count is 0 stays invisible, silent (its appearance methods sit in the queue), and, from the server's point of view, fully created and acknowledged. This is a strong candidate mechanism for the invisible Cellblock guard (#838), which the client creates but does not render. **It is a hypothesis until a live session shows `client.entity.create` with `outcome = parked` for the missing guard.** The new events exist to confirm or kill it.

## The manager and its maps

`EntityManager` is the singleton at `0x01ef244c` (getter `0x00c66ad0` / `0x00dd05a0`, constructor `0x00dd3330`). Its vtable is `0x019aaec4`; the `ServerMessageHandler` copy at `0x019ce980` holds the same entries three slots earlier (the create handler is at `0x019ce98c`). Fields used here, all read from the decompiles below:

| Offset | What | Evidence |
|---|---|---|
| `+0x08` | `ServerConnection*` | `0x00c6fc40`: `[[0x01ef244c]+8]` |
| `+0x0c` | local player `Entity*` | `0x00dd2b80`: `iVar3 = *(this+0xc); *(iVar3+0xc) != param_1` |
| `+0x14` | local player entity id | `0x00c6fc40`: `*(iVar5+0x14)` compared with `*(entity+0xc)` |
| `+0x18` | `std::map<int, Entity*>`: entities **in the world** | `0x00dd1d00` inserts here; `0x00dd0f70` (`isInWorld`) looks here |
| `+0x24` | `std::map<int, Entity*>`: created but **not entered** (the "cache") | `0x00dd2270` puts a low-count entity here; `0x00dd24f0` and `0x00dd2800` look here first |
| `+0x30` | `std::map<int, record>`: **pending enter** records (`enterAoI` before `createEntity`); the record's first word is the enter count | `0x00dd24f0`: `*piVar5 = *piVar5 + 1` on `BW__unknown_00dd5120(this+0x30, id)`; **inferred** that the count is the first word of the mapped value |
| `+0x3c` | `std::map<int, vector<8-byte entry>>`: **deferred method and property queue** | `0x00dd2b80` / `0x00dd29d0` append; `0x00dd1e40` drains; the vector's begin and end are at node `+0x14` / `+0x18` (`0x00dd1e40`: `piVar3[5]`, `piVar3[6]`, stride 8) |
| `+0x90` | entity-description map | `0x00c6f8f0` |

`Entity` fields: `+0x0c` id, `+0x10` enter count, `+0x14` type id (16 bits), `+0x18` client flags (bit 0 `CEF_Remote`; bit 1 is set at the end of `enterAoI`), `+0x32` a byte read by the appearance request (see below).

A VS2005/2008 `std::map` object is `{ proxy, _Myhead, _Mysize }`; the head node's `_Parent` is the root; a node is `{ _Left, _Parent, _Right, key (+0x0c), value (+0x10...) }`, and leaves point back at the head. The telemetry DLL's read-only walker (`hooks/entity_trace/map.rs`) uses exactly this layout, through `ReadProcessMemory`.

## The handlers

All are `ServerMessageHandler` slots, run on a Mercury network thread, and are `thiscall`. The argument counts come from each function's `ret N`.

| Vtable slot | Function | Address | Args after `this` | Ghidra name today |
|---|---|---|---|---|
| 3 (`0x019aaed0`) | `onEntityEnter` = **`enterAoI`** | `0x00dd24f0` | `id, space, vehicle` (`ret 0xc`) | `FUN_00dd24f0` |
| 4 (`0x019aaed4`) | `onEntityLeave` = **`leaveAoI`** | `0x00dd2800` | `id, cache_stamp` (`ret 8`; the decompiler shows only `id`) | `BW_client_entity_manager_5` |
| 6 (`0x019aaedc`) | `onEntityCreate` | `0x00dd2270` | `id, type, space, vehicle, stream` (`ret 0x14`) | `BW_client_entity_manager_4` |
| 8 (`0x019aaee4`) | `onEntityProperty` | `0x00dd29d0` | `id, msg_id, stream` (`ret 0xc`) | `FUN_00dd29d0` |
| 9 (`0x019aaee8`) | `onEntityMethod` | `0x00dd2b80` | `id, msg_id, stream` (`ret 0xc`) | `FUN_00dd2b80` |
| - | `enterWorld` | `0x00dd1d00` | `entity, space, vehicle, flag` (`ret 0x10`; the decompiler shows three) | `EntityManager_enterWorld` |
| - | entity destroy | `0x00dd1120` | `entity, arg` (`ret 8`) | `FUN_00dd1120` |
| - | queued-message replay | `0x00dd1e40` | `entity` (`ret 4`), returns `bool` in `al` | `FUN_00dd1e40` |

### `enterAoI` (`0x00dd24f0`)

Looks the id up in the cache map (`+0x24`) first.

- **Not in the cache.** If it is in the world map (`+0x18`): its enter count is bumped. If it is in neither map: the pending record at `+0x30` is bumped (created on first use), so the count is waiting for the create.
- **In the cache.** The entity's enter count is bumped and, if it is now positive, its record in `+0x30` (if any) is refreshed with `space` and `vehicle` (`node+0x1c`, `node+0x20`). **Only when `entity == this+0xc` (the local player) or `entity+0xc > 0x3fffffff` (a client-only id) does it then leave the cache map, drop the pending record, call `enterWorld` and replay the queue.** For any other entity the count goes up and nothing else happens.
- Then, when the connection exists, the id is below `0x40000000`, and it is not the local player, `requestEntityUpdate` is sent to the server for the id ([request-entity-update-cache-stamp.md](request-entity-update-cache-stamp.md) §1).
- Finally, `entity+0x18 |= 2` for a known entity.

### `onEntityCreate` (`0x00dd2270`)

- Entity in the cache map: taken out of it and reused.
- Entity in the world map: the assertion `NewEntity->getEnterCount() > 0` (`entity_manager.cpp` line `0x1e0`).
- Neither: the pending record at `+0x30` is read and erased, and its count becomes the new entity's enter count (`BW_client_entity_manager_7`, `0x00dd09e0`, which allocates through `0x00e9b8d0` and calls `GameEntityBase::Init` `0x00e685e0`).

Then the queue for the id is replayed (`0x00dd1e40`), and:

```text
if entity.enter_count < 1:
    assert !isInWorld(entity)        // line 0x211
    cache_map[id] = entity           // PARKED: no enterWorld, no appearance
else if !isInWorld(entity):
    enterWorld(entity, space, vehicle, 0)
```

So the outcome of a create is decided by the enter count the `enterAoI` left in the pending record. A create whose `enterAoI` never arrived, or arrived and was undone by a `leaveAoI`, parks.

### `leaveAoI` (`0x00dd2800`)

Decrements the enter count wherever the entity is held. When a world-map entity's count reaches 0 it is destroyed through `0x00dd1120` (unless flag bit 0 is set, in which case bit 1 is cleared and the entity is kept). A cached entity whose count reaches 0 is erased from the cache map after the `!pGeriatric->isClientFlagSet(SGW::CEF_Remote)` assertion (line `0x309`), and `0x00dd1fb0` purges its queue and pending record. A pending record that reaches 0 is purged the same way.

### `enterWorld` (`0x00dd1d00`)

Inserts the entity into the world map, then calls the appearance request `0x00e69150` with the label `"EntityManager::enterWorld"`.

### The appearance request (`0x00e69150`)

`thiscall(entity, const std::string* reason)`, `ret 4`. Eight other callers pass their own labels: `GameBeing_setAppearance`, `GameEntity_setTint`, `setBodySetName`, `setStaticMeshName`, `setFlags`, `addClientComponent`, `removeClientComponent` and `FUN_00e695c0` (call sites `0x00e00c6f`, `0x00e6dffe`, `0x00e6e0e5`, `0x00e6dedf`, `0x00e6e661`, `0x00e6e1fb`, `0x00e6e2d6`, `0x00e69637`). It has three exits, each logged (only when the debug flag `*PTR_DAT_01e218f8` is set) as `Appearance request - entity: ... - <suffix>`:

1. `entity->vtable[1]()` is false: **`ENTITY NOT READY`**. Nothing is scheduled.
2. Ready, and the byte at `entity+0x32` is 0: **`SCHEDULING JOB`**: `FUN_00e998e0(GEM+0x98, entity)`. That function (`thiscall(scheduler, entity)`, `ret 4`) asks the entity for its appearance data object (`vtable[2]`) and returns if there is none; otherwise it inserts a job keyed by the entity id.
3. Ready, and the byte is non-zero: **`HOLD FOR TRANSACTION`**. Nothing is scheduled.

The telemetry hook cannot call `vtable[1]`, so it reports the observed facts: whether the scheduler ran inside the call (`scheduled`), and the hold byte. `not_ready` means "not scheduled and the hold byte is 0"; `held_or_not_ready` means "not scheduled and the hold byte is set", which is exit 3 or exit 1.

### The deferred queue

`onEntityMethod` (`0x00dd2b80`):

```text
node = world_map.find(id)
entity = node ? node.value : (this.player && this.player.id == id ? this.player : null)
if entity: Client_NetIn_EntityMethodDispatch(entity, msg_id, stream)   // 0x00c6f8f0
else:      queue[id].push({ msg_id, copy of the remaining stream })    // +0x3c
```

`onEntityProperty` (`0x00dd29d0`) has no player exception, and for an entity in the world map it does not dispatch at all: it asks the stream for its remaining length and returns, leaving the message unread. For any other entity it queues the message with the flag byte `msg_id | 0x40`. The replay treats a `0x40` entry the same way (asks the stream for its length and drops it). **The BigWorld property message is therefore unused by this client**; SGW sends properties as ClientMethods (`onEntityProperty(propId, value)` in the entity definitions).

`0x00dd1e40` drains one entity's queue through the dispatcher. Its three callers are `onEntityCreate` (`0x00dd2429`), `enterAoI` (`0x00dd26e1`, inside the enter-world branch only) and `EntityManager__vfunc_0` (`0x00dd21d6`). `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) itself has exactly two callers, `onEntityMethod` (`0x00dd2c7f`) and the replay (`0x00dd1f3f`), which is why the telemetry DLL tags dispatches by hooking those two entry points and leaves the dispatcher alone (the client-patches DLL hooks it).

**Consequence.** A method sent to an entity that is parked in the cache map is queued and, because the entity never enters the world, never replayed. `BeingAppearance`, `onStaticMeshNameUpdate` and the other appearance methods for such an entity are exactly the ones that would have made it visible.

## The outgoing router (`0x00c6fc40`)

`stdcall(Entity* entity, EntityDescription* desc, MethodDescription* method, args)`, `ret 0x10` (the decompiler shows three; the fourth is the argument object it walks at the end). `entity == null` means the local player. `method+0x00` is the method name (an MSVC `std::string` object), `method+0x1c & 3` the route (`2` = base method through `startProxyMessage`; otherwise `startEntityMessage`, or `startAvatarMessage` for the local player), `method+0x44` the wire message id and `method+0x48` the extended sub-index (negative when unused). The function returns without sending when the connection is offline (`[conn+0x30c] == 0`).

## The CME event registry (`0x01f11fc4`)

`std::map<std::string, factory>`; a node is `{ _Left, _Parent, _Right, std::string key at +0x0c (size +0x20, capacity +0x24, buffer +0x10), factory pointer at +0x28 }`. Confirmed from `0x0158ea90` (the key compare reads `[node+0x24]` as capacity and `[node+0x20]` as size) and `0x00a5c0f0` (`call [node+0x28]`). The `this` of every `0x00a5c0f0` call is the map object. The DLL walks it in order once and emits the names as `client.cme.catalog`.

## The Lua error string

`lua51.dll` exports, verified against its export table on 2026-09-28: `?lua_type@@YAHPAUlua_State@@H@Z`, `?lua_tolstring@@YAPB_WPAUlua_State@@HPAI@Z` (returns `wchar_t*`, length in characters), `?lua_gettop@@YAHPAUlua_State@@@Z`, `?lua_isstring@@YAHPAUlua_State@@H@Z`, `?lua_typename@@YAPB_WPAUlua_State@@H@Z`, `?luaL_error@@YAHPAUlua_State@@PB_WZZ`. After a non-zero `lua_pcall`, the error value is at stack index -1.

## Corrections to earlier docs

- [request-entity-update-cache-stamp.md](request-entity-update-cache-stamp.md) §2.2 names slot 4 (`0x00dd2800`) `EntityManager_EnterAoI` and §3 describes `0x00dd29d0` as `EntityManager_LeaveAoI`. By the decompiles above, `0x00dd2800` is **`leaveAoI`** (it decrements the enter count) and `0x00dd29d0` is **`onEntityProperty`** (the deferred-leave slot at `+0x3c` that §3 describes is the property queue, keyed by entity id like the method queue). The path A/B behaviour §3 recovers is right; the function name is not.
- The decompiler's argument lists for `enterAoI`'s sibling `leaveAoI` (one), `enterWorld` (three) and `0x00dd1120` (one) are short by one stack argument each; the `ret N` of each is the authority.

## What live confirmation would settle

1. `client.entity.create` with `outcome = parked` and `entered_world = false` for the entities that never render, and `client.entity.enter` with `place_before = cache`, `entered_world = false` for the same ids: the hypothesis above.
2. `queued_msgs > 0` on those entities' events, and no `client.entity.queue_replay` for them.
3. Whether `enter_count` is 0 at the create for the failing guards (the pending record's first word being the count is inferred).
4. Whether the healthy NPCs' creates show `outcome = entered_world`, `via = create`.

## Not covered here

- The UE3 actor spawn for an entity (the job the appearance scheduler queues). The scheduler `0x00e998e0` and the job runner behind it are the next place to look for "appearance scheduled but the pawn never appeared".
- The layout of the pending record beyond its first word.
- `EntityManager__vfunc_0` (`0x00dd20b0`), the third caller of the queue replay.
