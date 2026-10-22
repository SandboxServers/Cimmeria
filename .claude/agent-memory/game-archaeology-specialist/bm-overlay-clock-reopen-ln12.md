---
name: bm-overlay-clock-reopen-ln12
description: LN-12 (2026-10-22) — BM reopen sends no BMSearch because overlay BlackMarket.lua now() uses worldGetDeltaSeconds (per-frame delta) as a clock; searchSuspended never clears
metadata:
  type: project
---

F8 (BM items only on first open) is OUR overlay Lua bug, not the 2009 binary.
`worldGetDeltaSeconds` = Lua binding `0x00aa19a0` -> `FUN_00aec850` -> `FUN_0054dea0(g_pGLevel)` -> float at `[[[GLevel+0x50]+0x3c]]+0x2BC`; stock UI uses it as a per-frame delta (Performance.lua FPS = 1/x, countdown subtractions). Overlay `BlackMarketMod.now()` treats it as a clock, so `searchSuspended` (4 s) and the pending timeouts (15 s) never expire, and `openSession` skips the auto-search on every later open. The test stub `overlay/test/stubs.lua:200` models it as a monotonic clock, which is why the tests passed.

**Why:** any overlay Lua that needs elapsed time must accumulate the delta in a PreRender handler; there is no wall-clock binding in the stock UI.
**How to apply:** recommended fix is D-LN2 option (b) (Lua clock plus a stub fix). A server push of `onBMAuctions` with clientKey 0 after `onBMOpen` works as a stopgap: the DLL is stateless and `CimmeriaBM.onAuctions` accepts unsolicited replies. Report: `.claude/reports/lab-nav-telemetry/LN-12-re.md`. Not live-verified (LN-13).
Fixed in the overlay by LN-12b (2026-10-22); live check owed after the owner publishes.
