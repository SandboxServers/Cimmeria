# Stat System

> **Last updated**: 2026-03-01
> **Status**: ~50% implemented

## Overview

The stat system provides 60+ named statistics per combatant entity, organized into a 6-tier dictionary structure. Each stat has six values: base min/current/max and dynamic min/current/max. The dynamic values represent the "effective" stat after buffs/debuffs, while base values represent the entity's innate capability.

Stats are implemented via the `Stat` class in `python/cell/SGWBeing.py`.

## Implementation Status

| Feature | Status | Notes |
|---------|--------|-------|
| 6-tier stat dictionary | DONE | statsBaseMin/Cur/Max, statsMin/Cur/Max |
| Stat creation from template | DONE | `statsTemplate` with 60+ stats |
| Stat mutation (absolute, %) | DONE | `change()`, `changeByPercent()`, etc. |
| Min/max clamping | DONE | Stat.setCurrent() clamps to [min, max] |
| Dirty flag tracking | DONE | Per-stat `dirty` / `baseDirty` flags |
| Health death callback | DONE | `onHealthChanged()` triggers death at 0 |
| Client stat sync (own entity) | DONE | All stats sent to owning client |
| Witness stat sync (public only) | DONE | `publicStats` subset sent to witnesses |
| Stat change callbacks | DONE | `callback` slot on Stat for health monitoring |
| Health/focus regen | NOT IMPL | `healthRegen`, `focusRegen` exist but no regen loop |
| Level-based stat scaling | NOT IMPL | Stats are template-initialized only |
| Derived stat calculation | NOT IMPL | No formula for stat interdependencies |
| Stat caps per level | NOT IMPL | Only template min/max bounds |
| Item stat bonuses | PARTIAL | `inventoryAdjustments` property exists |

## Stat Class (python/cell/SGWBeing.py)

```python
class Stat:
    __slots__ = ('min', 'cur', 'max', 'baseMin', 'baseCur', 'baseMax', 'dirty', 'baseDirty', 'callback')
```

### Methods

| Method | Args | Returns | Description |
|--------|------|---------|-------------|
| `setCurrent(value)` | absolute value | clamped value | Set current, clamp to [min, max] |
| `change(delta)` | signed delta | actual change | Add/sub from current, clamp |
| `changeByPercent(mult)` | % of current | actual change | Relative to current value |
| `changeByMaxPercent(mult)` | % of max | actual change | Relative to max value |
| `changeByMinMaxPercent(mult)` | % of range | actual change | Relative to (max - min) |
| `setMin(value)` | new minimum | -- | Update min, clamp current |
| `setMax(value)` | new maximum | -- | Update max, clamp current |
| `update(min, cur, max)` | all values | -- | Bulk update |
| `clone()` | -- | new Stat | Deep copy for template instantiation |

## Stat Enumeration (60+ stats)

### Primary Attributes

| Stat | Enum | Default | Purpose |
|------|------|---------|---------|
| `coordination` | `Atrea.enums.coordination` | 0/1/1 | Ranged QR bonus |
| `engagement` | `Atrea.enums.engagement` | 0/1/1 | Melee QR bonus, kinetic resistance |
| `fortitude` | `Atrea.enums.fortitude` | 0/1/1 | Health resistance |
| `morale` | `Atrea.enums.morale` | 0/1/1 | Unused (regeneration?) |
| `perception` | `Atrea.enums.perception` | 0/1/1 | QR defense, stealth detection |
| `intelligence` | `Atrea.enums.intelligence` | 0/1/1 | Focus resistance, minigames |

### Vitals

| Stat | Default Range | Purpose |
|------|---------------|---------|
| `health` | 0/100/100 | Hit points |
| `focus` | 0/0/0 | Mental resource (mana equivalent) |
| `energy` | 0/0/0 | Energy resource |
| `healthRegen` | 0/0/0 | HP regen per tick |
| `focusRegen` | 0/0/0 | Focus regen per tick |
| `energyRegen` | 0/0/0 | Energy regen per tick |

### Combat

| Stat | Purpose |
|------|---------|
| `accuracy` | Hit chance modifier (-1000 to 1000) |
| `defense` | Dodge/defense modifier |
| `qrMod` | Direct QR modifier |
| `damage` | Damage multiplier % (-100 to 100) |
| `penetration` | Armor penetration (-100 to 100) |
| `mitigation` | Damage mitigation |
| `awareness` | QR bonus from awareness |

### Armor Factor (by damage type)

| Stat | Max | Purpose |
|------|-----|---------|
| `physicalAF` | 50000 | Physical armor factor |
| `energyAF` | 50000 | Energy armor factor |
| `hazmatAF` | 50000 | Hazmat armor factor |
| `psionicAF` | 50000 | Psionic armor factor |
| `physicalDensity` .. `psionicDensity` | -- | Armor density per type |

### Absorption (by damage type)

| Stat Pattern | Max | Purpose |
|-------------|-----|---------|
| `absorb{Type}` | 1000 | Base absorption per damage type |
| `absorb{Type}Energy` | 1000 | Energy-sourced absorption |
| `absorb{Type}Item` | 1000 | Item-sourced absorption |

Types: Physical, Energy, Hazmat, Psionic, Untyped (5 types x 3 sources = 15 stats)

### Resistance

| Stat | Max | Purpose |
|------|-----|---------|
| `kineticRes` | 2000 | Energy stat resistance |
| `mentalRes` | 2000 | Focus stat resistance |
| `healthRes` | 2000 | Health stat resistance |
| `interruptRes` | -- | Interrupt resistance |

### Speed Modifiers

| Stat | Default | Purpose |
|------|---------|---------|
| `movementSpeedMod` | 100 | Movement speed % (100 = normal) |
| `rotationSpeedMod` | 100 | Rotation speed % |
| `speedReload` | 0 | Reload speed modifier |
| `speedGrenade` | 0 | Grenade warmup reduction % |
| `speedDeploy` | 0 | Deploy warmup reduction % |
| `speedAttack` | 0 | Attack warmup reduction % |
| `speedPet` | 0 | Pet-related speed |

### Other

| Stat | Purpose |
|------|---------|
| `ammoSlot1` .. `ammoSlot5` | Ammo counts per slot |
| `deploymentBarAmmo` | Deployment bar ammo |
| `response` | Response ability cooldown reduction % |
| `stealthRating` / `stealthMovement` | Stealth system |
| `revealRating` / `disguiseRating` / `disguiseDetection` | Disguise system |
| `coverQRModifier` / `coverAccuracy` / `coverDefense` | Cover system |
| `crouchingAccuracy` / `crouchingDefense` | Crouch modifiers |
| `tracking` / `stabilization` | Weapon tracking |
| `negation` / `recovery` / `restoration` / `subtlety` | Unused |
| `{Type}DamagePercent` | Damage % bonus per type (5 stats) |

## Public Stats (sent to witnesses)

Only these stats are sent to other players (via `sendStats()` with the public filter):

```python
publicStats = [
    movementSpeedMod, health, focus,
    ammoSlot1-5, rotationSpeedMod,
    energy, energyRegen
]
```

## Wire Format

### onStatUpdate / onStatBaseUpdate

```
StatUpdateList: ARRAY of
  { StatId: INT32, Min: INT32, Current: INT32, Max: INT32 }
```

## Data References

- **Stat enumerations**: `Atrea.enums` module (compiled from enumerations.xml)
- **Stat templates**: `SGWBeing.statsTemplate` dict (60+ entries)
- **Level scaling**: TODO - need to decompile level-up stat curves from client

## RE Priorities

1. **Level scaling curves** - How stats change per level (decompile character creation / level-up code)
2. **Regen formulas** - How `healthRegen` / `focusRegen` translate to actual regen ticks
3. **Derived stats** - Which stats are computed from others (e.g., AF from items)
4. **Stat caps** - Per-level caps for primary attributes and combat stats
5. **Item stat integration** - How equipped items modify the stat dictionary

## Related Docs

- [combat-system.md](combat-system.md) - Stats used in damage calculation
- [effect-system.md](effect-system.md) - Effects that modify stats
- [inventory-system.md](inventory-system.md) - Items that provide stat bonuses
