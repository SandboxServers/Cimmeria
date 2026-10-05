# Client Patches DLL

> **Diátaxis type**: explanation (ADR)
> **Audience**: engineers extending `cimmeria-client-patches`, or deciding where a new client-side fix belongs
> **Last updated**: 2026-10-04
> **Status**: Accepted (decision D2 of the [Black Market plan](../analysis/black-market/README.md), 2026-09-26). The Black Market receive and send paths are built, and verified only statically and by unit tests. The launcher injects the DLL on every `SGW.exe` launch (BM-06), and the UI overlay lives in `crates/client-patches/overlay/` (BM-05).

## Context

Some features in the 2009 `SGW.exe` were shelved half-built. The Black Market
is the clearest case. The client parses all six `onBM*` client methods (90–95)
and resolves each to a `MethodDescription`, but nothing was ever bound to
them, so the dispatcher drops every one silently. The client-to-server
emitters were never wired up either. The server can be complete and the
feature still cannot work without native code in the client.

Before this, the only native code injected into the client was
`cimmeria-client-telemetry`. That DLL observes the game. It is injected only
when the player opts into telemetry, and its lab-bridge feature is built only
for research sessions.

## Decision

Client fixes that gameplay needs live in their own DLL,
`cimmeria-client-patches` (`crates/client-patches/`). The launcher injects it
on every launch, whether or not telemetry is on.

- **Separate from telemetry.** A player who opts out of telemetry never loads
  the telemetry DLL, and a gameplay feature must not depend on a telemetry
  toggle.
- **Same shape as telemetry:**
  - an i686 `cdylib` plus an `rlib`;
  - a bootstrap thread started from `DllMain`;
  - MinHook detours;
  - every hook and FFI call behind `cfg(all(windows, target_arch = "x86"))`;
  - portable logic unit-tested on the host;
  - its own i686 CI workflow.
- **One codec for both ends.** Wire payloads are encoded and decoded by
  `cimmeria-patch-wire`. That crate is std-only, with no dependencies at all,
  so the DLL can link it, and the server links the same crate. An
  argument-order mismatch then fails a unit test rather than a play session.
  `cimmeria-wire` is too heavy to inject: it pulls in tokio, Mercury and,
  through the workspace-hack, sqlx.
- **A build fingerprint gate.** Every address belongs to one `SGW.exe` build.
  The DLL checks the prologue bytes at each one before hooking anything, and
  installs nothing on a mismatch. A different build then loses the feature
  instead of crashing.
- **Claim by name, on the drop path only.** The receive hooks act only when
  the dispatcher has already decided to drop a call, only for the method
  names they own, and only for the local player. Everything else passes
  through untouched.
- **Thread discipline.** The network thread only reads bytes and queues
  typed values. Lua and engine calls happen on the main thread, in the
  `FEngineLoop::Tick` detour. Every Lua call, including building the
  arguments, runs inside `lua_cpcall`, and the handler under `lua_pcall`.
- **Send below the event system.** The client-to-server path does not
  revive the client's shelved CME NetOut emitters. The DLL registers its
  own native Lua functions in a table it owns, `CimmeriaBMNative`, and each
  one calls `ServerConnection::startEntityMessage`, the call every working
  NetOut ends in, and writes the payload itself. It runs only on the main
  thread, only while the connection is online, and under a
  structured-exception guard. A send native reports failure as a return
  value, `nil` plus a reason, and never raises a Lua error.

## How it fits together

```text
server ──onBM* 90–95──▶ dispatcher detour (records entity, stream)
                          └─ drop-callee detour: name is onBM*? local player?
                               └─ decode with cimmeria-patch-wire ─▶ bounded queue
FEngineLoop::Tick detour (main thread) ◀── queue
   ├─ lua51.dll C API: CimmeriaBM.onAuctions(items, totalResults, clientKey) …
   │    └─ UI overlay (Lua): store, views, rows
   └─ every 30 frames: make sure CimmeriaBMNative is registered

UI overlay button ─▶ CimmeriaBMNative.bid(sequenceId, bidAmount)   (main thread)
   └─ validate, encode with cimmeria-patch-wire, check the connection
        └─ startEntityMessage(conn, 0x3D, 0); reserve(1) = sub-index; reserve(n) = payload
             └─ server ◀──cell 61–66── (0xBD, sub-index, payload)
```

The crate README states both Lua contracts the overlay builds against: the
[receive contract](../../crates/client-patches/README.md#the-lua-contract-for-the-ui-overlay)
(`CimmeriaBM.on*`, called by the DLL) and the
[send contract](../../crates/client-patches/README.md#the-send-contract-for-the-ui-overlay)
(`CimmeriaBMNative.*`, called by the overlay).
The addresses and their evidence are in
[black-market-client-io.md](../reverse-engineering/findings/black-market-client-io.md).

## How the launcher loads it

The launcher side is BM-06; the operator detail is in
[sgw-launcher.md](../client/sgw-launcher.md#client-patches-dll).

- **Always, with an opt-out.** **Launch SGW.exe** starts the game
  suspended, injects the DLL and resumes it. The checkbox **Load client
  patches** (`client_patches.enabled`, on by default) turns it off; the
  telemetry opt-in has no effect on it. A launch without the DLL says
  why in the launcher's status log.
- **Shipped inside the launcher.** The release workflow builds the i686
  DLL and embeds it in the launcher, which writes it to a
  content-addressed directory beside itself at launch.
- **Injected through a 32-bit helper.** The launcher stays 64-bit.
  Injection hands a remote thread the injector's own `LoadLibraryW`,
  which only exists at the target's bitness, and a 64-bit process
  cannot reach the 32-bit one in `SGW.exe` (a suspended WOW64 process
  has no 32-bit kernel32 mapped yet, measured, and a thread it starts
  there runs in 64-bit mode). So the launcher runs `sgw-start32.exe`, a
  small i686 helper (crate `cimmeria-start32`) it also embeds and keeps
  at a stable path beside itself: it starts `SGW.exe` suspended,
  injects the DLLs in order, resumes, and reports the pid, which the
  launcher follows. The contract is in the
  [client-launch README](../../crates/client-launch/README.md). A direct
  injection across bitness is refused with `BitnessMismatch`.
- **Order.** When the telemetry DLL goes in as well, this DLL goes
  first (`injection_order`). It hooks at once and normally meets the
  stock prologues, so it seldom needs the chain rule; the telemetry DLL,
  which has no gate, chains on top later.
- **Telemetry.** With telemetry on, the launcher reads this DLL's log
  and records one `client.patches.boot` event per session: the
  injection outcome, the DLL version, the fingerprint result per site
  and whether the hooks went in. It also records `client.patches.counts`
  (at most once a minute while a count moves, and at game exit): the
  claimed / delivered / dropped counters taken from the `(#n)` of the
  per-call lines, with the last reason per counter. The log lines it
  parses are a contract, listed in the crate README.
- **The UI overlay** ships as a manifest patch with `"root":
  "sgw_game"`, extracted into the client's `SGWGame/` directory, packed
  by `pack-client-overlay` in the launcher release.
- **Atera debug launches** do not load it: the bat starts `SGW.exe`
  itself.

## Consequences

- **Two DLLs hook two of the same functions:** `FEngineLoop::Tick` and the
  drop callee. Both use MinHook, which relocates an existing `E9 rel32` into
  its trampoline, so whichever DLL hooks second chains onto the first. The
  gate accepts a leading `E9` at those two sites only when the bytes after it
  are intact and the jump lands inside the loaded telemetry DLL's image.
  The two MinHook copies do not coordinate, so this DLL re-reads a
  prologue after MinHook has copied it and rebuilds the hook if the bytes
  changed. The launcher injects the DLLs one after the other, this one first.
  Neither DLL may unhook while the other is loaded, because MinHook's unhook
  restores its own saved bytes over a hook chained on top.
- **One DLL hooks at a time, in either load order.** Both DLLs link
  `cimmeria-client-hookgate`, which holds the chaining rule, the list of
  hook-owner modules (each DLL accepts the other's jumps, under the
  launcher's and cargo's file names) and a per-process named mutex
  (`Local\cimmeria-client-hooks-<pid>`). Each DLL holds the mutex from the
  moment it lists the loaded hook owners until its last hook is live. So
  whichever DLL goes second sees the first one's jump and chains onto it,
  and the first one's image is already loaded when the second lists the
  owners. Before this, the patches DLL could list the owners, then see a
  telemetry hook appear before its own install and refuse it as an
  unknown hook, leaving the Black Market off. The lock waits up to 10
  seconds; after that the DLL logs `install lock unavailable` and installs
  nothing (the Black Market stays off for that session), because hooking
  while the other DLL is mid-install could lose one of the two detours.
  The telemetry DLL and the lab bridge's dynamic hooks follow the same
  rule (`LockOutcome::permits_hooking`).
- **A build that does not match fails at once.** Before waiting for
  `lua51.dll`, the DLL reads every site once; any site that is neither
  stock nor an `E9` jump (whether that jump may be chained is decided
  later, under the lock) ends the boot with the same "nothing installed"
  line the launcher parses. `crates/sgw-testhost`, a 32-bit stand-in for
  `SGW.exe`, pins this: the DLL, injected through `sgw-start32`, logs
  every site, installs nothing, and the host exits cleanly
  (`client-dll-boot.yml`).
- **The receive path is generic.** "A shelved client method, matched by name,
  decoded in Rust and forwarded to Lua" works for any method the telemetry
  DLL's drop oracle reports. A new feature adds its names, a codec in
  `cimmeria-patch-wire`, and a Lua plan.
- **Name matching is the safety net against index drift.** If the server
  ever sends a Black Market payload under the wrong index, the method name
  will not match, and the call is dropped as before rather than misdecoded.
- **The fingerprint gate covers the send side too.** Besides
  `startEntityMessage` itself, it checks four small engine functions whose
  bytes contain the data offsets the send side reads: the
  `GameEntityManager` address, the online flag at `ServerConnection +
  0x30c`, the bundle's `reserve` vtable slot, and the engine's own
  `(conn, idx, 0)` call. A build that moved any of them installs nothing,
  receive included.
- **Tech competency stays blank for now (D7).** The plan decided on a
  native getter for the item definition's tech competency. A static read of
  the client's `getItemDefInfo` binding shows the lookup goes through the
  cooked-data cache (`FUN_00ae7180` → `FUN_004786f0` singleton →
  `FUN_00ae6c10` → `FUN_00ae5470` → `FUN_00d283e0`), on a miss calls what appears to be a load
  (`FUN_00ae4110`) for a definition the client has not cached, and returns
  a pointer reference-counted at `+0x04` and released through its vtable.
  Calling that chain unverified risks a crash in exactly the way the
  earlier Black Market attempts crashed, so `techCompetency` returns `nil`
  until a live check confirms the chain; the evidence doc lists it.
- **The DLL is not a telemetry channel.** It logs to `OutputDebugString` and
  to a file next to `SGW.exe`, and sends nothing to the server.
- **A 32-bit executable with "patch" in its name trips Windows' installer
  detection.** The crate's `build.rs` embeds an `asInvoker` manifest so the
  i686 test harness starts without elevation. `cimmeria-patch-wire` does the
  same.

## A runtime repair: `strstream` under Wine

One thing the DLL does is not a shelved feature. Under Wine, the client
cannot read its own cooked-data cache: it reads each archive entry through a
`std::strstream`, writing the bytes in and reading them straight back, and
`strstreambuf::underflow` in Wine's `msvcp80.dll` returns end of file for a
read that follows a write. The client does not check, keeps version 0 for
every cooked category and is resynced in full at every login, about ten
minutes each time. The mechanism and its evidence are Finding 9 of
[cooked-data-pipeline.md](../reverse-engineering/findings/cooked-data-pipeline.md#finding-9--under-wine-the-version-read-gets-no-bytes-back-from-strstreambuf).

The repair (`strstream_underflow/`) lives here because gameplay needs it and
it has to be in the process before the cache is opened. It runs on the
bootstrap thread straight after the first fingerprint check, before the wait
for `lua51.dll`, and does not depend on the Black Market installing.

- **Only under Wine.** It looks for `wine_get_version` in `ntdll.dll`. On
  Windows it does nothing and logs one line.
- **Only if a probe fails.** It builds a `strstreambuf(0)` through the
  runtime's own exports, writes four bytes and reads four back. A runtime
  that returns them is left alone, so a Wine that fixes the function, or a
  prefix that loads Microsoft's `msvcp80.dll`, is never touched.
- **Only if the object looks as expected.** After the probe's write, the put
  area must hold four bytes and the high-water mark must still be at the
  buffer's start. Any other shape is logged and left alone.
- **One pointer is written.** The entry of `strstreambuf`'s vtable that holds
  the address the DLL exports as `?underflow@strstreambuf@std@@MAEHXZ` is
  pointed at a shim. The entry is found by that address, not by an index.
  No code is patched and MinHook is not involved.
- **The shim does what Microsoft's function does first.** It raises
  `_Seekhigh` (`+0x44`) to the put pointer when writes have passed it, then
  calls the runtime's own `underflow`, which is correct once the mark is
  right.
- **It checks itself.** The probe runs again after the swap. If the round
  trip still fails, the slot is put back.

This is the only place the DLL writes outside `SGW.exe`, and it uses no
`SGW.exe` address, so the fingerprint table does not list it. It is not
specific to the desktop launcher or to macOS: any launcher that injects this
DLL into a client under Wine gets it.

It has run once against the real client (2026-10-05, the pinned Wine runtime on macOS): the probe failed, the shim went in, all 42 version reads at start returned real versions, and the login that followed needed no resync. The detail is in Finding 9.

It is a workaround for a fault that belongs upstream. The launcher's Wine
runtime is a third-party release pinned by hash, so it cannot carry a patch;
when a pinned runtime has the function fixed, the probe passes and the shim
stops being installed.

## Alternatives considered

| Option | Why not |
|---|---|
| Fold the patches into the telemetry DLL | Gameplay would depend on the telemetry opt-in (D2). |
| The launcher writes code caves into the process | One hand-assembled cave per method, a wire parser in assembly, and nothing testable off the client. |
| Revive the client's own CME event binding | Four live crashes during the earlier attempts; see the Black Market plan §4. |
| Patch `SGW.exe` on disk | Distributes a modified executable, and still needs a parser written in assembly. |
