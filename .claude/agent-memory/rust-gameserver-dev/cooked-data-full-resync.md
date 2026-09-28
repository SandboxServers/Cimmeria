---
name: cooked-data-full-resync
description: #840 design — mismatch = paced full-category resync (placeholder version first, real last, RequiredUpdates=0), misses served ahead of the stream, Play held only for no-miss-path categories; client cache RE facts
metadata:
  type: project
---

Since #840 (PR #1035, 2026-09-28), `crates/base-session/src/base/cooked_sync/` owns every cooked-data
version mismatch and every `elementDataRequest`:
- Resync: `onVersionInfo(InvalidateAll=1, RequiredUpdates=0, Version=!served)`, every entry, then
  `onVersionInfo(0,0,[],served)`. Paced at <= 24 outstanding reliable packets per session.
- Misses (`0xC1` pre-world, SGWPlayer `0xD5` in-world, `[i32 cat][i32 key]`) go out as the next transfer
  ahead of the stream. Rate limit: burst 100, 50/s, queue cap 256.
- Play waits only for `HELD_CATEGORIES` = 12, 16, 17, 18, 20, 21. Stream order: held set, then 3, 5, 4,
  then the rest, TextStrings (10) last.

**Why (client RE, headless Ghidra):**
- `onVersionInfo` (`0x00441630`) deletes every cache entry on InvalidateAll and stamps `Version` before
  any entry arrives, so the real version must go last.
- The per-category miss request fires only while `ServerSource+0x48` (RequiredUpdates) == 0 (e.g.
  `0x00cfe060`), so the resync sends 0.
- Miss-request emitters (callers of ctor `0x00cfdeb0`) exist for 1-11, 13, 14, 15, 19, and none for
  12, 16, 17, 18, 20, 21: those six are held.
- `0x0043bdb0` WRITES an entry; the V5 "RequestElement" name was wrong.

**How to apply:**
- In-world, `onVersionInfo` is SGWPlayer client method 96 (`build_version_info_to_player`, `0xBD` sub 35).
  `0x80` to the player is SGWPlayer method 0. Pre-world it is the Account's `0x80`.
- In-world `0xC0`/`0xC1` are chatJoin/chatLeave; the encrypted loop gates the cache arms on
  `!cooked_sync::in_world`. The old misroute read the client's login `/chatjoin channel-*` as
  category 12/16 and wiped 16 every login. User channels: #1039.
- Tests: the registry is a global keyed by addr, so each test needs its own port. Under `cfg!(test)` the
  task polls with `Duration::ZERO` (a yield). The task waits for window room BEFORE choosing its next
  send; otherwise a miss lands behind the entry it was already blocked on.
- The def-conformance scan (`crates/wire/src/mercury/def_conformance/base_methods.rs` BASEMSG_FILES)
  reads BASEMSG_* consts from named test files. Moving or deleting such a file panics that test.
- Headless Ghidra can decompile a bare label: `disassemble(addr)` then `createFunction(addr, null)`.
