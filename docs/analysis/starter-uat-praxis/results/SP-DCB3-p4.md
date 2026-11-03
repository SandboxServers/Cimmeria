PACKET SP-DCB3-p4 VERIFIED 2026-11-03 by Sonnet; driver attempts: 1 (run after p5); grade PARTIAL
Re-drive with repaired briefs (runner-DCB3-b, local lab, character Dcbdh). Grades are the verifier grades per part; the runner report does not itemise server timestamps, so rows without a time say so.

| Row | Result | Tier | Server evidence (time, scope, event, key fields) | Client evidence | Looker | Notes |
|---|---|---|---|---|---|---|
| CB-U10c Marsh | PASS | N1 (one retry) | Marsh dialog 4001 via chain 1051; `accepting mission` 641 "Preparation"; blurb 4000 | Marsh dialog read | n/a | The first Marsh click timed out at `read_mouse_over`; an unchanged retry worked |
| CB-U10c 62050 completes | C5-BLOCKED | n/a | 640 still active at step 2215 (not completed), so chain 11005 (needs 640 completed) cannot fire | n/a | n/a | Precondition from SP-DCB3-p3 (640 completion) not met; not a product bug |
| CB-U10c relog + 329 no re-offer | BLOCKED (harness) | n/a | not exercised | `client_target` on Prisoner 329 failed `on_screen` (camera pitch clamp 78.8 degrees) | n/a | Unexercised, not proven. The brief now resets the camera to -10 pitch first |

Row grade: CB-U10c PARTIAL. Marsh held; the two other clauses are blocked and unproven.

One WARN `movement.validation_reject` (teleport) at 11:26:43Z: a GOTO from the earlier position, not a row clause; not filed.

SIGN-OFF SHEET (NEEDS_HUMAN): none.

FINDINGS: none. Marsh chain 1051 lets 641 be accepted with 640 incomplete; the seed treats that as intended (order enforced by interaction bit 8388608), so it is not filed.

NOT RUN / BLOCKED:
- 62050 completes: needs 640 completed (SP-DCB3-p3 .14, inferred).
- Relog and 329 no re-offer: harness.
- Abandon and skip: not in this part.

BRIEF DEFECTS (fixed in this change):
- .3 and .9: the INTERACT recipe needs a camera reset to -10 pitch first; the macro now carries it.
- 640 completion (chain 1044 on teleport_in region 2) was not reachable from p3's own setup; p3 now has the GOTO (.14), and p4's precondition names it.

RESUME HINT: To grade "62050 completes", reach 640 completed (p3 .14), then click Marsh and expect `complete objective` 62055 from chain 11005. Re-drive the 329 relog check with the camera reset.
