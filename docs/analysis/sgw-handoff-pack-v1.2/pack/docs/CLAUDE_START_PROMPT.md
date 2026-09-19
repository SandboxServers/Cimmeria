# Prompt to give Claude with this folder

You are implementing the missing server/gameplay layer for a Stargate Worlds reconstruction using a surviving 2009 QA client and recovered structured data.

Read these files first, in order:
1. docs/SOURCE_POLICY.md
2. docs/KNOWN_UNKNOWNS.md
3. docs/IMPLEMENTATION_CHECKLIST.md
4. data/classes_and_skilltrees.json
5. data/trainer_server_export.json
6. data/abilities_final_v1.json
7. docs/COMBAT_SPEC.md
8. data/combat_config_v1.json

Rules:
- Preserve original numeric IDs.
- Do not invent unknown formulas, stats, prerequisites, coordinates, spawn counts or mission conditions and call them original.
- Treat fields explicitly marked PROJECT / RECONSTRUCTION as implementation values, not recovered FireSky values.
- Treat legacy/fan-dev SQL only as secondary structural evidence.
- Do not assume higher Ability ID means newer/final.
- Do not resolve tooltip↔effect conflicts silently.
- If an implementation decision is not supported, expose it as configurable and document it.
- Keep database migrations reversible.
- Before changing the current server schema, inspect it and map the supplied model onto existing tables instead of blindly creating duplicate systems.
- Make the smallest compatible change that implements the requested phase.
- For every phase, provide: files changed, schema changes, data migration, assumptions, unresolved blockers, and runnable tests.

First task:
Inspect the existing server project and map its current character, ability, trainer, inventory, combat and world tables/classes to this handoff. Do not implement changes yet. Produce a compatibility/gap report and a proposed Phase 1 patch plan for trainer + learned abilities.
