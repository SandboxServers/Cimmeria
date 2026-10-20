# UB-U3d: Carter's overhead marker before and after the 1423 turn-in

| Field | Value |
|---|---|
| Row | UB-U3d: Human at 1423's last step looks at Carter's overhead marker while the dialog-set bind is live, then after chain 23022 unbinds it. Record the marker before and after |
| Runnable | Yes, graded from [UB-U3](UB-U3.md) run A |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H2, through UB-U3 |
| Budget | none of its own |
| Piggyback | UB-U3 run A steps 5 and 8 (`u3d-marker-before`, `u3d-marker-after`) |

The live bind before the turn-in is dialog set 5433 on slot 2300 (chain 23020 on the Beta login; the row's "4643" is the dialog 23022 shows). Chain 23022 removes 5433 and completes 1423; chain 23040 then binds the 381 blurb (set 2305) on the same slot at `mission_completed`. So a marker that is still there after the turn-in can be the new 381 offer, not a stale one: the screenshot cannot tell them apart, the log can.

## Driver steps

None of its own. Run A steps 5 and 8 face Carter with `client_camera` and take the two screenshots.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Before | `u3d-marker-before` exists and shows Carter; record the marker (none, or its shape and colour) | screenshot |
| B. After | `u3d-marker-after` exists and shows Carter; record the marker | screenshot |
| C. Log order | 5433 is removed, then 2305 is added, between the two screenshots' UTC times | server log |

This row records evidence: it passes when A, B and C are all recorded. A missing screenshot makes it `not-run`, never `pass`.

## Orchestrator only: verify list

- `Content: removing dialog set` 5433 and `Content: adding dialog set` 2305 between run A's step 5 and step 8 UTC times.
- Write both markers in the evidence cell, for example "before: yellow ?, after: yellow !".
