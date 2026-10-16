# CB-N02c: Ambernol vial interact bit across relog

**Fault (gap B).** Chain 1031 (enter `Castle_Cellblock.Region11` while mission 639 step 2117 is active) advances to step 2145 and sets `ArmYourself_AmbernolVial` (spawn 12, template 18, base flags 0) to `{"op": "|", "mask": 1073741824}`. Interaction flags live in cell memory, so a relog builds the vial with flags 0. Chain 1032 (interact at 2145) destroys the vial and advances to 2144. A relog at step 2145 therefore leaves the vial unusable, and 1031 cannot fire again because 2117 is complete. Mission 639 stalls.

**Fix.** Chain 1036 (`player_loaded`, `Castle_CellBlock`, scope `player`, not once) re-paints the bit when `mission_639_step_2145_status` is `active`. It is the same shape as chain 1028 (CB-N02b) and chain 1045. The bit is needed only at 2145, so no other step gets it. The restore cannot outlive 2145 because chain 1032 destroys the vial (seed :1026, `destroy_entity`), so no later step sees a re-painted vial.

**Telemetry acceptance (not run, no live client).** After a relog at step 2145, expect DEBUG `Content: set interaction type` with `chain_id` 1036, `entity_tag` `ArmYourself_AmbernolVial`, `mask` 1073741824. At any other step, `content.resolve` should log `reason="condition_failed"` for chain 1036 and no set-interaction event. In the UAT, a spawn-order miss shows as the DEBUG line `Content: entity tag not found for SetInteractionType`: the button or vial was not yet spawned at `player_loaded`, so the restore did nothing.

**Test.** `live_db_chain_1036_restores_vial_bit_only_at_step_2145` in `chain_replay_tests/mission_639.rs` (resolve level, D-CC4). No executor code changed.

**CB-U03 steps (Human and Jaffa).**
1. Reach step 2145 of mission 639 (enter Region11 while step 2117 is active).
2. Relog.
3. Use the Ambernol vial.
4. Expect the vial to be picked up and the drone fight to start.

**Untested.** No live client run. CB-U03 is UATPending.
