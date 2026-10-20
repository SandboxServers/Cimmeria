# UB-U3c: Carter and Felger offer only the next mission

| Field | Value |
|---|---|
| Row | UB-U3c: Human at 1423 S4223 checks each offer before accepting. Carter offers only the next mission: 1423, 381, 473, 478. No later offer shows early |
| Runnable | Yes, graded from the [UB-U3](UB-U3.md) runs |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H2, through UB-U3 |
| Budget | none of its own |
| Piggyback | rides on UB-U3 run A steps 6, 9, 11; run B steps 7, 8; run C step 7 |

1423 is accepted on Omega, so on Beta its "offer" is the 4643 report. The row's chain of offers is Carter's 381 offer (1882), Felger's 473 offer (2018) and Carter's 478 offer (3223). 473 is offered by Felger, not Carter.

## Driver steps

None of its own. If UB-U3 stopped early, rerun UB-U3 from its fixture; never grade this row from a partial arc.

## Expected and pass/fail

| Click (UB-U3 step) | State | Pass when the dialog is | A FAIL is |
|---|---|---|---|
| Carter (A6) | 1423 S4224 | 4643 | 1882, 2018 or 3223 |
| Carter (A9) | 1423 done, 381 not taken | 1882 | 3223 |
| Felger (A11) | 381 S1408 | 1884 | 2018 |
| Carter (B7) | 381 done, 473 not taken | anything but 3223 (record it) | 3223 |
| Felger (B8) | 381 done, 473 not taken | 2018 | - |
| Carter (C7) | 473 done | 3223 | - |

The row passes when every click shows the dialog in the "pass" column and the server log shows no early offer.

## Orchestrator only: verify list

- For each click above, the one `Content: displaying dialog` line that follows its `fire_interact_tag: matched`, and its dialog id.
- No `displaying dialog` 3223 before `Content: completing mission` 473, and no 2018 before `Content: completing mission` 381.
- For B7, write in the evidence what Carter did show (a bound blurb, the default set or nothing).
