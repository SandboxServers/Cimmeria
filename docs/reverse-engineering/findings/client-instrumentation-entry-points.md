# Client Instrumentation — Resolved Entry Points (Phase 3-6)

> **Diátaxis type**: reference
> **Audience**: engineers implementing Phases 3-6 of `cimmeria-client-telemetry`
> **Last updated**: 2026-06-04
> **Companion docs**: [`client-instrumentation-hookpoints.md`](client-instrumentation-hookpoints.md) (the original anchor catalog), [`docs/architecture/client-telemetry.md`](../../architecture/client-telemetry.md) (the design)

Every Phase 3-6 inline/IAT/vtable hook from the anchor catalog, resolved to its function entry or IAT slot via Ghidra on 2026-06-04. Use this as the single source of truth when wiring future hooks — no per-anchor RE pass needed, just pick the address and detour signature.

All addresses are SGW.exe runtime addresses (image base `0x00400000`, ASLR disabled via `AtreaFixASLR.bat`).

---

## Phase 3 — Tier 3 game state

### State-flag broadcast (one inline hook covers all 9 flags)

| Target | Address | Signature | Hook role |
|---|---|---|---|
| `GameBeing::onStateFieldUpdate` (CME dispatcher) | **`0x00e01c90`** | `__thiscall void(GameBeing* this, void* event_data)` | **Primary hook target.** XOR-delta dispatcher that fires the per-flag handlers below for any bit-change in `bStateField`. Hooking here once captures every state-flag transition. |
| `FUN_00e7b4c0` (combat-ready / weapon anim) | `0x00e7b4c0` | `__thiscall fn(this, int)` | Sub-handler for `BSF_InCombat` (bit 3) and `BSF_Holster` (bit 8). Called from the dispatcher; no separate hook needed. |
| `GameEntity::unknown_00e6e330` | `0x00e6e330` | `__thiscall fn(this, void**)` | Sub-handler. Called from dispatcher. |
| `FUN_00dfff70` | `0x00dfff70` | `__fastcall fn(void*)` | Sub-handler. |
| `FUN_00e31aa0` | `0x00e31aa0` | `__stdcall fn(int)` | Sub-handler. |
| `FUN_00e060b0` | `0x00e060b0` | `__thiscall fn(this, int, u32, u32)` | Sub-handler. |

**Cross-ref**: full XOR-delta bit table at [`state-flag-broadcast.md`](state-flag-broadcast.md).

**Recommended detour**: read `bStateField` from event_data, compare against last-seen value, emit `client.state.flag_change` with `flag_name` (decoded from bit) and `new_value` fields. Sample 1/1 (rare event — combat/stealth/holster don't spam).

---

### Animation notify

| Target | Address | Signature | Hook role |
|---|---|---|---|
| `USGWAnimNotify_Event::Notify` (variant A) | **`0x00e974b0`** | `__thiscall fn(this, int)` | Body 0x00e974b0 - 0x00e9798b (1.2 KB). Inline hook. |
| `USGWAnimNotify_Event::Notify` (variant B) | **`0x00e97070`** | `__thiscall fn(this, int)` | Body 0x00e97070 - 0x00e97235 (450 B). Inline hook — likely a specialized notify variant. |

Two functions because UE3's animation system has both `Notify` (generic) and `NotifyTick` (per-frame during notify). Both are inline-hookable; emit `client.engine.anim_notify` with the notify type if recoverable from event_data offset 0x?? (needs decompile during impl).

---

### Cooked-data PAK load

| Target | Address | Signature | Hook role |
|---|---|---|---|
| Cooked-data category load entry | **`0x00420074`** | `__cdecl fn(u32, void**, void**)` | 6.6 KB body. Fires once per PAK load (21 categories). Hook for `client.engine.pak_load` with PAK name (recoverable from the second param). |

---

### Appearance + loot (CME auto-discovery, no per-event hook needed)

| Event | RTTI | Notes |
|---|---|---|
| `Event_AppearanceJob_Completed` | `0x01e21c80` (`.?AUEvent_AppearanceJob_Completed@@`) | 5 subscribers (SequenceManager, CharacterCreation, GameProxyPlayer, GameBeing, PortraitManager). Auto-discovered by Phase 3a's RTTI scan; no explicit hook code. |
| `Event_NetIn_LootDisplay` | (via Phase 3a RTTI scan) | The doc's two addresses (`0x00d804f0`, `0x00e248f0`) are 5-byte CME registration stubs (return `char*` const name and `TypeDescriptor*`); they are NOT dispatchers. Real hook is via CME subscribe. |

---

## Phase 3 — Tier 4 kismet/dispatcher (vtable swaps)

These are the highest-risk hooks in the entire plan because they sit in the script-execution hot path. **Do not enable in the same PR as their resolution work** — separate PR with focused review.

### Base RTTI starting points

| Class | RTTI string | Address |
|---|---|---|
| `UObject` | `.?AVUObject@@` | `0x01dae610` |
| `AActor` | `.?AVAActor@@` | `0x01db517c` |
| `USequence` | `.?AVUSequence@@` | `0x01dc37fc` |
| `APlayerController` | `.?AVAPlayerController@@` | `0x01dc8d88` |
| `FFrame` | `.?AUFFrame@@` | `0x01daf8d4` (useful for ProcessEvent identification — FFrame is constructed on the stack at the top of ProcessEvent) |

### AActor vtable (partial walk)

**AActor vtable @ `0x0183c408`** (COL @ `0x01b5770c`):

| Slot | Address | Notes |
|---|---|---|
| 0 | `0x00767210` | virtual ~AActor |
| 1 | `0x00769440` | (unknown) |
| 2 | `0x00561350` | (unknown) |
| 3 | `0x00696310` | (unknown) |
| 4 | `0x00af6810` | `FFileManagerError::vfunc_0` placeholder (stubbed) |
| 5 | `0x00872d60` | (unknown) |
| 6 | `0x004a0ec0` | (unknown) |
| ...25-30 | candidates for `AActor::Tick(FLOAT, ELevelTick)` | needs per-slot decompile to identify Tick by signature pattern (2 args after `this`, the first is `float` DeltaSeconds, large body) |

### Resolved vtable slots (2026-06-04 second Ghidra pass)

| Target | Signature | Slot | Entry | Notes |
|---|---|---|---|---|
| **`AActor::Tick`** | `__thiscall UBOOL(this, FLOAT DeltaSeconds, ELevelTick TickType)` | **88** | **`0x005e4200`** | Vtable `0x0183c40c`. Ghidra annotation confirms slot 88 across ~110 Actor subclasses (AKeypoint, APawn, AVehicle, ABrush, AGameInfo, ALight family, ACoverLink, ASGWRegion, etc.). Decompile confirms `param_2 == LEVELTICK_TimeOnly` branch + per-subclass virtual dispatch via `[this+0xf0, +0x1a0, +0x1a4, +0x1a8, +0x1d4]`. Body 333 B. **Sample 1/10 per actor class** + re-entry guard. |
| **`USequence::UpdateOp`** (the "Tick" for kismet) | `__thiscall UBOOL(this, FLOAT DeltaTime)` | **84** | **`0x006c61c0`** | Vtable `0x01854a84`. Also at slot 84 in UUISequence, UUIStateSequence. Decompile shows `check("!HasAnyFlags(RF_Unreachable)", ".\Src\UnSequence.cpp", 0x989)`, `kismetSequenceTimeout` config key, ElapsedTime accumulator at `this+0x114`, emits `Event_Kismet_SequenceFinished` CME event on finish. Body 855 B. **Sample 1/1** — sequences are infrequent. |

### Deferred slot identification (vtable known, slot needs implementation-time decompile)

| Target | UE3 source signature | What's known | What's deferred |
|---|---|---|---|
| `UObject::ProcessEvent` | `void(UFunction* Function, void* Parms, void* Result = NULL)` `__thiscall` | RTTI `.?AVUObject@@` @ `0x01dae610`; type_info @ `0x01dae608`; vtable @ `0x0180fe54` (~67 slots). **Ruled out**: slots 29 (`__thiscall(this, int*)`), 30 (same), 31 (same), 32 (`__thiscall void(this)` — single-arg, decompile shows `UnObj.cpp` line 0xCCA `GObjInitialized` check, behaves like `PostLoad`/`UpdateDefaults`), 40 (`Rename(const TCHAR*, UObject*, ERenameFlags)`). | The remaining ~35 unverified slots need a focused decompile-pattern search for `4-arg __thiscall void(this, void*, void*, void*)` with FFrame stack construction (~140-160 bytes alloca, FFrame vtable assignment near function entry) and indirect call to `[UFunction+0xa8]` (= UFunction::Func, the resolved native pointer). Estimate: 15-30 min of focused work with UE3 leaked-source headers open. **MANDATORY for hook**: FName allowlist + thread-local re-entry guard. Sample 1/100 default, 1/1 in diag mode. |
| `PropertyNode<T>` get/set | `virtual T get()`, `virtual void set(T)` per specialization | RTTI `.?AVPropertyNode@Detail@CME@@` @ `0x01daadb0` (base); type_info @ `0x01daada8`; COL @ `0x01b50e6c`. Adjacent RTTI strings include `BasicPropertyList`, `BasicPropertyTree`. | PropertyNode is heavily templated with 6+ specializations (`Property<long/int/bool/float/Vector3/wstring>` per the original anchor doc). Each specialization has its own vtable; each `get` + `set` is at the same slot index within their respective templated vtables. Implementation strategy: walk the base PropertyNode vtable to identify `get`/`set` slot indices, then enumerate the per-T vtables via RTTI scan and hook each. Estimate: 1-2 hours of focused work. |
| `SequenceManager::OnSequence` | (CME auto-discovered) | `Event_NetIn_onSequence` — caught by Phase 3a RTTI scan. | No address needed. |
| `Event_Kismet_SequenceFinished` | (CME auto-discovered) | RTTI scan. | No address needed. |

### APlayerController::ConsoleCommand (inline-hookable, simpler)

| Target | Address | Signature | Notes |
|---|---|---|---|
| `APlayerController::execConsoleCommand` | **`0x00539850`** | `__thiscall fn(this, int)` | The UnrealScript exec wrapper, registered in the FuncMap at `0x01db2460` paired with the string `"intAPlayerControllerexecConsoleCommand"` @ `0x01821250`. Ghidra auto-labeled it `AActor_execConsoleCommand` (heuristic miss — the FuncMap binding is authoritative). Hook here for `client.input.console_command`. |

---

## Phase 4 — Tier 5 UI/Lua/input

### CEGUI logger (vtable swap)

| Target | Address | Signature | Hook role |
|---|---|---|---|
| `CEGUI::Logger` (base) RTTI | `0x01de9c40` | — | Base abstract class. |
| `CEGUI::DefaultLogger` RTTI | `0x01e7daf4` | — | Concrete implementation; this is the class we subclass-swap. |
| `CEGUI::DefaultLogger` vtable | `0x01ac1ba8` | — | COL ptr at `0x01bf6628`. |
| **`DefaultLogger::logEvent` (vtable slot 1)** | **`0x012129E0`** | `__thiscall void(this, String const& message, LoggingLevel level)` | 920-byte body. **Primary hook target.** Captures every CEGUI log line (UI events, button clicks, layout loads, script errors). Sample 1/1 — UI events are rare. Emit `client.ui.cegui_log` with `message` + `level` fields. |

### Lua IAT entries (lua51.dll)

| API | IAT slot | Mangled name |
|---|---|---|
| `lua_pcall` | **`0x01988A0C`** | `?lua_pcall@@YAHPAUlua_State@@HHH@Z` |
| `lua_call` | **`0x01988904`** | `?lua_call@@YAXPAUlua_State@@HH@Z` |
| `lua_newstate` | **`0x01988656`** | `?lua_newstate@@YAPAUlua_State@@P6APAXPAX0II@Z0@Z` |

**IAT swap technique**: write a detour fn with matching signature, atomically swap the slot value, save the original. Every call from SGW.exe to `lua_pcall` / `lua_call` routes through the detour. Emit `client.lua.pcall` and `client.lua.call` with the function name (read from the Lua stack). Sample 1/1 — scripted UI calls aren't that frequent.

(58 lua51.dll imports total per the original anchor doc; the 3 above are the script-execution entrypoints. Other Lua imports — `lua_setfield`, `lua_pushvalue`, etc. — are observation-uninteresting.)

### Console + keypress

`APlayerController::execConsoleCommand` already covered above under Tier 4 (`0x00539850`).

**Keypress dispatch**: deferred — no explicit address in the anchor doc. UE3 routes keys via `UInput::InputKey` (virtual) → `APlayerController::InputKey` (virtual). Resolution strategy: walk `APlayerController` vtable from RTTI `0x01dc8d88` and find the slot matching `InputKey(EInputEvent, FName, EInputEvent, FLOAT, INT)` signature. Defer to Phase 4 implementation.

### Slash commands

`Event_SlashCmd_*` (1,477 events) handled by Phase 3a CME auto-discovery — no per-command address.

---

## Phase 5 — Tier 6 subsystem correlators

### FMOD (runtime-resolved — strategy change vs original doc)

| Component | Detail |
|---|---|
| `fmodex.dll` (IDT name) | `0x01d88228` |
| `fmod_event.dll` (IDT name) | `0x01d884c8` |
| `fmod_event_net.dll` (IDT name) | `0x01d8858c` |
| Only static import: `_FMOD_EventSystem_Create@4` | IAT slot **`0x01988234`** |

**Caveat**: the original anchor doc treated FMOD as IAT-hookable, but FMOD only statically imports `EventSystem_Create`. All other FMOD calls (`EventInstance::start`, `::stop`, etc.) are resolved at runtime via the COM-style vtable returned from `EventSystem_Create`. **Phase 5 implementation strategy**:

1. IAT-hook `FMOD_EventSystem_Create` to capture the `FMOD::EventSystem*` returned by FMOD.
2. From that pointer, walk FMOD's vtable to reach `EventSystem::getEvent` → `Event::createInstance` → `EventInstance::start/stop`.
3. Inline-hook or vtable-swap the resolved entries.

This is more work than a simple IAT walk and warrants its own PR.

### Bink (cinematic playback)

| Target | Address | Signature | Hook role |
|---|---|---|---|
| `FFullScreenMovieBink::Tick` (vfunc_1) | **`0x0050BBC0`** | `__thiscall fn(this, float DeltaSeconds)` | 154-byte body. Fires every frame during cinematic playback. **Primary hook target** — distinguishes cinematic-bound from stall. Emit `client.engine.bink_tick` (sampled 1/30 → ~1 emit/sec at 30 fps). |
| `BinkRender` string | `0x0181BA54` | — | ASCII string, used in cinematic log path. |
| `InitBinkRender` wstring | `0x0181BC2C` | — | UTF-16 string. Helper fn at `0x005080a0` is a 5-byte stub returning this string (NOT a hookable init function). |
| BinkW32.dll exports (full IAT list available) | `0x01989076` – `0x019891E8` range | various | If finer-grained Bink telemetry needed (per-frame decode, audio sync), the BinkW32 IAT has 17 imported functions in the contiguous range above. |

### PropertyNode<T> (CME's parallel observable system)

| Target | Address | Notes |
|---|---|---|
| `PropertyNode<Detail::CME>` RTTI | **`0x01daadb0`** | `.?AVPropertyNode@Detail@CME@@`. Generic base for the typed `Property<long/int/bool/float/Vector3/wstring>` family. |

**Implementation note**: PropertyNode is a templated observable system parallel to EventSignal — get/set virtual methods fire on every property change. Walk the RTTI to the base vtable (same RTTI→COL→vtable pattern as Phase 2), identify the `get` / `set` slots, then inline-hook or vtable-swap. Adjacent RTTI strings (`BasicPropertyList`, `BasicPropertyTree`) at `0x01daadcc`+ are related types.

### OS thread / module / focus correlators (IAT slots)

| API | IAT slot | DLL | Use |
|---|---|---|---|
| `CreateThread` | **`0x0196B65A`** | KERNEL32 | Thread timeline baseline — every thread the engine spawns becomes a SigNoz event with stack trace. |
| `LoadLibraryW` | **`0x0196B5BC`** | KERNEL32 | Module timeline + trigger for IAT re-scan (some DLLs load late). |
| `LoadLibraryA` | **`0x0196B5AC`** | KERNEL32 | Same as above; capture both for completeness. |
| `GetForegroundWindow` | **`0x0196AF20`** | USER32 | Focus correlation — was the user alt-tabbed during the stall? |

**Note**: original anchor doc cited `0x01d6b65c` for `CreateThread`, etc. — those addresses are stale or from a different binary. The IAT slots above were extracted from Ghidra's current external-locations table for SGW.exe on 2026-06-04 and are authoritative.

---

## Phase 6 — Tier 7 crash + on-disk

### IAT slots for crash filter replacement

| API | IAT slot | DLL | Use |
|---|---|---|---|
| `SetUnhandledExceptionFilter` | **`0x0196BAB2`** | KERNEL32 | IAT-replace to install our filter; save the original so we can chain after writing the minidump. |
| `MiniDumpWriteDump` | **`0x01987B76`** | DBGHELP | Called by our crash filter to write the minidump. No hook on this one — we call through it. |

**Implementation discipline**: the crash filter runs *during* a process crash. **Must be allocation-free, no panics, no Rust runtime assumptions** — the heap may be corrupt, thread-locals may be invalid, the loader lock may be held. Use only stack buffers + raw syscalls. Save the minidump filename to a pre-allocated buffer, write the dump, then call the saved original `SetUnhandledExceptionFilter` callback for the normal crash report flow.

### On-disk artifacts (no Ghidra needed)

| Surface | Path / source | Technique |
|---|---|---|
| Minidumps | `Binaries/CrashDumps/*.dmp` | Filesystem poll, tail-and-ship per file (one-shot — file is rotated by name). |
| Debug log | `Binaries/SGWDebugLog.log` | Tail same way launcher already handles `sgwdebuglog*` rotations. |
| Build marker | `EpicInternal.txt` | One-shot read at DLL bootstrap. |
| Atera config | `AtreaLoader.config.xml` | One-shot read at bootstrap (reference only — confirms which Atera config was active). |
| Session pcap / keys / log | `Binaries/sessions/*.{log,pcap,keys.txt}` | **DO NOT SHIP from DLL** — launcher already does. Duplicate shipping would double the SigNoz storage. |

---

## Summary table — every Phase 3-6 address at a glance

| Phase | Tier | Hook | Address | Technique |
|---|---|---|---|---|
| 3 | 3 | `GameBeing::onStateFieldUpdate` | `0x00e01c90` | Inline |
| 3 | 3 | `USGWAnimNotify_Event::Notify` (A) | `0x00e974b0` | Inline |
| 3 | 3 | `USGWAnimNotify_Event::Notify` (B) | `0x00e97070` | Inline |
| 3 | 3 | Cooked-data PAK load | `0x00420074` | Inline |
| 3 | 4 | `APlayerController::execConsoleCommand` | `0x00539850` | Inline |
| 3 | 4 | `UObject::ProcessEvent` | vtable `0x0180fe54`, slot TBD (5 candidates ruled out) | Vtable swap |
| 3 | 4 | `AActor::Tick` | **`0x005e4200` (slot 88)** | Vtable swap |
| 3 | 4 | `USequence::UpdateOp` | **`0x006c61c0` (slot 84)** | Vtable swap |
| 4 | 5 | `CEGUI::DefaultLogger::logEvent` | `0x012129E0` | Vtable swap |
| 4 | 5 | `lua_pcall` | IAT `0x01988A0C` | IAT |
| 4 | 5 | `lua_call` | IAT `0x01988904` | IAT |
| 4 | 5 | `lua_newstate` | IAT `0x01988656` | IAT |
| 5 | 6 | `FMOD_EventSystem_Create` | IAT `0x01988234` | IAT + vtable traversal |
| 5 | 6 | `FFullScreenMovieBink::Tick` | `0x0050BBC0` | Inline |
| 5 | 6 | `PropertyNode<T>` get/set | RTTI `0x01daadb0`, COL `0x01b50e6c`, slot TBD (per-T enumeration) | Vtable swap |
| 5 | 6 | `CreateThread` | IAT `0x0196B65A` | IAT |
| 5 | 6 | `LoadLibraryW` | IAT `0x0196B5BC` | IAT |
| 5 | 6 | `LoadLibraryA` | IAT `0x0196B5AC` | IAT |
| 5 | 6 | `GetForegroundWindow` | IAT `0x0196AF20` | IAT |
| 6 | 7 | `SetUnhandledExceptionFilter` | IAT `0x0196BAB2` | IAT replace |
| 6 | 7 | `MiniDumpWriteDump` | IAT `0x01987B76` | Direct call (no hook) |

**Total**: 21 hook surfaces across Phases 3-6.
**Resolved upfront** (after the 2026-06-04 + second-pass Ghidra work): **19** (17 from the first pass + AActor::Tick + USequence::UpdateOp from the second pass).
**Slot-deferred** (vtable identified, slot needs per-slot decompile at impl time): **2** — `UObject::ProcessEvent` (vtable at `0x0180fe54`, 5 candidate slots ruled out) and `PropertyNode<T>` get/set (RTTI + COL identified, per-T enumeration needed).
**Auto-discovered via CME RTTI scan** (Phase 3a): `Event_AppearanceJob_Completed`, `Event_NetIn_LootDisplay`, `Event_NetIn_onSequence`, `Event_Kismet_SequenceFinished`, `Event_SlashCmd_*` (1,477).

---

## Ghidra annotations applied 2026-06-04

The following Ghidra function renames + plate comments were applied so future sessions don't redo this work:

- `0x00416ec0` → `FEngineLoop__Tick` (Phase 2)
- `0x0054e9c0` → `UWorld__UpdateLevelStreamingInner` (Phase 2)
- `0x005527a0` → `UWorld__UpdateLevelStreaming` (outer iterator, Phase 2)
- `0x004c7ae0` → `FArchiveAsync__Serialize` (Phase 2)
- `0x004a8e10` → `UObject__StaticLoadObject` (Phase 2, corrected from earlier `LoadPackageInternal` mis-id)
- `0x005e4200` → `AActor__Tick` (vtable slot 88; this manifest second pass)
- `0x006c61c0` → `USequence__UpdateOp` (vtable slot 84; this manifest second pass)

Phase 3-6 entries above have function info but the rest were not renamed in this pass — the resolution is documented here and re-running Ghidra walks is unnecessary; rename at implementation time.
