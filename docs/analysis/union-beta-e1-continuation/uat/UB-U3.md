# UB-U3: the Human arc on Beta (1423, 381, 473, 478)

| Field | Value |
|---|---|
| Row | UB-U3: Human at 1423 S4223 arrives, talks to Carter, then does 1423, 381, 473 and 478 in order. Each click shows a dialog on the first press; each mission completes in order. Chains 23020, 23040-23052, 23070-23099, 23100-23129 |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H2 (the Carter stand, 1423 S4223) for run A. Runs B and C continue on the same character with no restage |
| Budget | run A 30, run B 32, run C 42 calls; one fresh driver per run |
| Piggyback | [UB-U3c](UB-U3c.md) (offers), [UB-U3d](UB-U3d.md) (Carter's marker), [UR1](UR1.md) (the 1423 reward), [UB-U6b](UB-U6b.md) (button 247) |

The arc is three driver runs because one run would pass the 50-turn cap. Do not relog between runs: the 381 login chain 23053 removes the cages. Stands, all in server metres: Carter stand (1009.4, 13.57, 507.85) is 4.5 m from Carter and from Felger. The hub stand (1014.9, 13.57, 508.35) is 3.8 m from the Admin Workstation. Both stands are inside the AdminBuilding region, so 478's enter-region chain 23103 need not fire, and the workstation click (23104 from S1746, 23105 from S1747) shows 3224.

## Orchestrator before the run

1. Stage F-H2. Dispatch run A at once.
2. After run A, dispatch run B with the same lease and no logout; after run B, run C. If a run ends BLOCKED, grade what it did and stop the arc.

## Driver steps: run A (1423, the reward, the 381 offer)

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. Expect world 23 near (1009.4, 13.57, 507.85).
2. DIALOG.
3. XP. Record it as `xp-before`.
4. `client_batch {"lease_id":L,"steps":[CASH, UTC]}`. Record `cash` as `cash-before`.
5. FIND(2300) (Carter). `client_camera {"lease_id":L,"face_entity_id":<id>,"pitch_deg":-5,"zoom":250}`. SHOT(`u3d-marker-before`).
6. CLICK(Carter). DLGREAD. FINISH. Then DIALOG and record what it returned: `closed` means a second dialog was open.
7. `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":3000}, CASH, UTC]}`. Then XP. Record them as `cash-after` and `xp-after`.
8. `client_camera` facing Carter as in step 5. SHOT(`u3d-marker-after`).
9. CLICK(Carter). DLGREAD. Record the text (expect the 381 offer). ACCEPT.
10. BAG. Expect Carter's Inventory Checklist (2636) in the mission bag.
11. FIND(2301) (Felger). CLICK(Felger). DLGREAD. FINISH. Then DIALOG and record what it returned.
12. BAG. Expect the checklist gone.

## Driver steps: run B (381 cages, the 473 offer)

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}` (already in the world: no relog). DIALOG.
2. `client_entity_find {"lease_id":L,"template_id":2320,"fields":["matches","player"]}`. Record all five cage ids with their positions; match each to the cage stands below by the nearest position. If FIND returns nothing, TP to the stand nearest the target and FIND again; if still nothing, record `not found` and mark that item `not-run`, never `pass`.
3. For each cage n = 1 to 5 in order: TP(stand n), then TAP(cage n's id). Stands: 1 (712.4, -65.99, 407.5); 2 (765.9, -62.85, 421.9); 3 (826.3, -35.48, 442.4); 4 (884.8, -15.69, 459.8); 5 (938.3, -3.48, 477.2). If a TAP reports "too far", FIND(2320), APPROACH that cage and TAP once more.
4. BAG. Record the Xenobiology Cage (3635) stacks and quantities.
5. TP(1009.4, 13.57, 507.85). FIND(2301). CLICK(Felger). DLGREAD. FINISH. DIALOG.
6. BAG. Record what is left of the cages.
7. FIND(2300). CLICK(Carter). DLGREAD. Record the text (it must not be the 478 offer). DIALOG.
8. CLICK(Felger). DLGREAD. Record the text (expect the 473 offer). ACCEPT.

## Driver steps: run C (473 crates, 478 worm)

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. DIALOG.
2. TP(1007.0, 14.37, 300.0) (outside Storage).
3. FIND(2325) (the crates). `client_move_to {"lease_id":L,"entity_id":<the Arc-42 crate, the one nearer x 1007>}` (walk in: this is what fires the Storage enter event). FIND(2325) again and record both ids and distances.
4. TAP(Arc-42). TAP(Arc-66). TAP(Arc-42). TAP(Arc-66). Record each result. (Four taps reach S1725 whether or not the region fired.)
5. TP(1009.4, 13.57, 507.85). FIND(2301). CLICK(Felger). DLGREAD. FINISH.
6. FIND(2300). CLICK(Carter). DLGREAD. FINISH. DIALOG.
7. CLICK(Carter). DLGREAD. Record the text (expect the 478 offer). ACCEPT.
8. TP(1014.9, 13.57, 508.35). FIND(2323) (the Admin Workstation). CLICK. PRESS(`Shut down the worm program`). (Button 247's seeded text, `dialog_screen_buttons`. If the client shows other text, record every button's text and press the one that is not Next, Back or Done.)
9. TP(1009.4, 13.57, 507.85). FIND(2300). CLICK(Carter). DLGREAD. FINISH.
10. JOURNAL(`u3-end`).

## Stop conditions

- A CLICK on Carter or Felger opens nothing twice: record FAIL for that step, skip the steps of that mission, and finish the run with the report.
- Run B: a cage TAP still fails after the retry: go on with the next cage; the turn-in at step 5 then shows the not-yet dialog, which is a valid observation.

## Expected and pass/fail

| Step | Pass when | Chain |
|---|---|---|
| A6 | Carter shows dialog 4643, one dialog only; 1423 completes | 23022 |
| A9 | Carter shows the 381 offer (1882); Accept gives 2636 | 23041, 23042 |
| A11 | Felger shows 1884; 2636 is removed; 381 goes to S1409 | 23044 |
| B3 | Each cage adds one 3635 and counts once | 23045-23049 |
| B5 | Felger shows 1883; five cages are removed; 381 completes | 23051 |
| B8 | Felger shows the 473 offer (2018); Accept takes 473 | 23071, 23072 |
| C3-C4 | 473 reaches S1725 through S1724 | 23073 or 23074/23075, then 23076-23079 |
| C5 | Felger shows 2015; S1726 | 23081 |
| C6 | Carter shows 2014; 473 completes | 23083 |
| C7 | Carter shows the 478 offer (3223); Accept takes 478 | 23101, 23102 |
| C8 | The workstation shows 3224; the press moves 478 to S1749 | 23104 or 23105, then 23106 |
| C9 | Carter shows 1995; 478 completes | 23108 |

The row passes when every step above passes, each click opened its dialog on the first press (no CLICK retry needed), and the missions completed in the order 1423, 381, 473, 478.

## Orchestrator only: verify list

- `lab fixture applied`, then `Content: advancing step` 1423 → 4224 on the login (23020).
- A6: `Content: displaying dialog` 4643, `Content: removing dialog set` 5433, `Content: completing mission` 1423; then 23040 binds 2305 on `mission_completed`. Exactly one `displaying dialog` for that click.
- A9: `fire_dialog_choice: matched` dialog_id 1882, then `accept_mission` 381 and `Content: granting item` 2636.
- A11: `Content: RemoveItem → RemoveInventoryItemByType` and `RemoveInventoryItemByType: persisted` for 2636; `Content: advancing step` 381 → 1409.
- B3: five `Content: granting item` 3635 and five `Content: incremented counter` beta_381_cages.
- B5: `Content: displaying dialog` 1883 (not 1884), the remove-item lines for 3635 and `Content: completing mission` 381.
- B7: no `displaying dialog` 3223 before 473 completes (UB-U3c).
- C3: whether `enter_region` Storage fired (23073) or the first crate tap advanced the step (23074/23075); record which.
- C4: `Content: complete objective` 2037 or 2038, then `Content: advancing step` 473 → 1725.
- C8: `fire_dialog_choice: matched` dialog_id 3224 (chain 23104 from S1746 or 23105 from S1747) with its `button_id` (UB-U6b: must be 247, not -1), then `Content: advancing step` 478 → 1749.
- C9: `Content: completing mission` 478.
