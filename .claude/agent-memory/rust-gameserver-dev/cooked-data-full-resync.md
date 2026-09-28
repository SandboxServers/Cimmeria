---
name: cooked-data-full-resync
description: #840 design — any cooked-data version mismatch is a paced full-category resync (placeholder version first, real version last); in-world 0xC0/0xC1 are chatJoin/chatLeave; client cache handler facts
metadata:
  type: project
---

Since #840 (PR #1035, 2026-09-28), `crates/base-session/src/base/cooked_sync/` owns every
`versionInfoRequest` mismatch: opening `onVersionInfo(InvalidateAll=1, RequiredUpdates=N,
Version=!served)`, N entry transfers paced at <= 24 outstanding reliable packets
(`SYNC_IN_FLIGHT_BUDGET`), then `onVersionInfo(0, 0, [], served)`. `playCharacter` is held
(`defer_until_synced`) until the session's resyncs finish. The per-key InvalidKeys path is gone.

**Why:** the user wants the client to hold exactly the server's category. The client handler
(`0x00441630`) deletes every cache entry on InvalidateAll and stamps `Version` BEFORE any entry
arrives, so the real version must go last (reliable ordering) or a mid-push disconnect looks
up to date. Pushed entries are applied as they arrive (`0x0043dad0`); `0x0043bdb0` WRITES an entry
(the V5 "RequestElement" name was wrong). `RequiredUpdates` counts entries, not fragments.

**How to apply:**
- In-world `0xC0`/`0xC1` are `SGWPlayer.chatJoin`/`chatLeave`; the encrypted loop gates the cache
  arms on `!cooked_sync::in_world`. The client's login `/chatjoin channel-chat|roleplay|alliance`
  used to be misread as category 12/16 at version 0x00680063 and wiped category 16 every login.
  User channels themselves are unimplemented (#1039).
- Tests: the resync registry is a global keyed by addr, so each test needs its own port.
  `cooked_sync::context` polls with `Duration::ZERO` (a yield) under `cfg!(test)`; a
  full 21-category resync (57.7k packets) runs in about 3 s in the unit tests.
- Headless Ghidra can decompile a bare label: `disassemble(addr)` then `createFunction(addr, null)`
  inside the probe script (read-only project, changes discarded).
