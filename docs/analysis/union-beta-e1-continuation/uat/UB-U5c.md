# UB-U5c: the 477 subjects, one click each

> **NOT RUNNABLE until UO-16 (HELD-B16).** Written in full; `uat-results.md` keeps it `not-run` until an Asgard character can be made.

| Field | Value |
|---|---|
| Row | UB-U5c: Asgard at 477 with the four subject binds live clicks Carter, Debeaux, Felger and Satterfield. Each click gives feedback on the first press, and the blurb does not re-show after that subject's objective completes |
| Runnable | **No** (HELD-B16) |
| Instance | `default` |
| Character | `Fxbetaasgdefa` |
| Fixture | F-A3 (the deck (699.0, 158.77, 70.4); 481, 420, 421, 1147, 443 C; 477, 479 NA). As run D of [UB-U5](UB-U5.md), no fixture: it continues there |
| Budget | 46 calls |
| Piggyback | run D of [UB-U5](UB-U5.md) |

**From the seeds** (`beta_e1_m477_chains.sql`). On login with 443 completed, 23435 binds the 477 blurb (dsm 2457) on Mimir. Mimir shows the offer 1997 (23436); Accept on its last screen (23437) accepts 477 at S1744, gives the Asgard Recording Device 2646 and binds dsm 2457 on all four subjects: Carter (slot 2300), Debeaux (2304), Felger (2301), Satterfield (2302). Each subject click runs one chain of its partition (23438-23453): it completes that subject's objective (O2060 Carter, O2061 Debeaux, O2062 Felger, O2063 Satterfield) and unbinds that subject. The fourth subject's chain instead advances to S1745 and binds the completion 4408 on Mimir. Mimir then shows 1996, takes 2646 and completes 477 (23459).

**What feedback to expect.** The row says "each click shows that subject's dialog". The design (UB-35) says the opposite: a tagged click that matches a chain suppresses the bind's auto-display, so the record click shows no blurb, and the objective update is the feedback. This file records both and grades the project rule: the first press must give visible feedback (a dialog, or the objective tracker or journal changing). Which one the client shows is written in the evidence, and the row text gets reconciled at the close-out.

Stands (server metres): Command Post (1009.4, 13.57, 507.85) for Carter (1011.405, 13.57, 511.851) and Felger (1007.405, 13.57, 511.851), about 4.5 m each; barracks south (1030.0, 14.38, 314.8) for Debeaux (1034.146, 13.38, 314.764); Quartermaster (1008.0, 14.44, 280.0) for Satterfield (1004.144, 13.44, 276.69), 5.1 m, so APPROACH first.

**SUBJ(template).** FIND(template). APPROACH. TAP(it). DLGREAD: record any dialog. SHOT(`u5c-<name>-1`). DIALOG. Then TAP(it) again and DLGREAD: record any dialog (the second press).

## Orchestrator before the run

Stage F-A3 and dispatch at once (skip when it runs as UB-U5 run D).

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG.
2. FIND(2305). APPROACH. CLICK(Mimir). DLGREAD (expect the offer 1997). ACCEPT. BAG (expect the Asgard Recording Device 2646).
3. TP(1009.4, 13.57, 507.85).
4. SUBJ(2300), name `carter`.
5. SUBJ(2301), name `felger`.
6. TP(1030.0, 14.38, 314.8). SUBJ(2304), name `debeaux`.
7. TP(1008.0, 14.44, 280.0). SUBJ(2302), name `satterfield`.
8. JOURNAL(`u5c-journal`).
9. TP(699.0, 158.77, 70.4) (the deck). FIND(2305). CLICK(Mimir). DLGREAD (expect 1996). FINISH. BAG (expect 2646 gone).

## Stop conditions

- Step 2 shows no 1997 or the accept fails: record it and stop. Nothing is bound without the accept.
- Budget: if fewer than 8 calls are left before step 9, skip the JOURNAL.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. First press | Each subject's first press gives visible feedback (a dialog, or the objective tracker or journal changing) and the server completes that subject's objective on it | SUBJ first DLGREAD, `u5c-<name>-1`, server log |
| B. No re-show | Each subject's second press shows no 477 blurb (1999) and fires no 477 chain | SUBJ second DLGREAD, server log |
| C. Return | After the fourth subject, Mimir shows 1996, takes 2646 and 477 completes | step 9, server log |
| D. Which feedback | Recorded per subject: dialog or objective update | the SUBJ results |

The row passes on A, B and C. D goes in the evidence.

## Orchestrator only: verify list

- Step 2: `fire_dialog_choice: matched` dialog_id 1997 (23437), `Content: accepting mission` 477, `Content: granting item` 2646, then four `Content: adding dialog set` 2457 lines (slots 2300, 2301, 2304, 2302).
- One subject chain per first press: Carter 23438-23441, Felger 23446-23449, Debeaux 23442-23445, Satterfield 23450-23453. With the order above that is 23438 (Carter first), 23447 (Felger, Debeaux still open), 23444 (Debeaux third) and 23453 (Satterfield last); a different chain means a different order of objectives, and that is fine if each subject fired exactly one.
- `Content: complete objective` 477 / 2060, 2062, 2061, then `Content: advancing step` 477 → 1745 and `Content: adding dialog set` 4408 (slot 2305). No complete_objective for the last subject (F19).
- No chain line and no `Content: displaying dialog` 1999 on any second press.
- Step 9: 23459: `Content: displaying dialog` 1996, the remove-item line for 2646, `Content: completing mission` 477.
