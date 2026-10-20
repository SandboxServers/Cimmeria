# UB-U4b: the 1426 camera is granted, then taken back

| Field | Value |
|---|---|
| Row | UB-U4b: Human at 1425 S4244 runs chain 23165 or 23166, then finishes 1426. Item 4832 (the camera) appears in the bag, then 23169 removes it |
| Runnable | Yes, graded from [UB-U4](UB-U4.md) |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H4, through UB-U4 |
| Budget | none of its own |
| Piggyback | UB-U4 run A steps 5 and 6, run B steps 2 and 3 |

UB-U4 accepts 1426 at 1425 S4244, so the accept is chain 23165. Chain 23166 (accept after 1425 is completed) cannot run on Beta, because 1425 never completes there (D-UB5 A). The row's "or" is satisfied by 23165.

## Driver steps

None of its own.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Granted | Run A step 6's BAG lists the Digital Camera (4832), quantity 1, in the mission bag | BAG output, server log |
| B. Taken back | Run B step 3's BAG no longer lists it | BAG output, server log |
| C. Once | One grant and one removal | server log |

## Orchestrator only: verify list

- `fire_dialog_choice: matched` dialog_id 4658, then (with debug on) `resolve_event: chain matched` 23165, then `Content: granting item` 4832.
- After the 4659 press: `Content: RemoveItem → RemoveInventoryItemByType` and `RemoveInventoryItemByType: persisted` for 4832 (23169). The line `no instance of this design id` here is a FAIL.
