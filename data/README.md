# data/ — Cooked Game Data

Pre-processed game assets loaded by the server at runtime.

27 files across 2 directories.

## Structure

```
data/
├── cache/      Cooked .pak files — binary game data ready for serving to the client
└── spaces/     Space/zone definitions (navmesh references, chunk layouts)
```

## cache/ — Cooked .pak Files

Binary packages containing game resource data that the server sends to the game client on connection. These are generated from the source XML/DB data during a "cook" step.

Contents include: abilities, effects, archetypes, items, dialog trees, visual components, and other client-facing resource definitions.

**Do not edit these files directly.** They are binary-encoded. To modify game data, edit the source SQL in `db/resources/` or the entity definitions in `entities/` and re-cook.

## spaces/ — Zone Definitions

Per-zone configuration: which chunks are loaded, navmesh references, spawn region boundaries, and spatial parameters.

These correspond to the zones defined in `entities/cell_spaces.xml` and `db/resources/Worlds/`.

## Related

- `entities/` — Entity definition XML (parsed at startup, not cooked)
- `db/resources/` — Source data that gets cooked into cache/
- `docs/engine/cooked-data-pipeline.md` — How the cook pipeline works
- `docs/engine/cooked-data-pak-format.md` — .pak binary format spec
