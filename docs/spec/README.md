---
title: Cimmeria Bible
chapter_id: spec.meta.index
status: verified
last_verified: 2026-05-13
verified_by: documentation-writer
audience: anyone using docs/spec/ as a reference
type: reference
---

# Cimmeria Bible

The canonical, evidence-backed reference for what Cimmeria (and the 2009 SGW server it emulates) does. Every chapter answers a single question — *"what does X do, and how do we know?"* — and walks from ground truth (the SGW binary) out to current Rust through a 5-section evidence chain.

**New to the bible? Start with [`how-to-read.md`](how-to-read.md).** It explains the chapter shape, what the status tags mean, and how to challenge a claim you think is wrong.

**Writing a chapter? Start with [`how-to-write.md`](how-to-write.md)** and the template at [`../../.templates/spec-chapter.md`](../../.templates/spec-chapter.md).

The bible is *what the system does*. Decisions about *why we chose X over Y* live in [`../architecture/`](../architecture/) as ADRs — the bible does not duplicate them.

---

## How to use this bible

| You want to... | Go to... |
|---|---|
| Look up a single behavior's canonical reference | Find the chapter in the [system index](#system-index) below, cite by `chapter_id`. |
| Understand the chapter format and trust signals | [`how-to-read.md`](how-to-read.md). |
| Author a new chapter | [`how-to-write.md`](how-to-write.md) → [`conventions.md`](conventions.md) → [`../../.templates/spec-chapter.md`](../../.templates/spec-chapter.md). |
| Look up bible vocabulary (Cell, AoI, propID, BSF_*, etc.) | [`glossary.md`](glossary.md). |
| Challenge a claim | [`how-to-read.md § challenging a claim`](how-to-read.md#challenging-a-claim). |
| Find which V5 finding doc maps to which chapter | Issue [#264](https://github.com/SandboxServers/Cimmeria/issues/264) second comment — the V5 evidence index. |

---

## Status snapshot

Phase 0 (scaffolding — what you are reading) is in place. Phase 0.5 (six infrastructure chapters) and Phase 1 (eleven gameplay chapters) are queued. See the phasing in issue [#264](https://github.com/SandboxServers/Cimmeria/issues/264).

| Phase | Chapters | Status |
|---|---:|---|
| 0 — meta layer (this scaffolding) | 4 + template + workflow | In place. |
| 0.5 — infrastructure prerequisites | 6 | Not yet authored. |
| 1 — gameplay | 11 | Not yet authored. |
| 2+ — triage remaining ~65 prior docs | TBD | Ongoing. |

A chapter that does not yet exist is listed below with `coming soon: <evidence-source>` so you know where the evidence lives in the meantime.

---

## System index

Each row is a `chapter_id`. The link target is the chapter when it exists; otherwise it lists the evidence source you should cite directly in the interim.

### `spec.meta` — bible apparatus

| chapter | status | location |
|---|---|---|
| `spec.meta.index` | verified | [this file](README.md) |
| `spec.meta.how-to-read` | verified | [`how-to-read.md`](how-to-read.md) |
| `spec.meta.how-to-write` | verified | [`how-to-write.md`](how-to-write.md) |
| `spec.meta.conventions` | verified | [`conventions.md`](conventions.md) |
| `spec.meta.glossary` | verified | [`glossary.md`](glossary.md) |

### `spec.engine` — BigWorld and CME internals

| chapter | status | source |
|---|---|---|
| `spec.engine.cme-event-signal` | coming soon (Phase 0.5) | evidence: [`../reverse-engineering/findings/cme-event-signal.md`](../reverse-engineering/findings/cme-event-signal.md) |
| `spec.engine.universal-rpc-dispatcher` | coming soon (Phase 0.5) | evidence: `ghidra://SGW.exe@0x00c6fc40`, [`../reverse-engineering/findings/combat-wire-formats.md`](../reverse-engineering/findings/combat-wire-formats.md) |
| `spec.engine.entity-description-parse-chain` | coming soon (Phase 0.5) | evidence: [`../reverse-engineering/findings/entity-property-sync.md`](../reverse-engineering/findings/entity-property-sync.md) |
| `spec.engine.cooked-data-pipeline` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/cooked-data-pipeline.md`](../reverse-engineering/findings/cooked-data-pipeline.md) |

### `spec.protocol` — wire formats and messaging

| chapter | status | source |
|---|---|---|
| `spec.protocol.mercury-wire-format` | draft | [`../drafts/spec/mercury-wire-format.md`](../drafts/spec/mercury-wire-format.md) |
| `spec.protocol.position-updates` | draft | [`../drafts/spec/position-updates.md`](../drafts/spec/position-updates.md) |
| `spec.protocol.entity-property-sync` | coming soon (Phase 0.5) | evidence: [`../reverse-engineering/findings/entity-property-sync.md`](../reverse-engineering/findings/entity-property-sync.md) |
| `spec.protocol.message-catalog` | coming soon (Phase 0.5) | evidence: [`../protocol/message-catalog.md`](../protocol/message-catalog.md), the 19 V5 wire-format finding docs |
| `spec.protocol.combat-wire-formats` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/combat-wire-formats.md`](../reverse-engineering/findings/combat-wire-formats.md) |

### `spec.world` — world entry and space

| chapter | status | source |
|---|---|---|
| `spec.world.world-entry` | coming soon (Phase 1, first gameplay chapter) | evidence: [`../reverse-engineering/findings/world-entry-pipeline.md`](../reverse-engineering/findings/world-entry-pipeline.md) |

### `spec.player` — player entity lifecycle

| chapter | status | source |
|---|---|---|
| `spec.player.character-creation` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/character-creation-pipeline.md`](../reverse-engineering/findings/character-creation-pipeline.md) |
| `spec.player.spawn` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/world-entry-pipeline.md`](../reverse-engineering/findings/world-entry-pipeline.md) (entry path), [`../reverse-engineering/findings/respawn-lifecycle.md`](../reverse-engineering/findings/respawn-lifecycle.md) (respawn arc) |
| `spec.player.death-respawn` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/respawn-lifecycle.md`](../reverse-engineering/findings/respawn-lifecycle.md) |
| `spec.player.animations` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/animation-system.md`](../reverse-engineering/findings/animation-system.md) |
| `spec.player.state-fields` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/state-flag-broadcast.md`](../reverse-engineering/findings/state-flag-broadcast.md) |
| `spec.player.faction-alignment` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/faction-alignment-system.md`](../reverse-engineering/findings/faction-alignment-system.md) |

### `spec.combat` — abilities, weapons, effects, damage

| chapter | status | source |
|---|---|---|
| `spec.combat.weapons-and-ammo` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/weapon-ammo-pipeline.md`](../reverse-engineering/findings/weapon-ammo-pipeline.md) |
| `spec.combat.abilities` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/ability-resolution-pipeline.md`](../reverse-engineering/findings/ability-resolution-pipeline.md) |
| `spec.combat.ability-resolution` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/ability-resolution-pipeline.md`](../reverse-engineering/findings/ability-resolution-pipeline.md) |
| `spec.combat.effects-execution` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/effect-execution-model.md`](../reverse-engineering/findings/effect-execution-model.md) |
| `spec.combat.damage-pipeline` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/combat-damage-analysis.md`](../reverse-engineering/findings/combat-damage-analysis.md) |
| `spec.combat.combat-lifecycle` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/state-flag-broadcast.md`](../reverse-engineering/findings/state-flag-broadcast.md) |
| `spec.combat.wire-formats` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/combat-wire-formats.md`](../reverse-engineering/findings/combat-wire-formats.md) |
| `spec.combat.loot-generation` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/loot-generation.md`](../reverse-engineering/findings/loot-generation.md) |

### `spec.inventory` — containers and equip

| chapter | status | source |
|---|---|---|
| `spec.inventory.containers-and-equip` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/inventory-state-machine.md`](../reverse-engineering/findings/inventory-state-machine.md) |

### `spec.missions` — mission lifecycle

| chapter | status | source |
|---|---|---|
| `spec.missions.lifecycle-and-objectives` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/mission-state-machine.md`](../reverse-engineering/findings/mission-state-machine.md) |

### `spec.npcs` — NPC AI and movement

| chapter | status | source |
|---|---|---|
| `spec.npcs.spawn-system` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/spawn-system-mechanics.md`](../reverse-engineering/findings/spawn-system-mechanics.md), [`../reverse-engineering/findings/npc-ai-state-machine.md`](../reverse-engineering/findings/npc-ai-state-machine.md) |
| `spec.npcs.movement-and-pathfinding` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/npc-movement-pathfinding.md`](../reverse-engineering/findings/npc-movement-pathfinding.md) |
| `spec.npcs.cover-behavior` | coming soon (Phase 1) | evidence: [`../reverse-engineering/findings/cover-system.md`](../reverse-engineering/findings/cover-system.md) |

### `spec.crafting` — crafting and research

| chapter | status | source |
|---|---|---|
| `spec.crafting.state-machine` | coming soon (Phase 2+) | evidence: [`../reverse-engineering/findings/crafting-state-machine.md`](../reverse-engineering/findings/crafting-state-machine.md) |

### `spec.gate-travel` — DHD and ring transport

| chapter | status | source |
|---|---|---|
| `spec.gate-travel.dhd-and-stargate` | coming soon (Phase 2+) | evidence: [`../reverse-engineering/findings/stargate-dhd-state-machine.md`](../reverse-engineering/findings/stargate-dhd-state-machine.md) |

---

## Deprecated chapters

When a chapter is replaced by a successor, the old `chapter_id` goes here so commit messages and old PRs that cite it still have a landing pad. Empty for now.

| chapter | superseded by | reason |
|---|---|---|

---

## Disputed chapters

Chapters with one or more `disputed_by:` issues open. Each row links to the chapter and the dispute thread. Empty for now.

| chapter | disputed by |
|---|---|

---

## What the bible is not

Worth repeating up front:

- **Not tutorials.** Tutorials live in [`../guides/`](../guides/) and the project [`../../README.md`](../../README.md).
- **Not ADRs.** Architecture decisions live in [`../architecture/`](../architecture/). The bible says what the system does, not why we picked it.
- **Not process docs.** [`../../CLAUDE.md`](../../CLAUDE.md), [`../../TESTING.md`](../../TESTING.md), and [`../../.github/copilot-instructions.md`](../../.github/copilot-instructions.md) cover how-we-work.
- **Not WIP notes.** RE session logs and V5 status live under [`../reverse-engineering/`](../reverse-engineering/). The bible cites them as evidence; it does not replicate them.

If you are about to add a doc and you are not sure where it goes: not here unless it has a `chapter_id` and a 5-section evidence chain. The point of the bible is single-source-of-truth; broaden the scope and that invariant breaks.
