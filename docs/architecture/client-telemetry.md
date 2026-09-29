# Client-Side Telemetry — Architecture

> **Diátaxis type**: explanation
> **Audience**: engineers extending or reviewing the `cimmeria-client-telemetry` DLL and its launcher-side injector (issue #417)
> **Last updated**: 2026-09-29
> **Engine capture (2026-09-28)**: adds the engine layer on top of the hooks below: log sinks for BigWorld, UE3, log4cxx and the OS, plus subsystem seams (actors, Matinee, level streaming, Bink, FMOD, PhysX, file I/O, D3D9, frame health): 6 inline, 4 vtable and 15 IAT hooks, a vectored exception handler and 3 D3D9 COM patches. See [Engine layer: log sinks and subsystem seams](#engine-layer-log-sinks-and-subsystem-seams). Statically verified, not yet run in a live client.
> **Status**: Phases 2-5 substantially landed, **none of it yet run inside the real client** (until 2026-09-29 the launcher never injected the DLL; it now does for players who opt in to telemetry, see [Who gets the DLL](#who-gets-the-dll) and the fingerprint section). **32 hooks total** across 3 techniques: 22 inline JMP hooks (4 engine + state-flag dispatcher, anim notify A+B, console command, Bink tick, entity-method drop oracle, CME event-registry lookup, and 11 entity-lifecycle, inbound-message and outgoing-RPC hooks added 2026-09-28), 7 IAT-swap hooks (3 Lua + 4 OS), 3 vtable-swap hooks (CEGUI logger, now with the line text + AActor::Tick + USequence::UpdateOp). The two CME subscribers (`onClientMapLoad`, `onClientReady`) were removed on 2026-09-28: the subscribe API they called is not one (see Hook taxonomy item 1). Every address was re-checked against the QA `SGW.exe` on 2026-09-27 and is fingerprinted. **Removed pending re-resolution (#989)**: `Mercury::Nub::handleMessage` and the cooked-data PAK load, whose anchors were not function entries. **Deferred**: CME RTTI auto-discovery (~270 more events; needs `.rdata` scanner), FMOD runtime vtable traversal, ProcessEvent slot search, PropertyNode<T> per-T enumeration, Phase 6 crash filter.

How `cimmeria-client-telemetry.dll` is side-loaded into `SGW.exe` by `sgw-launcher`, what it observes, and how those observations flow into SigNoz alongside the server-side OTLP stream.

The per-anchor hook table lives in [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md). This document is the design rationale and the stack-pick justification — read the anchor doc for "where do I hook function X?" and this doc for "why does the whole thing look like this?"

## Goal

Give server-side debuggers a client-side view of every meaningful event happening inside `SGW.exe` — frame ticks, level streaming transitions, async I/O completions, CME EventSignal dispatches, log lines, crashes — without modifying game behavior. Output lands in SigNoz as its own service, `service.name = cimmeria-client`, so an end-to-end SigNoz trace can include both server and client spans for the same session.

The motivating use case: the cold-relog freeze investigation (2026-05-26). Server-side we can see Mercury sending N packets and getting N-3 ACKs back; what we can't see is whether the client's render thread is stuck on a disk read or which `.upk` is loading when frame ticks stop. Tier-1 hooks answer that.

## Trust model

Same machine, same user, same launcher session. The launcher is already a trusted desktop app the developer installed — DLL injection doesn't escalate privileges. The DLL ships alongside `sgw-launcher.exe` (signed at deployment time; not per-injection). No remote-code-execution surface: the DLL itself only reads from SGW.exe's memory; outbound network traffic is HTTPS to the launcher's HMAC-protected `/api/telemetry/upload-chunk` endpoint.

**Project preference (issue #417):** authored from scratch. AteraLoader.exe / AtreaRL.dll are reverse-engineering references only — their behaviour in [`docs/technical/atrearl-loader.md`](../technical/atrearl-loader.md) is useful for "what hooks work in practice," not as code we extend or wrap.

**No longer strictly emit-only under `--features lab-bridge`.** The base DLL is emit-only: it reads SGW.exe memory and ships observations out; it takes no inbound commands. The **Live Research Lab** ([`live-research-lab.md`](live-research-lab.md)) adds an inbound command channel behind the off-by-default `lab-bridge` cargo feature — Lua eval, memory read/write, non-freezing hook install, and native calls on the client main thread. Activation is double-gated: the code exists only in a DLL built with that feature, and even then starts only when `current-session.json` carries a `lab` block that only the lab supervisor writes. A telemetry DLL handed to anyone else physically lacks the bridge, so the trust model above holds unchanged for every non-lab build. The lab's rulebook and the operating manual live in [`../guides/live-research-lab.md`](../guides/live-research-lab.md).

## Who gets the DLL

Owner decision, 2026-09-29: players get the DLL, but only when they opt in to telemetry in the launcher.

- **Opt-in on.** **Launch SGW.exe** starts the telemetry session first: the dev-session handshake (at most 10 s) mints a player token and writes `current-session.json` next to `SGW.exe`, because the DLL reads its token and upload endpoint from that file as it boots. The game then starts through `sgw-start32` with the client-patches DLL and, after it, the telemetry DLL, the same order as the lab's `launch_request`. Both upload with the same player token.
- **Opt-in off.** The launch is the non-telemetry launch: client patches only, no session, no telemetry DLL. The checkbox takes effect from the next launch.
- **Never blocks play.** No session (server down, refused, timed out): the game starts without the DLL. DLL missing or refused: the game starts without it, and the status log and the launcher's `client.telemetry_dll.launch` event say why. A failed two-DLL injection is retried with the client patches alone, then plainly.

**Only the player build ships.** The release build compiles the DLL with default features (`tools/launcher-release/build.sh i686`), never `lab-bridge`, and embeds it in the launcher, which writes it to `<launcher dir>/client-telemetry/<sha256 prefix>/` at launch (a dev launcher uses a `cimmeria-client-telemetry.dll` beside itself). A `lab-bridge` build logs `cimmeria_client_telemetry::LAB_BRIDGE_MARKER` at boot, so the text is in its image; three checks refuse an image containing it: the launcher's `build.rs` will not embed it, the release `verify` stage fails, and the launcher will not inject one it finds at launch. The `sgw-testhost` boot tests pin the check against real builds (the lab build carries the marker, the default build does not and opens no port even with a `lab` block in its session). Every `client.dll.attached` event carries `dll_flavor` (`player` or `lab-bridge`).

**Known limit.** The DLL reads the session once, at boot, and never refreshes its token. Dev-session tokens live 8 hours, so a session longer than that stops uploading DLL events (the launcher's own uploads refresh and continue).

## Architecture

```text
sgw-launcher (egui, 64-bit) -- Launch SGW.exe, telemetry opted in
   1. POST /auth/dev-session (no session_kind -> player token)
      write <Binaries>/sessions/current-session.json
   2. sgw-start32.exe (i686 helper, embedded in the launcher):
     - spawn SGW.exe suspended
     - inject cimmeria-client-patches.dll, then cimmeria-client-telemetry.dll
     - resume
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

Since 2026-09-29 the server runs a fourth OTLP log provider whose resource is `service.name = cimmeria-client` with `cimmeria.source = client` (`otel::client_resource`), and the log routing sends the ingest's `client.native` replays there and nowhere else (`otel::is_client_target`). Query `service.name = 'cimmeria-client'`; the DLL's event name is the `client_target` attribute and the log body. Each row also carries the session's `session_id`, `install_id`, `cimmeria.session_kind` (`lab` or `player`) and `lab`, plus `account_id`, `player_id`, `method_index`, `level_name`, `dll_version`, `fingerprint_usable`, and on a governor rollup `rollup_target` and `rollup_count`, when the event's `fields` has them. The attribute table is in [observability.md](observability.md#log-indexes-and-parity-with-the-log-files).

This replaces the old `service_name = "cimmeria-client"` event field, which was a stand-in for a real resource and left the rows inside `cimmeria-server`.

Player DLL rows carry `cimmeria.session_kind = player`, lab rows `lab`; see [Who gets the DLL](#who-gets-the-dll).

## Volume control: the governor

Added 2026-09-29. One lab client produced about 333,000 `client.native` rows an hour, and 86% of them were `client.engine.sequence_tick`. The hooks' own samplers (1 in 10, 1 in 1000) were not enough, and a burst of a hot stream could fill the 4096-slot upload ring and push out an entity-lifecycle event. The governor (`crates/client-telemetry/src/governor/`) sits between the hooks and that ring. `queue::governed_channel` builds it, and every `Producer::try_emit` passes through `Governor::admit` before it takes a slot.

**Classification is one table.** `governor/classify.rs` decides a class for every event in three steps, and the first step that applies wins:

1. Level `warn` or `error`: **must-keep**.
2. Fields that report a failure (`ok: false`, `success: false`, `failed: true`, a non-null `error`, or `outcome`/`result` in `NON_HAPPY_OUTCOMES`): **must-keep**.
3. The first matching row of `RULES`. If no row matches, the event is **budgeted**.

| Class | Targets (rows in `RULES`) | What happens |
|---|---|---|
| Must-keep | `client.entity.*` (create, enter, entered_world, leave, destroyed, appearance_request, queue_replay); `client.mercury.error`, `.fragment*`, `.bundle*`, `client.dispatch.method_dropped`; `client.hooks.*`; `client.dll.*`, `client.session.*`, `client.cme.catalog*`; `client.lua.error`, `client.os.exception`, `client.ue3.assert*`/`fatal*`, `client.physx.error*`/`assert*`, `client.io.open_failed`, `client.engine.load_failed`/`hitch`/`level_stream_slow`; `client.telemetry.*` | Forwarded untouched: never throttled, collapsed or summarized |
| Per-entity | `client.cme.event` (key `name`), `client.mercury.entity_method` and `entity_property` (key `msg_id`), `client.net.out` (key `method`) | The first 16 per (target, `entity_id`, key) are forwarded, and the rest go into the rollup. The table resets at every scene change, so each world entry gets a fresh 16. An event without `entity_id` is budgeted |
| Hot | `client.engine.sequence_tick`, `actor_tick`, `tick`, `bink_tick`, `async_archive_serialize`, `static_load_object` (key `package_name`), `update_level_streaming`; `client.frame_tick`; `client.lua.pcall`, `client.lua.call`; `client.os.get_foreground_window` | Never forwarded one by one; always summarized |
| Budgeted | everything else (`client.ui.cegui_log`, `client.mercury.packet_in`, `client.streaming.update`, ...) | Forwarded while the target is under its budget (burst 20, then 2 per second); the rest go into the rollup |

Per-entity and budgeted events also **collapse**. An event identical to the previous event *on the same target* (same level, same fields) is held as a repeat. When a different event arrives on that target, or when the window closes, one repeat event goes out. It has the same target, level and fields, plus `repeat_count` (the repeats held, not counting the first event, which was forwarded), `repeat_first_ts_ms` and `repeat_last_ts_ms`. Must-keep events are never collapsed.

**Rollups.** Every 10 seconds, at every scene change (a `client.streaming.update` event whose `level_name` differs from the last one seen; `client.ui.cegui_log` reuses the field name for its log severity, so only the targets in `governor::SCENE_TARGETS` count), and at shutdown, each target with absorbed events emits one `client.telemetry.rollup` event per reason it was absorbed for (`client.cme.event` with and without an `entity_id` gives a `per_entity_overflow` and an `over_budget` rollup):

| Field | Meaning |
|---|---|
| `rollup_target` | The target summarized |
| `reason` | `hot_stream`, `over_budget` or `per_entity_overflow` |
| `trigger` | `window`, `scene_change` or `shutdown` |
| `count`, `rate_per_sec` | Exact count and count per second over the window |
| `window_start_ms`, `window_end_ms`, `window_ms`, `first_ts_ms`, `last_ts_ms` | When |
| `key_field`, `distinct_keys`, `distinct_keys_capped`, `top_keys`, `other_key_events`, `last_key` | Exact top 10 values of the key field, and the keyed events outside the top 10. Up to 256 distinct keys are tracked; `distinct_keys_capped` says there were more |
| `numeric` | `{field: {n, min, max, sum}}` for up to 16 numeric fields |
| `last_fields` | The last absorbed event's fields, as an exemplar (for a load freeze: what was loading last) |

**Nothing is dropped silently.** For every target, the rows forwarded plus the `repeat_count` of its repeat events plus the `count` of its rollups add up to the events the hooks raised. The volume test checks this for every target. Every 60 seconds, and at shutdown, `client.telemetry.health` reports the governor's totals (`seen_total`, `forwarded_total`, `must_keep_total`, `rolled_up_total`, `collapsed_total`, ...), `ring_dropped_total` (a full ring) and `upload_dropped_total` (batches discarded after failed POSTs). Both counts existed before but were never reported. If either has grown since the last report, the health event is `warn`. `uncollapsed_events_total` counts events that arrived while the collapse table was full and so were not checked for repeats.

**Shutdown flush: not wired yet.** The shutdown rollups and health event run only when the uploader's stop callback returns `true`. The DLL passes `|| false` today (`boot.rs`, pending the Phase 7 stop flag), so when `SGW.exe` exits, the last window's rollups and held repeats are lost: up to 10 seconds of absorbed events. The conservation above holds for every closed window, not for the tail of a session.

**Measured effect.** Replayed through the governor on the uploader's 2 s cadence, the hour measured above (333,263 events) comes out as 4,134 rows, a 98.8% cut. All 855 must-keep events come through unchanged (`governor/tests/volume.rs`).

**What stays local, and full volume on purpose.** The lab bridge's ring is fed by `hooks::emit` before the producer, so `client_events_read` still sees the stream as the hooks raised it. The governor reads two capture switches from the same places as the engine-sink switches: the `capture` block of `current-session.json` and `CIMMERIA_CLIENT_CAPTURE`. Either source can turn a switch on.

- `raw` forwards everything. Nothing is collapsed or summarized, but health events are still sent. Use it for a lab session that needs full volume on purpose.
- `firehose` keeps the governor on with 16 times the budget and 4 times the per-entity K. Hot streams are still summarized.

The local log records the mode at boot (`telemetry governor: governed`).

**Server-side guard.** The ingest keeps per-session counters and a budget of 30,000 events per session per minute (`admin-api` `routes/telemetry/session_budget.rs`). A governed client sends fewer than 100 events a minute. Over the budget, only warn/error rows and the must-keep families above are replayed. The rest are counted, and every chunk that suppressed something logs a `launcher.ingest` warn with `reason = session_over_budget` and the session's totals. See [telemetry.md](../operations/telemetry.md#volume-control-and-the-runaway-guard).

## Volume control: the governor

Added 2026-09-29. One lab client produced about 333,000 `client.native` rows an hour, and 86% of them were `client.engine.sequence_tick`. The hooks' own samplers (1 in 10, 1 in 1000) were not enough, and a burst of a hot stream could fill the 4096-slot upload ring and push out an entity-lifecycle event. The governor (`crates/client-telemetry/src/governor/`) sits between the hooks and that ring. `queue::governed_channel` builds it, and every `Producer::try_emit` passes through `Governor::admit` before it takes a slot.

**Classification is one table.** `governor/classify.rs` decides a class for every event in three steps, and the first step that applies wins:

1. Level `warn` or `error`: **must-keep**.
2. Fields that report a failure (`ok: false`, `success: false`, `failed: true`, a non-null `error`, or `outcome`/`result` in `NON_HAPPY_OUTCOMES`): **must-keep**.
3. The first matching row of `RULES`. If no row matches, the event is **budgeted**.

| Class | Targets (rows in `RULES`) | What happens |
|---|---|---|
| Must-keep | `client.entity.*` (create, enter, entered_world, leave, destroyed, appearance_request, queue_replay); `client.mercury.error`, `.fragment*`, `.bundle*`, `client.dispatch.method_dropped`; `client.hooks.*`; `client.dll.*`, `client.session.*`, `client.cme.catalog*`; `client.lua.error`, `client.os.exception`, `client.ue3.assert*`/`fatal*`, `client.physx.error*`/`assert*`, `client.io.open_failed`, `client.engine.load_failed`/`hitch`/`level_stream_slow`; `client.telemetry.*` | Forwarded untouched: never throttled, collapsed or summarized |
| Per-entity | `client.cme.event` (key `name`), `client.mercury.entity_method` and `entity_property` (key `msg_id`), `client.net.out` (key `method`) | The first 16 per (target, `entity_id`, key) are forwarded, and the rest go into the rollup. The table resets at every scene change, so each world entry gets a fresh 16. An event without `entity_id` is budgeted |
| Hot | `client.engine.sequence_tick`, `actor_tick`, `tick`, `bink_tick`, `async_archive_serialize`, `static_load_object` (key `package_name`), `update_level_streaming`; `client.frame_tick`; `client.lua.pcall`, `client.lua.call`; `client.os.get_foreground_window` | Never forwarded one by one; always summarized |
| Budgeted | everything else (`client.ui.cegui_log`, `client.mercury.packet_in`, `client.streaming.update`, ...) | Forwarded while the target is under its budget (burst 20, then 2 per second); the rest go into the rollup |

Per-entity and budgeted events also **collapse**. An event identical to the previous event *on the same target* (same level, same fields) is held as a repeat. When a different event arrives on that target, or when the window closes, one repeat event goes out. It has the same target, level and fields, plus `repeat_count` (the repeats held, not counting the first event, which was forwarded), `repeat_first_ts_ms` and `repeat_last_ts_ms`. Must-keep events are never collapsed.

**Rollups.** Every 10 seconds, at every scene change (a `client.streaming.update` event whose `level_name` differs from the last one seen; `client.ui.cegui_log` reuses the field name for its log severity, so only the targets in `governor::SCENE_TARGETS` count), and at shutdown, each target with absorbed events emits one `client.telemetry.rollup` event per reason it was absorbed for (`client.cme.event` with and without an `entity_id` gives a `per_entity_overflow` and an `over_budget` rollup):

| Field | Meaning |
|---|---|
| `rollup_target` | The target summarized |
| `reason` | `hot_stream`, `over_budget` or `per_entity_overflow` |
| `trigger` | `window`, `scene_change` or `shutdown` |
| `count`, `rate_per_sec` | Exact count and count per second over the window |
| `window_start_ms`, `window_end_ms`, `window_ms`, `first_ts_ms`, `last_ts_ms` | When |
| `key_field`, `distinct_keys`, `distinct_keys_capped`, `top_keys`, `other_key_events`, `last_key` | Exact top 10 values of the key field, and the keyed events outside the top 10. Up to 256 distinct keys are tracked; `distinct_keys_capped` says there were more |
| `numeric` | `{field: {n, min, max, sum}}` for up to 16 numeric fields |
| `last_fields` | The last absorbed event's fields, as an exemplar (for a load freeze: what was loading last) |

**Nothing is dropped silently.** For every target, the rows forwarded plus the `repeat_count` of its repeat events plus the `count` of its rollups add up to the events the hooks raised. The volume test checks this for every target. Every 60 seconds, and at shutdown, `client.telemetry.health` reports the governor's totals (`seen_total`, `forwarded_total`, `must_keep_total`, `rolled_up_total`, `collapsed_total`, ...), `ring_dropped_total` (a full ring) and `upload_dropped_total` (batches discarded after failed POSTs). Both counts existed before but were never reported. If either has grown since the last report, the health event is `warn`. `uncollapsed_events_total` counts events that arrived while the collapse table was full and so were not checked for repeats.

**Shutdown flush: not wired yet.** The shutdown rollups and health event run only when the uploader's stop callback returns `true`. The DLL passes `|| false` today (`boot.rs`, pending the Phase 7 stop flag), so when `SGW.exe` exits, the last window's rollups and held repeats are lost: up to 10 seconds of absorbed events. The conservation above holds for every closed window, not for the tail of a session.

**Measured effect.** Replayed through the governor on the uploader's 2 s cadence, the hour measured above (333,263 events) comes out as 4,134 rows, a 98.8% cut. All 855 must-keep events come through unchanged (`governor/tests/volume.rs`).

**What stays local, and full volume on purpose.** The lab bridge's ring is fed by `hooks::emit` before the producer, so `client_events_read` still sees the stream as the hooks raised it. The governor reads two capture switches from the same places as the engine-sink switches: the `capture` block of `current-session.json` and `CIMMERIA_CLIENT_CAPTURE`. Either source can turn a switch on.

- `raw` forwards everything. Nothing is collapsed or summarized, but health events are still sent. Use it for a lab session that needs full volume on purpose.
- `firehose` keeps the governor on with 16 times the budget and 4 times the per-entity K. Hot streams are still summarized.

The local log records the mode at boot (`telemetry governor: governed`).

**Server-side guard.** The ingest keeps per-session counters and a budget of 30,000 events per session per minute (`admin-api` `routes/telemetry/session_budget.rs`). A governed client sends fewer than 100 events a minute. Over the budget, only warn/error rows and the must-keep families above are replayed. The rest are counted, and every chunk that suppressed something logs a `launcher.ingest` warn with `reason = session_over_budget` and the session's totals. See [telemetry.md](../operations/telemetry.md#volume-control-and-the-runaway-guard).

## Gameplay seams: what the client accepted and what its UI complained about

Two events added on 2026-09-28 give an agent (or a person reading SigNoz) the client's side of a play session. Neither has been seen from the live client yet; the anchors and string layouts were checked against the QA binary only.

| Target | Hook | Fields | Level | Volume control |
|---|---|---|---|---|
| `client.cme.event` | Inline, CME event-registry create `0x00a5c0f0` | `event` (class name, e.g. `Event_NetIn_onDialogDisplay`), `kind` (`net_in`, `net_out`, `net`, `action`, `ui`, `slash_cmd`, `cache`, `other`), `suppressed`, `truncated` | `info` for `net_in`, `debug` otherwise | Per-name token bucket: burst 8, then 4 per second; the next emitted event of that name carries the dropped count in `suppressed` |
| `client.ui.cegui_log` | Vtable slot, `CEGUI::DefaultLogger::logEvent` | `level` (0-4), `level_name` (`errors` … `insane`), `message` (up to 512 characters), `suppressed`, `truncated` | `error` for Errors, `warn` for Warnings, `debug` otherwise | Errors and warnings are throttled per message text, the other levels per level |

Since 2026-09-28 an `Event_NetIn_*` that the network thread raises while dispatching an inbound method for an entity also carries `entity_id` (plus `type_id` when known and `msg_id`), and is throttled per (event, entity) instead of per name; see the next section.

`client.cme.event` with `kind = net_in` is the positive half of the dispatch oracle: every inbound entity method the client routed shows up by name, and `client.dispatch.method_dropped` reports the ones it discarded. A server method that the server logged as sent and that appears in neither stream was lost below the dispatcher. The same hook also names input actions (`Event_Action_*`) and connection events, which the throttle keeps from drowning the rest.

The CEGUI message is read as an MSVC `std::wstring`, the type this client's CEGUI `String` is, with the bounded reader in `crates/client-telemetry/src/msvc_string.rs`; the detour calls nothing on it. CEGUI exceptions, including the `ScriptException`s the tolua glue throws for a failed Lua binding call, are logged through this logger, so UI-script failures that never reach a `lua_pcall` caller should surface here.

Under the `lab-bridge` feature both events are also pushed to the bridge's local ring, as kinds `cme.event` and `cegui.log`, so `client_events_read` returns them without a SigNoz round trip.

## Entity lifecycle, inbound and outbound messages, Lua errors and the CME catalog

Added 2026-09-28 for the invisible-guard class of bug (#838): the client creates an NPC but does not render it, and the server cannot see why. Every anchor, argument count and layout is in [client-entity-lifecycle.md](../reverse-engineering/findings/client-entity-lifecycle.md); none of it has run in the live client yet. All of it is in the DLL every player gets (no `lab-bridge` needed), and each event is also pushed to the bridge's local ring under `lab-bridge` (kind = target without the `client.` prefix).

**Volume control.** Every per-entity event goes through one throttle keyed by (event, entity id): a token bucket of burst 8 and 4 per second per pair, so an entity that never spoke before always gets through, and the next emitted event of a pair carries `suppressed`. The table restarts (fresh bursts, no merged buckets) after 8192 tracked pairs. Levels: `info` for lifecycle and queued messages (the evidence), `debug` for delivered messages (the firehose). A pipeline can index `level >= info` as the signal stream and `debug` separately.

| Target | Hook (address) | Fields | Level |
|---|---|---|---|
| `client.entity.enter` | `enterAoI` `0x00dd24f0` | `entity_id`, `space_id`, `vehicle_id`, `entered_world`, `place_before`, `place_after` (`world`, `cache`, `pending`, `none`), `enter_count`, `entity_flags`, `pending_enter_count`, `queued_msgs`, `is_local_player`, `state_incomplete` | `info` |
| `client.entity.create` | `onEntityCreate` `0x00dd2270` | as above, plus `type_id`, `payload_len`, and `outcome`: `entered_world`, `parked` (the create left the entity in the cache map), `in_world`, `already_in_world`, `no_entity` | `info` |
| `client.entity.entered_world` | `enterWorld` `0x00dd1d00` | `entity_id`, `type_id`, `space_id`, `vehicle_id`, `via` (`create`, `enter`, `replay`, `other`) | `info` |
| `client.entity.leave` | `leaveAoI` `0x00dd2800` | `entity_id`, `cache_stamp`, `destroyed`, and the state fields | `info` |
| `client.entity.destroyed` | destroy `0x00dd1120` | `entity_id`, `type_id`, `enter_count` | `info` |
| `client.entity.appearance_request` | appearance request `0x00e69150` (+ scheduler `0x00e998e0`) | `entity_id`, `type_id`, `outcome` (`scheduled`, `not_ready`, `held_or_not_ready`, `not_scheduled`), `reason` (`EntityManager::enterWorld`, `GameEntity::setTint`, ...), `hold_byte` | `info` |
| `client.entity.queue_replay` | replay `0x00dd1e40` | `entity_id`, `type_id`, `queued_msgs` (before), `replayed` | `info` |
| `client.mercury.entity_method` | `onEntityMethod` `0x00dd2b80` | `entity_id`, `msg_id`, `path` (`delivered`, `local_player`, `queued`), `type_id`, `len`; for `queued` also `place`, `queued_msgs`, `enter_count` | `debug` delivered, `info` queued |
| `client.mercury.entity_property` | `onEntityProperty` `0x00dd29d0` | same; `path` is `known_entity_ignored` or `queued` (the client ignores the BigWorld property message for a known entity) | `debug` / `info` |
| `client.net.out` | `RouteOutgoingEntityRpc` `0x00c6fc40` | `method` (name), `route` (`base`, `cell`), `msg_id`, `sub_index`, `entity_id`, `to_local_player` | `info`, per (method, entity) |
| `client.lua.error` | IAT `lua_pcall`, non-zero return | `status`, `status_name` (`runtime`, `syntax`, `memory`, `error_handler`), `nargs`, `message` (up to 512 characters, read only if the error value is a string), `truncated`, `suppressed` | `warn`, throttled per message text |
| `client.cme.catalog`, `client.cme.catalog_done` | CME registry walk, once, on the first `client.cme.event` | chunks of 40 `names` (comma-joined, sorted), `chunk`, `count`; then `total`, `chunks` and one `kind_<family>` count per event family | `info` |
| `client.cme.event` (extended) | `0x00a5c0f0` | adds `entity_id`, `type_id`, `msg_id` to an `Event_NetIn_*` created inside an entity dispatch | `info` |
| `client.dispatch.method_dropped` (extended) | `0x01590f30` | adds `entity_id`, `type_id`, `msg_id` when the drop happens inside a tagged dispatch | `warn` |

**How the entity id reaches a CME event.** `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) writes the entity id into the event it creates (`*(event+8) = *(msg+0xC)` after the factory returns), and the factory hook only sees the name. The dispatcher is hooked by the client-patches DLL and stays untouched. It has exactly two callers, `onEntityMethod` and the queue replay, so the DLL hooks those two entry points instead and sets a thread-local dispatch context (entity id, type, message id) around the original; the factory hook reads it. The context is restored by a drop guard, so a C++ exception through the dispatch leaves nothing stale on the network thread.

**Reading a missing entity.** For an NPC that was created but never rendered, query `client.entity.*` by `entity_id`: `create.outcome = parked` with `entered_world = false` and a `client.entity.enter` with `place_before = cache` and `entered_world = false` say the client never made it live; `queued_msgs > 0` and no `queue_replay` say its appearance methods are still waiting; `appearance_request.outcome = scheduled` with no pawn says the fault is downstream of the client's own bookkeeping.

**Safety.** Every read of a game structure goes through `ReadProcessMemory` (`cimmeria_client_hookgate::os::read_bytes`), so a stale pointer reads as missing instead of faulting the network thread; the std::map walker is bounded (96 steps a lookup, 20,000 nodes for the catalog); the CME catalog runs on its own thread after a 2 second delay, so the game thread never waits on it; every detour forwards its arguments and result untouched and keeps telemetry code inside `catch_unwind`. All eleven new inline sites are in the fingerprint gate.

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
| `client.hooks.capabilities` | once, after every install | `hook.<name>` = `installed` / `failed: <why>` / `skipped: <why>`, `installed`, `attempted`, `capture.unfilter`, `capture.firehose`, `capture.raw` | `warn` if any hook failed, else `info` | once |

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
| `firehose` | raises every sink's rate limit from burst 8 / 4 per second to burst 64 / 64 per second, and lowers the hitch and slow-step thresholds. The limit still exists. It also widens the [telemetry governor](#volume-control-the-governor) (16 times the budget, 4 times the per-entity K). |
| `raw` | turns the telemetry governor off for the upload path: every event is forwarded as raised (health events still go out). The sinks' own rate limits still apply. |

```json
{ "install_id": "...", "telemetry": { "...": "..." }, "capture": { "unfilter": true, "firehose": false, "raw": false } }
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
