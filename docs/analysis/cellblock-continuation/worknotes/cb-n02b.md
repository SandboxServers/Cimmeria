# CB-N02b: keep the 329 cell door button live across relog

**Fault (gap A of [cb-n02s.md](cb-n02s.md)).** Chains 1014/1015 (the Prisoner 329 dialog choice at step 2114) advance mission 638 to step 2115 and set `329_CellDoorButton` (spawn 22, template 22, base flags 0) to `{"op": "|", "mask": 256}`. Chain 1016 starts the Livewire game when the player clicks that button at step 2115; chain 1017 clears the bit at victory and advances to 2116. Interaction flags live in cell memory and a login builds a fresh instance with flags 0, so a relog at step 2115 leaves the button inert and the player stuck in the cell.

**Fix.** Chain 1028 in `db/resources/Content/Seed/castle_cellblock_chains.sql`. On `player_loaded` for `Castle_CellBlock`, when `mission_638_step_2115_status` is `active`, it runs `set_interaction_type` on `329_CellDoorButton` with `{"op": "|", "mask": 256}`. Chain 1017 clears the bit at 2116. Same shape as chain 1045. The change adds seed rows only and no executor code.

**Telemetry.** No new events. After a relog at step 2115, SigNoz shows the DEBUG event `Content: set interaction type` (target `cimmeria_cell_content`) with `chain_id` 1028, `entity_tag` `329_CellDoorButton` and `mask` 256. At any other step, `content.resolve` logs `reason="condition_failed"` for chain 1028 and no set-interaction event follows. In the UAT, a spawn-order miss shows as the DEBUG line `Content: entity tag not found for SetInteractionType`: the button or vial was not yet spawned at `player_loaded`, so the restore did nothing.

**Test.** `live_db_mission_638_login_restores_door_button_bit_only_at_step_2115` in `crates/cell-content/src/cell/content/chain_replay_tests/mission_638.rs` resolves a `PlayerLoaded` event for chain 1028 at steps 2115, 2114 and 2116.

**CB-U03 steps (Human and Jaffa).**
1. Reach step 2115 of mission 638 (the Prisoner 329 dialog choice).
2. Relog.
3. Click the cell door button.
4. Expect the Livewire game to open.

**Untested.** No live client run. The telemetry sentence is not yet confirmed in SigNoz.
