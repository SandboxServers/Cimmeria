# CB-N02d: keep the retrieval drone hostile across relog

**Fault (gap C, degraded play, not a strand).** Chain 1032 arms
`ArmYourself_PrisonerRetrievalUnit` (spawnlist row 10, template 4, seeded
NEUTRAL) with `set_aggression` level 1 when the player takes the Ambernol
vial, then advances to step 2144. Step 2144 needs objectives 2482 (kill the
drone) and 2484 (take cover). A relog at 2144 builds a fresh instance with
the drone NEUTRAL again, so it never attacks the player.

**Fix.** Chain 1037 (`castle_cellblock_chains.sql`, directly after chain
1032) re-arms the drone on `player_loaded` for `Castle_CellBlock` while step
2144 is active and objective 2482 is not completed. Action: `set_aggression`
level 1 only. No `generate_threat` (same shape as chain 1009, the 622 guard
re-arm), and no replay of sequence 10001.

**Telemetry.** After a relog at 2144 with the drone alive: INFO, target
`content`, `event="set_aggression"` with `chain_id` 1037 and `tag`
`ArmYourself_PrisonerRetrievalUnit`, `to` HOSTILE. If the drone is not
spawned yet at login, the WARN `event="set_aggression_tag_miss"` appears
instead; that is a finding for the UAT, not a code fault.

**CB-U03 steps.**
1. Reach step 2144 (vial taken, objective 2482 open).
2. Relog before killing the drone.
3. Expect the drone to show hostile and engage once in range and sight.
4. Confirm the telemetry line above.

**Tested.** `live_db_chain_1037_rearms_drone_only_while_kill_open_at_step_2144`
(`chain_replay_tests/mission_639_drone_relog.rs`) resolves chain 1037 at
resolve level for four mission states: 2144 active with 2482 open (exactly
one `SetAggression(drone, 1)`), 2482 completed, step 2145, and step 2343.
Revert proof: without the four 1037 statements the loader returns `None`
and the test fails with `drone re-arm chain 1037 must exist`.

**Untested.** The in-world effect (drone turns hostile and engages after a
real relog) is not covered by a test; it needs CB-U03 on a live client.
The spawn timing at login (tag-miss case) is not covered either.
