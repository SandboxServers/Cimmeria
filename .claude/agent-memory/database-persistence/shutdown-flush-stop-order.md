---
name: shutdown-flush-stop-order
description: How player state survives a server stop (TG-DB-02): ShutdownFlush round trip, 10 s deadline, container grace settings, and how to live-DB test stop_all
metadata:
  type: project
---

Since TG-DB-02 (2026-10-10, D-TG15) a stop saves in-world players: the cell loop's shutdown arm sends one
`CellToBaseMsg::ShutdownFlush` and awaits the reply; the base's cell-message task (still alive, still holding the
pool) handles it inline and saves each player in a JoinSet task until `SHUTDOWN_FLUSH_DEADLINE` (10 s).
`stop_all` needs no extra wait: `cell.stop()` returns only after the reply. TG-DB-01's
`player_state_not_persisted` WARN now fires only for players missing from the reply.

**Why:** colo deploys used to lose every in-world player's position and dirty ammo.

**How to apply:**
- Watchtower ignores `stop_grace_period`; it uses `WATCHTOWER_TIMEOUT` (default 10 s). Both are 45 s in
  `docker/compose.yml`; any longer shutdown work must fit inside them.
- A services-crate live-DB test of `start_all`/`stop_all` must chdir to the repo root first: `CellService` reads
  `entities/spaces.xml` relative to CWD, else every world is "Unknown". See
  `crates/services/src/shutdown_flush_live_db_tests.rs` (sentinels 0x7000_1F10..12).
- Use a second `CreateEntity` (it has a reply) as a barrier after `ConnectEntity`: the cell loop's `select!` may
  take the shutdown notify before queued messages.

Related: [[live-db-verification]]
