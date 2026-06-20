# Crafting System — Full Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (binary RTTI + Python reference + Rust codebase cross-checked)
> **Sources**: Ghidra `SGW.exe` decompilation; `deprecated/python/cell/Crafter.py`;
>   `deprecated/python/cell/commands/Crafting.py`; `crates/entity/src/crafting.rs`;
>   `crates/services/src/base/crafting/`; `docs/reverse-engineering/findings/crafting-state-machine.md`;
>   `docs/reverse-engineering/findings/crafting-wire-formats.md`
> **Tracking issue**: replaces #53

## Completeness assessment

Crafting is unusual among the "missing" systems: its **infrastructure is ~100% done**
(state struct, DB persistence with 5 live-DB tests, world-entry blueprint sync, the
`onUpdateDiscipline` emit, GM grants, blueprint/discipline/paradigm DB seed). What is
missing is the **activity layer** — the six cell handlers that actually do work are
parse-and-log stubs (~3% complete).

| Subsystem | Client (SGW.exe) | Server (Python) | Rust server | Rust % |
|---|---|---|---|---|
| State struct + DB persistence | VCrafting client-side | `Crafter.__init__` | `crates/entity/src/crafting.rs` + `base::crafting::persistence` | 100% |
| World-entry blueprint sync (`onUpdateKnownCrafts`, 139) | expects `ARRAY<INT32>` | `onClientReady` | `map_loaded.rs` step 23 | 100% |
| `onUpdateDiscipline` emit (136) | INT32+INT32 | `gainExpertise` | byte-exact test in entity crate | 100% |
| GM grant expertise / ASP | — | `commands/Crafting.py` | `base::crafting::handlers` | 100% |
| `spendAppliedSciencePoints` (95) | INT32 disciplineId | full paradigm+prereq check | parse → `UNIMPLEMENTED` | 5% |
| `craft` (96) | craftId + ARRAY + qty | full validate/consume/timer/grant | parse → `UNIMPLEMENTED` | 5% |
| `research` (97) | itemId + kickers | researchable check + random +5 | `UNIMPLEMENTED` | 2% |
| `reverseEngineer` (98) | itemId | blueprint lookup, bias, recover | `UNIMPLEMENTED` | 2% |
| `alloying` (99) | craftId + tier item + elems | tier/quality validation | `UNIMPLEMENTED` | 2% |
| 3s induction timer | TimerUpdate → `UEvent_UI_CraftInductionStart` | `Atrea.addTimer(3.0)` | absent | 0% |
| `onUpdateCraftingOptions` (140) + entity gate | FIXED_DICT; `isCraftingAllowed` | `craftingEntityFlags` | absent | 0% |

**Overall: ~28% restored** (infrastructure ~100%, activity layer ~3%).

## Architecture

Server-authoritative request/response. Client sends one of six cell methods; the server
validates, runs a 3s induction timer, and pushes result events. Client is a pure display
consumer — no client-side state machine.

- **Client class**: `class_SGW::Crafting` (RTTI-confirmed), aka `VCrafting`. Drives the
  crafting window via `SGWScriptedWindow` (Scaleform).
- **Server class (Python)**: `Crafter`, held as `entity.crafting` on each `SGWPlayer`.
- **Craft type enum** (confirmed from `Crafting_isCraftTypeAllowed` @ `0x00e465d0` +
  `Crafting_getKnownBlueprints` @ `0x00e46830` switch statements; string literals at
  `0x019559a4`/`0x019559c0`/`0x019559e0`/`0x01955a00`):

  | Value | Name | Notes |
  |---|---|---|
  | 1 | CraftBlueprint | blueprint at `this+0x38` |
  | 2 | CraftResearch | shares static empty placeholder (no blueprint) |
  | 4 | CraftReverseEng | shares static empty placeholder (no blueprint) |
  | 8 | CraftAlloy | blueprint at `this+0x10` |

## Wire messages

### Client → Server (cell methods)

| Idx | Name | Payload | Confidence |
|---|---|---|---|
| 95 | `spendAppliedSciencePoints` | `INT32 disciplineSeqId` | HIGH (.def) |
| 96 | `craft` | `INT32 craftId` + `ARRAY<INT32> items` + `INT32 quantity` | HIGH (.def) |
| 97 | `research` | `INT32 itemId` + `ARRAY<INT32> kickers` | HIGH (.def) |
| 98 | `reverseEngineer` | `INT32 itemId` | HIGH (.def) |
| 99 | `alloying` | `INT32 craftId` + `INT32 currentTierItemId` + `ARRAY<INT32> lowerTierItems` | HIGH (.def) |
| 100 | `respecCrafting` | (no args) | HIGH (RTTI `0x01de9e6c`, stub `0x00aea3d0`) |

### Server → Client (client methods)

| Idx | Name | Payload | Confidence |
|---|---|---|---|
| 112 | `onCraftingRespecPrompt` | `INT32 CostToRespec` | HIGH (.def) |
| 136 | `onUpdateDiscipline` | `INT32 disciplineSeqId` + `INT32 expertise` | HIGH (byte-exact test) |
| 137 | `onDisciplineRespec` | (no args) | HIGH (RTTI `0x019c20c4`) |
| 138 | `onUpdateRacialParadigmLevel` | **UNKNOWN** (see open Q) | MEDIUM (RTTI `0x00e45a60`) |
| 139 | `onUpdateKnownCrafts` | `ARRAY<INT32> craftList` | HIGH (emitted in `map_loaded.rs`) |
| 140 | `onUpdateCraftingOptions` | `CraftingOptions` FIXED_DICT (4 × `CraftingInfo{items:ARRAY<INT32>, entities:ARRAY<INT32>}`) | HIGH (.def + Python `debugAllCraft`) |

## Activity logic (from `Crafter.py`, confirmed against Ghidra strings)

- **craft (96)**: guard busy → blueprint known → items in `INV_Main`/`INV_Crafting` → not an
  alloy blueprint → match `componentSet` → sufficient qty → consume → 3s timer →
  `pickedUpItem(product, qty)` + `gainExpertise(discipline, 1)`. Error strings `0x019da800`,
  `0x019da890` confirm the server-side `isCraftingAllowed` entity gate.
- **research (97)**: item `researchable`, kickers flagged `kicker` → consume → eligible
  disciplines = known AND `expertise < techCompetency` → `chance = 100 - expertise + 5×kickerCount`
  → roll → 3s timer → `gainExpertise(discipline, 5)` on success.
- **reverseEngineer (98)**: item `reverseEngineerable` → find blueprints producing it → bias =
  `techCompetency/expertise` (if tc>exp) else `1 + 0.4×(tc-exp)/exp` → recover
  `floor(rand × min(bias,1) × component.qty)` per component.
- **alloying (99)**: alloy blueprint known → `ALLOYING_ELEMENTARY_COUNTS[quality]` elementary
  components, each `tier == component.tier - 1` → consume → 3s timer → product + expertise +1.
- **spendAppliedSciencePoints (95)**: ASP≥1, paradigm level met, prereq disciplines at
  expertise≥50 → `learnDiscipline(id, 1)` → consume ASP → `onUpdateDiscipline`.

Server enforces a **crafting zone/entity gate** (`isCraftingAllowed` @ `0x00e465d0`,
`craftingEntityFlags` CELL_PRIVATE INT32) — entirely absent in Rust; players can currently
craft from anywhere once handlers exist.

## Open questions

1. **`onUpdateRacialParadigmLevel` (138) wire format** — RTTI `0x00e45a60`; the INT8-level
   inference comes from the Python `level` cap (5), not a decompiled emitter. → x64dbg D.2.
2. **Respec confirm flow** — does the client re-send `respecCrafting` after the cost prompt, or
   a separate confirm? Only one `RespecCraft` EventHandler in the binary. → x64dbg D.1.
3. **`ALLOYING_ELEMENTARY_COUNTS`** — quality-indexed count table in
   `deprecated/python/common/Constants.py`; not yet ported to Rust.
4. **`craftingEntityFlags` values** — flag meaning (which tool types) undocumented. → x64dbg D.3.
5. **Busy state** — `beginBusy`/`endBusy` are commented out in all four Python paths; mirror that
   (keep the guard, skip the set).

## Dynamic-analysis needs (x64dbg — debugger not currently connected)

- **D.1 Respec confirm**: BP `0x00d68450` (`Event_NetOut_RespecCraft` handler vfunc_0). Hit count
  on respec confirm: fires once (prompt only) or twice (query + confirm)?
- **D.2 `onUpdateRacialParadigmLevel` format**: BP `0x00e45a60` (VCrafting member callback). Dump
  event object; compare against `onUpdateDiscipline` (8 bytes). Confirm INT8 vs INT32 level.
- **D.3 `isCraftingAllowed` gate**: BP `0x01952048` (Scaleform `isCraftingAllowed`). Dump
  `this->craftingEntityFlags` as the player approaches/leaves a crafting station.
- **D.4 Craft timer delivery**: BP `0x00e45ce0` (VCrafting `TimerUpdate` callback). Confirm whether
  the server sends an initial TimerUpdate on craft start or a dedicated "craft started" message.
- **D.5 `onUpdateCraftingOptions` FIXED_DICT bytes**: BP at the universal wire-send for a
  CraftingOptions event (emitter registered `0x019c207c`/`0x019c20a0`). Capture raw bytes to pin
  the nested-array encoding.

## Ghidra annotations made

- `0x00e465d0` `Crafting_isCraftTypeAllowed` — PRE_COMMENT: craft-type enum (1/2/4/8) with the
  string-literal evidence addresses.
- `0x00e46830` `Crafting_getKnownBlueprints` — PRE_COMMENT: returns blueprint collection by craft
  type; Research/ReverseEng share a static empty placeholder.
