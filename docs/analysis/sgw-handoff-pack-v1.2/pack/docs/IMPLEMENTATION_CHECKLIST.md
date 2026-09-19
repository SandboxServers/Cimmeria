# v1.2 Combat Implementation Update

Combat formulas are now complete for PROJECT Final v1. Use `COMBAT_SYSTEM_FINAL_V1.md` / `SGW_Combat_System_Final_v1.json` as the implementation authority for reconstructed formulas. Raw Cooked ability/effect payloads still outrank reconstruction values.

# v1.1 Combat Update

Accuracy/Defense + directional Cover is now finalized for **PROJECT v1**.
Read `docs/COMBAT_ACCURACY_COVER_FINAL_V1.md` and `data/SGW_Combat_Accuracy_Cover_Final_v1.json`.
The original retail formulas remain unknown; these are explicitly reconstruction values.

# Implementation Checklist

## Phase 0 — Compatibility audit
- Back up database and server code.
- Identify current character/archetype, abilities, effects, inventory, trainer, world and mission tables/classes.
- Map current IDs to recovered IDs.
- Report collisions and legacy fan-server assumptions.
- Do not patch yet.

## Phase 1 — Learned abilities + trainer
- Import `trainer_server_export.json`.
- Implement/preserve skill points.
- Implement level, branch-point and prerequisite validation.
- Persist learned abilities.
- Expose unavailable abilities as unavailable to the client.
- Start with Soldier only.
- Run the trainer QA suite.
- Load the remaining six archetypes after Soldier passes.

## Phase 2 — Ability runtime
- Bind learned Ability IDs to recovered ability/effect data.
- Enforce weapon requirements.
- Implement cooldown/warmup.
- Implement ammo/resource consumption.
- Preserve tooltip/effect conflicts in logs/config.

## Phase 3 — Combat resolver
- Implement the pipeline in `COMBAT_SPEC.md`.
- Keep unresolved formulas configurable.
- Implement directional cover and crouch state.
- Implement Focus/Health and state/resist hooks.
- Add combat logging sufficient to reproduce hit decisions.

## Phase 4 — Weapons/items
- Import weapon-family and auto-attack mappings.
- Implement 4-slot bandolier/active weapon semantics expected by client.
- Implement reload and ammo-type toggles.
- Keep TechComp scaling configurable until recovered.

## Phase 5 — Character starts
- Implement only start worlds that are approved in `starter_loadouts.json`.
- Do not infer unknown starter inventory.
- Fix char-creation appearance serialization separately from class progression.

## Phase 6 — World/mission content
- Use the world workbooks in `references/world_content/`.
- Respect strict map scopes, especially Castle_CellBlock ending at Mission 688.
- Rebuild persistent spawns/regions/scripts only from evidence or explicitly labeled reconstruction.

## Phase 7 — NPC/enemy/loot
- Create master spawn/archetype/loot mappings.
- Use recovered ability IDs for enemy combat kits where supported.
- Do not scale stats with guessed retail formulas.

## Phase 8 — QA/regression
- Execute `QA_TESTS.md`.
- Record source-backed vs reconstruction failures separately.
- Never fix a reconstruction mismatch by altering source-backed raw data.
