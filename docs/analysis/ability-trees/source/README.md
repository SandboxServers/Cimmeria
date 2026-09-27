# Ability Trees: Source Data

> Type: reference. Audience: packet workers and reviewers.
> Updated: 2026-09-25. Companions: [campaign README](../README.md), [work packets](../work-packets.md).

## Files

| File | What it is | Authority |
|---|---|---|
| `SGW_All_Classes_Progression_EMULATOR_FINAL_v2_LEVEL50.xlsx` | The owner's EMULATOR FINAL v2 workbook, received 2026-09-25. SHA-256 `ad9af21448794ca1eea2bae609016b5b55ad12f2bedec542c5f3b84cf1b21dba`. | **Canonical.** Tree content, progression rules (sheet `13_Progression_Rules`), the level 1-50 table (sheet `15_Emulator_Level_1_50`) and emulator decisions (sheet `16_Emulator_Decisions`). |
| `final-v1-implementation-handoff.md` | The FINAL v1 Claude handoff that came with the first zip (`SGW_ABILITY_TREES_CLAUDE_HANDOFF_FINAL_V1`). | Superseded wherever the v2 workbook's rules sheets differ. Still the source of the AT-01 to AT-06 packet shape and the import-safety checklist (its §5). |
| `final-v1-manifest.json` | The v1 manifest (workbook SHA `c7884096…`). | Historical. |
| `trainer_server_export.json` | Sheet `12_Trainer_Server_Export` of the v2 workbook as pretty JSON: 439 rows in archetype, `tree_index`, `node_order` order, with the workbook SHA-256 in its `source` block. | **Generated** by [`tools/ability_trees/generate_seed.py`](../../../../tools/ability_trees/README.md). A review copy for diffs; the workbook stays canonical. |

The v1 zip's CSV and JSON exports are not committed. AT-05 extracts the node table from sheet `12_Trainer_Server_Export` of the v2 workbook, so the seed is generated from the canonical file, not from a hand-kept copy.

## Generator

[`tools/ability_trees/generate_seed.py`](../../../../tools/ability_trees/README.md) verifies the workbook's SHA-256, runs the v1 handoff §5 import checks plus the campaign's own (30 or fewer nodes per branch, the starter-collision check with 1646 as the D-AT09 exception), and writes `trainer_server_export.json`, `db/resources/Archetypes/Seed/archetype_ability_tree.sql` and `db/resources/Abilities/Seed/trainer_abilities.sql`. `--check` regenerates in memory and exits 1 if a committed file drifted; `--from-json` runs without openpyxl. The reserve pool (sheet `11_Reserve_Review`) is not seeded.

## v1 to v2 delta

The coordinator compared the two workbooks on 2026-09-25:

- **Node data is identical.** All 439 rows of `12_Trainer_Server_Export` match the v1 JSON export field for field (keys `(archetype, branch, node_order)`, zero differences). The `project_status` column still reads `FINAL_V1`.
- **The rules changed.** v2 adds or settles:
  - `required_branch_points` counts trainer points spent **across the whole archetype**, not per branch ("PROJECT FINAL v1.1 clarification"). Prerequisites still enforce the branch path.
  - The level cap is **50**. The XP thresholds for levels 1-20 keep today's values, and levels 21-50 grow 15% per level, rounded to 5,000.
  - Training points are **1 at level 1, then +1 per level**, 50 in total at the cap. This replaces today's 2 per level.
  - Only trainer purchases count as spent points. Starter, mission, system, GM and weapon grants satisfy prerequisites but never add spend.
  - Respec refunds and removes trainer-purchased nodes only, resets the spend counter, and must be atomic and replay-safe.
  - A known starter collision: universal starter ability 1646 is also a Goa'uld Servant Lord node. Reconcile it before Goa'uld UAT.
  - The level-50 client entry is a display sentinel; there is no level 51.
