# RE Findings

This directory contains 63 per-system reverse engineering findings with evidence.

## Documents

| Document | Phase | Systems | Confidence |
|----------|-------|---------|------------|
| `combat-wire-formats.md` | 2 | Combat, abilities, stats, effects, timers | HIGH |
| `inventory-wire-formats.md` | 2 | Inventory, items, stores, loot | HIGH |
| `entity-property-sync.md` | 2 | Property IDs, method IDs, entity creation | HIGH |
| `gate-travel-wire-formats.md` | 3 | Stargate dialing, address discovery, passage | HIGH |
| `mission-wire-formats.md` | 3 | Missions, steps, objectives, tasks, rewards | HIGH |
| `organization-wire-formats.md` | 3 | Squads, guilds, strike teams, roster, ranks | HIGH |
| `crafting-wire-formats.md` | 3 | Craft, research, reverse engineer, alloy | HIGH |
| `minigame-wire-formats.md` | 4 | Minigame matchmaking, calls, spectating, helpers | HIGH |
| `chat-wire-formats.md` | 4 | Chat channels, tells, ignore, friends, GM petitions | HIGH |
| `mail-wire-formats.md` | 4 | Mail send/receive, attachments, COD, archive | HIGH |
| `black-market-wire-formats.md` | 4 | Auction search, create, bid, cancel, watch list | HIGH |
| `contact-list-wire-formats.md` | 4 | Contact lists, members, online/offline events | HIGH |
| `group-wire-formats.md` | 4 | Group authority, member coordination, mob groups | HIGH |
| `trade-wire-formats.md` | 4 | Player-to-player trade proposals, lock, confirm | HIGH |
| `duel-wire-formats.md` | 4 | Duel challenge, response, forfeit, participants | HIGH |
| `pet-wire-formats.md` | 4 | Pet abilities, stances, player-routed commands | HIGH |
| `entity-types-wire-formats.md` | 4 | Account, SGWEntity, SGWSpawnableEntity, SGWPet | HIGH |
| `entity-creation-wire-formats.md` | 5 | CREATE_BASE_PLAYER, CREATE_CELL_PLAYER, FORCED_POSITION, VIEWPORT_INFO, entity lifecycle | HIGH |
| `position-movement-wire-formats.md` | 5 | forcedPosition, detailedPosition, 32 avatarUpdate variants, velocity/direction compression | HIGH |
| `space-viewport-wire-formats.md` | 5 | Space, viewport, entity lifecycle, resource delivery, position updates | HIGH |
| `system-protocol-wire-formats.md` | 5 | Connection protocol handlers, decompilation evidence, message dispatch | HIGH |
| `cme-event-signal.md` | 6 | CME EventSignal emit pipeline (5 callees), `TypedEmitInfo`/`CallbackImpl` class anatomy, RTTI accessor clusters | HIGH |
| `state-flag-broadcast.md` | 7 | BSF_* flag master table (9 flags), `FUN_00e01c90` XOR-delta dispatch, witness-broadcast bugs for issues #219/#232/#249 | HIGH |
| `respawn-lifecycle.md` | 7 | Full death + respawn lifecycle — BSF_Dead, Defeat Window wire format, callForAid/GiveRespawner NetOut, respawner selection, same-world vs cross-world execution; issues #232/#233 | HIGH |
| `ability-resolution-pipeline.md` | V5 | Ability activation (button press → useAbility emit), TCM/TargetGround enum values, onEffectResults QR codes, 5 timer handlers (types 0-13), channeled ability cancel via ConfirmEffect, CooldownManager UI bridge | HIGH |
| `mission-state-machine.md` | V5 | MissionSet client-side state machine — onMissionUpdate/onStepUpdate/onObjectiveUpdate/onTaskUpdate handlers, timer routing, reward delivery, sharing flow, UI token table, MissionSet/MissionEntry/StepEntry/ObjectiveEntry/TaskEntry field layouts | HIGH |
| `world-entry-pipeline.md` | 4b | Full connect-to-in-world pipeline — 8 phases, CREATE_BASE_PLAYER/CELL_PLAYER wire formats, onClientMapLoad field-name audit, RESET_ENTITIES/ENABLE_ENTITIES exchange, mapLoaded bundle contents, CME signal bus inventory | HIGH |
| `architectural-anomalies.md` | V5 (W-anom) | Three CME EventSignal anomalies resolved: BM emitters use Pattern B (not unknown mechanism); GiveInventory NetOut has no client subscriber (server-only signal); SGWHomeless is `class_SGWHomeless`, an in-editor developer tool class | HIGH |
| `cooked-data-pipeline.md` | V5 (W-cooked) | 21 ServerSource categories (1–21) with binary-confirmed PAK filenames; LibCategory/ServerSource struct layout; 5 CME events per category; onVersionInfo/onCookedDataError handler logic; ZipStorageBase open+MetaData-write path; contradiction with existing pipeline doc category table | HIGH |
| `mercury-nub-anatomy.md` | V5 | Mercury `Nub` / `BaseNub` / `ChannelInternal` / `Connection` class layouts (22 functions, 4 struct anatomies); two-channel-map design; network thread loop; `Nub::send` 4-phase pipeline; rdtsc inactivity vs our `MAX_RETRIES`; two latent wire gaps (REPLY piggyback XOR-inverted length, ACK batching per 10ms tick) | HIGH |
| `dialog-portrait-lookup.md` | V5 | Dialog portrait + speaker-name lookup — wire EntityId path (not DatabaseId) through LookupEntityListenerEntry → slot 17 → UnitMappingChanged → createCharacterPortrait; CookedData SpeakerID PAK parse at piVar2[7]; empty-name fallback to player name; Prisoner 329 + Col Marsh root-cause diagnoses | HIGH |
| `client-wire-emit-suppression.md` | V5 | Client-side gates suppressing wire emit — Heal Focus arg-validation drop at `0x00aa2910`; P90 bandolier-swap Lua `getActiveSlotForContainer` no-op gate; in-flight ability queue at `GameEntityManager+0x228`; proposed server-side mitigations (resend `onActiveSlotUpdate` post-`onClientReady`, ensure `AbilityCooldownUpdate` drains the in-flight queue) | HIGH (binary anatomy) / MEDIUM (proposed mitigations need playtest) |
| `auth-and-crypto-modernization-targets.md` | #434 | Auth login transport (libcurl), client SHA-1 site, anti-debug status, Mercury crypto v2 targets | HIGH |
| `animation-system.md` | V5 | Animation system — sequence lookup, playback dispatch, and combat/weapon animation wiring in the client binary | HIGH |
| `annotation-script-shift-bugs.md` | Tooling | Annotation-script cyclic-shift bugs — how a growing strings table mis-aligns named functions, and the incidents to watch for on re-run | HIGH |
| `atrea-editor.md` | V5 | Atrea Editor (in-game UnrealEd) — architecture and developer-tool surface recovered from the binary | MEDIUM |
| `character-creation-pipeline.md` | V5 | Character creation pipeline — full analysis of the create-character flow from UI through wire to entity instantiation | HIGH |
| `client-instrumentation-hookpoints.md` | V5 | Client instrumentation hookpoints — addresses suitable for telemetry/RE hooks into client subsystems | HIGH |
| `client-instrumentation-entry-points.md` | Phase 3-6 manifest | Companion to hookpoints — resolved function entries + IAT slots for all Phase 3-6 `cimmeria-client-telemetry` hooks: state-flag dispatcher `0x00e01c90`, anim notify `0x00e974b0`/`0x00e97070`, cooked-data load `0x00420074`, ConsoleCommand `0x00539850`, CEGUI logEvent `0x012129E0`, Bink tick `0x0050BBC0`, Lua/OS/crash IAT slots | HIGH (Ghidra-resolved 2026-06-04) |
| `combat-damage-analysis.md` | V5 | Combat damage system — client-binary analysis of damage resolution and the numbers the client expects | HIGH |
| `cover-system.md` | V5 | Cover system — client-binary analysis of cover nodes and how cover affects combat | MEDIUM |
| `crafting-state-machine.md` | V5 | Crafting state machine — client-side craft/research/reverse-engineer state transitions | HIGH |
| `effect-execution-model.md` | V5 | Effect execution model — client-binary analysis of how effects/buffs/debuffs are applied and ticked | HIGH |
| `faction-alignment-system.md` | V5 | Faction / alignment system — how faction standing and alignment are tracked and surfaced to the client | MEDIUM |
| `inventory-state-machine.md` | V5 | Inventory state machine — client-side analysis of inventory operation transitions and locks | HIGH |
| `loot-generation.md` | V5 | Loot generation pipeline — how loot rolls and container contents are produced and delivered | MEDIUM |
| `mercury-protocol-internals.md` | V5 | Mercury protocol internals — client-binary analysis of the Mercury transport layer beyond the Nub anatomy | HIGH |
| `minigame-architecture.md` | V5 | Minigame architecture — client-binary analysis of the SmartFoxServer-based minigame subsystem | HIGH |
| `npc-ai-state-machine.md` | V5 | NPC AI state machine — client-binary analysis of mob aiState transitions (Idle/Fighting/Dead/Leashing) | HIGH |
| `npc-movement-pathfinding.md` | V5 | NPC movement and pathfinding — client-binary analysis of mob movement, navmesh use, and patrol routing | MEDIUM |
| `right-click-routing-on-corpse.md` | V5 | Right-click routing — why corpses fail to open loot; the corpse-context-menu dispatch diagnosis | HIGH |
| `spawn-system-mechanics.md` | V5 | Spawn system mechanics — client-binary analysis of spawn sets, regions, and spawnable-entity wiring | HIGH |
| `stargate-dhd-state-machine.md` | V5 | Stargate DHD state machine — dial-home-device interaction states and gate-activation flow | HIGH |
| `stat-scaling-formulas.md` | V5 | Stat scaling & XP progression — recovered stat-scaling and leveling formulas | MEDIUM |
| `struct-field-layouts.md` | V5 | FIXED_DICT struct field layouts — client-binary anatomy of key FIXED_DICT structures | HIGH |
| `weapon-ammo-pipeline.md` | V5 | Weapon / ammo pipeline — clip sizes, ammo consumption, and bandolier-slot wiring recovered from the binary | HIGH |
| `crafting-restoration.md` | Restore | Crafting (craft/research/reverse-engineer/alloy/spendASP) — 3-layer completeness assessment + phased Rust restoration plan (supersedes #53, tracked by #567) | HIGH |
| `organization-restoration.md` | Restore | Organization / squad / guild — completeness + phased plan; 9-rank + 26-bit permission model, OrgAuthority service (supersedes #68, tracked by #568) | HIGH |
| `duel-restoration.md` | Restore | Duel system — challenge/arena/PvP-flag lifecycle, SGWDuelMarker entity; never server-implemented originally (supersedes #70, tracked by #569) | HIGH |
| `pet-restoration.md` | Restore | Pet / companion — SGWPet entity, command dispatch, ownership/AoI; includes pet-wire-formats.md corrections (new, tracked by #570) | HIGH |
| `black-market-restoration.md` | Restore | Black market / auction house — listings/bids/expiry/CoD; CEGUI UI; includes wire-format corrections (supersedes #67, tracked by #571) | HIGH |
| `black-market-client-window-patch.md` | Restore/Client | Black market **client window** — runtime binary patch (deferred wide-Lua-injection) that opens the BM window; root-causes dropped method 90 (never bound into the dispatch map); owner-confirmed working; full recipe + addresses (tracked by #571 + launcher-integration issue) | HIGH |
| `contact-list-restoration.md` | Restore | Contact list (friends/ignore/presence fanout) — generic named-list model; answers #275 (supersedes #71, tracked by #572) | HIGH |

## Finding Format

Each finding should follow the template in [evidence-standards.md](../evidence-standards.md):

```markdown
## Finding: [Short Description]

**Confidence**: HIGH / MEDIUM / LOW
**Date**: YYYY-MM-DD
**Sources**: [list with addresses/lines]

### Description
[What was discovered]

### Wire Format (if applicable)
| Offset | Size | Type | Field |
|--------|------|------|-------|
| ...    | ...  | ...  | ...   |

### Evidence
[Decompiled code, cross-references]

### Implementation Impact
[What this means for the server code]
```

## Architecture Note

All wire formats are derived from `.def` files + `alias.xml` type definitions. This is possible because ALL entity method calls route through a single **universal RPC dispatcher** at `0x00c6fc40` in the client binary. The dispatcher serializes arguments using BigWorld's `DataType::addToStream` virtual methods — the wire encoding is determined entirely by the type system, not per-method handler code.

See `combat-wire-formats.md` for the full decompilation evidence and BigWorld DataType encoding reference table.