# AT-05b Worknote: FINAL v2 Seed Import

> Type: reference. Audience: ability-trees campaign coordinator.
> Updated: 2026-09-26. Companions: [README.md](../README.md), [work-packets.md](../work-packets.md), [audit.md](../audit.md), [source data](../source/README.md).

## Contract

- **Packet:** AT-05b, seed import (AT-05a rebased onto AT-01 plus the live-DB guards).
- **Decisions in force:** D-AT04 (Free Jaffa / Shol'va maps to `ARCHETYPE_Sholva` only, Jaffa gets no rows), D-AT06 (debug list 1 offers every node), D-AT09 (starter ability 1646 is the one allowed collision).
- **Branch:** `trees/at05-seed-import` (draft PR #807), in worktree `.claude/worktrees/agent-a9a037326bb90baa4`.
- **Base:** `origin/main` @ `b5130081` (AT-07, #812), which includes AT-01 (#813) and AT-E1 (#809). The old campaign-plan commit `cb80de95` was dropped (squash-merged as #805).
- **Owned paths:**
  - `tools/ability_trees/` (generator, from AT-05a)
  - `db/resources/Archetypes/Seed/archetype_ability_tree.sql`, `db/resources/Abilities/Seed/trainer_abilities.sql` (generated)
  - `docs/analysis/ability-trees/source/trainer_server_export.json` (generated)
  - `crates/services/src/ability_tree/tests/seed_live_db.rs` (new), `catalog_live_db.rs`, `tests/mod.rs`
  - `crates/services/src/base/world_entry/methods/player_load/meta.rs` (tree-walk test expectation, from AT-05a)
  - `docs/gameplay/ability-system.md`, `docs/gap-analysis.md`, this worknote
- **Read set:** `TREES-WORKER-RULES.md`; `work-packets.md` (contract, AT-05a, AT-05b); `README.md` (D-AT04, D-AT06, D-AT09); `handoffs/session-resume.md`; `crates/services/src/ability_tree/` (catalog, tests); `crates/services/src/cell/spawner/abilities.rs` (`load_trainer_abilities`); `base/world_entry/methods/progression/` (tests); `db/resources/Archetypes/Types/EArchetype.sql`; `db/resources/_primary_keys.sql`.

## Evidence

1. **Why PR #807's live-DB jobs failed at ~56 s.** Confirmed from the CI log of run 36193427193: `psql:db/resources/Archetypes/Seed/archetype_ability_tree.sql:14: ERROR: column "required_branch_points" of relation "archetype_ability_tree" does not exist`. The seed wrote AT-01's columns on a base without them. On the new base the fresh `db/database.sql` loads (`reload-db.sh`, `ON_ERROR_STOP=1`, exit 0).
2. **Rebase.** `git rebase --onto origin/main cb80de95`, and a second `git rebase origin/main` after #812 landed, were both conflict-free. #812's edit to `sgw_player.sql` does not overlap.
3. **Generator after rebase.** `python tools/ability_trees/generate_seed.py` rewrote nothing (`git status` clean), and `--check` and `--from-json --check` both exit 0.
4. **"The existing `handle_train_ability` live-DB fixture that assumed stub rows" does not exist.** No live-DB test calls the base `handle_train_ability` (`grep -rn handle_train_ability crates/services/src`). The cell-side `handle_train_ability_tests` and `train_trainer_agreement` build their trees in memory with `TreeNode::with_defaults`, so they never read the seed. The only seed-shape assertions were AT-01's `catalog_live_db` (stub defaults) and `meta.rs::ability_tree_walks_full_enum_range_without_panic` (moved to `1..=7` in AT-05a). Both are now correct for the real data.

## Validator output (for the PR description)

```text
Ability-tree seed validation: OK
  workbook      SGW_All_Classes_Progression_EMULATOR_FINAL_v2_LEVEL50.xlsx
  sha256        ad9af21448794ca1eea2bae609016b5b55ad12f2bedec542c5f3b84cf1b21dba
  nodes         439
  unique ids    419
  branches      21
  shared ids    19
  raw cost 0    59
  above lvl 20  243
  per archetype:
    Soldier                ARCHETYPE_Soldier         72  (Automatic Weapons 22, Heavy Weapons 25, Command 25)
    Commando               ARCHETYPE_Commando        64  (Demolitions 23, Stealth / Infiltration 22, Precision / Marksmanship 19)
    Scientist              ARCHETYPE_Scientist       59  (Medical 22, Support 20, Robotics 17)
    Archaeologist          ARCHETYPE_Archeologist    65  (Anthropology 21, Sociology 20, Archaeology 24)
    Asgard                 ARCHETYPE_Asgard          66  (Attack Programs 24, Defensive Programs 21, Scientific Programs 21)
    Free Jaffa / Shol'va   ARCHETYPE_Sholva          51  (Heritage 21, Tau'ri 8, Tactics 22)
    Goa'uld                ARCHETYPE_Goauld          62  (Ashrak 21, Battle Lord 21, Servant Lord 20)
    (ARCHETYPE_Jaffa             no rows, D-AT04)
  trainer list 1 rows: 439; other-list rows kept: 0
  note: allowed starter collision (D-AT09): Goa'uld / Servant Lord #2 (ability 1646) is granted at creation to char_def [10, 19]
check: committed files match the generator
```

## Design decisions

- The guards load through `AbilityTreeCatalog::load`, the loader the cell uses, so they pin what the server sees and not only what the table holds. The trainer and ability matches are SQL anti-joins, because the trainer list has no catalog of its own in `ability_tree`.
- One test beyond the acceptance list: `seed_prerequisites_are_nodes_of_the_same_branch`. A cross-branch or missing prerequisite would make a node permanently untrainable, and the generator's check for it does not run in CI.
- `seed_every_tree_row_has_an_ability` duplicates the foreign key on purpose. `AbilityTreeCatalog::load` inner-joins `abilities`, so a dropped constraint would lose rows silently.
- `catalog_live_db` now compares every loaded node's v2 columns against its raw row, so a loader column swap fails. A non-default floor stops the comparison passing vacuously on a stub seed.

## Commands run

| Command | Exit | Result |
|---|---|---|
| `python tools/ability_trees/generate_seed.py --check` (and `--from-json --check`) | 0 | Validator OK, files match |
| `python tools/ability_trees/generate_seed.py`, then `git status --short` | 0 | No change: byte-identical |
| `$L/reload-db.sh` | 0 | Loads in 30 s; per-archetype counts checked with `psql` |
| `$L/live-db-test.sh ability_tree::tests` | 0 | 15 run, 15 passed, 0 skipped |
| `$L/live-db-test.sh "::"` (full services live-DB suite, fresh reload) | 0 | 3333 run, 3333 passed, 0 skipped |
| `$L/lane.sh cargo +1.98.1 clippy -p cimmeria-services --all-targets -- -D warnings` | 0 | Clean |
| `$L/lane.sh cargo fmt --all`, then `-- --check` | 0 | Applied to `seed_live_db.rs`, then clean |

No live-DB test self-skipped: every run went through `live-db-test.sh`, which sets `DATABASE_URL`.

## Regression proof

Two scripts, run under lane holds against `sgw_agent_a9a037326bb90baa4` and then undone by the full-suite reload.

Phase 1 broke the seed in place: it deleted Soldier branch 0's capstone row, dropped `archetype_ability_tree_ability_id_fkey` and pointed a Commando node at ability 999999, and made a Scientist branch-1 node require the branch-0 root. All five `seed_live_db` tests failed (5 run, 0 passed):

- `seed_per_archetype_node_counts_match_final_v2`: `Soldier (archetype 1) node count`, left 71, right 72.
- `seed_every_branch_has_one_root_and_one_level_50_capstone`: `Soldier branch 0: one capstone`, left 0, right 1.
- `seed_every_tree_row_has_an_ability`: `tree rows without an ability` (panicked at `seed_live_db.rs:184` before rustfmt).
- `seed_prerequisites_are_nodes_of_the_same_branch`: `prerequisite ... is in another branch` (`seed_live_db.rs:104` before rustfmt).
- `seed_trainer_rows_and_tree_rows_match`: `trainer rows without a tree node` (`seed_live_db.rs:144` before rustfmt).

Phase 2 removed the orphan row and reset every row to the stub defaults (`required_branch_points=0, skill_point_cost=1, is_branch_root=false, is_capstone=false, branch_name=NULL`). `catalog_loads_seed_v2_columns_and_joined_training_cost` failed with `seed sets spend gates`: the stub shape that AT-01's old assertion required now fails.

## Known gaps

- No in-client pass with the new trees. Branches now reach 25 nodes (the client's `MAX_BUTTONS` is 30) and unlock levels reach 50.
- The per-archetype counts are pinned twice, in the generator's `EXPECTED` and in `seed_live_db::EXPECTED_NODES`. A deliberate workbook change must update both.

## Integration edits for the coordinator

- `work-packets.md`: set AT-05b to Review, and note that the "existing `handle_train_ability` live-DB fixture" never existed (Evidence 4).
- PR #807 description: paste the validator block above. The PR is ready for CI on the new base.
