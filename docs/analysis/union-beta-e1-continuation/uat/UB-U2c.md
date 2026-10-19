# UB-U2c: each object's draw matches its name

| Field | Value |
|---|---|
| Row | UB-U2c: stand at each object (templates 2320-2326, 2328-2335) and compare its draw with its name. The draw matches; a wrong or missing draw names the template. Fallback mesh is 445's ring switch |
| Runnable | Yes (the Asgard deck objects are reached by the ungated beam, so no Asgard character is needed) |
| Instance | `default` |
| Character | `Uat Fxbetahumdefa` |
| Fixture | F-H5 (restaged; the gate plaza, 1423 completed). Run B continues with no restage |
| Budget | run A 45, run B 22 calls |
| Piggyback | Mimir's stance for [UB-U2](UB-U2.md) (on the deck) |

**From the seeds** (server metres; one stop per template, the first spawn of each):

| Stop | Stand | Template (tag) at |
|---|---|---|
| Cage | (712.4, -65.99, 407.5) | 2320 (XenoCage_1) (712.389, -66.99, 411.698) |
| Scanners | (746.5, -18.5, 78.0) | 2330 (AsgardScanner_1-5) around (746.5, -19.6, 68.6) |
| Deck | by the beam, landing (698.98, 157.93, 70.36) | 2331 (AnalyticalMachine) (696.44, 157.77, 64.36); 2334 (ConsciousnessSimulator) (699.44, 157.93, 67.36); Mimir 2305 (696.44, 157.77, 67.36) |
| Hub | (1014.9, 13.57, 508.35) | 2322 (HubComputer) (1018.405, 13.90, 506.851); 2323 (AdminWorkstation) (1018.405, 13.57, 509.851) |
| Crates | (1005.5, 14.37, 280.0) | 2325 (Crate_Arc42, Crate_Arc66) (1007.015 / 1004.015, 13.4, 275.82) |
| Fire | (1026.5, 14.37, 268.2) | 2328 (FireSuppressantManager) (1019.327, 13.37, 271.18); 2329 (FireControlHub) (1022.327, 13.37, 268.18) |
| Infirmary | (960.5, 16.97, 289.5) | 2332 (HologramEmitter) (961.3, 15.97, 284.9); 2333 (DisabledDrone) (959.8, 15.97, 284.9) |
| Science | (653.9, 20.77, 810.2) | 2321 (FourPiecesCase) (654.9, 19.77, 814.1); 2324 (ScienceComputer) (652.9, 19.77, 814.1) |
| Ruin | (122.2, 28.17, 352.6) | 2335 (RuinNote) (122.2, 27.17, 356.8) |
| Camp | (-548.6, 7.77, 143.5) | 2326 (JaffaBox) (-546.1, 6.77, 146.5) |

What each should look like, from `entity_templates_beta_e1.sql`: 2320 a xenobiology sample cage, 2321 an artifact display case, 2322 a hub computer, 2323 an admin workstation, 2324 a science computer, 2325 wrecked SGC shipping crates, 2326 a box of Jaffa tools, 2328 an Asgard fire suppressant manager, 2329 a fire control hub, 2330 an Asgard terminal console (an environmental reading device), 2331 an analytical machine, 2332 a hologram emitter, 2333 a disabled Goa'uld drone (it draws a creature body, not a prop mesh: that is by design), 2334 a consciousness simulator, 2335 a note with the nameplate "Note". Template 2327 is reserved and has no row, so it is not checked.

**The beam.** Spawn 149 (template 3, tag `BetaSite_00000007`) at (746.555, -19.21, 68.75) sits among the scanners; clicking it runs chain 23314, which moves the player to the deck (698.98, 157.93, 70.36) with no mission check. Spawn 151 (`BetaSite_Beam_000007_Ship`) at (700.48, 157.95, 70.365) on the deck runs chain 23315 back down to (745.05, -19.60, 68.75).

## Orchestrator before the run

Stage F-H5 again and dispatch run A at once; run B follows on the same lease with no logout.

## Driver steps

At each stop: TP(the stand) unless the step says otherwise. FIND each template of the stop and record every match's id, `name`, `distance_m` and positions. Then `client_camera {"lease_id":L,"face_entity_id":<the stop's first object>,"pitch_deg":-15,"zoom":250}` and SHOT(`u2c-<stop>`), stop one of `cage`, `scanners`, `deck`, `hub`, `crates`, `fire`, `infirmary`, `science`, `ruin`, `camp`.

Run A:

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}`. WAIT17. DIALOG.
2. Cage: TP, FIND(2320), camera, SHOT.
3. Scanners: TP, FIND(2330), camera, SHOT.
4. Beam up: FIND(3), take the match nearest (746.6, -19.2, 68.8), APPROACH, TAP(it). Then `client_batch {"lease_id":L,"steps":[{"op":"wait","ms":5000},{"op":"player_state","fields":["world_id","position"]}, UTC]}`. Record the position (expect the deck, y about 158).
5. Deck (no TP): FIND(2331), FIND(2334), FIND(2305), camera on 2331, SHOT(`u2c-deck`). Then camera on Mimir and SHOT(`u2-mimir`).
6. Beam down: FIND(3), take the match nearest (700.5, 158.0, 70.4), TAP(it) (no APPROACH: it is about 1.5 m away). Then the step-4 batch. Record the position (expect about (745.05, -19.6, 68.75)). If you are still on the deck, TP(746.5, -18.5, 78.0) and record "beam down failed".
7. Hub: TP, FIND(2322), FIND(2323), camera, SHOT.
8. Crates: TP, FIND(2325), camera, SHOT.
9. Fire: TP, FIND(2328), FIND(2329), camera, SHOT.
10. Infirmary: TP, FIND(2332), FIND(2333), camera on 2332, SHOT. 2333 is a drone body lying in place: record whether it draws.

Run B:

1. `lab_ensure_in_world {"lease_id":L,"character":"Uat Fxbetahumdefa"}` (no relog). DIALOG.
2. Science: TP, FIND(2321), FIND(2324), camera, SHOT.
3. Ruin: TP, FIND(2335), camera, SHOT.
4. Camp: TP, FIND(2326), camera, SHOT.

## Stop conditions

- Step A4 does not reach the deck: record it, skip A5 and A6, and go on at A7. The deck objects are then `not-run` for this row.

## Expected and pass/fail

| Check | Pass when | Evidence |
|---|---|---|
| A. Found | Each template is found at its seeded x and z (within 1 m), with its name | FIND results |
| B. Draw | Each screenshot shows an object whose look matches its name (a cage, a scanner, a computer, a crate, ...) | the screenshots |
| C. Beam | A4 lands on the deck and A6 back on the ground | A4 and A6 positions |

The row passes on A and B for every template. Name each template whose draw is wrong, missing or the fallback (445's ring switch) in the evidence. C is a byproduct: record it, and give it to [UB-U5](UB-U5.md) as early evidence.

## Orchestrator only: verify list

- A4: `fire_interact_tag: matched` for `BetaSite_00000007`, then `Content: teleporting entity` (chain 23314) for this character.
- A6: the same for `BetaSite_Beam_000007_Ship` (chain 23315).
- Judge every screenshot yourself against the template's name; the driver's claim is not the grade.
