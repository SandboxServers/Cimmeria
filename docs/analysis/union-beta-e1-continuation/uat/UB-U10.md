# UB-U10: offer Accept only on the final screen

> **NOT RUNNABLE until UO-16 (HELD-B16).** Written in full; `uat-results.md` keeps it `not-run` until an Asgard character can be made.

| Field | Value |
|---|---|
| Row | UB-U10: Asgard offered 477 (dialog 1997), then 479 (dialog 3228), pages through each. Accept shows only on the last screen (64049, 64068), and pressing it accepts on the first press |
| Runnable | **No** (HELD-B16) |
| Instance | `default` |
| Character | `Fxbetaasgdefa` |
| Fixture | part A: F-A3 (deck; 443 C, 477 NA). Part B: F-A4 (deck; 481 to 477 C, 479 NA), restaged after a logout |
| Budget | part A 24, part B 18 calls |
| Piggyback | UB-U5 run E step 2 gives early evidence for part B; this file is still the grade |

**From the seeds** (UB-DP, `dialog_screen_buttons.sql`). Offer 1997 (477, dsm 2456) has five screens, 64045 to 64049; UB-DP moved its Accept button (row 1737) from screen 64047 to the last screen 64049. Offer 3228 (479, dsm 2461) has three screens, 64066 to 64068; UB-DP moved its Accept (row 2597) from 64066 to 64068. Mimir shows 1997 with 443 completed and 477 not active (23436); its Accept fires 23437. Mimir shows 3228 with 477 completed and 479 not active (23461); its Accept fires 23462. Mimir (696.44, 157.77, 67.36) is 4 m from the fixture point.

**PAGE.** DLGREAD: record the screen's page number if shown, its first 80 characters and every button's text and window. SHOT(`u10-<offer>-p<n>`). If a button with text `Accept` is listed, stop paging. Otherwise click the button whose text is `Next` with `client_window_click {"lease_id":L,"target":"<its window>"}`.

## Orchestrator before the run

Stage F-A3, dispatch part A. After it, have a driver call `lab_logout`, stage F-A4, and dispatch part B.

## Driver steps

Part A (477, five screens):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG.
2. FIND(2305). APPROACH. CLICK(Mimir).
3. PAGE, offer `1997`, up to five times (n = 1 to 5), until Accept shows.
4. PRESS(`Accept`). Record whether a dialog is still open after it.
5. BAG (expect the Asgard Recording Device 2646). JOURNAL(`u10-477`).

Part B (479, three screens):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG.
2. FIND(2305). APPROACH. CLICK(Mimir).
3. PAGE, offer `3228`, up to three times, until Accept shows.
4. PRESS(`Accept`).
5. JOURNAL(`u10-479`).

## Stop conditions

- No `Next` and no `Accept` on a screen: SHOT, record FAIL for paging, FINISH, and stop that part.
- Accept shows on an early screen: record the screen number, do not page further, and go on with step 4 (that is a FAIL of A, still press it to grade B).

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Last screen only | Accept is on screen 5 of 1997 and screen 3 of 3228, and on no earlier screen | PAGE records, `u10-*` screenshots |
| B. First press | One Accept press accepts the mission: 477 then 479 appears in the journal, 2646 is in the bag after 477 | step 4, step 5, server log |

The row passes on A and B for both offers.

## Orchestrator only: verify list

- Part A: `fire_dialog_choice: matched` dialog_id 1997 (23437) with the Accept button's id (not -1), `Content: accepting mission` 477, `Content: granting item` 2646.
- Part B: the same for 3228 (23462) and `Content: accepting mission` 479.
- One press each: no second dialog choice for the same dialog before the accept line.
- If the packet tap is on, the screen ids sent are 64045 to 64049 and 64066 to 64068, and the Accept button rides only on 64049 and 64068.
