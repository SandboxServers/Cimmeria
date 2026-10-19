# UB-01a: Beta gate 6 arrival pin, respawners 6 and 7, exemption removed (pilot)

> **Type:** How-to (work packet) · **Index:** [beta-work-packets.md](../beta-work-packets.md) · **Wave:** B0

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 40k |
| Branch / worktree | `union-beta-e1/ub-01a-gate-pin` / `ub-01a` |
| Commit subject | `feat(beta-e1): UB-01a gate 6 arrival pin and real respawner positions` |
| Depends on | UB-00b (ARRIVAL, RESPAWN_6, RESPAWN_7 sealed) and UB-00c (ID ranges clear). Ship after UB-00c's docs PR. |
| Ship order | First code packet of the campaign |

## Why

A traveller to Beta_Site_Evo_1 lands on the gate row (617.866, -59.314, 388.971), inside the gate's own volume, because gate 6 has no arrival pin. Respawners 6 and 7 sit at (0, 0, 0), so a player who dies on Beta respawns at the world origin, off the mesh. `seeded_respawners_are_not_at_the_world_origin` exempts both through `UNAUTHORED_BY_DESIGN`. This packet fixes all three and removes the exemption. The test that would catch a regression already exists.

## Contract excerpt

- Gate 6: `db/resources/Worlds/Seed/stargates.sql:79`, world 23, yaw 1.571, row (617.866, -59.314, 388.971). The `stargates` table has `arrival_x, arrival_y, arrival_z, arrival_yaw` columns (see the Agnos row, gate 15, in the same file).
- Arrival pin = the map's only PlayerStart (635.761, -60.134, 388.448), yaw 1.571. RESTORED_SOURCE position (map pin), CIMMERIA_RECONSTRUCTION use. It is 17.9 m out on the gate's +X facing axis, on nav component 251. **Use UB-00b's ARRIVAL row if it differs**, and say so in the report.
- Respawners: `db/resources/Worlds/Seed/respawners.sql`, rows 6 (`Respawner`) and 7 (`Respawner_bet_Infirmary`). Use UB-00b's RESPAWN_6 and RESPAWN_7 values.
- Exemption: `crates/cell-catalog/src/cell/spawner/tests/live_db_loaders.rs:255-256`, `const UNAUTHORED_BY_DESIGN: [(i32, &str); 2] = [(6, "Beta_Site_Evo_1"), (7, "Beta_Site_Evo_1")];`.
- Guard precedent: `crates/cell-world/src/cell/gate_dakara_e1_tests.rs`. Its test `live_db_dakara_gate_arrival_pin_is_on_the_plaza_outside_the_gate_volume` is the model. It is registered in `crates/cell-world/src/cell/mod.rs:39-40` as `#[cfg(test)] mod gate_dakara_e1_tests;`.

## Rules (binding)

1. PowerShell only; start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-01a;`. Absolute paths.
2. Never run `cargo` directly; use `pwsh tools/build-lane/lane.ps1 cargo ...`. Live-DB tests go through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads your `sgw_ub-01a`. Run in the foreground and check the test count is above 0.
3. Edit only the allowlisted files.
4. Seed files are CRLF. Edit them in place; after any rewrite, normalise with `[IO.File]::WriteAllText('<abs>', ([IO.File]::ReadAllText('<abs>') -replace "\r?\n", "\r\n"))`.
5. Seeds live in `db/resources/` only. No `db/scripts/` and no migrations.
6. Files stay under 500 lines (the new test file should land at about 200).
7. Never touch the `sgw` database. No lab and no client.
8. No `git stash`, `git worktree prune` or push. One local commit.
9. Keep the existing row order and column order. Add the four arrival columns to gate 6's column list the way the Agnos row does.
10. Put a provenance comment above each edited row.

## Allowlist

| File | Change |
|---|---|
| `db/resources/Worlds/Seed/stargates.sql` | gate 6 row: add the arrival columns |
| `db/resources/Worlds/Seed/respawners.sql` | rows 6 and 7: real positions; update the comment above them |
| `crates/cell-catalog/src/cell/spawner/tests/live_db_loaders.rs` | remove `UNAUTHORED_BY_DESIGN` and its use |
| `crates/cell-world/src/cell/gate_beta_e1_tests.rs` | new |
| `crates/cell-world/src/cell/mod.rs` | `#[cfg(test)] mod gate_beta_e1_tests;` next to `gate_dakara_e1_tests` |

## Steps

1. **Gate 6.** Replace the row at `stargates.sql:79` with the same values plus the arrival columns:

   ```sql
   -- UB-01a (CIMMERIA_RECONSTRUCTION): arrival pin = the map's only PlayerStart
   -- (635.761, -60.134, 388.448), 17.9 m out on gate 6's +X facing axis, nav component 251.
   INSERT INTO stargates (address1, address2, address3, address4, address5, address6, address_origin, stargate_id, name, pitch, prefab_sequence, roll, world_id, x_pos, y_pos, yaw, z_pos, event_set_id, arrival_x, arrival_y, arrival_z, arrival_yaw) VALUES (4, 7, 28, 38, 10, 2, 21, 6, 'Beta Site E1', 0, 'Beta_Site_Evo_1.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq', 0, 23, 617.86602800000003, -59.313999000000003, 1.571, 388.97100799999998, 10009, 635.761, -60.134, 388.448, 1.571);
   ```

2. **Respawners.** Set rows 6 and 7 to RESPAWN_6 and RESPAWN_7 from `docs/analysis/union-beta-e1-continuation/placement-seal.md`. Replace the comment block above them (it says the room has no point set) with a UB-01a CIMMERIA_RECONSTRUCTION comment that names the seal.
3. **Exemption.** In `live_db_loaders.rs`, delete `UNAUTHORED_BY_DESIGN` and the branch that skips its rows. Keep the Castle checkpoint logic as it is. The existing test must now pass on its own merits.
4. **Guard test.** Create `crates/cell-world/src/cell/gate_beta_e1_tests.rs` by copying the helper functions (`repo_file`, the mesh loader, `v`, `disc_off_mesh`, `reaches`, `dist_xz`, `dhd_spawns`) from `gate_dakara_e1_tests.rs`. Point them at `../../data/spaces/beta_site_evo_1.nav`. Constants: `BETA_GATE: i32 = 6`, `PIN = [635.761, -60.134, 388.448]`, `PLAYER_START = PIN`, and a `CONTROL` point that UB-00b names on component 70 (west valley).

   Tests, each `#[tokio::test]` with `require_db_or_skip!()` and a `///` comment naming the revert it catches:
   - `live_db_beta_gate_arrival_pin_is_on_component_251_outside_the_gate_volume`: copy the Dakara test with these changes. The distance band is `15.0..=20.0`. Instead of `PLAZA_FLOOR_Y`, check `(arrival[1] - PIN[1]).abs() <= 0.25`. Reachability: the pin reaches the gate row and DHD spawn 40, and it does not reach `CONTROL`.
   - `live_db_beta_respawners_stand_on_component_251_and_reach_the_gate`: load `respawners` for world 23 (rows 6 and 7), and assert each is off the origin, standable at r = 0.6, and `reaches(mesh, pos, PIN)`.

5. **Revert proof.** (a) Remove the arrival columns from gate 6, rerun `gate_beta_e1`, and expect a failure. (b) Set row 7 back to (0, 0, 0), rerun `seeded_respawners_are_not_at_the_world_origin`, and expect a failure. Restore both and rerun green. Quote all four results in the report.

## Lane checks (in order)

```powershell
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world -p cimmeria-cell-catalog
Set-Location <wt>; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world -p cimmeria-cell-catalog --all-targets -- -D warnings
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 gate_beta_e1
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 seeded_respawners_are_not_at_the_world_origin
Set-Location <wt>; pwsh tools/build-lane/live-db-test.ps1 gate_dakara_e1
```

The last run proves the shared way-back guard still holds.

## Acceptance

- Gate 6 has an arrival pin. Respawners 6 and 7 are real positions, the exemption is gone, and all three live filters are green with counts above 0.
- SigNoz (`cimmeria-cell`), checked live in UB-90 row UB-U1: a gate crossing to Beta_Site_Evo_1 logs **no** `reason = "arrival_off_navmesh"` warning and no `reason = "arrival_unrecoverable"` error (`crates/cell-world/src/cell/arrival.rs`; see `docs/gameplay/gate-travel.md` "Where the placement is chosen"). Today's respawner fallback cannot fire because every Beta respawner sits at (0,0,0). A death on Beta logs the respawn at RESPAWN_6 or RESPAWN_7. Grep the exact log text and fields in `arrival.rs` and the respawn path before naming them in the report.

## Docs rows

None. The doc-update-map has no row for per-world seed coordinates, and `docs/gameplay/gate-travel.md` describes the mechanism, not each world's pins. Do not edit it.

## Return

`UB-01a <done|blocked|failed> <sha|-> C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ub-01a\.claude\reports\union-beta-e1\UB-01a-coder.md`
