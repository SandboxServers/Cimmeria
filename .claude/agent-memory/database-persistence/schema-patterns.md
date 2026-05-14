# Schema Patterns & File Locations

> [!WARNING] Confidence: STALE — verify against current `db/sgw/` and `crates/services/src/base/character_create.rs`
>
> Triaged 2026-05-13 (Phase −0.5 step 4). The "C++ Python Reference (persistence operations)" section uses paths like `python/base/Account.py` — the mechanical pass should have rewritten these to `deprecated/python/base/Account.py`. Verify before citing. The sgw_player column inventory looks structurally accurate but Cimmeria has shipped multiple schema migrations since this was written; re-read the latest `db/sgw/Players/Tables/sgw_player.sql` before relying on the DEFAULT-vs-NOT-NULL distinctions here.
>
> No promotion target yet — persistence chapters are not in the Phase 0.5 / Phase 1 priority list (see issue #264). Keep this memory for now; consider promoting to `spec.persistence.character-state` if/when persistence chapters get authored.

## Schema Files
- `db/sgw/Accounts/Tables/account.sql` -- account table (account_id, account_name, password, accesslevel, enabled)
- `db/sgw/Players/Tables/sgw_player.sql` -- player table (36 columns, many with defaults)
- `db/sgw/Inventory/Tables/sgw_inventory.sql` -- player inventory (inherits sgw_inventory_base)
- `db/sgw/Inventory/Tables/sgw_inventory_base.sql` -- base inventory schema
- `db/sgw/Shards/Seed/shards.sql` -- shard list
- `db/sgw/_foreign_keys.sql` -- all FK constraints
- `db/sgw/_primary_keys.sql` -- all PK constraints
- `db/sgw/_indexes.sql` -- indexes (mail_lookup, inventory char lookup)
- `db/sgw/_sequence_ownership.sql` -- sequence ownership

## Resources Schema
- `db/resources/Archetypes/Seed/char_creation.sql` -- character creation seed data (23 entries)
- `db/resources/Archetypes/Tables/char_creation.sql` -- char_creation table definition
- `db/resources/Archetypes/Types/EAlignment.sql` -- enum type
- `db/resources/Archetypes/Types/EArchetype.sql` -- enum type
- `db/resources/Archetypes/Types/EGender.sql` -- enum type
- `db/resources/Worlds/Seed/worlds.sql` -- world definitions (200+ entries)
- `db/resources/Worlds/Tables/worlds.sql` -- worlds table

## C++ Python Reference (persistence operations)

Paths use the post-tree-reorganization `deprecated/python/` layout (the legacy `python/` tree was moved into `deprecated/` in commit `46df5bc`). The reference is for cross-checking the persistence shape — the deprecated Python is not the runtime.

- `deprecated/python/base/Account.py` -- Character CRUD: createCharacter, deleteCharacter, requestCharacterVisuals, playCharacter, sendCharacterList
- `deprecated/python/base/SGWPlayer.py` -- Player load (player_name, access_level), persist stub
- `deprecated/python/cell/SGWPlayer.py` -- Cell-side persistence
- `deprecated/python/common/Constants.py` -- SKIN_TINTS table, BAG_SIZES
- `deprecated/python/common/defs/CharacterCreation.py` -- CharDef loading from resources.char_creation
- `deprecated/python/Atrea/enums.py` -- Enum integer values

## sgw_player Column Inventory (for INSERT completeness tracking)
Required columns (no DEFAULT):
- account_id (integer, NOT NULL)
- alignment (integer, NOT NULL, CHECK 0-5)
- archetype (integer, NOT NULL, CHECK 0-8)
- gender (integer, NOT NULL, CHECK 1-3)
- player_name (varchar(64), NOT NULL, UNIQUE)
- extra_name (varchar(64), NOT NULL)
- world_location (varchar(64), NOT NULL, FK -> resources.worlds)
- bodyset (varchar(64), NOT NULL)
- pos_x (real, NOT NULL)
- pos_y (real, NOT NULL)
- pos_z (real, NOT NULL)
- skin_color_id (integer, NOT NULL, CHECK 0-15)

Has DEFAULT:
- player_id (integer, DEFAULT nextval, PK)
- level (integer, DEFAULT 1)
- title (integer, DEFAULT 0)
- heading (smallint, DEFAULT 0)
- naquadah (integer, DEFAULT 0)
- exp (integer, DEFAULT 0)
- first_login (integer, DEFAULT 1)
- world_id (integer, NULLable, FK -> resources.worlds)
- known_stargates (integer[], DEFAULT '{}')
- components (varchar(200)[], DEFAULT '{}')
- abilities (integer[], DEFAULT '{}')
- access_level (integer, DEFAULT 0)
- bandolier_slot (integer, DEFAULT 0)
- interaction_maps (player_interaction_map[], NULLable)
- training_points (integer, DEFAULT 0)
- discipline_ids (integer[], DEFAULT '{}')
- racial_paradigm_levels (integer[], DEFAULT '{}')
- applied_science_points (integer, DEFAULT 0)
- blueprint_ids (integer[], DEFAULT '{}')
- known_respawners (integer[], DEFAULT '{}')
