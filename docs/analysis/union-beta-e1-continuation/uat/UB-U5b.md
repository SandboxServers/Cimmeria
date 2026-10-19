# UB-U5b: the 420 scanner quota across a relog

> **NOT RUNNABLE until UO-16 (HELD-B16).** Written in full; `uat-results.md` keeps it `not-run` until an Asgard character can be made.

| Field | Value |
|---|---|
| Row | UB-U5b: Asgard at 420 S1455 collects 3 scanners, relogs on the deck, beams down and collects 5. Mimir shows 1954 (not the turn-in) before the 5th and takes 5 after |
| Runnable | **No** (HELD-B16) |
| Instance | `default` |
| Character | `Fxbetaasgdefa` |
| Fixture | F-A2 (the deck (699.0, 158.77, 70.4), 4 m off Mimir; 481 C, 420 S1455, the rest NA; no items). Run B continues with no restage |
| Budget | run A 30, run B 40 calls |
| Piggyback | none |

**From the seeds** (`beta_e1_m420_chains.sql`). Each scanner `BetaE1_AsgardScanner_1` to `_5` (template 2330) grants one Asgard Scanner 2647 and adds 1 to the counter `beta_420_scanners`, once per player (23343-23347). Counters and `once` reset on a relog (Q5), so a login at S1455 removes up to 5 scanners from the bag (23351) and rebinds Mimir's turn-in 2343 (23348): the quota starts over and the bag matches it. Mimir at S1455 shows 1954 with the counter below 5 (23350) and the turn-in 3057 at 5 or more (23349: takes 5 scanners, completes 420).

Scanners (server metres): 1 (740.48, -19.43, 68.64), 2 (746.48, -19.54, 62.64), 3 (746.48, -19.80, 74.64), 4 (752.48, -20.20, 68.64), 5 (740.48, -19.43, 62.64). Ground pad spawn 149 at (746.555, -19.21, 68.75) among them; deck pad spawn 151 at (700.48, 157.95, 70.365).

## Local macros

UP, DOWN and MIMIR as in [UB-U5 § Local macros](UB-U5.md#local-macros-this-file-only). **SCAN(n).** `client_move_to {"lease_id":L,"entity_id":<scanner n's id>}`, then TAP(it). Record the TAP result.

## Orchestrator before the run

Stage F-A2 and dispatch run A at once; run B follows on the same lease.

## Driver steps

Run A (3 scanners, relog on the deck):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG.
2. DOWN. FIND(2330): record all five matches and match each to the list above by x and z.
3. SCAN(1), SCAN(2), SCAN(3).
4. BAG (expect 2647 x3).
5. UP (the relog must be on the deck).
6. RELOG. Record the position (expect the deck).
7. BAG (expect no 2647: 23351 removed them).
8. MIMIR (expect 1954, not 3057). SHOT(`u5b-mimir-after-relog`). FINISH.

Run B (5 scanners, Mimir before the 5th):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}` (no relog). DIALOG.
2. DOWN. FIND(2330) (ids changed after the relog).
3. SCAN(1), SCAN(2), SCAN(3), SCAN(4). After SCAN(1), record whether the scanner clicked before the relog granted again.
4. BAG (expect 2647 x4).
5. UP. MIMIR (expect 1954). SHOT(`u5b-mimir-at-4`). FINISH.
6. DOWN. FIND(2330). SCAN(5).
7. BAG (expect 2647 x5).
8. UP. MIMIR (expect 3057). FINISH.
9. BAG (expect no 2647).

## Stop conditions

- A step-6 relog lands off the deck: record the position and stop. The row is `not-run` (the relog was not on the deck), not FAIL.
- A SCAN grants nothing after the one retry: record it and go on with the next scanner; at the end, the counts tell the story.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Relog reset | After the relog the bag holds no scanners and Mimir shows 1954 | A7, A8, `u5b-mimir-after-relog` |
| B. Re-collect | Each scanner grants again after the relog, so 4 clicks give 4 | B3, B4 |
| C. Not yet | With 4, Mimir shows 1954, not the turn-in | B5, `u5b-mimir-at-4` |
| D. Turn-in | With 5, Mimir shows 3057, takes the 5 and 420 completes | B8, B9, server log |

The row passes on A to D.

## Orchestrator only: verify list

- A6: on login, `Content: adding dialog set` 2343 (23348) and the remove-item line for 2647 (23351).
- A8 and B5: `display_dialog` 1954 from 23350; no 23349 line.
- B3: `Content: granting item` 2647 for each scanner after the relog (the counter and `once` reset). A scanner that grants nothing after the relog is a FAIL of B.
- B8: 23349: `Content: displaying dialog` 3057, the remove-item line for 2647 qty 5 with `persisted`, `Content: completing mission` 420, then the 421 blurb bind (23360).
