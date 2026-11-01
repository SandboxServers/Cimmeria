# Lab-autopilot client anchors

> **Type:** Reference
> **Audience:** Engineers building the lab-autopilot DLL and daemon features (`crates/client-telemetry/src/bridge/`, `crates/lab/`), and anyone checking a client address the lab reads
> **Last updated:** 2026-11-01
> **Companions:** [UE3 back-port facts](../../analysis/lab-autopilot/ue3-backport-verified.md) (raw values of static pass 2 and the live pass), [lab-autopilot contract](../../analysis/lab-autopilot/contract.md) (C4, C6, C12), [cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) §10, [controller-mouse-input-model.md](controller-mouse-input-model.md), [system-protocol-wire-formats.md](system-protocol-wire-formats.md) (the game clock), [address-map.md](../address-map.md)

This page is the one record of the client addresses the lab-autopilot campaign uses. It copies the answers already settled on 2026-10-31 (static pass 2, "S" rows, and live pass, "L" rows, in [ue3-backport-verified.md](../../analysis/lab-autopilot/ue3-backport-verified.md)) and adds what packet AX-01 read statically on 2026-11-01: the level `Actors` array that gates the `actors` sample field and the `actor_growth` detector, and the object that owns the BigWorld game clock.

Sources and confidence words:

- **live**: read or called on the lab client on 2026-10-31 (L1-L8), or earlier dated live use cited inline.
- **static**: read from the QA `SGW.exe` (UE3 build about 3004, i686, image base `0x00400000`, no ASLR) with headless Ghidra (`tools/re/ghidra-headless/Probe.java`, `-readOnly -noanalysis`). AX-01's own reads are marked "AX-01".
- **likely**: inferred from names, later UE3 source or a partial read; not proven in our binary.

AX-01 ran no client, no x64dbg and no lab.

## Traces

- **`UWorld::SingleLineCheck` `0x00875970`**: `__thiscall(UWorld* GWorld, FCheckResult& Hit, AActor* Source, const FVector* End, const FVector* Start, DWORD TraceFlags, const FVector* Extent, int)`, ends `ret 0x1c` at `0x00875a23`. It returns TRUE when nothing was hit (S1.4, L1).
- **`MultiLineCheck` `0x008721b0`**: allocates the result list on `GMem` `0x01ea5750` (S1.4). Not called by the lab.
- **`FCheckResult`, 0x48 bytes**: Actor `+0x04`, Location `+0x08`, Normal `+0x14`, Time `+0x20` (default 1.0), Item `+0x24` (default -1), Material `+0x28`, Component `+0x30`, LevelIndex `+0x40` (default -1) (S1.5).
- **Flags (live)**: `0x2086` hits BSP, floors and static meshes; BigWorld pawns, NPCs and the own body, hit **only with bit `0x1`** (`0x20bf`); `0x2086` and `0x20be` miss them. Passing the own pawn as `Source` excludes it. `0x3fff` raises a guarded AV at `0x00914fd3` and the client survives (L2).
- **Hit kinds**: a BSP hit reports `Actor` = WorldInfo (vtable `0x018a02b4`) and `Item` = the BSP node, so test `Item >= 0` for "hit the level"; a static mesh reports its `StaticMeshActor` (vtable `0x0184f08c`); a pawn hit reports the `SGWGamePawn` (vtable `0x019dffd4`, which is `Entity+0x08`) (L2).
- `AActor::Trace` helper `0x006e6d20`; script natives `AActor_execTrace` `0x006e7be0`, `execFastTrace` `0x006e1490` (S1.1-S1.3). `EncroachingWorldGeometry` is not in the exe as ASCII or UTF-16 (S1.8).
- **Calling it from the lab**: `client_call_native` takes at most 6 arguments, so a 7-argument call goes through a cdecl shim on a VirtualAlloc'd RWX page (`8b442404 ff701c ff7018 ff7014 ff7010 ff700c ff7008 ff7004 8b08 ff5020 c3`: ECX = block[0], push block[7..1], call block[8]) (live pass).

## World and loading

- **`GWorld` = `0x01EE2684`**, a `UWorld*`: object "TheWorld", class `World`, vtable `0x01838E7C` (L7). Statically, the `!GWorld->NetDriver` assert at `0x008f7329` reads `[0x01EE2684]+0xD0` (S2.5). Ghidra names the global `g_pGLevel`; the address map's older "global `WorldInfo*`" label was wrong and AX-01 corrected it (see [address-map.md](../address-map.md) and [world-entry-pipeline.md](world-entry-pipeline.md)).
- **WorldInfo is not a global.** It is `Actors[0]` of the persistent level, `[[[GWorld+0x50]+0x3C]]` (S2.5, and the `Event_World_Loaded` emitter `0x005541a0` reads exactly `**(*(world+0x50)+0x3c)`, AX-01). It is also the actor a BSP trace hit reports (vtable `0x018a02b4`, L2).
- **A zone change replaces `GWorld`**: the pointer changed on each of two `/gmgotolocation` zone changes (L3). Detect a load by a `GWorld` pointer change, not by a flag poll.
- **`0x01EE2B6C` is set to 1 while a map load is in progress** (written 1 by the `LoadMap` path `0x008f6930` at `0x008f735b` and by `0x00553110`; cleared in `0x005d6370`; the `Event_World_Loaded` emitter `0x005541a0` returns 0 while it is non-zero) (S4.1). **It was never seen set from the bridge drain**: 2894 samples across two zone changes all read 0, because the bridge reads on the main thread at the `FEngineLoop::Tick` drain and cannot sample inside a blocking load (L3). The older "set after `Event_World_Loaded` fires" label was wrong; AX-01 corrected it in the address map and the world-entry finding.

## View point and camera

- **`0x005d9690` is the `eventGetPlayerViewPoint` thunk**: `FindFunctionChecked` with FName `0x110c` ("GetPlayerViewPoint", global `0x01ee14a4`), then `ProcessEvent` through vtable `+0xe8`, `ret 8`. Called on the PlayerController it returns the orbit view point one camera distance behind the pawn, and the point moves with the camera yaw (L4). `ULocalPlayer::CalcSceneView` `0x005d3970` calls it for the renderer (S3.2).
- **The camera actor's `Location` (`+0xDC`) reads zero**; do not use it (L4).
- **Camera object at `PC+0x2B0`** (`ASGWCamera_Player`, vtable `0x019E1134`): flags `+0x348` (`0x4` inverts yaw, `0x8` inverts pitch), gain `+0x358` = 20.0 live, distance `+0x360` (250 live), pitch `+0x368`, yaw `+0x36C` (S3.3, L5; [cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) §10).
- **AddYaw `0x00e7e600`**, AddPitch `0x00e7e590`: rotator units, so counts per radian = 10430.4 / gain = **521.5** at gain 20; `AddYaw(100.0)` moved yaw 0 to 2000 (S5.2, L5). Whether DirectInput motion alone turns the camera is still untested (AX-02 step 6).
- The PlayerController is found by scanning the persistent level's `Actors` array for vtable `0x019E2B2C` (`supervisor/world/memory.rs` `find_player_camera`, live since 2026-10-10); the old `0x0054d8d0` chain reads a WorldInfo field that is 0 in a normal game ([cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) §10.1).

## Death

- **State field: `GameBeing +0x158`**, bit 0 = `BSF_Dead`. `FUN_00e01c90` XORs the new value against `+0x158` and dispatches bit `0x1` to `0x00e6e330` (S4.2; [state-flag-broadcast.md](state-flag-broadcast.md)).
- **Live on an NPC only**: a friendly NPC's `+0x158` went `0x0` to `0x41` after `/gmkilltarget` (L6). The player's bit has not been seen set: the player cannot be killed in SGC (no hostiles, and the GM kill refuses players). AX-02 step 5 is the optional live check.

## Reflection

- **`UObject::ProcessEvent` `0x004e06a0`**, `__thiscall(this, UFunction*, void* Parms)`, vtable slot 58 (S2.1). `0x004513d6` is a wxWidgets thunk, not ProcessEvent.
- **`FindFunctionChecked` `0x0049ffb0`**: `__thiscall(obj, FName.Index, FName.Number, int)`, `ret 0xc`, forwards to `0x0049ff40`. Live on the PC with FName `0x110c` it returned a `UFunction` with FunctionFlags `0x420982` and ParmsSize 24 (live pass). It is the checked variant: never pass a name you have not resolved.
- **`UFunction`**: Name `+0x2c`, Children `+0x4c`, FunctionFlags `+0x8c`, iNative `+0x90`, ParmsSize `+0x9e`, ReturnValueOffset `+0xa0`, `Func` `+0xa8` (S2.2).
- **`UField`/`UProperty`**: Next `+0x40`, ArrayDim `+0x44`, ElementSize `+0x48`, PropertyFlags `+0x4c`, Offset `+0x64` (S2.3).
- **`StaticFindObject` `0x004a2470`** (`__cdecl(UClass*, UObject* Outer, const wchar_t*, BOOL)`, Outer `-1` = any), `StaticFindObjectFast` `0x0049f420`, FName constructor `0x0049e960(&FName, wchar_t*, 1)` (S2.4). `GNames` `0x01ecade0`, entry text UTF-16 at `+0x10` (L7). UObject Outer `+0x28`, Name `+0x2c/+0x30`, Class `+0x34` ([client-engine-sinks-and-seams.md](client-engine-sinks-and-seams.md)).

## CEGUI

- **`WindowManager::ms_Singleton` `0x01F010C0`**: the window registry is an `std::map<String, Window*>` at `+0`, head `+4`, size `+8`; node left `+0`, parent `+4`, right `+8`, key `CEGUI::String` at `+0xc` (UTF-16 buffer `+0x10`, length `+0x20`, capacity `+0x24`), value `Window*` `+0x28`, is-nil byte `+0x2d` (S6, L8).
- **Size `+8` = 21416 after login**, equal to a Lua recursive count under `Root` (L8). A native walk must page or filter.
- `WindowManager::getWindow` `0x011ac660` throws `UnknownObjectException` on a missing name (S6). The `CEGUI` Lua global is not exposed; `getWindow("Root")` works (L8).

## Actors

AX-01 read the engine's own actor iterator, which is the code evidence the `actors` field needed.

- **`FActorIterator` constructor `0x00555070`, `operator++` `0x00554730`** (AX-01; `IsInGameThread()` assert at `Engine.h` line `0x232`; used by `AActor_execAllActors` `0x006e4c10`, whose loop asserts `CurrentActor` at `Engine.h` line `0x256`). Cleaned up:

  ```c
  // it: [0] actor index, [1] level index, [2] done, [3] visited, [4] current actor
  level = GWorld->Levels[it[1]];                     // data GWorld+0x44, count GWorld+0x48
  while (current == NULL) {
      if ((level+0x164 == 0 || DAT_01eb0838) && ++it[0] < level[+0x40]) {
          current = ((AActor**)level[+0x3c])[it[0]]; // Actors data +0x3c, count +0x40
      } else if (++it[1] >= GWorld[+0x48]) {
          done = 1; break;
      } else {
          level = GWorld->Levels[it[1]];
          it[0] = (level != GWorld[+0x50]);          // 0 for the persistent level, 1 for others
          ...                                        // same read as above
      }
  }
  ```

- So **`GWorld+0x50` is the persistent level** (the iterator compares each level against it), **`GWorld+0x44/+0x48` is the `Levels` array** (data, count), and **level `+0x3C/+0x40` is the `Actors` `TArray`** (data, count). Null slots are skipped, so the count includes empty slots.
- The dynamic-actor iterator (`0x005e6710`, `operator++` `0x005e61e0`, used by `AActor_execDynamicActors` `0x006e4e30`) starts at level `+0xd8` minus 1, so `+0xd8` is the first dynamic actor index (`iFirstDynamicActor` is the later-UE3 name, likely).
- **Live use**: the lab finds the local PlayerController by scanning `[[GWorld+0x50]+0x3C]` up to `+0x40` entries for its vtable (`crates/lab/src/supervisor/world/memory.rs` `find_player_camera`), and that scan has driven `client_camera` live since 2026-10-10; the count read 0x32 live, with `Actors[0]` the `AWorldInfo` ([cegui-mouse-input-feed.md](cegui-mouse-input-feed.md) §10.1).

**`actor_count` verdict: confirmed.** `[[GWorld+0x50]+0x40]` is the actor count of the persistent level, on code evidence (the iterator above) plus live use. Two limits for C6 `actors` and C12 `actor_growth`:

1. It counts the **persistent level only**. Actors in streamed sub-levels sit in `GWorld->Levels[1..]` (`+0x44/+0x48`) and are not in it. If a zone streams its content, sum the `+0x40` of every level in `Levels`.
2. It counts **slots, including null ones**. Whether a destroyed actor leaves a null slot until a later compaction (the later-UE3 behaviour) is not read in this build; for a growth detector that only makes the count lag a destroy, never miss a spawn.

## Clock

- **Owner: the `ServerConnection`.** The `tickSync` handler `0x00dd6d00` (Ghidra has no function there; AX-01 created one read-only) is `__thiscall(ServerConnection*, {u32 gameTime, u32 tickRate}*)`: it copies `+0x33c` to `+0x334`, stores `gameTime` at `+0x32c`, stores the time pointer `+0x174` at `+0x34c`, reads `*(+0x174)` into `+0x33c` and stores `tickRate * 0.001` at `+0x344` (AX-01; agrees with [system-protocol-wire-formats.md](system-protocol-wire-formats.md) and `docs/analysis/crafting/worknotes/cr-02.md`).
- **A global the DLL can read**: the singleton pointer **`0x01EF2264`**, created on first use by `0x00c6f870` (`scalable_malloc(0x3c)`, then constructor `0x00c6f690`) and also read and written by `0x00c6f840` (AX-01 xrefs). The constructor stores the `ServerConnection` at singleton `+0x28` ([system-protocol-wire-formats.md](system-protocol-wire-formats.md)). So the clock fields are at `[[0x01EF2264]+0x28] + 0x32c`.
- **The clock value** is `FUN_00dd6c60(ServerConnection)` = `((u32)+0x32c + (*(+0x34c) - +0x334) / +0x344) / hertz`, with `hertz` the float global `0x01e51cb8` set by `updateFrequencyNotification` (AX-01 decompile). C4's `game_t` already uses `hooks/ability_trace/clock.rs`; this record only names the owner.
- **`WorldInfo.TimeSeconds`: not found.** There is no `TimeSeconds` string in the exe (the name lives in the script packages), and `AActor_execSetTimer` `0x00527dc0` does not read it. The runtime route is the S2.6 property walk on WorldInfo's class with the FName for `TimeSeconds`.

## Anchor table

| anchor | address/offset | confidence (live, static, likely) | source row | still open |
|---|---|---|---|---|
| SingleLineCheck | `0x00875970`, thiscall, 7 stack args, `ret 0x1c`, TRUE = no hit | live | S1.4, L1 | extent traces untested |
| MultiLineCheck | `0x008721b0` (GMem `0x01ea5750`) | static | S1.4 | not used by the lab |
| `FCheckResult` | 0x48 bytes; Actor `+4`, Location `+8`, Normal `+0x14`, Time `+0x20`, Item `+0x24`, Component `+0x30`, LevelIndex `+0x40` | live | S1.5, L2 | none |
| Trace flags | `0x2086` level; `0x20bf` (bit `0x1`) also pawns; `0x3fff` guarded AV `0x00914fd3` | live | L2 | terrain and outdoor floors (AX-64 U4) |
| BSP hit actor | WorldInfo, vtable `0x018a02b4`; Item = BSP node | live | L2 | none |
| `AActor::Trace` helper | `0x006e6d20` | static | S1.2 | none |
| 7-arg call shim | cdecl shim on an RWX page (bytes above) | live | live pass | none |
| `GWorld` | `0x01EE2684`, class `World`, vtable `0x01838E7C`; `+0xD0` NetDriver | live | L7, S2.5 | none |
| Persistent level | `[GWorld+0x50]` | static (AX-01 iterator) and live use | AX-01, cegui-mouse-input-feed §10.1 | none |
| `Levels` array | `GWorld+0x44` data, `+0x48` count | static | AX-01 (`0x00554730`) | not read live |
| WorldInfo | `[[[GWorld+0x50]+0x3C]]` (`Actors[0]`) | live | S2.5, cegui-mouse-input-feed §10.1 | none |
| Loading signal | `GWorld` pointer change | live | L3 | none |
| Load flag | `0x01EE2B6C` = 1 during `LoadMap`; never seen set from the bridge | static (meaning), live (unobservable) | S4.1, L3 | none; do not poll |
| `eventGetPlayerViewPoint` | `0x005d9690`, thiscall `(PC, &Loc, &Rot)`, `ret 8` | live | L4, S3.2 | none |
| Camera actor `Location` | `+0xDC` reads zero | live | L4 | why it is zero (not needed) |
| Camera object | `PC+0x2B0`, vtable `0x019E1134`; flags `+0x348`, gain `+0x358`, distance `+0x360`, pitch `+0x368`, yaw `+0x36C` | live | L5, S3.3 | none |
| Mouse gain | 20.0, so 521.5 counts per radian | live | L5, S5.2 | DI-only mouse-look (AX-02) |
| AddYaw / AddPitch | `0x00e7e600` / `0x00e7e590` | live (AddYaw), static (AddPitch) | L5, S3.3 | none |
| PlayerController | found by vtable `0x019E2B2C` in the level `Actors` array | live | `memory.rs`, cegui-mouse-input-feed §10.2 | none |
| State field | `GameBeing +0x158`, bit 0 Dead | live on an NPC, static for the player | S4.2, L6 | player death (AX-02 step 5) |
| ProcessEvent | `0x004e06a0`, slot 58 | static | S2.1 | a by-name call not yet made |
| FindFunctionChecked | `0x0049ffb0`, thiscall `(obj, Index, Number, int)`, `ret 0xc` | live | live pass | none |
| `UFunction` fields | Children `+0x4c`, FunctionFlags `+0x8c`, ParmsSize `+0x9e`, `Func` `+0xa8` | live (flags, ParmsSize), static (rest) | S2.2, live pass | none |
| `UProperty` fields | Next `+0x40`, ElementSize `+0x48`, PropertyFlags `+0x4c`, Offset `+0x64` | static | S2.3 | super-struct offset |
| StaticFindObject | `0x004a2470`, cdecl | static | S2.4 | none |
| FName constructor | `0x0049e960` | static | S2.4 | none |
| GNames | `0x01ecade0`, text UTF-16 at `+0x10` | live | L7 | none |
| CEGUI WindowManager | `0x01F010C0`; map size `+8` (21416 after login) | live | S6, L8 | `getType` and auto-child listing not built |
| `getWindow` | `0x011ac660` (throws when missing) | static | S6 | none |
| `actor_count` | `[[GWorld+0x50]+0x40]` count, `+0x3C` data | **confirmed**: static (AX-01 `FActorIterator` `0x00554730`) and live use | AX-01, `memory.rs`, cegui-mouse-input-feed §10.1 | persistent level only; null slots counted |
| First dynamic actor | level `+0xd8` | static (offset), likely (name) | AX-01 (`0x005e6710`) | not read live |
| Clock owner | `ServerConnection`: `[[0x01EF2264]+0x28]`; `gameTime` `+0x32c`, prev arrival `+0x334`, now `+0x33c`, period `+0x344`, time ptr `+0x34c` | static | AX-01 (`0x00c6f870`, `0x00dd6d00`), system-protocol-wire-formats | `[0x01EF2264]` not read live |
| Clock value | `FUN_00dd6c60`; hertz float `0x01e51cb8` | static | AX-01, system-protocol-wire-formats | none |
| `WorldInfo.TimeSeconds` | not found | none | AX-01 | runtime property walk (S2.6) |
