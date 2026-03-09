# SGW.exe Decompilation Index

Decompiled from SGW.exe — the Stargate Worlds MMO client executable.

**Excluded**: UE3 engine (rendering, physics, core objects, standard actors),
FCollada, SpeedTree, MFC/Win32, compiler runtime, exception handlers.

**Total output**: 19.3 MB across 14 files

## Files

| # | File | Description | Size |
|---|------|-------------|------|
| 01 | 01_sgw_game_classes.c | SGW-specific game classes (player, pawn, camera, regions, specs) | 210 KB |
| 02 | 02_bigworld_network.c | BigWorld networking, entities, messages, data types | 456 KB |
| 03 | 03_cegui_ui.c | CEGUI UI framework integration and Lua bindings | 111 KB |
| 04 | 04_cryptopp.c | CryptoPP encryption (AES/Rijndael, SHA, HMAC-MD5) | 7 KB |
| 05 | 05_gfx_scaleform.c | GFx/Scaleform Flash UI + ActionScript runtime (GAS*) | 296 KB |
| 06 | 06_game_events.c | Game event handlers, network callbacks, slash commands | 264 KB |
| 07 | 07_game_entities.c | Game entity types (GamePlayer, GameMob, GamePet, etc.) | 54 KB |
| 08 | 08_game_data.c | Cooked game data, database records (items, effects, gates) | 25 KB |
| 09 | 09_game_ui_visuals.c | UI scripting, character appearance, Bink movies | 77 KB |
| 10 | 10_game_systems.c | World map, commands, actions, interaction sets | 13 KB |
| 11 | 11_libraries.c | Compression (CZip), TinyXML, concurrency, nVidia | 92 KB |
| 12 | 12_game_debug_config.c | Debug tools, system options, config, audio effects | 121 KB |
| 13 | 13_other_game.c | Uncategorized game-related code | 670 KB |
| 14 | 14_standalone_named.c | Named standalone functions (no class attribution) | 17414 KB |
