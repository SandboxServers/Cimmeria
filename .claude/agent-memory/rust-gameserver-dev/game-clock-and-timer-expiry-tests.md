---
name: game-clock-and-timer-expiry-tests
description: The client game clock is TICK_SYNC.gameTime / hertz; timer expiries use game_clock::game_time_secs(); tests of an absolute expiry must wait until the clock is past its epoch (f32 rounding hides a relative expiry)
metadata:
  type: project
---

`crates/wire/src/mercury/game_clock/` is the one server clock (CR-02, 2026-09-26):
epoch pinned in `Orchestrator::new`, 10 ticks/s, `tickRate` = 100 **ms per tick**. The
client clock is `TICK_SYNC.gameTime / hertz` (SGW.exe `FUN_00dd6c60`). Any
`onTimerUpdate` start sends `game_time_secs() + duration`; `0.0` clears.

Test trap: the epoch starts at the first read in the test process. A window check
`before + d <= expiry <= after + d` passes a regressed *relative* expiry `d` when
`before` is ~1e-7, because `d + 1e-7` rounds to `d` in f32. Call `game_clock::init()` and
wait until `game_time_secs() >= 0.01` before taking `before`.

Test trap 2: a login test that pins the time-sync bytes must read the ticks back from the
packet (and wait until the clock has passed tick 0), not rebuild with a fixed tick.

Do not trust PR #718 (pre-split, open): it assumed 100 ticks/s, which is 10x off the client.
See [[method-idx-duplicate-table-drift]] for where the method constants live.
