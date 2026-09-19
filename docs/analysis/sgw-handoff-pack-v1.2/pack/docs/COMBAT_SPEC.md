# Combat System Specification — Current Reconstruction State

## Source-backed behavioral skeleton

The surviving tutorial and Cooked data support this flow:

1. Validate active weapon, ability requirements, ammo/resource and cooldown.
2. Validate target/range/geometry.
3. Determine directional cover: None / Low / Medium / High.
4. Apply crouch as a separate defensive state.
5. Resolve Accuracy vs Defense with cover/crouch modifiers.
6. Resolve hit/miss.
7. Execute linked effect records.
8. Resolve damage type, armor/resistance/penetration.
9. Apply Focus/Health behavior.
10. Apply states such as Suppression, Snare, Fear, Knockdown, Disarm, Interrupt, DoT/HoT.
11. Consume ammo/resource.
12. Update threat/AI/combat state.

The tutorial explicitly establishes:
- right-click auto-attack;
- every weapon has a basic auto-attack;
- finite ammo and reload;
- some abilities require a specific weapon;
- directional cover indicator;
- High cover = green, Medium = yellow, Low = orange;
- crouching gives a defensive benefit;
- when Focus is depleted, Health is hit more often and accuracy decreases.

## Known ability modifiers

- Ability 1450 `Cover Penetration`: +100 Cover Penetration
- 1451 `Cover Stance`: +200 Cover Defense
- 1452 `Duck and Cover`: +100 Crouching Defense
- 1454 `Hunker Down`: +100 Cover Defense
- 1458 `Stance: Ranged Specialist`: +100 Ranged Accuracy

These values establish stats/modifiers, but **do not establish the full hit formula**.

## Damage payload rule

Where a tooltip and linked Effect disagree, retain both values and flag the conflict. The linked Effect is the current runtime-payload candidate unless runtime/script evidence proves otherwise.

## Unresolved — do not guess as original

- Accuracy vs Defense formula
- Low/Medium/High cover numeric bonuses
- Cover Defense vs Cover Penetration formula
- crouch stacking order
- armor/resistance/penetration formula
- Focus threshold and exact Health/accuracy behavior
- TechComp/Tier/Quality/Applied Science scaling
- AoE vs cover/LoS
- level/character scaling
- stacking order/caps

Implement these as configuration points first. A future project balance formula may fill them, but must be labeled reconstruction.
