# Client Patches DLL

> **Diátaxis type**: explanation (ADR)
> **Audience**: engineers extending `cimmeria-client-patches`, or deciding where a new client-side fix belongs
> **Last updated**: 2026-09-27
> **Status**: Accepted (decision D2 of the [Black Market plan](../analysis/black-market/README.md), 2026-09-26). The Black Market receive path is built. Sending, the launcher's always-inject step and the UI overlay are later packets.

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

## How it fits together

```text
server ──onBM* 90–95──▶ dispatcher detour (records entity, stream)
                          └─ drop-callee detour: name is onBM*? local player?
                               └─ decode with cimmeria-patch-wire ─▶ bounded queue
FEngineLoop::Tick detour (main thread) ◀── queue
   └─ lua51.dll C API: CimmeriaBM.onAuctions(items, totalResults, clientKey) …
        └─ UI overlay (Lua): store, views, rows
```

The crate README states the Lua contract the overlay builds against:
[crates/client-patches/README.md](../../crates/client-patches/README.md#the-lua-contract-for-the-ui-overlay).
The addresses and their evidence are in
[black-market-client-io.md](../reverse-engineering/findings/black-market-client-io.md).

## Consequences

- **Two DLLs hook two of the same functions:** `FEngineLoop::Tick` and the
  drop callee. Both use MinHook, which relocates an existing `E9 rel32` into
  its trampoline, so whichever DLL hooks second chains onto the first. The
  gate accepts a leading `E9` at those two sites only when the bytes after it
  are intact and the jump lands inside the loaded telemetry DLL's image.
  The two MinHook copies do not coordinate, so this DLL re-reads a
  prologue after MinHook has copied it and rebuilds the hook if the bytes
  changed. The launcher should still inject the DLLs one after the other.
  Neither DLL may unhook while the other is loaded, because MinHook's unhook
  restores its own saved bytes over a hook chained on top.
- **The receive path is generic.** "A shelved client method, matched by name,
  decoded in Rust and forwarded to Lua" works for any method the telemetry
  DLL's drop oracle reports. A new feature adds its names, a codec in
  `cimmeria-patch-wire`, and a Lua plan.
- **Name matching is the safety net against index drift.** If the server
  ever sends a Black Market payload under the wrong index, the method name
  will not match, and the call is dropped as before rather than misdecoded.
- **The DLL is not a telemetry channel.** It logs to `OutputDebugString` and
  to a file next to `SGW.exe`, and sends nothing to the server.
- **A 32-bit executable with "patch" in its name trips Windows' installer
  detection.** The crate's `build.rs` embeds an `asInvoker` manifest so the
  i686 test harness starts without elevation. `cimmeria-patch-wire` does the
  same.

## Alternatives considered

| Option | Why not |
|---|---|
| Fold the patches into the telemetry DLL | Gameplay would depend on the telemetry opt-in (D2). |
| The launcher writes code caves into the process | One hand-assembled cave per method, a wire parser in assembly, and nothing testable off the client. |
| Revive the client's own CME event binding | Four live crashes during the earlier attempts; see the Black Market plan §4. |
| Patch `SGW.exe` on disk | Distributes a modified executable, and still needs a parser written in assembly. |
