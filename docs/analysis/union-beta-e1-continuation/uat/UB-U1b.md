# UB-U1b: the journal before and after an SGU step advances

| Field | Value |
|---|---|
| Row | UB-U1b: open the journal before and after a chain advances an SGU step (Human at 1423 S4223; Asgard at 443 S1486). Record whether the client lists the step and objective and whether the objective text updates |
| Runnable | Human half: yes, graded from the UB-U1 run. Asgard half: **NOT RUNNABLE until UO-16** (HELD-B16) |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa`; Asgard half `Fxbetaasgdefa` |
| Fixture | F-H1 (Human, through UB-U1); F-A3 variant for the Asgard half (443 at S1486 instead of completed) |
| Budget | none of its own (Human); 15 calls (Asgard) |
| Piggyback | Human half rides on [UB-U1](UB-U1.md) steps 3 and 9 |

Every SGU step carries `en=false`, which the server never reads. This row records what the client shows; it is evidence, not a pass on a server behaviour.

## Human half (from UB-U1)

No extra driver steps. Use UB-U1's screenshots `u1b-before` (Omega, 1423 at S4223) and `u1b-after` (Beta, after chain 23020 advanced it to S4224), and UB-U1 step 3 and 9's visible-window lists.

## Asgard half (NOT RUNNABLE until UO-16)

Fixture: F-A3, but with 443 staged `{"id":443,"step":1486}` and 477, 479 NA, at [1026.5, 14.37, 268.2] (4.2 m off the Fire Control Hub, template 2329 at (1022.327, 13.37, 268.18)). The click on it at S1486 is chain 23415: it advances 443 to S3455 and binds Mimir's turn-in (dialog set 4579).

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG.
2. JOURNAL(`u1b-asg-before`).
3. FIND(2329) (the Fire Control Hub), APPROACH, TAP. Then FINISH if a dialog opened.
4. JOURNAL(`u1b-asg-after`).

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Journal opens | Both journal screenshots show the mission journal | the two screenshots |
| B. Step listed | Record whether 1423 (443) and its current step and objective are listed before | `*-before` |
| C. Text updates | Record whether the objective text after differs from before | `*-after` |

This row passes when A holds and B and C are recorded with screenshots, whatever they show. A "no journal window" answer is a FAIL of the JOURNAL macro, not of the row: record it and set the row `not-run` with that reason.

## Orchestrator only: verify list

- The server log shows the step advance between the two screenshots (`Content: advancing step`, 1423 to 4224; or 443 to 3455), with the screenshot timestamps on either side of it.
- Compare the two screenshots yourself; write what changed in the evidence cell.
