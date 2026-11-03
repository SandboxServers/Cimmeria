# debug-armory: seed-gap plan

> **Type:** reference. **Audience:** the orchestrator, the owner and the DR-S packet agents. **Updated:** 2026-11-02
> **Companions:** [ledger](README.md) (F9, D-DR10 to D-DR17), [work packets](work-packets.md#wave-s-seed-gaps), lab issue notes under [results/issues/](results/issues/).

The Armory lab runs DR-L2 (Soldier), DR-L3 (Commando) and DR-L4 (Scientist) left two kinds of gap. Abilities the server refuses with `ability_refused no_mechanics` are **seed gaps**. Abilities whose weapon moniker no item carries are **data gaps**. This plan classifies each gap under the owner's rule. Only the EVIDENCED fixes become packets. Everything else is an owner decision, and the ability keeps refusing until the owner rules.

## How the server decides "no mechanics"

- An effect counts as a mechanic only if it has `HealthDamage > 0`, `FocusDamage > 0`, or a registered script: `effect_has_mechanic`, `crates/entity/src/abilities/implemented.rs:32-43`.
- At the ability level, `ability_has_mechanics` (`crates/cell-combat/src/cell/abilities/use_ability/no_mechanics.rs`) also accepts ammo, cover, reload, pet summons, deployables and ammo toggles.
- A damage NVP row needs no `script_name`. The generated Aimed Shot siblings 1401 and 916 carry no script and still work.

## Sources checked, and what they hold

- **Client data.** The client `SourceCache.en-us` paks (`CookedDataAbilities.pak`, `CookedDataEffects.pak`, `CookedDataItems.pak`) are byte-identical to the repo's `data/cache/`.
  - Client effect XML has **no name-value pairs**, so an effect's designer text (`effect_desc`) is the only primary source for a number.
  - Every client `EffectIds` list matches the seed's, so no ability is missing an effect link.
- **Seed** (`db/resources/`). It is Project Giza's reconstruction from those paks: the `effect_desc` and `abilities.description` texts are the client's strings.
- **Docs and memory.** Checked:
  - [ability-mechanics README](../ability-mechanics/README.md): D-AB03 and D-AB04 at lines 65-66, D-AB09 at 71, D-AB11 at 73, D-AB12 at 74.
  - [deployables README](../deployables/README.md): phase table at lines 78-86.
  - [weapon-requirement-audit.md](../class-start-v6/weapon-requirement-audit.md): lines 56-59.
  - [class-start-v6 OD-CS16](../class-start-v6/README.md): line 76.
  - The `effect_nvps.sql` precedent at lines 70-71: "the ability tooltip says 15%; the effect row is what executes".
  - `.claude/agent-memory/`: no other source with numbers.

## Class legend

| Class | Meaning | What happens |
|---|---|---|
| **EVIDENCED** | Every value and link comes from a primary source, cited by file and line. | A DR-S packet, unless a missing server mechanism blocks it. |
| **INFERRED** | Derived by analogy, pattern or a balance guess. Each row says what is inferred and from what. | An owner decision. The ability keeps refusing. |
| **UNKNOWN** | No numbers or no mechanism anywhere. | An owner decision. The ability keeps refusing. |

Counts: **3 EVIDENCED** (1 fixable now, 2 blocked on server code), **24 INFERRED** (20 abilities, 3 data-gap abilities and item 3128), **46 UNKNOWN**. Each ability is counted once: 2864 shows up in both the Soldier and Commando runs. The Archeologist run, [DR-L5-archeologist-0-no-mechanics.md](results/issues/DR-L5-archeologist-0-no-mechanics.md) (746, 781, 784, 791, 1081 to 1083, 1183, 1187, 1188, 1190 to 1192, 2074), is **outside this plan**. Classify it the same way once its run is graded.

## EVIDENCED

| Ability | Name | Lab issue | Gap | Evidence | Fixable now? | Packet |
|---|---|---|---|---|---|---|
| 640 | Aimed Shot: Torso | [DR-L3-commando-2](results/issues/DR-L3-commando-2-no-mechanics.md) | Its only effect, 703 "Direct Damage", has no NVPs. The damage grammar rejects its text, which is listed in `tools/ability_mechanics/reports/damage.txt:258`. | Effect text `effects.sql:7386`: "Single Target / Target -800F / 80H". Rank siblings: 1401 Arm (`effects.sql:2696`, "-500F / -50H", generated rows `effect_nvps.sql:426-427`) and 916 Leg (`effects.sql:7654`, same text, rows `effect_nvps.sql:344-345`). | **Yes** | **DR-S1** |
| 858 | Leadership | [DR-L2-soldier-2](results/issues/DR-L2-soldier-2-no-mechanics.md) | Effect 921 has no NVPs. | `effects.sql:2435` and the tooltip `abilities.sql:1369` agree: +50% Focus Regeneration for 20 s. | **No**: regen is a percentage of a base rate, and that model (AB-05, D-AB03 NEEDS OWNER, D-AB04 PROPOSED) is not built. | none (D-DR17) |
| 860 | Marked Prey | [DR-L3-commando-1](results/issues/DR-L3-commando-1-no-mechanics.md) | Effect 923 has no NVPs. | `effects.sql:7670` "-200 Defense: 30 Seconds (1 hit)" with `EF_ClearOnDamage`. The tooltip `abilities.sql:4931` gives the same -200. A `Defense` NVP has precedent at `effect_nvps.sql` row 351. | **No**: clear-on-damage (AB-11) has only the `StatBuffRemoval::Damage` variant (`crates/cell-world/src/cell/effects/stat_buff/ledger.rs:49`) and nothing calls it. Without that hook the debuff would last its full 30 s, which is wrong. | none (D-DR17) |

**640: the one reading the packet makes.**

- **The sign.** The text says "80H" with no minus. The packet reads it as a missing minus, so `HealthDamage 80`. The reason: both siblings in the same Aimed Shot family, and every generated damage row, write the Health term as "-NH", with Focus at ten times Health (800 : 80 here, 500 : 50 in both siblings). A positive "80H" on a direct-damage shot would mean healing the target.
- **The tooltip.** The ability tooltip at `abilities.sql:339` says "-400F / -40H". The packet uses the effect row, 800 / 80, because the effect row is what executes. The same split holds for both siblings: Arm's tooltip says 300/30 against an effect of 500/50 (`abilities.sql:1991`), and Leg's says 200/20 against 500/50 (`abilities.sql:4924`). The generator already uses the effect text for both (precedent: `effect_nvps.sql:70-71`).
- If the owner wants tooltip values instead, that is a change to the whole family and is outside DR-S1.

858 and 860 land through the generator once AB-05 and AB-11 exist. Until then they keep refusing. D-DR17 asks for their priority.

## INFERRED (owner decisions; the ability keeps refusing)

| Ability | Name | Lab issue | Gap | What would be inferred, and from what | Decision |
|---|---|---|---|---|---|
| 718 | Selective Fire | DR-L2-soldier-0 | Channel effect 778 has no NVPs. | The per-tick numbers are in the text (`effects.sql:1265`), but the tick count is not: the client row has PulseCount 1, so "channelled" can't be turned into pulses without guessing. | D-DR10 |
| 725 | Sustained Fire | DR-L2-soldier-0 | Effect 757 and its wave siblings 758 and 759 have no NVPs. | `effects.sql:7493` says "Channeled: 5 ticks" per wave, but the tooltip (`abilities.sql:5221`) totals 1200 over 50 pulses. The wave count and the "exponential" ramp would be guesses. | D-DR10 |
| 1477 | All Out Assault | DR-L2-soldier-0 | Effect 5268 and its siblings 5267, 5269 and 5270 have no NVPs. | `effects.sql:7274` gives per-target damage, but the cone width and the tick shape (`abilities.sql:1490`) would be guesses. | D-DR10 |
| 656 | Sustained Bursts | DR-L2-soldier-1 | Effect 721 has no NVPs. | `effects.sql:7446` says "Channeled: 50 ticks" of 150, which is 7500 in total. The tooltip (`abilities.sql:1610`) says 1500. Which one is right would be a guess. | D-DR10 |
| 1483 | Steady Launch | DR-L2-soldier-1 | Effect 1779 has no NVPs. | `effects.sql:3004` "+10% Interrupt Resistance: 30 Seconds with missiles". Two things would be inferred: the percent-to-points unit (D-AB09 is PROPOSED only) and the "with missiles" condition, which has no mechanism. | D-DR11 |
| 1019 | Shield: Interruption | DR-L4-scientist | Effect 3085 has no NVPs. | `effects.sql:4335` "+10%" interrupt resistance: the same unit question (D-AB09). Also, the ability is a toggle on a Target ability (`abilities.sql:1783`), so who it applies to is unclear. | D-DR11 |
| 1231 | Shield: Adaptive | DR-L4-scientist | Aura effect 3128 has no NVPs. | `effects.sql:1873` is a `TCM_Aura` collection. Routing an aura is D-AB12 (NEEDS OWNER), and the absorb amount is not in the text (`abilities.sql:1951`). | D-DR11, D-DR14 |
| 1232 | Shield: Reflective | DR-L4-scientist | Its aura effects have no NVPs. | "Redirects a portion of damage" (`abilities.sql:1953`) gives no number. Aura routing is D-AB12. | D-DR11, D-DR14 |
| 1891 | Stance: Rally | DR-L2-soldier-2 | Effects 2430 and 5309 have no NVPs. | The effect says +10% (`effects.sql:3560`) and the tooltip says +15% (`abilities.sql:2494`). It is a group buff (D-AB12) on the unbuilt regen model (AB-05). | D-DR14 |
| 1893 | Stance: Commander | DR-L2-soldier-2-stances | Effect 3524 has no NVPs. | `effects.sql:1967` "Group target: 50% increase to Attack apeed". No stat maps to attack speed, and group routing is D-AB12. | D-DR14 |
| 2864 | EMP Grenade | DR-L2-soldier-2, DR-L3-commando-0 | Effects 4200 to 4206 are unparsed. | Effect 4200 (`effects.sql:5642`) says "-0F/-225H". The tooltip (`abilities.sql:1067`) says "Mechanical -500F/-50H, Other -100F/-0H" plus a 20 s Disorient. Which effect hits which target class would be inferred. | D-DR13 |
| 1259 | Sticky Bomb | DR-L3-commando-0 | Effect 1431 has no NVPs. | It detonates on a 10 s timer (`abilities.sql:117`), which needs a delayed-detonation mechanism. The "-80F" at `effects.sql:2742` breaks the 10 : 1 pattern and may be a typo. | D-DR13 |
| 3358 | Detonate C-4 | DR-L3-commando-0 | No damage NVPs. | The numbers exist only in the tooltip, "per C-4" (`abilities.sql:4405`). It depends on 887's C-4 spawn, which doesn't exist. | D-DR13 |
| 1882 | Launch Grenade: Tear Gas | DR-L2-soldier-1-ground | No effects at all. | The numbers are only in the tooltip (`abilities.sql:524`: -100 Accuracy, -50% Range, Focus DoT -50 for 10 ticks). Fixing it means writing new server-only effect rows (ids, TCM, pulse shape) from a tooltip. | D-DR12 |
| 1329 | Incendiary Grenade | DR-L2-soldier-2 | No effects at all. | Tooltip only (`abilities.sql:126`: 300F/30H, then a DoT of 100F/10H, 15 × 2 s). Same as 1882. | D-DR12 |
| 1894 | Phase Grenade | DR-L2-soldier-2-stances | No effects at all. | Its tooltip (`abilities.sql:536`) is word for word 1329's, so it is probably a copy-paste placeholder. Even the numbers would be inferred. | D-DR12 |
| 854 | Smoke Grenade | DR-L2-soldier-2 | No effects at all. | Tooltip only (`abilities.sql:1366`: +200 Defense to targets, Medium AE, no duration). | D-DR12 |
| 994 | Deployable: Stim Pack | DR-L4-scientist | Effect 3371 has no NVPs, and there is no deployables row. | `effects.sql:1916` "Heals 5% Focus every 3 seconds" with 100 pulses. Two things would be inferred: an ally-pulse routing (D-AB12) and a `deployables` row, whose prefab, phase and spawn are not in client data (deployables README lines 78-86). | D-DR14 |
| 1225 | Deployable: Health Regen | DR-L4-scientist | Effect 3373 has no NVPs, and there is no deployables row. | `effects.sql:4659` "5% Heal every 3 seconds": the same as 994. | D-DR14 |
| 1015 | Deployable: Gravity Well | DR-L4-scientist | Its effects (3113 and siblings) have no NVPs. | `effects.sql:1850` lists "Secondary" removals (AB-07 scope), and the deployables README marks 1015 "unresolved". | D-DR14 |
| 997 | Full Mag: Dart Pistol | DR-weapon-audit-fail | It requires the moniker ITEM_Dart_Rifle (3257416555), which no client item carries. | One could swap in ITEM_DartPistol (2389790449): the sibling 1000 "Dart Rifle: Multi-Shot" (`abilities.sql:14`) requires that moniker. The audit calls the tag "likely a wrong tag" (weapon-requirement-audit.md:56-59), and OD-CS16 decided "no seed change". | D-DR15 |
| 1246 | Stealthed Strike | DR-L3-commando-1-item-melee | It requires ITEM_Melee (3756430796), which no client item carries. | One could substitute ITEM_Blade. The audit itself calls that substitution an "inference". OD-CS16 decided no seed change. | D-DR15 |
| 1355 | Lethal Strike | DR-L3-commando-1-item-melee | It requires ITEM_Melee, as 1246 does. | As 1246 (`abilities.sql:2022`). | D-DR15 |
| item 3128 | GW-3128 (gear row) | [DR-L2-canary](results/DR-L2-canary.md) | It binds 559 "Automatic Weapon Auto Attack" (event 7) and 595 (event 6), but the item doesn't carry ITEM_Automatic_Weapon (3175425141). | 203 client items carry that moniker; item 3128 in the client pak does not. Adding the moniker, or dropping the binding, would be inferred either way. The client data is self-inconsistent. Default: no change, and the row is graded as client-faithful. | D-DR16 |

## UNKNOWN (owner decisions; the ability keeps refusing)

| Abilities | Lab issue | Gap | Why there is no basis | Decision |
|---|---|---|---|---|
| 1456, 1455, 2102 | DR-L2-soldier-1-ground, DR-L2-soldier-2 | No effects, or effects with no numbers. | The text gives a shape ("Flamethrower: Ground Target AOE: Short / 25 Ammo") but no amount. | D-DR12 |
| 1363, 1366, 1868, 642, 886, 888, 890 | DR-L3-commando-0 | Effects with no number in either the effect text or the tooltip. | None in client data. | D-DR12 |
| 965, 1014, 1215, 1226, 2955, 1223 | DR-L4-scientist, DR-L4-scientist-ground | No effects, or no numbers. The deployables README marks 1014 and 1223 "no effects". | None. | D-DR12 |
| 1224 | DR-L4-scientist-ground | No effects. | The deployables README calls it greenfield and leaves it to the owner. | D-DR12 |
| 883, 884, 901 | DR-L3-commando-0 | Disarm or detect a mine. | No mine entity or mechanism. | D-DR12 |
| 887, 2173 | DR-L3-commando-0 | Spawn a C-4 charge; Speed-Deploy. | No spawn or deploy-speed mechanism, and no numbers. | D-DR12 |
| 1236 | DR-L4-scientist | A threat deployable. | Needs a threat script (deployables README). | D-DR12 |
| 943, 944, 945 | DR-L4-scientist | Conversation: Mechanical. | It has no minigame effect row. The minigame launchers (MG-62) cover Humanoid only. | D-DR12 |
| 649, 874, 875, 1244 | DR-L3-commando-1 | Reveal and detect. | Out of scope by D-AB11 (stealth). The 1244 tooltip and effect also disagree. | D-DR12 |
| 872, 1253, 880, 1250 | DR-L3-commando-1, DR-weapon-audit-fail | Fade and stealth. 1250's moniker ITEM_Stealth_* is on no item. | Out of scope by D-AB11 (no stealth system). Covered by OD-CS16. | D-DR12, D-DR15 |
| 962, 963, 964, 966, 971, 972, 1206, 1212, 1214, 967, 968, 1207 | DR-L4-scientist | Turrets. | They need the turret system (#1009); there are no turret rows and no numbers. | D-DR12 |

## Owner decisions

All of these are **BlockedDecision**. The orchestrator asks; the architect does not.

| ID | Question | Abilities | Recommended default |
|---|---|---|---|
| D-DR10 | Channel and pulse shape: may a channel's tick count and spacing be taken from the text ("5 ticks", "50 ticks") when the client row says PulseCount 1? When the effect text and the tooltip disagree on the total, which one wins? | 718, 725, 1477, 656 | Keep refusing. If yes, the effect text wins (consistent with DR-S1), as a generator change to the damage family's pulse grammar, not hand rows. |
| D-DR11 | Percent units for resistance: adopt D-AB09 (10 points per 1%) for "+10% Interrupt Resistance"? And how should conditions like "with missiles" and toggles on Target abilities work? | 1483, 1019, 1231, 1232 | Keep refusing until D-AB09 is decided. |
| D-DR12 | Should server-only effects be written from tooltips for abilities with no effect rows, or with no numbers at all? Should the out-of-scope systems (stealth, mines, C-4, turrets, Conversation: Mechanical) be pulled in? | 1882, 1329, 1894, 854 and all the UNKNOWN rows | No. They stay refusing and are listed as known gaps. |
| D-DR13 | Conditional and sequenced damage: target-class splits (EMP mechanical vs other), timers (Sticky Bomb) and per-charge detonation (Detonate C-4). | 2864, 1259, 3358 | Keep refusing; each needs a mechanism before it needs numbers. |
| D-DR14 | Group, aura and deployable-heal routing, under D-AB12. Includes Rally's 10% against 15% and Commander's attack speed. | 1891, 1893, 1231, 1232, 994, 1225, 1015 | Wait for D-AB12. |
| D-DR15 | Reopen OD-CS16? It would retag 997 to ITEM_DartPistol and 1246 and 1355 to ITEM_Blade. 1250 stays with stealth. | 997, 1246, 1355, 1250 | No: OD-CS16 stands, and the rows stay BLOCKED (D-DR3). |
| D-DR16 | GW-3128: add ITEM_Automatic_Weapon to item 3128, drop its 559 and 595 bindings, or keep it as the client has it? | item 3128 | Keep it as the client has it, and grade the row "client-faithful". |
| D-DR17 | Priority of AB-05 (regen model; needs D-AB03 and D-AB04) and AB-11 (the clear-on-damage hook). Once built, they unblock the EVIDENCED values for 858 and 860 through the generator. | 858, 860 | Schedule them in the ability-mechanics campaign, not here. |
