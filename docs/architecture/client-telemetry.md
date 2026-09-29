# Client-Side Telemetry — Architecture

> **Diátaxis type**: explanation
> **Audience**: engineers extending or reviewing the `cimmeria-client-telemetry` DLL and its launcher-side injector (issue #417)
> **Last updated**: 2026-09-27
> **Status**: Phases 2-5 substantially landed, **none of it yet run inside the real client** (the launcher never injected the DLL; see the fingerprint section). **37 hooks total** across 3 techniques: 27 inline JMP hooks (4 engine + state-flag dispatcher, anim notify A+B, console command, Bink tick, entity-method drop oracle, CME event-registry lookup, and 11 entity-lifecycle, inbound-message and outgoing-RPC hooks added 2026-09-28, and 5 Mercury receive-path hooks added 2026-09-29), 7 IAT-swap hooks (3 Lua + 4 OS), 3 vtable-swap hooks (CEGUI logger, now with the line text + AActor::Tick + USequence::UpdateOp). The two CME subscribers (`onClientMapLoad`, `onClientReady`) were removed on 2026-09-28: the subscribe API they called is not one (see Hook taxonomy item 1). Every address was re-checked against the QA `SGW.exe` on 2026-09-27 and is fingerprinted. **Removed pending re-resolution (#989)**: `Mercury::Nub::handleMessage` and the cooked-data PAK load, whose anchors were not function entries. **Deferred**: CME RTTI auto-discovery (~270 more events; needs `.rdata` scanner), FMOD runtime vtable traversal, ProcessEvent slot search, PropertyNode<T> per-T enumeration, Phase 6 crash filter.

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

## Mercury receive path: packets, fragments, bundles

Added 2026-09-29 to confirm, from inside the client, how it receives, reassembles and processes a multi-fragment reliable bundle (the 15-fragment, 18,367-byte AoI bundle whose tail is never processed although every fragment was ACKed). Anchors, layouts and the rules each event rests on are in [client-mercury-receive-path.md](../reverse-engineering/findings/client-mercury-receive-path.md); none of it has run in the live client yet. Same rules as above: the DLL every player gets, mirrored to the lab ring under `lab-bridge`, kind = target without `client.`.

The client's Mercury logger is a one-byte stub in this build (`0x0081c2e0` is `ret`), so none of its `[Mercury] ...` reasons ever print. These events are where they come back.

| Target | Hook (address) | Fields | Level |
|---|---|---|---|
| `client.mercury.packet_in` | `Nub::processFilteredPacket` `0x01580840`, with the window note from `queueAckForPacket` `0x0158cba0` | footers parsed from a copy of the datagram: `flags`, `flag_names`, `len`, `payload_len`, `reliable`, `on_channel`, `fragmented`, `has_acks`, `ack_count`, `seq`, `frag_first`, `frag_last`, `request_offset`; the window: `disposition` (`delivered`, `buffered`, `duplicate_buffered`, `out_of_window`, `old_duplicate`), `in_seq_at_before`/`_after`, `buffered_after`, and `released` (in-order packets freed, buffered followers included) / `ahead` / `behind` / `window`; `result` (the filter's return), `bad_packets_delta` (the client's own bad-packet counter, `Nub+0xf8`), `ack_only` | `debug` ordinary, `info` fragment or buffered, `warn` non-happy |
| `client.mercury.fragment` | `Nub::processPacket` `0x0157fd20` | `outcome` (`group_started`, `added`, `completed`, `duplicate`, `mangled_footers`, `bundle_missing`, `group_restarted`, `group_discarded`, `illegal_footers`, `no_channel`), `seq`, `frag_first`, `frag_last`, `expected_fragments`, `payload_len`, `remaining_before`/`_after`, `held_before`/`_after`, `held_bytes_*`, `open_group_last`, and on a fragment that did not simply join `held_seqs`; on `completed`: `assembled_packets`, `assembled_bytes`, `expected`, `count_matches` | `info` happy, `warn` non-happy |
| `client.mercury.bundle` | `Nub::processOrderedPacket` `0x0157c820` (game thread), fed per message by `Bundle::iterator::unpack` `0x01579830` | `phase=start` (assembled bundles only): `source`, `packets`, `total_bytes`, `seq_first`, `seq_last`, `boundaries` (payload offsets where each packet after the first begins). `phase=end`: the same plus `messages`, `dispatched` (the client's own counter), `nub_aborted_delta`, `consumed_bytes`, `unconsumed_bytes`, `straddled_messages`, `first_msg_id`, `last_msg_id`, `last_msg_offset`, `last_msg_len`, `exit` (`clean_end`, `unknown_message_id`, `corrupted_header`, `other`), `result`, and on an abort `abort_offset`, `abort_packet_index`, `abort_packet_seq`, `abort_msg_id`, `fault` (`header_does_not_fit_packet`, `body_runs_out_of_packets`, `length_expand_failed`), `header_len`, `header_bytes_in_packet`, `packet_len`, `abort_cursor` | `info` assembled, `debug` clean single-packet, `warn` abort |
| `client.mercury.error` | paired with every non-happy event above | `stage` (`packet`, `fragment`, `bundle`), `reason`, plus the parent event's fields | `warn` |

**Offsets** in `client.mercury.bundle` are payload offsets: every packet's bytes after its flags byte, concatenated, so a fragment boundary is a number and a message header at `abort_offset` straddling `boundaries[i]` is visible at a glance.

**Volume.** Per-packet events are emitted for every fragment, every packet while a fragment group is in flight (until 5 s after the last fragment that left a group open), every packet buffered for a gap, and every non-happy packet, all unthrottled. Ordinary traffic goes through the per-name bucket (burst 8, 4/s). A non-happy outcome of any kind bypasses every throttle, by construction: the gate is a function of the outcome and is unit-tested (`report::tests`), not a property of the hook. A clean single-packet bundle is throttled; an assembled or aborted one never is.

**Reading a partial bundle.** `client.mercury.fragment` `completed` with `count_matches = true` and `assembled_bytes` equal to what the server sent says reassembly was whole; then `client.mercury.bundle` `end` says what the message loop did with it. `exit = corrupted_header` with `fault = header_does_not_fit_packet` and an `abort_offset` a few bytes before a `boundaries` entry is the fragment-boundary header split; `exit = clean_end` with `dispatched` equal to `messages` and `unconsumed_bytes = 0` clears the Mercury layer and moves the search to the entity layer (`client.mercury.entity_method` `path = queued`). A `fragment` event with `outcome = mangled_footers` or `bundle_missing` is a fragment the client dropped after ACKing it.

**Cost and safety.** The packet filter reads each datagram once (one `ReadProcessMemory` of at most 2048 bytes); a fragment adds two group reads; the message loop adds three small reads per message on the game thread. Every read is checked, every list walk is bounded (128 nodes), every detour forwards its arguments and result untouched inside `catch_unwind`, and the bundle trace is cleared by a drop guard so a C++ exception through the message loop leaves nothing stale. `queueAckForPacket`'s four stack words are forwarded blindly (`ret 0x10`): the hook reads only the channel (`this`), so an argument-order surprise there cannot corrupt the call.

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
