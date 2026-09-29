# Client Telemetry — Instrumentation Seam Survey

> **Diátaxis type**: reference
> **Audience**: engineers extending `cimmeria-client-telemetry` past its current 25 hooks
> **Last updated**: 2026-09-28
> **Confidence**: HIGH for the four seams implemented this pass (re-verified against the QA `SGW.exe` via headless Ghidra decompile + disassembly). MEDIUM-to-LOW for the "needs fresh RE" candidates — addresses/strings are real but their containing functions were not decompiled this pass. The three "dead ends" entries are HIGH confidence (fully decompiled and shown to be something other than what the earlier catalog claimed).

Survey of instrumentation seams beyond the DLL's current hook set, done for the client-telemetry live-instrumentation push (issue tracked by the team). Companion to [`client-instrumentation-hookpoints.md`](client-instrumentation-hookpoints.md) (the per-tier anchor catalog) and [`client-instrumentation-entry-points.md`](client-instrumentation-entry-points.md) (the resolved Phase 3-6 manifest) — this doc covers what's *beyond* both: seams neither catalog resolved, seams the catalog got wrong, and the four new hooks this pass added.

## What changed this pass

- **`Mercury_Nub_handleMessage`** (`0x0157bd30`) — re-resolved for #989 and re-added. Every inbound Mercury message gates through it (`*msg == -1` sentinel check before dispatch), so it is a legitimate per-message choke point. See `client-instrumentation-hookpoints.md`'s anchor-correction note for the argument-count subtlety (4 stack dwords, only 3 nameable from the decompile).
- **The cooked-data PAK load anchor was investigated and NOT re-added.** `0x00420074` turned out to be a real function entry (unlike the old `0x0041f620` guess), but it is the one-time startup constructor for all ~20 cooked-data categories, not a per-load runtime event. See the "Dead ends" section below and the corrected rows in both companion docs.
- **`client.session.identity`** — a new event, not a hook. Reads the local player's entity id (the client↔server join key) through `GameEntityManager` at `onClientReady` time. See `crate::identity` in the crate and the "Join key" section below.
- **Two new inline hooks**, both `ServerConnection` message handlers on the main thread, both re-confirmed against the QA build via headless Ghidra this pass:
  - `ServerConnection_forcedPosition` (`0x00dd9ee0`) → `client.movement.forced_position`
  - `ServerConnection_createBasePlayer` (`0x00dddca0`) → `client.entity.create_base_player`

## Join key: player entity id, account name, server address

The server logs `entity_id` at world entry ("World entry: sending RESET_ENTITIES") and stamps account ids on its own events. Pairing a client-side session with the matching server-side session needs at least one shared value.

**Player entity id — implemented.** `GameEntityManager` (`g_EntityManager`, `0x01ef244c`) `[+0x08]` is `ServerConnection*`; `ServerConnection[+0x16c]` is `playerEntityID_`, confirmed by assembly evidence at two independent sites in `system-protocol-wire-formats.md` (`RESET_ENTITIES` clears it; `createBasePlayer` reads it back out). `crate::identity::read_local_player_entity_id()` reads the chain with no hook, no fingerprint gate — just three null-checked reads through [`cimmeria_client_hookgate::os::read_bytes`] (`ReadProcessMemory`, never faults). Emitted as `client.session.identity { player_entity_id }` from the existing `onClientReady` CME subscriber, which already fires exactly at "the client acks readiness to the server" — the same trigger the server pairs against `world_entry.init_player_state`.

**Account name — not implemented, and not recoverable by this pass's methods.** The `Account` entity's only base properties are `characterList` and `activePlayerID` ([`entity-types-wire-formats.md`](entity-types-wire-formats.md) §1) — `AccountName` is used once, inside the SOAP login handshake (`login-handshake.md`), and never re-appears on the wire or in any documented client-side cache. Finding it would mean tracing the gSOAP-deserialized login-reply struct's heap layout (the same struct that holds the plaintext password at `ServerConnection+0x3C`, per [`auth-and-crypto-modernization-targets.md`](auth-and-crypto-modernization-targets.md)) forward to wherever — if anywhere — the account name field survives past the login handshake. Not attempted this pass; flagging for a dedicated RE session if the join key turns out to need it (today `install_id`/`machine_id`/`session_id` from `current-session.json`, already stamped on every event, cover session identity — see [`client-telemetry.md`](../../architecture/client-telemetry.md)).

**Server address — not implemented.** Would need an IAT hook on `connect`/`WSAConnect` in `ws2_32.dll`. The technique is proven (see `iat_hooks/imports.rs`: an `Import` is a module + symbol name resolved via `GetProcAddress`, not a blind hardcoded VA, so adding one is mechanically the same as the existing 7 IAT hooks) but this exact build's IAT slot address for `connect` was not located this pass. Low effort, low risk, deferred only for lack of a probing pass — good first pick for a follow-up.

## Ranked candidates

Ranked by (value for debugging) ÷ (risk + remaining RE effort). "Status" of DONE means implemented this pass; NEW ADDRESS NEEDED means the concept is real but no function address survived verification; NOT ATTEMPTED means genuinely unexplored this pass.

| # | Seam | Value | Risk | Status |
|---|---|---|---|---|
| 1 | `Mercury_Nub_handleMessage` (Mercury dispatch) | High — per-message visibility, closes the #989 gap | Low (re-verified, fingerprinted) | **DONE** |
| 2 | `ServerConnection_forcedPosition` (server-authoritative position correction) | High — direct hit on "client thinks it's here, server disagrees" | Low (re-verified, fingerprinted, single safe read) | **DONE** |
| 3 | `ServerConnection_createBasePlayer` (local entity creation) | Medium-high — proves world entry actually created the local player, independent of CME | Low (post-hook read only, no stream interference) | **DONE** |
| 4 | `client.session.identity` (join key) | High — makes every other client event correlatable to a server session | None — no hook, defensive reads only | **DONE** |
| 5 | Mercury send-path / resend / packet loss | High — the send-side mirror of #4 in `mercury-protocol-internals.md`'s send stack (`ServerConnection__send` `0x00dd8930`, `Channel::send` `0x01576f90`) | Medium — likely hot path (every outbound message), needs a fresh decompile pass to find a resend-specific choke point rather than the generic send | NOT ATTEMPTED |
| 6 | `connect`/`WSAConnect` IAT hook (server address) | Medium — the last join-key field | Low (proven technique, just needs the IAT slot located) | NEW ADDRESS NEEDED |
| 7 | AoI enter/leave (`createEntity` 0x09 / `leaveAoI` 0x0C client handlers) | High — "what does the client think is in view" is the other half of every AoI bug investigated so far (see `docs/architecture/player-ghost-aoi-cascade.md`) | Medium — handler addresses not in any existing finding doc; needs a fresh Ghidra pass from the message-dispatch table | NOT ATTEMPTED |
| 8 | Effect start/stop CME event (`FUN_00e0a9e0`, "removes an effect from the active list, emits a CME event" per `effect-execution-model.md`) | Medium-high — ability/effect visibility for combat debugging | Low if the emitted event name resolves cleanly (CME subscribe, same technique as `onClientReady`) | NEW ADDRESS NEEDED — event name not extracted this pass |
| 9 | Cooked-data `Event_NetIn_onVersionInfo` (the corrected "PAK N loaded" signal) | Medium — the original #989 goal, properly scoped this time | Medium — CME-subscribable in principle, but each of ~20 categories is a separately template-instantiated handler; only category 6 is fully resolved (`0x00441630`) | NEW ADDRESS NEEDED (partial: 1 of ~20 categories resolved) |
| 10 | `lua_pcall` error-code decoding | Low-medium — the hook already exists (Phase 4 IAT), it just doesn't decode the int return value into an event field today | Low — no new hook, a detour edit | ENHANCEMENT (not a new seam) |

### Not ranked — no address found or explicitly out of scope this pass

From the requested survey scope, these were considered but not carried to a ranked entry, either because no plausible client-side address turned up in the existing findings corpus or because verifying one would need a dedicated fresh-RE session larger than this pass's budget:

- **Entity property updates applied** — the property-sync path is documented at the protocol level (`docs/protocol/entity-property-sync.md`) but the specific client-side "apply an incoming property write" function wasn't traced to an address in this pass.
- **UI window open/close** — partially covered already: `CEGUI::DefaultLogger::logEvent` (existing vtable hook) captures layout loads and UI log lines, which is the closest existing proxy.
- **Mission/dialog events** — `docs/reverse-engineering/findings/dialog-portrait-lookup.md` and the mission-chain docs have relevant addresses for specific sub-questions (portrait lookup, entity mapping) but no single "mission accepted"/"dialog opened" client hookpoint was identified this pass.
- **Zone/space transitions, loading screens** — already covered by the existing `UWorld::UpdateLevelStreamingInner` and `ULevelStreaming::SetLevelStatus` Tier-1 hooks; a dedicated "loading screen shown/hidden" UI-side hook wasn't investigated.
- **Frame-time hitches, memory pressure** — `FEngineLoop::Tick` (existing hook, sampled 1/100) is the load-bearing proxy; a dedicated hitch detector would need to track inter-call deltas client-side, which is an aggregation policy decision, not a new address.
- **D3D device lost** — no reference to `IDirect3DDevice9::Reset`/`TestCooperativeLevel` or a device-lost handler was found in the existing findings corpus; genuinely unexplored.
- **Audio (FMOD)** — already catalogued as deferred in `client-instrumentation-entry-points.md` (Phase 5, `FMOD_EventSystem_Create` IAT + runtime vtable traversal) — no new information this pass.
- **Input latency** — `APlayerController::execConsoleCommand` (existing hook) and the deferred `InputKey` vtable slot (also already catalogued) are the closest existing/near-term seams; a dedicated latency measurement would need two correlated timestamps (key event → applied effect), which is a design question, not just an address.

## Dead ends

Recorded so the next investigator doesn't re-spend the same Ghidra passes. Same failure class as #989 — a string that looks like a clean per-event anchor turns out to be embedded in an unrelated, much larger function.

### `BWConnection::ConnectFailure` / `NotifyConnectionLost` / `ConnectionTimeout` (Tier 2 of `client-instrumentation-hookpoints.md`)

All three were carried in the hookpoints catalog as "string-anchored discovery" targets with specific addresses (`0x0180b9f4`, `0x0182d114`, `0x018474ec`). Decompiling their containing functions this pass (headless Ghidra, 2026-09-28) shows none of the three is a focused connection-lifecycle handler:

- `0x0180b9f4` ("ConnectFailure") sits inside `FUN_0049ba90` (`0x0049ba90`), an ~860-line decompile dominated by a static-initialization guard (`TEST byte ptr [0x01ecc5f4],0x1` / `OR ... 0x1`) — a large, multi-purpose function where the string is one incidental branch, not its purpose.
- `0x0182d114` ("NotifyConnectionLost") sits inside `FUN_00511750` (`0x00511750`), which decompiles as a big class-name/RTTI **registration table** builder (repeated `PUSH name; CALL 0x0049e960; MOV [static_slot],...` blocks) — a one-time reflection-table constructor, not a per-event handler.
- `0x018474ec` ("ConnectionTimeout") sits inside `UObject__unknown_005dc280` (`0x005dc280`) — this one decompiles cleanly and is *small*, but it is UE3's **`UClass` property registration function** for `NetConnection`'s `Client` config category: it registers `ConnectionTimeout`, `InitialConnectTimeout`, `KeepAliveTime`, `RelevantTimeout`, `MaxClientRate`, and a dozen other config field *names* as reflection metadata, called once at class-init. `"ConnectionTimeout"` here is a `UProperty` name string, not a network event.

None of the three is re-proposed. A real connection-lifecycle hook, if wanted, needs to start from the actual `BWConnection`/`Mercury::Nub` class hierarchy (RTTI walk) rather than a string search — the same lesson `annotation-script-shift-bugs.md` already recorded for a different subsystem.

### Cooked-data PAK load, `0x00420074`

Covered above under "What changed this pass" and in both companion docs' corrected rows. Repeating the core fact here because it is the same failure class: a real function entry is not automatically the *right* function for the claimed purpose.

## Evidence detail for the new hooks

### `Mercury_Nub_handleMessage` — `0x0157bd30`

- **Evidence**: headless Ghidra decompile + disassembly, 2026-09-28. Ghidra's own analysis (independent of this pass) already named the function; the decompile shows the debug string is printed on a sentinel mismatch (`*msg != -1`) inside a function whose 5 exits are all `ret 0x10`.
- **Calling convention**: `__thiscall`, this in ECX, 4 stack dwords (`param_1` dead on entry, `msg`, `iface`, and an unresolved 4th slot forwarded untouched).
- **Technique**: MinHook inline, matching the project's existing 12 inline hooks.
- **Rate**: fires per inbound Mercury message — sampled 1/20.
- **Risk**: low. Prologue fingerprinted; the only memory access beyond the trampoline call is reading the same first byte the original itself reads unconditionally.

### `ServerConnection_forcedPosition` — `0x00dd9ee0`

- **Evidence**: `position-movement-wire-formats.md` (full 49-byte wire struct, `W-mercury-bible` pass) plus this pass's headless decompile/disassembly, which reconfirmed the function entry and prologue bytes against the QA build.
- **Calling convention**: `__thiscall(this, args)`, 1 stack argument.
- **Technique**: MinHook inline.
- **Rate**: rare — only on genuine server-side position corrections. Unsampled.
- **Risk**: low. Reads `*args` (the entity id, offset 0 of the struct) before the trampoline call — the same field the original function's own first real read touches, so no new invariant.

### `ServerConnection_createBasePlayer` — `0x00dddca0`

- **Evidence**: `entity-creation-wire-formats.md` (debug string `"ServerConnection::createBasePlayer: id %d\n"` at `0x019d00e0`) plus this pass's decompile/disassembly.
- **Calling convention**: `__thiscall(this, stream)`, 1 stack argument. `stream` is a live cursor — the original reads the entity id and class id off it via a vtable call (`(**(code**)(*stream+4))(4)`) that **advances the stream**.
- **Technique**: MinHook inline, **post-hook** read: the detour calls the trampoline first (so the original's own stream read happens exactly once), then reads `ServerConnection::playerEntityID_` (`this+0x16c`) — the field the original just finished writing — rather than peeking the stream itself, which would double-consume it and corrupt the real parse.
- **Rate**: once per world entry.
- **Risk**: low, but this is the one hook this pass added where getting the ordering wrong (pre- vs post-hook) would have introduced a real bug rather than just bad telemetry — worth flagging for reviewers.

## Related docs

- [`client-instrumentation-hookpoints.md`](client-instrumentation-hookpoints.md) — per-tier anchor catalog (corrected rows for #989)
- [`client-instrumentation-entry-points.md`](client-instrumentation-entry-points.md) — resolved Phase 3-6 manifest (corrected cooked-data PAK load row)
- [`cme-event-signal.md`](cme-event-signal.md) — CME EventSignal framework, the technique behind `client.session.identity`'s trigger point and every deferred CME-subscribe candidate above
- [`cooked-data-pipeline.md`](cooked-data-pipeline.md) — the real per-category runtime version-negotiation flow (`Event_NetIn_onVersionInfo`) that replaces the old "PAK N loaded" plan
- [`position-movement-wire-formats.md`](position-movement-wire-formats.md), [`entity-creation-wire-formats.md`](entity-creation-wire-formats.md) — source evidence for the two new hooks
- [`../../architecture/client-telemetry.md`](../../architecture/client-telemetry.md) — design doc, updated phasing for this pass's additions
- [`annotation-script-shift-bugs.md`](annotation-script-shift-bugs.md) — the recurring "wrong anchor in an otherwise-plausible table" failure mode this survey's dead-ends section is another instance of
