# Source Policy

Use these labels everywhere in code comments, migrations, implementation notes, and QA reports:

- **CONFIRMED / SOURCE-BACKED** — directly supported by raw client/runtime/server/internal technical evidence.
- **USER-CONFIRMED** — established by project-owner testing or project evidence but not independently recovered from raw source.
- **RECONSTRUCTION / INFERENCE** — a deliberate implementation choice based on incomplete evidence.
- **PARTIAL / UNRESOLVED / MISSING** — evidence is incomplete or contradictory.

## Source hierarchy

1. Raw QA client/runtime: UMAP/UPK/U, SGW.exe/config, Cooked SourceCache XML, Common schemas/entity definitions, UI/audio.
2. Original/direct server/database exports and internal technical docs with established provenance.
3. Cross-source exports: dialogues, mission/item exports, minimaps, technical workbooks.
4. User-confirmed observations/testing.
5. Reconstruction/community/project planning.

## Non-negotiable rules

- Never invent a value and label it original/canonical.
- Never assume a higher Ability ID is newer or more correct.
- Preserve tooltip↔effect conflicts instead of silently reconciling them.
- Preserve unknown monikers as unknown until evidence resolves them.
- Legacy/fan-dev SQL is a structural hint, not canonical final design.
- The 2009 QA client contains unfinished/test data; presence in the client is evidence of that snapshot, not proof of intended retail design.
