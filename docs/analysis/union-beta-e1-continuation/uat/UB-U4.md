# UB-U4: 1425, 1426 and 1427 after 478 (Human)

| Field | Value |
|---|---|
| Row | UB-U4: Human with 478 completed runs to 1425 S4244 and 1427 S4250. 1425 and 1427 stay open in the journal and 1426 completes (D-UB5 A: no Jackson or Woolsey on Beta) |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H4 (the Carter stand; 1423, 381, 473, 478 completed; 1425-1427 not active). Run B continues with no restage or relog |
| Budget | run A 34, run B 38 calls; one fresh driver per run |
| Piggyback | [UB-U4b](UB-U4b.md) (the camera), [UB-U2b](UB-U2b.md) (dialog 4663's header), [UB-U6b](UB-U6b.md) (buttons 456, 457, 458) |

Stands (server metres): Carter (1009.4, 13.57, 507.85), 4.5 m from Carter. Hub (1014.9, 13.57, 508.35), 3.8 m from the Hub Computer (template 2322). Science (653.9, 20.77, 810.2), 4.0 m from the Science Computer (2324) and from the Four Pieces case (2321). Ruin (122.2, 28.17, 352.6), 4.2 m from the ruin note (2335).

## Orchestrator before the run

Stage F-H4 and dispatch run A at once. After run A, dispatch run B on the same lease with no logout.

## Driver steps: run A (1425, the 1426 offer and camera)

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG.
2. FIND(2300). CLICK(Carter). DLGREAD (expect the 1425 offer). ACCEPT.
3. TP(1014.9, 13.57, 508.35). FIND(2322). CLICK(it). PRESS(`Search the SGU Network.`).
4. TP(653.9, 20.77, 810.2). FIND(2324). CLICK(it). PRESS(`Hack the computer's login.`).
5. TP(1009.4, 13.57, 507.85). FIND(2300). CLICK(Carter). DLGREAD (expect the 1426 offer). ACCEPT.
6. BAG. Record the Digital Camera (4832): present or not, its bag and slot.
7. TP(653.9, 20.77, 810.2). FIND(2321) (the Four Pieces case). TAP(it).

## Driver steps: run B (the upload, 1426 turn-in, 1427)

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}` (no relog). DIALOG.
2. TP(1014.9, 13.57, 508.35). FIND(2322). CLICK(it). PRESS(`Upload the photo to Daniel Jackson.`).
3. BAG. Record whether the Digital Camera is still there.
4. TP(1009.4, 13.57, 507.85). FIND(2300). CLICK(Carter). DLGREAD. FINISH. DIALOG (record what it returned).
5. CLICK(Carter). DLGREAD (expect the 1427 offer). ACCEPT.
6. TP(122.2, 28.17, 352.6). FIND(2335). CLICK(it). DLGREAD: record the header line exactly, even when blank (UB-U2b). SHOT(`u2b-4663`). FINISH.
7. TP(1009.4, 13.57, 507.85). FIND(2300). CLICK(Carter). DLGREAD. FINISH.
8. JOURNAL(`u4-journal`).

## Stop conditions

- A step's CLICK opens nothing after the retry: record FAIL, then skip only the steps of that mission. Run B needs run A's step 7; if it failed, run B is BLOCKED.

## Expected and pass/fail

| Step | Pass when | Chain |
|---|---|---|
| A2 | Carter shows 4654; Accept takes 1425 | 23131, 23132 |
| A3 | The Hub Computer shows 4655; the press moves 1425 to S4243 | 23133, 23134 |
| A4 | The Science Computer shows 5497; the press moves 1425 to S4244 and binds Carter's 1426 blurb | 23135, 23136, 23162 |
| A5 | Carter shows 4658; Accept takes 1426 and grants 4832 | 23163, 23165 |
| A7 | The case click moves 1426 to S4246 | 23167 |
| B2 | The Hub Computer shows 4659; the press moves 1426 to S4247 and removes 4832 | 23168, 23169 |
| B4 | Carter shows 4660; 1426 completes | 23171 |
| B5 | Carter shows 4662; Accept takes 1427 | 23191, 23192 |
| B6 | The note shows 4663; 1427 goes to S4249 | 23193 |
| B7 | Carter shows 4664; 1427 goes to S4250 and stays open | 23195 |
| B8 | The journal lists 1425 and 1427 as open and not 1426 | `u4-journal` |

The row passes on every step above. 1425 at S4244 and 1427 at S4250 staying open is the design (D-UB5 A), not a FAIL.

## Orchestrator only: verify list

- `fire_dialog_choice: matched` with dialog_id 4655 / button_id 456, 5497 / 457 and 4659 / 458 (UB-U6b: -1 is a FAIL for that button).
- `Content: advancing step` 1425 → 4243 → 4244; 1426 → 4246 → 4247; 1427 → 4249 → 4250.
- `Content: granting item` 4832 at A5; the remove-item lines and `RemoveInventoryItemByType: persisted` for 4832 at B2.
- `Content: completing mission` 1426 once; no `completing mission` 1425 or 1427.
