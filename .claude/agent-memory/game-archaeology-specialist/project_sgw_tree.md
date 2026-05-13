---
name: project_sgw_tree
description: Full layout of game/sgw UE3 client tree — directory map, high-value artifact locations, gotchas for server RE work
metadata:
  type: project
---

# game/sgw — UE3 client tree layout

**Why:** First-contact reconnaissance pass (2026-05-12). Every future dig into client behavior should check this map first to avoid re-discovering the directory structure.

**How to apply:** Use as a lookup when asked "where would the client define X?" — match artifact type to the directory listed below.

## Top-level structure

```
game/sgw/
  Common/       — BigWorld engine config (XML) + entity defs (res/)
  Resources/    — Only EULA.rtf and two .URL shortcuts — essentially empty/vestigial
  Working/      — All real content: UE3 engine tree + compiled binaries
    Engine/     — UE3 base engine Config/ (BaseEngine.ini, etc.) + assets
    SGWGame/    — Game-specific layer (Config/, Content/, CookedPC/, Logs/, etc.)
    binaries/   — SGW.exe, AtreaLoader.exe, launcher scripts, all DLLs
```

## High-value artifact locations

### Entity definitions (BigWorld server contract)
- `game/sgw/Common/res/entities/entities.xml` — master entity type list (Account, SGWPlayer, SGWBeing, SGWMob, etc.)
- `game/sgw/Common/res/entities/defs/*.def` — per-entity property/method definitions (same filenames as entities/defs/ in repo root)
- `game/sgw/Common/res/entities/editor/` — editor-only entity metadata

### Client configuration / networking
- `game/sgw/Working/SGWGame/Config/DefaultEngine.ini` — UE3 engine settings, startup movies, post-process
- `game/sgw/Working/SGWGame/Config/DefaultGame.ini` — game-level settings
- `game/sgw/Working/SGWGame/Config/DefaultInput.ini` — input bindings
- `game/sgw/Working/SGWGame/Config/DefaultGUI.ini` — GUI/HUD settings
- `game/sgw/Working/SGWGame/Config/AnimMap.xml` — animation name mappings
- `game/sgw/Working/Engine/Config/` — base engine config (BaseEngine.ini, GameplayEngine.ini, etc.)
- **Note:** DefaultEngine.ini BasedOn chain points to `Engine/Config/GameplayEngine.ini`

### Connection / launcher config
- `game/sgw/Working/binaries/AtreaLoader.config` — **ENCRYPTED/BINARY** — contains server connection config (host, ports). Cannot be read as text.
- `game/sgw/Working/binaries/AtreaLoader.config.xml` — may be plaintext version; not yet spot-checked
- `game/sgw/Working/binaries/jt_search.json` — unknown; worth checking
- `game/sgw/Working/binaries/SGWLogConfig.xml` — logging configuration

### Compiled UnrealScript (source of message/method definitions)
- `game/sgw/Working/SGWGame/Content/FRScript/*.u` — compiled .u script packages:
  - `SGWGame.u` — primary game logic (most valuable)
  - `Engine.u`, `Core.u`, `IpDrv.u` — UE3 base classes
  - `GFxUI.u` — Scaleform UI
  - Content/FRScript also has: `BindableActions.xml`, `SystemOptions.xml`

### Cooked UE3 packages (CookedPC)
- `game/sgw/Working/SGWGame/CookedPC/Packages/` — ~200 .upk files
  - `KIS-Abilities.upk`, `KIS-abilities_*.upk` — Kismet ability definitions (one per faction/class)
  - `KIS-Global.upk`, `KIS-LogicModules.upk` — global Kismet logic
  - `Gameplay.upk`, `Playability.upk` — core gameplay packages
  - `Character/` subdirectory — per-race/faction character packages (ACC_*, AR_*, AD_* naming)
  - Level maps in `CookedPC/Maps/` — Agnos, Dakara, Harset, Lucia, Omega_Site, SGC, Login_Map, etc.
  - `DefaultUI.upk`, `EncounterFrontEnd.upk` — UI packages

### Slash commands
- `game/sgw/Common/xml/slash_commands/SlashCommands.xml` — player-visible slash commands
- `game/sgw/Common/xml/slash_commands/FinalSlashCommands.xml` — finalized set
- `game/sgw/Common/xml/slash_commands/InternalSlashCommands.xml` — internal/GM commands

### Shared XML data
- `game/sgw/Common/xml/SGWShared/CookedData/` — shared cooked XML data

## Companion resources in repo

| game/sgw artifact | Repo equivalent | Notes |
|---|---|---|
| `Common/res/entities/defs/*.def` | `entities/defs/*.def` | Exact same files — repo copy is the canonical working version |
| `Common/res/entities/entities.xml` | Not directly replicated | Root entity type list; BigWorld engine reads this |
| Compiled .u scripts | `deprecated/cpp/` | Deprecated C++ server had own entity impl |
| CookedPC .upk packages | `data/spaces/`, `data/cache/` | Server-side processed versions |
| AtreaLoader.config (encrypted) | `deprecated/python/` may have hints | Launcher config encryption not yet cracked |

## Gotchas / open questions

1. **AtreaLoader.config is encrypted binary** — the .xml sibling may be plaintext; check it.
2. **Resources/ is nearly empty** — EULA + URL shortcuts only. The name is misleading; real resources are in Working/SGWGame.
3. **entity defs are byte-for-byte identical** between game/sgw and entities/defs — no processing gap here.
4. **SGWGame.u** (compiled UnrealScript) is the most information-dense artifact for message/property RE that hasn't been fully catalogued. Needs UE3 .u decompiler (e.g., UELib/UModel) to extract readable class definitions.
5. **Map list in CookedPC/Maps/** shows 20 zones — Agnos, Agnos_Library, Beta_Site, Castle, Dakara, Harset, Ihpet_Crater x2, Lucia, Menfa x2, Omega_Site, SGC, Login_Map, etc. Good cross-reference for zone IDs in the server.
6. **Character package naming convention:** ACC_ = accessories, AR_ = armor, AD_ = animation, then faction suffix (G=Goa'uld, J=Jaffa, H=Human).
