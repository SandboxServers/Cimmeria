# v1.2 — Combat Final v1

The full PROJECT Final-v1 combat model is now available in:
- `docs/COMBAT_SYSTEM_FINAL_V1.md`
- `data/SGW_Combat_System_Final_v1.json`
- `references/class_and_combat/SGW_Combat_System_Final_v1.xlsx`

This supersedes the separate Accuracy/Cover-only v1.1 file for implementation decisions, while preserving it as audit history.

# SGW Claude Server Handoff Pack v1

Generated: 2026-09-19

This pack is the implementation handoff for the **Stargate Worlds reconstruction project**. It is designed so a server developer can give the folder to Claude and have Claude work from explicit structured data instead of reconstructing context from chat history.

## What is ready

- **7 archetypes**
- **21 skill branches**
- **439 Final-v1 purchasable ability nodes**
- **280 reserve/review candidates**
- recovered ability/effect master for **1,886 abilities**
- recovered weapon family, ammo, auto-attack and weapon-ability mapping
- trainer/server JSON + CSV export
- source policy and implementation rules
- starter/world high-level routing index
- combat pipeline with unresolved formulas explicitly left unresolved
- QA checklist and implementation phases

## Read this order before coding

1. `docs/SOURCE_POLICY.md`
2. `docs/KNOWN_UNKNOWNS.md`
3. `docs/IMPLEMENTATION_CHECKLIST.md`
4. `data/classes_and_skilltrees.json`
5. `data/trainer_server_export.json`
6. `data/abilities_final_v1.json`
7. `docs/COMBAT_SPEC.md`
8. `data/combat_config_v1.json`
9. `docs/WORLD_CONTENT_INDEX.md`

## Critical distinction

The skill-tree **branch ownership and many ability mechanics are source-backed**, but the exact Final-v1 unlock levels, branch-point gates, skill-point cost, node ordering and prerequisite graph are a **project reconstruction** unless explicitly stated otherwise.

Do not convert a reconstruction field into an “original SGW value” merely because it is present in a Final-v1 JSON.

## Main machine-readable files

- `data/classes_and_skilltrees.json`
- `data/trainer_server_export.json`
- `data/trainer_server_export.csv`
- `data/abilities_final_v1.json`
- `data/abilities_full_recovered_master.json`
- `data/ability_reserve_review.json`
- `data/items_weapons_ammo.json`
- `data/weapons_full_recovered_variants.json`
- `data/starter_loadouts.json`
- `data/world_progression.json`
- `data/combat_config_v1.json`

## Recommended first implementation target

Implement **trainer + learned-ability persistence** without changing combat formulas:

`character → archetype → branch → node → unlock requirements → skill points → learned ability`

Then validate one class end-to-end (Soldier) before bulk-loading all seven.
