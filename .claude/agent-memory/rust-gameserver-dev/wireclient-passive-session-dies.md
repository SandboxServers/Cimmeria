---
name: wireclient-passive-session-dies
description: A listen-only wireclient GameSession never sends or acks (LoopbackPeer acks only piggyback, its keepalive reads a TestClock nobody advances) and is reaped after 60 s; use sparbot::run's AUTHENTICATE heartbeat for any long-lived bot or test.
metadata:
  type: project
---

A `GameSession` (crates/wireclient) that only receives dies after about 60 s. This was found in SS-U2 on 2026-09-27.

- `LoopbackPeer` queues acks and sends them only piggybacked on the next outbound packet.
- Its `tick()` keepalive reads the injected `TestClock`, which `GameSession` never advances, so it never fires.
- The server reaps a client silent for 60 s (`base-session` `tick_sync.rs` `INACTIVITY_TIMEOUT`).

**Why:** the NA37 tests finish in seconds, so nobody hit this until a bot had to sit in the world.

**How to apply:** for anything that holds a session longer than a few seconds, send an unreliable `GameSession::authenticate()` every ~250 ms. The real idle client does the same at about 6/s, and the packet carries the acks. `sparbot::run` already does this. A mutation proved it: without the heartbeat, `sparbot_session_outlives_the_inactivity_reap` fails with `ServerSilent`. Related: `reference_client_idle_send_cadence.md` in main-session memory.
