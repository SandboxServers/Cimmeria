# Combat Formulas — What Is Recoverable

> **Type:** reference (status + answer). **Last updated:** 2026-09-18.
> **Evidence docs:** [client evidence](combat-formulas-client-evidence.md) (cooked UnrealScript, entity XML, Lua UI, `SGW.exe`) and [game-data evidence](combat-formulas-game-data-evidence.md) (the seeded `resources` schema). This page consolidates their conclusions; read them for the reproducible queries, addresses and per-row references.

## Short answer

**No — the original combat formulas are not in anything we have, and no shipped artifact can produce them.** Every original artifact (client binary, cooked UnrealScript, cooked XML, the seeded game database, the UI Lua) was searched for all seven questions. The original server, where the maths ran, was never distributed.

What we **do** have, all verified, is the *unit system and vocabulary* the formulas operated in, plus a good deal of designer-written numbers on individual abilities. That is enough to fix the units and the order of magnitude of every stat; it is not enough to reconstruct the combination rules (thresholds, curves, ordering, caps).

## Evidence classes used here

| Tag | Meaning |
|---|---|
| **ORIGINAL-DATA** | Shipped game data or shipped declarations (`alias.xml`, `enumerations.xml`, the cooked ability/effect/item data, the seeded DB rows that came from them). |
| **ORIGINAL-RE** | Read from the decompiled client (`SGW.exe`, Ghidra) or the cooked `.u` scripts. |
| **DESIGNER-TEXT** | A description/tooltip string the original designers wrote (e.g. effect 2003 `Cover Defense +100`). Reliable about intent, not a formula. |
| **FAN-GUESS** | The earlier fan emulator's reconstruction — `deprecated/python/` (`AbilityManager.py` `DamageCalc`, `common/Config.py` `QR_*`) and the Rust ported from it. **Not canonical** (owner-confirmed 2026-09-18). Never cited as evidence. |
| **UNKNOWN** | The shipped data does not say. |

## What is verified, per question

### 1. Accuracy vs Defense → hit/miss

- **Verified (ORIGINAL-DATA):** the units. From the original `alias.xml` designer comments ([`entities/defs/alias.xml`](../../../entities/defs/alias.xml), byte-identical to the client copy under `Common/res/entities/defs/`): `accuracy` "modifies outgoing ranged and melee QR by +0.01 per point" (line 204), `defense` "modifies incoming … QR by −0.01 per point" (205), `qrMod` "modifies both attacking and defending QR by 1 per point" (206). So **Accuracy and Defense are both denominated in QR, 100 points = 1 QR.**
- **Verified (ORIGINAL-DATA):** the outcome vocabulary — Hit / Miss / Critical / DoubleCritical / Glancing (`entities/defs/enumerations.xml`, result-code enum) — and that QR is rolled **per effect**: `EF_DontUseQR` (bit 4, value **16**, `enumerations.xml:1101`) is set on 754 of 3,216 cooked effects, and 717 damage-type effects roll QR while buffs/debuffs carry the bypass. `EF_CalculateQRFromTarget` (bit 22) is set on 39 effects.
- **UNKNOWN:** how a QR value becomes a hit/miss/crit probability (thresholds, distribution, base values). Not in `SGW.exe`, the `.u` scripts, or any cooked table.
- **Designer intent, with caveats:** Accuracy/Defense buffs are authored as `±100`/`±200` (effects 1982, 903, 2002). The QR↔ACC statements disagree across a few abilities (game-data doc, Q1), so no constant is derived from them.

### 2. Cover, Cover Penetration, Crouching Defense

- **Verified (ORIGINAL-DATA):** the stat semantics in `alias.xml`: `coverDefense` "increases the defense of a player in cover by −0.01 QR" per point (235), `crouchingDefense` "… while crouching by −0.01 QR" (237), and `coverAccuracy` "increases the accuracy of attacks against a target inside cover by +0.01 QR" (234). Cover is therefore a **QR modifier in the same 0.01-QR units as Accuracy/Defense**, not a damage reduction.
- **Verified (ORIGINAL-DATA, linkage):** **"Cover Penetration" is the `coverAccuracy` stat** — ability 1450 `+100 Cover Penetration` invokes effect 1741 `+100 CoverAccuracy`; `EStats` has no separate `coverPenetration`. Designer text also uses QR directly ("Ignores 2 QR of Cover", ability 1487; "+1 QR Cover Penetration", effect 4995).
- **Verified (ORIGINAL-DATA):** cover geometry is tagged `ECoverHeight` Low / Mid / High / LOS and `ECoverQuality` Good / Better / Best / None (9,353 seeded nodes, `cover_nodes`) — but **no numeric bonus is attached to any height or quality**, anywhere.
- **Designer-text values:** effect 2003 `Cover Defense +100`; Hunker Down (ability 1454) `+100 Cover Defense`, its effects 1746/1747 agree; Cover Stance (ability 1451) tooltip `+200 Cover Defense` but its effect 4565 is `+100 CoverDefense`; Duck and Cover (ability 1452) tooltip `+100 Crouching Defense`; effect 4338 `Cover Debuff` (Secondary Targets, AOE Medium) `−400 Cover Defense`.
- **UNKNOWN:** the Low/Medium/High bonus values; how Cover Defense and Cover Penetration net against each other (both are additive QR in the units above, so an additive net is the natural reading — but that is inference, not evidence); any crouching calculation beyond the stat's own definition. Ability 1452's "+100 Crouching Defense" tooltip contradicts the effect it invokes (1743, `+200 CoverDefense: 15 seconds`), and ability 1451's `+200` disagrees with its effect 4565 (`+100`), so tooltips alone are not reliable for cover/crouch values.

### 3. Weapon scaling (TechComp, Tier, Quality, Applied Science)

- **Verified (ORIGINAL-DATA):** the shape of what shipped. Items carry Tier 1–5, TechComp 0–100, Quality 1000–5000 and AppliedScienceID 0–4; among the 1,436 seeded weapons mean TechComp rises 12.2 → 26.4 → 38.9 → 47.7 → 52.7 across tiers 1–5 (overlapping ranges); clip size and range do **not** vary by tier; 1,435 of 1,436 are Normal quality; Applied Science is a four-way partition of the discipline tree.
- **UNKNOWN / absent:** **no shipped item or weapon field is a combat stat** — `resources.items` has 23 columns and none is damage/accuracy/penetration. Weapon output was authored on the *ability/effect* side (the `HealthDamage` / `FocusDamage` name-value pairs and designer text), not derived from item stats client-side, and the client UI only prints "Tech Comp: n".

### 4. Armor / Resistance / Mitigation / Penetration; damage types

- **Verified (ORIGINAL-DATA):** semantics in `alias.xml` (lines ~207–269): the four Armor Factors (`physicalAF`, `energyAF`, `hazmatAF` "hazmat/contamination", `psionicAF`) with per-type "density" bonuses; `mitigation` "armor mitigation percent (0–100%)" (243); `penetration` "decreases the target's final calculated mitigation by 1%" per point (225); `kineticRes` / `mentalRes` / `healthRes` "resistance to all harmful kinetic / mental / health effects by 1% additive".
- **Verified (ORIGINAL-DATA, structural):** **resistances are separate QR rolls gating crowd-control effects, not flat reductions.** 79 "Roll"-named resistance effects each share an `effect_sequence` step with the effect they gate; kinetic gates Knockdown/Stun/Snare, mental gates Suppression/Disorient/Fear/Confuse, health gates DoT/Wound/Slow/Blind.
- **Verified (ORIGINAL-DATA):** damage types. `EDamageType` has **five** values — Untyped, Physical, Energy, Hazmat (= "contamination"), Psionic. **Fire, Ice, Sonic do not exist as damage types** (zero matches for ice/cold/frost/thermal/sonic/acid in effects or abilities; "fire" appears only as a verb or ability name, e.g. "Cover Fire", "Penetration Fire Buff"). `EMitigationType` declares 15 material subtypes (Impact, Concussive, Slashing, Piercing, Plasma, Radiation, Electrical, Particle, Biological, Chemical, Thermal, Cold, …) but is **bound to no column, effect or ability** in the shipped data.
- **UNKNOWN:** the AF-to-reduction curve; the order of AF, mitigation, resistance, absorb and penetration; how `EMitigationType` maps onto `EDamageType`. Penetration's only ability-side text is qualitative ("Penetration: Increased"), copy-pasted across unrelated abilities, so treat it as a placeholder.

### 5. Focus

- **Verified (ORIGINAL-DATA):** damage is authored as a Focus/Health **pair**, exactly 10:1 in 7 of 8 original `effect_nvps` rows (288 of 299 ability descriptions; the outlier 5:1 is the sole `RangedEnergyDamage` entry). Archetype pools are 760 Health / 1570 Focus (+10 / +70 per level), identical across archetypes. The client combat text treats Health and Focus deltas as independent optional entries.
- **Verified (DESIGNER-TEXT):** Focus is used as a **gate**: bonus damage against low-Focus targets (Red Mist, ability 1359: `-500F/-50H`, `-1500F/-150H vs Low Focus`; Executioner's Fire, ability 1978: "Bonus Damage when opponent Focus is below 50%"; effects 1410/1602/1608 "Target <25% Focus").
- **UNKNOWN:** anything linking Focus to Accuracy — **no shipped artifact does** — and any Focus→Health bleed rule when Focus reaches 0.

### 6. Buff / debuff stacking and caps

- **Verified (ORIGINAL-DATA):** `EEffectFlag`'s 25 labels are the complete original vocabulary and contain **no** stacking flag, stack count, diminishing-returns flag, immunity timer or cap field. Immunity is a **moniker** (`IMMUNITY_*`) applied/checked/removed by dedicated effects; stance exclusivity is authored as an explicit "Remove Effect of moniker EFFECT_Stance" before the buff — the shipped stacking model is remove-then-apply.
- **UNKNOWN:** flat-vs-percentage ordering and caps. Only two of 3,216 effects mention stacking ("Stacks with …", effects 3663/3664).

### 7. Area effects vs cover / line of sight

- **Verified (ORIGINAL-DATA):** AoE sizes are symbolic bands only (`Short/Medium/Long/Extreme/Weapon` × `Narrow/Medium/Wide/Beam`, `ETargetCollectionParams`), bound to no data; the client error string "You do not have Line of Sight to your target"; `HEIGHT_LOS` on 4 cover nodes. Designer text exists for an AoE cover debuff (effect 4338, `−400 Cover Defense`, AOE Medium) and a cone attack that "Ignores 2 QR of Cover" (Penetrating Barrage, ability 1487).
- **UNKNOWN:** whether AoE is LOS-tested, reduced by cover, or subject to falloff. Zero LOS mentions across 3,216 effects.

## Where the current Rust code diverges from verified data

Cross-checking `crates/` against the original enumerations found two concrete divergences. Both were verified against the client copy of `enumerations.xml`:

1. **`EF_DONT_USE_QR`** — `crates/entity/src/abilities/defs.rs:58` is `32`. The original bit is **16** (`enumerations.xml:1101`); 32 is `EF_HasInductionBar`. The constant is also not read, so the QR bypass on 754 effects is not honoured and every `+200 Accuracy` buff is rolled as if it could miss.
2. **`EDamageType` numbering** — the original values are `DT_Untyped=13, DT_Physical=14, DT_Energy=15, DT_Hazmat=16, DT_Psionic=18` (`enumerations.xml`, `EDamageType`); `defs.rs:82-86` uses 0–4 and `pipeline.rs` sends that as the wire `damage_code`. **Check against a client capture before changing it** — a pcap of what the client accepts takes precedence over the declaration.

Everything else the server does for these seven areas (QR distribution parameters, the `0.05`/`0.01` coefficients, AF/mitigation ordering, resistance as flat percentages, the Focus→Health spillover divisor) is **FAN-GUESS or UNKNOWN**; the resistance-as-flat-percent model in particular conflicts with the shipped structure (resistance as a gating QR roll).

## What would close each gap

| Gap | Only real fix |
|---|---|
| QR → outcome, AF/mitigation curve, ordering, caps, Focus→accuracy/health, AoE-vs-cover | **The original server code or documentation** (the Atrea/BigWorld cell scripts and native combat module). It was never shipped in the client install. Sources that could contain it: an original server/dev build, internal design documents, or a leaked repository. Nothing in this repository qualifies. |
| Live behaviour (hit rates, damage numbers) | Recorded gameplay from the **original live/beta servers** (video, combat logs, screenshots with stat panels). The pcaps and `SGWDebugLog.log` in the client folder are emulator artifacts, not original evidence. |
| Design intent | Archived official material (developer blogs, forum posts, guides, the beta wiki). Secondary sources — label them as such and never treat as formula ground truth. |
| Remaining client-side leads | The Ghidra task list at the end of [combat-formulas-client-evidence.md](combat-formulas-client-evidence.md) (five items, including the native `getUnitCoverHeight` and the `Multiplier` relay at `0x00c81b60`). None is expected to contain formulas — the prior "server-side only" conclusion held — but they are the last unchecked client corners. |
| One lead in our own repo | The Rust Focus→Health spillover cites an original Atrea `.script` graph as its source. Verify that file exists and is original before relying on the `/300` divisor; nothing in the resource database supports or refutes it. |

## Known disagreements with existing docs

The client-evidence page lists four claims in `cover-system.md`, `stat-scaling-formulas.md` and `combat-damage-analysis.md` that its evidence disputes (the main one: `0x00904d80` is the cover-node mesh spawner, not `getUnitCoverHeight`). They are not edited here; resolve them when those pages are next touched.

## Recommendation

Treat the combat model as a **design decision parameterized by the verified units**, not as a recovery: keep Accuracy/Defense/cover/crouch/penetration in 0.01-QR units, resist as gating QR rolls, damage as Focus/Health pairs at the authored ratios, and mark every constant beyond that as a tunable with an explicit FAN-GUESS provenance comment. Fix the two divergences above first, since they are checkable against original data today.
