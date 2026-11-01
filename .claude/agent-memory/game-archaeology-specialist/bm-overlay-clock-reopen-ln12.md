---
name: bm-overlay-clock-reopen-ln12
description: LN-12 (2026-10-22) - BM reopen sent no BMSearch because overlay BlackMarket.lua now() used worldGetDeltaSeconds (a per-frame delta) as a clock; fixed by LN-12b, which accumulates the delta in BlackMarketMod.clock
metadata:
  type: project
---

F8 (BM items only on first open) was OUR overlay Lua bug, not the 2009 binary.
`worldGetDeltaSeconds` = Lua binding `0x00aa19a0` -> `FUN_00aec850` -> `FUN_0054dea0(g_pGLevel)` -> float at `[[[GLevel+0x50]+0x3c]]+0x2BC`; stock UI uses it as a per-frame delta (Performance.lua FPS = 1/x, countdown subtractions). Overlay `BlackMarketMod.now()` treated it as a clock, so `searchSuspended` (4 s) and the pending timeouts (15 s) never expired, and `openSession` skipped the auto-search on every later open. The test stub modelled it as a monotonic clock, which is why the tests passed; it now returns `env.frameDelta`.

**Why:** any overlay Lua that needs elapsed time must accumulate the delta in a PreRender handler; there is no wall-clock binding in the stock UI.
**How to apply:** any overlay Lua that needs elapsed time must accumulate the delta in a PreRender handler (BlackMarketMod.clock); there is no wall-clock binding in the stock UI. Report: `.claude/reports/lab-nav-telemetry/LN-12-re.md`. Fixed by LN-12b; live check owed after the owner publishes.

INVALIDATED WHEN: worldGetDeltaSeconds stops being a per-frame delta, or BlackMarketMod.clock is removed
checked: 2026-11-01
