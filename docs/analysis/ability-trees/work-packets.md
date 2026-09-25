# Ability Trees Work Packets

> Type: how-to. Audience: the coordinator and packet workers.
> Updated: 2026-09-25. Companions: [launch prompt and decisions](README.md), [compatibility audit](audit.md), [source data](source/README.md), [testing playbook](../../../TESTING.md), [NPC AI ledger](../npc-ai-restoration/work-packets.md) (same dispatch rules).

## Dispatch rules

- Worktrees live under `.claude/worktrees/`, one per worker (Agent `isolation: "worktree"`). Branches are `trees/<packet>-<slug>`.
- Every cargo call goes through the shared build lane (`/c/Users/Steve/AppData/Local/Temp/cimmeria-castle/lane.sh <cmd>`). Live-DB tests use the worktree's own `sgw_<worktree>` database on :5433; never reload another campaign's database.
- Each worktree needs the `external/` junction. Remove it with `cmd /c rmdir external` before deleting the worktree.
- Run clippy on the CI toolchain (`cargo +<CI stable> clippy`), not the machine default.
- Squash-merge after green CI. When a PR's CI predates the latest `main`, the coordinator runs a trial merge plus tests first.
- Initial state: documentation only, against `main` @ `acbcc22e`. No packet has started.

Status vocabulary: **Ready**, **BlockedDependency**, **BlockedDecision**, **Writing**, **Review**, **Integrated**, **UATPending**, **Done**.

`rust-gameserver-dev` is the default writer. `database-persistence` advises on schema, `server-authority-enforcer` reviews AT-03, AT-04 and AT-08, `testing-validation-engineer` reviews the AT-01 and AT-03 regression strategy, and `documentation-writer` reviews the doc updates each packet owes (see the [CLAUDE.md doc map](../../../CLAUDE.md)).

## Contract fixed by this ledger

Parallel packets build against these names, so a worker who needs to change one raises it with the coordinator instead of renaming locally.

**`resources.archetype_ability_tree`** (AT-01) keeps its six columns and adds:

| Column | Type | Default | From the export |
|---|---|---|---|
| `required_branch_points` | `integer NOT NULL` | `0` | `required_branch_points` |
| `skill_point_cost` | `integer NOT NULL CHECK (>= 0)` | `1` | `skill_point_cost` |
| `is_branch_root` | `boolean NOT NULL` | `false` | `is_branch_root` |
| `is_capstone` | `boolean NOT NULL` | `false` | `is_capstone` |
| `branch_name` | `text` | `NULL` | `branch` |
| `project_status` | `text` | `NULL` | `project_status` |

The table also gets a primary key `(archetype, tree_index, ability_index)` and `UNIQUE (archetype, ability_id)`. There is **no** global unique constraint on `ability_id`, because 19 ids are shared between archetypes. `ability_index` holds the workbook `node_order`, and `prerequisite_abilities` holds the primary and additional prerequisites merged: deduplicated, sorted, with 0 removed.

**`sgw_player`** (AT-01) adds:

- `trained_abilities integer[] NOT NULL DEFAULT '{}'`: provenance. Trainer purchases only.
- `tree_points_spent integer NOT NULL DEFAULT 0 CHECK (>= 0)`: the archetype-wide spend. A character's archetype never changes, so one counter per character is enough.

**Rust** (AT-01). One catalog serves the cell and the base:

- `AbilityTreeCatalog` holds `TreeNode { archetype_id, tree_index, node_order, ability_id, level, prerequisites, required_branch_points, skill_point_cost, is_branch_root, is_capstone, branch_name, raw_training_cost }`. `raw_training_cost` is joined from `resources.abilities.training_cost`.
- One loader query, ordered `tree_index, ability_index`.
- `evaluate_train(&TrainContext) -> Result<TrainPlan, TrainReject>` is the only place a gate lives. `TrainPlan` carries `tree_index` and `cost`. `train.rs` calls it to decide a purchase, and `trainer.rs` calls it for the `trainable` byte (audit A-23, A-27).
- Gates are one function each, in one file per gate family: `gates/node.rs` (exists, known, archetype, level, prerequisites), `gates/spend.rs` (AT-03), `gates/trainer.rs` (AT-04). Parallel packets then add files instead of editing the same one.

Module placement is AT-01's choice: a shared `crates/services/src/ability_tree/` directory is the suggestion, because both `base` and `cell` consume it.

## Dependency graph and waves

```text
Wave 0 (now, parallel)     Wave 1 (after AT-01, parallel)            Wave 2              Owner
AT-01 foundation ────┬──► AT-02 one source of truth ─────────────┐
AT-E1 client evidence├──► AT-03 spend gate + atomic debit ───────┼──► AT-08 respec ──► AT-09 close-out ─► AT-06 UAT
AT-05a seed generator┼──► AT-04 trainer authority + feedback ────┤
                     ├──► AT-05b seed import (merge of AT-05a) ──┤
                     └──► AT-07 level cap 50 + TP economy ───────┘
```

AT-01 is the only bottleneck. It is kept small: schema, catalog, predicate, no behaviour change. AT-E1 and AT-05a run beside it because they touch no Rust file that AT-01 owns.

**Contended files.** The coordinator merges these one packet at a time:

- `cell/cell_methods/player/vendor/train.rs`: AT-01, then AT-04 (reject feedback only; the gates live in `gates/`).
- `base/world_entry/methods/progression/mod.rs`: AT-03 (`handle_train_ability`), AT-07 (`grant_xp` and `LEVEL_XP`), AT-08 (respec handler). These are different functions. Merge AT-03 before AT-07.
- `base/world_entry/methods/player_load/core/player_data.rs`: AT-02 (tree fetch), AT-03 (the new `sgw_player` columns in the `SELECT`).
- `mercury/world_data/stats.rs`: AT-02 deletes the tree arrays, AT-07 deletes `LEVEL_EXP`.

## Common acceptance

- Every behaviour change ships a regression guard that **fails when the fix is reverted** ([TESTING.md](../../../TESTING.md)). Gates get a unit test per rejection on `evaluate_train`. The debit and persistence get live-DB tests (`require_db_or_skip!`, exact-sentinel cleanup). Wire output gets byte-exact tests. New WARNs get `LogCapture` tests.
- New log targets are added to `OTEL_FILTER` with its pinning assertion.
- Each packet updates the docs it owes: `docs/gameplay/` (abilities and progression), `docs/gap-analysis.md`, and `docs/protocol/` when a wire message changes.

## Wave 0

### AT-01

**Status:** Ready. **Scope title:** Schema, shared tree catalog and the single trainability predicate. **Depends:** none. **Advisor:** database-persistence, testing-validation-engineer.

**Scope:**

- The contract above: table columns, keys, the two `sgw_player` columns, the `level_sanity` check left alone (AT-07 owns it), and the table's entry in `_primary_keys.sql`.
- `AbilityTreeCatalog` and its loader. Move the cell startup cache (`cell/spawner/abilities.rs:173`) onto it. The base keeps its query until AT-02.
- `evaluate_train` with today's six gates moved in unchanged (audit A-01). `train.rs` and `trainer.rs` both call it.
- Load `trained_abilities` and `tree_points_spent` onto the cell player entity at player load, so AT-03 and AT-08 need no second plumbing pass.
- **No behaviour change.** The defaults make every existing row behave exactly as today.

**Acceptance:**

- The existing `handle_train_ability_tests` and trainer tests pass unchanged.
- A new test proves that the `trainable` byte and the purchase decision agree for every seeded node of a fixture tree, including one node per rejection reason.
- A live-DB test loads the catalog from a fresh `db/database.sql` and checks the new columns' defaults.

### AT-E1

**Status:** Ready. **Scope title:** Client evidence for the trainer UI. **Depends:** none. **Writer:** game-archaeology-specialist (Ghidra plus the client's Lua under `..\SGW\Stargate Worlds-QA\Working\SGWGame\Content\UI`). Documentation only.

**Questions, each answered with an address or file:line:**

1. How do native `getTrainableList(tab)` and `getTrainableInfo(id)` combine `onAbilityTreeInfo` with the `onTrainerOpen` list (audit A-23)? Is a tree node absent from the trainer list hidden or greyed? Is the order taken from the tree or from the trainer list?
2. Which `EConditionHandlerFeedback` values render readable text for a failed train under `ERRORCODE_SYSTEM_Ability`: level too low, missing prerequisite, not enough points, not at a trainer? Does the client print `onErrorCode` in chat or on screen?
3. Does `ON_ENTITY_PROPERTY(TrainingPoints)` fire `Events.PropertyUpdated`, so `AbilityMod.onPropertyUpdated` refreshes the counter while the window is open?
4. Level 50: does the client carry a level or XP table of its own (world data, `onMaxExpUpdate`) that caps at 20, and what does the XP bar show at the cap?
5. Respec: what `respecAbilities()` sends, and what the client expects back (method 72 `resetMyAbilities`, then `onKnownAbilitiesUpdate`, the points property and a trainer re-send?).

**Output:** `docs/reverse-engineering/findings/ability-trainer-ui.md`, indexed in the findings README, plus a short note in `worknotes/at-e1.md` that feeds decisions D-AT08 and D-AT10.

### AT-05a

**Status:** Ready. **Scope title:** Seed generator and import validator. **Depends:** the contract above (it writes the columns AT-01 creates). **Advisor:** database-persistence.

**Scope:**

- `tools/ability_trees/`: one script that reads sheet `12_Trainer_Server_Export` of the canonical workbook (path and SHA-256 pinned, see [source/README.md](source/README.md)) and writes the following files.
  - `db/resources/Archetypes/Seed/archetype_ability_tree.sql`, replacing the stub. It carries the mapping `Free Jaffa / Shol'va → ARCHETYPE_Sholva` and nothing for `ARCHETYPE_Jaffa` (D-AT04). Branches map to `tree_index 0..2` in workbook order.
  - `db/resources/Abilities/Seed/trainer_abilities.sql`: debug list 1 offers every node of every archetype (D-AT06).
  - A committed JSON export beside the workbook for review diffs.
- The script runs every check in the v1 handoff §5 and fails loudly on any. Add a check of 30 or fewer nodes per branch (audit A-22) and the starter-collision check against `char_creation_abilities`, with 1646 as the single allowed exception (D-AT09).
- The reserve pool (sheet `11_Reserve_Review`) is not seeded.
- The script is deterministic: running it twice gives byte-identical output, and CI does not need the workbook library.

**Acceptance:** the validator output is committed in the PR description (439 nodes, 419 unique ids, 21 branches, 19 shared ids, 59 raw-cost-0 nodes). The PR opens as a draft and becomes **AT-05b** once AT-01 has merged.

## Wave 1 (after AT-01 merges)

### AT-02

**Status:** BlockedDependency (AT-01). **Scope title:** One source of truth for `onAbilityTreeInfo`. **Advisor:** aoi-witness-broadcast (wire).

**Scope:**

- Build the player-load `AbilityTreeData` from `AbilityTreeCatalog`, not from a second query.
- Delete `archetype_ability_tree()` and its arrays from `stats.rs`. An archetype with no rows gets an empty tree and one WARN, not a fabricated one.
- Move the world-data tests off `archetype_ability_tree(2)` onto a fixture catalog.

**Acceptance:** a byte-exact `onAbilityTreeInfo` test built from a fixture catalog, and a test that fails if the tree order ever differs from the catalog order the trainer uses. A grep test that no ability-id array literal remains in `mercury/world_data/`.

### AT-03

**Status:** BlockedDependency (AT-01). **Scope title:** Archetype-wide spend gate and atomic purchase. **Advisor:** server-authority-enforcer, database-persistence, testing-validation-engineer.

**Scope:**

- `gates/spend.rs`: reject when `tree_points_spent < required_branch_points` (`TrainReject::SpendGate`), and when points are below `skill_point_cost` (`TrainReject::NotEnoughPoints`).
- `CellToBaseMsg::TrainAbility` carries `cost` and `tree_index`.
- The base `UPDATE` in one statement: append to `abilities` and `trained_abilities`, `training_points -= cost`, `tree_points_spent += cost`, guarded by `training_points >= cost AND NOT (abilities @> ARRAY[id])`. It returns both counters, and `AbilityGranted` carries both back to the cell.
- A structured WARN `abilities event=train_raw_cost_zero` when a purchased node's `raw_training_cost = 0`. The source value is never rewritten.

**Acceptance:**

- A live-DB test that a replayed purchase debits once and increments spend once.
- A live-DB test that a failed guard leaves all four fields untouched.
- A unit test that a capstone is rejected at level 50 with too little spend and accepted with enough.
- A test that a starter-granted ability satisfies a prerequisite but adds no spend (v2 rule).

### AT-04

**Status:** BlockedDependency (AT-01). **Scope title:** Trainer authority, reject feedback and the points refresh. **Advisor:** server-authority-enforcer.

**Scope:**

- `gates/trainer.rs`. `last_interaction_target` must be set, must resolve to a live entity whose template is in `template_trainer_lists`, must offer this ability to this archetype, and must still pass `interact_target_in_range`. Reuse the interact gate; do not add a new distance constant.
- Rejection feedback (D-AT08): `onErrorCode` with the codes AT-E1 names, then an `onTrainerOpen` re-send when a trainer is pinned. A replayed purchase of an already-known ability stays silent.
- Handle `AbilityGranted` by sending `ON_ENTITY_PROPERTY(GENERICPROPERTY_TrainingPoints, remaining)` before the trainer re-send (audit A-07), using a shared builder with the level-up bundle.

**Acceptance:** a unit test per trainer-gate rejection (no pin, pin not a trainer, not offered, out of range, target despawned), a byte-exact test of the points property and of each error code, and an ordering test for the grant burst.

### AT-05b

**Status:** BlockedDependency (AT-01, AT-05a). **Scope title:** Seed import. Rebase AT-05a onto AT-01 and add the live-DB guards.

**Acceptance:** live-DB tests assert the per-archetype counts from the v1 handoff §11 against a fresh `db/database.sql`: Soldier 72, Commando 64, Scientist 59, Archaeologist 65, Asgard 66, Sholva 51, Goa'uld 62, and Jaffa 0 (documented, D-AT04). Also: one root and one level-50 capstone per branch, every `trainer_abilities` row matched by a tree row, and every tree row matched by an ability. The existing `handle_train_ability` live-DB fixture that assumed stub rows is updated.

### AT-07

**Status:** BlockedDependency (AT-01, for the merge order only; the work can start in Wave 0 if a worker is free). **Scope title:** Level cap 50 and the v2 training-point economy (D-AT02). **Advisor:** combat-systems-advisor (level-scaled stats), database-persistence.

**Scope:**

- `MAX_LEVEL = 50`, with `LEVEL_XP` extended from sheet `15_Emulator_Level_1_50`. Levels 1-20 are unchanged, and index 50 is a sentinel.
- Delete `LEVEL_EXP` from `stats.rs`; every XP consumer reads the `game` crate's table.
- `level_sanity CHECK (level <= 50)`.
- Training points: new characters start with 1, and each level grants 1 (`TRAINING_POINTS_PER_LEVEL = 1`). Update the seeded characters in `db/sgw/Players/Seed/sgw_player.sql` to the v2 total for their level. The colo database reloads from the seed on deploy.
- Check each level-indexed table for an out-of-range read above 20 (stats per level, `archetype_stats` `*_per_level`, mob level scaling, `gmGiveXp`) and clamp or extend it.
- AT-E1 question 4 decides whether the client needs anything at the cap.

**Acceptance:** `grant_xp` tests at 20→21, 49→50 and at the cap (no level 51, no points past 50). A 50-entry table test against the workbook values. A live-DB test that the check accepts 50 and rejects 51.

## Wave 2

### AT-08

**Status:** BlockedDependency (AT-03, AT-04). **Scope title:** Respec (v2 rule, D-AT03). **Advisor:** server-authority-enforcer, database-persistence.

**Scope:**

- Implement cell method 72 `resetMyAbilities` at a pinned trainer (AT-04's gate), in one base `UPDATE`:
  - `abilities` minus `trained_abilities`;
  - `training_points += tree_points_spent`;
  - `tree_points_spent = 0`, `trained_abilities = '{}'`;
  - `naquadah -= cost`, guarded by `naquadah >= cost`.
- Replay-safe: a second respec with nothing trained is a no-op that charges nothing.
- Then send the known-abilities update, the points property and the trainer re-send. The price stays at today's 1000 until it is sourced (D-AT10). Remove refunded abilities from the hotbar the way AT-E1 question 5 says the client expects.

**Acceptance:**

- A live-DB test: non-trainer grants survive, points are refunded exactly, and a replay is free.
- A test that respec with too little naquadah changes nothing and sends feedback.

### AT-09

**Status:** BlockedDependency (all). **Scope title:** Close-out. Write `handoffs/session-resume.md` with the owner's UAT checklist, update `docs/project-status.md` and `docs/gap-analysis.md`, and put `/release` on the last PR.

## AT-06: owner UAT (colo, after the release)

For each showcase archetype, in order Soldier, Commando, Scientist, Archaeologist, Free Jaffa (Shol'va): make a character, go to the Interaction Debug NPC (template 25), and open the trainer.

1. Three tabs appear, in workbook order. Locked nodes are visible and greyed.
2. Buy the root. The point counter drops at once, and the ability appears in the known list.
3. Click a node whose prerequisite you lack (use a crafted packet, or wait for a stale window). The press shows an error, and nothing is spent.
4. Buy the tier-1 nodes in one branch. Watch a tier-2 node in **another** branch open once archetype-wide spend reaches 4 and its own branch prerequisite is known.
5. Relog and reopen the trainer. Learned nodes and points persist.
6. Walk away from the trainer and replay a train packet. It is rejected with feedback.
7. Double-click a purchase. Only one point is spent.
8. Level with `gmGiveXp` to 21 and then to 50. The XP bar behaves, there is no level 51, the capstone opens at 50 once its path and spend are met, and you have 50 points in total.
9. Respec. Trainer nodes go away, and starter abilities stay. Points are refunded and 1000 naquadah is charged.
