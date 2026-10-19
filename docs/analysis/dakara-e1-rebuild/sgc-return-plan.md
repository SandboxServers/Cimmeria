# Dakara_E1: the SGC return for real gate travel (wave 6d, DK-SGC)

> Type: how-to (plan and work packets). Audience: the orchestrator, the `packet-coder` named in each packet, the read-only `packet-reviewer`, and the Union SGC coordinator (§ Notice).
> Written 2026-10-19 (architect) for the owner's **option C, 2026-10-19**: Dakara's gate travel to the SGC (gate 27) is real dialling, and the SGC return is built to serve it. Against `origin/main` @ `e7459ddcb` (#1928, DK-40b #1929). Ledger and status: [README](README.md#packets). Decisions: D-DK2, D-DK24, D-DK40, D-DK44, D-DK46 to D-DK48. Findings: F40 to F47.

## What option C asks for, and what it needs

Option C gives the SGC end of the trip three things:

1. A dial window at the SGC: an SGC DHD or an equivalent prop.
2. A way home: dial back and walk through gate 27.
3. An arrival on the Gate Room floor, so a player who dials gate 27 arrives standing up.

The trip is then two-way for everyone, including a player who dials gate 27 off-story. D-DK2 is accepted with it: the start grant is Omega Site only, and gate 27 is a later story grant (8313, 8432, 8433).

| Piece | State on `main` | What this plan does | Packet |
|---|---|---|---|
| Arrival pin on gate 27 | **Missing.** Row "SGC W1" (172.79, -8.47, 6.014) has no `arrival_*` pin. The row is inside gate volume 1008, and the nearest walkable point is 3.4 m away and 1.3 m lower (F45). DK-30b-r and DK-33-r now cross by real travel with no coordinates, so this row is where they land. | Pin it to the sealed P13 (167.00, -10.80, 18.00), yaw 0.0 (D-DK47) | DK-SGC-01 |
| Dial window at the SGC | **Missing.** World 58 has no prop with `INT_DHD` (bit 16). | Give template 37 "SGC Gate Control" (spawn 68, the Control Room screen 3.1 m from Harriman's P01) `interaction_type` 16 (D-DK46) | DK-SGC-02 |
| Address rows | **Nothing needed.** Every gate arrival teaches the origin and destination gates (F40). Anyone who arrives by gate already knows gate 27 and the gate they came from. | none | - |
| A return chain | **Nothing needed.** `try_open_dhd` opens the window for any `INT_DHD` prop, and the dial and the walk-through are the engine's own (F46). | none | - |
| Two-way guard (DK-01) | **Covers `player_loaded` grants only** (F44). Gate 27 is granted by a click and by an offer (8313, 8432, 8433), so the guard does not see it. | Add a guard for every enabled `grant_stargate_address`: the target world must be enterable and have a DHD | DK-SGC-02 |
| Story crossings (C1, C2b, C4) | US-24, US-30 and US-32 are open PRs, on Harriman's interact with `cross_world_teleport` and their own coordinates | Unchanged. The terminal is a general exit beside them (D-DK48) | SGC, not this plan |

### Findings this plan adds

| # | Finding | Evidence |
|---|---|---|
| F46 | **One seed bit makes a prop a DHD.** `try_open_dhd` (`crates/cell-interactions/src/cell/interactions/dhd.rs:40`) needs only `interaction_type_flags & INT_DHD` on the target. It emits `onDisplayDHD` (120) with the origin glyph of the world's lowest gate id: on world 58 that is gate 27, glyph 1. The interact handler calls it only after `fire_interact_tag` and `fire_interact_template` find no matching chain, and a chain matches only if its conditions pass. On `main` and on every open SGC branch, the terminal's tag chains are 3102 and 3104, both conditioned on 1568 step 4638. So the window opens at every other moment, and at 4638 the press shuts the gate as US-22 built it. The two chains change the glow bit (1073741824) only, with ops `\|` and `~`, so they never clear bit 16. Template 37 is spawned once, as spawn 68. | `interact.rs:183-248`, `event_dispatch/interaction.rs:55-56`, `sgc_w1_union_1568_chains.sql`, `spawnlist.sql:147` |
| F47 | **P13 is the sealed Gate Room arrival.** US-01 moved H3 from (172.79, -10.9, 12.0), which is off the mesh, to P13 (167.00, -10.80, 18.00): component 47, poly 887, 13.3 m from the gate row. Every probe within 1.5 m of P13 is floor. The strip on the gate's axis (x 172-175, z 8-18) has no floor polygon, so the pin sits off the axis: the dot of its out-vector with the gate's +Z facing is 0.90. Component 47 holds the Gate Room, the Control Room (P01) and Requisitions. The SGC chains that still carry the old H3 are test constants only (`union_sgc/rig.rs:65`) and a label-format fixture (`dakara_e1_missions/labels.rs:52`). | `docs/analysis/union-sgc-continuation/placement.md` rows P01, P13, ARRIVAL |

## Ownership and collisions

Option C has this campaign build at the SGC end. Every file the two packets touch was checked on `origin/main` and on the open SGC branches `union-sgc/us-24`, `us-30`, `us-31` and `us-32` (US-16 #1933 is escort-only; US-23 #1934 has merged).

| Item | Owner before | This plan | Collision check |
|---|---|---|---|
| Gate 27 row, `db/resources/Worlds/Seed/stargates.sql` line 69 | SGC (D-US9: "chains carry H3; the row is untouched") | DK-SGC-01 pins it. **This revises D-US9** under option C (§ Notice, item 1) | No SGC PR touches `stargates.sql`. The SGC's own chains keep their coordinates; no SGC chain lands on gate 27 |
| Template 37, `db/resources/Entities/Seed/entity_templates.sql` line 19 | SGC (US-22 uses spawn 68 by tag) | DK-SGC-02 sets `interaction_type` 16 on this row only | No open SGC PR touches the file. US-11 (merged) adds templates 5800+ on other lines. US-22's tests mask `& MISSION_OBJECT`, so the new bit cannot break them |
| `crates/cell-world/src/cell/gate_sgc_w1_tests.rs` (new) and one `mod` line in `cell/mod.rs` | - | DK-SGC-01 creates it, DK-SGC-02 adds to it | New file. `mod.rs` has no in-flight edit |
| `crates/cell-interactions/.../gate_travel/tests/debug_area_live_db.rs` | Debug Area (DA-07) | DK-SGC-01 empties `EXPECTED_OFF_MESH` | Must change in the same commit as the pin, or the Debug Area test fails |
| `docs/gameplay/gate-travel.md`, `docs/content/debug-area.md`, `docs/guides/unified-uat.md` row DA-U45 | shared | DK-SGC-01 and DK-SGC-02, one paragraph each | DK-99e edits other rows of `unified-uat.md`. A line conflict is impossible; a textual merge is clean |
| `.claude/agent-memory/movement-teleport-advisor/sgc-w1-crossings.md` | committed memory (#1930) | DK-SGC-02 corrects the "no DHD spawn" bullet | none |
| Chain ids | - | **None used.** The plan adds no chain, spawn, template, point set or dialog | No id-range risk |
| `union_sgc/rig.rs:65` `ARRIVAL` (old H3) | SGC | Not touched; asked of SGC (§ Notice, item 4) | - |

The live SGC worktrees (`us-16`, `us-23`, `us-24`, `us-30`, `us-31`, `us-32`) are read-only to this campaign. No DK-SGC packet edits any file in them or any SGC chain file.

## Wave 6d: packets, order, merge

| Step | Packet | Size | Role | Depends on | Worktree, branch |
|---|---|---|---|---|---|
| 1 | [DK-SGC-01](#dk-sgc-01-gate-27-arrival-pin-on-the-gate-room-floor) gate 27 arrival pin | about 35k tokens | packet-coder | none | `dk-sgc` (new, from `main`), `dakara/dk-sgc-return` |
| 2 | [DK-SGC-02](#dk-sgc-02-the-sgc-gate-control-terminal-dials-and-the-way-back-guard) SGC dial terminal and the way-back guard | about 30k tokens | packet-coder | DK-SGC-01's packet loop at `ship` | same worktree, a second commit |
| - | [UAT rows](#uat-rows-dk-u35-to-dk-u37) DK-U35 to DK-U37 | - | owner (lab) | the DK-SGC PR merged and deployed | DK-99e copies them into the UAT guide |

**Shape: one PR, two stacked commits in one worktree.** DK-SGC-02 adds tests to the file DK-SGC-01 creates, so the two coders run one after the other, never in parallel. Each packet gets its own review loop. DK-SGC-02's reviewer gets the range `main..HEAD` over both commits, and that pass stands in for the wave review (two packets, one branch, nothing else in the wave). Then `ops` ships one PR: `ship.ps1 pr -C <dk-sgc worktree>`, then `ship.ps1 merge <PR> --retire dk-sgc`.

**Runs beside wave 6c step 2.** DK-SGC shares no file with DK-14c, DK-15b, DK-30b-r, DK-33-r or DK-32c, so it can be dispatched now.

**Merge order:**

1. The DK-SGC PR, as soon as it is green. It waits on no SGC PR.
2. The DK-30b PR (DK-30b, DK-30b-n and the DK-30b-r commit `28376d808` on `dakara/dk-30b`) after **US-30 (#1937) and the DK-SGC PR** have merged.
3. The DK-33 PR (DK-33, DK-33-n and the DK-33-r commit `11bb681a9` on `dakara/dk-33`) after **US-31 (#1935) and the DK-SGC PR** have merged.

Once DK-SGC merges, the gate 27 grants in 8313, 8432 and 8433 lead to a world with a dial window, and DK-SGC-02's new guard fails any grant that does not. The D-DK40 and D-DK44 holds are replaced by "DK-SGC merged". DK-40b already merged (#1929), so this plan unblocks nothing there.

## Rules for both DK-SGC packets

1. `Host: windows`. PowerShell only: no bash, WSL or Git Bash. Use absolute paths inside your worktree. Prefix each lane call with `Set-Location <worktree>;` and check that the lane log lands under `logs/<worktree>/`.
2. Every compiling `cargo` call goes through the lane, run from the worktree root: `pwsh tools/build-lane/lane.ps1 cargo ...`. Never call `cargo` directly, and never add `--target-dir`. Live-DB tests run through `pwsh tools/build-lane/live-db-test.ps1 <filter>`, which reloads this worktree's own `sgw_<worktree>` database from `db/resources/` first. Run lane checks in the foreground, one at a time.
3. Never `git stash`, `git worktree prune`, `git push`, `gh pr`, `git rebase`, or delete a worktree. Commit once, locally, with the subject given and the attribution lines from your brief.
4. Edit only the files your packet names. Files stay under 700 lines (soft cap 500).
5. Every file under `docs/` and every seed file is CRLF; keep each file's line endings as they are. Check with `[IO.File]::ReadAllText('<absolute path>')`: a CRLF file must contain "`r`n" and no "`n" without a "`r" before it.
6. Seeds change in `db/resources/` only. Never add a `db/scripts/` migration, and never edit `db/database.sql`. Every changed row gets a comment above it with its evidence label and the decision it serves.
7. No new chain, spawn, template, dialog, sequence or address grant, and no client patch. Never edit an SGC chain file (`sgc_w1_union_*.sql`) or an SGC test (`chain_replay_tests/union_sgc/`).
8. Never launch the game client, the launcher or any lab tool. No IPs, credentials, tokens or account names in any file.
9. A test you add must fail when your seed change is reverted. Run that revert once, record the failing assertion message in your report, and then restore the change.
10. Write your report to `<worktree>\.claude\reports\dakara\<packet>-packet-coder.md`: files changed, each check's final `status=` line, the revert proof, the commit hash, and every deviation with its reason. Return exactly one line: `<packet> <done|blocked|failed> <commit-sha|-> <report-path>`. A failing check gets at most five fix rounds; after that, commit nothing and return `failed`. On a context stop past about 90k tokens, commit only passing work (subject suffix `(part N)`), write `docs/analysis/dakara-e1-rebuild/worknotes/<packet>.md` and return `blocked` with the reason `context`. If a test you did not write fails, do not edit it: report it and return `blocked`.

## Contract excerpt

- **Gate row loader.** `crate::cell::spawner::load_stargates(&pool)` returns `HashMap<i32, Stargate>`. Each `Stargate` has `world_name`, `x`, `y`, `z`, `yaw`, `address_origin`, `debug_dial_hub`, `arrival: Option<..>` and `desired_arrival() -> ([f32; 3], f32)`. `desired_arrival` gives the pin when one is seeded and the row otherwise.
- **Regions.** `load_regions_from_db(&pool)`, `REGION_FLAG_STARGATE`, `is_point_in_region(&region.points, p)`. SGC_W1's gate volume is point set 1008 `SGC_W1.Stargate`: a cylinder of r 2.5 and h 10, based at (172.934, -8.476, 4.91).
- **Navmesh.** `NavMesh::load(Path::new("../../data/spaces/sgc_w1.nav"))`, plus `is_point_valid` and `find_path`, the same as `gate_dakara_e1_tests.rs`.
- **The interact range.** `crate::cell::space_manager::MAX_INTERACT_DISTANCE` is 5.0, compared as a 3-D distance.
- **`INT_DHD`** is `cimmeria_entity::interaction_flags::INT_DHD` (16). The `entity_templates.interaction_type` column reads as `i64`.
- **Space setup for the way-back guard.** Copy it from `gate_dakara_e1_tests.rs:239-246`: `SpaceManager::new(1)`, `parse_spaces_xml(entities/spaces.xml)`, `create_startup_spaces(entities/cell_spaces.xml)`, then `world_is_enterable(world)`.
- **Sealed points** (`docs/analysis/union-sgc-continuation/placement.md`):

  | Name | Point | Meaning |
  |---|---|---|
  | P13 | (167.00, -10.80, 18.00) | the arrival, on component 47 |
  | P01 | (172.08, -6.40, 46.70) | Harriman's Control Room spot, on component 47, 3.1 m from spawn 68 |
  | P05 | (147.00, 1.30, -197.00) | the Failsafe, on component 3: on the mesh but unreachable from P13 |
  | Floor y | -10.85 | the Gate Room and Requisitions floor |

---

## DK-SGC-01: gate 27 arrival pin on the Gate Room floor

`Host: windows` · worktree `dk-sgc` (new, from `main`) · branch `dakara/dk-sgc-return` · role `packet-coder` · about 35k tokens · decisions D-DK44, D-DK47 · finding F45, F47.

**Goal.** A player who dials gate 27 arrives standing on the Gate Room floor at P13, facing into the room, not inside the gate volume. Seed row plus a live-DB seed guard. No code change.

**Files (only these):**

1. `db/resources/Worlds/Seed/stargates.sql`
2. `crates/cell-world/src/cell/gate_sgc_w1_tests.rs` (new)
3. `crates/cell-world/src/cell/mod.rs`
4. `crates/cell-interactions/src/cell/gate_travel/tests/debug_area_live_db.rs`
5. `docs/gameplay/gate-travel.md`
6. `docs/content/debug-area.md`
7. `docs/guides/unified-uat.md` (row DA-U45 only)

**Do.**

1. **Seed.** In `stargates.sql`, replace the gate 27 row (line 69, `... 27, 'SGC W1', ...`) with the pinned form. The column list is the gate 25 row's (line 121), and the values are unchanged except for the four new ones at the end:

   ```sql
   INSERT INTO stargates (address1, address2, address3, address4, address5, address6, address_origin, stargate_id, name, pitch, prefab_sequence, roll, world_id, x_pos, y_pos, yaw, z_pos, event_set_id, arrival_x, arrival_y, arrival_z, arrival_yaw) VALUES (28, 26, 5, 36, 11, 30, 1, 27, 'SGC W1', 0, 'SGC_W1.Main_Sequence.Prefabs.GLB-Stargate_Prefab_Seq', 0, 58, 172.78999300000001, -8.4700000000000006, 0, 6.0140000000000002, 10004, 167.00, -10.80, 18.00, 0.0);
   ```

   Above it, add a comment block in the style of gate 25's (lines 87-120), with these facts and no others:
   - The row is ORIGINAL_DATA, the gate prefab's origin. It lies inside gate volume 1008 `SGC_W1.Stargate` (r 2.5), and the nearest walkable point is 3.4 m away and 1.3 m lower.
   - The pin is PROJECT_FINAL_RECONSTRUCTION (Dakara rebuild DK-SGC-01, owner option C 2026-10-19, D-DK44, D-DK47). It is the Union SGC campaign's sealed P13 (US-01, `placement.md`): component 47, poly 887, 13.3 m from the row.
   - The pin is off the gate's axis because the axis strip x 172-175, z 8-18 has no floor polygon. Yaw 0.0 faces into the room, toward the Control Room.
   - World 58 is instanced, so `validate_gate_arrival` returns Unvalidated and accepts the pin as written. That is why the guard checks the pin against the mesh.
   - `.gotolocation SGC_W1` with no coordinates lands on this point too.
   - The three Gate Room Jaffa (spawns 65-67) are 14 m away. The SGC campaign's US-34 despawns them on return.
   - Guard: `gate_sgc_w1_tests.rs` in cimmeria-cell-world. To correct the pin, edit the four `arrival_*` values here and `PIN` there.

2. **Guard.** Create `gate_sgc_w1_tests.rs`. Copy the helpers `repo_file`, `v`, `disc_off_mesh`, `reaches` and `dist_xz` **verbatim** from `gate_dakara_e1_tests.rs`. Each gate test file keeps its own copy (the house pattern in `gate_beta_e1_tests.rs` and `dakara_start_tests.rs`); a shared module is out of scope. Add `sgc_mesh()`, which loads `../../data/spaces/sgc_w1.nav`. Start with a module doc comment in the style of the Dakara file's lines 1-16: what it guards, the seed-plus-real-mesh pairing, and why it walks paths. Constants:

   ```rust
   /// Stargate 27, `SGC W1`.
   const SGC_GATE: i32 = 27;
   /// The arrival pin as seeded: the Union SGC campaign's sealed P13.
   const PIN: [f32; 3] = [167.00, -10.80, 18.00];
   const PIN_YAW: f32 = 0.0;
   /// The Gate Room and Requisitions floor (placement.md, P10 and P13).
   const GATE_ROOM_FLOOR_Y: f32 = -10.85;
   /// P01, Harriman's Control Room spot, beside the Gate Control screen.
   const CONTROL_ROOM: [f32; 3] = [172.08, -6.40, 46.70];
   /// P05, the Failsafe: on the mesh (component 3), not reachable from the Gate Room (47).
   const CONTROL: [f32; 3] = [147.00, 1.30, -197.00];
   ```

   Test `live_db_sgc_w1_gate_arrival_pin_is_on_the_gate_room_floor_outside_the_gate_volume` (`#[tokio::test]`, `require_db_or_skip!()`). Follow the Dakara pin test (lines 109-203) with these differences:
   - Gate 27 is on `"SGC_W1"`. `gate.arrival.is_some()` holds, with the message "gate 27 lost its arrival pin: travellers land on the gate row again, inside the gate's own volume". `desired_arrival() == (PIN, PIN_YAW)`.
   - Facing: `along(gate.yaw) > 0.85` and `along(yaw) > 0.85`, not 0.99. Say in a comment that the axis has no floor, so the pin is 26 degrees off it. The distance from the row is within `5.0..=15.0`.
   - Volumes: the same two checks. The row is inside a stargate volume on SGC_W1 (the control), and the pin is inside none.
   - **No DHD assertion** (DK-SGC-02 adds the terminal).
   - Mesh: `(arrival[1] - GATE_ROOM_FLOOR_Y).abs() <= 0.25`. Discs at r 0.6 and 1.2 are fully on the mesh. `reaches(&mesh, arrival, CONTROL_ROOM)`. `mesh.is_point_valid(&v(CONTROL))`, and `!reaches(&mesh, arrival, CONTROL)`.
   - Doc comment: "**Guard (DK-SGC-01): gate 27's arrival pin.**", then what makes each assertion fail.

3. **Register.** In `crates/cell-world/src/cell/mod.rs`, after `mod gate_dakara_e1_tests;` (line 44), add:

   ```rust
   /// Seed guards for the SGC_W1 end of real gate travel (stargate 27's
   /// arrival pin, DK-SGC-01; the dial terminal and the way-back guard, DK-SGC-02).
   #[cfg(test)]
   mod gate_sgc_w1_tests;
   ```

4. **Debug Area list.** In `debug_area_live_db.rs`, lines 33-39: `const EXPECTED_OFF_MESH: [i32; 0] = [];`. Replace the doc comment with one line saying no offered gate's arrival is off its mesh, and that gate 27 was the last (pinned by DK-SGC-01, 2026-10-19). Change nothing else.

5. **Docs.**
   - `gate-travel.md` § Arrival placement, line 25. Replace the sentence that starts "Three gates are pinned:" with one that names the five pinned gates: 15 `Agnos` (keep its parenthesis), 29 `Debug Area` (the Z1 arrival), 6 `Beta Site E1`, 25 `Dakara E1` and 27 `SGC W1` (both below). Keep the Harset sentence.
   - After the `### Dakara E1 (gate 25)` subsection (it ends at line 53), add `### SGC W1 (gate 27)`. Give it one short paragraph: the row is inside volume 1008 and off the mesh; the pin is P13 (167.00, -10.80, 18.00), yaw 0.0, sealed by the Union SGC campaign (US-01) and pinned by DK-SGC-01 for real gate travel (Dakara D-DK44). Then a two-row check table, Navmesh (component 47 with the Control Room; discs at 0.6 and 1.2 m) and Gate volume (outside it, 13.3 m from the row, off the unwalkable axis strip), and one sentence on the guard file.
   - § What a GM can dial from the Debug Area, lines 213-219. Replace from "Applied as if each world enforced its mesh, one would fail..." to the end of line 219 with: "Applied as if each world enforced its mesh, every arrival would still pass. Gate 27 `SGC W1` was the last off-mesh row and is pinned since DK-SGC-01. A live-DB test fails if one stops passing." Delete the table and its trailing sentence.
   - `docs/content/debug-area.md`, lines 1234-1236. Replace the "**Known arrival problem.**" paragraph with: "**SGC W1 arrival.** Gate 27 lands on its pin on the Gate Room floor, 13 m in front of the gate (DK-SGC-01). Report it if you arrive in or under the gate." Keep the file's existing line wrapping style.
   - `docs/guides/unified-uat.md`, row DA-U45, notes cell. Replace "SGC W1 lands you 3.4 m off the navmesh." with "SGC W1 lands you on the Gate Room floor (gate 27's pin, DK-SGC-01)."

**Revert proof.** Put the old row back (no `arrival_*` columns) and run checks 5 and 6. The new test fails at "gate 27 lost its arrival pin", and `debug_area_live_db` fails with `[27]` against `[]`. Restore the pin.

**Checks, in order** (each must end `status=ok` or with every test passing):

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
2. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-interactions`
3. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
4. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-interactions --all-targets -- -D warnings`
5. `pwsh tools/build-lane/live-db-test.ps1 gate_sgc_w1_tests`
6. `pwsh tools/build-lane/live-db-test.ps1 debug_area_live_db`
7. `pwsh tools/build-lane/live-db-test.ps1 gate_dakara_e1_tests` (unchanged; a regression check)

If check 5 fails on a mesh assertion with the pin as given, do not move the pin. P13 was sealed against this mesh, so report the failing point and return `blocked`.

**Docs rows owed** ([doc-update map](../../agents/doc-update-map.md)): "Per-system game-mechanic docs" (`gate-travel.md`); "UAT steps" (`unified-uat.md` DA-U45); `debug-area.md` as the Debug Area's tester page.

**Commit:** `feat(seed): pin stargate 27's arrival to the SGC_W1 Gate Room floor (DK-SGC-01)`.

---

## DK-SGC-02: the SGC Gate Control terminal dials, and the way-back guard

`Host: windows` · worktree `dk-sgc` (after DK-SGC-01's commit and its packet loop at `ship`) · branch `dakara/dk-sgc-return` · role `packet-coder` · about 30k tokens · decisions D-DK40, D-DK46 · findings F44, F46.

**Goal.** At the SGC a right-click on the "SGC Gate Control" screen in the Control Room opens the dial window, so a player can dial home and walk through gate 27. The guard then holds for every content address grant, not only `player_loaded` ones: a granted gate's world must have a dial window.

**Files (only these):**

1. `db/resources/Entities/Seed/entity_templates.sql` (the template 37 row only)
2. `crates/cell-world/src/cell/gate_sgc_w1_tests.rs`
3. `docs/gameplay/gate-travel.md` (§ DHD interaction only)
4. `.claude/agent-memory/movement-teleport-advisor/sgc-w1-crossings.md`

**Do.**

1. **Seed.** In `entity_templates.sql`, the template 37 row (line 19, `VALUES (37, 'Em-Props.EM-ViewScreen02', 'GLB_Components.WorldObject_Small', NULL, 4, 0, NULL, ...`): change the sixth value, `interaction_type`, from `0` to `16`, so it reads `NULL, 4, 16, NULL,`. Change nothing else on the row: `static_interaction_sets` stays `'{}'`. On the blank line above the row, add a comment:
   - It is CIMMERIA_RECONSTRUCTION, Dakara rebuild DK-SGC-02, owner option C 2026-10-19 (D-DK40, D-DK46).
   - The SGC_W1 map has no DHD prop. The Gate Control screen (spawn 68, the only spawn of this template) carries `INT_DHD` (bit 16), so a right-click opens the dial window with gate 27's glyph.
   - The Union SGC chains 3102/3104 (1568 step 4638) still take the press at that step, because a matching `interact_tag` chain suppresses the window. Their glow ops (`|`/`~` on 1073741824) leave bit 16 alone.
   - Guard: `gate_sgc_w1_tests.rs`.

2. **Guard: the terminal.** In `gate_sgc_w1_tests.rs`, add `use cimmeria_entity::interaction_flags::INT_DHD;` and `use crate::cell::space_manager::MAX_INTERACT_DISTANCE;`. Add test `live_db_sgc_w1_has_one_dial_terminal_beside_harriman`:
   - Query `SELECT s.spawn_id, s.tag::text, s.template_id, s.x, s.y, s.z FROM resources.spawnlist s JOIN resources.entity_templates t ON t.template_id = s.template_id WHERE s.world_id = 58 AND t.interaction_type & $1 <> 0 AND s.set_name IS NULL ORDER BY s.spawn_id`, bound to `INT_DHD`. Assert exactly one row, equal to `(68, "SGC_W1_GateTerminal1", 37)` for the first three fields, with the message "SGC_W1 has no dial terminal: a player who dials gate 27 cannot dial home (D-DK40)".
   - The 3-D distance from the spawn to `CONTROL_ROOM` is `<= MAX_INTERACT_DISTANCE`, and `reaches(&sgc_mesh(), PIN, CONTROL_ROOM)` holds: the arrival walks to where the press is allowed.
   - The non-hub gates on `"SGC_W1"`, from `load_stargates`, are exactly `[27]`, and gate 27's `address_origin` is in `1..=38`, so the window gets a real glyph.

3. **Guard: every granted address has a way back.** Add test `live_db_every_granted_address_has_a_way_back`. It is the general form of `live_db_every_address_a_player_loaded_chain_grants_has_a_way_back` (`gate_dakara_e1_tests.rs:218`); leave that test as it is. Copy `dhd_spawns` verbatim from `gate_dakara_e1_tests.rs:84-98`.
   - Query `SELECT DISTINCT a.chain_id, a.target_id FROM resources.content_actions a JOIN resources.content_chains c ON c.chain_id = a.chain_id WHERE a.action_type::text = 'grant_stargate_address' AND c.enabled ORDER BY a.chain_id`, as `Vec<(i32, Option<i32>)>`.
   - Control: the list contains `(8001, Some(5))`, DK-01's Omega grant.
   - For each row, the target is not NULL and has a gate row, and that gate is not `debug_dial_hub`. Its world is `world_is_enterable`, using the space setup from the contract. `has_dhd(world)`, from `dhd_spawns`, holds, with the message "chain {id} grants stargate {target} on {world}, which has no DHD: a one-way trip".
   - Doc comment: "**Guard (DK-SGC-02, D-DK40): no address grant is a one-way trip.**" Add that it covers the click and offer grants of gate 27 (8313, 8432, 8433), which the `player_loaded` guard does not see (F44).

4. **Docs.** `gate-travel.md` § DHD interaction: after the paragraph that starts "Template 1 (`GLB-DHD_00`..." (line 95), add a paragraph:
   - SGC_W1 has no DHD prop on its map, so the Gate Control screen in the Control Room (template 37, spawn 68) carries `INT_DHD` instead (Dakara DK-SGC-02, owner option C).
   - A right-click opens the dial window with gate 27's glyph. At Union SGC mission 1568 step 4638 the same press shuts the gate (chain 3102), because a matching interact chain takes the click first.
   - Name both guard tests, and say that the second one covers every content address grant.

5. **Agent memory.** In `sgc-w1-crossings.md`:
   - Replace the bullet "World 58 has a gate volume (point set 1008) but no DHD spawn..." with: "World 58's dial window is the Gate Control screen (template 37, spawn 68, `INT_DHD` since DK-SGC-02), and gate 27 is pinned to P13 (DK-SGC-01). A player can dial home natively. Story crossings still run on Harriman's interact with `cross_world_teleport`."
   - Replace the "17 m under the floor" bullet with: "Gate 27's row is 3.4 m off the mesh and 1.3 m above the floor; travellers land on its pin, P13 (167.00, -10.80, 18.00)."
   - In the `description:` front matter, replace "content has no dial action" with "content has no dial action; a native dial home exists since DK-SGC". Leave the rest.

**Revert proof.** Set template 37's `interaction_type` back to `0` and run check 3. `live_db_sgc_w1_has_one_dial_terminal_beside_harriman` fails with "SGC_W1 has no dial terminal". Then, still at `0`, temporarily change chain 8001's target in `db/resources/Content/Seed/` from 5 to 27 and run check 3 again. `live_db_every_granted_address_has_a_way_back` fails with "...which has no DHD: a one-way trip" (the Dakara `player_loaded` guard fails too; that is expected). Restore both, and record both messages. This temporary edit of the 8001 row is the only exception to rule 4, and `git diff` must show no change outside the named files before you commit.

**Checks, in order:**

1. `pwsh tools/build-lane/lane.ps1 cargo fmt -p cimmeria-cell-world`
2. `pwsh tools/build-lane/lane.ps1 cargo clippy -p cimmeria-cell-world --all-targets -- -D warnings`
3. `pwsh tools/build-lane/live-db-test.ps1 gate_sgc_w1_tests`
4. `pwsh tools/build-lane/live-db-test.ps1 debug_area_gate_tests` (`live_db_every_seeded_dhd_opens_and_the_debug_area_has_one` must still pass: world 58 has gate 27, and template 37's mesh is not a `%DHD%` mesh)
5. `pwsh tools/build-lane/live-db-test.ps1 union_sgc::gate_room_1568` (US-22's terminal tests, a regression check)
6. `pwsh tools/build-lane/live-db-test.ps1 gate_dakara_e1_tests`

**Docs rows owed:** "Per-system game-mechanic docs" (`gate-travel.md`); project memory, committed with the change that produced it.

**Commit:** `feat(seed): SGC_W1 Gate Control screen opens the dial window; every address grant has a way back (DK-SGC-02)`.

---

## UAT rows DK-U35 to DK-U37

These are owner-run, after the DK-SGC PR is deployed. DK-99e copies them into the unified UAT guide; they are in the README's M4 and M5 checklist.

| # | Do | Expect | Notes |
|---|---|---|---|
| DK-U35 | At the SGC (any character; `.gotolocation SGC_W1` or a gate arrival), go up to the Control Room and right-click the "SGC Gate Control" screen beside Harriman. Dial Dakara E1 (or another gate you know) and walk down the ramp into gate 27 | The dial window opens on the first right-click and lists the gates you know. The gate opens, walking in loads Dakara at its plaza pin, and you can come back the same way | D-DK40, D-DK46 (MEDIUM: the client's cursor for this prop is unchecked). If no cursor or window appears, say so (fallback: bind dialog set 6891, or spawn a DHD prop). If the ramp stops you short of the gate, say where |
| DK-U36 | From the Dakara or Omega DHD, dial SGC W1 and walk through | You arrive standing on the Gate Room floor about 13 m in front of the gate and left of its axis, facing into the room, not in or under the gate | D-DK44, D-DK47. Until the SGC campaign's US-34 merges, the three Gate Room Jaffa (14 m away) may attack (D-US5) |
| DK-U37 | On a Tau'ri at 1568 step 4638 (Gate Control glowing), right-click the screen; then right-click it again | The first press shows 5383 and moves the step to 4639, and **no** dial window opens. The second press opens the dial window | F46: a matching interact chain takes the click. US-22 |

## Decisions

| # | Decision | Status | Reason |
|---|---|---|---|
| D-DK40 | The way back from the SGC | **Decided (owner, option C, 2026-10-19)**: built by DK-SGC-02 | The terminal is a general exit for everyone, on or off story |
| D-DK44 | Gate 27's arrival point | **Decided (owner, option C, 2026-10-19)**: pinned to P13 by DK-SGC-01 | H3 was resealed as P13 by US-01; the old H3 is off the mesh |
| D-DK46 | What carries the dial window at the SGC | **Decided (architect; option C allows "an equivalent dial window")**: template 37 gets `INT_DHD`. Confidence MEDIUM on the client cursor; DK-U35 checks it | No new spawn or template, so no id-range risk. It is the in-fiction gate control, and US-22's step 4638 press keeps working (F46). Fallbacks, in order: bind `'{6891}'` ("Dial Home Device (DHD)") on template 37, or spawn template 1 in the Control Room |
| D-DK47 | The pin's point and facing | **Decided (architect)**: P13 (167.00, -10.80, 18.00), yaw 0.0 | The SGC campaign's sealed floor point, 13.3 m from the row, off the unwalkable axis strip; facing +Z is into the room |
| D-DK48 | Should the SGC's story departures (C1, C2b, C4) become hand dials? | **BlockedDecision (owner, non-blocking)**; the SGC campaign's D-US6 | Recommended: no. Keep Harriman's interact for the story, with the terminal as the general exit. A hand dial at C2b or C4 would skip Harriman's step advance; that is recoverable (dial back, press Harriman), and the optional safety nets in § Notice, item 2 close it |

**SGC D-US4 and D-US6.** D-US6 ("walk into gate 27 instead of crossing on Harriman's interact") is **partly answered**: walking into gate 27 now works for any dial. Whether the story crossings move to it stays the owner's call (D-DK48). D-US4 (play sequences 10047 and 10060 before each Harriman crossing) is **not touched**: native dials play the client's own gate sequences, and Harriman's crossings keep their rows.

## Notice to the Union SGC coordinator

Post this through `ops` to the board's Union SGC topic before the DK-SGC PR opens. It is a notice, not a merge gate.

> Dakara D-DK40/D-DK44, owner option C (2026-10-19): Dakara's gate travel to the SGC is real dialling, and Dakara's wave 6d (DK-SGC, one PR) builds the SGC end. Files: `stargates.sql` (gate 27 row) and `entity_templates.sql` (template 37 row), plus Dakara-owned tests and docs. It uses no chain, spawn, template or dialog id, and it shares no file with #1933 to #1938.
>
> 1. **D-US9 revised.** Gate 27 gets an `arrival_*` pin at your sealed P13 (167.00, -10.80, 18.00), yaw 0.0. Your chains keep their own coordinates; none of them lands on gate 27.
> 2. **New dial window.** Template 37 ("SGC Gate Control", spawn 68) gets `INT_DHD` (16): a right-click opens the dial window. Your 3102/3104 keep the press at 1568/4638, and their glow ops leave bit 16 alone. A Tau'ri could now dial home by hand at 1651/4931 or 1655/4944 and skip Harriman's advance. That is recoverable (dial back, press Harriman). Optional safety nets are your call and your ids (spares 3143-3147, 3170, or 3178-3179 if free). Use 0 ms actions only, because delayed actions drop on handoff:
>    - N1: `stargate_crossed 'Dakara_E1'` with `world 58`. At 4931 with O5956 done, advance 4929 then 5171; at 4929, advance 5171.
>    - N2: `stargate_crossed 'Omega_Site'` at 4944 or 4941, advance to 4940.
> 3. **US-34** should key its despawns on world 58, archetype and 1568 status, as planned. A gate arrival is a fresh instance and fires `player_loaded`, so off-story arrivals are covered.
> 4. **Stale constant.** `union_sgc/rig.rs:65` `ARRIVAL` still holds the old H3 (172.79, -10.9, 12.0). Please move it to P13 when you next touch the rig.
> 5. **D-US6** is partly answered (walking into gate 27 works); D-US4 is untouched. The owner question is Dakara D-DK48, with the recommendation to keep Harriman for the story.

## Held packets: what changes for them

| Packet | Before | After this plan |
|---|---|---|
| DK-30b-r (`dakara/dk-30b`, `28376d808`) | Merge after US-30, D-DK40, D-DK44 | Merge after **US-30 (#1937) and DK-SGC** |
| DK-33-r (`dakara/dk-33`, `11bb681a9`) | Merge after US-31, D-DK44 | Merge after **US-31 (#1935) and DK-SGC** |
| DK-40b | Merged (#1929) | Nothing to unblock |
| DK-99e | Copies DK-U21 to DK-U34 | Also copies DK-U35 to DK-U37; its "waits on" suffix names DK-SGC instead of D-DK40/D-DK44 |
