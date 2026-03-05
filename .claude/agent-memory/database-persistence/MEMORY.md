# Database & Persistence Agent Memory

## Schema Conventions
- Tables use `snake_case` naming: `sgw_player`, `sgw_inventory`, `account`, `shards`
- Columns use `snake_case`: `player_id`, `account_id`, `world_location`
- Sequences: `sgw_characters_character_id_seq` (for player_id), `sgw_inventory_item_id_seq`
- Primary keys: `account_pkey`, `sgw_player_pkey`, `sgw_inventory_pkey`
- Unique constraints: `sgw_player_player_name_key` (player_name is unique)
- See [schema-patterns.md](schema-patterns.md) for FK/cascade details

## Key Foreign Keys (from `db/sgw/_foreign_keys.sql`)
- `sgw_player.account_id` -> `account(account_id)` ON DELETE CASCADE
- `sgw_player.world_id` -> `resources.worlds(world_id)` ON DELETE RESTRICT
- `sgw_player.world_location` -> `resources.worlds(world)` ON DELETE RESTRICT
- `sgw_inventory.character_id` -> `sgw_player(player_id)` ON DELETE CASCADE
- `sgw_inventory.type_id` -> `resources.items(item_id)` ON DELETE RESTRICT
- `sgw_mission.player_id` -> `sgw_player(player_id)` ON DELETE CASCADE

## Enum Mappings (from Python `Atrea/enums.py`)
- Alignment: 0=Undefined, 1=Praxis, 2=SGU, 3-5=unused, 6=End
- Archetype: 0=Any, 1=Soldier, 2=Commando, 3=Scientist, 4=Archeologist, 5=Asgard, 6=Goauld, 7=Sholva, 8=Jaffa
- Gender: 1=Male, 2=Female, 3=Other (DB constraint: 1-3)

## Body Set Format
- DB stores `Package.Asset` format: `BS_HumanMale.BS_HumanMale`, `BS_JaffaMale.BS_JaffaMale`
- Client expects this format for onCharacterVisuals
- Resources schema: `resources.char_creation.body_set` varchar(255)

## Skin Tint RGBA Values (from `python/common/Constants.py`)
- 16 entries, indices 0-15, stored as 32-bit RGBA
- DB constraint: `skin_color_id >= 0 AND skin_color_id <= 15`
- Client expects RGBA value, NOT the index

## World ID Mapping (from `db/resources/Worlds/Seed/worlds.sql`)
- CombatSim: world_id=1
- Castle_CellBlock: world_id=12
- SGC_W1: world_id=58

## Rust Rewrite DB Status (as of 2026-03-04)
- Auth credential validation: WORKING (validate_credentials in auth.rs)
- Character list query: WORKING (query_character_list in base.rs)
- Character creation INSERT: PARTIAL (missing world_id, components, abilities, access_level, starting positions)
- Character deletion: WORKING (better than C++ -- has account ownership check)
- Character visuals: PARTIAL (missing inventory query for equipped items, wrong tint values)
- World entry query: PARTIAL (space_id mapping is stub, all worlds -> CombatSim)
- DB pool wiring: CORRECT (orchestrator -> auth + base)
