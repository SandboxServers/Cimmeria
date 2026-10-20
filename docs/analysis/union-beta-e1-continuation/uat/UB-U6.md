# UB-U6: the Free Jaffa arc (1500, 1502)

| Field | Value |
|---|---|
| Row | UB-U6: Free Jaffa (archetype 7) at 1500 S4369 arrives, talks to Carter, takes the ring pair to the Nine Peaks camp and talks to Ras'nor. 1500 and 1502 complete. 1503 is not built (D-UB5 A) |
| Runnable | Yes, once the user has made `Fxbetafjdefa` by hand ([fixtures.md § Characters](fixtures.md#characters)) |
| Instance | `default` |
| Character | `Fxbetafjdefa` |
| Fixture | F-FJ1 ((1009.4, 13.57, 515.85), 4.5 m north of Carter; 1500 S4369) |
| Budget | 48 calls |
| Piggyback | [UB-U2b](UB-U2b.md) (dialogs 5042 and 5043), [UB-U6b](UB-U6b.md) (buttons 575 and 576) |

**From the seeds.** The Beta login at S4369 advances 1500 to S4370 and binds Carter (23220); Carter then shows 5026 and completes 1500 (23222), which binds Ras'nor's offer (23240). Ras'nor shows the offer 5041 (23241); any answer to it accepts 1502 at S4376 (23242). Walking into the NinePeaks region (centre (-549.1, 6.77, 142.5), r 20) moves S4376 to S4377 (23243); if it does not fire, the Jaffa box click does (23244). The box shows 5042; its button (575) moves to S4378 and opens 5043 (23246); 5043's button (576) grants the medallion 5175 and moves to S4481 (23248). Ras'nor then shows 5044, takes the medallion and completes 1502 (23250).

**The ring trip.** Hub pad region 2300 at (146.716, 8.77, 403.273) and SW pad 2301 at (-791.6, -51.43, -92.6); each ring console (template 3) stands on its pad. The SW pad is about 340 m from the camp, so the run teleports from the SW pad to the camp; the ring leg itself is graded in full by [UB-U8](UB-U8.md).

Stands (server metres): hub ring (146.7, 9.77, 409.3), 6 m off the hub console. Outside the camp (-549.1, 7.77, 175.0), 32 m north of the region centre. Camp (-548.6, 7.77, 143.5), 4.0 m from Ras'nor.

## Orchestrator before the run

1. During the approved session, ask the user to make `Fxbetafjdefa` (Free Jaffa) on the GM account and leave it at character select.
2. Stage F-FJ1 and dispatch at once.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetafjdefa"}`. WAIT17. DIALOG.
2. FIND(2300). CLICK(Carter). DLGREAD (expect 5026). FINISH. DIALOG (record what it returned).
3. TP(146.7, 9.77, 409.3). RING(2300, 2301, 146.716, 8.77, 403.273), label `u6-sw-arrival`. Expect a final position within 3 m of (-791.6, -51.43, -92.6).
4. TP(-548.6, 7.77, 143.5). FIND(2306) (Ras'nor). APPROACH.
5. CLICK(Ras'nor). DLGREAD: record the header and the text. ACCEPT.
6. TP(-549.1, 7.77, 175.0) (out of the camp). FIND(2326) (the Jaffa box). `client_move_to {"lease_id":L,"entity_id":<the box id>}` (walk back in: this is what fires the camp's enter event). Record the result.
7. CLICK(the box). DLGREAD: record the header line exactly, even when blank. SHOT(`u2b-5042`).
8. PRESS(`Hack the box.`). Its last DLGREAD should show a new dialog: record its header exactly. SHOT(`u2b-5043`).
9. PRESS(`Hack the box.`).
10. BAG. Record the Sacred Tribal Medallion (5175).
11. FIND(2306). APPROACH. CLICK(Ras'nor). DLGREAD (expect 5044). FINISH.
12. BAG. Record whether the medallion is gone.

## Stop conditions

- Step 3 does not reach the SW pad: record the position, SHOT, and go on at step 4 (the TP does not need the ring).
- Step 8 opens no second dialog: CLICK(the box) once more (chain 23247 shows 5043 at S4378), then step 9.

## Expected and pass/fail

| Step | Pass when | Chain |
|---|---|---|
| 1 | 1500 S4369 → S4370; the Omega address granted | 23220, 23003 |
| 2 | Carter shows 5026; 1500 completes | 23222 |
| 3 | The ring moves you to the SW pad | ring regions 2300 → 2301 |
| 5 | Ras'nor shows 5041; 1502 accepted at S4376 | 23241, 23242 |
| 6-7 | 1502 at S4377 (by the region or the box) and the box shows 5042 | 23243 or 23244; 23245 |
| 8 | The press reaches S4378 and 5043 opens | 23246 |
| 9 | The press grants 5175 and reaches S4481 | 23248 |
| 11 | Ras'nor shows 5044, takes 5175 and completes 1502 | 23250 |

The row passes when 1500 and 1502 both complete with the steps above. Step 3 failing alone fails the ring part only: grade it under UB-U8 and write it in the evidence.

## Orchestrator only: verify list

- `lab fixture applied`; `Content: granting stargate address` 5 (23003; 23001 or 23002 firing for this character is a FAIL); `Content: advancing step` 1500 → 4370.
- `Content: displaying dialog` 5026 and `Content: completing mission` 1500, then `Content: adding dialog set` 5885 (slot 2306).
- Ring: `Sent onRingTransporterList`, the `setRingTransporterDestination` packet (to_server, 2300 → 2301), `onSequence` packets and the `ring.passenger_released` event.
- Step 6: whether `enter_region` NinePeaks advanced 1502 (23243) or the box click did (23244). Record which: both are a pass.
- `fire_dialog_choice: matched` dialog_id 5042 button_id 575, then 5043 button_id 576 (UB-U6b: -1 is a FAIL for that button).
- `Content: granting item` 5175, then at step 11 the remove-item lines with `persisted`, and `Content: completing mission` 1502.
- No 1503 offer or chain.
