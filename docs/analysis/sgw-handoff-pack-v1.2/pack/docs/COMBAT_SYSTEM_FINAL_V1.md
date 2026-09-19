# SGW Combat System Final v1

**Status: PROJECT FINAL v1**

This is the complete playable combat reconstruction currently intended for server implementation. It preserves recovered Stargate Worlds ability/effect payloads and clearly separates them from reconstructed formulas.

## Resolution order

1. Validate actor, target, weapon/program, learned ability, cooldown and ammo/resource.
2. Resolve target/range/geometry/incoming direction.
3. Resolve direct/cone/melee cover and crouch.
4. Resolve Accuracy vs Defense where a hit roll applies.
5. Explosion/ground AoE resolves per-target LoS and blast exposure.
6. Load linked Effect payload.
7. Apply item-bound TechComp/quality/ammo scaling to numeric payload.
8. Apply armor mitigation and penetration.
9. Apply explicit damage resistance/vulnerability.
10. Apply Focus component.
11. Resolve Focus-dependent ordinary Health component.
12. Resolve statuses and explicit resist rolls.
13. Consume ammo/Energy/charges.
14. Update threat, AI and combat state.

## Accuracy / Defense / Cover

```text
effectiveAccuracy =
    baseAccuracy
  + abilityAccuracyModifier
  + signedAccuracyModifiers
  + focusAccuracyPenalty

coverAfterPen =
  max(0, coverTierDefense + coverDefenseBuffs - coverPenetration)

effectiveDefense =
    baseDefense
  + signedDefenseModifiers
  + crouchDefense
  + coverAfterPen

hitChance =
  clamp(10%, 95%,
        75% + (effectiveAccuracy - effectiveDefense) / 1000)
```

Cover Defense:
- None +0
- Low +100
- Medium +200
- High +300
- Crouch +100 separately

Direct cover affects hit chance, not post-hit damage.

## Armor / Resistance / Penetration

Project coverage weights:
- Head 15%
- Torso 35%
- Hands 10%
- Legs 25%
- Feet 15%

A full set of matching armor pieces that each say `30% Mitigation` therefore results in **30% total mitigation**, not 150%.

```text
rawArmor =
  Σ(slotWeight × explicitPieceMitigationPct)

penetrationFraction =
  clamp(-30%, +50%, penetrationRating / 1000)

armorAfterPen =
  clamp(0%, 60%, rawArmor - penetrationFraction)

resistance =
  clamp(-50%, +60%, sum(explicit matching resistance %))

finalDamage =
  scaledPayload
  × (1 - armorAfterPen)
  × (1 - resistance)
```

Project channel mapping:
- Ballistic → Physical / Kinetic
- Ablative → Energy / Plasma / Fire / Ice
- Contamination → Contamination / Radiation / Poison / Disease
- Stealth → only explicit generic mitigation
- unknown/Sonic/Mental → no generic armor family until explicit evidence exists

Penetration reduces armor mitigation only. Status resist ratings are a separate system.

## Focus / Health

The surviving tutorial says that as Focus depletes, **Health damage happens more often** and **Accuracy goes down**.

Project v1 interprets this literally:

```text
focusRatio = currentFocus / maxFocus

focusAccuracyPenalty =
  -200 × (1 - focusRatio)

healthComponentChance =
  10% + 80% × (1 - focusRatioAfterFocusDamage)
```

So:
- 100% Focus: 0 Accuracy penalty, 10% ordinary H-component chance
- 50% Focus: -100 Accuracy, 50% H-component chance
- 0% Focus: -200 Accuracy, 90% H-component chance

For paired `F/H` attacks:
1. mitigate/scale both payloads;
2. apply Focus damage;
3. recompute Focus ratio;
4. roll whether the ordinary Health component applies.

Health-only effects always apply directly. Focus-only effects do not invent Health damage.

## Status resistance

When an effect explicitly contains a resist roll:

```text
statusApplyChance =
  clamp(10%, 95%,
        85% + (effectPower - targetResistRating) / 1000)
```

Default `effectPower = 0` unless recovered data provides a modifier.

Use the linked effect's explicit resist type:
- Mental Resist Roll
- Kinetic Resist Roll
- Health Resist Roll

Immunity monikers override the formula with 0% application chance.

## AoE / Cover / Line of Sight

Explosion exposure:
- None: 100%
- Low: 85%
- Medium: 70%
- High: 50%
- hard world occlusion: 0%

Use the **explosion origin** as the incoming direction for cover.

Damage:
```text
aoeDamage = normalMitigatedDamage × blastExposure
```

Status:
```text
aoeStatusChance = statusApplyChance × blastExposure
```

Cone attacks are not explosion AoE; resolve each target using normal hit/cover rules from the attacker.

Persistent ground hazards ignore cover once the target is inside the hazard volume.

## TechComp / Quality scaling

Recovered item range: **TC1–TC55**.

```text
tcMultiplier =
  1 + (clamp(TC,1,55)-1) × 0.0125
```

Quality:
- Poor 0.95
- Normal 1.00
- Good 1.03
- Great 1.06
- Fantastic 1.10

```text
itemPayloadMultiplier =
  tcMultiplier
  × qualityMultiplier
  × explicitItemPayloadModifier
```

Examples:
- TC1 Normal = 1.000
- TC25 Normal = 1.300
- TC55 Normal = 1.675
- TC55 Fantastic = 1.8425

No generic Tier multiplier, Applied Science multiplier or character-level damage multiplier in v1.

## Ammo fallback values

Only two recovered descriptions require a numeric fallback for v1:

- Hollow Point: ×1.15 damage, -100 Penetration
- Armor Piercing: ×0.90 damage, +150 Penetration

Incendiary, EMP, Explosive and dart variants remain **effect-driven**. Do not invent missing generic payloads.

## Stacking

- different flat-rating monikers sum;
- same flat-rating moniker: strongest magnitude, duration refresh;
- different percentage monikers sum then stat-cap;
- same percentage moniker does not stack;
- one stance per stance family;
- one ammo mode per weapon;
- same DoT/HoT moniker refreshes/replaces;
- different DoT/HoT monikers may coexist;
- same CC state uses max(current remaining,new duration), never additive duration.

Caps:
- Armor 60%
- Damage Resistance -50% to +60%
- Movement 0.40× to 1.50×
- Hit chance 10–95%
- Status chance 10–95%
- Focus Accuracy penalty 0 to -200

## What remains unknown

The exact original retail formulas are still not recovered. This v1 is deliberately configurable so that any future authentic formula can replace the reconstruction without changing raw ability/effect data.
