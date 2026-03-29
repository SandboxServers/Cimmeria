# entities/ — XML Entity Definitions

Entity type definitions for all BigWorld entities in Stargate Worlds. These XML files define the contract between the game client, server, and Python scripts — every property, method, and interface that entities expose.

50 files across 3 directories.

## Structure

```
entities/
├── entities.xml            Master entity registry — lists all 18 entity types
├── cell_spaces.xml         World space definitions (which zones are CellApp spaces)
├── spaces.xml              Space configuration
├── custom_alias.xml        Type aliases (e.g., EntityID, MailID)
├── custom_enumerations.xml Enumerations (StatType, AbilityType, AIState, etc. — 70+ types)
├── defs/                   Per-entity .def files
│   ├── SGWPlayer.def       Player entity (68 properties, 175+ methods with interfaces)
│   ├── SGWMob.def          NPC/enemy entity
│   ├── SGWBeing.def        Base being interface
│   ├── Account.def         Account management entity
│   └── ...                 (18 total entities, 18 interfaces)
└── editor/                 Editor-facing entity definitions
```

## Entity Types (18 total)

| Entity | Class ID | Description |
|---|---|---|
| `SGWSpawnableEntity` | 0 | Base spawnable world object |
| `SGWBeing` | 1 | Base being (has stats, health) |
| `SGWPlayer` | 2 | Player character |
| `SGWGmPlayer` | 3 | GM player (elevated permissions) |
| `SGWMob` | 4 | NPC/enemy |
| `SGWPet` | 5 | Player pet |
| `SGWDuelMarker` | 6 | Duel zone marker |
| `Account` | 7 | Account/login entity |
| Plus 10 more entities... | | |

## How Entity Definitions Work

Each `.def` file specifies:
- **Properties** — synchronized state (`<Properties>` section, with `<Flags>` for sync direction: OwnClient, OtherClients, AllClients, BASE, etc.)
- **CellMethods** — cell-side callable methods
- **BaseMethods** — base-side callable methods
- **ClientMethods** — methods the server calls on the client
- **Implements** — interface mixins (`SGWPlayer` implements 10+ interfaces)

The Rust `cimmeria-defs` crate parses these files at startup to build the entity type registry. Python scripts use them implicitly via the Boost.Python binding.

See [docs/engine/entity-type-catalog.md](../docs/engine/entity-type-catalog.md) for the full entity catalog with all properties and methods.
