---
name: voicedata-handler-and-recv-loop-stall
description: ServerConnection+0x168 is pHandler_ (non-null in world), so voiceData 0x33 is NOT a null-handler no-op; the BaseApp recv loop awaits handlers inline, so any sleep in a handler stalls every session.
metadata:
  type: reference
---

**voiceData (0x33) is forwarded in world, not dropped.** `ServerConnection_voiceData` (`0x00dd68b0`) tests `this+0x168` and calls its vtable `+0x34` (slot 13). The same `this+0x168` is the handler `createBasePlayer`, `createCellPlayer`, `forcedPosition`, `spaceData`, `spaceViewportInfo` and `restoreClient` call, and `ServerConnection_logOn` stores it from its third argument (`decompiled/14_standalone_named.c:302393`). So it is stock BigWorld's `pHandler_` (SGW: `SGWMessageQueue`, per `client-entity-lifecycle.md`), set from login on. What slot 13 does (probably `onVoiceData`, possibly an empty default) is unverified: check in Ghidra before using 0x33 as a harmless filler. Docs saying "SGW registers no voice handler" (`space-viewport-wire-formats.md` VOICE_DATA) mix up "no voice subsystem" with "null handler". Found reviewing lab-chaos CH-07 (2026-10-11).

**BaseApp inbound is one task.** `crates/base/src/base/connect_loop/mod.rs` `run_connect_loop` awaits `handle_datagram` inline for every client. A handler that sleeps (for example a `tokio::time::sleep` inside `dispatch_deferred` reached from `onClientReady` or `cancelMovie`) stops inbound processing for every session. Tick-sync is a separate per-session task, so the client's 15 s `NetInactivityTimeout` stays fed, but the server's own 60 s `last_recv` reap (`base-session/src/base/tick_sync.rs`) does not. Timer work belongs in a spawned task or a hold, like `cinematic_aoi_hold::arm_timeout`.

Related: [[interface-element-length-escape]], [[na38-client-orders-reliable-stream]].
