# Client-Side Telemetry — Architecture

> **Diátaxis type**: explanation
> **Audience**: engineers extending or reviewing the `cimmeria-client-telemetry` DLL and its launcher-side injector (issue #417)
> **Last updated**: 2026-10-04
> **Engine capture (2026-09-28)**: adds the engine layer on top of the hooks below: log sinks for BigWorld, UE3, log4cxx and the OS, plus subsystem seams (actors, Matinee, level streaming, Bink, FMOD, PhysX, file I/O, D3D9, frame health): 6 inline, 4 vtable and 15 IAT hooks, a vectored exception handler and 3 D3D9 COM patches. See [Engine layer: log sinks and subsystem seams](#engine-layer-log-sinks-and-subsystem-seams). Statically verified, not yet run in a live client.
> **Status**: Phases 2-5 substantially landed; the launcher injects the DLL for players who opt in to telemetry (see [Who gets the DLL](#who-gets-the-dll) and the fingerprint section). **68 hooks total** across 3 techniques: 57 inline JMP hooks (4 engine + state-flag dispatcher, anim notify A+B, console command, Bink tick, entity-method drop oracle, CME event-registry lookup, and 11 entity-lifecycle, inbound-message and outgoing-RPC hooks added 2026-09-28, 5 Mercury receive-path hooks added 2026-09-29, 3 SequenceManager drop hooks and 3 `Debug:log` hooks added 2026-09-29, and 24 ability hooks added 2026-10-04, partly seen live on 2026-10-04 (see [Ability telemetry](#ability-telemetry-clientability)): 12 for presses, sends and the sequence join ([AB-C1, AB-C2](#presses-and-sends-ab-c1-ab-c2)) and 12 for `client.ability.applied` and the `onSequence` net-in drop ([AB-C4, AB-C5](#what-the-client-applied-clientabilityapplied-ab-c4))), 8 IAT-swap hooks (3 Lua + 4 OS + raw `recvfrom`), 3 vtable-swap hooks (CEGUI logger, now with the line text + AActor::Tick + USequence::UpdateOp). The two CME subscribers (`onClientMapLoad`, `onClientReady`) were removed on 2026-09-28: the subscribe API they called is not one (see Hook taxonomy item 1). Every inline address is fingerprinted; the `recvfrom` IAT slot is checked against the loaded Winsock export before swapping. **Removed pending re-resolution (#989)**: `Mercury::Nub::handleMessage` and the cooked-data PAK load, whose anchors were not function entries. **Deferred**: CME RTTI auto-discovery (~270 more events; needs `.rdata` scanner), FMOD runtime vtable traversal, ProcessEvent slot search, PropertyNode<T> per-T enumeration, Phase 6 crash filter.

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
| Must-keep | `client.entity.*` (create, enter, entered_world, leave, destroyed, appearance_request, queue_replay); `client.mercury.error`, `.fragment*`, `.bundle*`, `.request_misparse`, `.unpack_fault`, `client.dispatch.method_dropped`; `client.hooks.*`; `client.dll.*`, `client.session.*`, `client.cme.catalog*`; `client.lua.error`, `client.os.exception`, `client.ue3.assert*`/`fatal*`, `client.physx.error*`/`assert*`, `client.io.open_failed`, `client.engine.load_failed`/`hitch`/`level_stream_slow`; `client.telemetry.*`; `client.ability.*` (already throttled per name at the hook, D-AU5) | Forwarded untouched: never throttled, collapsed or summarized |
| Per-entity | `client.cme.event` (key `name`), `client.mercury.entity_method` and `entity_property` (key `msg_id`), `client.net.out` (key `method`), `client.sequence.dropped` (key `path`) | The first 16 per (target, `entity_id`, key) are forwarded, and the rest go into the rollup. The table resets at every scene change, so each world entry gets a fresh 16. An event without `entity_id` is budgeted |
| Hot | `client.engine.sequence_tick`, `actor_tick`, `tick`, `bink_tick`, `async_archive_serialize`, `static_load_object` (key `package_name`), `update_level_streaming`; `client.frame_tick`; `client.lua.pcall`, `client.lua.call`; `client.os.get_foreground_window` | Never forwarded one by one; always summarized |
| Budgeted | everything else (`client.ui.cegui_log`, `client.lua.debug_log` below `warn`, `client.mercury.packet_in`, `client.streaming.update`, ...) | Forwarded while the target is under its budget (burst 20, then 2 per second); the rest go into the rollup |

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
| `client.lua.error` | IAT `lua_pcall`, non-zero return | `status`, `status_name` (`foreign_exception` for -1, `yield`, `runtime`, `syntax`, `memory`, `error_handler`), `nargs`, `message` (up to 512 characters, read only if the error value is a string), `value_type` (Lua type of the top value), `function_source` + `function_line` (-1 only: `short_src` and `linedefined` of the function that was called; only when `lua_getinfo`'s first 24 bytes match the QA `lua51.dll`, whose `lua_State` and `lua_Debug` layouts the reader assumes), `truncated`, `suppressed` | `warn`, throttled per message text, or per status and function when there is no message |
| `client.lua.debug_log` | Inline, the three `ScriptedDebug` tolua bindings behind `Debug:log` / `warn` / `error` (`0x00aa1620`, `0x00aa1710`, `0x00aa1800`) | `channel` (`log`, `warn`, `error`), `source` (`cimmeria_bm` for `[Cimmeria BM]` lines, `cimmeria` for other `[Cimmeria` lines, else `ui`), `text` (up to 512 characters), `truncated`, `suppressed` | `error` / `warn` for those channels; `log` is `info` for `[Cimmeria` lines and `debug` otherwise. Throttled per (channel, message shape with digits collapsed): burst 8, 4 a second |
| `client.cme.catalog`, `client.cme.catalog_done` | CME registry walk, once, on the first `client.cme.event` | chunks of 40 `names` (comma-joined, sorted), `chunk`, `count`; then `total`, `chunks` and one `kind_<family>` count per event family | `info` |
| `client.cme.event` (extended) | `0x00a5c0f0` | adds `entity_id`, `type_id`, `msg_id` to an `Event_NetIn_*` created inside an entity dispatch | `info` |
| `client.dispatch.method_dropped` (extended) | `0x01590f30` | adds `entity_id`, `type_id`, `msg_id` when the drop happens inside a tagged dispatch | `warn` |

**Why `client.lua.error` rows had no message (fixed 2026-09-29).** In the 2026-09-29 colo session 8 of 9 rows were `status: -1, status_name: unknown, message: null`. `lua51.dll` is C++-compiled: `luaD_rawrunprotected` (`0x10008600`) catches with one `catch (...)` (`Catch_All@1000864f`) that sets the status to -1 when a Lua `error()` had not already set 2-5. So -1 is an exception Lua did not raise (a C++ throw from a CEGUI or tolua binding, or a structured exception). `luaD_seterrorobj` (`0x100084c0`) writes an error value only for 2-5; for -1 it leaves the called function on top, so there is no message to read. The row now names the status `foreign_exception` and, when the stack has a free slot (checked from `L->top` / `L->stack_last`, so nothing grows), names the handler through `lua_pushvalue` + `lua_getinfo(">S")`. The exception text, when CEGUI logged it, is the `client.ui.cegui_log` error just before; an SEH fault is `client.os.exception`. Layouts and anchors are from Ghidra on the QA `lua51.dll`, recorded in `hooks/lua_stack.rs`.

**`Debug:log` went nowhere (C10, 2026-09-29).** `Debug` is the tolua usertype `ScriptedDebug` (registered at `0x00ad46e5`). Its `log`, `warn` and `error` bindings check `self` and a string argument, build a `std::wstring` and call `0x0081c2e0`, a bare `ret`: the shipping client compiled the logger out, so neither the stock UI's lines nor the Black Market overlay's `[Cimmeria BM]` lines reached any file. The detour reads argument 2 with `lua_type` + `lua_tolstring` (string only: nothing converted or allocated) and then runs the binding unchanged. It is in the player DLL, not only the lab build: three inline sites, all in the fingerprint gate (the 41-byte prologue includes the push of the `ScriptedDebug` type string, so it pins what the function is as well as the build). Under `lab-bridge` the lines are also in the local ring as `lua.debug_log`.

**How the entity id reaches a CME event.** `Client_NetIn_EntityMethodDispatch` (`0x00c6f8f0`) writes the entity id into the event it creates (`*(event+8) = *(msg+0xC)` after the factory returns), and the factory hook only sees the name. The dispatcher is hooked by the client-patches DLL and stays untouched. It has exactly two callers, `onEntityMethod` and the queue replay, so the DLL hooks those two entry points instead and sets a thread-local dispatch context (entity id, type, message id) around the original; the factory hook reads it. The context is restored by a drop guard, so a C++ exception through the dispatch leaves nothing stale on the network thread.

**Reading a missing entity.** For an NPC that was created but never rendered, query `client.entity.*` by `entity_id`: `create.outcome = parked` with `entered_world = false` and a `client.entity.enter` with `place_before = cache` and `entered_world = false` say the client never made it live; `queued_msgs > 0` and no `queue_replay` say its appearance methods are still waiting; `appearance_request.outcome = scheduled` with no pawn says the fault is downstream of the client's own bookkeeping.

**Safety.** Every read of a game structure goes through `ReadProcessMemory` (`cimmeria_client_hookgate::os::read_bytes`), so a stale pointer reads as missing instead of faulting the network thread; the std::map walker is bounded (96 steps a lookup, 20,000 nodes for the catalog); the CME catalog runs on its own thread after a 2 second delay, so the game thread never waits on it; every detour forwards its arguments and result untouched and keeps telemetry code inside `catch_unwind`. All eleven new inline sites are in the fingerprint gate.

## Mercury receive path: packets, fragments, bundles

Added 2026-09-29 to confirm, from inside the client, how it receives, reassembles and processes a multi-fragment reliable bundle (the 15-fragment, 18,367-byte AoI bundle whose tail is never processed although every fragment was ACKed). Anchors, layouts and the rules each event rests on are in [client-mercury-receive-path.md](../reverse-engineering/findings/client-mercury-receive-path.md); none of it has run in the live client yet. Same rules as above: the DLL every player gets, mirrored to the lab ring under `lab-bridge`, kind = target without `client.`.

The client's Mercury logger is a one-byte stub in this build (`0x0081c2e0` is `ret`), so none of its `[Mercury] ...` reasons ever print. These events are where they come back.

| Target | Hook (address) | Fields | Level |
|---|---|---|---|
| `client.mercury.socket_recv` | `recvfrom` IAT slot `0x017eff60` (WS2_32), restricted to the Mercury caller's 1472-byte receive buffer | `socket`, `requested_len`, `peer` (IPv4 source when available), `outcome` (`received`, `message_too_large`, `socket_error`); on success `wire_len`, `wire_fingerprint` (16 hex digits, FNV-1a of exact UDP bytes), `at_buffer_capacity`; on failure `wsa_error` | `info` received, `warn` Winsock error; `WSAEWOULDBLOCK` is omitted |
| `client.mercury.rx_gap` | `UnAckedHandler::queueAckForPacket` `0x0158cba0` | `event` (`rx_gap_open`, `rx_gap_stall`, `rx_gap_closed`), `channel_ptr`, `expected_seq`, `buffered_count`, `peak_buffered`; stall/close add `duration_ms`, close adds `next_expected`. A channel silent for 60 s drops its tracker, so a reused channel address starts clean | `info` open/close, `warn` stall (2 s); all must-keep |
| `client.mercury.packet_in` | `Nub::processFilteredPacket` `0x01580840`, with the window note from `queueAckForPacket` `0x0158cba0` | footers parsed from a copy of the datagram: `flags`, `flag_names`, `len`, `payload_len`, `reliable`, `on_channel`, `fragmented`, `has_acks`, `ack_count`, `seq`, `frag_first`, `frag_last`, `request_offset`; the window: `disposition` (`delivered`, `buffered`, `duplicate_buffered`, `out_of_window`, `old_duplicate`), `in_seq_at_before`/`_after`, `buffered_after`, and `released` (in-order packets freed, buffered followers included) / `ahead` / `behind` / `window`; `result` (the filter's return), `bad_packets_delta` (the client's own bad-packet counter, `Nub+0xf8`), `ack_only` | `debug` ordinary, `info` fragment or buffered, `warn` non-happy |
| `client.mercury.fragment` | `Nub::processPacket` `0x0157fd20` | `outcome` (`group_started`, `added`, `completed`, `duplicate`, `mangled_footers`, `bundle_missing`, `group_restarted`, `group_discarded`, `illegal_footers`, `no_channel`), `seq`, `frag_first`, `frag_last`, `expected_fragments`, `payload_len`, `remaining_before`/`_after`, `held_before`/`_after`, `held_bytes_*`, `open_group_last`, and on a fragment that did not simply join `held_seqs`; on `completed`: `assembled_packets`, `assembled_bytes`, `expected`, `count_matches` | `info` happy, `warn` non-happy |
| `client.mercury.bundle` | `Nub::processOrderedPacket` `0x0157c820` (game thread), fed per message by `Bundle::iterator::unpack` `0x01579830` | `phase=start` (assembled bundles only): `source`, `packets`, `total_bytes`, `seq_first`, `seq_last`, `boundaries` (payload offsets where each packet after the first begins). `phase=end`: the same plus `messages`, `dispatched` (the client's own counter), `nub_aborted_delta`, `consumed_bytes`, `unconsumed_bytes`, `straddled_messages`, `first_msg_id`, `last_msg_id`, `last_msg_offset`, `last_msg_len`, `exit` (`clean_end`, `unknown_message_id`, `corrupted_header`, `other`), `result`, and on an abort `abort_offset`, `abort_packet_index`, `abort_packet_seq`, `abort_msg_id`, `fault` (`header_does_not_fit_packet`, `body_runs_out_of_packets`, `length_expand_failed`), `header_len`, `header_bytes_in_packet`, `packet_len`, `abort_cursor` | `info` assembled, `debug` clean single-packet, `warn` abort |
| `client.mercury.error` | paired with every non-happy event above | `stage` (`packet`, `fragment`, `bundle`), `reason`, plus the parent event's fields | `warn` |

**Offsets** in `client.mercury.bundle` are payload offsets: every packet's bytes after its flags byte, concatenated, so a fragment boundary is a number and a message header at `abort_offset` straddling `boundaries[i]` is visible at a glance.

**Volume.** Per-packet events are emitted for every fragment, every packet while a fragment group is in flight (until 5 s after the last fragment that left a group open), every packet buffered for a gap, and every non-happy packet, all unthrottled. Ordinary traffic goes through the per-name bucket (burst 8, 4/s). A non-happy outcome of any kind bypasses every throttle, by construction: the gate is a function of the outcome and is unit-tested (`report::tests`), not a property of the hook. A clean single-packet bundle is throttled; an assembled or aborted one never is.

**Reading a transmit hole.** Join a server `mercury.tx_hole` `tx_hole_stall` to its `mercury.reliable_send` row by `peer` and `seq`. Both carry `wire_len`, `wire_fingerprint` and `send_site`. The fingerprint matches a `client.mercury.socket_recv` row only when the exact datagram reached the client's Winsock boundary. There is no matching fingerprint further in: `processFilteredPacket` sees the packet after the channel filter has decrypted it, so its bytes are plaintext. Ghidra shows the Mercury socket caller (`0x0158a200`) asks `recvfrom` for **1472 bytes** and sets `Packet+0x24` only on success. A server payload of 1488 bytes therefore predicts `WSAEMSGSIZE` (`wsa_error = 10040`) at this socket, before `client.mercury.packet_in`; the new event is the live check. `client.mercury.rx_gap` reports the missing `expected_seq` and how long later reliable packets stayed buffered. Absence of a matching socket row alone is inconclusive because ordinary socket rows are budgeted; Winsock errors are must-keep warnings. The fingerprint is a diagnostic join key, not a security hash, and raw payloads are never logged.

**Reading a partial bundle.** `client.mercury.fragment` `completed` with `count_matches = true` and `assembled_bytes` equal to what the server sent says reassembly was whole; then `client.mercury.bundle` `end` says what the message loop did with it. `exit = corrupted_header` with `fault = header_does_not_fit_packet` and an `abort_offset` a few bytes before a `boundaries` entry is the fragment-boundary header split; `exit = clean_end` with `dispatched` equal to `messages` and `unconsumed_bytes = 0` clears the Mercury layer and moves the search to the entity layer (`client.mercury.entity_method` `path = queued`). A `fragment` event with `outcome = mangled_footers` or `bundle_missing` is a fragment the client dropped after ACKing it.

**Cost and safety.** The packet filter reads each datagram once (one `ReadProcessMemory` of at most 2048 bytes); a fragment adds two group reads; the message loop adds three small reads per message on the game thread. Every read is checked, every list walk is bounded (128 nodes), every detour forwards its arguments and result untouched inside `catch_unwind`, and the bundle trace is cleared by a drop guard so a C++ exception through the message loop leaves nothing stale. `queueAckForPacket`'s four stack words are forwarded blindly (`ret 0x10`): the hook reads only the channel (`this`), so an argument-order surprise there cannot corrupt the call.

## SequenceManager drops: `client.sequence.dropped`

Added 2026-09-29 after a colo session in which an NPC's `onSequence` (client method 1) reached the client's dispatcher and drew nothing ([npc-attack-presentation.md](../reverse-engineering/findings/npc-attack-presentation.md), PR #1116). The server logs every sequence it sends; this event says which ones the client's `SequenceManager` then threw away, and why. Every anchor below was read from the QA `SGW.exe` in Ghidra on 2026-09-29 (decompiled, disassembled to the `ret`, prologue bytes in the fingerprint gate); none of it has run in a live client yet. It is in the DLL every player gets, and mirrored to the lab ring under `lab-bridge` (kind `sequence.dropped`).

| Function | Address | Signature | What the hook does |
|---|---|---|---|
| `Event_Cache_ElementReady` handler | `0x00d06f30` | `thiscall(this, evt, arg)`, `ret 8` | Before the original runs, works out from memory which of the requests filed under the ready sequence id it will drop, and which stale ones its prune will erase |
| Play step | `0x00d06dd0` | `thiscall(this, data, request, source_entity)`, `ret 0xc` | Reads the request's ids, then watches whether the step reaches the instantiate call |
| Kismet instantiate | `0x00d067e0` | `thiscall(this, out_instance*, name, pawn)`, `ret 0xc` | Inside a play step only: records whether it ran and the instance it wrote (the slash-command and editor callers pass straight through) |

The request layout (`FUN_00d13780`): `+0x00` `KismetEventSetSeqID` (the multimap key), `+0x0c` `SourceID`, `+0x10` `TargetID`, `+0x14` `_time64` filing time, `+0x1c` `ViewType`, `+0x20` `InstanceId`. Requests live in a `std::multimap<int, Request*>` at `SequenceManager+0x48`. The Source is looked up the way the game does (`FUN_00dd0de0`: the entity manager singleton at `0x01ef244c`, its world map at `+0x18`); the pawn is `Entity+0x08`.

| `path` | Where | Condition | Level |
|---|---|---|---|
| `no_source_entity` | ready handler | No client entity for `SourceID` (for event 5001 or `ViewType` 1/2 too, which otherwise wait for the appearance job) | `info` |
| `no_source_pawn` | ready handler | The Source entity exists without a pawn and the sequence is not one the handler defers | `info` |
| `no_cooked_data` | ready handler | The Source has a pawn but the ready event carries no sequence data | `info` |
| `expired` | ready handler's prune (`FUN_00d05450(5, map)`) | More than 5 requests remain and this one was filed 31 s or more ago; carries `age_secs` | `info` |
| `culled_by_distance` | play step | The step returned before instantiating: the nearer endpoint is beyond the local viewer's view-distance setting | `debug` |
| `instance_refused` | play step | The instantiate call wrote no instance (sequences switched off, the active-instance cap, or the script did not load) | `info` |

Fields: `path`, `stage` (`cache_ready`, or `appearance_ready` when the play step ran from the `Event_AppearanceJob_Completed` path at `0x00d055f0`), `sequence_id`, `entity_id` (the Source), `target_id`, `view_type`, `instance_id`, `event_id` (the cooked data's Kismet event id) and `suppressed`. A deferral is not a drop and emits nothing; a play step that instantiates emits nothing.

**Volume and safety.** Throttled per (path, Source entity), burst 8 then 4 a second, and classed per-entity in the governor (key `path`). Every read goes through `ReadProcessMemory`; the multimap walk is bounded (256 requests under one id, 64 for the prune check, 96 steps a descent), and nothing calls game code. The distance cull cannot be recomputed without calling the engine, so it is observed, not predicted: the play step's only early return skips the instantiate call.

**The `Event_NetIn_onSequence` handler (`0x00d05790`) is hooked since 2026-10-04** ([Ability telemetry](#what-the-client-showed-clientabilityshown-and-the-onsequence-drop-ab-c5)): it reads the event's fields through the game's own `GetInt` / `GetByte` instead of the property tree, so the paragraph below is history. **Before that: not hooked.** It drops a sequence whose `SourceID` has no client entity before any request exists. Its drop branch holds the ids only inside the event's CME `BasicPropertyTree` (read through `Mercury__unknown_00e3cba0` / `Detail__unknown_00438b50`), and that tree's layout is not verified, so a hook there could report only "something was dropped". Proposal: verify the property tree's node layout (the `SourceID` and `KismetEventSetSeqID` entries are `long` properties) in Ghidra, then hook `0x00d05790` with a thread-local flag set by a hook on the request parser `0x00d13780` (reached only when the Source exists): no parse means `path = no_source_entity`, `stage = net_in`. Until then, a server `abilities.sequence` row with a matching `client.mercury.entity_method` (`msg_id = 1`) and neither a ready-handler drop nor a spawned emitter points at this branch.

## Ability telemetry: `client.ability.*`

Added 2026-10-04 for the ability-mechanics campaign (Part 2 of [lab-uat-and-telemetry.md](../analysis/ability-mechanics/lab-uat-and-telemetry.md)). Anchors and their evidence: [ability-client-hook-anchors.md](../reverse-engineering/findings/ability-client-hook-anchors.md). These events follow one cast through the client by the server's `cast_id`. They are in the DLL every player gets (D-AU4), and each is also pushed to the lab ring under `lab-bridge` (kind = target without `client.`). **Live status: partly verified.** The decoders are tested against synthetic wire bytes and the checked-in definitions, and the anchors were read from the QA `SGW.exe`. The first live smoke run (colo, 2026-10-04) observed `client.ability.press`, `sent`, `sent_seq` and `applied` rows for one Heal Focus press, through the lab ring: `applied` `kind = cooldown` for the warmup and cooldown timers (`timer_type` 1 and 2, matching the server's two `onTimerUpdate` sends) and `kind = stat`. `client.ability.recv` was broken in that run and is being fixed in #1203. `shown`, `press_dropped` and the `onSequence` net-in drop were not exercised.

**Volume (D-AU5).** One table holds every ability bucket (`hooks/ability_trace/throttle.rs`): burst 8, then 4 a second per name, and the next event of that name carries the dropped count as `suppressed`. Two rules sit on it. A press and its answer are kept or dropped together: rows with a `press_id` follow the decision made for their `press` row (AB-C2, below). Recv, applied and shown rows have their own per-name buckets, keyed by method or kind and by `self` (the local player) or `other` (any other being), so a stat storm cannot hide an `onEffectResults` and a fight's NPC traffic cannot starve the player's own rows. Because the hook already budgets them, the governor forwards every `client.ability.*` row untouched (`KeepReason::AbilityTrace` in `governor/classify.rs`), and the ingest's runaway guard replays them over budget (`PRIORITY_PREFIXES` in `admin-api` `session_budget.rs`).

### Presses and sends (AB-C1, AB-C2)

Added 2026-10-04 (AB-C1 and AB-C2 of the [ability-mechanics telemetry plan](../analysis/ability-mechanics/lab-uat-and-telemetry.md#part-2-client-telemetry-shipped-to-every-player-ab-c)). These rows answer "did this press leave the client, with which target, and in which packet": `client.ability.sent_seq` carries the outbound packet range, which the server's `use_ability_recv` / `use_ability_on_ground_recv` row (`abilities`, `stage = recv`) joins through its `mercury_seq` field (AB-T2, #1176). The anchors are in [ability-client-hook-anchors.md](../reverse-engineering/findings/ability-client-hook-anchors.md), which was static only (headless Ghidra); this packet added the GamePet send and the two `start*Message` functions from the QA image. **The 2026-10-04 live smoke run observed `press`, `sent` and `sent_seq` rows** for a Heal Focus press, so the press chain, the router decode and the bundle join ran live on that path. The table below still marks every anchor UNVERIFIED: that run did not record which press source fired (hotbar or `lua`), and the pet, `press_dropped` and proxy-route branches were not exercised. They are in the DLL every player gets (D-AU4), not behind `lab-bridge`, and each event is mirrored to the lab ring under `lab-bridge` (kinds `ability.press`, `ability.press_dropped`, `ability.sent`, `ability.sent_seq`).

| Target | When | Fields | Level |
|---|---|---|---|
| `client.ability.press` | The press chain knows the ability, or knows the press failed | `press_id`, `source` (`hotbar` for `useAction`; `lua` for `useAbility`, which the Ability window's button and any script both call, so native code cannot tell them apart), `slot` (the hotbar action id, 1-based), `ability_id`, `target_id` (what the client is about to send as `TargetID`), `self_cast`, `pet_id`, `pending_expired` (posted presses no router call claimed within 5 s, or 60 s for a ground reticle, since the last press) | `info` |
| `client.ability.press_dropped` | The client discarded the press, or the router refused an allowlisted send | `press_id` (null for a send with no press: a GM slash command, the respec button), `source`, `slot`, `ability_id`, `method` (router drops), `pet_id`, `reason`, `drop_site` (the branch address), `route_rows` (`9\|10` for `class_mismatch`) | `info` |
| `client.ability.sent` | The router reached `startEntityMessage` or `startProxyMessage` for an allowlisted method | `send_id`, `press_id`, `press_to_sent_ms` (AB-C6: the claimed press's age on the client clock; null with no press), `method`, `cell_index`, `route`, `msg_id`, `sub_index`, the decoded arguments (`ability_id`, `target_id`, `ground_xyz`, `pet_id`, `toggle`, `effect_id`, `accepted`; only those the method has), `args_missing`, `client_target_id`, `client_target_inferred` (always `true` until live-verified) | `info` |
| `client.ability.sent_seq` | `Nub::send` finished the bundle that carried the call | `send_id`, `press_id`, `method`, `ability_id`, `mercury_seq_first`, `mercury_seq_last`, `packets`, `seq_bits` (28), `evicted_unmatched` | `info` |

A press gets exactly one answer: a `press_dropped`, or a `sent` that carries its `press_id`. Two exceptions, both visible in the rows: a hotbar slot holding a non-ability action (an item, a macro) reaches no ability executor and reports nothing, and a ground ability's press ends at the reticle until the player places it (its `useAbilityOnGroundTarget` `sent` then carries the press id; a cancelled reticle has no client branch to report and shows up as `pending_expired` on a later press).

**Allowlist.** `useAbility` (68), `useAbilityOnGroundTarget` (69), `petInvokeAbility` (88), `petAbilityToggle` (89), `confirmationResponse` (4: `INT32 aEffectId`, `UINT8 aAccepted`), `resetMyAbilities` (72, sent as `Event_NetOut_RespecAbility`), `trainAbility` (77), and the GM debug methods `gmDebugAbility` (169), `gmDebugCombat` (170), `gmDebugCombatVerbose` (171), `gmDebugHeal` (172) and `gmDebugAbilityOnMob` (176). `toggleCombatDebug` (2, 3) is absent: the client cannot send it.

**Reasons.** Only the client's own branches; it checks no cooldown, range, target or death, so there is no such reason.

| `reason` | `drop_site` | Branch |
|---|---|---|
| `bad_args` | `0x00aa9569` / `0x00aa2997` | The `useAction` / `useAbility` binding's argument check failed and it raised a Lua error (seen as the binding unwinding before its next step) |
| `no_action` | `0x00ad959e` | Empty hotbar slot, or an action id outside 1..=200 |
| `pet_missing` | `0x00e3cfb1` | The pet action's entity is not a `GamePet` |
| `not_known` | `0x00d2afcf`, or `0x00d3a862` for a pet | The ability is not in the client's (or the pet's) `AbilitySet`. The press never reaches the wire |
| `pet_state_flag` | `0x00d3a84a` | `GamePet+0x38` lacks bit `0x400` (meaning unresolved) |
| `pet_ability_flag` | `0x00d3a875` | The pet's ability record has bit `0x8` of `+0x98` set (meaning unresolved) |
| `not_connected` | `0x00c6fc68`, `0x00c6fc77`, `0x00c6fca8` | Router rows 6 to 8: no `ServerConnection`, not connected, or the local player is not in the entity maps |
| `class_mismatch` | `0x00c6fcd2\|0x00c6fd1d\|0x00c6fd41` | Router rows 9 and 10: the entity type has no description mapping, or its class chain lacks the method's class (how a non-GM player's `gmDebug*` call dies). Only game code tells the two apart |

Rows 6 to 8 are read from memory before the router runs; rows 9 and 10 are observed (no `start*Message` ran). Row 1 (`actionsEnabled` false, or a button with no action) is pure Lua and never reaches native code.

**How each hook works.**

| Hook | Address | Signature | Role | Live status |
|---|---|---|---|---|
| `useAction` tolua thunk | `0x00aa94e0` | `cdecl int(lua_State*)` | Opens a `hotbar` press scope; a press that unwinds before the slot step is `bad_args` | UNVERIFIED |
| `useAbility` tolua thunk | `0x00aa2910` | `cdecl int(lua_State*)` | Opens a `lua` press scope; unwinding before the lookup is `bad_args` | UNVERIFIED |
| `FUN_00ad9580` | `0x00ad9580` | `cdecl void(actionId, self)` | Records the slot; reads the slot first (`[[EM+0x8c]+0x4c]`, vector at `+8`) so an empty one is `no_action` | UNVERIFIED |
| `FUN_00d2afc0` | `0x00d2afc0` | `thiscall(set, abilityId, targetId)`, `ret 8` | Emits the press row; returns without the send builder = `not_known` | UNVERIFIED |
| `FUN_00d2ae40` | `0x00d2ae40` | `thiscall(set, record, targetId)`, `ret 8` | The event is posted (or the ground reticle opens, `record+0x48 == 3`): leaves a pending send | UNVERIFIED |
| `PetAbilityAction::execute` | `0x00e3cf40` | `thiscall(action, self)`, `ret 4` | Pet press row (`action+0xc` ability, `+0x10` pet); no GamePet send = `pet_missing` | UNVERIFIED |
| GamePet send | `0x00d3a820` | `thiscall(pet, abilityId, targetId)`, `ret 8` | Reads its three gates from memory, then leaves a pending `petInvokeAbility` | UNVERIFIED |
| `RouteOutgoingEntityRpc` | `0x00c6fc40` | `stdcall(entity, desc, method, args)`, `ret 0x10` | The existing `client.net.out` hook. For an allowlisted method it decodes `args` with the game's `GetInt` `0x00e3cba0`, `GetFloat` `0x00e3cc20` and `GetByte` `0x00d434d0` (each called with the descriptor's own argument-name `std::string`), reads `client_target_id`, claims the oldest pending press with the same method and ability id, and reports `sent` or `press_dropped` after the original returns | UNVERIFIED (the decode) |
| `startEntityMessage` | `0x00dd6a60` | `thiscall(conn, msgId, entityId)`, `ret 8` | Marks the router call as sent (`0x00dd8010` tail-calls it) | UNVERIFIED |
| `startProxyMessage` | `0x00dd6980` | `thiscall(conn, msgId)`, `ret 4` | Same, base route | UNVERIFIED |
| `Channel::send` | `0x01576f90` | `thiscall(channel) -> int` | Tags the bundle at `channel+0x28` with the sends recorded since the last bundle, before the original hands it to the network thread; undoes the tag if the bundle was not detached | UNVERIFIED |
| `Nub::send` | `0x01582160` | `thiscall(nub, addr, bundle, channel)`, `ret 0xc` | Network thread. Claims a tagged bundle by its pointer (never dereferenced: it may be freed before the call returns) and records the sequence numbers, then emits `sent_seq` | UNVERIFIED |
| Reliable sequence counter | `0x0158bb40` | `thiscall(channelInternal) -> u32` | One value per packet while a tagged bundle is being sent; a thread-local check otherwise | UNVERIFIED |

`client_target_id` is `GameBeing+0xfc` of `[[EM+0x8c]+0x4]` (`EM = [0x01ef244c]`), the field the hotbar sends as `TargetID`. That this pointer is the `GameBeing` with no cast adjustment is the finding's open question 1, hence `client_target_inferred`. Note that `useAbility(id, Unit.Target)` sends the target's own `+0xfc`, so on the `lua` path `target_id` and `client_target_id` can legitimately differ.

**Joining a press to the server's cast.** The server's `useAbility` receipt row (`event = use_ability_recv`) carries `mercury_seq`, the inbound packet sequence (AB-T2), and the launch that follows mints `cast_id`. Join that row to `client.ability.sent_seq` by `mercury_seq` in `mercury_seq_first..mercury_seq_last`, with the 28-bit modular test `((mercury_seq - first) & 0x0fffffff) <= ((last - first) & 0x0fffffff)`, never `first <= mercury_seq <= last` (the counter wraps). Then `send_id` leads to the `sent` row and `press_id` to the press. A bundle sent without the reliable counter reports null bounds and `packets = 0`.

**Volume and safety.** D-AU5: a burst of 8, then 4 a second, per name, with `suppressed` on the next row through. A press and its answer are one unit: the decision is made once, on the `press` row, against its source's bucket, and every later row with the same `press_id` (`press_dropped`, `sent`, `sent_seq`) follows it, so both rows go or neither does. Suppressed presses are counted on the next `press` row through. Rows with no `press_id` (a router refusal or a send with no press) use their own bucket, named by drop reason or method. The governor keeps `client.ability.*` (must-keep, `ability_trace`) so it never separates a press from its answer, and the ingest replays the prefix over budget. Every read of game memory goes through `ReadProcessMemory`; the pending table holds 32 presses and the join tables 64 sends and 32 bundles, each counting what it evicts. The bag readers are the only game code called; they run outside `catch_unwind`, so a C++ exception from one unwinds the way the game expects. Every hooked function and the three readers are in the fingerprint gate (16 bytes each), so another build installs none of them. A hook that misbehaves in the lab is switched off by name with `CIMMERIA_CLIENT_HOOKS_DISABLE` (`ability_*`). The tests install each detour with MinHook on a stand-in of the same ABI, wired like the game's chain, then remove it (`hooks/inline_hooks/ability/tests.rs`).

**First live checks (for the coordinator).** `client.hooks.inline.installed` for the 12 `ability_*` hooks; a hotbar press of a known ability gives `press` then `sent` with the same `press_id` and a `sent_seq` whose range contains the server's receipt seq; `client_target_id` equals the server's `setTargetID`; a press of an ability the client was never taught gives `not_known`; a non-GM player's combat-debug slash command gives `class_mismatch`. Watch `Nub::send` in particular: an extra frame around it is the same kind of change that broke `processOrderedPacket` (see `DEFAULT_OFF`).

### What the client received: `client.ability.recv` (AB-C3)

No new hook. The existing `EntityManager::onEntityMethod` detour (`0x00dd2b80`) reads the message's argument bytes before the original consumes them, and the bytes `[cursor, end)` are the arguments in `.def` order. The cursor is never moved. A message id that cannot be one of the methods below costs one compare; an extended id (61, the player's `0xBD`) costs a one-byte read of its sub-index before the rest is read.

**Which stream (fixed 2026-10-04).** In the live client `onEntityMethod` is not called from the Nub with its `MemoryIStream`. `SGWMessageQueue` (vtable `0x01b14f3c`) is the connection's handler on the network thread: its `onEntityMethod` (`0x01563630`) copies the arguments into an `EntityMethodMessage` that owns a `MemoryOStream`, and `EntityMethodMessage::process` (`0x01561ac0`) later calls the `EntityManager`'s `onEntityMethod` with that `MemoryOStream`'s `BinaryIStream` subobject. The hook reads both layouts (`hooks/ability_trace/recv_stream.rs`):

| Stream | vtable | `remaining` (slot 2) | read cursor | end |
|---|---|---|---|---|
| `MemoryOStream`'s `BinaryIStream` subobject (every live message) | `0x019ce734` | `0x00dd3f80` | `+0x14` | `+0x0c` |
| `MemoryIStream` (a direct Nub dispatch) | `0x01b18e38` | `0x0157af60` | `+0x08` | `+0x0c` |

A stream is matched by its vtable or by its `remaining` slot. The first version knew only the `MemoryIStream` layout and returned without a trace for every live message, so the first live run (colo, 2026-10-04) had `client.ability.applied` rows and no `client.ability.recv` at all.

**`client.ability.recv_skipped`.** A message that may be one of the methods below and is not decoded is never dropped silently. Fields: `reason`, `msg_id`, `method_index` and `method` (null when an extended id's sub-index was not read), `entity_id`, `path`, `len` (when the window was read), `vtable` (hex, for `unknown_stream`). Reasons: `unknown_stream` (neither layout), `bad_window` (the stream, cursor or end unreadable, or the cursor past the end), `read_failed` (the sub-index or the arguments unreadable), all `warn`; and `receiver_unknown` (`info`): a player-only method for an entity in no map and not the local player, which may be ours. One bucket per reason (`recv_skipped:<reason>`, the D-AU5 limits) with `suppressed`. A message that is not one of ours (another method, an `onPlayerCommunication` on another channel, an index 27 and up for a non-player) is not a skip.

The decoder is driven by a table of each method's `.def` argument list (`hooks/ability_trace/recv_methods.rs`). A test reads `entities/defs/` and `alias.xml` and fails if any argument name, type or dictionary layout drifts; another checks every index against [client-method-dispatch-table.md](../protocol/client-method-dispatch-table.md).

| Method (index) | Receivers | Fields after the common ones |
|---|---|---|
| `onSequence` (1) | any being | `sequence_id`, `source_id`, `target_id`, `primary_target`, `impact_time`, `nvps_count`, `nvps` (`[name, value]` rows), `view_type`, `instance_id`; `cast_id` = `instance_id` when non-zero |
| `onTimerUpdate` (12) | any being | `timer_id`, `timer_type`, `source_id`, `secondary_id`, `total_time`, `complete_time` (game-clock seconds) |
| `onEffectResults` (14) | any being | `source_id`, `ability_id`, `effect_id`, `target_id`, `result_code`, `results_count`, `results` (`[StatID, Delta, DamageCode, StatResultCode]` rows); `cast_id` = `effect_id` |
| `onStateFieldUpdate` (19) | any being | `state_field` |
| `onStatUpdate` (20), `onStatBaseUpdate` (21) | any being | `stats_count`, `stats` (`[StatId, Min, Current, Max]` rows): one event per message, not per stat |
| `onPlayerCommunication` (28) | player | `speaker`, `speaker_flags`, `channel`, `text`; **feedback channel (9) only**: other channels are players' chat and are not reported. One that does not decode as far as `Channel` is a `warn` with no decoded fields |
| `onKnownAbilitiesUpdate` (101) | player | `ability_ids_count`, `ability_ids` |
| `onErrorCode` (121) | player | `system_id`, `instance_id`, `error_code` |
| `onAbilityTreeInfo` (141) | player | `ability_lists_count`, `ability_lists` |

Common fields: `method`, `method_index`, `entity_id` (the receiver), `msg_id`, `len` (argument bytes), `path` (`delivered`, `local_player`, `queued`). A `queued` message is applied later, when its entity enters the world, so its receive time is not its apply time. Arrays keep their first 32 elements and strings their first 256 characters; the count fields always hold the declared length, also when the payload was cut at the 4 KiB read cap (`bytes_capped`; that row is `info` with `decode_error = truncated in <ArgName>` and the elements that arrived). Level `info`; a payload that does not decode as its `.def` says is a `warn` with `decode_error`, which bypasses the bucket: `truncated in <ArgName>` when the bytes run out, or `trailing_bytes` (with `trailing_bytes = N`) when bytes are left after the last argument. No method is allowed trailing bytes.

Indices 0 to 26 mean the same method on every being, so they are decoded for any receiver. 27 and up are `SGWPlayer`'s own and are decoded only for the local player or an entity of type 2 or 3 (`SGWPlayer`, `SGWGmPlayer`): an `SGWMob`'s index 28 is a different method. Extended ids are player-only for the same reason.

`Ability_Interrupt` is not a method. It is an `onSequence` whose sequence the server looked up for Kismet event 1002, and the wire carries only the sequence id, so a recv row cannot name it; join it to the server's `abilities.sequence` row by `cast_id`. `onSendCombatDebug` is not a client method; the native combat-debug lines reach the client as feedback-channel `onPlayerCommunication`, which the table above reports.

### What the client applied: `client.ability.applied` (AB-C4)

Eleven inline hooks on the stock handlers, plus the existing `onStateFieldUpdate` hook. Every address, argument count and `ret` was read from the QA `SGW.exe` disassembly on 2026-10-04 and is in the fingerprint gate. The handlers' fields are read before the original runs, through the game's own event getters `GetInt` `0x00e3cba0`, `GetFloat` `0x00e3cc20` and `GetByte` `0x00d434d0` (`bool thiscall(event, const std::string*, T*)`, `ret 8`; also fingerprinted): the same calls the handler makes, on its thread, with a hand-built MSVC 2008 `std::string` name (`hooks/ability_trace/event_bag.rs`). The getters are game code with a C++ exception frame, so they are called outside `catch_unwind`: a throw (realistically `bad_alloc` while copying the property tree) unwinds through the `thiscall-unwind` detour to the game's own handler, as it would from the handler's own call. The probes only note that they were called. A row's `now` and `remaining` come from the client's game clock, read from memory the way `0x00dd6c60` computes it (`(ticks + fraction) / hertz`, `hooks/ability_trace/clock.rs`).

| `kind` | Hooks | Fields |
|---|---|---|
| `effect_bar_add` | `EffectSet` timer handler `0x00e09160` (`thiscall(this, event, subject)`, `ret 8`), entry-lookup probe `0x00e08570`, announce probe `0x00e0a9e0`, display-data-request probe `0x00e0a810`, post probe `0x00e0a2d0` (`thiscall(ui, int* id, record*)`, `ret 8`) | `entity_id` (the bar's owner, `[this+0x2c]`), `effect_id`, `timer_id`, `timer_type`, `source_id`, `secondary_id`, `total_time`, `complete_time`, `now`, `remaining`, and `ui`: `posted` (the add went to the UI: `0x00e0a2d0` ran), `data_requested` (the display data was not cached: the client sent `Event_NetOut_elementDataRequest`, category 9, for the effect id instead of drawing it), `data_request_pending` (a request was already outstanding, so nothing was sent) or `no_ui` (no effect UI existed yet, so nothing was sent at all) |
| `effect_bar_refresh` | same | as above, no `ui`: the entry existed and its interval moved |
| `effect_bar_clear` | same | the entry existed and the complete time is not in the future: the server's end of an effect (it sends `0.0`); the bar drops the entry on its next clock check |
| `effect_bar_ignored` | same | `reason = expired_on_arrival`: no entry, and the complete time is already past, so the handler creates nothing |
| `cooldown` | `CooldownManager` timer handler `0x00ea6af0` (`ret 8`), button-callback probe `0x00ea62b0` (`thiscall(this, type, id, float, float)`, `ret 0x10`) | `outcome`: `applied` (a hotbar button took it; `ui_values` are the two floats it was given), `no_button` (no button registered this `(Type, ID)`; not reported for effect timers, type 5), `other_source` (`SourceID` is not the manager's owner; `debug`); `ability_id`, the timer fields, `now`, `remaining` |
| `stat`, `stat_base` | `GameBeing` stat handlers `0x00e01f40` / `0x00e02060` (`ret 8`) and their per-stat functors `0x00e004e0` / `0x00e005b0` (`ret 0x10`) | `entity_id`, `stats_count`, `stats` as `[StatId, Min, Current, Max]` rows (the recv row's order; the functor is called as `(StatId, Max, Min, Current)`): one event per message |
| `state_flag` | the existing `GameBeing::onStateFieldUpdate` hook `0x00e01c90` | `entity_id`, `old` and `new` (`[this+0x158]` before and after the original), `changed`, `set`, `cleared` |

**Effect expiry is not an event.** No native call removes an entry when its time runs out: the bar computes the remaining time from the clock each time it draws. No verified seam exists for an "expired" event, so none is raised; a row's `complete_time` is the expiry on the same clock as `now`. The UI's display-cache lookup (`0x00e0a6f0`) is not hooked; `ui = posted` is observed at the post itself.

### What the client showed: `client.ability.shown` and the `onSequence` drop (AB-C5)

Combat text, chat lines and the effect bar are drawn by Lua: the native side raises a UI event and the script window calls the Lua function subscribed to it by name, through `lua_pcall`. The existing `lua_pcall` and `lua_call` IAT detours name the function being called by where it is defined (`lua_getinfo(">S")` on a copy of it, after the same stack-room check the error reader uses) and read its arguments before the call. A function that is not one of the handlers below is remembered by its closure address (`lua_topointer`), so the per-frame calls cost one lookup; the set is forgotten every 60 seconds because a collected closure's address can be reused.

| `kind` | Handler (stock client UI, file and line) | Fields |
|---|---|---|
| `combat_text` | `SCTMod.onUnitCombat`, `SCT.lua:83` | `ability_id`, `hit_type`, `stat_list` (`<table>`) |
| `combat_chat_line` | `CHAT_onUnitCombat`, `ChatEvents.lua:63` | same |
| `feedback_line` | `ChatMod.onMessageReceived`, `ChatWindow.lua:93`, channel 9 only | `speaker`, `speaker_flags`, `channel_id`, `channel_name`, `text`. Other channels are players' chat and are not reported |
| `effect_bar_ui` | `EffectsMod.onUnitEffectsUpdate`, `Effect.lua:7` | `unit` |
| `sequence_played` | the existing `SequenceManager` play-step hook, when the instantiate produced an instance | `sequence_id`, `entity_id` (Source), `target_id`, `instance_id`, `cast_id`, `event_id` (the cooked sequence's Kismet event), `interrupt` (`event_id == 1002`: this is where `Ability_Interrupt` becomes visible), `stage` |

Every Lua row has `handler` and `status`: `ok`, or `failed` with `lua_status` when the handler's `lua_pcall` failed, or `failed` with `raised = true` when a handler called through `lua_call` raised a Lua error. That error is a C++ throw (the client's `lua51.dll` is C++-compiled) that unwinds through the `extern "C-unwind"` detour; a drop guard (`shown::ReportOnExit`) reports the call on the way out. That is defined behaviour for a `C-unwind` frame, and on `i686-pc-windows-msvc` the guard runs as an SEH cleanup funclet, the mechanism the entity-dispatch scopes already rely on. The guard touches no Lua state and cannot unwind itself. The enclosing `lua_pcall` still reports the error as `client.lua.error`. A call whose argument count is not the script's gets `arity_mismatch` and the raw `args` instead of names. The file and line pairs are the stock client's; a UI patch that moves one of these functions needs its row in `hooks/ability_trace/shown.rs` updated (a test checks them against a client copy when `SGW_CLIENT_UI` names one). `client.lua.error` rows whose message or function source names `ActionButtons.lua`, `Effect.lua` or the `SCT` scripts now carry `ui_area = ability`.

**The `onSequence` net-in drop.** `SequenceManager::onSequence` (`0x00d05790`, `thiscall(this, event, subject)`, `ret 8`) is now hooked. Before the original runs, the detour reads `SourceID` with `GetInt` and looks it up in the entity manager's world map, as the handler does with `0x00dd0de0(manager, id, 0)`; when the entity is absent it reads `KismetEventSetSeqID`, `TargetID`, `InstanceId` and `ViewType` and reports `client.sequence.dropped` with `path = no_source_entity` and `stage = net_in`, through the same throttle as the other drop paths. Every `client.sequence.dropped` row now carries `cast_id` (the request's `InstanceId`) when it is non-zero.

### Timing of a cast (AB-C6)

Three intervals, all on the client's clock (the DLL's monotonic milliseconds, `ability_trace::now_ms`), so no clock sync with the server is involved. The joins are in `hooks/ability_trace/timing.rs`; each runs before the per-name bucket, so a suppressed row still updates the tables and the histograms.

| Interval | Field, on | How it joins |
|---|---|---|
| press to send | `press_to_sent_ms` on `client.ability.sent` | the press the router claimed (`press_id`); null for a send with no press |
| send to first answer | `send_id`, `press_id`, `sent_method`, `send_reply` (`first` \| `follow_up`), and on the first reply `sent_to_recv_ms`, on `client.ability.recv` | a reply names its ability and kind: `onEffectResults.AbilityID` (results), the `ID` of a warmup or cooldown `onTimerUpdate` (types 1, 2), the `InstanceID` of an ability-system `onErrorCode` (system 0, a refusal). `onSequence` names none. See "The send join" below |
| receive to applied | `recv_to_applied_ms` on `client.ability.applied` | the latest receive of the method that feeds the handler (`onTimerUpdate` for `effect_bar_*` and `cooldown`, keyed by entity and timer id; `onStatUpdate`, `onStatBaseUpdate`, `onStateFieldUpdate` by entity) within 5 s. A `queued` receive applied later than 5 s gets no field |

**The send join.** Only sends the server answers with cast replies are held: `useAbility`, `useAbilityOnGroundTarget`, `petInvokeAbility`. `trainAbility`, `resetMyAbilities`, `confirmationResponse`, `petAbilityToggle` and `gmDebug*` never wait, so they cannot take a cast's answer. A held send waits 5 s for its first reply (the server answers a receipt within a round trip), at most 32 are held. Only a reply addressed to the local player's own entity joins; a witnessed cast of the same ability never does. A reply belongs to the oldest answered send of its ability that has not had a reply of its kind yet, or whose bound `cast_id` it carries (`onEffectResults`); only when none takes it does it claim the oldest held send (FIFO). A refusal opens and ends a cast, so it claims a held send first when there is one. An answered send takes its cast's replies for 30 s after the last. So with two sends of one ability held, cast 1's warmup, cooldown and results all join send 1 and cast 2's first reply claims send 2. Follow-up replies carry `send_id`, `press_id`, `sent_method` and `send_reply = follow_up`, with no interval and no histogram observation.

The server's `cast_id` reaches the client only as `onEffectResults.EffectID` (the recv row's `cast_id`), so a press is joined to its cast through `send_id` on the first answer, or through `client.ability.sent_seq` and the server's receipt row.

**Histograms.** `client.ability.timing`, one event per series with observations, shipped by the uploader on the governor's health cadence (60 s) and at shutdown (`governor/ability_timing.rs`). Fields: `stage` (`press_to_sent`, `sent_to_recv`, `recv_to_applied`), `label` (the sent or received method, or the applied kind with the four `effect_bar_*` kinds as `effect_bar`; never an id), `count`, `sum_ms`, `min_ms`, `max_ms`, and per-bucket counts `le_10_ms`, `le_25_ms`, `le_50_ms`, `le_100_ms`, `le_250_ms`, `le_500_ms`, `le_1000_ms`, `le_2500_ms`, `le_5000_ms`, `le_10000_ms`, `gt_10000_ms` (not cumulative). At most 64 series per window; a further label counts under `other`. The events are generated by the governor, so they are never throttled or rolled up.

### Coverage gate (AB-C7)

[telemetry-coverage.md](../analysis/ability-mechanics/telemetry-coverage.md) is generated by `tools/telemetry-coverage/abilities.py` and checked in CI (`--check`). It crosses every ability method of the dispatch tables with the client's send and receive hooks and the server's receipt and send rows, reading the tables each side keeps for it. The client's half is declared in `hooks/ability_trace/coverage.rs` (`CLIENT_SENDS`, `CLIENT_RECVS`): the script fails when the declaration differs from its method set, and the crate's tests fail when a declared method does not resolve through `decode::spec_for` (the router hook's lookup) or `recv_methods::resolve` (the receive hook's) from its own wire id.

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

### Mercury receive-path anomaly events (2026-09-29)

| Event | Level | When |
|---|---|---|
| `client.mercury.request_misparse` | warn | `unpack` is about to parse a message as a request (iterator next-request offset == cursor) in a packet without the has-requests flag; the client's uninitialized iterator field, see [client-mercury-receive-path.md](../reverse-engineering/findings/client-mercury-receive-path.md#the-iterators-next-request-offset-is-never-initialized-confirmed-live-2026-09-29) |
| `client.mercury.unpack_fault` | warn | Any other `unpack` error, with the client's decoded length and the raw bytes at the cursor |

Hook switches: `CIMMERIA_CLIENT_HOOKS_DISABLE` leaves named inline hooks uninstalled; `CIMMERIA_CLIENT_HOOKS_ENABLE` installs default-off hooks. `mercury_process_ordered_packet` is default-off because sitting in its call path changes the client's behaviour.
