# Stat Scaling & XP Progression — Analysis

> **Last updated**: 2026-03-08
> **Source**: SGW.exe Ghidra decompilation + alias.xml + .def files
> **Confidence**: HIGH — formulas from designer comments in alias.xml, validated against binary

---

## Key Finding: No XP Tables or Stat Curves in Client

All XP tables, stat scaling per level, and derivation formulas are **server-side only**. The client receives pre-computed values via network events. However, the **stat derivation formulas** are documented in alias.xml XML comments.

---

## XP System

### Architecture

- `experience` — INT32, CELL_PRIVATE. Current XP **into this level** (resets on level-up)
- `maxExperience` — XP threshold to reach next level (pushed via `onMaxExpUpdate`)
- `level` — INT8, CELL_PUBLIC, default 1. Max possible 127 (practical cap: 50)

### Network Events

| Event | Direction | Type | Description |
|-------|-----------|------|-------------|
| `onExpUpdate` | Server→Client | INT32 | Current XP |
| `onMaxExpUpdate` | Server→Client | INT32 | XP needed for level |
| `onLevelUpdate` | Server→Client | INT32 | New level |

### Server Methods for XP

| Method | Args | Description |
|--------|------|-------------|
| `giveXPForLevel` | INT32 level | GM: grant XP equivalent to a level |
| `awardSquadXP` | mobDefId, mobPos, mobLevel(INT32), xpAward(INT32) | Squad XP distribution |
| `onKillCredit` | specId, dbId, xpAward(INT32) | Kill XP with pre-calculated amount |

### Client Binary

- `getExperience()` at `0x00ad8fa0`: reads `playerProxy->0x8C->0xB4`
- `getMaxExperience()` at `0x00ad8fc0`: reads `playerProxy->0x8C->0xB8`

### Level Properties (SGWPlayer.def)

| Property | Type | Description |
|----------|------|-------------|
| `gainLevelLock` | INT8 | Prevents leveling during operations |
| `timeLastLevelled` | FLOAT | BigWorld timestamp of last level-up |
| `timeSpentThisLevel` | FLOAT | Time at current level |

---

## Stat Storage Architecture (SGWCombatant.def)

6 parallel `StatList` dictionaries:

| Dictionary | Description |
|------------|-------------|
| `statsBaseMin` | Base minimum (before buffs) |
| `statsBaseCurrent` | Base current (before buffs) |
| `statsBaseMax` | Base maximum (before buffs) |
| `statsMin` | Effective minimum (after buffs) |
| `statsCurrent` | Effective current (after buffs) |
| `statsMax` | Effective maximum (after buffs) |

### adjustStat Server Method

```
adjustStat(statName:STRING, damageType:INT32,
           modCurrent:INT32, modMin:INT32, modMax:INT32,
           modBaseCurrent:INT32, modBaseMin:INT32, modBaseMax:INT32,
           mitigation:FLOAT, runtimeDict:PYTHON)
```

### Client Stat Structure (`getUnitStat` at `0x00aebf80`)

Returns 7 fields from GameBeing's std::map at +0x160:
`{id, current, max, min, baseCurrent, baseMax, baseMin}`

---

## Complete Stat List with Formulas (from alias.xml comments)

### Primary Attributes (6)

| Stat | Effects |
|------|---------|
| `coordination` | +0.05 ranged attack QR/pt, +0.1% interrupt resistance/pt |
| `engagement` | +0.05 melee attack QR/pt, +1% kinetic resistance/pt |
| `fortitude` | +10 health/pt, +1% health resistance/pt |
| `intelligence` | +0.5% Super Trump Card chance (Converse minigame)/pt, +1% mental resistance/pt |
| `morale` | +100 focus/pt, +0.5% focus regen rate/pt |
| `perception` | -0.05 defense QR/pt, +0.5% stealth/disguise/reveal checks/pt |

### Vital Stats

| Stat | Effect |
|------|--------|
| `health` | +1 health |
| `focus` | +1 focus |
| `energy` | +1 energy |
| `healthRegen` | +1% health regen rate |
| `focusRegen` | +1% focus regen rate |
| `energyRegen` | +1% energy regen rate |

### Combat Stats (QR System)

| Stat | Effect |
|------|--------|
| `accuracy` | +0.01 outgoing ranged/melee QR/pt |
| `defense` | -0.01 incoming ranged/melee QR/pt |
| `qrMod` | +1 both attacking and defending QR/pt |
| `awareness` | +0.01 outgoing ranged/melee QR/pt |
| `tracking` | -0.01 QR shift from defensive movement/pt |
| `stabilization` | -0.01 QR shift from attacker movement/pt |
| `coverAccuracy` | +0.01 accuracy against targets in cover/pt |
| `coverDefense` | -0.01 defense while in cover/pt |
| `crouchingAccuracy` | +0.01 accuracy while crouching/pt |
| `crouchingDefense` | -0.01 defense while crouching/pt |
| `coverQRModifier` | +1 attack and defend QR behind cover/pt |

### Armor / Resistance

| Stat | Effect |
|------|--------|
| `physicalAF` | Physical armor factor |
| `energyAF` | Energy armor factor |
| `hazmatAF` | Hazmat armor factor |
| `psionicAF` | Psionic armor factor |
| `kineticRes` | +1% kinetic effect resistance (additive) |
| `mentalRes` | +1% mental effect resistance (additive) |
| `healthRes` | +1% health effect resistance (additive) |
| `mitigation` | Armor mitigation % (0-100) |
| `interruptRes` | +1% interrupt resistance |

### Damage Modifiers

| Stat | Effect |
|------|--------|
| `damage` | +1% damage done (all attacks/effects) |
| `penetration` | -1% target's final mitigation |
| `negation` | -1% chance of full resist (subtractive) |
| `recovery` | +1% healing done via abilities |
| `restoration` | +1% healing received from effects |

### Density Stats (Flat AF Bonus)

| Stat | Effect |
|------|--------|
| `physicalDensity` | +1 AF vs physical |
| `energyDensity` | +1 AF vs energy |
| `hazmatDensity` | +1 AF vs hazmat |
| `psionicDensity` | +1 AF vs psionic |

### Absorb Stats (Damage Shields)

Direct: `absorbPhysical`, `absorbEnergy`, `absorbHazmat`, `absorbPsionic`, `absorbUntyped`
Item-charged: `absorbPhysicalItem`, `absorbEnergyItem`, `absorbHazmatItem`, `absorbPsionicItem`, `absorbUntypedItem`
Energy-charged: `absorbPhysicalEnergy`, `absorbEnergyEnergy`, `absorbHazmatEnergy`, `absorbPsionicEnergy`, `absorbUntypedEnergy`

### Movement / Speed

| Stat | Effect |
|------|--------|
| `movementSpeedMod` | Multiply speed by curr/100 |
| `rotationSpeedMod` | Multiply rotation by curr/100 |
| `speedReload` | -1% reload time |
| `speedGrenade` | -1% grenade warmup |
| `speedDeploy` | -1% deployable warmup |
| `speedAttack` | -1% attack warmup |
| `speedPet` | -1% pet ability warmup |
| `response` | -1% response ability reuse time |

### Stealth / Disguise

| Stat | Effect |
|------|--------|
| `stealthRating` | +1% stealth rating (additive) |
| `stealthMovement` | +1% stealth movement speed |
| `revealRating` | +1 reveal rating |
| `disguiseRating` | Reaches 0 → leaves disguise |
| `disguiseDetection` | Ticks down target's disguise |

### Misc

| Stat | Effect |
|------|--------|
| `rangeModifier` | +1% range |
| `subtlety` | -0.1% threat generated |
| `ammoSlot1`–`ammoSlot5` | Ammo in bandolier slots |
| `deploymentBarAmmo` | Deployment bar ammo |

---

## Derived Stat Formulas (Server Implementation)

| Derived Stat | Formula |
|-------------|---------|
| Max Health | base + (fortitude × 10) |
| Max Focus | base + (morale × 100) |
| Focus Regen | base × (1 + morale × 0.005) |
| Ranged Attack QR | base + (coordination × 0.05) + (accuracy × 0.01) + (awareness × 0.01) |
| Melee Attack QR | base + (engagement × 0.05) + (accuracy × 0.01) + (awareness × 0.01) |
| Defense QR | base − (perception × 0.05) − (defense × 0.01) |
| Interrupt Resist | base + (coordination × 0.1%) + (interruptRes × 1%) |
| Kinetic Resist | base + (engagement × 1%) + (kineticRes × 1%) |
| Mental Resist | base + (intelligence × 1%) + (mentalRes × 1%) |
| Health Resist | base + (fortitude × 1%) + (healthRes × 1%) |
| Movement Speed | base × (movementSpeedMod ÷ 100) |
| Damage Multiplier | 1 + (damage × 0.01) |
| Effective Mitigation | mitigation% − penetration% (clamped 0-100) |
| Threat Multiplier | 1 − (subtlety × 0.001) |

---

## Game Enumerations (from enumerations.xml)

### EArchetype
| Value | Name |
|-------|------|
| 0 | Any |
| 1 | Soldier |
| 2 | Commando |
| 3 | Scientist |
| 4 | Archeologist |
| 5 | Asgard |
| 6 | Goauld |
| 7 | Sholva |
| 8 | Jaffa |

### EDamageType
| Value | Name |
|-------|------|
| 13 | Untyped |
| 14 | Physical |
| 15 | Energy |
| 16 | Hazmat |
| 18 | Psionic |

### EResultCode
| Value | Name |
|-------|------|
| 0 | None |
| 1 | Hit |
| 2 | Miss |
| 3 | Critical |
| 4 | DoubleCritical |
| 5 | Glancing |

### EStateField (bStateField bitmask)
| Bit | Name |
|-----|------|
| 0 | Dead |
| 1 | AutoCycling |
| 2 | Crouching |
| 3 | InCombat |
| 4 | PlayingMinigame |
| 5 | InStealth |
| 6 | MovementLock |
| 7 | Walking |
| 8 | Holster |

---

## What Cimmeria Still Needs (Server-Only)

1. **XP table** — level→threshold mapping. Must be reconstructed from game data or estimated.
2. **Per-level stat scaling** — how base stats increase per level per archetype. Server-only.
3. **Stat derivation implementation** — use the formulas above when calculating effective stats.
4. **XP award calculation** — based on mob level, player level, squad composition.
