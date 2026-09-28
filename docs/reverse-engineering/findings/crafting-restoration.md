# Crafting System — Full Restoration Findings

> **Date**: 2026-06-20
> **Phase**: Post-V5 deep restoration assessment
> **Confidence**: HIGH (binary RTTI + Python reference + Rust codebase cross-checked)
> **Sources**: Ghidra `SGW.exe` decompilation; `deprecated/python/cell/Crafter.py`;
>   `deprecated/python/cell/commands/Crafting.py`; `crates/entity/src/crafting.rs`;
>   `crates/base-session/src/base/crafting/`; `docs/reverse-engineering/findings/crafting-state-machine.md`;
>   `docs/reverse-engineering/findings/crafting-wire-formats.md`
> **Tracking issue**: replaces #53

## Completeness assessment

> **Update (2026-09-27, crafting campaign close-out CR-13).** The activity layer this document planned is now built, server-side, by the crafting campaign ([ledger](../../analysis/crafting/README.md), CR-01 to CR-17). Nothing has been run in a client yet; the owner's CR-14 UAT is next. The table below replaces the 2026-06-20 assessment. That assessment also overstated the world-entry sync as 100% done: only the known-blueprints list (139) was sent at login, and disciplines (136), paradigm levels (138) and the ASP total were not (campaign audit C-60, C-05).

| Subsystem | Client (SGW.exe) | Server (Python) | Rust server (2026-09-27) |
|---|---|---|---|
| State struct + DB persistence | VCrafting client-side | `Crafter.__init__` | `crates/entity/src/crafting.rs` + `base::crafting::persistence`; unchanged since #427 apart from the respec's spent-ASP counter |
| World-entry sync (136, 138, 139, ASP, 140) | expects each message | `onClientReady` | One reliable bundle after `onClientReady` (`base::crafting::sync`); before the campaign only 139 was sent |
| `onUpdateDiscipline` emit (136) | INT32+INT32 | `gainExpertise` | Sent at login, on learning, on every expertise gain and by the GM grants |
| GM grants | — | `commands/Crafting.py` | ASP and expertise (#427), `.allcraft`, `.craftkit`, `.learnblueprint` |
| `spendAppliedSciencePoints` (95) | INT32 disciplineId | full paradigm+prereq check | Implemented: one locked, replay-safe transaction; every refusal a line |
| `craft` (96) | craftId + ARRAY + qty | full validate/consume/timer/grant | Implemented: exact component-set match, consumed when the bar ends |
| `research` (97) | itemId + kickers | researchable check + random +5 | Implemented: the client's kicker rules, server-side roll, teaches the item's blueprint |
| `reverseEngineer` (98) | itemId | blueprint lookup, bias, recover | Implemented: recovery rises with expertise (campaign decision D-CR06), not the legacy bias |
| `alloying` (99) | craftId + tier item + elems | tier/quality validation | Implemented: the client's counts by stack quantity (Normal 10, Good 5, Great 2, Fantastic 1) |
| `respecCrafting` (100) with 112 and 137 | prompt Yes sends 100 | not implemented | Implemented: a player's `.respeccraft` opens the prompt, its Yes resets (CR-10) |
| 3s induction timer | TimerUpdate type 16 → `UEvent_UI_CraftInductionStart` | `Atrea.addTimer(3.0)` | Per-player induction engine, one running and ten in all, absolute expiry on the server's single game clock |
| `onUpdateCraftingOptions` (140) + entity gate | FIXED_DICT; `isCraftingAllowed` | `craftingEntityFlags` | Stations by `ENTITYFLAG_Craft_*` within 5 units, Field Crafting Tools in the crafting bag, sent at login and on change; the gate is enforced on the base |

The rest of this document is the 2026-06-20 plan and its evidence. Where it and the campaign disagree, the campaign's [crafting-client-ui.md](crafting-client-ui.md) and [crafting-items.md](crafting-items.md) findings and the [crafting system reference](../../gameplay/crafting-system.md) win.

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
  | 1 | CraftBlueprint | |
  | 2 | CraftResearch | shares static empty placeholder (no blueprint) |
  | 4 | CraftReverseEng | shares static empty placeholder (no blueprint) |
  | 8 | CraftAlloy | |

  **Correction (2026-09-26, CR-E1, audit C-61):** the offsets this table previously listed
  (`this+0x38`/`this+0x10`) as "blueprint" storage for `CraftBlueprint`/`CraftAlloy` were
  mislabelled, copying the same error the pre-existing Ghidra `PRE_COMMENT` on
  `Crafting_isCraftTypeAllowed` (`0x00e465d0`) carried. `Crafting_isCraftTypeAllowed` actually
  returns `this+0x38`/`this+0x40`/`this+0x48`/`this+0x50` for cases 1/8/2/4 respectively, and
  those four offsets hold the **(tool, machine) `CraftingInfo` pairs that message 140
  (`onUpdateCraftingOptions`) populates**, not a client-side known-blueprints cache. The Ghidra
  comment has been corrected accordingly. See
  [crafting-client-ui.md §2](crafting-client-ui.md#2-craftingoptions-140-unpacker-chain-and-the-c-61-correction-q2)
  for the full unpacker chain and evidence trail. The real known-blueprints accessor is the
  separate `Crafting_getKnownBlueprints` @ `0x00e46830`, whose own internal offsets were not
  re-derived by this correction.

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
| 138 | `onUpdateRacialParadigmLevel` | `INT32 aRacialParadigmId` + `INT8 aLevel` (resolved 2026-09-25, #728; see [crafting-wire-formats.md](crafting-wire-formats.md)) | HIGH (.def) |
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
`craftingEntityFlags` CELL_PRIVATE INT32). **Update (2026-09-27):** Rust enforces it on the base
(`base::crafting::gate`): a verb needs a station, a covering Field Crafting Tool, or the GM's
"craft anywhere".

## Open questions

> **Update (2026-09-27).** All five are answered: (1) method 138 is `INT32` + `INT8` (#728); (2) the client sends `respecCrafting` once, from the prompt's Yes ([crafting-client-ui.md §1](crafting-client-ui.md)); (3) the counts are the client's own, not `ALLOYING_ELEMENTARY_COUNTS` (§5 there); (4) the 140 pairs are (tool, machine) per craft type (§2 there, audit C-61); (5) Rust replaces the busy flag with the induction queue.

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
