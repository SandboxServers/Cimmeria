# ServerEd vs. the content engine — gap analysis

> **Last updated**: 2026-05-07
> **Audience**: Tech lead deciding whether to invest in tooling or extend the engine. Engineers triaging which legacy capabilities to resurrect.
> **Diátaxis type**: Explanation. Captures a decision and the tradeoffs behind it.

ServerEd is the legacy SGW visual node-graph editor that designers used to author mission/level/effect scripts. Its source lives at [tools/ServerEd/](../../tools/ServerEd/) (Qt 4/5 C++). This document compares its vocabulary and workflow to Cimmeria's current Rust content engine, identifies the gaps, and recommends what's worth resurrecting.

The short version: **ServerEd's *engine* surface is mostly already covered by Cimmeria's chain engine.** Around 20 trivial gaps (new enum variants) and 3-4 architectural gaps remain. **Its *tooling* surface — visual graph, DB lookup, live reload — is almost entirely uncovered.** The highest-ROI investments are validation + DB-lookup tooling and the trivial enum variants. Visual editing should wait until designer workflow is a real audience.

---

## 1. What ServerEd is and was

A Qt-based visual node-graph editor ([ServerEd.pro:7](../../tools/ServerEd/ServerEd.pro#L7)) that compiled designer-authored node graphs into Python source files for the legacy Atrea/SGW server's `cell.Script` framework.

- **Targets** three script types: `Mission`, `Level`/space, `Effect`. Outputs to `python/cell/missions/`, `python/cell/spaces/`, `python/cell/effects/` ([mainwindow.cpp:188-197](../../tools/ServerEd/mainwindow.cpp#L188-L197)).
- **Codegen pipeline:** dead-code elimination, Tarjan SCC cycle detection ([scriptcompiler.cpp:1182-1228](../../tools/ServerEd/scriptcompiler.cpp#L1182-L1228)), multi-pass optimizer up to 10 iterations ([scriptcompiler.cpp:1024-1114](../../tools/ServerEd/scriptcompiler.cpp#L1024-L1114)).
- **Output format:** hand-readable Python subclassing `Script` or `EffectScript` ([scriptcompiler.cpp:800-811](../../tools/ServerEd/scriptcompiler.cpp#L800-L811)). No bytecode. No SQL. No DB rows.
- **Live reload:** custom binary protocol over TCP, six message ops, `ReloadScriptRequest` pushed compiled scripts to a running server ([serverconnector.h:7-103](../../tools/ServerEd/serverconnector.h#L7-L103)).
- **Audience:** mixed. Property-browser UI and DB lookup widget targeted designers; Python escape hatches (custom `<Method>` blocks, `#if CONNECTED(Port)` preprocessor at [scriptcompiler.cpp:1513-1565](../../tools/ServerEd/scriptcompiler.cpp#L1513-L1565)) suggest heavy engineering involvement.

In Cimmeria's emulator the `python/cell/` tree is reference-only — no Python runs in production. Anything ServerEd produced has been or will be re-expressed as either Rust gameplay code or content-engine chain rows.

---

## 2. ServerEd's vocabulary — 109 nodes

Node templates live in [entities/editor/Nodes.xml](../../entities/editor/Nodes.xml) (109 `<Node>` definitions). The schema is the four-axis structure from [scriptdefinitions.h:65-80](../../tools/ServerEd/scriptdefinitions.h#L65-L80): `ref`, `type` ∈ `{Variable, Event, Condition, Action}`, `category`, plus typed I/O ports, properties, methods, imports, and seven script lifecycle hooks.

- **Variables (12)** — `Var_Bool/Int/Float/String/Vec3/Entity/Player`, plus 5 `EffectParam_*` (Bool/Int/Float/Str/Vec3) for effect-instance parameter binding.
- **Events (18)** — `Effect`, `MissionUpdate`, `MissionStepUpdate`, `MissionObjectiveUpdate`, `MissionTaskUpdate`, `Custom`, `Designer`, `Stargate`, `Dialog`, `DialogSetMap`, `DialogChoice`, `EntityInteract`, `Spawn`, `Dead`, `Stat`, `GenericRegion`, `ScriptLoaded`, `Loaded`, `Item`, `Inventory`, `Teleport`, `TeleportOut`.
- **Conditions (7)** — type-comparators for each scalar (`Cmp_Bool/Int/Float/Str/Entity/Vec3`), plus `Counter_Int`.
- **Actions (66)** — by category: Effect (2), Misc (8), Entity (24), Mission (10), Player (16), Variables (4 arithmetic ops).

Node bodies use templated Python with `VAR.R{}`, `VAR.W{}`, `PROPERTY{}`, `TRIGGER{}`, `PROPAGATE{}`, `LOCAL{}`, `NODEID{}` substitutions — a small DSL on top of plain Python.

---

## 3. Side-by-side coverage

The Cimmeria engine today has 24 Trigger variants ([triggers.rs:20-98](../../crates/content-engine/src/triggers.rs#L20-L98)), ~12 Condition variants ([conditions.rs:12-95](../../crates/content-engine/src/conditions.rs#L12-L95)), and ~50 Action variants ([actions.rs:20-237](../../crates/content-engine/src/actions.rs#L20-L237)).

Mapping by ServerEd category — abridged. The table groups by intent rather than node-by-node.

### Variables (12 nodes)
**Cimmeria equivalent:** none. Chains are flat `[trigger] → [conditions] → [actions]` lists with no inter-action dataflow. Variables only make sense in a *graph* model; ServerEd's `Var_*` and `Act_Get*` read-back nodes only existed to thread a value from one node's output into another's input. Cimmeria addresses this differently: action targets are addressed by tag/id directly, and shared state (mission status, counter, archetype) lives in `ExecutionContext` and is read by conditions, not by actions. **Architectural gap, not a feature gap** — see §5.

### Events (18 nodes) → Triggers (24)
| ServerEd | Cimmeria | Notes |
|---|---|---|
| `Event_Effect`, `Event_MissionUpdate`, `Event_MissionStepUpdate`, `Event_MissionObjectiveUpdate` | `OnEffectInit`, `OnMissionStep`, `OnMissionCompleted` | Mostly covered |
| `Event_MissionTaskUpdate` | **MISSING** | Mission *tasks* are absent (Cimmeria's mission model is mission → step → objective; legacy data has a fourth tasks layer) |
| `Event_Custom`, `Event_Designer` | `OnCustomEvent` | Direct equivalent |
| `Event_Stargate` | **MISSING** | Out of scope until stargate flow exists |
| `Event_Dialog`, `Event_DialogSetMap`, `Event_DialogChoice` | `OnDialogOpen`, `OnDialogSetOpen`, `OnDialogChoice` | Covered |
| `Event_EntityInteract` | `OnInteractTag`, `OnInteractTemplate` | Covered |
| `Event_Spawn`, `Event_Dead` | `OnEntityCreated`, `OnEntityDeath` | Covered |
| `Event_Stat` (stat threshold monitor) | **MISSING** | `StatBelowMax` is a *condition*, not a trigger |
| `Event_GenericRegion` | `OnRegionEnter`, `OnRegionExit` | Covered |
| `Event_ScriptLoaded` | **MISSING** | Module-init hook |
| `Event_Loaded` | `OnPlayerLoaded` | Covered |
| `Event_Item` | `OnItemUse`, `OnItemAcquired` | Covered |
| `Event_Inventory` (generic inventory monitor) | **MISSING** | |
| `Event_Teleport` | `OnTeleportIn` | Covered |
| `Event_TeleportOut` | **MISSING** | |

### Conditions (7) → Conditions (~12)
ServerEd's typed comparators (`Cmp_Bool/Int/Float/Str/Entity/Vec3`) are subsumed by `PropertyEquals` + `PropertyInRange` + the type-specific variants (`MissionStatus`, `StepStatus`, `ObjectiveStatus`, `Archetype`, `Counter`, `StatBelowMax`). Generic typed `<`/`>`/`!=` between two arbitrary variables is **not expressible** because Cimmeria has no graph variables. `Counter_Int` is fully equivalent to `Condition::Counter` + `IncrementCounter`/`ResetCounter` actions.

### Actions (66) → Actions (~50)
Most direct action equivalents exist. The notable gaps:

- **All read-back actions** (`Act_GetEntity`, `Act_GetLocation`, `Act_GetProperty`, `Act_GetStat`, `Act_GetDistance`, `Act_GetFacing`, `Act_GetCombatState`, `Act_GetAmmoStat`, `Act_GetActiveSlot`, `Act_GetMission*`) — same architectural gap as variables. They retrieved values into graph variables.
- **Arithmetic** (`Act_AddInt`, `Act_SubInt`, `Act_MulInt`, `Act_DivInt`) — same gap.
- **Randomness** (`Act_UniformRandom`, `Act_RandomRoll`) — no inline probability gate; loot tables have RNG built in.
- **Combat-state plumbing** (`Act_CheckEffect`, `Act_SetCombatState`, `Act_LockMovement`, `Act_AddComponent`, `Act_DelComponent`) — none of these exist as actions/conditions today.
- **Player-private property updates** (`Act_DynUpdate`, `Act_UpdateLocalProperty`) — no scope-aware property action.
- **Mission tasks** (`Act_UpdateMissionTask`) — task layer doesn't exist in Cimmeria's mission model.
- **Item updates** (`Act_UpdateItems`) — durability / charge / property mutation on existing item stacks.
- **Stargate addresses** (`Act_StargateAddress`) — out of scope until stargate exists.
- **Fan-in barrier** (`Act_Gate`) — chains are stateless per-trigger; no chain-scoped state across multiple triggers.
- **Designer `Act_Log`** — debug logging is the engine's job today, not exposed as a node.

For the full node-by-node mapping, see the agent audit at the bottom of this doc.

---

## 4. The gap list — three tiers

### Tier A — trivial enum additions (effort: 1-3 days each)

These are pure data-only additions to existing enums. No engine architecture changes.

- `Trigger::OnTeleportOut { region_id }` — mirror of `OnTeleportIn`.
- `Trigger::OnInventoryChange` — generic inventory-change watcher.
- `Trigger::OnStatThreshold { stat_id, op, value }` — stat-crossing-threshold event (vs. the existing `StatBelowMax` condition).
- `Trigger::OnNpcArrived { entity_tag, region_key }` — fired when a `MoveWaypoint` target reaches its destination. Requires path interpolation Rust-side first.
- `Condition::HasEffect { effect_id }`, `Condition::HasMoniker { name }`, `Condition::EffectAbsent { effect_id }`.
- `Action::Kill { entity_tag }` — vs. `DespawnEntity`. `Kill` triggers death events; `DespawnEntity` does not.
- `Action::SetCombatState { combat: bool }`, `Action::LockMovement { locked: bool }`.
- `Action::SetMoniker { name, value }`, `Action::RemoveMoniker { name }` — CRC32-hashed string flags on entities.
- `Action::RandomRoll { probability, on_success_chains, on_fail_chains }` — inline probability gate.
- `Action::UpdateItem { item_id, properties }` — durability/charge mutation.

### Tier B — architectural changes (effort: weeks)

- **Graph-scoped variables / read-back actions.** The single biggest gap. Adding this requires either:
  - A new "computed parameter" mechanism where actions reference other actions' outputs (e.g., `${prev.entity_id}` syntax), or
  - Promoting chains to a real DAG with typed edges.

  Without graph variables, `Act_Get*`, arithmetic nodes, and inline math operations don't make sense. Effort: 3-6 weeks for a minimal version.
- **Fan-in / barrier nodes (`Act_Gate`).** Requires per-chain instance state surviving across multiple triggers.
- **Mission tasks layer.** Schema + data + content-engine work, not an enum variant.
- **`EffectParam_*` typed binding.** First-class parameterized effect actions.

### Tier C — out of scope (don't reimplement)

- `Act_Log` — designer-facing debug logging. The engine's existing `tracing` events are the right shape.
- Free-form Python escape hatches (custom `<Method>` blocks, raw script in node bodies). The whole point of the data-driven rewrite is to *not* have these.
- `#if CONNECTED(Port)` preprocessor — codegen optimization, irrelevant in a runtime-evaluated chain model.
- Tarjan SCC cycle detection, dead-code elimination, inlining — artifacts of compiling Python source.

See [proposed-extensions.md](proposed-extensions.md) for the prioritized engine roadmap that draws from Tier A.

---

## 5. Tooling gaps — bigger than the engine gaps

The *engine* surface comparison is encouraging. The *tooling* surface is where real ground is missing.

### Database lookup workflow

ServerEd reads `<DatabaseRef>` queries from `Nodes.xml` (e.g. [Nodes.xml:13-16](../../entities/editor/Nodes.xml#L13-L16) for abilities — 16 refs total) and provides type-ahead search for designers picking ability/dialog/effect/item/mission IDs ([objectdatabase.h:10-47](../../tools/ServerEd/objectdatabase.h#L10-L47), [scriptdatabaselookupwidget.h:10-39](../../tools/ServerEd/scriptdatabaselookupwidget.h#L10-L39)).

In Cimmeria, the closest thing is `psql` against the live DB (for designers who can write SQL) or the [admin-api](../../crates/admin-api/) editor. There is no "type 'frost' to search dialogs" UX. For designers picking IDs to put in JSON chain rows, **this is a real workflow gap.**

### Live server connection / hot reload

ServerEd's `ReloadScriptRequest` ([serverconnector.h:92-103](../../tools/ServerEd/serverconnector.h#L92-L103)) pushed compiled scripts to a running server and forced module reimport. The legacy "edit graph, click Compile & Reload, see effect in-game" loop has no analog in Cimmeria — content changes require a server restart.

### Static validation at edit time

ServerEd validated chains at edit time: required-port warnings ([scriptcompiler.cpp:914-919](../../tools/ServerEd/scriptcompiler.cpp#L914-L919)), unknown-property warnings, cycle detection. Cimmeria validates chains only at load time ([engine_loader.rs](../../crates/services/src/cell/content/engine_loader.rs)) — and silently drops malformed rows with a `warn!` rather than failing loud. **A typo in chain authoring shows up as a silent no-op in-game** — the worst possible failure mode.

### Visual graph authoring

For complex 15-step branching missions, designers saw flow as boxes and arrows, not JSON. Authoring chain SQL by hand is a step backward in expressiveness for that audience. But the audience is the question — see §7.

### Bulk recompile

ServerEd had "pick a directory, recompile every `.script`" ([mainwindow.cpp:147-205](../../tools/ServerEd/mainwindow.cpp#L147-L205)). Cimmeria's equivalent is CI; no designer-driven "regenerate all chains" button. Less critical because chain SQL doesn't compile — it's already in the canonical form.

---

## 6. Workflow comparison

| Capability | ServerEd | Cimmeria | Gap severity |
|---|---|---|---|
| Authoring format | Visual node graph | Hand-written SQL / JSON | Medium (audience-dependent) |
| Type-ahead ID lookup | ✅ DB-backed search | ❌ `psql` or memorize | High for non-engineer authors |
| Edit-time validation | ✅ inline | ❌ load-time `warn!` only | **High** — silent runtime failures |
| Hot reload to running server | ✅ TCP push | ❌ restart server | Medium |
| Cycle / unreachable detection | ✅ Tarjan SCC | ❌ none | Low — chain model is flat, cycles less likely |
| Bulk recompile | ✅ button | ✅ CI / migration script | None |
| Live preview in client | ✅ via reload | ❌ restart + relog | Medium |

---

## 7. Recommendations

Ranked by ROI, assuming one-engineer effort estimates.

| Priority | Recommendation | Form | Effort |
|---|---|---|---|
| **1. Do** | Add the Tier A enum variants ([proposed-extensions.md](proposed-extensions.md)) tied to real shipped content | Engine extension | Per-variant: 1-3 days, incremental |
| **2. Do** | Build a chain-validation CLI (`cargo run -p content-engine --bin validate-chains`) that loads chains, type-checks references against the DB, warns on dangling `TriggerChain` refs and unreachable conditions | Tooling, not engine | 1 week. Eliminates a whole class of silent-no-op bugs. |
| **3. Consider** | DB lookup workflow — TUI or web tool with type-ahead for ability/item/dialog IDs. Check whether [crates/cimmeria-content-editor](../../crates/) (excluded from the build allowlist in [CLAUDE.md](../../CLAUDE.md)) already covers some of this scope. | Tooling crate | 1-2 weeks for a CLI with `fuzzy-matcher` + `sqlx` |
| **4. Consider** | Hot-reload RPC: `POST /content/reload-chains` on `admin-api` that re-runs `engine_loader::build_engine` and atomically swaps the engine | Engine + admin-api | 3-5 days. Big iteration-speed win. |
| **5. Defer / decide** | Visual node editor. Wait until you've decided whether designers will be a real audience for Cimmeria — the current audience appears to be developers writing JSON, who benefit more from validation than from graphs. | New tooling crate | 4-8 weeks for a first cut |
| **6. Defer / decide** | Graph-scoped variables and read-back actions. Big architectural change. **Worth it only if** you commit to a visual editor; without one, the JSON form of "node A's output → node B's input" is unusable by hand. | Engine architecture | 4-6 weeks. Decide alongside #5. |
| **7. Let die** | `Act_Log`, all of Tier C, Python escape hatches, the `scriptcompiler.cpp` codegen+optimizer pipeline, the Tarjan SCC machinery | — | — |
| **8. Let die unless data shows demand** | `EffectParam_*` typed binding, mission-task layer, `Act_GetMission*` read-backs. The legacy data uses these, but the audited content (16 of 1040 missions are functional, see [content-inventory.md](content-inventory.md)) is small enough that you can refactor the few real cases into chains without porting the abstraction. | — | — |

**Bottom line.** ServerEd's *engine* surface (the 109 nodes) is mostly already covered by Cimmeria's chain engine, with ~20 trivial gaps and 3-4 architectural gaps. Its *tooling* surface (DB lookup, live reload, visual graph) is almost entirely uncovered. The highest-ROI investments today are:

1. The Tier A enum variants tied to real content needs ([proposed-extensions.md](proposed-extensions.md)).
2. A chain-validation CLI to kill silent-no-op bugs.

Visual editing should wait until you have a designer audience to serve.

---

## Related

- [content-engine.md](content-engine.md) — Reference for the current Cimmeria engine surface.
- [proposed-extensions.md](proposed-extensions.md) — Roadmap of Tier A and Tier B engine extensions.
- [extending-the-engine.md](extending-the-engine.md) — How-to for adding new variants.
- [tools/ServerEd/](../../tools/ServerEd/) — Legacy editor source (read-only reference).
- [entities/editor/Nodes.xml](../../entities/editor/Nodes.xml) — Full 109-node vocabulary the editor exposed.
