---
name: client-telemetry-seam-survey-2026-09-28
description: Issue #989 resolution (Mercury dispatch re-added, cooked-data PAK load correctly re-diagnosed and left out), the join-key read (GameEntityManager -> ServerConnection -> playerEntityID_), two new ServerConnection hooks, and three misattributed BWConnection lifecycle string anchors
metadata:
  type: project
---

Branch `re/client-telemetry-seams` (off `feat/client-telemetry-live`), worktree `client-seams-re`. Full writeup:
`docs/reverse-engineering/findings/client-telemetry-seam-survey.md`.

**#989 Mercury dispatch — RESOLVED.** `0x0157bd30` (Ghidra already named it `Mercury_Nub_handleMessage`
independently of this pass) is the real per-message dispatch gate: `*msg == -1` sentinel check before
falling through to the handler-interface dispatch. `__thiscall`, this in ECX, **4 stack dwords** confirmed
by `ret 0x10` — but the decompiler could only name 3 (`param_1` is dead on entry — reassigned before ever
read; `param_2` is the message struct; `param_3` is a handler-interface pointer). The 4th slot is real
(the `ret 0x10` is unambiguous) but nothing in the decompiled body touches it — forwarded untouched in the
detour. Re-added, fingerprinted, sampled 1/20.

**#989 cooked-data PAK load — NOT re-added, correctly re-diagnosed.** `0x00420074` (the old "nearest
padding entry" guess) IS a real function entry after all (Ghidra names it `Detail__unknown_00420074` and
decompiles it cleanly) — but it's the **one-time startup constructor** for all ~20
`LibCategory<LibCategoryKey<N,...>>` descriptors (builds `CookedDataKismetSetEvent.pak`,
`CookedDataKismetSeqEvent.pak`, etc. once at launch). Hooking it gives zero per-load signal. The OLD
`0x0041f620` "nearest padding" guess (from a prior byte-scan heuristic, not Ghidra's own function boundary)
is a completely unrelated `LaunchMisc.cpp` engine-init function (shader/XML config), nothing to do with
cooked data — **lesson: when re-resolving a "nearest entry before X" anchor, always ask Ghidra's own
`getFunctionContaining`/`D:` token first, not a manual byte-scan; Ghidra had the right function
(`0x00420074`) the whole time.** The real per-category runtime signal is `Event_NetIn_onVersionInfo`
(CME-subscribable, [[cooked-data-pipeline]] Finding 4) but it's template-instantiated once per category —
only category 6 is fully resolved (`ServerSource_onVersionInfo_Handler_cat6` @ `0x00441630`). Needs the
deferred CME RTTI auto-discovery scanner, or per-category manual resolution, to cover all ~20.

**Join key implemented: player entity id, not account name/server address.** Chain:
`g_EntityManager` (`0x01ef244c`, fixed VA holding a pointer) `-> [+0x08]` = `ServerConnection*`
`-> [+0x16c]` = `playerEntityID_` (u32, 0 = none). Cross-validated by TWO independent pre-existing finding
docs with assembly evidence at different call sites: `black-market-client-io.md` (`conn = [[GEM]+0x08]`,
BM send path) and `system-protocol-wire-formats.md` (`ServerConnection` field map — `RESET_ENTITIES` clears
`+0x16c` on `keepBase==false`, `createBasePlayer` reads it back out as `entityId`). No hook needed — a
defensive 3-level read via `cimmeria_client_hookgate::os::read_bytes` (`ReadProcessMemory`, never faults),
emitted as `client.session.identity` from the existing `onClientReady` CME subscriber.
Account name is **provably unrecoverable** from documented client memory: the `Account` entity's only base
properties are `characterList`/`activePlayerID` ([[entity-types-wire-formats]] §1) — `AccountName` is used
once in the SOAP login handshake and never re-appears on the wire or in any cache. Server address needs a
`connect`/`WSAConnect` IAT hook (technique proven, IAT slot for this exact build not located this pass —
cheap follow-up).

**Two new hooks, both `ServerConnection` message handlers, both already had strong evidence (debug format
strings, not bare name strings) in pre-existing finding docs** — much safer to verify than a fresh string
search:
- `ServerConnection_forcedPosition` (`0x00dd9ee0`) -> `client.movement.forced_position`. Pre-hook read of
  `*args` (entity id) — safe because it mirrors the original's own first real read.
- `ServerConnection_createBasePlayer` (`0x00dddca0`) -> `client.entity.create_base_player`.
  **Post-hook** read of `this+0x16c` (same `playerEntityID_` field as the join key) — the original consumes
  its entity id off a **live stream cursor** via a vtable call that advances the stream, so a pre-hook peek
  would have double-consumed those bytes and corrupted the real parse. Call trampoline first, then read.

**Dead ends — three BWConnection lifecycle string anchors in `client-instrumentation-hookpoints.md`
Tier 2 are misattributed**, same failure class as #989's original wrong anchors (a string that looks like
a clean per-event marker turns out to be inside an unrelated giant function):
- `ConnectFailure` (`0x0180b9f4`) -> inside `0x0049ba90`, an ~860-line function dominated by a
  static-init guard (`TEST/OR [0x01ecc5f4],0x1`) — multi-purpose, not a connection handler.
- `NotifyConnectionLost` (`0x0182d114`) -> inside `0x00511750`, a one-time class-name/RTTI
  **registration-table** builder (repeated `PUSH name; CALL 0x0049e960; MOV [static],...` blocks).
- `ConnectionTimeout` (`0x018474ec`) -> inside `UObject__unknown_005dc280` (`0x005dc280`) — small and
  clean-decompiling, but it's UE3's `UClass` **property-registration** function for `NetConnection`'s
  `Client` config category (registers `ConnectionTimeout`/`InitialConnectTimeout`/`KeepAliveTime`/etc. as
  `UProperty` name strings, called once at class-init). The string is a property NAME, not an event.

None re-proposed. A real connection-lifecycle hook needs an RTTI class-hierarchy walk from
`BWConnection`/`Mercury::Nub`, not a string search.

Headless Ghidra recipe used throughout: [[headless-ghidra-decompile-workaround]]. All probes ran against
`C:\Users\Steve\source\projects\SGW\Stargate Worlds-QA\Working\binaries` (project `SGW`), Ghidra install
`C:\ghidra_12.0.4_PUBLIC`. `D:<addr>` (decompile, uses `getFunctionContaining` — trust this over a manual
byte-scan for "nearest entry") and `X:<addr>` (xrefs) did all the work; no `PTR:`/`FINDPTR:` needed this
time.
