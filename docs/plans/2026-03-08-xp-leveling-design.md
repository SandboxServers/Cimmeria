# XP & Leveling System Design

> Date: 2026-03-08
> Status: Approved

## Overview

Implement the XP accumulation, level-up, stat scaling, training point grants, and kill XP pipeline. This fills the critical path gap between combat and character progression — currently a level-1 player and a level-20 player are mechanically identical.

## Scope

**In scope:**
- XP table ported from Python reference (cumulative thresholds, MAX_LEVEL 20)
- Level-up logic with multi-level-up support
- Stat scaling on level-up (health/focus per level from archetype DB data)
- Training point grants (2 per level)
- Kill XP pipeline (mob death → XP grant to attacker)
- All client wire messages (onExpUpdate, giveXPForLevel, onMaxExpUpdate, onStatUpdate, onEntityProperty for TP)

**Out of scope:**
- Primary attribute scaling per level (health/focus only)
- PvP kill XP
- XP from minigames or crafting
- Ability respec
- Applied science points
- XP sharing in groups/parties

## XP Table

Port the Python `Constants.LEVEL_EXP` table as-is (cumulative thresholds):

```
Level:  1     2     3     4     5     6      7      8      9      10
XP:     100   200   300   600   1000  1600   2500   4000   6000   9000

Level:  11     12     13     14     15     16      17      18      19      20
XP:     14000  18000  25000  40000  60000  90000   120000  180000  250000  400000
```

Known anomaly: level 12 threshold (18000) is lower gap-to-gap than level 11 (14000). Ported as-is per decision — can smooth later.

**XP model**: Cumulative. `player.xp` stores total earned XP. Level up when `xp > LEVEL_XP[level]`.

## Level-Up Effects

On each level-up:

1. **Stat scaling**: `max_health = base_health + healthPerLevel * (level - 1)`, same for focus. Values from `archetypes` table (currently 10 hp/level, 70 focus/level for all archetypes). Current health/focus restored to new max (full heal on level-up).

2. **Training points**: Grant 2 points per level. 38 total by level 20.

3. **Client notifications** (matching Python wire messages):
   - `onExpUpdate(total_xp)` — XP bar
   - `giveXPForLevel(new_level)` — level-up VFX/sound to self + witnesses
   - `onMaxExpUpdate(next_level_threshold)` — XP bar cap
   - `onStatUpdate(stat_data)` — updated health/focus max
   - `onEntityProperty(GENERICPROPERTY_TrainingPoints, points)` — TP UI

## Kill XP

**Formula**: `xp = 10 * mob_level`. Level-1 mob gives 10 XP, level-10 gives 100. Simple, tunable.

**No XP from player kills** (PvP XP is a separate system).

**Hook point**: `handle_use_ability()` in `crates/services/src/cell/abilities.rs`, after the `target_died` check (line ~137). Only grants XP when the target is a mob (not a player).

## Data Flow

```
Player uses ability → target dies → CellApp detects death
  → CellApp computes kill XP from mob level
  → CellApp sends CellToBaseMsg::GrantXP { player_entity_id, xp_amount }
  → BaseApp receives, calls player.grant_xp(amount)
  → While xp > threshold: level up, scale stats, grant 2 TP
  → BaseApp sends client messages per level-up
```

## Code Changes

| Component | Location | Changes |
|---|---|---|
| XP table + level-up logic | `crates/game/src/player.rs` | Replace quadratic formula with Python table, add stat scaling + TP in `level_up()` |
| Stat scaling | `crates/entity/src/stats.rs` | Add `scale_for_level()` to `StatList` using archetype healthPerLevel/focusPerLevel |
| Kill XP grant | `crates/services/src/cell/abilities.rs` | After `target_died`, compute XP, send `GrantXP` to BaseApp |
| XP wire messages | `crates/services/src/base/` | Handle `GrantXP`, call level-up, send 5 client messages |
| Mob level tracking | `crates/services/src/cell/spawner.rs` | Add `level` field to spawn definitions |
| New message variant | `crates/services/src/cell/mod.rs` or equivalent | Add `CellToBaseMsg::GrantXP { entity_id, amount }` |
| XP persistence | `crates/services/src/database.rs` | Persist updated XP/level/TP on save |

## Dependencies

- Existing combat system (damage calculation, death detection) — working
- Existing stat system (StatList, dirty tracking, serialization) — working
- Existing CellToBase message channel — working
- Archetype table in DB (healthPerLevel, focusPerLevel) — data exists, never used
