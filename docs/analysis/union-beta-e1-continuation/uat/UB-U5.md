# UB-U5: the Asgard arc 481 to 479, with the beam

> **NOT RUNNABLE until UO-16 (HELD-B16).** The Asgard arc has no real-player entry until Omega's UO-16 is built. The steps are written in full so the row can run the day it lands; until then `uat-results.md` keeps it `not-run`.

| Field | Value |
|---|---|
| Row | UB-U5: an Asgard character (archetype 5) at 481 S1760 with item 3399 arrives, talks to Carter, beams up, talks to Mimir, then does 420, 421, 1147, 443, 477 and 479 in order. Beam down and up again at any point. Every mission completes in order; each beam click moves you to the deck or the ground |
| Runnable | **No** (HELD-B16) |
| Instance | `default` |
| Character | `Fxbetaasgdefa` (made by hand when UO-16 lands, [fixtures.md § Characters](fixtures.md#characters)) |
| Fixture | F-A1 (the Carter stand (1009.4, 13.57, 507.85); 481 S1760, the rest NA; 3399 x1). Runs B to E continue on the same character with no restage |
| Budget | A 45, B 40, C 38, D: run [UB-U5c](UB-U5c.md), E 30 calls |
| Piggyback | run D is [UB-U5c](UB-U5c.md); run E's offer is also [UB-U10](UB-U10.md) part 2 evidence |

**From the seeds** (`beta_e1_m481_chains.sql` to `beta_e1_m479_chains.sql`, chains 23310-23467). Every chain is gated on archetype 5. Each turn-in on Mimir completes the mission, and the `mission_completed` trigger of the next mission binds its blurb on Mimir (slot 2305) at once:

| Mission | Offer (accept chain) | Steps and the chain for each | Turn-in |
|---|---|---|---|
| 481 | (fixture) | login S1760 → S1761, binds Carter 2465 (23310); Carter shows 1915, takes 3399, S1762, binds Mimir 2467 (23312); the ground pad completes O2080 beside the beam (23316) | Mimir 1916 (23317) |
| 420 | Mimir 1953 (23341, 23342) | five scanners, one each (23343-23347, counter `beta_420_scanners`) | Mimir 3057 at 5 or more (23349); 1954 below 5 (23350) |
| 421 | Mimir 1987 (23361, 23362) | enter Infirmary S1460 → S1461 (23363, or the emitter click 23364); emitter shows 3722, S1462 (23365); drone S1464, binds 4585 (23366) | Mimir 3901 (23368) |
| 1147 | Mimir 3904 (23391, 23392) | the Analytical Machine S3458 → S3459, binds 4590 (23393) | Mimir 3905 (23395) |
| 443 | Mimir 1918 (23406, 23407: binds Carter 3728) | Carter 3895, S1483, binds Satterfield 3780 (23409); Satterfield 3161, S1484 (23411); enter FireSite S1485 (23412, or the manager click 23413); manager S1486 (23414); fire control hub S3455, binds 4579 (23415) | Mimir 1917 (23417) |
| 477 | Mimir 1997 (23436, 23437: gives 2646, binds the four subjects) | see [UB-U5c](UB-U5c.md) (23438-23453) | Mimir 1996, takes 2646 (23459) |
| 479 | Mimir 3228 (23461, 23462) | Machine S1750 → S1751 (23463); Simulator S1752 (23464); Machine S3644, binds 3527 (23465) | Mimir 3227 (23467) |

**Where things are** (server metres): Carter (1011.405, 13.57, 511.851) and Felger from the stand (1009.4, 13.57, 507.85). Ground pad spawn 149 (`BetaSite_00000007`) at (746.555, -19.21, 68.75) with the five scanners 6 to 8.5 m around it; deck pad spawn 151 at (700.48, 157.95, 70.365). Mimir (696.44, 157.77, 67.36), the Analytical Machine (696.44, 157.77, 64.36) and the Consciousness Simulator (699.44, 157.93, 67.36) on the deck. Infirmary stand (960.5, 16.97, 289.5), inside the region (r 10 round the emitter). Satterfield (1004.144, 13.44, 276.69), about 7 m outside the FireSite edge (about 19 m from its centre (1022.3, 14.5, 271.2), r 12); fire stand (1026.5, 14.37, 268.2), inside it.

## Local macros (this file only)

- **UP.** TP(746.5, -18.5, 78.0). FIND(3): take the match nearest (746.6, -19.2, 68.8). APPROACH. TAP(it). Then `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":5000},{"op":"player_state","fields":["world_id","position"]}, UTC]}`. Record the position: expect the deck (y about 158).
- **DOWN.** FIND(3): take the match nearest (700.5, 158.0, 70.4). TAP(it). Then the UP batch. Expect about (745.05, -19.6, 68.75).
- **MIMIR.** FIND(2305). APPROACH. CLICK(Mimir). DLGREAD: record the dialog.

## Orchestrator before the run

Stage F-A1 and dispatch run A at once. Runs B, C, D (the UB-U5c file, with its fixture line ignored: no restage) and E follow on the same lease with no logout.

## Driver steps

Run A (481, 420):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}`. WAIT17. DIALOG. BAG (expect the Communiqué 3399).
2. FIND(2300). CLICK(Carter). DLGREAD (expect 1915). FINISH. BAG (expect 3399 gone).
3. UP. Record the position.
4. MIMIR (expect 1916). FINISH.
5. MIMIR (expect the offer 1953). ACCEPT.
6. DOWN. FIND(2330): record all five matches with id and position.
7. For each scanner, in the order of the FIND list: `client_move_to {"lease_id":L,"entity_id":<its id>}`, then TAP(it). Five times.
8. BAG (expect Asgard Scanner 2647 x5).
9. UP. MIMIR (expect 3057). FINISH. BAG (expect the scanners gone).

Run B (421, 1147):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}` (no relog). DIALOG.
2. MIMIR (expect the offer 1987). ACCEPT.
3. DOWN. TP(960.5, 16.97, 289.5) (the Infirmary).
4. FIND(2332). TAP(it). DLGREAD. If no dialog 3722 opened, TAP(it) once more and DLGREAD. FINISH.
5. FIND(2333). TAP(it). SHOT(`u5-drone`).
6. UP. MIMIR (expect 3901). FINISH.
7. MIMIR (expect the offer 3904). ACCEPT.
8. FIND(2331). APPROACH. TAP(it).
9. MIMIR (expect 3905). FINISH.

Run C (443):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}` (no relog). DIALOG.
2. MIMIR (expect the offer 1918). ACCEPT.
3. DOWN. TP(1009.4, 13.57, 507.85). FIND(2300). CLICK(Carter). DLGREAD (expect 3895). FINISH.
4. TP(1008.0, 14.44, 280.0). FIND(2302). APPROACH. CLICK(Satterfield). DLGREAD (expect 3161). FINISH.
5. WALK(1026.5, 14.37, 268.2) (walk into FireSite; this fires its enter event).
6. FIND(2328). APPROACH. TAP(it).
7. FIND(2329). APPROACH. TAP(it).
8. UP. MIMIR (expect 1917). FINISH.

Run D (477): run [UB-U5c](UB-U5c.md) steps 2 to 9 on this character (no restage, no login: skip its step 1 and run DIALOG instead).

Run E (479):

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetaasgdefa"}` (no relog). DIALOG.
2. MIMIR (expect the offer 3228). ACCEPT.
3. FIND(2331). APPROACH. TAP(it).
4. FIND(2334). APPROACH. TAP(it).
5. FIND(2331). APPROACH. TAP(it).
6. MIMIR (expect 3227). FINISH.
7. DOWN, then UP (the "again" beam pair). Record both positions.
8. JOURNAL(`u5-journal-end`).

## Stop conditions

- A3 does not reach the deck: record it, SHOT, and stop. Every later step needs the deck.
- Any expected dialog does not open after the one retry: record FAIL for that step and stop the run (the next mission is gated on this one).

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Order | Each mission is offered only after the one before it completes, and all seven complete: 481, 420, 421, 1147, 443, 477, 479 | DLGREADs, server log |
| B. Each step | Every expected dialog opens on the first click and every step change in the table follows | DLGREADs, server log |
| C. Beam | Every UP lands on the deck and every DOWN on the ground | the UP and DOWN positions |
| D. Items | 3399 is taken by Carter, 2647 x5 by Mimir, 2646 given at 477's accept and taken at its turn-in | BAGs, server log |

The row passes on A to D.

## Orchestrator only: verify list

- `lab fixture applied`; `Content: granting stargate address` 5 by 23002 only (23001 or 23003 firing for this character is a FAIL); `Content: advancing step` 481 → 1761.
- `Content: complete objective` 481 / 2080 on the first UP (23316), and `Content: teleporting entity` on every UP and DOWN (23314, 23315).
- `Content: completing mission` for 481, 420, 421, 1147, 443, 477, 479, in that order, each followed by the next mission's blurb bind (`Content: adding dialog set` 2344, 2349, 4588, 4581, 2457, 2462).
- 421: whether the Infirmary enter (23363) or the emitter click (23364) moved S1460; both pass. 443: whether FireSite (23412) or the manager (23413) moved S1484; both pass.
- The log texts are indicative: grep `crates/cell-content/` for the exact text before quoting it.
