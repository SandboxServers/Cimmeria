# UB-U7b: 381 turn-in with the cages in two stacks

| Field | Value |
|---|---|
| Row | UB-U7b: Human holding cages 3635 in two stacks at 381's turn-in. The first stack is removed and the second stays: a known gap, record what is left |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H3 (restaged: cage 1 stand, 1423 completed, 381 S1409, no items) |
| Budget | 30 calls |
| Piggyback | none |

**Why no cage fixture.** A fixture could stage the cages, but the login chain 23053 removes them at Play (381 S1409). So the run collects the five cages after login, then splits the stack in the bag with `client_drag_drop` (`split: true` moves one item, as in the bank rows BANK-C2/C3). That leaves stacks of 4 and 1 at the turn-in.

**The known gap, from the code.** The turn-in (chain 23051) runs `remove_item` 3635 qty 5. `RemoveInventoryItemByType` takes the first stack it finds; when the stack is smaller than the count it removes the whole stack and stops. So Felger completes 381 and one cage stays in the bag. This row records that; it does not fix it.

Cage stands (server metres): 1 (712.4, -65.99, 407.5); 2 (765.9, -62.85, 421.9); 3 (826.3, -35.48, 442.4); 4 (884.8, -15.69, 459.8); 5 (938.3, -3.48, 477.2).

## Orchestrator before the run

Stage F-H3 again (after UB-U3b, 381 is completed, so the restage is needed). Dispatch at once.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG.
2. `client_entity_find {"lease_id":L,"template_id":2320,"fields":["matches","player"]}`. Record the five ids and match them to the stands.
3. TAP(cage 1). Then for n = 2 to 5: TP(stand n), TAP(cage n).
4. `client_inventory {"lease_id":L,"containers":["Mission"],"include_empty":true,"fields":["containers"]}`. Record the cage stack's slot, its quantity (expect 5) and one empty slot.
5. `client_drag_drop {"lease_id":L,"from":{"container":"Mission","slot":<the cage slot>},"to":{"container":"Mission","slot":<the empty slot>},"split":true,"wait_ms":4000}` (slots are 1-based, as `client_inventory` reports them). Record the result.
6. Same call as step 4. Record every cage stack with its slot and quantity (expect 4 and 1). If there is still one stack of 5, record "split did nothing", SHOT(`u7b-bag`) and go on.
7. TP(1009.4, 13.57, 507.85). FIND(2301). CLICK(Felger). DLGREAD. FINISH.
8. Same call as step 4. Record every cage stack left, with its slot and quantity.
9. `client_ui_sequence {"lease_id":L,"actions":[{"do":"key","key":"b"},{"do":"wait","ms":1500}]}`. SHOT(`u7b-bag-after`). Then the same key again to close the bag.

## Stop conditions

- A cage TAP fails after its retry: stop after step 4 and report (without five cages, the counter cannot reach the turn-in).
- Step 6 shows one stack: the row is `not-run` (no two stacks); finish steps 7 and 8 anyway and report.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Two stacks | Step 6 shows two cage stacks that sum to 5 | step 6 |
| B. Turn-in | Felger shows 1883 and 381 completes (the counter, not the bag, gates the turn-in) | step 7, server log |
| C. What is left | Record the stack left at step 8 (expected: the second stack, 1 cage) | step 8, `u7b-bag-after` |

The row passes on A, B and C recorded. Any leftover is the known gap, not a FAIL; write its slot and quantity in the evidence. No cage left is a surprise: record it and flag it for the orchestrator.

## Orchestrator only: verify list

- Five `Content: granting item` 3635 and five `Content: incremented counter` beta_381_cages.
- At the turn-in: `Content: RemoveItem → RemoveInventoryItemByType`, then `RemoveInventoryItemByType: requested count exceeds stack size; removing whole stack` and `RemoveInventoryItemByType: persisted` once (the first stack), then `Content: completing mission` 381.
- The quantity in the "exceeds" line tells which stack went first; write it in the evidence.
