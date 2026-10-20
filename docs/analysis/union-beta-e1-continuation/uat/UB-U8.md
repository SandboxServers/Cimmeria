# UB-U8: the Beta ring pair, out and back

| Field | Value |
|---|---|
| Row | UB-U8: Free Jaffa at 1502's first step takes ring 148 near the gate to the SW ring and back. The ring UI lists the destination and you arrive and return. Ring regions 2300 and 2301 |
| Runnable | Yes, once `Fxbetafjdefa` exists |
| Instance | `default` |
| Character | `Fxbetafjdefa` |
| Fixture | F-FJ2 ((146.7, 9.77, 409.3), 6 m off the hub ring pad; 1500 completed, 1502 S4376) |
| Budget | 25 calls |
| Piggyback | none |

**From the seeds.** Region 2300 'BetaE1_Ring_Hub' at (146.716, 8.77, 403.273), r 2.53, lists {2301}; region 2301 'BetaE1_Ring_SW' at (-791.6, -51.43, -92.6), r 2.53, lists {2300}. Both use event set 10000 and need no mission. Each pad has a ring console (template 3) on it: spawn 148 at the hub, spawn 23115 at the SW pad. The row's "near the gate" means the hub ring; it is about 470 m from the gate, and the nearer console (spawn 40 at the gate) is the DHD, not a ring.

The destination picker is a world-map click the lab cannot make, so the run picks with the stock `setRingTransporterDestination` binding (N3), as the Debug Area ring rows do.

## Orchestrator before the run

Stage F-FJ2 and dispatch at once. Turn the packet tap on.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Fxbetafjdefa"}`. WAIT17. DIALOG.
2. FIND(3). APPROACH. TAP(the console). Then `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":2000}, UTC]}` and `client_ui_state {"lease_id":L}`: record the visible windows. SHOT(`u8-picker-hub`).
3. `client_batch {"lease_id":L,"steps":[{"op":"lua","id":"pick","chunk":"return pcall(setRingTransporterDestination, 2300, 2301)"},{"op":"wait","ms":500}, UTC]}`. Record `pick`.
4. WALK(146.716, 8.77, 403.273). WAIT17. SHOT(`u8-sw-arrival`). Record the position.
5. WALK(-791.6, -51.43, -86.6) (6 m off the pad). Record the position.
6. FIND(3). APPROACH. TAP(the console). Then the step-2 wait batch, `client_ui_state`, SHOT(`u8-picker-sw`).
7. Same as step 3 with `2301, 2300`.
8. WALK(-791.6, -51.43, -92.6). WAIT17. SHOT(`u8-hub-return`). Record the position.

## Stop conditions

- Step 4's position is still at the hub after WAIT17: record it, SHOT, and stop (the return leg needs the SW pad).

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. List | The hub console sends a destination list naming 2301, and the SW console one naming 2300 | packet tap, `u8-picker-*` |
| B. Out | Step 4 position within 3 m of (-791.6, -51.43, -92.6) | step 4, `u8-sw-arrival` |
| C. Back | Step 8 position within 3 m of (146.716, 8.77, 403.273) | step 8, `u8-hub-return` |
| D. Picker | Record whether a picker window opened at steps 2 and 6 | `client_ui_state`, screenshots |

The row passes on A, B and C. D is recorded: the row says "the ring UI lists the destination", and a picker that does not draw while the list packet is sent is a client finding, not a server FAIL; write it in the evidence.

## Orchestrator only: verify list

- Two `Sent onRingTransporterList` lines (or `onRingTransporterList` to_client in the tap), the first listing 2301, the second 2300.
- Two `setRingTransporterDestination` packets to_server: 2300 → 2301, then 2301 → 2300.
- `onSequence` packets for each leg (at least two per leg) and two `ring.passenger_released` events.
- No 1502 step change (the ring needs no mission and fires no 1502 chain).
