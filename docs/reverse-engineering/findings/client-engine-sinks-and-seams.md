# Client Engine Sinks and Seams

> **Diátaxis type**: reference
> **Audience**: engineers extending `cimmeria-client-telemetry`, and anyone reading its `client.bw.*`, `client.ue3.*`, `client.log4cxx.*`, `client.engine.*`, `client.media.*`, `client.audio.*`, `client.physx.*`, `client.io.*` and `client.gfx.*` events in SigNoz
> **Last updated**: 2026-09-28
> **Confidence**: every address, layout and signature below was read from the QA `SGW.exe` in Ghidra (decompile or disassembly) or from the PE import and export tables of the client's own DLLs, and each is pinned by a unit test or the fingerprint gate. **None of it has been seen from a live client yet.** Where a value comes from outside this build (SDK headers, the Bink 1.x `BINK` struct) it says so.
>
> **See also**: [`client-instrumentation-hookpoints.md`](client-instrumentation-hookpoints.md) (the game-layer anchors and the Tier-1 hooks), [`client-instrumentation-entry-points.md`](client-instrumentation-entry-points.md) (Phase 3-6 anchors), [`../../architecture/client-telemetry.md`](../../architecture/client-telemetry.md) (the design and the event catalog).

The engine layer of the client (BigWorld, UE3, the C runtime, the OS and the middleware DLLs) already has logging compiled in, and a set of places where a failure is swallowed. This is the recovered map of both, and the hooks built on it (`crates/client-telemetry/src/hooks/sinks/` and `seams/`).

## The five logging paths

| # | Path | What is in the real client | Hook |
|---|---|---|---|
| 1 | BigWorld `DEBUG_MSG` .. `CRITICAL_MSG`, `MF_ASSERT` | Runs. Filtered by a threshold inside the message helper; default output goes to `OutputDebugStringA` | `client.bw.message` |
| 2 | UE3 `GLog` (`debugf`, `warnf`, script `Log()`) | **Compiled out.** `Launch.log` of a real session is `Log: Log file open` / `Log: Release logging initialized.` and nothing else | `client.ue3.log` (low volume) |
| 3 | UE3 `GError` (`appErrorf`, `appFailAssertFunc`) | Runs; ends the process | `client.ue3.fatal_error` |
| 4 | log4cxx (`SGWLogConfig.xml`: root `all`, console and `SGWDebugLog.log`) | Runs. The real log is 65 000 lines of `DEBUG common - inside writeLock` lock traces and a few real `ERROR common - Error opening static cache archive ...` | `client.log4cxx.event` |
| 5 | `OutputDebugStringA/W` | Runs (paths 1 and 2's default outputs, plus anything else) | `client.os.debug_string` |

`log4cxx::UnrealAppender` (`SGW.exe`'s own `AppenderSkeleton` subclass; `getName` at `0x00c64680` returns `"UnrealAppender"`) maps a log4cxx level to UE3's `debugf`/`warnf`; both are compiled out, so it forwards nothing. It is not a duplicate of the log4cxx sink.

### BigWorld message helper

`DebugMsgHelper::message` at `0x00a36460`: `__thiscall(this, const int* header, const char* fmt, va_list)`, `ret 0xc`. Two xrefs, both wrappers:

- `0x00a35210` (cdecl varargs, 30 callers, all in `ServerConnection`, `EntityManager` and `Mercury::Nub`) builds the `va_list` and calls it;
- `0x00a351d0` (the assertion wrapper, 14 callers) -> `0x00a36ac0` -> `0x00a36900` (formats with `vsprintf`, shows "Do you want to enter debugger?" for a critical one) -> `0x00a36650` -> `0x00a36460` with the format `"%s"`.

`header` is `{ component_priority, message_priority }`. The function locks the implementation object (`*this`; its first field is the critical section) and passes a message only when `header[0] + impl[0x3c] <= header[1]`; otherwise it returns without calling any output. Past the filter it runs the message callbacks (`impl + 0x30`) and, if none handled it, the default output `0x00a353b0` -> `0x00a352f0`, which builds `"<PRIORITY>: "` from the table at `0x01922380` and calls `_vsnprintf`, `OutputDebugStringA`, and (when `0x01ef0713` is set) `fprintf(stderr, ...)`.

Priority names, table order: `TRACE`, `DEBUG`, `INFO`, `NOTICE`, `WARNING`, `ERROR`, `CRITICAL`, `HACK`, then a null slot.

### UE3 log devices

`GLog` is the `FOutputDeviceRedirector` (RTTI `.?AVFOutputDeviceRedirector@@`, type descriptor `0x01dafeac`, vtable `0x01815188`): slot 1 `Serialize(const TCHAR*, EName)` at `0x004ce0b0`, `ret 8`. From the game thread it forwards to every registered device; from another thread it queues a copy. Devices: `FOutputDeviceFile` (`0x004cd960`), `FOutputDeviceDebug` (`0x004cc9f0`), the console devices.

`GError` is `FOutputDeviceWindowsError` (vtable `0x018150f8`): slot 1 `Serialize(const TCHAR*, EName)` at `0x004ce3a0`, `ret 8`. It calls `IsDebuggerPresent` and, if one is attached, writes to address 3 to break; on the first error sets the critical-error flag `0x01ead7c4`, copies the message into the buffer at `0x01ea57a0`, throws if `0x01ead7cc` is set, calls slot 4 (`HandleError`) and `UGameEngine::unknown_004910b0(1)`, which ends the process.

The UE3 `check()` reporter at `0x00486000` is `__cdecl(const char* expr, const char* file, int line)`. It copies the strings into `std::string`s, hands them to the assertion reporter (singleton `0x00a5ab70`, `0x00a5ad80`) and **returns**: a failed `check` in this build is reported and survived. It is called from `UnLevAct.cpp`, `UnWorld.cpp` and `UnObj.cpp` (for example `"GWorld == this"` at `UnLevAct.cpp:0x86`, `"ThisActor->IsValid()"` at `:0x1ad`, `"StreamingLevel"` at `UnWorld.cpp:0x440`, `"ObjectClass"` at `UnObj.cpp:0x14dd`).

### `FName` and `UObject` layout

| Item | Address / offset | Evidence |
|---|---|---|
| `FName::Names` (`TArray<FNameEntry*>`) | data `0x01ecade0`, count `0x01ecade4` | `FName::StaticInit` (`0x0049ba90`) zeroes it (`MOVQ [0x01ecade0], XMM0`); `FName -> string` (`0x0049b190`) indexes it |
| `FNameEntry` | `+0x00` index, `+0x08` flags (`u32`), `+0x10` name (UTF-16) | `0x0049b190`; `FOutputDeviceDebug::Serialize` (`0x004cc9f0`) reads flags at `+8` |
| Suppressed log name | flags bit `0x1000` | `0x004cc9f0` skips such a name (and names `0x5a`, `0x314`) |
| `UObject::Outer` | `+0x28` | `GetOutermost` (`0x0049f090`) loops `eax = [eax+0x28]` |
| `UObject::Name` | `+0x2c` (index), `+0x30` (number) | `SpawnActor` copies `Class + 0x2c` into the actor's `Tag` |
| `UObject::Class` | `+0x34` | `SpawnActor` compares `Template + 0x34` with the class argument |
| `FName` text | name, then `_<number-1>` when the number is non-zero | `0x0049b190` |

> **Correction.** [`../address-map.md`](../address-map.md) listed `FName::GNames` at `0x01eadbc0` while quoting the RVA `0x01ACADE0`, whose VA is `0x01ecade0`. `0x01ecade0` is what `StaticInit` and every reader use (and what `editor-source-mapping.md` says). The address-map row is corrected in the same change.

## log4cxx

`SGW.exe` imports 60-odd names from `log4cxx.dll` (the DLL shipped in `binaries`). Two are the logger's `forcedLog`, the function every log macro ends in once its level check passes; four are the level checks:

| IAT slot | Import |
|---|---|
| `0x017f0160` | `Logger::forcedLog(const LevelPtr&, const std::string&, const LocationInfo&) const` |
| `0x017f0188` | `Logger::forcedLog(const LevelPtr&, const std::wstring&, const LocationInfo&) const` |
| `0x017f017c` / `0x017f01ac` / `0x017f01c4` / `0x017f01d8` | `Logger::isErrorEnabled` / `isWarnEnabled` / `isDebugEnabled` / `isInfoEnabled` |

Layouts, read from the bytes of the shipped `log4cxx.dll`:

- `Level::toInt()` is `mov eax, [ecx+0xc]`: the level int is at `Level + 0xc` (`ALL` = `INT_MIN`, `TRACE` 5000, `DEBUG` 10000, `INFO` 20000, `WARN` 30000, `ERROR` 40000, `FATAL` 50000, `OFF` = `INT_MAX`).
- `Logger::getName(std::wstring&)` copies from `Logger + 0xc`: the logger name is a wide `std::string` at `+0xc`.
- `LocationInfo(const char* file, const char* method, int line)` stores `line` at `+0`, `file` at `+4`, `method` at `+8`.
- `LevelPtr` is an `ObjectPtrT<Level>`. `UnrealAppender::append` reads `*(getFatal() + 4)`, which puts the `Level*` at `+4` behind a vtable pointer; the sink does not depend on it and accepts whichever of `+4`, `+0` points to a standard level.

## Subsystem seams

### Actors

`UWorld::SpawnActor` at `0x00876970`: `__thiscall`, `this` = the `UWorld`, eleven stack arguments, `ret 0x2c` at both `RET` sites.

| # | Argument |
|---|---|
| 1 | `UClass* Class` (`NULL` returns `NULL`) |
| 2, 3 | `FName Name` (index, number) |
| 4 | `FVector* Location` (copied to the actor at `+0xdc`) |
| 5 | `FRotator* Rotation` (`+0xe8`) |
| 6 | `AActor* Template` |
| 7 | `bNoCollisionFail` |
| 8 | `bRemoteOwned` (swaps `Role` and `RemoteRole`) |
| 9 | `AActor* Owner` (its outer picks the level) |
| 10 | `APawn* Instigator` |
| 11 | `bNoFail` |

It returns the actor, or `NULL` on a null, abstract or deprecated class, a template of another class, a spawn location failing the collision check (unless `bNoCollisionFail`), a level that is not current, or an actor that destroys itself in `PreBeginPlay`. Callers: `AActor::execSpawn` (`0x006e1640`), `USeqAct_Interp::Activated` (its replicated actor), engine code.

`UWorld::DestroyActor` at `0x00875290`: `__thiscall(this, AActor*, bNetForce, bShouldModifyLevel)`, `ret 0xc`, returns a `UBOOL`. `0` without destroying when the actor is pending kill (`flags & 5`) or, on a client, replicated from the server and `bNetForce` is not set; `1` when destroyed or already destroyed (`flags & 8`).

### Matinee

`USeqAct_Interp` (RTTI `.?AVUSeqAct_Interp@@`, vtable `0x018ad494`): slot 84 `UpdateOp` at `0x007b0940`, slot 85 `Activated` at `0x007b06a0`, slot 86 `DeActivated` at `0x007a6730` (slot 86's name is inferred from what it does and from `USequenceOp`'s layout; 84 and 85 are confirmed by their bodies).

- `this + 0x8c` is the `InputLinks` data pointer, `this + 0x90` the count; each link is `0x28` bytes with `bHasImpulse` as bit 0 of the byte at `+0x0c`. `UpdateOp` tests links 0-4 (`Play`, `Reverse`, `Stop`, `Pause`, `Change Dir`) and clears them.
- `this + 0x104` is the position (`float`); `this + 0x11c` the `InterpData`, whose `InterpLength` is the `float` at `+0x90`.

### Level streaming

`UWorld::UpdateLevelStreamingInner` (`0x0054e9c0`) is a resumable state machine, one step per call, time-sliced against a budget (`_DAT_01b4c0b0`). `StreamingLevel + 0x44` is `LoadedLevel`; `StreamingLevel + 0x60` bit 0 is `bIsVisible` (asserted clear on entry, set at the end from `LoadedLevel[0x59] == 0`). A step that flips the bit is the level becoming visible; a step over budget is a streaming hitch.

### `StaticLoadObject`

`0x004a8e10` returns `NULL` after the localised `Core.ObjectNotFound` report (`FUN_0049ee60`) when neither the loaded object nor its package resolves. The load flags' meaning in this build is not recovered; they are reported raw.

### Bink

`_BinkOpen@8` (IAT `0x017effa4`) and `_BinkClose@4` (`0x017effa8`), `__stdcall`. The client opens movies **from memory** (`FUN_00509820`: `_BinkOpen_8(buffer, 0x4004400)`), so the first argument is a buffer, not a name: the file name is not available at this seam. The header offsets (`Width` `+0`, `Height` `+4`, `Frames` `+8`, `FrameNum` `+0xc`, `FrameRate` `+0x14`, `FrameRateDiv` `+0x18`) are the Bink 1.x SDK `BINK` layout, **not** confirmed against this build; the event carries `plausible` so a wrong layout shows up as data, not as a crash.

### FMOD Event

`?start@Event@FMOD@@QAG?AW4FMOD_RESULT@@XZ` (IAT `0x017f00a4`), `?stop@Event@FMOD@@QAG?AW4FMOD_RESULT@@_N@Z` (`0x017f0080`), `?getInfo@Event@FMOD@@QAG?AW4FMOD_RESULT@@PAHPAPADPAUFMOD_EVENT_INFO@@@Z` (`0x017f0084`). `QAG` is a `__stdcall` member, and MSVC passes `this` of those on the stack: `FUN_009035f0` calls `start` as `PUSH event; CALL` and `stop` as `PUSH 1; PUSH event; CALL`, with no stack adjustment after either. `FMOD_RESULT` values are reported as integers; no name table is attached (the SDK header is not available to confirm one).

### PhysX

`NxCreatePhysicsSDK` (IAT `0x017efd34`, called from `FUN_005590c0`) is given UE3's `FNxOutputStream` (RTTI `.?AVFNxOutputStream@@`, vtable `0x01839d94`):

| Slot | Address | Method | Body |
|---|---|---|---|
| 0 | `0x0055c5e0` | `reportError(NxErrorCode, const char* message, const char* file, int line)`, `ret 0x10` | copies the message into a string, compares it with `"Mesh has a negative volume!"` and `"Creating static compound shape"`, and does nothing else |
| 1 | `0x0055c5d0` | `reportAssertViolation(const char*, const char*, int)`, `ret 0xc` | `mov eax, 2; ret 0xc` |
| 2 | `0x00af6810` | `print(const char*)` | `RET 4` |
| 3 | `0x0055c700` | scalar deleting destructor | |

So every PhysX warning and assertion is discarded. `NxErrorCode` 1-5 are `INVALID_PARAMETER`, `INVALID_OPERATION`, `OUT_OF_MEMORY`, `INTERNAL_ERROR`, `ASSERTION`; the debug codes' numeric values are from memory of the 2.8 headers and are not used.

### File open

`CreateFileA` (IAT `0x017ef2a4`) and `CreateFileW` (`0x017ef2a8`). The detour reads `GetLastError` straight after the original returns and restores it before returning; a regression test makes the reporting path clobber it and fails without the restore.

### D3D9

`Direct3DCreate9` (IAT `0x017effd8`) returns an `IDirect3D9*`; its vtable slot 16 is `CreateDevice`, and the created device's slots 3 (`TestCooperativeLevel`) and 16 (`Reset`) are the seams. These are the documented D3D9 COM layouts; the vtables live in `d3d9.dll`, so there is no fixed address to fingerprint. **Limitation:** the hook lands about 1.5 s after the DLL attaches. If `SGW.exe` has already called `Direct3DCreate9` by then, the device is missed; nothing reports that.

## What is confirmed, and what is not

| Confirmed by Ghidra decompile/disassembly or the PE tables | Not confirmed (flagged in the code and the event docs) |
|---|---|
| All inline addresses and signatures; the vtable slots; every IAT slot and its import name; the `FName` and `UObject` offsets; the log4cxx layouts (from the DLL's bytes); the BigWorld header and filter | The Bink header layout; `FMOD_RESULT` and `NxErrorCode` debug-code names; the meaning of `StaticLoadObject`'s load flags; that slot 86 of `USeqAct_Interp` is `DeActivated`; anything about live behaviour |

## Open questions

- Does the client's `Direct3DCreate9` call come before the hook lands? (A live `client.gfx.device_created` answers it.)
- What are the `StaticLoadObject` load-flag bits in this build, so an optional lookup can be told from a real failure?
- Is there a cheap anchor for texture and mesh streaming (mip streaming)? None recovered.
- The movie name at `FUN_00509820`: its `param_1` looks like a wide string used to build the name before the memory open (the decompile passes it to a wide-string constructor); hooking it (rather than `_BinkOpen`) would give the movie's name.

## Cross-references

- Design and event catalog: [`../../architecture/client-telemetry.md`](../../architecture/client-telemetry.md).
- Address rows: [`../address-map.md`](../address-map.md) § "Engine log sinks and subsystem seams".
- Tests: `crates/client-telemetry/src/hooks/sinks/*` and `seams/*` (each pins its addresses and drives its detour over real memory), plus the fingerprint sites in `src/fingerprint.rs`.
