# Client-Side Telemetry — Architecture

> **Diátaxis type**: explanation
> **Audience**: engineers extending or reviewing the `cimmeria-client-telemetry` DLL and its launcher-side injector (issue #417)
> **Last updated**: 2026-06-04
> **Status**: Phase 2 fully landed — 2 CME hooks (`onClientMapLoad`, `onClientReady`) + 5 inline hooks (Mercury dispatch, FEngineLoop::Tick, FArchiveAsync::Serialize, UWorld::UpdateLevelStreamingInner, UObject::StaticLoadObject). All 5 inline-hook signatures confirmed via Ghidra decompile against UE3 leaked source. `StaticLoadObject` emit captures `package_name` field for cold-load freeze investigation.

How `cimmeria-client-telemetry.dll` is side-loaded into `SGW.exe` by `sgw-launcher`, what it observes, and how those observations flow into SigNoz alongside the server-side OTLP stream.

The per-anchor hook table lives in [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md). This document is the design rationale and the stack-pick justification — read the anchor doc for "where do I hook function X?" and this doc for "why does the whole thing look like this?"

## Goal

Give server-side debuggers a client-side view of every meaningful event happening inside `SGW.exe` — frame ticks, level streaming transitions, async I/O completions, CME EventSignal dispatches, log lines, crashes — without modifying game behavior. Output lands in SigNoz under filter `service_name = cimmeria-client` so an end-to-end SigNoz trace can include both server and client spans for the same session.

The motivating use case: the cold-relog freeze investigation (2026-05-26). Server-side we can see Mercury sending N packets and getting N-3 ACKs back; what we can't see is whether the client's render thread is stuck on a disk read or which `.upk` is loading when frame ticks stop. Tier-1 hooks answer that.

## Trust model

Same machine, same user, same launcher session. The launcher is already a trusted desktop app the developer installed — DLL injection doesn't escalate privileges. The DLL ships alongside `sgw-launcher.exe` (signed at deployment time; not per-injection). No remote-code-execution surface: the DLL itself only reads from SGW.exe's memory; outbound network traffic is HTTPS to the launcher's HMAC-protected `/api/telemetry/upload-chunk` endpoint.

**Project preference (issue #417):** authored from scratch. AteraLoader.exe / AtreaRL.dll are reverse-engineering references only — their behaviour in [`docs/technical/atrearl-loader.md`](../technical/atrearl-loader.md) is useful for "what hooks work in practice," not as code we extend or wrap.

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
   - replay through tracing layer with service_name="cimmeria-client"
        |
        v   OTLP gRPC
SigNoz   (filter: service_name="cimmeria-client")
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
| Injection | `CreateProcess(SUSPENDED)` + `CreateRemoteThread(LoadLibraryW)` | Same pattern every ASI loader / ReShade / Special K uses; Defender-default-clean |
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

1. **CME EventSignal subscription** — zero-patch observation via the host's own pub-sub registration. `CmeEventSignal_Subscribe` at `0x00a5c150` does an unguarded `std::set` insert; we register fake-vtable subscribers and the host calls us. RTTI-driven auto-discovery for ~120 + ~150 distinct `Event_NetIn_*` / `Event_NetOut_*` handler classes (Phase 2).
2. **Inline (retour-rs)** — for non-event functions (frame tick, level streaming, async I/O).
3. **Vtable swap** — for `UObject::ProcessEvent`, `AActor::Tick`, `CEGUI::Logger`. Single atomic store, re-entry-safe.
4. **IAT patching** — for WinSock receive, file I/O, thread/library lifecycle. PE walk + `VirtualProtect` dance, re-scan on `LoadLibraryW` for late-binding modules.
5. **String-anchored discovery** — shipping build strips most names; log strings remain. Anchor on a string → xref → that's the function. Pre-loaded into the anchor table from existing RE work.
6. **log4cxx appender tee** — the client already uses log4cxx (`log4cxx.dll` ships, `SGWLogConfig.xml` configures it). Add a custom appender at runtime rather than patching log functions. Zero hot-path patching for the chunk of telemetry already produced as text.
7. **CEGUI::Logger interposition** — `CEGUI::Logger` is a virtual class (RTTI `0x0192c2bc`). Swap in a subclass. Captures every UI log line, button click, layout load, script error, focus event.

## Wire format

The DLL ships events as NDJSON, one event per line, gzipped, POSTed to the launcher's existing `/api/telemetry/upload-chunk` endpoint. Authentication reuses the launcher's HMAC dev-session token — the launcher passes it to the DLL at injection time (mechanism: shared-memory section, deliberately deferred to first hook PR).

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
- Server side: [`crates/admin-api/src/routes/telemetry.rs`] `client_native_event_matches_launcher_shape`

If either side renames a field, the symmetric test fails loudly.

## Service name routing

SigNoz queries against this stream filter on `service_name="cimmeria-client"`. The replay layer in admin-api emits each `ClientNative` event with `service_name = "cimmeria-client"` as a structured tracing field; the OTLP exporter forwards it as a span attribute.

**Caveat:** This is not the OpenTelemetry-spec `service.name` Resource attribute — that one is set once at server boot via `OTEL_SERVICE_NAME` and can't be overridden per-request. The tracing macro grammar in our version also doesn't accept dotted-string field keys (`"service.name"`), so we use the snake_case alias. Proper Resource-level override would require either a second `Resource` for the cimmeria-client events or a `tracing-attributes` shim that rewrites the field name on emit. Deferred — the data is the same; only the SigNoz query key differs.

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
| 2b/c/d | Tier-1 hooks — both CME EventSignal subscribes (`Event_NetIn_onClientReady`, `Event_NetIn_onClientMapLoad`) + `Mercury::Nub::handleMessage` inline hook via MinHook. Sampling counter for hot-path hooks. Phase-status update. | LANDED |
| 2-deferred | The remaining 4 tier-1 inline hooks (`FEngineLoop::Tick`, `UWorld::UpdateLevelStreaming`, `FArchiveAsync::Read*`, `LoadPackage`) need per-anchor RE to derive function entry points from their RTTI/xref anchors — see `hooks/inline_hooks.rs` module doc for the table. Scaffolding is in place; adding each is a one-liner once the entry address is known. | DEFERRED |
| 3 | CME EventSignal full coverage — RTTI walk, auto-generated `Event_NetIn_*` / `Event_NetOut_*` subscriptions | |
| 4 | Game state + animation + effects + cooked data | |
| 5 | UI / Lua / kismet / dispatcher | |
| 6 | Subsystem correlators — FMOD, Bink, PropertyNode | |
| 7 | Crash + on-disk artifact shipping | |

## CI

- `.github/workflows/client-telemetry-build.yml` — Windows-native CI for `i686-pc-windows-msvc`. fmt + clippy `-D warnings` + build (asserts `.dll` artifact lands) + nextest.
- Main `.github/workflows/test.yml` excludes the Windows-only cdylib from the Linux workspace check.
- `crates/launcher/`'s existing CI continues to cover the injector module via its own pipeline.

## Cross-references

- [`docs/reverse-engineering/findings/client-instrumentation-hookpoints.md`](../reverse-engineering/findings/client-instrumentation-hookpoints.md) — per-anchor hook table
- [`docs/reverse-engineering/findings/client-instrumentation-entry-points.md`](../reverse-engineering/findings/client-instrumentation-entry-points.md) — **resolved Phase 3-6 entry points** (companion to hookpoints — all addresses + IAT slots + signatures pre-resolved so Phase 3-6 implementation skips the RE round-trip)
- [`docs/reverse-engineering/findings/cme-event-signal.md`](../reverse-engineering/findings/cme-event-signal.md) — full CME EventSignal emit pipeline
- [`docs/architecture/observability.md`](observability.md) — the broader OTLP / SigNoz pipeline this plugs into
- [`docs/architecture/dev-session-telemetry.md`](dev-session-telemetry.md) — the launcher telemetry pipeline the DLL reuses
- [`docs/operations/telemetry.md`](../operations/telemetry.md) — operator runbook (per-category controls, opt-out, crash-dump shipping — extends with client-side toggle in follow-up)
- [`docs/technical/atrearl-loader.md`](../technical/atrearl-loader.md) — third-party RE reference (behavioural only — not code we use)
