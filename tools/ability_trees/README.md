# tools/ability_trees — ability-tree seed generator

`generate_seed.py` turns the owner's canonical ability-tree workbook into the committed seed SQL, and refuses to write anything if the data breaks an import rule. It is packet AT-05 of the [ability-trees campaign](../../docs/analysis/ability-trees/work-packets.md).

## Inputs and outputs

| | Path |
|---|---|
| Reads | `docs/analysis/ability-trees/source/SGW_All_Classes_Progression_EMULATOR_FINAL_v2_LEVEL50.xlsx`, sheet `12_Trainer_Server_Export` (SHA-256 pinned in the script) |
| Reads | `db/resources/Abilities/Seed/abilities.sql`, `db/resources/Archetypes/Seed/char_creation.sql`, `db/resources/Archetypes/Seed/char_creation_abilities.sql`, and the current `db/resources/Abilities/Seed/trainer_abilities.sql` |
| Writes | `docs/analysis/ability-trees/source/trainer_server_export.json`: sheet 12 as pretty JSON, for review diffs |
| Writes | `db/resources/Archetypes/Seed/archetype_ability_tree.sql`: one row per node, with the AT-01 columns |
| Writes | `db/resources/Abilities/Seed/trainer_abilities.sql`: debug list 1 offers every node; rows of any other list are carried over |

The reserve pool (sheet `11_Reserve_Review`) is never read or seeded.

Mapping: `ability_index` is the workbook `node_order`, `tree_index` follows the workbook branch order (0, 1, 2) per archetype, and `prerequisite_abilities` is the primary plus the additional prerequisites, deduplicated and sorted, with 0 removed. `Free Jaffa / Shol'va` maps to `ARCHETYPE_Sholva` only; `ARCHETYPE_Jaffa` gets no rows (D-AT04).

## Run it

From the repo root:

```bash
python tools/ability_trees/generate_seed.py              # validate and regenerate (needs openpyxl)
python tools/ability_trees/generate_seed.py --check      # validate and exit 1 if a committed file drifted
python tools/ability_trees/generate_seed.py --from-json --check
                                                         # same check from the committed JSON, stock Python only
```

Exit codes: 0 success, 1 drift (`--check`), 2 validation or input failure. Output is deterministic. `--check` ignores CRLF versus LF, and a normal run keeps each file's current line endings (`--eol lf|crlf` forces one).

CI does not run the script. The generated SQL is committed, so nothing downstream needs openpyxl.

## What it validates

Every check fails the run with a message naming the node.

- The workbook's SHA-256 and sheet header match the pinned values.
- Every node id and every prerequisite id exists in `abilities.sql`.
- The counts match the pinned workbook: 439 nodes, 419 unique ids, 21 branches, 19 ids shared between archetypes, 59 nodes with raw `training_cost` 0, 243 nodes above level 20, and the per-archetype counts.
- Each archetype has exactly three branches, in the order the `tree_index` map expects.
- Each branch has exactly one root (node 1, no prerequisites) and exactly one capstone, which unlocks at level 50.
- Node orders are unique and contiguous from 1, and a branch has 30 nodes or fewer (the client's `AbilityMod.MAX_BUTTONS`).
- Unlock levels come from {1, 5, 10, ..., 50}. Unlock level and `required_branch_points` never fall as node order rises, and one tier has one spend gate.
- An ability appears at most once per archetype, so `(archetype, ability_id)` can be unique.
- Every non-root node has a prerequisite. Every prerequisite is a node of the same branch, unlocks no later, and has no higher spend gate. No node requires itself, and the prerequisite graph has no cycle.
- No purchasable node is granted at character creation to the same archetype (`char_creation_abilities`, joined to the archetype through `char_creation`). The only allowed exception is ability 1646 for Goa'uld (D-AT09), listed in `ALLOWED_STARTER_COLLISIONS`.

## Changing the data

Edit the workbook, update `WORKBOOK_SHA256` (and `EXPECTED` if the counts change on purpose), run the script, and commit the workbook, the JSON and both SQL files together.
