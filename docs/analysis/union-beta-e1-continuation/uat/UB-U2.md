# UB-U2: cast and life NPCs stand on the ground

| Field | Value |
|---|---|
| Row | UB-U2: walk the gate plaza, Command Post, barracks, Quartermaster and Science front. Every cast and life NPC stands on the ground, not in the air or in a wall. Clicking a life NPC does nothing |
| Runnable | Yes |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H5 (restaged after UB-U7; (633.0, -59.83, 405.0), 1423 completed) |
| Budget | 45 calls |
| Piggyback | none |

**From the seeds** (`spawnlist_beta_e1_hub.sql`, `_life.sql`, `_ninepeaks.sql`; server metres, y up):

| Area | Stand | Who (template) at |
|---|---|---|
| Gate plaza | (633.0, -59.83, 405.0) | Life_Gate_1 (2308) (626, -60.83, 399); Life_Gate_2 (2308) (642.5, -60.83, 374.5) |
| Command Post | (1009.4, 13.57, 507.85) | Carter (2300) (1011.405, 13.57, 511.851); Felger (2301) (1007.405, 13.57, 511.851); Life_Cmd_1/2 (2308) (1000, 13.57, 500 / 497) |
| Barracks | (1028.0, 8.37, 361.3) | Life_Hut_1 (2308) (1033.7, 7.37, 361.349); Life_Hut_2 (2308) (1033.7, 7.37, 424.563) |
| Barracks south | (1030.0, 14.38, 314.8) | Debeaux (2304) (1034.146, 13.38, 314.764) |
| Quartermaster | (1008.0, 14.44, 280.0) | Satterfield (2302) (1004.144, 13.44, 276.69); Life_QM_1/2 (2308) (1002.433, 13.44, 281.388), (999.113, 13.57, 273.732) |
| Science front | (626.6, 21.64, 777.0) | Life_Sci_1 (2308) (622.6, 20.64, 777) |
| Nine Peaks camp (extra) | (-548.6, 7.77, 143.5) | Ras'nor (2306) (-545.1, 6.77, 141.5); SickJaffa 1-3 (2307); Life_Camp 1-3 (2309) around (-557, 6.77, 145) |

The camp is not in the row's list but holds the rest of the cast and life NPCs, so it is checked too. Mimir (2305) is on the Asgard deck and is checked by [UB-U2c](UB-U2c.md), which goes up there.

## Orchestrator before the run

Stage F-H5 again and dispatch at once.

## Driver steps

For each area in the table, in order: TP(the stand) (skip the TP for the gate plaza: you start there). Then FIND each template listed for that area and record every match's id, name, `distance_m` and positions (both `server` and `client` when given). Then `client_camera {"lease_id":L,"face_entity_id":<the first NPC's id>,"pitch_deg":-10,"zoom":300}` and SHOT(`u2-<area>`) with area one of `gate`, `cmd`, `barracks`, `debeaux`, `qm`, `sci`, `camp`.

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG.
2. Gate plaza: FIND(2308), camera, SHOT(`u2-gate`).
3. Command Post: TP, FIND(2300), FIND(2301), FIND(2308), camera on Life_Cmd_1, SHOT(`u2-cmd`).
4. The life-NPC click: APPROACH(Life_Cmd_1's id), TAP(it). Then `client_ui_state {"lease_id":L}`: record any window that opened. DIALOG.
5. Barracks: TP, FIND(2308), camera, SHOT(`u2-barracks`).
6. Barracks south: TP, FIND(2304), camera, SHOT(`u2-debeaux`).
7. Quartermaster: TP, FIND(2302), FIND(2308), camera on Satterfield, SHOT(`u2-qm`).
8. Science front: TP, FIND(2308), camera, SHOT(`u2-sci`).
9. Camp: TP, FIND(2306), FIND(2307), FIND(2309), camera on Ras'nor, SHOT(`u2-camp`).

## Stop conditions

- Budget: if fewer than 8 calls are left before step 9, skip the camp and report; the camp is extra.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Present | Every NPC in the table is found once, within 1 m of its seeded x and z | FIND results |
| B. On the ground | In each screenshot the NPCs stand on the floor: not floating, not sunk, not in a wall. A client y that differs from the seed by more than 1 m is a FAIL for that NPC | screenshots, FIND `client` vs `server` |
| C. Life click | Step 4 opens no window and the server shows no dialog | step 4, server log |

The row passes on A, B and C. Name each failing NPC by its tag in the evidence.

## Orchestrator only: verify list

- Step 4: `fire_interact_tag: no chains matched` (or no interact line at all) and no `Content: displaying dialog` for that click. Any dialog is a FAIL of C.
- Look at each screenshot yourself; the driver's "on the ground" claim is not the grade.
