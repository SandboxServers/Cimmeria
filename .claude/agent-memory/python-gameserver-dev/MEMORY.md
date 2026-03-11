# Python Game Logic Agent Memory

## Key Reference Files
- `python/base/Account.py` — Account entity (char list, create, delete, play, visuals, version/data requests)
- `python/base/SGWPlayer.py` — Base-side player entity (load, minigame, chat, organizations)
- `python/cell/SGWPlayer.py` — Cell-side player entity (inventory, movement, combat, missions, interactions)
- `python/common/Constants.py` — SKIN_TINTS[16], BAG_SIZES, BagFillOrder, level XP table
- `python/common/Config.py` — DEVELOPER_MODE, DISALLOW_DUPLICATE_CHARACTERS, combat QR constants
- `python/common/defs/CharacterCreation.py` — CharDef validation, visual group/choice validation
- `python/common/defs/Def.py` — DefMgr: ResourceCategories (1-22), version history, resource serving

## Critical Data Patterns
- **Bodyset format in DB**: `BS_HumanMale.BS_HumanMale` (doubled with dot separator)
- **SKIN_TINTS**: 16 ARGB u32 values, index 0=0x2F1308FF through index 15=0x8D3F24FF
- **GENDER enum**: GENDER_Male=1, GENDER_Female=2 (NOT 0-indexed)
- **Starting positions**: Castle_CellBlock (-334.23, 73.47, -228.03), SGC_W1 (201.5, 1.31, 49.72)
- **ResourceCategories**: 1=sequence, 2=ability, 3=mission, 4=item, 5=dialog, 6=event_set, 7=char_creation, 8=interaction_set_map, 9=effect, 10=text, 11=error_text, 12=world_info, 13=stargate, 14=container, 15=blueprint, 16=applied_science, 17=discipline, 18=racial_paradigm, 19=special_words, 20=interaction
- **sgw_player.components**: varchar(200)[] array — stores visual component path strings
- **sgw_player.abilities**: integer[] array — stores starting ability IDs

## Account Method Index (Exposed-Only)
- 0xC0=versionInfoRequest (ClientCache), 0xC1=elementDataRequest (ClientCache)
- 0xC2=logOff, 0xC3=createCharacter, 0xC4=playCharacter, 0xC5=deleteCharacter
- 0xC6=requestCharacterVisuals, 0xC7=onClientVersion

## Account Client Method Index
- 0x80=onVersionInfo, 0x81=onCookedDataError, 0x82=onCharacterList
- 0x83=onCharacterCreateFailed, 0x84=onCharacterVisuals, 0x85=onCharacterLoadFailed

## Character Creation Flow (Python Reference)
1. Parse args: name, extraName, charDefId, visualChoices[], skinTintColorId
2. Validate charDefId against DB-loaded char_creation resource
3. Validate visual choices (getAllChoices): check groups exist, choices valid, required groups present
4. Validate name length >= 3 and uniqueness via DB query
5. Validate skinTintColorId in range [0, 15]
6. Build component list from visual choices (non-item components only)
7. Look up world_id from world_info resource
8. Collect starting ability IDs from char_creation_abilities
9. INSERT into sgw_player with all columns including components, abilities, starting position
10. Create starting inventory items from visual choices with item_ids
11. Append to in-memory cache, send updated character list

## requestCharacterVisuals Flow (Python Reference)
1. Check character ownership via findCharacter (in-memory cache)
2. If items not yet loaded: query sgw_inventory for equipped items (containers 3-14, slot 0)
3. Look up item visual components from item definitions
4. Append item visual components to character's component list
5. Map skin_color_id through SKIN_TINTS[] to get ARGB value
6. Send: playerId, bodyset, components[], primaryTint=0xFF, secondaryTint=0xFF, skinTint=ARGB

## See Also
- [rust-comparison.md](rust-comparison.md) — Detailed Rust vs Python behavioral comparison (2026-03-04)
