# UB-U3b: 381 cages across a relog

| Field | Value |
|---|---|
| Row | UB-U3b: Human at 381 S1409 collects 3 cages, relogs, collects 5 and turns in to Felger. The count behaves as designed across the relog, Felger takes 5 and the cage stack is right. Record whether a cage clicked before the relog can be clicked again |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H3 (cage 1 stand, 1423 completed, 381 S1409, no items) |
| Budget | 40 calls |
| Piggyback | none |

**The design, from the seeds.** The cage counter `beta_381_cages` and each cage's `once` flag last for one session. On a Beta login at S1409, chain 23053 removes up to 5 cages (`remove_item` 3635 qty 5) and chain 23050 rebinds Felger. So after the relog the bag holds no cages, the counter is back at 0, every cage can be clicked again, and Felger takes the turn-in only after 5 fresh clicks. That is what "the count survives the relog as designed" means here; a cage count that carries over is a FAIL.

Cage stands (server metres): 1 (712.4, -65.99, 407.5); 2 (765.9, -62.85, 421.9); 3 (826.3, -35.48, 442.4); 4 (884.8, -15.69, 459.8); 5 (938.3, -3.48, 477.2). The Carter stand (1009.4, 13.57, 507.85) is 4.5 m from Felger.

## Orchestrator before the run

Stage F-H3 and dispatch at once.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG.
2. BAG. Expect no Xenobiology Cage.
3. `client_entity_find {"lease_id":L,"template_id":2320,"fields":["matches","player"]}`. Record the five ids and match them to the stands by position.
4. TAP(cage 1). TP(stand 2). TAP(cage 2). TP(stand 3). TAP(cage 3).
5. BAG. Record the cage stack (expect 3).
6. RELOG (character `Uat Fxbetahumdefa`).
7. BAG. Record the cages (expect none).
8. `client_entity_find` for template 2320 again (the ids change after a relog). Record them.
9. TP(stand 1). TAP(cage 1). Record the result: this is the "clicked before the relog" check.
10. TP(stand 2). TAP(cage 2). TP(stand 3). TAP(cage 3). TP(stand 4). TAP(cage 4). TP(stand 5). TAP(cage 5).
11. BAG. Record the cage stack (expect 5).
12. TP(1009.4, 13.57, 507.85). FIND(2301). CLICK(Felger). DLGREAD. FINISH.
13. BAG. Record what is left of the cages.

## Stop conditions

- Step 6 fails to get back in the world twice: BLOCKED.
- A TAP fails after its retry: go on; the turn-in then shows the not-yet dialog, which is a valid observation.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Before the relog | 3 cages in one stack after step 5 | step 5 BAG |
| B. Relog clears | No cages after step 7 | step 7 BAG, server log |
| C. Re-click | Cage 1 grants a cage again at step 9 | step 9, server log |
| D. Turn-in | Felger shows the turn-in (1883), not the not-yet dialog (1884), and step 13 shows no cages | step 12 DLGREAD, step 13 BAG |
| E. Completion | 381 completes | server log |

The row passes on A to E. If C fails (a pre-relog cage cannot be clicked again), D still decides the row: record C in the evidence, because the row asks for it.

## Orchestrator only: verify list

- `lab fixture applied`; on that login `RemoveInventoryItemByType: no instance of this design id in a container the character can reach` from 23053 (nothing to remove yet) is expected and fine.
- Steps 4 and 10: one `Content: granting item` 3635 and one `Content: incremented counter` beta_381_cages per TAP, with `fire_interact_tag: matched` for each `BetaE1_XenoCage_n`.
- Step 6: on the relog login, `Content: RemoveItem → RemoveInventoryItemByType`, then `RemoveInventoryItemByType: requested count exceeds stack size; removing whole stack` (3 held, 5 asked) and `RemoveInventoryItemByType: persisted` (23053), and `Content: adding dialog set` 2302 (23050).
- Step 12: `Content: displaying dialog` 1883, the remove-item lines for 3635 qty 5 and `Content: completing mission` 381 (23051). A `displaying dialog` 1884 here is a FAIL of D.
