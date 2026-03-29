# python/ — Entity Scripts and Game Logic

Python 3.4.1 scripts embedded in the C++ server via Boost.Python. These are the **reference implementation** of game logic — the Rust server (`crates/`) reimplements this behavior in Rust, using these scripts as the source of truth for how things should work.

164 files across 4 directories.

## Directory Structure

```
python/
├── base/           Scripts running on the BaseApp (persistent entity side)
│   ├── minigame/   Minigame base-side logic
│   └── *.py        Account, Inventory, SpawnRegion, ChannelManager, etc.
├── cell/           Scripts running on the CellApp (spatial simulation side)
│   ├── actions/    Entity action handlers
│   ├── commands/   Admin/debug commands
│   ├── effects/    Effect application scripts
│   ├── interactions/  Interaction handlers (NPCs, objects)
│   ├── missions/   Mission step scripts
│   │   ├── Castle_CellBlock/  Praxis tutorial missions
│   │   ├── SGC_W1/            SGU tutorial missions
│   │   ├── Harset/            Harset zone missions
│   │   └── General/           Generic mission scripts
│   ├── profiles/   Entity spawn profiles
│   ├── spaces/     Per-zone space scripts
│   └── *.py        SGWPlayer, SGWMob, AbilityManager, EffectManager, etc.
├── common/         Shared utilities used by both base and cell
│   └── defs/       Common type definitions
└── Atrea/          Atrea-specific entity scripts
```

## Key Files

| File | Lines | Purpose |
|---|---|---|
| `cell/AbilityManager.py` | 1091 | Ability activation, cooldowns, combat sequences |
| `cell/EffectManager.py` | ~600 | Effect application, removal, pulsing |
| `cell/SGWPlayer.py` | ~700 | Player entity: movement, combat, XP, stats |
| `cell/SGWMob.py` | 397 | NPC AI: Fighting state, threat, ability selection |
| `base/Account.py` | ~300 | Character creation/deletion, login flow |
| `cell/Lootable.py` | 221 | Loot generation algorithm |
| `base/Crafter.py` | 575 | Crafting: disciplines, research, alloys |
| `base/Trade.py` | 244 | Player-to-player trade |

## Language Constraints (Python 3.4.1)

- No f-strings — use `"{}".format(x)` or `"%s" % x`
- No type hints
- No `async`/`await`
- No `dataclasses`, `walrus operator`, `match/case`
- No `pathlib`, limited `os.path`
- `imp` module available but deprecated

## Relationship to Rust Server

The Python scripts are **not executed by the Rust server**. When implementing a feature in Rust, read the corresponding Python script to understand the expected behavior, then implement it in the appropriate Rust crate (usually `crates/services/` or `crates/game/`).

For implementation status of each system, see [docs/gameplay/README.md](../docs/gameplay/README.md).
