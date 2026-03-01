# Systematic Reverse Engineering Plan for Stargate Worlds

## Context

Cimmeria's C++ server infrastructure is 75-80% complete (Mercury networking, entity lifecycle, database, auth, AoI, navigation all working). What's missing is primarily **game logic** (combat completion, crafting, gate travel, minigames, guilds, mail, etc.) and some **BigWorld engine subsystems** (watchers, space slicing, LOD).

SGW.exe is loaded in Ghidra with ~85,000 unnamed functions. The binary is rich with debug artifacts: **253 Event_NetOut_* strings** (client-to-server messages), **167 Event_NetIn_* strings** (server-to-client messages), BigWorld source path assertions, RTTI type descriptors, UE3 exec stubs, Mercury debug strings, and CME framework symbols. These are high-confidence naming anchors.

The client data directory (`Stargate Worlds-QA/`) has canonical entity .def files, XSD schemas, and 140KB of enumerations defining the complete client-server contract.

BigWorld Engine reference source (1.9.1 and 2.0.1, 3,631 files) is available in `external/` for cross-referencing.

**Goal**: Replicate STBC's methodical, evidence-driven RE approach — structured docs with hub-and-spoke architecture, Ghidra annotation scripts for systematic function naming, specialized agents, and rigorous evidence standards.

---

## Phase 1: Foundation (Week 1)

### 1a. Create docs/ structure

```
docs/
├── README.md                         # Master navigation hub
├── protocol/                         # Wire formats, Mercury, message serialization
│   ├── README.md
│   ├── message-catalog.md            # HUB: All 420 Event_NetOut/NetIn with handler addresses
│   ├── mercury-wire-format.md
│   ├── entity-property-sync.md
│   ├── login-handshake.md
│   └── position-updates.md
├── gameplay/                         # Per-system game mechanics
│   ├── README.md                     # HUB: System dashboard (status, % complete, key events)
│   ├── combat-system.md              # QR, damage pipeline, armor, absorption
│   ├── ability-system.md             # Warmup, cooldown, targeting, TCM
│   ├── effect-system.md              # Pulsing, duration, stat changes, diminishing returns
│   ├── stat-system.md                # 70+ stats, 6-tier dict, regen
│   ├── inventory-system.md           # Bags, slots, equip, containers
│   ├── crafting-system.md            # Disciplines, paradigms, tech comp
│   ├── mission-system.md             # Steps, objectives, tasks, rewards
│   ├── gate-travel.md                # Stargate dialing, ring transporters
│   ├── minigame-system.md            # 5 minigames + framework
│   ├── organization-system.md        # Guilds, ranks, permissions, vaults
│   ├── group-system.md               # Teams, squads, loot distribution
│   ├── chat-system.md                # Channels, tells, DND/AFK
│   ├── mail-system.md
│   ├── trade-system.md
│   ├── black-market.md
│   ├── pet-system.md
│   └── duel-system.md
├── engine/                           # BigWorld + CME internals
│   ├── README.md
│   ├── entity-type-catalog.md        # HUB: All 18 entities + 18 interfaces
│   ├── bigworld-architecture.md
│   ├── cme-framework.md              # CME::PropertyNode, EventSignal, SoapLibrary
│   ├── cooked-data-pipeline.md       # .pak format, XSD schemas
│   ├── watcher-system.md
│   └── space-management.md
├── architecture/                     # Cimmeria server architecture
│   └── service-architecture.md
├── analysis/                         # Investigation logs
│   ├── event-net-mapping.md          # Event names ↔ .def method correlation
│   └── bigworld-reference-index.md   # Index of useful BW reference files by topic
├── guides/
│   ├── evidence-standards.md
│   ├── reading-decompiled-code.md
│   └── entity-def-guide.md
└── reverse-engineering/
    ├── PLAN.md                       # This file
    ├── STATUS.md                     # Progress tracker (updated each session)
    ├── annotation-scripts/           # 10 Jython scripts
    ├── function-naming-progress.md
    ├── address-map.md                # Key vtables, globals, important functions
    └── findings/                     # Per-system RE findings with evidence
```

### 1b. Write 10 Ghidra annotation scripts

| # | Script | Target | Yield | Confidence |
|---|--------|--------|-------|------------|
| 01 | `rtti_annotator.py` | `.?AV...@@` RTTI → vtable/class naming | 200-400 | HIGH |
| 02 | `ue3_exec_annotator.py` | `int*exec*` strings → UE3 native functions | 500-1000 | HIGH |
| 03 | `bigworld_source_annotator.py` | `..\..\Server\bigworld\src\*` paths → BW functions | 100-300 | HIGH-MED |
| 04 | `event_signal_annotator.py` | `Event_NetOut_*`/`Event_NetIn_*` → handlers | ~420 | HIGH |
| 05 | `mercury_annotator.py` | `Mercury::*` debug strings → protocol functions | 30-50 | HIGH |
| 06 | `cme_framework_annotator.py` | `CME::*` mangled symbols → framework functions | 50-100 | HIGH |
| 07 | `vtable_annotator.py` | Named vtables → `ClassName::vfunc_N` propagation | 2000-5000 | MEDIUM |
| 08 | `lua_binding_annotator.py` | toLua registration → wrapper functions | 100-200 | MED-HIGH |
| 09 | `string_discovery.py` | Remaining `::` debug strings | 500-2000 | LOW-MED |
| 10 | `xref_propagation.py` | Call chain inference from named neighbors | 1000-3000 | LOW |

**Total**: 4,000-12,000 functions named. ~40-60% of **server-relevant** functions.

Script 04 is the most valuable for server development — it maps every client↔server message to a decompilable handler function.

### 1c. Create hub documents

**message-catalog.md**: All 253 Event_NetOut + 167 Event_NetIn mapped to .def methods, handler addresses, RE status, implementation status.

**entity-type-catalog.md**: All 18 entities + 18 interfaces with property counts by flag, method counts, implementation %.

**gameplay/README.md**: Dashboard of every game system with status, key events, key .def interfaces.

---

## Phase 2: Combat & Core Systems RE (Weeks 2-4)

### 2a. Combat system completion (server is ~70%)

**Decompile in Ghidra:**
- `Event_NetOut_UseAbility` — client ability request serialization
- `Event_NetOut_SetAutoCycle` — auto-attack toggle format
- `Event_NetIn_onEffectResults` — effect result notification format
- `Event_NetIn_TimerUpdate` — cooldown/warmup timer sync
- `Event_NetIn_onStatUpdate` / `onStatBaseUpdate` — stat change format

**Missing to implement**: TCM_Cone/AoE/Chain targeting, channeled abilities, threat/aggro, diminishing returns.

**Cross-ref**: `AbilityManager.py` (1091 lines), `SGWCombatant.def` (28 props, 19 cell methods), `Ability.xsd`, `Effect.xsd`, `enumerations.xml`

### 2b. Inventory completion (server is ~80%)

**Decompile**: `Event_NetIn_onContainerInfo`, `onUpdateItem`, `onRemoveItem`, `Event_NetOut_RepairItem`

**Missing**: stat recalculation on equip, repair, org vault integration.

### 2c. Entity property sync protocol

**Decompile**: Functions referencing `entity_manager.cpp` and `servconn.cpp` strings.
**Cross-ref**: BW 2.0.1 `src/lib/connection/` and `src/lib/entitydef/`
**Critical**: Wrong property IDs = client desync/crash.

---

## Phase 3: Missing Systems RE (Weeks 5-8)

| Week | System | Status | Key Events to Decompile |
|------|--------|--------|------------------------|
| 5 | Gate travel | ~20% | `onDialGate`, `setupStargateInfo`, `onStargatePassage` |
| 6 | Mission completion | ~40% | `onMissionUpdate`, `onStepUpdate`, `MissionOffer`, `MissionRewards` |
| 7 | Crafting | 0% | `onUpdateDiscipline`, `onUpdateCraftingOptions`, `onCraftingRespecPrompt` |
| 8 | Organization | ~5% | `OrganizationCreation`, `onOrganizationInvite`, vault events |

---

## Phase 4: Secondary Systems RE (Weeks 9-12)

| Weeks | System | Key Events |
|-------|--------|------------|
| 9-10 | Minigames (5 stubs) | `onStartMinigame`, `onEndMinigame`, `MinigameCallDisplay`/`Result`/`Abort` |
| 9 | Chat/channels | `onChatJoined`, `onChatLeft`, `onNickChanged` |
| 10 | Mail | `onMailHeaderInfo`, `onMailRead`, `onSendMailResult` |
| 11 | Pet system | `PetAbilities`, `PetStances`, `PetStanceUpdate` |
| 11-12 | Black market | `BMOpen`, `BMAuctions`, `BMAuctionUpdate` |
| 12 | Duel | `onDuelChallenge`, `onDuelEntitiesSet` |

---

## Phase 5: BigWorld Engine Subsystems (Weeks 13-16)

From **reference source analysis** (no Ghidra needed):

- **Watcher system** — reactive property monitoring (BW 2.0.1 `src/lib/server/`)
- **Space slicing** — multi-CellApp load balancing
- **LOD entity updates** — bandwidth optimization
- **Distributed checkpointing** — crash recovery

---

## Agent Roster

| # | Agent | Focus |
|---|-------|-------|
| 1 | **Protocol Archaeologist** | Event handler decompilation, wire format documentation |
| 2 | **Combat Systems Analyst** | Ability/effect/stat mechanics validation against client |
| 3 | **Game Data Analyst** | .pak format, XSD schemas, CookedElementBase |
| 4 | **Entity System Mapper** | Property sync, entity lifecycle, BW↔UE3 bridge |
| 5 | **Minigame RE** | 5 stubbed minigames, call/response protocol |
| 6 | **BigWorld Reference Analyst** | Engine gap analysis from BW reference source |
| 7 | **Implementation Bridge** | Translate RE findings → Cimmeria code changes |

---

## Evidence Standards

**HIGH**: Decompiled function + corroborated by ≥2 sources (.def, XSD, BW reference)
**MEDIUM**: Single source only (e.g., just .def or just BW analogy)
**LOW**: Inferred from patterns, incomplete decompilation, or external reports

Every finding: confidence level, source citations with addresses/lines, wire format table if applicable, implementation impact.

---

## Key Files

| File | Role |
|------|------|
| `entities/defs/SGWPlayer.def` (~65KB) | Complete player contract — 11 interfaces |
| `entities/defs/interfaces/SGWCombatant.def` | Combat: 44 props, 15 cell methods, 4 client methods |
| `python/cell/AbilityManager.py` (1091 lines) | Core combat/ability logic — most important Python file |
| `external/engines/BigWorld-Engine-2.0.1/src/lib/connection/` | BW reference for client-server protocol |
| `SGW client: Common/res/entities/enumerations.xml` (140KB) | Rosetta Stone for all game constants |
| `SGW client: Common/xml/SGWShared/CookedData/*.xsd` | 21 data format schemas |

---

## Verification

1. **Annotation scripts**: Run each, verify function count, spot-check 10 names per script
2. **Wire format docs**: Construct test messages, send to client, verify no disconnect
3. **Game system docs**: Validate against existing Python implementation, flag discrepancies
4. **Hub documents**: Every Event_NetOut/NetIn accounted for, every .def method mapped
5. **Cross-reference**: Key findings verified against both Ghidra decompilation AND BW reference source
