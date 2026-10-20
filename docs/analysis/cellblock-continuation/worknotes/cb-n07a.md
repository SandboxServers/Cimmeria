# CB-N07a worknote: barracks escape response

Packet: [w4-packets.md](../w4-packets.md#cb-n07a--barracks-alarm-bark).
Seal: [cb-n07-seal.md](cb-n07-seal.md) (verdict approve, CB-N07m). `## Copy` and `## SQL` match the packet byte for byte.

## What was added

- Chain 11020 in `db/resources/Content/Seed/castle_cellblock_chains.sql`: `enter_region('Castle_Cellblock.Region12')`, once, gated on `step_status(687, 2355) = 'active'`, one `npc_bark` to screen 200170 (speaker `NID Guard`, say). No spawn, aggression, threat or objective change.
- Screen 200170 (dialog 60160, index 1) in `db/resources/Dialogs/Seed/dialog_screens.sql`; the sequence value moves to 200170. Dialog 60160 is never displayed.
- Test file `crates/cell-content/src/cell/content/chain_replay_tests/cellblock_barracks_response.rs`:
  - `live_db_cellblock_barracks_response_barks_once_while_2355_is_active`
  - `live_db_cellblock_barracks_response_refuses_the_crate_and_cleared_phases`
  - `live_db_cellblock_barracks_response_refuses_another_room`
- Once set in `na13_chain_armed_aggro.rs` (`live_db_once_chains_match_the_seeded_set`) grows to include 11020.
- `docs/content/mission-chains.md`: section "Cellblock barracks response (CB-N07a)".

## Mutation proofs

Each mutation was made in the seed, run with `live-db-test.ps1 cellblock_barracks_response`, then restored (SHA-256 of the seed matched the pre-mutation value after restore).

- (a) Condition value `'active'` to `'completed'`: `barks_once_while_2355_is_active` fails with `chain 11020 must resolve one NpcBark: []` (0 vs 1). `refuses_the_crate_and_cleared_phases` also fails, on the cleared phase, because the chain now resolves there.
- (b) Trigger key `Castle_Cellblock.Region12` to `Castle_Cellblock.Region6`: `barks_once_while_2355_is_active` fails (0 vs 1) and `refuses_another_room` fails with `chain 11020 must not resolve outside Region12`.

Extra proof: removing `11020` from the once set in `na13_chain_armed_aggro.rs` makes `live_db_once_chains_match_the_seeded_set` fail (left seven ids, right six). Restored.

## Lanes

- `cargo fmt -p cimmeria-cell-content`: ok
- `cargo fmt --all -- --check`: ok
- `cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings`: ok
- `cargo nextest run -p cimmeria-cell-content --filter-expr 'test(cellblock_barracks_response)'`: 3 passed
- `cargo nextest run -p cimmeria-content-engine`: 296 passed (seed linters green)
- `live-db-test.ps1 cellblock_barracks_response`: 3 passed, none skipped
- `live-db-test.ps1 marsh_barks`: 14 passed
- `live-db-test.ps1 cellblock_ambience`: 3 passed
- `live-db-test.ps1 once_chains_match_the_seeded_set`: 1 passed

## Open

CB-U11 is owner-run (UATPending): the lab check of the client hint for Region12 (A1) and the in-game bark. Not run here (no lab).
