# db/ — PostgreSQL Schemas

PostgreSQL 17.9 schema files. The database runs on port 5433 with role `w-testing`.

362 SQL files organized by game system.

## Structure

```
db/
├── database.sql        Master setup: create database, roles, extensions
├── split_schemas.py    Utility script that generated the resources/ split
├── sgw/                Game schema — accounts, characters, items, etc.
├── resources/          Content data — 18 game systems (see below)
└── scripts/            Additional utility scripts
```

> **Schema policy:** the seed data lives in `db/resources/` and is edited
> **directly**. Do not add migration scripts under `db/scripts/` — there is no
> migration framework; the schema is reloaded from source on setup.

## resources/ — Content Data (18 Game Systems)

Each system has up to 4 file types: `Tables/`, `Seed/`, `Sequences/`, `Types/`.

| System | Tables | Key Data |
|---|---|---|
| `Abilities/` | 7 tables | 1,887 abilities |
| `Effects/` | ~5 tables | 3,217 effects |
| `Missions/` | ~8 tables | 1,040 missions |
| `Items/` | ~6 tables | 6,059 items |
| `Dialogs/` | ~6 tables | 5,411 dialogs |
| `Entities/` | ~5 tables | 153 entity templates |
| `Loot/` | ~4 tables | 2 loot tables (nearly empty) |
| `Archetypes/` | ~3 tables | 8 archetypes, 23 character defs |
| `Combat/` | types only | Combat enumerations |
| `Events/` | ~4 tables | Kismet event sets |
| `AI/` | types only | NPC AI behavior enumerations |
| `Content/` | ~2 tables | Content metadata |
| `Gameplay/` | types only | Gameplay enumerations |
| `Social/` | types only | Social system enumerations |
| `System/` | types only | System configuration |
| `Texts/` | ~2 tables | Localization strings |
| `Visuals/` | ~3 tables | Visual configuration |
| `Worlds/` | ~4 tables | 91 world/zone definitions |

Support files per system: `_foreign_keys.sql`, `_functions.sql`, `_indexes.sql`, `_primary_keys.sql`, `_schema.sql`, `_sequence_ownership.sql`, `_triggers.sql`

### Generated seed

Some seed is written by a checked-in generator rather than by hand. Change the generator's input and rerun it; never hand-edit its output.

| Seed | Generator | What it writes |
|---|---|---|
| `Archetypes/Seed/archetype_ability_tree.sql`, `Abilities/Seed/trainer_abilities.sql` | [tools/ability_trees/](../tools/ability_trees/README.md) | The ability trees, from the owner's workbook |
| `Items/Seed/crafting_item_effects.sql` | [tools/crafting/](../tools/crafting/README.md) | Blueprint and paradigm items |
| `Effects/Seed/effect_nvps.sql` (the `-- ability-mechanics generated <family>` blocks only) and the `script_name` of those effects in `Effects/Seed/effects.sql` | [tools/ability_mechanics/](../tools/ability_mechanics/README.md) | Effect NVPs read from each effect's own designer text. Every row is RECONSTRUCTION and quotes its source; rows outside the blocks are hand-authored. CI runs its `--check` |

## sgw/ — Game Schema

Core game tables: accounts, characters, character inventory, missions, effects, respawners, etc.

## Loading the Database

```powershell
# Full pipeline (recommended):
pwsh setup.ps1

# Database only (wipe and reload):
pwsh setup.ps1 -SkipBuild -ForceDatabase

# Manual:
.\db.bat
```

## Connection Details

- Host: `localhost:5433`
- Database: `sgw`
- Role / credentials: `w-testing` / `w-testing` (local-dev trust auth)
- Test account: `test` / `test` (SHA1 hashed)

## Content Coverage

See [docs/content/README.md](../docs/content/README.md) for a full audit of what data exists vs. what's connected and functional. Short version: the data is rich (112,626 rows) but most content is not yet wired up to Python/Rust scripts.
