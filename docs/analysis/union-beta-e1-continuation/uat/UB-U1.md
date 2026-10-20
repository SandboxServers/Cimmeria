# UB-U1: Omega to Beta arrival and back (Human)

| Field | Value |
|---|---|
| Row | UB-U1: Human on Omega with 1423 at S4223 dials gate 6, then dials back. Expect arrival at (635.761, -60.134, 388.448) facing +X, the Omega address granted, and the dial back lands at the Omega gate |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` (create on first use, [fixtures.md § Characters](fixtures.md#characters)) |
| Fixture | F-H1 (Omega, respawner 180, 1423 S4223) |
| Budget | 30 calls |
| Piggyback | UB-U1b, Human half (the journal before and after 1423 S4223 → S4224) |

## Orchestrator before the run

1. Stage F-H1 ([fixtures.md](fixtures.md)). If `betahum` does not exist yet, skip staging, let the driver create it (step 1), log it out with a one-call driver, then stage and rerun.
2. Note the UTC time; the verify window starts here.

## Driver steps

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. Then WAIT17. Expect world 18 (Omega) near (34.7, -51.4, -138.0).
2. DIALOG.
3. JOURNAL(`u1b-before`).
4. CHATX(`/gmdhd 6`). Then `client_batch` with a 5000 ms wait and UTC.
5. WALK(34.7, -50.63, -147.5) (into the Omega gate's event horizon).
6. `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":20000},{"op":"lua","id":"world","chunk":"return getCurrentWorldID()"}, UTC]}`. If `world` is not 23, run this batch once more.
7. WAIT17. Then `client_player_state {"lease_id":L,"fields":["world_id","position","yaw","heading","rotation"]}` (if `fields_missing`, record `fields_available` and call once with the ones that mention yaw, heading, rotation or direction).
8. SHOT(`u1-arrival`).
9. JOURNAL(`u1b-after`).
10. READ(`u1addr`, `address`). Then READ(`u1sys`, `Beta`).
11. DIALOG. Then CHATX(`/gmdhd 5`). Then `client_batch` with a 5000 ms wait and UTC.
12. WALK(619.4, -59.31, 388.9) (into the Beta gate).
13. Same as step 6, but expect world 18.
14. WAIT17. SHOT(`u1-back-at-omega`).

## Stop conditions

- Step 1 fails twice: BLOCKED, report the error.
- After step 6 twice, the world is still 18: record "no gate travel", SHOT, skip to step 14 and report.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Arrival | Step 7 world 23, position within 2 m of (635.761, -60.134, 388.448) | step 7 result, `u1-arrival` |
| B. Facing | Facing +X: yaw or heading reads as east (+X) | step 7 result, `u1-arrival` (gate behind or beside you, plaza ahead) |
| C. Address | The server log shows the Omega address (5) granted to this character on the Beta arrival | server log |
| D. Step | 1423 advances S4223 → S4224 on the arrival and Carter's dialog set 5433 is bound (slot 2300) | server log |
| E. Back | Step 13 world 18 and step 14 at the Omega gate (within 15 m of (34.705, -50.634, -149.092)) | step 14 position, `u1-back-at-omega` |

The row passes on A, C, D and E. B is recorded; if the client gives no yaw field, grade B from the screenshot and say so.

## Orchestrator only: verify list

From the run window in `cimmeria-cell`, for this character:

- `Gate travel: dial accepted, gate opens in 4s` (or `gmDHD: dialing stargate`) after step 4, and again after step 11.
- A `player_loaded` on world 23, then `Content: granting stargate address` with address 5 (chain 23001; 23002 or 23003 firing for a Human is a FAIL).
- `Content: advancing step` for 1423 to step 4224 (chain 23020) and `Content: adding dialog set` 5433 (slot 2300). With debug on: `resolve_event: chain matched` for 23001 and 23020.
- No `spawn_world_unknown` event.
- `lab fixture applied` before any of these. Without it, the run is BLOCKED.
