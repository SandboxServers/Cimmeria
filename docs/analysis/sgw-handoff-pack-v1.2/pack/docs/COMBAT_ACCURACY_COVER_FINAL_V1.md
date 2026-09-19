# SGW Combat — Accuracy / Defense / Cover Final v1

**Status:** PROJECT FINAL v1.  
This is a playable reconstruction, not a claim that the exact retail FireSky formula has been recovered.

## Formula

```text
effectiveAccuracy =
    baseAccuracy
  + abilityAccuracyModifier
  + signedAccuracyModifiers

rawCoverDefense =
    coverTierDefense
  + signedCoverDefenseModifiers

coverAfterPenetration =
  max(0, rawCoverDefense - coverPenetration)

effectiveDefense =
    baseDefense
  + signedDefenseModifiers
  + crouchDefense
  + coverAfterPenetration

rawHitChance =
  0.75 + (effectiveAccuracy - effectiveDefense) / 1000

finalHitChance =
  clamp(0.10, 0.95, rawHitChance)
```

## Project-v1 values

| State | Defense |
|---|---:|
| No cover | 0 |
| Low cover | +100 |
| Medium cover | +200 |
| High cover | +300 |
| Crouch | +100 |

`+100` rating therefore changes hit chance by **10 percentage points** before clamping.

At equal Accuracy and Defense:
- open: 75%
- Low cover: 65%
- Medium cover: 55%
- High cover: 45%
- High + crouch: 35%

## Cover Penetration

Cover Penetration reduces **only the cover component**:

```text
coverAfterPenetration = max(0, coverDefense - coverPenetration)
```

It cannot make cover negative and does not bypass ordinary Defense or crouch Defense.

## Damage

Cover is a **hit-resolution mechanic** in v1, not direct damage mitigation.

If an attack hits, continue through:
`Effect payload → Armor/Resistance/Penetration → Focus/Health → statuses`.

## Direction

The server must resolve cover relative to the incoming attack direction. A defender flanked outside the protected arc receives `CoverTier=None` for that attack.

`120°` is only a project fallback arc if the current map/runtime cover implementation provides no native arc result.

## Source conflicts to preserve

- Ability 1451 `Cover Stance`: tooltip says +200 Cover Defense; linked effect says +100.
- Ability 1452 `Duck and Cover`: tooltip says +100 Crouching Defense; linked effect says +200 Cover Defense.

Do not silently rewrite these source records to make them fit this formula.

## Still unresolved

- AoE / grenade / mortar interaction with cover and line of sight
- original retail Accuracy-vs-Defense formula
- original Low/Medium/High numeric cover bonuses
- original crouch numeric bonus
- armor/resistance/penetration
- Focus-based accuracy/Health behavior

All should remain configurable.
