# Client-Side Telemetry — Architecture

> **Diátaxis type**: explanation
> **Audience**: engineers extending or reviewing the `cimmeria-client-telemetry` DLL and its launcher-side injector (issue #417)
> **Last updated**: 2026-09-28
> **Engine capture (2026-09-28)**: adds the engine layer on top of the hooks below: log sinks for BigWorld, UE3, log4cxx and the OS, plus subsystem seams (actors, Matinee, level streaming, Bink, FMOD, PhysX, file I/O, D3D9, frame health): 6 inline, 4 vtable and 15 IAT hooks, a vectored exception handler and 3 D3D9 COM patches. See [Engine layer: log sinks and subsystem seams](#engine-layer-log-sinks-and-subsystem-seams). Statically verified, not yet run in a live client.
> **Status**: Phases 2-5 substantially landed, **none of it yet run inside the real client** (the launcher never injected the DLL; see the fingerprint section). **21 hooks total** across 3 techniques: 11 inline JMP hooks (4 engine + state-flag dispatcher, anim notify A+B, console command, Bink tick, entity-method drop oracle, CME event-registry lookup), 7 IAT-swap hooks (3 Lua + 4 OS), 3 vtable-swap hooks (CEGUI logger, now with the line text + AActor::Tick + USequence::UpdateOp). The two CME subscribers (`onClientMapLoad`, `onClientReady`) were removed on 2026-09-28: the subscribe API they called is not one (see Hook taxonomy item 1). Every address was re-checked against the QA `SGW.exe` on 2026-09-27 and is fingerprinted. **Removed pending re-resolution (#989)**: `Mercury::Nub::handleMessage` and the cooked-data PAK load, whose anchors were not function entries. **Deferred**: CME RTTI auto-discovery (~270 more events; needs `.rdata` scanner), FMOD runtime vtable traversal, ProcessEvent slot search, PropertyNode<T> per-T enumeration, Phase 6 crash filter.

How `cimmeria-client-telemetry.dll` is side-loaded into `SGW.exe` by `sgw-launcher`, what it observes, and how those observations flow into SigNoz alongside the server-side OTLP stream.

The per-anchor hook table lives in [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md). This document is the design rationale and the stack-pick justification — read the anchor doc for "where do I hook function X?" and this doc for "why does the whole thing look like this?"

## Goal

Give server-side debuggers a client-side view of every meaningful event happening inside `SGW.exe` — frame ticks, level streaming transitions, async I/O completions, CME EventSignal dispatches, log lines, crashes — without modifying game behavior. Output lands in SigNoz as its own service, `service.name = cimmeria-client`, so an end-to-end SigNoz trace can include both server and client spans for the same session.

The motivating use case: the cold-relog freeze investigation (2026-05-26). Server-side we can see Mercury sending N packets and getting N-3 ACKs back; what we can't see is whether the client's render thread is stuck on a disk read or which `.upk` is loading when frame ticks stop. Tier-1 hooks answer that.

## Trust model

Same machine, same user, same launcher session. The launcher is already a trusted desktop app the developer installed — DLL injection doesn't escalate privileges. The DLL ships alongside `sgw-launcher.exe` (signed at deployment time; not per-injection). No remote-code-execution surface: the DLL itself only reads from SGW.exe's memory; outbound network traffic is HTTPS to the launcher's HMAC-protected `/api/telemetry/upload-chunk` endpoint.

**Project preference (issue #417):** authored from scratch. AteraLoader.exe / AtreaRL.dll are reverse-engineering references only — their behaviour in [`docs/technical/atrearl-loader.md`](../technical/atrearl-loader.md) is useful for "what hooks work in practice," not as code we extend or wrap.

**No longer strictly emit-only under `--features lab-bridge`.** The base DLL is emit-only: it reads SGW.exe memory and ships observations out; it takes no inbound commands. The **Live Research Lab** ([`live-research-lab.md`](live-research-lab.md)) adds an inbound command channel behind the off-by-default `lab-bridge` cargo feature — Lua eval, memory read/write, non-freezing hook install, and native calls on the client main thread. Activation is double-gated: the code exists only in a DLL built with that feature, and even then starts only when `current-session.json` carries a `lab` block that only the lab supervisor writes. A telemetry DLL handed to anyone else physically lacks the bridge, so the trust model above holds unchanged for every non-lab build. The lab's rulebook and the operating manual live in [`../guides/live-research-lab.md`](../guides/live-research-lab.md).

## Architecture

```
sgw-launcher (egui)
   inject.rs:
     - create_process_suspended(SGW.exe)
     - inject_dll(process_handle, dll_path)
     - SuspendedProcess::resume()
        |
        v   CreateProcess(SUSPENDED) -> VirtualAllocEx -> WriteProcessMemory
            -> CreateRemoteThread(LoadLibraryW) -> ResumeThread
SGW.exe + cimmeria-client-telemetry.dll
   DllMain  (loader lock — minimum work)
     - record module handle
     - GetModuleFileNameW capture
     - spawn bootstrap thread
     - return TRUE
        |
        v  after loader lock clears
   Bootstrap thread (catch_unwind guard)
     - install hook layer (deferred to Phase 2-7)
     - spawn uploader thread
     - park for process lifetime
        |
        v
   Hook layer                       Event queue (thingbuf MPMC)
     - CME EventSignal subscribe      - Lock-free, bounded ~64K
     - Inline (retour-rs)             - Producer: hooks (never block)
     - Vtable swap                    - Consumer: uploader thread
     - IAT (WinSock, file I/O)        - Drop-on-full -> atomic counter
     - CEGUI::Logger interpose
     - log4cxx file tail
        |
        v   batched every ~2 s
   Uploader thread (ureq + rustls-tls, no tokio)
     - NDJSON gzipped POST
     - HMAC bearer reuses launcher's dev-session token
        |
        v   HTTPS
admin-api / /api/telemetry/upload-chunk    (cimmeria-admin-api)
   - HMAC verify
   - deserialize TelemetryEvent::ClientNative
   - replay through tracing (target client.native)
   - routed to the cimmeria-client log provider
        |
        v   OTLP gRPC
SigNoz   (service.name = cimmeria-client)
```

## Stack picks

Choices made for issue #417 after 2026-current-best-practice research. Each row links to the deliberation in the issue body's "Stack decisions" table; this document records the same choices in operator-grade form.

| Layer | Pick | Why |
|---|---|---|
| Target triple | `i686-pc-windows-msvc` | SGW.exe is 32-bit; `-gnu` was demoted to Rust Tier 2 in May 2025 |
| Init pattern | Bootstrap thread from `DllMain`, `OnceLock`, `catch_unwind` at every FFI seam | Windows loader-lock rules forbid real work in DllMain |
| Inline hooks (v1) | `retour-rs` (Hpmason fork) | CI-green on `i686-pc-windows-msvc`, stable API |
| Inline hooks (v2) | `safetyhook` via thin FFI wrapper | When hot-install during gameplay matters (deferred) |
| IAT hooks | Hand-rolled PE walk + `goblin` | Re-scan on `LoadLibraryW` for delay-loaded modules |
| Vtable hooks | Single atomic pointer store, restore on detach | Standard UE3 modding idiom; atomic + re-entry-safe |
| MPMC ring | `thingbuf` | `crossbeam-queue::ArrayQueue` is provably not lock-free |
| Injection | `CreateProcess(SUSPENDED)` + `CreateRemoteThread(LoadLibraryW)` | Same pattern every ASI loader / ReShade / Special K uses; Defender-default-clean. The injector passes its own `LoadLibraryW` address, so it must be 32-bit like `SGW.exe`: the 64-bit launcher injects through the i686 `sgw-start32` helper ([client-launch README](../../crates/client-launch/README.md)), and a 64-bit direct injection is refused with `BitnessMismatch`. The launcher injects `cimmeria-client-patches` first when both DLLs go in ([client-patches.md](client-patches.md)) |
| HTTP from DLL | `ureq` + `rustls-tls` | No tokio runtime inside an injected DLL; no `opentelemetry-otlp` SDK |
| ProcessEvent filter | `FName` integer allowlist (built once), thread-local re-entry guard | String compare in a function called millions of times/sec halts the game |
| CME subscriber object | `#[repr(C)]` fake-vtable struct, `extern "thiscall"` slots, **static `.data` allocation** | Subscriber lifetime: process-lifetime mandatory (see below) |

## Subscriber lifetime — the hard constraint

The CME EventSignal subscriber set stores subscribers by raw pointer with no refcount, no destructor, and **no externally-callable Unsubscribe path** (Ghidra-validated, see [`client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md#cme-eventsignal--the-framework)).

This is the load-bearing engineering constraint for the entire DLL:

1. **`CmeMemberCallback` objects live in `.data` as `static`s.** Fixed address, no allocator dependency, immune to `DLL_PROCESS_DETACH` timing.
2. **The DLL never unloads.** If detach freed the DLL's `.data`, the next signal emit would dispatch through a dangling vtable pointer and crash SGW.exe.
3. **`DLL_PROCESS_DETACH` is a deliberate no-op** in [`crates/client-telemetry/src/boot.rs`]. There is no API to call; defensive cleanup would itself be the bug.
4. **No heap-allocated subscriber state** — anything stored alongside the fake-vtable must also be `.data` static (or live behind a `&'static` reference). Per-subscriber state goes in a static struct accessible via the subscriber field at `CmeMemberCallback+0x4`.

For instrumentation purposes (read-only telemetry) the "DLL never unloads" stance is correct. The host process exits, the kernel reclaims everything, the constraint never materialises as a leak.

## Hook taxonomy

Seven techniques, applied per-tier per the [hookpoints anchor doc](../reverse-engineering/findings/client-instrumentation-hookpoints.md):

1. **CME EventSignal subscription** — **not built; the premise was wrong.** `0x00a5c150` is `count(name)` on the CME event-factory registry, not a subscribe, and `0x00a5c0f0` creates events from a `std::string` name rather than looking up a signal. The subscriber install was removed on 2026-09-28 ([cme-event-signal.md § Correction](../reverse-engineering/findings/cme-event-signal.md#correction-2026-09-28-the-registry-is-an-event-factory-not-a-subscriber-api)). Its replacement is an inline hook on `0x00a5c0f0` (`client.cme.event`, below), which names every event the client creates by name without subscribing to anything. A real subscriber path (`FUN_00a37790` / `FUN_00a374a0`) is unverified, and the lifetime rules in the previous section apply if it is ever used.
2. **Inline (retour-rs)** — for non-event functions (frame tick, level streaming, async I/O).
3. **Vtable swap** — for `UObject::ProcessEvent`, `AActor::Tick`, `CEGUI::Logger`. Single atomic store, re-entry-safe.
4. **IAT patching** — for WinSock receive, file I/O, thread/library lifecycle. PE walk + `VirtualProtect` dance, re-scan on `LoadLibraryW` for late-binding modules.
5. **String-anchored discovery** — shipping build strips most names; log strings remain. Anchor on a string → xref → that's the function. Pre-loaded into the anchor table from existing RE work.
6. **log4cxx appender tee** — the client already uses log4cxx (`log4cxx.dll` ships, `SGWLogConfig.xml` configures it). Add a custom appender at runtime rather than patching log functions. Zero hot-path patching for the chunk of telemetry already produced as text.
7. **CEGUI::Logger interposition** — `CEGUI::Logger` is a virtual class (RTTI `0x0192c2bc`). Swap in a subclass. Captures every UI log line, button click, layout load, script error, focus event.

## Wire format

The DLL ships events as NDJSON, one event per line, gzipped, POSTed to `/api/telemetry/upload-chunk`. It reads the token and the upload endpoint from `current-session.json` (written by the launcher or the lab supervisor). That file's `upload_endpoint` is the upload **base** (`…/api/telemetry`), as the dev-session mint returns it; the DLL appends `/upload-chunk` unless the value already ends with it (`uploader::chunk_url`). Before 2026-09-29 it posted to the base verbatim, so a launcher-written session never delivered a batch. `client.dll.attached` carries the DLL's `dll_version`.

The event variant on both sides is `ClientNative`:

```json
{
  "type": "client_native",
  "ts_ms": 1700000000000,
  "seq": 42,
  "target": "client.streaming.state_change",
  "level": "debug",
  "fields": {
    "level": "sg1_p9q",
    "status": 2
  }
}
```

Wire shape pinned by paired tests:

- Launcher side: [`crates/launcher/src/telemetry/events.rs`] `client_native_serializes_with_expected_shape`
- Server side: [`crates/admin-api/src/routes/telemetry/`] `client_native_event_matches_launcher_shape`

If either side renames a field, the symmetric test fails loudly.

## Service name routing

Since 2026-09-29 the server runs a fourth OTLP log provider whose resource is `service.name = cimmeria-client` with `cimmeria.source = client` (`otel::client_resource`), and the log routing sends the ingest's `client.native` replays there and nowhere else (`otel::is_client_target`). Query `service.name = 'cimmeria-client'`; the DLL's event name is the `client_target` attribute and the log body. Each row also carries the session's `session_id`, `install_id`, `cimmeria.session_kind` (`lab` or `player`) and `lab`, plus `account_id`, `player_id`, `method_index`, `level_name`, `dll_version` and `fingerprint_usable` when the event's `fields` has them. The attribute table is in [observability.md](observability.md#log-indexes-and-parity-with-the-log-files).

This replaces the old `service_name = "cimmeria-client"` event field, which was a stand-in for a real resource and left the rows inside `cimmeria-server`.

**Production gap.** The normal **Launch SGW.exe** path injects only `cimmeria-client-patches`; the telemetry DLL is injected by the lab supervisor and by the unexposed `LaunchSgwWithClientTelemetry` worker command, and is not packaged with launcher releases. Until an owner decision puts it in front of players, `cimmeria-client` rows from players come from the launcher's tailed logs, and DLL rows come from lab sessions.

## Gameplay seams: what the client accepted and what its UI complained about

Two events added on 2026-09-28 give an agent (or a person reading SigNoz) the client's side of a play session. Neither has been seen from the live client yet; the anchors and string layouts were checked against the QA binary only.

| Target | Hook | Fields | Level | Volume control |
|---|---|---|---|---|
| `client.cme.event` | Inline, CME event-registry create `0x00a5c0f0` | `event` (class name, e.g. `Event_NetIn_onDialogDisplay`), `kind` (`net_in`, `net_out`, `net`, `action`, `ui`, `slash_cmd`, `cache`, `other`), `suppressed`, `truncated` | `info` for `net_in`, `debug` otherwise | Per-name token bucket: burst 8, then 4 per second; the next emitted event of that name carries the dropped count in `suppressed` |
| `client.ui.cegui_log` | Vtable slot, `CEGUI::DefaultLogger::logEvent` | `level` (0-4), `level_name` (`errors` … `insane`), `message` (up to 512 characters), `suppressed`, `truncated` | `error` for Errors, `warn` for Warnings, `debug` otherwise | Errors and warnings are throttled per message text, the other levels per level |

`client.cme.event` with `kind = net_in` is the positive half of the dispatch oracle: every inbound entity method the client routed shows up by name, and `client.dispatch.method_dropped` reports the ones it discarded. A server method that the server logged as sent and that appears in neither stream was lost below the dispatcher. The same hook also names input actions (`Event_Action_*`) and connection events, which the throttle keeps from drowning the rest.

The CEGUI message is read as an MSVC `std::wstring`, the type this client's CEGUI `String` is, with the bounded reader in `crates/client-telemetry/src/msvc_string.rs`; the detour calls nothing on it. CEGUI exceptions, including the `ScriptException`s the tolua glue throws for a failed Lua binding call, are logged through this logger, so UI-script failures that never reach a `lua_pcall` caller should surface here.

Under the `lab-bridge` feature both events are also pushed to the bridge's local ring, as kinds `cme.event` and `cegui.log`, so `client_events_read` returns them without a SigNoz round trip.

## Engine layer: log sinks and subsystem seams

The client already logs through five paths and swallows failures at a dozen seams, and none of it reached SigNoz. `src/hooks/sinks/` hooks the five logging paths; `src/hooks/seams/` hooks the seams where a failure is silent. The game layer (CME events, Mercury, entities, Lua, the UI) is separate: the tables above and `client.cme.event`. Recovered addresses, layouts and evidence: [`client-engine-sinks-and-seams.md`](../reverse-engineering/findings/client-engine-sinks-and-seams.md). **None of these has been seen from a live client yet**; the second table says what each one needs to be confirmed.

### Log sinks

| Target | Hook | Fields | Level | Volume control |
|---|---|---|---|---|
| `client.bw.message` | inline, `DebugMsgHelper::message` `0x00a36460` | `message`, `priority`, `priority_name` (`TRACE`..`HACK`), `component_priority`, `fmt_addr`, `filtered` (the client's own threshold would have dropped it), `suppressed`, `truncated` | by priority: `debug` (trace, debug), `info`, `warn` (warning, hack), `error` | per (format string address, priority) |
| `client.ue3.log` | inline, `FOutputDeviceRedirector::Serialize` `0x004ce0b0` (`GLog`) | `category` (the `FName`), `event`, `message`, `suppressed_category`, `suppressed`, `truncated` | by category (`Error` `error`, `*Warning` `warn`, `Dev*` `debug`, else `info`) | per category; warnings per message shape. Low volume: `debugf`/`warnf` are compiled out of this build |
| `client.ue3.fatal_error` | inline, `FOutputDeviceWindowsError::Serialize` `0x004ce3a0` (`GError`) | `category`, `event`, `message`, `fatal` | `error` | none; also written synchronously to the local log, because the process ends |
| `client.ue3.assert` | inline, the `check()` reporter `0x00486000` | `expr`, `file`, `line` | `error` | per source line. A failed `check` is reported and survived |
| `client.log4cxx.event` | IAT `Logger::forcedLog` (narrow `0x017f0160`, wide `0x017f0188`) | `logger`, `level`, `level_int`, `message`, `file`, `line`, `method`, `wide` | by level | per (logger, level, message shape): the lock trace collapses to one bucket |
| `client.os.debug_string` | IAT `OutputDebugStringA` `0x017ef32c` / `W` `0x017ef230` | `message`, `api` | `info` | per message shape; skips what a known sink already reported |
| `client.os.exception` | vectored exception handler | `code`, `code_name`, `address`, `module`, `rva`, `access`, `target`, `noncontinuable`, `thread_id` | `error` | per (code, site). Not C++ throws, not debugger plumbing, never stack overflow |
| `client.hooks.capabilities` | once, after every install | `hook.<name>` = `installed` / `failed: <why>` / `skipped: <why>`, `installed`, `attempted`, `capture.unfilter`, `capture.firehose` | `warn` if any hook failed, else `info` | once |

### Subsystem seams

| Target | Hook | Fields | Level | Needs live confirmation |
|---|---|---|---|---|
| `client.engine.spawn_actor` | inline, `UWorld::SpawnActor` `0x00876970` | `class`, `actor`, `location`, `ok`, `no_collision_fail`, `no_fail` | `debug`; `warn` when it returns `NULL` | class names resolve through the recovered `UObject` layout |
| `client.engine.destroy_actor` | inline, `UWorld::DestroyActor` `0x00875290` | `class`, `actor`, `destroyed`, `net_force` | `debug` | same |
| `client.engine.matinee` | vtable `USeqAct_Interp` slots 85, 86 | `event` (`activated`/`deactivated`), `sequence`, `inputs`, `position`, `length`, `cut_short` | `info` | that slot 86 is `DeActivated`; the offsets |
| `client.engine.level_visible`, `client.engine.level_stream_slow` | inside the `UpdateLevelStreamingInner` hook | `package`, `elapsed_ms`, `visible` | `info`, `warn` (a step over 30 ms; 10 ms under `firehose`) | the `bIsVisible` bit and the package walk |
| `client.engine.load_failed` | inside the `StaticLoadObject` hook | `name`, `filename`, `flags` | `warn` | how often optional lookups fail this way |
| `client.media.bink_open`, `client.media.bink_close` | IAT `_BinkOpen@8` `0x017effa4`, `_BinkClose@4` `0x017effa8` | `ok`, `flags`, `width`, `height`, `frames`, `fps`, `plausible`; `frame_num`, `completed` | `info`; `warn` when the open returns `NULL` | the Bink header layout (SDK layout, not this build's) |
| `client.audio.event` | IAT `Event::start` `0x017f00a4` / `stop` `0x017f0080` | `action`, `name`, `result`, `immediate` | `debug`; `warn` on a non-zero `FMOD_RESULT` | the `getInfo` name lookup |
| `client.physx.error`, `client.physx.assert` | vtable `FNxOutputStream` slots 0, 1 (`0x01839d94`) | `code`, `code_name`, `message`, `file`, `line` | by code; `error` for asserts | that the SDK reports through this stream |
| `client.io.open_failed` | IAT `CreateFileW` `0x017ef2a8` / `A` `0x017ef2a4` | `path`, `error`, `error_name`, `write`, `disposition` | `debug` for not-found, else `warn` | volume of not-found probes |
| `client.gfx.device_created`, `client.gfx.device_reset`, `client.gfx.device_state` | IAT `Direct3DCreate9` `0x017effd8`, then the COM vtables | `hresult`, `hresult_name`, the requested mode, `elapsed_ms`, `lost`, `needs_reset` | `info`; `warn` on a failure | whether the hook lands before the game creates its device |
| `client.engine.hitch` | inside the `FEngineLoop::Tick` hook | `gap_ms`, `frame`, `working_set_mb` | `warn` (a tick-to-tick gap of 200 ms; 100 ms under `firehose`) | the threshold against real frame times |
| `client.engine.memory` | inside the `FEngineLoop::Tick` hook | `working_set_mb`, `peak_working_set_mb`, `private_mb`, `avail_virtual_mb`, `total_virtual_mb`, `machine_load_percent` | `info`; `warn` under 256 MB of free address space | every 30 s |

### Capture switches

Off by default. Read once at boot, from the optional top-level `capture` block of `current-session.json` (written by the launcher or the lab supervisor) and from the `CIMMERIA_CLIENT_CAPTURE` environment variable (a comma-separated list); either can turn a switch on.

| Switch | Effect |
|---|---|
| `unfilter` | lifts the client's own thresholds so they and its own outputs see more: the BigWorld filter threshold (`impl[0x3c]`) is set very low, the four log4cxx `is*Enabled` checks answer `true`, the UE3 suppress flag (`0x1000`) is cleared on the categories that are logged. It changes what the client itself writes to `SGWDebugLog.log` and `OutputDebugString`: a lab and debug switch. |
| `firehose` | raises every sink's rate limit from burst 8 / 4 per second to burst 64 / 64 per second, and lowers the hitch and slow-step thresholds. The limit still exists. |

```json
{ "install_id": "...", "telemetry": { "...": "..." }, "capture": { "unfilter": true, "firehose": false } }
```

### Budget

Every sink and seam limits per distinct message with the shared token bucket ([`name_throttle.rs`](../../crates/client-telemetry/src/hooks/name_throttle.rs)): burst 8, then 4 a second, the swallowed count riding the next event that gets through as `suppressed`. The bucket key is chosen per stream so a hot message cannot hide a rare one (the format string's address for BigWorld; the message with digits collapsed for log4cxx and debug strings, so the client's 65 000-line lock trace is one bucket; the class name for actors; the path for file failures). Hooks on a hot path (the tick, streaming, spawn) do a clock read, a compare and at most one bucket lookup when nothing is reportable.

## The fingerprint gate, the local log and the hook ABI

The DLL had never run inside the client until 2026-09-27: the launcher's
telemetry launch path was dead code. An audit of every anchor against the
QA `SGW.exe` then found that five of them would have corrupted or crashed
the game:

- The seven IAT addresses were the slots' on-disk contents (hint/name
  RVAs) read as addresses. They pointed into UTF-16 strings in `.rdata`.
- `Mercury::Nub::handleMessage` pointed at its own log string, and the
  cooked-data PAK load at the middle of a function. Both hooks are
  removed until #989 re-resolves them.
- The CEGUI logger slot was the destructor (the vtable starts one slot
  after the RTTI locator pointer), whose `ret 4` does not match the
  detour's two arguments.
- `execConsoleCommand` and `onStateFieldUpdate` both pop two stack
  arguments (`ret 8`) where the detours declared one.

Three rules now keep that from recurring:

- **Fingerprint gate** (`src/fingerprint.rs`). Before any hook goes in,
  the bootstrap compares the first bytes of every function it hooks or
  calls, and the value in every vtable slot it swaps, with the QA build.
  On any mismatch it installs no hooks at all, and reports why. A
  different build, or a process that is not `SGW.exe`, loses the hook
  telemetry and nothing else. `FEngineLoop::Tick` and the drop callee may
  already carry the client-patches DLL's MinHook jump, which is chained
  onto; the rule and the install lock both DLLs take are in
  `cimmeria-client-hookgate` and [client-patches.md](client-patches.md).
  Each IAT slot is also checked on its own before it is swapped: it must
  hold exactly the address its import resolves to.
- **Local log.** The DLL writes `cimmeria-client-telemetry.log` next to
  `SGW.exe` (and to `OutputDebugString`): the version, the session file,
  every site's fingerprint result, each hook's install outcome and the
  lab bridge's listener. Nothing about it depends on the upload working.
  The fingerprint result also goes to SigNoz as `client.hooks.fingerprint`
  (`usable`, plus one `site.<name>` field per site).
- **Hook ABI.** Every detour of an original that can throw (engine, Lua,
  CME), and the function-pointer type it calls the original through, uses
  the matching `-unwind` ABI: `thiscall-unwind`, `C-unwind`. UE3 raises
  errors as C++ exceptions, and so does the client's `lua51.dll` (it
  imports `_CxxThrowException`), so a Lua error inside a `lua_call` made
  under an outer `lua_pcall` unwinds through the `lua_call` detour. With a
  plain ABI that unwind aborts the process at the detour (#915). The
  Win32 IAT detours use `stdcall-unwind` too, for one rule instead of a
  list of exceptions. Rust code inside a detour stays inside
  `catch_unwind`, so a Rust panic cannot unwind into the game. The CME
  callbacks stay plain `thiscall`: they call nothing in the game.

## What we DON'T hook

To preserve "observe without changing behavior":

- **`FMalloc::Malloc`** (`FMallocCME` at RTTI `0x01d8f87c`) — millions of calls/sec.
- **`UObject::ConditionalDestroy`** and GC-adjacent paths — use-after-free risk.
- **`BeginScene` / `EndScene`** and render-state-affecting D3D9 entry points.
- **PhysX inner-loop callbacks** — high-frequency on dedicated threads.
- **`wxWidgets`** (statically linked, Atrea editor framework) — irrelevant for the game client.

## Phasing

| Phase | Scope | Status |
|---|---|---|
| 0 | RE prep — anchor table, Unsubscribe decompile, ABI doc | LANDED |
| 1 | Foundation — crate skeleton, DllMain bootstrap, injector + launch wiring, `ClientNative` variant on both sides, CI | LANDED |
| 2a | Event queue (crossbeam-channel MPMC) + in-DLL uploader (ureq+rustls, gzipped NDJSON) + session.json loader + `client.dll.attached` first event | LANDED (PR #504) |
| 2b/c/d | Tier-1 hooks — both CME EventSignal subscribes (`Event_NetIn_onClientReady`, `Event_NetIn_onClientMapLoad`) + `Mercury::Nub::handleMessage` inline hook via MinHook. Sampling counter for hot-path hooks. Phase-status update. | LANDED; both CME subscribes REMOVED 2026-09-28 (the subscribe API was misidentified) and replaced by `client.cme.event`; `handleMessage` removed 2026-09-27 (#989) |
| 2-deferred | The remaining 4 tier-1 inline hooks (`FEngineLoop::Tick`, `UWorld::UpdateLevelStreaming`, `FArchiveAsync::Read*`, `LoadPackage`) — resolved + landed during the upfront Ghidra pass on 2026-06-04. | LANDED |
| 3 | Game state + kismet + tick — state-flag dispatcher, anim notify A+B, cooked-data PAK load, APlayerController::execConsoleCommand (inline) + AActor::Tick, USequence::UpdateOp (vtable swap). **CME RTTI auto-discovery** (~270 events) and **UObject::ProcessEvent** vtable-swap deferred. | LANDED (manifest-driven 2026-06-05) — partial |
| 4 | UI / Lua — CEGUI::DefaultLogger::logEvent (vtable swap) + lua_pcall, lua_call, lua_newstate (IAT swap). Console command already covered in Phase 3. | LANDED (manifest-driven 2026-06-05) |
| 5 | Subsystem correlators — Bink tick (inline, in Phase-3 commit) + CreateThread, LoadLibraryW/A, GetForegroundWindow (IAT swap). **FMOD** runtime vtable traversal and **PropertyNode<T>** per-T enumeration deferred. | LANDED (manifest-driven 2026-06-05) — partial |
| 6 | Crash + on-disk artifact shipping (SetUnhandledExceptionFilter IAT, MiniDumpWriteDump call, log/dump file tailers). IAT slots known; not yet implemented. | DEFERRED |
| 7 | Engine capture: the log sinks (`hooks/sinks/`) and the subsystem seams (`hooks/seams/`), the `capture` switches and the `client.hooks.capabilities` event. Adds 6 inline sites (17 total) and 4 vtable slots (7 total) to the fingerprint gate. | LANDED 2026-09-28 (static evidence; live confirmation pending) |

## CI

- `.github/workflows/client-telemetry-build.yml` — Windows-native CI for `i686-pc-windows-msvc`. fmt + clippy `-D warnings` + build (asserts `.dll` artifact lands) + nextest. Clippy and nextest each run twice, once with `--features lab-bridge`, because the bridge only compiles under that feature (#916).
- `.github/workflows/client-dll-boot.yml` — boots the DLL without the game. `tools/testhost/stage.sh` builds `sgw-testhost.exe` (a 32-bit stand-in for `SGW.exe`), `sgw-start32.exe`, and both telemetry builds for i686. The tests in `crates/sgw-testhost/tests/dll_boot.rs` then inject the DLL through the helper and check four things:
  - the attach line and the fingerprint verdict in the local log;
  - that no hooks went in;
  - a clean host exit;
  - the upload of `client.dll.attached` and `client.hooks.fingerprint` to a mock endpoint, with the session token.

  The lab build must bind the session's loopback port, refuse a wrong token, accept the right one, and answer a request. With no `Tick` hook in the host, that answer is the dispatch timeout.
- Main `.github/workflows/test.yml` excludes the Windows-only cdylib from the Linux workspace check.
- `crates/launcher/`'s existing CI continues to cover the injector module via its own pipeline.

## Cross-references

- [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md) — per-anchor hook table
- [`docs/reverse-engineering/findings/client-instrumentation-entry-points.md`](../reverse-engineering/findings/client-instrumentation-entry-points.md) — **resolved Phase 3-6 entry points** (companion to hookpoints — all addresses + IAT slots + signatures pre-resolved so Phase 3-6 implementation skips the RE round-trip)
- [`docs/reverse-engineering/findings/client-engine-sinks-and-seams.md`](../reverse-engineering/findings/client-engine-sinks-and-seams.md) — the engine-layer log sinks and subsystem seams: addresses, layouts, evidence, what is not yet confirmed
- [`docs/reverse-engineering/findings/cme-event-signal.md`](../reverse-engineering/findings/cme-event-signal.md) — full CME EventSignal emit pipeline
- [`docs/architecture/client-patches.md`](client-patches.md) — the always-injected gameplay-patch DLL. It hooks `FEngineLoop::Tick` and the drop-oracle function (`0x01590f30`) too; both DLLs chain through MinHook, so neither may unhook while the other is loaded
- [`docs/architecture/observability.md`](observability.md) — the broader OTLP / SigNoz pipeline this plugs into
- [`docs/architecture/dev-session-telemetry.md`](dev-session-telemetry.md) — the launcher telemetry pipeline the DLL reuses
- [`docs/operations/telemetry.md`](../operations/telemetry.md) — operator runbook (per-category controls, opt-out, crash-dump shipping — extends with client-side toggle in follow-up)
- [`docs/technical/atrearl-loader.md`](../technical/atrearl-loader.md) — third-party RE reference (behavioural only — not code we use)
- [`docs/architecture/live-research-lab.md`](live-research-lab.md) — the ADR for the `lab-bridge` inbound channel this DLL gains under the feature flag; [`docs/guides/live-research-lab.md`](../guides/live-research-lab.md) is its operating manual / rulebook
