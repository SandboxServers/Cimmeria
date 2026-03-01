# RE Findings

This directory contains per-system reverse engineering findings with evidence.

## Organization

Findings are organized by game system, matching the `docs/gameplay/` structure:

```
findings/
├── combat/           # Ability pipeline, damage, targeting
├── inventory/        # Container sync, equip, stores
├── missions/         # Mission state, rewards, objectives
├── gate-travel/      # Stargate dialing, zone transitions
├── crafting/         # Disciplines, recipes, alloys
├── organizations/    # Guild creation, ranks, vaults
├── protocol/         # Mercury wire format, property sync
└── engine/           # BigWorld internals, CME framework
```

## Finding Format

Each finding should follow the template in [evidence-standards.md](../../guides/evidence-standards.md):

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

## Directories

Subdirectories will be created as findings accumulate. No empty directories.
