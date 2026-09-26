---
name: mission-1360-step-4038-unreachable
description: Mission 1360 (Frost's Letter) has no chain that advances step 4037 to 4038, so Harset's turn-in chain 6501 (gated on 4038 active) is unreachable. Found 2026-09-26.
metadata:
  type: project
---

Mission 1360 is accepted in the Cellblock (chain 1121) on step 4037. The
Cellblock campaign left step 4038 to "the Castle side"; the Harset campaign
(H30, chains 6501/6502/6505) owns only 4038 and gates every chain on
`step_status 1360/4038 eq active`. No seed anywhere has
`advance_step 1360 '4038'` (checked 2026-09-26), so the mission stays on 4037
forever and the letter can never be delivered. Harset comments call "arriving
with 1360 at 4038" the guaranteed first-visit state; it is not.

**Why:** two campaigns each scoped the other side of the seam.
**How to apply:** before touching 1360, grep for an advance to 4038. Fix
options: an arrival chain that advances 4037→4038 (e.g. `player_loaded` in
Castle or Harset_CmdCenter gated on `step_status 1360/4037 eq active`), or
regate 6501 on 4037. Which world owns the advance is a product call. The
Cellblock `step_stalled` WARN on 4037 is expected by design in any case.
