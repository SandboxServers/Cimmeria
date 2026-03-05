# RE Findings

This directory contains per-system reverse engineering findings with evidence.

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

## Architecture Note

All wire formats are derived from `.def` files + `alias.xml` type definitions. This is possible because ALL entity method calls route through a single **universal RPC dispatcher** at `0x00c6fc40` in the client binary. The dispatcher serializes arguments using BigWorld's `DataType::addToStream` virtual methods — the wire encoding is determined entirely by the type system, not per-method handler code.

See `combat-wire-formats.md` for the full decompilation evidence and BigWorld DataType encoding reference table.
