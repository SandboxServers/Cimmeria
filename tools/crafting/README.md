# tools/crafting: crafting seed generators

`generate_item_effects.py` builds `db/resources/Items/Seed/crafting_item_effects.sql`, the table that says what using an item does to a player's crafting state. It is the only writer of that seed: to change which item teaches which blueprint, edit the mapping CSV and rerun the script.

## Inputs and outputs

| | Path |
|---|---|
| Reads | `docs/analysis/crafting/source/blueprint-items.csv`: the Blueprint item to blueprint mapping recovered from the client's cooked data (`docs/reverse-engineering/findings/crafting-items.md`) |
| Reads | `db/resources/Items/Seed/items.sql`, `db/resources/Entities/Seed/blueprints.sql`, `db/resources/Archetypes/Seed/racial_paradigm.sql` |
| Writes | `db/resources/Items/Seed/crafting_item_effects.sql` (CRLF) |

Seeded rows:

- every CSV row with `confidence` `high`, `medium-high` or `medium` (193 items). A `blueprint_id` cell may list several ids separated by `;`, and each becomes a row: item 8882 teaches 367 and 369;
- one row per "Racial Paradigm Guide: <paradigm>" item in `items.sql`, matched to `racial_paradigm` by name (7805-7809).

The 96 rows with `confidence` `none` are never seeded. They name a product the 2009 content never shipped, or only a group of candidate blueprints, and a wrong guess would teach the player something the item does not say.

## Run it

From the repo root, with stock Python 3:

```bash
python tools/crafting/generate_item_effects.py           # validate and regenerate
python tools/crafting/generate_item_effects.py --check   # exit 1 if the committed seed drifted
```

Exit codes: 0 success, 1 drift (`--check`), 2 validation or input failure. Output is deterministic. CI does not run the script; the generated SQL is committed.

## What it validates

- The CSV header is the expected seven columns.
- Every seeded item exists in `items.sql`, is a "Blueprint: " item, and is listed once.
- Every blueprint id exists in `blueprints.sql`.
- There is exactly one guide per paradigm, and the paradigms are ids 1-5.
- The counts are pinned: 193 Blueprint items, 194 blueprint rows, 5 guides. A changed mapping needs a deliberate edit to the constants.

The live-DB guard `crafting::item_use::tests::seed` (crate `cimmeria-base-session`) checks the loaded table against the CSV again, and that no `item_use` content trigger listens for one of these items.
