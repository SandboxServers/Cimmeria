# Client Instrumentation Hookpoints

> **Diátaxis type**: reference
> **Audience**: engineers writing or reviewing `cimmeria-client-telemetry` hooks (issue #417)
> **Last updated**: 2026-05-26
> **Confidence**: HIGH for Tier-1 anchors validated by Ghidra decompile + string-search pre-issue. MEDIUM for Tier-2+ anchors derived from existing RE docs without per-anchor re-verification.

The injected client-side telemetry DLL (issue #417) hooks SGW.exe at the addresses and entry points catalogued here. Anchors are stable because SGW.exe has ASLR disabled (`AtreaFixASLR.bat` clears `IMAGE_DLLCHARACTERISTICS_DYNAMIC_BASE` on-disk), so addresses are the same on every run.

Hook techniques: **CME subscribe** (register a fake-vtable subscriber via `CmeEventSignal_Subscribe`), **inline** (retour-rs at a known address), **vtable swap** (atomic store on a known vtable slot), **IAT** (rewrite import-table slot), **interpose** (CEGUI / log4cxx — install a subclass at runtime via the host framework's own registration API).

## CME EventSignal — the framework

The killer surface: an injected DLL can register subscribers to any of the client's typed event signals via the host's own pub-sub API. No patching required, no type gatekeeper on Subscribe.

| Symbol | Address | Role |
|---|---|---|
| `CmeEventSignal_GetSystem` | `0x0155f790` | Singleton accessor for the CME EventSignal system |
| `CmeEventSignal_LookupByName` | `0x00a5c0f0` | Resolve a signal handle from a name string |
| `CmeEventSignal_SetField` | `0x0043b850` | Set a key/value field on a signal object |
| `CmeEventSignal_Subscribe` | `0x00a5c150` | **Subscriber insertion — no RTTI check, std::set insert by pointer** |
| `CmeEventSignal_InvokeMemberCallback` | `0x00e04570` | Dispatch site — unconditionally calls `*(this+0x8)` |

**`CmeMemberCallback` ABI** (12 bytes, `[+0x0 vtable][+0x4 subscriber][+0x8 method_ptr]`). To register a hook:

1. Statically allocate a 12-byte struct in our DLL's `.data`.
2. Stamp `+0x0` with a pointer to a fake vtable we own (slot 0 = destructor — no-op for static allocation; slot 5 = `InvokeMemberCallback` — point to the real `0x00e04570` so dispatch finds our method_ptr).
3. Put a pointer to per-hook state at `+0x4` (becomes the `this` of our handler).
4. Put our `extern "thiscall" fn(this, event_data)` at `+0x8`.
5. Look up the target signal handle via `CmeEventSignal_LookupByName(system, "Event_NetIn_<X>")`.
6. Call `CmeEventSignal_Subscribe(handle, our_callback)`.

**Lifetime rule:** the signal stores subscribers by pointer with no refcount and no destructor. Our `CmeMemberCallback` objects MUST live for process lifetime.

**Unsubscribe path: REFUTED** (Ghidra decompile, 2026-05-26).

The neighbourhood map around Subscribe:

| Address | Role |
|---|---|
| `0x00a5bed0` | `ostream` error-reporting helper — not subscribe-related |
| `0x00a5c0f0` | LookupByName analogue (shares the `FUN_0158ea90` set-insert wrapper) |
| `0x00a5c150` | **Subscribe** |
| `0x00a5c190` | SEH wrapper → calls `0x00a5c690` (signal-internal teardown) |
| `0x00a5c1d0` | Empty-set sentinel constructor at `+0x4` |
| `0x00a5c210` | Red-black-tree node unlink/rebalance (internal — calls `scalable_free` on the node) |
| `0x00a5c580` | Full `std::set::erase`-equivalent (validates iterator at `+0x2d`, calls `0x00a5c210`) |
| `0x00a5c690` | Signal's own teardown — only caller of `0x00a5c580`, calls `scalable_free` on `*(param_1+4)` |

`FUN_00a5c580` (the erase) is called **only from `FUN_00a5c690`**, which is the signal's own internal teardown path — not an externally-callable Unsubscribe. There is **no `Unsubscribe` symbol** in the binary. The destructor at `CmeMemberCallback` vtable slot 0 is the **RTTI type-descriptor accessor**, not an auto-unsubscribe.

**ABI confirmation:** the subscriber set is `std::set<CmeMemberCallback*>` keyed by raw pointer value (not by RTTI name string). The RTTI name at vfunc_3 only matters at dispatch time; set membership is pointer-equality.

**Required posture for the injected DLL:**

1. **`CmeMemberCallback` objects live in `.data` as `static`s** — fixed address, no allocator dependency, immune to DLL_PROCESS_DETACH timing. NOT heap allocations.
2. **DLL never unloads.** If detach were to free the DLL's `.data`, the next signal emit would dispatch through a dangling vtable pointer and crash SGW.exe. Our `DLL_PROCESS_DETACH` handler in [`crates/client-telemetry/src/boot.rs`] is a deliberate no-op for this reason.
3. **No defensive-cleanup code that calls into the signal system on shutdown** — there is no API to call.

See [`cme-event-signal.md`](cme-event-signal.md) for the full Pattern A vs Pattern B emit pipeline analysis. The subscribe path is uniform across both patterns.

**RTTI scan for auto-discovery:** the `.rdata` section contains 935 decorated `TypedEmitInfo` MSVC type-descriptor strings (e.g. `.?AU?$TypedEmitInfo@VEvent_NetIn_onClientMapLoad@@@EventSignal@CME@@`). At DLL load we can walk `.rdata`, prefix-match `.?AU?$TypedEmitInfo@`, demangle, and enumerate every CME event class without hardcoding names. This is the basis for Phase 2's auto-generated subscription set.

## Tier 1 — Critical path for the relog freeze (Phase 1)

The minimum hook set that turns "client froze somewhere during world entry" into a SigNoz trace.

| Function | Anchor → Entry | Technique | Event name | Status |
|---|---|---|---|---|
| `FEngineLoop::Tick` | RTTI `0x01d8f838` → **entry `0x00416ec0`** | Inline | `client.engine.tick` (sampled 1/100) | ENABLED (PR #504) |
| `UWorld::UpdateLevelStreaming` | StreamingLevel xref @ `0x01837518` → **entry `0x0054e9c0`** | Inline | `client.engine.update_level_streaming` | SCAFFOLDED — entry resolved, signature (Ghidra recovered `this, int, float*`) needs RE before enable to avoid stack-mismatch crash |
| `ULevelStreaming::SetLevelStatus` | `0x01837518`, `0x01906e30` | Inline | `client.streaming.state_change` | CONFIRMED |
| `FArchiveAsync::Serialize` (vtbl slot 1) | RTTI `0x01dafd0c` → vtable `0x01814198` → **entry `0x004c7ae0`** | Inline | `client.engine.async_archive_serialize` (sampled 1/1000) | ENABLED (PR #504) |
| `LoadPackageInternal` | "FailedLoadPackage" xref @ `0x0180f104` → **entry `0x004a8e10`** | Inline | `client.engine.load_package` | SCAFFOLDED — entry resolved (192 xrefs, recursive self-call confirms `LoadPackageInternal`), but Ghidra recovered 7 cdecl args; need to map to UE3-source signature before enable |
| `Event_NetIn_onClientMapLoad` handler | TypedEmitInfo `0x01e4da90` | CME subscribe | `client.network.on_client_map_load` | CONFIRMED |
| `Event_NetIn_onClientReady` handler | string `0x019c2828` ("onClientReady") | CME subscribe | `client.network.on_client_ready` | CONFIRMED (string at address is the bare `onClientReady` — the full `Event_NetIn_onClientReady` is the RTTI-derived signal name) |
| `recvfrom` / `WSARecv` | `ws2_32.dll` IAT | IAT | `client.os.udp_recv` (sampled) | DEFERRED — IAT walker work |
| `Mercury::Nub::handleMessage` | `0x01b18be0` | Inline | `client.mercury.dispatch` | CONFIRMED |
| log4cxx appender tee | `log4cxx.dll` (config @ `SGWLogConfig.xml`) | Interpose | `client.log.<level>` | DEFERRED — appender install path TBD |

**Anchor corrections from the issue body's original draft:**

- `"Decrypted packet received"` — **does not exist** in the binary. The actual post-decrypt entry is `Mercury::Nub::handleMessage` at `0x01b18be0` (string anchor `Mercury::Nub::handleMessage: received the wrong kind of message!`).
- `0x019c2828` — the string at this address is `onClientReady` (bare), not `Event_NetIn_onClientReady`. The full event name comes from the RTTI descriptor; the bare string is the in-binary anchor.

## Tier 2 — Network protocol visibility (Phase 2)

| Hook category | Technique | Count | Discovery |
|---|---|---|---|
| All `Event_NetIn_*` handlers | CME subscribe | ~120 classes | RTTI walk at DLL load |
| All `Event_NetOut_*` handlers | CME subscribe | ~150 classes | RTTI walk at DLL load |
| `Event_Net_Connected` / `Event_Net_Disconnected` | CME subscribe | 2 | RTTI walk |
| `Event_Cache_ElementReady` / `Event_Cache_ElementError` | CME subscribe | ~10 | RTTI walk |
| `BWConnection::ConnectFailure` | Inline | 1 | String anchor `0x0180b9f4` |
| `BWConnection::NotifyConnectionLost` | Inline | 1 | String anchor `0x0182d114` |
| `BWConnection::ConnectionTimeout` | Inline | 1 | String anchor `0x018474ec` |
| Mercury Nub thread entry | Inline | 1 | RTTI `CME::Win32ThreadEx::ThreadEntry<Mercury::Nub::NetworkTask>` @ `0x01b18f78` |

**Binary string counts** (validated via Ghidra `search_strings`):
- `Event_NetOut_*` raw occurrences: **1,946** (issue body previously claimed 3,426 — corrected here)
- `Event_NetIn_*` raw occurrences: **1,433** (distinct handler class count ~120 — RTTI-derivable)
- `Event_SlashCmd_*`: **1,477** (exact match for the issue body's claim)

## Tier 3 — Game state, animation, effects, loot (Phase 3)

Anchors from existing RE docs; per-anchor Ghidra revalidation deferred to implementation time.

| Hook | Anchor | Technique | RE source |
|---|---|---|---|
| State flag broadcast — `BSF_InCombat`, `Crouching`, `Stealth`, `MovementLock`, `Walking`, `Holster`, `Dead`, + 2 others | `0x00e01c90` (dispatcher) + per-flag handlers at `0x00e7b4c0`, `0x00e6e330`, `0x00dfff70`, `0x00e31aa0`, `0x00e060b0` | Inline | [`state-flag-broadcast.md`](state-flag-broadcast.md) |
| `Event_AppearanceJob_Completed` | RTTI `0x01e21c80` (5 subscribers: `SequenceManager`, `CharacterCreation`, `GameProxyPlayer`, `GameBeing`, `PortraitManager`) | CME subscribe (free piggyback on existing signal) | [`appearance-system.md`](appearance-system.md) |
| `USGWAnimNotify_Event::Notify` | `0x00e974b0`, `0x00e97070` | Inline | [`animation-system.md`](animation-system.md) |
| `onEffectResults` dispatch (16 result codes incl. `EFFECT_PULSE_BEGIN`/`END`) | CME via `Event_NetIn_*` | CME subscribe | [`effect-execution-model.md`](effect-execution-model.md) |
| Cooked data category load (21 PAKs) | `0x00420074` | Inline | [`cooked-data-pipeline.md`](cooked-data-pipeline.md) |
| `Event_NetIn_LootDisplay` + `DBInvItem` cache warm | `0x00d804f0`, `0x00e248f0` | CME subscribe | [`loot-generation.md`](loot-generation.md) |

## Tier 4 — Kismet, dispatcher, matinee (Phase 3 cont.)

| Hook | Technique | Sampling |
|---|---|---|
| `SequenceManager::OnSequence` | CME subscribe via `Event_NetIn_onSequence` | 1/1 |
| `USequence::Tick` | Inline | 1/1 |
| `Event_Kismet_SequenceFinished` | CME subscribe | 1/1 |
| `UObject::ProcessEvent` | Vtable swap | 1/100 default, 1/1 in diag mode — **FName integer allowlist + thread-local re-entry guard mandatory** |
| `AActor::Tick` | Vtable swap | 1/10 per actor class |

## Tier 5 — UI / Input / Lua / slash commands (Phase 4)

| Hook | Anchor | Technique |
|---|---|---|
| `CEGUI::Logger` | RTTI `0x0192c2bc` | Vtable subclass swap — captures every UI log line, button clicks, layout loads, script errors, focus events |
| `lua_pcall` / `lua_call` | `lua51.dll` IAT (58 imports) | IAT — captures every scripted UI action |
| `Event_SlashCmd_*` (1,477) | CME subscribe | RTTI-driven auto-discovery |
| `APlayerController::ConsoleCommand` | Inline | 1/1 |
| Keypress dispatch | Inline | Sampled |

## Tier 6 — Subsystem correlators (Phase 5)

| Hook | Anchor | Why |
|---|---|---|
| FMOD `EventInstance::start` / `stop` | `fmodex.dll` (`0x01d88228`), `fmod_event.dll` (`0x01d884c8`), `fmod_event_net.dll` (`0x01d8858c`) | "Client thinks combat started" vs server state — desync correlator |
| `BinkRender` / `InitBinkRender` | `0x0181ba54`, `0x0181bc2c` | Cinematic boundary markers — distinguishes stall from expected video |
| `PropertyNode<T>` get/set | RTTI `0x01daadB0` (Property<long/int/bool/float/Vector3/wstring>, BasicPropertyList, BasicPropertyTree) | CME's parallel observable system to EventSignal |
| `CreateThread` IAT | `0x01d6b65c` | Thread timeline baseline |
| `LoadLibraryW` IAT | `0x01d6b5be` | Module timeline + trigger for IAT re-scan |
| `GetForegroundWindow` IAT | `0x01d6af22`, `0x01b2de1c` | Focus correlation (alt-tab during stall?) |

## Tier 7 — Crash + on-disk artifact shipping (Phase 6)

| Surface | Source / IAT | Technique |
|---|---|---|
| Unhandled exceptions | `SetUnhandledExceptionFilter` IAT @ `0x01d6bab4` | Replace, write minidump via `MiniDumpWriteDump` IAT @ `0x01d87b78`, ship as correlated event, call original |
| `Binaries/CrashDumps/*.dmp` | On-disk poll | Tail-and-ship (one-shot per file) |
| `Binaries/SGWDebugLog.log` | On-disk tail | Same pattern as launcher telemetry uses for `sgwdebuglog*` rotations today |
| `Binaries/sessions/*.{log,pcap,keys.txt}` | On-disk | Launcher already ships these — DLL must not duplicate |
| `EpicInternal.txt` | One-shot read at session start | Diagnostic build marker |
| `AtreaLoader.config.xml` | One-shot read at session start | Reference only — confirms which Atera config was active |

## Subsystems we intentionally don't hook

To preserve "observe without changing behavior":

- **`FMalloc::Malloc`** (`FMallocCME` at RTTI `0x01d8f87c`) — millions of calls/sec; allocator hooks blow up log replay.
- **`UObject::ConditionalDestroy`** and GC-adjacent paths — use-after-free risk.
- **`BeginScene` / `EndScene`** and render-state-affecting D3D9 entry points.
- **PhysX inner-loop callbacks** — high-frequency on dedicated threads.
- **`wxWidgets`** (statically linked, Atrea editor framework) — not in the game-client surface.

## Related docs

- [`cme-event-signal.md`](cme-event-signal.md) — full CME emit pipeline, Pattern A vs Pattern B, MemberCallback layout
- [`../address-map.md`](../address-map.md) — canonical address registry
- [`../../technical/atrearl-loader.md`](../../technical/atrearl-loader.md) — third-party reference for what hook surfaces work in practice (not code we use)
- [`../../technical/atrealoader-exe.md`](../../technical/atrealoader-exe.md) — third-party loader's behavior as RE reference
- [`../../architecture/client-telemetry.md`](../../architecture/client-telemetry.md) — design + stack picks for the injected DLL (TBD — lands with Phase 1 PR)
