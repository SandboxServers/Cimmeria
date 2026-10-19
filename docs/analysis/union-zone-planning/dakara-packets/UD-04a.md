# UD-04a: Human exit valve on Dakara_E1 (chain 63500)

> **Type:** How-to (work packet) · **Wave:** U1 · **Status:** BlockedDecision D-UD3 (option A); BlockedDependency UD-00 (the ID row)
> **Index:** [dakara-work-packets.md](../dakara-work-packets.md)

| Field | Value |
|---|---|
| Role | `packet-coder` |
| Size | about 35k working tokens |
| Branch / worktree | `union-dakara/ud-04a-human-exit-valve` / `ud-04a` |
| Commit subject | `feat(content): Dakara_E1 Omega address for a Human on mission 1570 (UD-04a)` |
| Depends on | D-UD3 = A and "Union ships it"; UD-00 merged. If DK absorbs it into DK-10b instead, this packet is dropped. |
| Ship order | With or before SGC US-24 (chain 3123-3131, the C1 departure), which is what first puts a Human on Dakara |

## Why

A Human who arrives from the SGC on mission 1570 (crossing C1) lands on Dakara_E1 with no stargate address. The only Dakara grant, chain 8001, is `archetype eq 7`. SGC_W1 has no DHD, so they cannot leave until DK-33 at the end of 1654, which D-DK5 blocks. Chain 63500 teaches them Omega Site (gate 5), as 8001 does for the Free Jaffa. A native arrival at Omega teaches Dakara back, so it gives a two-way trip. Provenance: **CIMMERIA_RECONSTRUCTION**. Evidence: free-jaffa-route.md rule 5 (keep the early Omega address), D-DK2 (Omega only at the start), and chain 8001 as the precedent.

DK-U10 still holds: a Human with **no** 1570 (a GM visitor) gets nothing. DK's test `live_db_dakara_arrival_sends_a_visitor_nothing` stages archetype 1 with no missions, so it must stay green.

## Contract excerpt

- Content tables and shapes, copied from `db/resources/Content/Seed/dakara_e1_space_chains.sql` (chain 8001):
  ```sql
  INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority) VALUES (...);
  INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order) VALUES (...);
  INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order) VALUES (...);
  INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order) VALUES (...);
  ```
- `mission_status` reads the context key `mission_<id>_status`. A missing key counts as `not_active` (`crates/content-engine/src/conditions/tests.rs:48-57`). `neq not_active` therefore matches both `active` and `completed`.
- `fire_player_loaded` fills the mission keys from the entity's `missions`, through `populate_mission_context`.
- `grant_stargate_address` is idempotent. It logs INFO `Content: granting stargate address` (`executor/stargate.rs:181`), sends the client `updateStargateAddress(id, 1, 0)` and sends the base `CellToBaseMsg::GrantStargateAddress`.
- Archetypes: 7 is the Free Jaffa, 8 the Praxis Jaffa, and 1 to 6 are Tau'ri. DK-10b gates its Tau'ri rows `archetype lte 6`.
- **The 1..6 range is intentional and differs from the Union Human gate** (`gte '1'` and `lte '4'`, [execution contract](../execution-contract.md), Shared definitions). This chain is a rescue valve that mirrors DK-10b's Tau'ri rows: it admits Asgard (5) and Goa'uld (6) players too, because a Tau'ri-side player standing on Dakara_E1 on 1570 must not be stranded, and 1570 is only reachable through the Human SGC route. Do not narrow it to 1..4 and do not widen it to 7 or 8. The packet pins both ends with tests (archetypes 5 and 6 admitted, 7, 8 and unstamped refused).

## Rules (binding; packet-coder skips CLAUDE.md)

1. PowerShell only. Start every command with `Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a;`. Use absolute paths. No bash.
2. Never run `cargo` directly. Use `pwsh tools/build-lane/lane.ps1 cargo <cmd> -p <crate>`, and run live-DB tests with `pwsh tools/build-lane/live-db-test.ps1 <filter>`. Run them in the foreground. Check that the lane log lands under your worktree's `logs/`.
3. Edit only the five files in the allowlist.
4. The seed `.sql` file and any `docs/**/*.md` file are CRLF. Write the file, then convert it with `[IO.File]` on the absolute path.
5. Seeds live only in `db/resources/`. No `db/scripts/*.sql`.
6. Files stay under 500 lines (hard cap 700).
7. Never touch the `sgw` database. The live-DB script uses your worktree's own `sgw_ud-04a`. No lab, no client.
8. No `git stash`, no `git worktree prune`, no push. One local commit.
9. Never edit `dakara_e1_space_chains.sql`, `dakara_e1_space.rs` or any DK file. Chains 8001-8499 belong to DK.
10. Give each SQL block a comment with its provenance label.

## Allowlist

| File | Change |
|---|---|
| `db/resources/Content/Seed/union_dakara_e1_chains.sql` | **New**, CRLF. Chain 63500 |
| `db/database.sql` | Add one line after `\ir resources/Content/Seed/dakara_e1_space_chains.sql` |
| `crates/cell-content/src/cell/content/chain_replay_tests/union_dakara_e1.rs` | **New**. Tests |
| `crates/cell-content/src/cell/content/chain_replay_tests/mod.rs` | Add `mod union_dakara_e1;` between `mod start_minigame_difficulty;` and `mod world_condition;` (on `main` at 2026-10-18; if SGC's `mod union_sgc;` is already there, put it before that one, `d` sorts before `s`) |
| `.github/instructions/content-chains.instructions.md` | Add a § "Union Dakara_E1 - 63500-63549" block after the Dakara_E1 block |

## Steps

1. **Seed file.** Create `db/resources/Content/Seed/union_dakara_e1_chains.sql`:
   ```sql
   -- Union Dakara_E1 chains (docs/analysis/union-zone-planning/dakara-work-packets.md).
   -- Block 63500-63549 (UD-00 ledger; the coordinator merges it into id-allocation.md). DK owns 8001-8499; never put DK chains here.

   -- 63500 - CIMMERIA_RECONSTRUCTION. A Human on mission 1570 (crossing C1 from the SGC)
   -- learns Omega Site, as chain 8001 does for the Free Jaffa. Evidence: free-jaffa-route.md
   -- rule 5, D-DK2, chain 8001. Idempotent (grant_stargate_address), so no `once`.
   -- A Human visitor with no 1570 gets nothing (DK-U10).
   INSERT INTO content_chains (chain_id, description, scope_type, scope_id, enabled, priority)
   VALUES (63500, 'Union Dakara_E1 - Human on 1570: learn the Omega Site address (UD-04a)', 'space', 61, true, 0);
   INSERT INTO content_triggers (chain_id, event_type, event_key, scope, once, sort_order)
   VALUES (63500, 'player_loaded', 'Dakara_E1', 'player', false, 0);
   INSERT INTO content_conditions (chain_id, condition_type, target_id, target_key, operator, value, sort_order)
   VALUES (63500, 'archetype', NULL, NULL, 'gte', '1', 0),
          (63500, 'archetype', NULL, NULL, 'lte', '6', 1),
          (63500, 'mission_status', 1570, NULL, 'neq', 'not_active', 2);
   INSERT INTO content_actions (chain_id, action_type, target_id, target_key, params, delay_ms, sort_order)
   VALUES (63500, 'grant_stargate_address', 5, NULL, '{}', 0, 0);
   ```
   Why two archetype rows: `Condition::Archetype` reads a missing archetype as `-1` (`crates/content-engine/src/conditions/mod.rs:333-338`), and `-1 lte 6` is true. Without `gte 1`, an entity whose archetype was never stamped would pass. `lte` and `gte` are parsed by `crates/content-engine/src/loader/condition.rs:177` and its neighbours; re-check before writing. Convert the file to CRLF.
2. **database.sql.** After line `\ir resources/Content/Seed/dakara_e1_space_chains.sql`, add `\ir resources/Content/Seed/union_dakara_e1_chains.sql`. Keep the file's existing line endings: check `[IO.File]::ReadAllText(...)` for `"`r`n"` and match it.
3. **Test module** `union_dakara_e1.rs`. Model it on `chain_replay_tests/dakara_e1_space.rs`: read lines 1-133 for the imports, `arrive`, `staged_space`, `stage_player` and `enter_world`, and copy those helpers into your file. They are private there, so don't import them. Changes from the copy:
   - `arrive` takes an extra `missions: &[(i32, &str)]` and sets `mission_<id>_status` for each.
   - `stage_player` takes an extra `with_1570: Option<bool>`. `Some(false)` adds an active mission, `Some(true)` a completed one, using `MissionInstance::new(1570, 4906, vec![])` from `cimmeria_entity::missions` (see `crates/cell-content/src/cell/content/mod.rs:86-88, 394-404, 430-432`). Call `.complete()` for the completed case.
   - Use other entity ids: `PLAYER_EID 63501`, `PLAYER_ID 63502`, `ACCOUNT_ID 63503`.

   Constants: `CHAIN = 63500`, `OMEGA_SITE_GATE = 5`, `HUMAN = 1`, `FREE_JAFFA = 7`, `PRAXIS_JAFFA = 8`, `DAKARA = "Dakara_E1"`.

   Tests, each `#[tokio::test]`, starting `let pool = require_db_or_skip!(); let engine = build_engine(Some(&pool)).await;`:
   - `live_db_human_on_1570_resolves_the_omega_grant_on_dakara`: for each status in `["active", "completed"]` and each archetype 1 to 6, `arrive(DAKARA, Some(a), &[(1570, status)])`, filter `resolved.actions` to `chain == CHAIN`, and assert it equals `vec![(CHAIN, Action::GrantStargateAddress { stargate_id: 5 })]`. Also `super::assert_no_deferred_actions(&resolved, CHAIN)`. Reverting the seed (no chain 63500) makes it fail.
   - `live_db_valve_stays_shut_for_a_visitor_a_jaffa_and_other_worlds`:
     - archetypes 1 to 6 with no missions, and with `(1570, "not_active")`;
     - archetypes 7 and 8, and `None`, with `(1570, "active")`;
     - archetype 1 with `(1570, "active")` on `"Dakara_E1_StoryRm"` (world 62), `"SGC_W1"` (58) and `"Omega_Site"` (18). These are the exact `worlds.world` values in `db/resources/Worlds/Seed/worlds.sql`.

     Assert that no action has `chain == CHAIN`. It fails if either archetype row is dropped (the `None` case catches a dropped `gte 1`), the `lte 6` row is flipped to `gte`, the mission condition is dropped or flipped to `eq not_active`, or the trigger's world key is cleared.
   - `live_db_human_on_1570_is_granted_omega_executed`: `staged_space` with archetype `Some(1)` and `with_1570 Some(false)`, then `enter_world`. Assert the sent messages include an `EntityMethodCall` with `method_index == UPDATE_STARGATE_ADDRESS` and args `5i32.to_le_bytes() ++ [1, 0]`, and a `GrantStargateAddress { stargate_id: 5, player_id: PLAYER_ID, .. }`. Assert `known_stargates == vec![5]`. A second `enter_world` sends no second `GrantStargateAddress`. Copy the destructuring style from `dakara_e1_space.rs:265-305`, but match by `iter().any`, not by exact slice, because other DK chains may add messages later.
   - `live_db_human_visitor_without_1570_is_sent_nothing_executed`: archetype `Some(1)`, `with_1570 None`. `enter_world` sends nothing and `known_stargates` stays empty (DK-U10).

   Give each test a `///` doc comment that says what reverting would break.
4. **mod.rs.** Add `mod union_dakara_e1;` in its alphabetical slot.
5. **content-chains.instructions.md.** After the Dakara_E1 block (the line `A new player_loaded chain without a mission gate needs the same property.`), add:
   ````markdown
   ### Union Dakara_E1 - 63500-63549, one file

   The Union continuation ([docs/analysis/union-zone-planning/dakara-work-packets.md](../../docs/analysis/union-zone-planning/dakara-work-packets.md)) owns `63500-63799`; chains use `63500-63549`.

   ```text
   union_dakara_e1_chains.sql      Human exit valve, bridge: 63500-63549 (63500 used)
   ```

   Chain 63500 is a `player_loaded` grant gated on `archetype gte 1`, `archetype lte 6` and `mission_status 1570 neq not_active`, so a Human visitor without 1570 still gets nothing. An `lte` archetype gate needs the `gte 1` floor, because a missing archetype reads as `-1`.
   ````
   Match the file's existing line endings.
6. **Lane checks, in order, in the foreground:**
   ```powershell
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a; pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-content
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a; pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-content --all-targets -- -D warnings
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a; pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::union_dakara_e1
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a; pwsh tools/build-lane/live-db-test.ps1 chain_replay_tests::dakara_e1_space
   Set-Location C:\Users\Steve\source\projects\Cimmeria\.claude\worktrees\ud-04a; pwsh tools/build-lane/live-db-test.ps1 gate_dakara_e1
   ```
   The last two are DK's guards and must stay green. `gate_dakara_e1` includes `live_db_every_address_a_player_loaded_chain_grants_has_a_way_back`, which now also covers 63500. Confirm the test counts in the output are above zero: a filter that matches nothing passes silently.
7. **Revert proof.** Temporarily comment out the four INSERTs of chain 63500 and rerun step 6's third command. The two positive tests must fail. Restore the file, rerun the command, and record both results in your report.
8. Commit the five files only.

## Acceptance

- The four new tests pass, and the two DK suites and `gate_dakara_e1` stay green. The revert proof is in the report.
- Telemetry (SigNoz, service `cimmeria-cell`): on a Human's Dakara entry with 1570 active, an INFO line `Content: granting stargate address` with `chain_id=63500` and `stargate_id=5`. Nothing for a visitor. The report names the field set copied from `executor/stargate.rs:170-182`.
- The diff touches the five allowlisted files only. The seed file is CRLF (CR count = LF count).

## Docs rows (doc-update-map)

- "Game systems, content chains, or content-engine actions": `.github/instructions/content-chains.instructions.md` (step 5). `docs/content/mission-chains.md` waits for UD-99.

## Return

One line: `UD-04a <done|blocked|failed> <sha|-> <report path>`.
