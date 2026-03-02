# Game Content Data Audit

> **Last updated**: 2026-03-01

A content-level audit of all game data populating the Stargate Worlds server emulator. While the [gap analysis](../gap-analysis.md) catalogued 369 features across 38 systems, this audit maps what *content* actually exists: which zones are playable, where mission chains terminate, how NPCs connect to dialogs connect to missions, and where the original developers ran out of time.

## Top-Level Findings

1. **The game was in mid-development** — massive content authoring (6K items, 5K dialogs, 3K effects, 1K missions) but minimal systems wiring
2. **Only 2 of 24 zones are playable** — Castle_CellBlock (Praxis tutorial) and SGC_W1 (SGU tutorial)
3. **Only 1.5% of missions have scripts** — 16 of 1,040 missions are functional
4. **Only 2 of 8 archetypes are implemented** — Soldier and Commando have ability trees; 6 others are placeholders
5. **90% of items are orphaned** — not referenced by any loot table, vendor, reward, or crafting system
6. **Zero missions have XP or currency rewards** — all reward_xp and reward_naq fields are 0
7. **The loot system has 3 entries** — 2 loot tables with 3 total drops
8. **Dialog conditions are unused** — mission-gated dialog is 100% script-driven, not DB-driven

## Playability Matrix

Zone × Content-Type grid showing what exists in each zone.

| Zone | Verdict | Script | Nav | NPCs | Missions | Dialogs | Gates | Transport | Loot |
|------|---------|--------|-----|------|----------|---------|-------|-----------|------|
| Castle_CellBlock | **PLAYABLE** | 511 ln | YES | 28 | 15 scripted | YES | — | — | 2 tables |
| SGC_W1 | **PLAYABLE** | 480 ln | YES | 26 | 3 scripted | YES | YES | — | — |
| Castle | **PARTIAL** | 344 ln | — | 33 | 2 scripted | YES | YES | — | — |
| Harset | **PARTIAL** | 71 ln | YES | 22 | 1 of 26 scripted | YES | YES | 5 rings | — |
| Lucia | PARTIAL (transport) | 101 ln | — | 12 | — | — | YES | 11 rings | — |
| Menfa_Dark | PARTIAL (transport) | 37 ln | — | 25 | — | — | YES | 3 rings | — |
| Omega_Site | PARTIAL (transport) | 53 ln | — | 7 | — | — | YES | 5 beams | — |
| Omega_Site_CmdCenter | PARTIAL (transport) | 21 ln | — | 1 | — | — | — | 1 beam | — |
| Harset_CmdCenter | PARTIAL (transport) | 31 ln | — | 1 | — | — | — | teleport | — |
| Agnos | SHELL | — | YES | — | — | — | YES | — | — |
| Tollana | SHELL | — | — | 3 | — | — | YES | — | — |
| Beta_Site_Evo_1 | SHELL | — | — | 5 | — | — | YES | — | — |
| Dakara_E1 | SHELL | — | — | 1 | — | — | YES | — | — |
| Ihpet_Crater_Dark | SHELL | — | — | 1 | — | — | YES | — | — |
| Ihpet_Crater_Light | SHELL | — | — | 1 | — | — | YES | — | — |
| Menfa_Light | SHELL | — | — | — | — | — | YES | — | — |
| Sewer_Falls | SHELL | — | — | — | — | — | — | — | — |
| Agnos_Library | SHELL | — | — | — | — | — | — | — | — |
| SGC | SHELL | — | — | — | — | — | YES | — | — |
| Harset_Market | SHELL | — | — | — | — | — | — | — | — |
| Harset_StorageRm | SHELL | — | YES | — | — | — | — | — | — |
| Dakara_E1_StoryRm | SHELL | — | — | — | — | — | — | — | — |
| Tollana_Curia | SHELL | — | — | — | — | — | — | — | — |
| SandBox | DATA-ONLY | 18 ln | — | 1 | — | — | — | — | — |

Plus 67 DB-only worlds (not in spaces.xml) including 27 planned game zones, 25 MissionTest worlds, and various test maps.

## Content Summary

| Content Type | Total | Connected | Orphaned | % Connected |
|---|---|---|---|---|
| Missions | 1,040 | 16 (scripted) | 1,024 | 1.5% |
| Items | 6,059 | 595 | 5,464 | 9.8% |
| Abilities | 1,886 | ~172 | ~1,714 | 9.1% |
| Effects | 3,216 | 4 (scripted) | 3,212 | 0.12% |
| Dialogs | 5,411 | ~50 (est.) | ~5,361 | ~0.9% |
| Dialog Sets | 1,178 | ~10 | ~1,168 | 0.8% |
| Entity Templates | 153 | ~50 (spawned) | ~103 | 33% |
| Speakers | 602 | ~135 (named) | 467 | 22% |
| Worlds | 91 | 24 (in spaces.xml) | 67 | 26% |

## Document Index

| Document | Lines | Description |
|----------|-------|-------------|
| [content-inventory.md](content-inventory.md) | ~1,040 | Statistical inventory of all content types with completeness metrics and cross-reference counts |
| [zone-audit.md](zone-audit.md) | ~900 | Per-zone completeness scorecard with verdicts for all 24 defined zones plus 67 DB-only worlds |
| [mission-chains.md](mission-chains.md) | ~1,400 | Complete mission chain traces (scripted + unscripted analysis + inferred reconstruction + per-mission scorecard) |
| [association-map.md](association-map.md) | ~1,060 | The crazy wall: entity-to-everything map, mission connections, broken FKs, orphaned content, reconstruction web diagrams |
| [archetype-content-map.md](archetype-content-map.md) | ~505 | Per-archetype content availability: stats, ability trees, starting equipment, faction matrix |
| [reconstruction-map.md](reconstruction-map.md) | ~715 | What can be rebuilt from existing data vs holes vs never-built, with priority recommendations |

## How to Read These Documents

**Start here** if you want to know:
- *"What zones can I play in?"* → [zone-audit.md](zone-audit.md), Playability Matrix above
- *"How far can I play through the story?"* → [mission-chains.md](mission-chains.md), Part A
- *"What content exists for my archetype?"* → [archetype-content-map.md](archetype-content-map.md)
- *"How does everything connect?"* → [association-map.md](association-map.md), the reconstruction web
- *"What can we rebuild vs what's missing forever?"* → [reconstruction-map.md](reconstruction-map.md)
- *"Just give me the numbers"* → [content-inventory.md](content-inventory.md)

## Zone Progression (Inferred)

```
PRAXIS START                              SGU START
     |                                        |
Castle_CellBlock (L1)                    SGC_W1 (L1)
     |                                        |
Castle (L1+)                             SGC (L1+)
     |                                        |
     +--- Harset (L1+, hub) ------------------+
                    |
              Men'fa (L10-15) ← 38 missions, 0 scripts
                    |
          Ihpet Crater (L18) ← 22 missions, 0 scripts
                    |
            Dakara E2 (L24-25) ← 26 missions, 0 scripts
                    |
            Dakara E3 (L33) ← 24 missions, 0 scripts
                    |
        Return to Ihpet (L38) ← 16 missions, 0 scripts
                    |
    Finding Col. Wertz (L40) ← 16 missions, 0 scripts
    Find the Venio (L40-43) ← 17 missions, 0 scripts
                    |
           Endgame (L46-50) ← scattered missions
```

**Confidence:** Castle_CellBlock and SGC_W1 chains are CONFIRMED from Python scripts. Everything below Harset is MEDIUM confidence, inferred from mission levels and labels.

## Reconstruction Priority

### Quick Wins (days)
- Stargate address population (TRIVIAL — data exists inline)
- Navmesh generation for 19 remaining zones (NavBuilder works)
- Mission reward XP/currency formulas (algorithmic)
- Dialog condition SQL updates (populate empty arrays)

### Medium Investment (weeks)
- Mission scripts from DB data (~300 SCRIPTABLE missions with rich step text)
- Effect script generation (stub generator + 4 working templates)
- Loot table generation (items + mob levels → probability tables)
- Spawn placement (templates + zones → coordinates)

### Long-term Design (months)
- Ability trees for 6 placeholder archetypes (~500 new abilities needed)
- Combat formula calibration
- SHELL zone content (14 zones with art but zero gameplay)

### Out of Scope (requires game design team)
- PvP/dueling system design
- End-game content
- Unique archetype identities for 6 classes

## Confidence Framework

All documents use a 5-tier confidence system:

| Tag | Meaning | Source |
|-----|---------|--------|
| **CONFIRMED** | Verified in DB + code + tested | Direct evidence from running server |
| **HIGH** | Present in DB or code, cross-referenced | DB query + code read |
| **MEDIUM** | Inferred from cross-referencing multiple sources | DB joins, file matching, text parsing |
| **LOW** | Based on client file names or structural inference | Client directory listing |
| **ASSUMPTION** | MMO industry norms or developer intent guess | No direct evidence |

## Related Documents

- [Gap Analysis](../gap-analysis.md) — System-level feature gap analysis (369 features, 38 systems)
- [Gameplay Systems Dashboard](../gameplay/README.md) — Per-system implementation status
- [Project Status](../project-status.md) — Overall project roadmap
- [Game Data](../game-data.md) — High-level game data overview
