# Combat formulas: what the shipped game data actually proves

**Type:** Reference (Diátaxis). **Status:** evidence survey, not a specification.
**Scope:** the seven open combat-formula questions, answered *only* from data that
shipped with Stargate Worlds — the `resources` schema seeded from the original
`pg_dump`, the type/enum declarations carried in that dump, and the canonical
`entities/defs/`. Everything else is labelled as such.

## Why this document exists

[combat-damage-analysis.md](combat-damage-analysis.md) established that the 2009
client binary contains **no combat formulas** — damage, hit resolution and
mitigation were resolved entirely server-side, and the server was never shipped.
That leaves three candidate evidence sources for the formulas:

1. the shipped **resource database** (content authored by the original designers),
2. the shipped **type declarations** (the stat/damage taxonomy the server used),
3. the **legacy fan Python server** under `deprecated/python/`.

Source 3 is **not evidence**. It is a previous fan developer's reconstruction and
is treated here as a hypothesis to be checked, never as a citation. The current
Rust combat code was ported from that reconstruction, so it inherits the same
status unless a comment cites original data.

This document mines sources 1 and 2 systematically and reports, per question,
what is proven, what is designer intent expressed in prose, and what simply is
not there.

### Provenance legend

| Tag | Meaning |
|---|---|
| **ORIGINAL-DATA** | A value or structural relationship present in the shipped dump. Cited by table + primary key, or seed file + line. |
| **DESIGNER-TEXT** | A human-authored tooltip/description string written by the original designers. Evidence of *intent*; the number may or may not match what the server computed. |
| **INFERENCE** | My reasoning over the above. Never a formula. Always flagged. |
| **FAN-GUESS** | Originates in `deprecated/python/` or the Rust port of it. Not evidence. |
| **NOT IN THE DATA** | Checked and absent. A valid result. |

Counts in this document were produced against the `resources` schema of the
seeded database (3,216 effects, 1,886 abilities, 6,059 items, 9,353 cover nodes).
Every query is reproduced in [Appendix A](#appendix-a-extraction-queries).

### Summary

| # | Question | Does the shipped data give a formula? | What it *does* give | Rust today |
|---|---|---|---|---|
| 1 | Accuracy vs Defense → hit/miss | **No** | QR is per-effect and `EF_DontUseQR` says when it is skipped (754 effects); Acc/Def deltas cluster on 100/200; three inconsistent QR↔ACC statements | Implemented, FAN-GUESS formula; `EF_DontUseQR` mis-valued and never read |
| 2 | Cover / Cover Penetration / Crouching | **No** | 9,353 nodes tagged height × quality, no numeric bonus; "Cover Penetration" **is** `coverAccuracy`; cover is denominated in **QR** | Not implemented — cover is AI positioning only |
| 3 | Weapon scaling (TechComp/Tier/Quality/Science) | **No** | tier→`tech_comp` is monotone (12.2→52.7 mean); quality is unused (1,435/1,436 weapons Normal); damage lives on *abilities*, not items | Not implemented |
| 4 | Armor / Resist / Mitigation / Penetration | **No** | Resistance is a **separate QR roll** co-sequenced with what it gates (79/79); kinetic/mental/health partition the CC list; penetration is qualitative only | Implemented FAN-GUESS pipeline; `KINETIC_RES` never read; damage type hardcoded Physical |
| 5 | Focus → accuracy / Health damage | **No** | Focus damage authored at 10× Health damage (7 of 8); pools 1570/760, +70/+10 per level; `<25%`/`<50%` Focus gates exist | Accuracy link absent (correctly); Health bleed implemented with an unsourced `/300` |
| 6 | Stacking order and caps | **No** | No stacking/DR/cap/immunity-timer flag exists at all; immunity is a **moniker**; exclusivity is authored as explicit remove-then-apply | Effect-instance refresh implemented; **no stat-modifier system at all** |
| 7 | AoE vs cover / LOS | **No** | AoE sizes are symbolic bands only; zero LOS mentions; `HEIGHT_LOS` tags 4 nodes | Collection implemented; cover/LOS/falloff absent; LOS wired to NPC AI only |

The honest headline: **the shipped data contains no combat formula for any of the
seven questions.** What it contains is the *shape* — which stats exist, which
effects roll, what gates what, and roughly how big a designer thought one buff
should be.

---

## 0. Where the combat numbers live — and where they do not

This is the single most important result of the survey, because it bounds every
answer below.

### 0.1 The stat taxonomy shipped. The stat values did not.

The dump carries a complete `EStats` enumeration of **82 stats**
(`db/resources/Combat/Types/EStats.sql:6-89`) — `accuracy`, `defense`, `qrMod`,
`physicalAF`, `coverDefense`, `penetration`, `mitigation`, `negation`, the
absorb family, all of it. **ORIGINAL-DATA.**

It is bound to nothing. A join of `pg_type` against
`information_schema.columns` shows that `EStats`, `EDamageType`,
`EMitigationType`, `EStatType`, `EStatLevel`, `EStatResultCode`,
`EStatValueType`, `ECombatantState`, `EStateField`, `ETargetCollectionParams`,
`EWeaponType` and `EWeaponRange` are **all unreferenced** — declared by the dump,
used by no column in any table ([query A.1](#a1-enum-usage)). Only
`ECoverHeight` and `ECoverQuality` (on `cover_nodes`) are actually bound.

So the shipped resource DB knows *what stats exist* and knows nothing about
*what values they take*.

### 0.2 The character schema persists no combat stats either

`public.sgw_player` has 35 columns: `level`, `archetype`, `discipline_ids`,
`racial_paradigm_levels`, `applied_science_points`, `abilities`,
`bandolier_slot`, `state_field`, position, cosmetics. There is **no** accuracy,
defense, armor, mitigation, resistance, health or focus column
([query A.2](#a2-character-schema-stat-columns)). **ORIGINAL-DATA.**

**INFERENCE:** combat stats were *derived at runtime* from
level + archetype + trained disciplines + equipped items, recomputed on login,
and never persisted. The derivation is the missing formula set. Nothing in the
shipped data constrains it beyond what follows.

### 0.3 The only numeric combat payload in the whole resource DB is 17 rows

`resources.effect_nvps` is the name/value side-table that attaches numbers to
effects. It contains **21 rows total**, of which **17 are original**
(`nvp_id` 1–17, present at the pre-split commit `4b8dea9f`) and **4 are
Cimmeria-authored** additions (`nvp_id` 18–19 via PR #493, 100 via #496, 200 via
#497 — see `git log db/resources/Effects/Seed/effect_nvps.sql`).

The original 17 rows use exactly **three** names
([query A.3](#a3-effect_nvps-full-dump)):

| `name` | rows (original) | values seen |
|---|---|---|
| `HealthDamage` | 8 | 10, 15, 15, 16, 20, 25, 25 |
| `FocusDamage` | 8 | 80, 100, 150, 150, 200, 250, 250 |
| `HealPercentage` | 1 | 35.00 |

Nine effects in 3,216 carry a number. **ORIGINAL-DATA.**

That is the entire numeric combat payload of the shipped content database. There
is no accuracy value, no defense value, no armor factor value, no mitigation
value, no cover bonus value, and no weapon damage value anywhere in it.

### 0.4 What *is* rich: designer prose and structural flags

Two things survive in quantity and carry real signal:

- **`effects.effect_desc` / `abilities.description`** — free-text design notes
  written by the original content team, full of statements like
  `+200 CoverDefense: 15 seconds`. **DESIGNER-TEXT.** These are the closest
  thing to a spec that shipped.
- **`effects.flags`** — a bitmask over the original `EEffectFlag` enum
  (`db/resources/Effects/Types/EEffectFlag.sql`), which encodes *how the server
  was told to resolve each effect*. This is **ORIGINAL-DATA** and is structurally
  more informative than any single number.

The two `EEffectFlag` bits that matter here:

| Bit | Value | Label | Populated rows |
|---|---|---|---|
| 4 | `16` | `EF_DontUseQR` | 754 of 3,216 |
| 22 | `4194304` | `EF_CalculateQRFromTarget` | 39 of 3,216 |

**ORIGINAL-DATA** ([query A.4](#a4-effect-flag-decode)).

> `effects.script_name` is **not** original evidence. The column exists in the
> dump DDL, but all 16 non-NULL values (`RangedPhysicalDamage`, `HealHealth`,
> `Suppression`, …) are Cimmeria's own `EffectScript` registry keys in
> `crates/services/src/cell/effects/registry.rs`. Do not cite it as original.

---

## 1. Accuracy vs Defense → hit / miss

### Verified data

**The QR roll is per-effect, and the data says when it is skipped.**
`EF_DontUseQR` (bit 4, value 16) is set on 754 effects. Cross-tabulating against
whether the description looks like damage ([query A.5](#a5-dontuseqr-vs-damage)):

| `EF_DontUseQR` | looks like damage | effects |
|---|---|---|
| not set | no | 1,745 |
| not set | **yes** | **717** |
| set | no | 729 |
| set | yes | 25 |

**ORIGINAL-DATA.** The pattern is unambiguous: damage effects roll QR, buffs and
debuffs bypass it. Every `+N Accuracy` / `+N CoverDefense` buff effect inspected
carries flag 16 (e.g. effect 700 flags 21, effect 1743 flags 21, effect 4299
flags 17 — all `= 1|16` or `1|4|16`). Effects 641/646/654 (the auto-attack damage
effects) have `flags = 0`.

**QR can be computed from the target instead of the user.**
`EF_CalculateQRFromTarget` (bit 22) is set on 39 effects. The population is
telling: grenade/AoE damage (`927`, `1463`, `1993`, `2729`), crowd control
(`583` Knockdown, `1173` Stun, `1466` Flashbang Stun, `1992` Disorientation,
`3114` Snare) and — critically — the **resist-roll** effects
(`711`, `729`, `935`, `1174`, `1465`). **ORIGINAL-DATA.**

**Accuracy and Defense are a matched pair in the content.** 26 distinct
designer statements pair them or move them alone ([query A.6](#a6-accuracydefense-statements)).
The magnitudes cluster hard on **100 and 200**, with `+150/-40` (13 effects,
example 1089), one `+1000` (effect 928) and one `-2` (ability 863).

### Designer text

The QR ↔ Accuracy/Defense link is stated three times, and the three statements
**do not fully agree**:

| Source | Statement |
|---|---|
| effect `4782` | name `Defensive QR Bonus`, desc `Defense: +100` |
| ability `1729` (`Marksman's Stance`) | `Increases Ranged QR by +1, Decreaes Defense by +1 QR` |
| effect `4299` (the effect ability 1729 actually invokes) | `Single Target  Target +150 ACC, -100 DEF` |

**DESIGNER-TEXT.** Reading 1729 against its own effect 4299: `+1 QR` was written
up as `+150 ACC`, and `-1 QR` as `-100 DEF`.

Two further rows give an explicit point↔percent conversion — the only two in the
database ([query A.7](#a7-point-percent-pairs)):

| Effect | Description |
|---|---|
| `2004` | `+50 (5%) Mental Resist buff` |
| `2005` | `Increased Threat Rating Subtlety -100 (10% increase to threat)` |

**DESIGNER-TEXT.** Both resolve to **10 stat points = 1 percentage point**.

Other QR-denominated statements: effect `701` `+5 QR`; effect `4383`
`+2 QR in Mini-game State`; effect `4995` `+1 QR Cover Penetration`; ability
`1487` `Ignores 2 QR of Cover`.

### Not in the data

- **No QR result-code table.** The 20-entry result-code table described in
  `combat-damage-analysis.md` comes from the client dispatch side, not from here.
  Nothing in the resource DB enumerates result codes; `EStatResultCode`
  (`SRC_None`, `SRC_Absorb`, `SRC_Immune`, `SRC_Mortal`) is declared and
  unreferenced.
- **No accuracy or defense value** for any archetype, item, NPC template or
  ability. The 26 statements above are deltas in prose; there is no base to add
  them to.
- **No hit/miss threshold, no crit threshold, no distribution.**
- **INFERENCE (flagged, not a formula):** the designer text is consistent with
  Accuracy and Defense being *inputs to the QR roll* rather than a separate
  to-hit check — effect 4782 is literally named "QR Bonus" and its only content
  is `Defense: +100`. The conversion rate is **not** pinned down: `+1 QR` is
  written as both `+150 ACC` and `-100 DEF` in the same ability. Do not implement
  a constant from this.

### Current Rust behaviour + provenance

Live path: `useAbility` → `crates/services/src/cell/combat/damage_apply/mod.rs:52`
`apply_damage_to_target` → `cell/combat/damage/{qr.rs, pipeline.rs}`.
(`crates/game/src/combat/` is dead scaffolding, referenced nowhere outside its
own module.)

`crates/services/src/cell/combat/damage/qr.rs:50-73` — **FAN-GUESS**:

```
ranged: qr += COORDINATION.cur * 0.05      melee: qr += ENGAGEMENT.cur * 0.05
qr -= defender PERCEPTION * 0.05
qr += ACCURACY * 0.01 ;  qr -= DEFENSE * 0.01
qr += attacker QR_MOD ;  qr -= defender QR_MOD
qr += AWARENESS * 0.01
```

The file cites its source verbatim at `qr.rs:7`
(`Reference: python/cell/AbilityManager.py:181-230 (DamageCalc class)`) and
`qr.rs:16` (`From python/common/Config.py and python/cell/AbilityManager.py.`).
The `× 0.01` on Accuracy/Defense is the fan author's choice; it happens to be
in the same order of magnitude as the designer text (§ above) but is not
derived from it.

- `qr.rs:21` `QR_ALPHA_BETA = 1.4`, `qr.rs:24` `QR_MULTIPLIER = 2.0`,
  `qr.rs:105-110` `Beta(1.4, 1.4 + qr*2.0)` for `qr >= 0`, else
  `Beta(1.4 - qr*2.0, 1.4)`. **FAN-GUESS** (`Config.py` `QR_*`). *Note: the beta
  distribution here is the correct python shape — the "linear approximation"
  divergence that used to be flagged in this area is no longer present.*
- `qr.rs:27-30` thresholds `MISS < 0.07`, `GLANCING < 0.20`, `HIT < 0.80`,
  `CRITICAL < 0.93`, else `DOUBLE_CRIT`; mapped at `qr.rs:128-140`.
  **FAN-GUESS** (`Config.py:38-41`). Nothing in the shipped data corroborates
  these four numbers.
- `crates/entity/src/abilities/defs.rs:73-78` result codes
  `RC_NONE=0 HIT=1 MISS=2 CRITICAL=3 DOUBLE_CRITICAL=4 GLANCING=5`. **UNKNOWN** —
  not in the resource data; would need checking against the 20-entry table in
  [combat-damage-analysis.md](combat-damage-analysis.md).
- `pipeline.rs:25` `QR_DAMAGE_MULTIPLIER = 2.0`; `:44`
  `raw = base_damage * qr_rand * QR_DAMAGE_MULTIPLIER`; `:63`
  `qr_damage = (res_damage * (1.0 + qr_result.qr)).round()`. **FAN-GUESS.**
  Crit and Miss have no separate damage branch — the only early return is
  `raw <= 0` (`pipeline.rs:45`), so a **Miss still deals damage** (≈10% of base
  at `qr_rand = 0.05`). Matches the python it was ported from.

**Divergences from the shipped data that this survey exposes:**

| Shipped data | Rust | Impact |
|---|---|---|
| `EF_DontUseQR` = bit 4 (**16**), set on 754 effects | `defs.rs:58` `EF_DONT_USE_QR = 32` (wrong bit — 32 is `EF_HasInductionBar`) **and never read** | Buffs/debuffs that must bypass the QR roll are rolled anyway |
| `EF_CalculateQRFromTarget` = bit 22 (4194304), 39 effects | unmodelled; `damage_apply/mod.rs:93` always rolls attacker-vs-defender | Grenade AoE, CC and all 79 resist rolls use the wrong side's stats |
| `EStats` ids are sparse (App. B) | `crates/entity/src/stats/stat_ids.rs` matches `enumerations.xml` correctly | OK |

Several other `EF_*` constants at `defs.rs:56-62` are composite values that
correspond to no single original flag (`EF_STUN = 12` is `ClearOnDeath |
ClearOnDamage`; `EF_SUPPRESSION = 76`; `EF_DOT = 516`). They are currently only
consumed by debug logging in `cell/abilities/cone_aoe/flag_categories.rs`.

Not implemented at all: crouch terms in QR (python has
`qr += crouchingAccuracy*0.01` / `qr -= crouchingDefense*0.01`), `tracking`,
`stabilization`. (`coverQRModifier`, `coverAccuracy` and `coverDefense` are
read since NA32 as modifiers of the cover damage reduction, see §2.) NA32 also swapped the beta branches so the mean
rises with QR; see `docs/gameplay/combat-system.md` "The qrRand distribution".
DoT pulses bypass the roll entirely — fixed `qr_rand = 0.5`, `qr = 0.0`,
`RC_HIT` (`cell/effects/pulsing/tick.rs:229-233`). **FAN-GUESS / UNKNOWN.**

---

## 2. Cover — Low/Mid/High, Cover Defense vs Cover Penetration, Crouching Defense

### Verified data

**Cover geometry shipped in full.** `resources.cover_nodes` holds **9,353 nodes
across 1,353 chunks** ([query A.8](#a8-cover-node-distribution)), each carrying
position, an orientation float, a height enum and a quality enum:

| | `QUALITY_Good` | `QUALITY_Better` | `QUALITY_Best` | `QUALITY_None` |
|---|---|---|---|---|
| **`HEIGHT_Low`** | 192 | 124 | 1,237 | 427 |
| **`HEIGHT_Mid`** | 360 | 389 | 1,698 | 562 |
| **`HEIGHT_High`** | 516 | 495 | 2,811 | 538 |
| **`HEIGHT_LOS`** | 4 | 0 | 0 | 0 |

**ORIGINAL-DATA.** Note the fourth height value: `HEIGHT_LOS`
(`db/resources/Combat/Types/` via the `ECoverHeight` enum) — cover height and
line-of-sight blocking are the *same* enumeration, with 4 nodes tagged as pure
LOS blockers.

**The cover nodes carry no numeric bonus.** The only unaccounted-for column is
`tail`, a fixed 4-byte blob on every row. Decoded as little-endian `f32` it is a
second angle: the most common values are `db0fc9bf` = −1.5708 (−π/2),
`490e49c0` = −3.1416 (−π), `ff12c93f` = +1.5709 (π/2), `1bcb96c0` = −4.712
(−3π/2), plus 665 rows of `00000000`
([query A.9](#a9-cover-tail-blob)). **ORIGINAL-DATA / INFERENCE on the decode.**
There is no field anywhere on a cover node that could hold a defense value.

**"Cover Penetration" is not a stat — it is `coverAccuracy`.** This is proven by
the ability→effect linkage, not by prose ([query A.10](#a10-cover-ability-effect-chains)):

| Ability | Ability description | Invokes effect | Effect description |
|---|---|---|---|
| `1450` **Cover Penetration** | `+100 Cover Penetration` | `1741` | `Single Target  +100 CoverAccuracy` |
| `2130` `MS020_080817_CoverPenetration` | `+100 Cover Penetration` | `2895` | `Single Target  +100 CoverAccuracy` |
| `2201` `Stance: Sharp Shooter` | `Cover Accuracy +100` | `4293` | `Single Target  Target +100 Cover Accuracy` |

**ORIGINAL-DATA (the `effect_ids` array linkage) + DESIGNER-TEXT (the strings).**
`EStats` has `coverAccuracy` and `coverDefense` but **no** `coverPenetration`
entry, which corroborates this.

**Crouching is a broadcast state field.** `EStateField` declares
`BSF_Crouching` at bit 2 (`BSF_Dead`=0, `BSF_AutoCycling`=1, `BSF_Crouching`=2,
`BSF_InCombat`=3, …). `EStats` declares `crouchingAccuracy` and
`crouchingDefense`. **ORIGINAL-DATA.** The mechanism for gating a crouch bonus
therefore existed and shipped.

### Designer text

35 effects and a cluster of abilities mention cover
([query A.11](#a11-cover-and-qr-text)). The stat magnitudes:

| Statement | Count | Example |
|---|---|---|
| `+100 CoverDefense` / `+100 Cover Defense` | 4 | effects 1746, 1747, 2003, 4565 |
| `+200 CoverDefense` | 2 | effects 1743, 2887 |
| `-400 Cover Defense` | 1 | effect 4338 |
| `-100 Cover Defense Debuff` | 1 | effect 4706 |
| `+100 CoverAccuracy` / `Cover Accuracy` | 3 | effects 1741, 2895, 4293 |
| `+200 Cover ACC` | 2 | effects 905, 3001 |
| `+100 Crouching Defense` | 2 (abilities only) | abilities 1452, 2123 |
| `+1 QR Cover Penetration` | 1 | effect 4995 |
| `Ignores 2 QR of Cover` | 2 (abilities) | abilities 1487, 2098 |

**Cover is denominated in QR.** Ability `1487` *Penetrating Barrage* —
`Ignores 2 QR of Cover` — and effect `4995` *Penetration Fire Buff* —
`+1 QR Cover Penetration` — both express cover benefit in QR steps rather than
stat points. This matches the `coverQRModifier` entry in `EStats`, which has no
non-cover analogue.

**A designer-text inconsistency worth knowing about:** ability `1452`
*Duck and Cover* is described `+100 Crouching Defense`, but the single effect it
invokes (`1743`) is described `+200 CoverDefense: 15 seconds`. The ability
tooltip and the effect note disagree on both the stat and the magnitude.
`crouchingDefense` is named in `EStats` and in two ability tooltips and **never**
in any effect description.

### Not in the data

- **No numeric bonus attached to `HEIGHT_Low` / `HEIGHT_Mid` / `HEIGHT_High`.**
  Checked every column of `cover_nodes` and `cover_sets`. The height and quality
  enums are geometry tags consumed by the (server-side) cover evaluator; the
  values that evaluator returned are absent.
- **No numeric bonus attached to `QUALITY_Good` / `Better` / `Best` / `None`.**
- **No table mapping (height, quality) → QR, defense, or anything else.**
- **No `coverPenetration` stat.** The concept maps onto `coverAccuracy`.
- **No crouching bonus value.** Two ability tooltips claim `+100`; no effect
  agrees with them.

### Current Rust behaviour + provenance

> [!WARNING]
> Superseded by NA32 (D-NA15a): cover is now a 10-60% damage reduction
> rated by the node's quality and height, for a defender at a cover node
> that faces the attacker. See `docs/gameplay/combat-system.md`
> "Cover as damage reduction". The text below records the state before NA32.

**Cover confers no combat benefit. NOT IMPLEMENTED.** Cover exists only as NPC-AI
positioning input.

- The stats are declared and seeded to zero, and nothing in combat reads them:
  `crates/entity/src/stats/stat_ids.rs:42` `COVER_QR_MODIFIER = 48`;
  `:70-73` `COVER_ACCURACY = 66`, `COVER_DEFENSE = 67`,
  `CROUCHING_ACCURACY = 68`, `CROUCHING_DEFENSE = 69`. Seeded `(0,0,0)` at
  `stats/stat_list.rs:68,96-99`. **ORIGINAL-DATA-BACKED** for the ids (they match
  `enumerations.xml`), **NOT IMPLEMENTED** for the behaviour.
- `BSF_CROUCHING` is a broadcast flag only:
  `crates/services/src/cell/cell_methods/combatant.rs:21` `BSF_CROUCHING = 1<<2`,
  `:31-43` `setCrouched`. `calculate_qr` never reads it.
- `crates/services/src/cell/cover/types.rs:12-39` `CoverHeight`
  `Low = 0.71` (commented *crouch-defeatable*), `Mid = 1.07`, `High = 1.52`,
  `Los = 2.52`. These are **ORIGINAL-DATA-BACKED** — the comment cites
  *"Binary-confirmed heights via direct memory read of `DAT_018f41c8/cc/d0/d4` in
  SGW.exe"* ([cover-system.md](cover-system.md)). They are *metres of geometry*,
  not combat bonuses. Note the variant is `Mid`, matching `HEIGHT_Mid`.
- `cover/types.rs:67-77` `CoverQuality::score_factor` `Best 1.0 / Better 0.66 /
  Good 0.33 / None_ 0.0` — **INFERENCE in code**, used only for AI cover scoring,
  never for defense.
- `cover/scoring.rs:50-62` `CoverWeights` defaults (distance 1.0, def_cover 1.0,
  off_cover 0.5, move 0.3, cross_path 0.2, cover 1.0,
  squad_affinity_penalty_per_ally 0.2), commented *"Defaults from the V5 RE
  notes."* AI-only.
- No "Cover Penetration" concept exists in the Rust code. Per §2 above it should
  map to `COVER_ACCURACY` (66), not to `PENETRATION` (57) — `PENETRATION` is
  used only in armor mitigation (`pipeline.rs:59`).

> [!WARNING]
> [docs/gameplay/combat-system.md](../../gameplay/combat-system.md) line 30
> claims `Crouch / cover stance | PARTIAL | State flag set, affects QR`. The
> second half is wrong — the flag is set and broadcast, but `calculate_qr`
> (`damage/qr.rs:50-73`) contains no crouch or cover term. That row should read
> "state flag set and broadcast; no combat effect".

---

## 3. Weapon scaling by TechComp / Tier / Quality / Applied Science

### Verified data

**`resources.items` has 23 columns and not one of them is a combat stat.** The
full column list is `item_id, applied_science_id, description, icon_location,
name, quality_id, tech_comp, tier, container_sets, max_stack_size, moniker_ids,
max_ranged_range, min_ranged_range, visual_component, max_melee_range,
min_melee_range, discipline_ids, flags, ammo_types, default_ammo_type,
clip_size, charges`. **ORIGINAL-DATA.** There is no damage, accuracy,
penetration, mitigation or armor column.

**Items do not carry stats via monikers either.** `items.moniker_ids` is
populated on 4,996 of 6,059 items, but `resources.monikers` has only **53 rows**,
all of them pure category tags: `CATEGORY_Weapons`, `CATEGORY_Armor`,
`ITEM_Assault_Rifle`, `ITEM_Zat`, `COOLDOWN_Grenade_Launcher`,
`Soldier_HeavyWeapons`, … ([query A.12](#a12-moniker-dump)). Zero stat payload.

**The tier → tech_comp relationship is real and monotone.** Filtering to items
tagged `CATEGORY_Weapons` (moniker `3901383057`), n = 1,436
([query A.13](#a13-weapon-tier-quality-techcomp)):

| Tier | Quality | n | `tech_comp` min | max | mean | `clip_size` range | `max_ranged_range` range |
|---|---|---|---|---|---|---|---|
| 1 | Normal | 432 | 1 | 45 | 12.2 | 0–250 | 0–40 |
| 2 | Great | 1 | 20 | 20 | 20.0 | 7 | 35 |
| 2 | Normal | 338 | 5 | 33 | 26.4 | 0–250 | 0–40 |
| 3 | Normal | 339 | 33 | 48 | 38.9 | 0–250 | 0–40 |
| 4 | Normal | 173 | 45 | 50 | 47.7 | 0–250 | 0–40 |
| 5 | Normal | 153 | 50 | 55 | 52.7 | 0–250 | 0–40 |

**ORIGINAL-DATA.** Mean `tech_comp` rises monotonically with tier
(12.2 → 26.4 → 38.9 → 47.7 → 52.7) while the ranges overlap, so `tech_comp` is
*correlated with* tier, not a function of it. Note that `clip_size` and
`max_ranged_range` do **not** scale with tier at all — the same 0–250 and 0–40
spread appears in every tier.

**Quality is effectively unused for weapons.** 1,435 of 1,436 weapons are
`ITEM_QUALITY_Normal`. Across all 6,059 items: Normal 5,907, Good 68, Great 62,
Poor 18, Fantastic 4 ([query A.14](#a14-quality-distribution)). The four
`Fantastic` items are `Anat`, `The Venio`, `Testing Ballistic Vest` and
`Furling Carapace` — one of which is explicitly a test asset. **ORIGINAL-DATA.**

**Applied Science is a 4-way partition of the discipline tree, not a weapon
scalar.** `resources.applied_science` has exactly four rows: `1 Biomedical
Engineering`, `2 Materials Engineering`, `3 Power Systems Engineering`,
`4 Electronic Engineering`. `resources.disciplines` (78 rows) hangs off it and
carries `tech_competency`, `racial_paradigm_level`, `row`, `column`.
**ORIGINAL-DATA.**

`tech_competency` tracks the tree `row` at **5 points per row**
([query A.15](#a15-discipline-tech-competency)):

| `row` | 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |
|---|---|---|---|---|---|---|---|---|---|---|
| `tech_competency` | 1, 2, 5 | 10, 15 | 15 | 20 | 25 | 25, 30 | 35 | 40 | 45 | 50 |

Exact for rows 3–5 and 7–10; rows 1, 2 and 6 carry a second value. **ORIGINAL-DATA.**

Weapons distribute evenly across all four sciences (396 / 250 / 521 / 268), each
spanning the full `tech_comp` 1–55 range — so Applied Science partitions *who can
build it*, not *how strong it is*.

### Designer text

Nothing. No item description in the database states a damage, accuracy or
penetration number. 220 of 6,059 item descriptions contain any digit at all, and
spot-checking those turns up flavour text and crafting quantities, not stats.

### Not in the data

- **Weapon damage is absent entirely.** There is no column, no side-table, no
  moniker and no NVP that carries it. The nine `effect_nvps` damage values
  (§0.3) belong to *abilities*, not weapons — `Pistol Auto Attack Damage`
  (effect 641, `-100F/-10H`) is one number for all pistols.
- **No tier → stat table, no quality multiplier, no tech_comp curve.** The table
  above is the scaling *shape* that shipped, and it stops at `tech_comp`.
- **INFERENCE (flagged):** given that abilities carry the damage numbers and
  items carry only `tech_comp`/`tier`/`ammo_types`/`clip_size`, weapon-to-weapon
  differentiation at the same tier was probably expressed through *which ability
  set the weapon grants* rather than through per-item stat rolls. `items` has no
  ability column, so this cannot be confirmed from the data.

### Current Rust behaviour + provenance

**NOT IMPLEMENTED.** No weapon property affects damage.

- `items.quality_id` / `tech_comp` / `tier` / `applied_science_id` exist in the
  schema (`db/resources/Items/Tables/items.sql:8,12-14`) and are read by **no**
  combat path. The only Rust `INSERT`s naming them are test fixtures
  (`crates/services/src/base/character/request_visuals_live_db_tests.rs:117`,
  `crates/services/src/base/console_authoring/tests.rs:93`).
- `AbilityDef` (`crates/entity/src/abilities/defs.rs:101-117`) has no scaling
  fields. Base damage comes **only** from the effect NVPs
  (`crates/services/src/cell/combat/damage_apply/mod.rs:113-134`), with an
  unknown-ability fallback of `(15, 0)` at `:133`. **ORIGINAL-DATA-BACKED** —
  this correctly reflects §0.3, that the shipped numbers live on abilities, not
  items.
- `damage_apply/mod.rs:135-141` doubles player health damage, commented
  `// Temp: 2x player damage so players can kill NPCs before dying`.
  **FAN-GUESS / acknowledged placeholder.**
- `applied_science_points` is wired to crafting only
  (`crates/entity/src/crafting.rs:64`); `tech_competency` appears only in
  `crates/services/src/minigame/session.rs:51`.
- `crates/game/src/inventory/items.rs:5` declares
  `ItemQuality { Common..Legendary }` — dead code, and it does **not** match the
  shipped `EItemQuality` (`Fantastic/Great/Good/Normal/Poor`).
- Level scaling is HP/Focus only:
  `crates/entity/src/stats/stat_list.rs:324-344`
  `max = base + per_level*(level-1)`, with `10` / `70` hardcoded at
  `crates/services/src/mercury/world_data/stats.rs:21-40`.
  **ORIGINAL-DATA-BACKED** — those two constants match
  `resources.archetypes.healthPerLevel` and `focusPerLevel` exactly (§5), though
  they are hardcoded rather than read from the table.
- NPCs: `HP = 200 + level*50`, `FOCUS = 200`
  (`crates/services/src/cell/space_manager/spawn.rs:185-192`). **FAN-GUESS.**

---

## 4. Armor / Resistance / Mitigation / Penetration and damage types

### Verified data

**The damage-type taxonomy is five wide.** `EDamageType`
(`db/resources/Combat/Types/EDamageType.sql:6-12`): `DT_Untyped`, `DT_Energy`,
`DT_Hazmat`, `DT_Physical`, `DT_Psionic`. **ORIGINAL-DATA.** Unreferenced by any
column.

**A second, finer taxonomy also shipped — and is likewise unused.**
`EMitigationType` (`db/resources/Combat/Types/EMitigationType.sql:6-22`) has
15 entries in a two-level hierarchy:

| Family | Subtypes |
|---|---|
| `MITIGATION_Physical` | `_Impact`, `_Concussive`, `_Slashing`, `_Piercing` |
| `MITIGATION_Energy` | `_Plasma`, `_Radiation`, `_Electrical`, `_Particle` |
| `MITIGATION_Environmental` | `_Biological`, `_Chemical`, `_Thermal`, `_Cold` |

**ORIGINAL-DATA.** Note there is no Psionic family here, and `Environmental`
occupies the slot `EDamageType` calls `Hazmat`.

**`EStats` carries four parallel mitigation families**, in three layers
(`db/resources/Combat/Types/EStats.sql`): `physicalAF / energyAF / hazmatAF /
psionicAF` (armor factor), `physicalDensity / energyDensity / hazmatDensity /
psionicDensity`, and `PhysicalDamagePercent / EnergyDamagePercent /
HazmatDamagePercent / PsionicDamagePercent / UntypedDamagePercent`. Plus a
15-entry absorb family (`absorbPhysical`, `absorbPhysicalItem`,
`absorbPhysicalEnergy`, × 5 types), plus scalar `mitigation`, `penetration` and
`negation`. **ORIGINAL-DATA.** No values anywhere.

**Resistance is resolved as a separate QR roll, not as flat reduction.** This is
the strongest structural result in this section. The content contains **188
resist-named effects** ([query A.16](#a16-resist-effect-families)), and the
largest families are literally named *rolls*:

| Effect name (normalised) | Count | Example |
|---|---|---|
| `mental resist` | 58 | 709 |
| `mental resist roll` | 26 | 707 |
| `kinetic resist roll` | 26 | 711 |
| `kinetic resist` | 23 | 791 |
| `health resist roll` | 19 | 716 |
| `mental resist check` | 5 | 797 |
| `health resist` | 4 | 2725 |
| `health resist check` | 2 | 929 |

**ORIGINAL-DATA.** These roll effects carry `EF_CalculateQRFromTarget` and have
`EF_DontUseQR` **clear** — so they do roll, and they roll against the *target's*
stats.

**Every resist roll shares an `effect_sequence` step with the effect it gates —
79 out of 79** ([query A.24](#a24-resist-roll-sequence-pairing)). Ability `868`
*Flashbang Grenade* is the canonical shape:

| `effect_sequence` | Effects in that step |
|---|---|
| 0 | `Kinetic Resist Roll` (1465) + `Flashbang Stun: 5 Seconds` (1466) |
| 1 | `Explosion` (938) |
| 2 | `Health Resist Roll` (935) + `Flashbang Blind Debuff` (937) |

**And the pairing is semantically typed.** Grouping each roll by what it shares a
step with ([query A.25](#a25-resist-type-to-cc-family-mapping)):

| Roll stat | Gates (count) |
|---|---|
| `kineticRes` | Knockdown 8 · Aimed Shot 4 · Snare 4 · Flashbang Stun 4 · Missile Knockdown 3 · C-4 3 |
| `mentalRes` | Suppression 14 · Direct Damage 10 · Disorient 4 · Fear 3 · Confuse 2 |
| `healthRes` | DoT 5+2 · Wound 3+2 · Slow 3 · Blind 3+2 · Snare Debuff 2 · Contagion 2+2 |

**ORIGINAL-DATA.** The three resistance stats partition the `ECombatantState`
crowd-control list (`PLAYER_STATE_Blind`, `_Confuse`, `_DoT`, `_Disease`,
`_Disorient`, `_Fear`, `_KnockBack`, `_KnockDown`, `_Slow`, `_Snare`, `_Stun`,
`_Suppression`): kinetic covers displacement CC, mental covers perception CC,
health covers physiological/DoT effects.

**INFERENCE (flagged):** `kineticRes` / `mentalRes` / `healthRes` were QR
modifiers on a dedicated resist-roll effect that gated the co-sequenced effects —
not subtractive damage reducers. The resist-roll effects carrying no
`effect_nvps` payload is consistent with this: they contribute a roll, not a
number. What the data does **not** say is how a failed roll was signalled or
whether it produced partial or zero application.

### Designer text

19 effects and a cluster of abilities state armor/mitigation intent
([query A.17](#a17-mitigation-and-penetration-text)):

| Statement | Effect(s) |
|---|---|
| `+15% Phys AF` | 3353 |
| `+15% Energy AF` / `+10% Energy AF` | 3354, 3745 |
| `+15% Contamination AF` / `+10% Contamination AF` | 3355, 3854 |
| `Energy AF +100  300 Seconds` | 4813 |
| `Energy AF Reduced 10%` | 4791 |
| `Target +10% Physical Mitigation` | 3148 |
| `Target +15% Physical Mitigation` | 4270, 4271 |
| `Target +15% Contamination Mitigation` | 4272 |
| `Energy Mitigation -100  Duration 30 seconds` | 4657 |
| `+ 20% Mitigation All types` | 4303 |
| `Physical Damage Mitigation: 100%` (20s) | 4992 |
| `Energy Damage Mitigation: 100%` (20s) | 4994 |
| `+150 Negation` | 4475 |
| `+50 (5%) Mental Resist buff` | 2004 |
| `+100 Mental Resistance` | 1748 |
| `+15 Kinetic Resist` | 2392 |

**DESIGNER-TEXT.** Three vocabulary facts fall out:

1. **"Contamination" is the designers' word for Hazmat.** No `contaminationAF`
   exists in `EStats`; `hazmatAF` does. `EMitigationType` calls the same family
   `Environmental`. Three names, one concept.
2. **AF and "Mitigation" are used interchangeably in prose** — effect 4645 is
   named `Energy Mitigation buff` and described `Energy AF + 10%`. `EStats` has
   them as separate entries (`energyAF` and a scalar `mitigation`), so the
   designer prose is looser than the stat model.
3. **AF appears as both a percentage and a flat point value** (`+15% Energy AF`
   vs `Energy AF +100`), which is unresolvable from the text alone.

**Penetration is qualitative in the content, never numeric.** Every ammunition
ability states a tradeoff in words, with no numbers at all:

| Ability | Text |
|---|---|
| `719` Armor Piercing Ammunition | `Damage Type: Physical  Penetration: Increased  Damage: Reduced` |
| `715` Hollow Point Ammunition | `Damage Type: Physical  Penetration: Decreased  Damage: Increased` |
| `723` Incendiary Ammunition | `Damage Type: Energy  Penetration: Nominal  Damage: Nominal` |

**DESIGNER-TEXT.** Caution: the Hollow Point string is copy-pasted verbatim onto
EMP (`1445`), Explosive (`1446`) and five Dart-type abilities (`990`, `991`,
`992`, `998`, `999`) — all of which claim `Damage Type: Physical` even where that
is clearly wrong for the ammo. Treat these as placeholder text, not as a
damage-type assignment.

### Not in the data

- **Fire, Ice, Radiation, Plasma and Sonic do not exist as damage types.**
  Word-boundary search across all 3,216 effect names+descriptions and all 1,886
  ability names+descriptions ([query A.18](#a18-damage-subtype-word-search)):

  | Term | effects | abilities |
  |---|---|---|
  | `fire` | 103 | 38 |
  | `contamination` | 9 | 11 |
  | `incendiary` | 8 | 0 |
  | `radiation` | 4 | 0 |
  | `plasma` | 4 | 3 |
  | `poison` | 4 | 5 |
  | `piercing` | 1 | 1 |
  | `psionic` | 1 | 0 |
  | `concussive` | 0 | 1 |
  | `electrical` | 0 | 1 |
  | `particle` | 0 | 1 |
  | **`ice`, `cold`, `frost`, `thermal`, `sonic`, `acid`** | **0** | **0** |

  And `fire` is overwhelmingly the verb: 19 of the 103 are *Cover Fire*,
  *Penetration Fire*, *Suppressing Fire* and friends, and the remainder are
  ability flavour names (`Fire Blast`, `Fire Zone Cone Damage`, `Fire DoT`,
  `Steady Fire Buff`). There is no damage-type column for any of them to populate.
- **No armor-factor, mitigation, resistance, penetration or negation *value*** on
  any archetype, item or NPC template.
- **No mitigation curve, cap, or diminishing function.**
- **`EMitigationType` is referenced by nothing.** The 15-entry subtype hierarchy
  shipped as a declaration and was never bound to content. Whether the live server
  used it is unknown from this data.

### Current Rust behaviour + provenance

**IMPLEMENTED for all four AF families in the pipeline; only Physical is
reachable in live play.**

`crates/services/src/cell/combat/damage/pipeline.rs` — **FAN-GUESS**
(ported from `deprecated/python/cell/AbilityManager.py` `DamageCalc`):

```
:50  damage_bonus  = DAMAGE.cur / 100.0 + 1.0
:55  af            = calculate_armor_factor(defender, damage_type)
:59  af_mitigation = (af * (def MITIGATION.cur - atk PENETRATION.cur).max(0.0) / 100.0).round()
:62  res_damage    = raw * damage_bonus * (1.0 - stat_resist)
:63  qr_damage     = (res_damage * (1.0 + qr)).round()
:64  af_damage     = (qr_damage - af_mitigation).max(0)
```

- `pipeline.rs:181-189` `calculate_armor_factor` sums
  `PHYSICAL_AF + PHYSICAL_DENSITY` (and the Energy/Hazmat/Psionic equivalents);
  `UNTYPED = 0`. **ORIGINAL-DATA-BACKED** in shape — the AF/Density pairing
  matches the `EStats` layout and the python `armorFactorStats` map.
- `pipeline.rs:167-179` `stat_resist`: for `HEALTH` →
  `FORTITUDE*0.01 + HEALTH_RES*0.01`; for `FOCUS` →
  `INTELLIGENCE*0.01 + MENTAL_RES*0.01`; else `0.0`. **FAN-GUESS.**
  **`KINETIC_RES` (id 29) is never read anywhere in the codebase** — the python
  had an `energy` branch (`engagement*0.01 + kineticRes*0.01`) that was not
  ported. Given §4 above (kinetic gates knockdown/stun/snare, 26 resist-roll
  effects), this is the largest single gap in the resist model.
- `pipeline.rs:73-77,126-159` absorption drains `ABSORB_<type>`, then `_ENERGY`,
  then `_ITEM`, and only when `stat_id == HEALTH && af_damage > 0`.
  **ORIGINAL-DATA-BACKED** in stat layout; the drain *order* is a Cimmeria
  decision (python only summed the pools) — see
  [abilities-and-effects-system.md](../../architecture/abilities-and-effects-system.md).
- **No cap on `stat_resist`** — it can exceed 1.0 and flip damage negative before
  the `max(0)` at `:64`. With the archetype grant at
  `crates/entity/src/stats/stat_list.rs:195-197` (`KINETIC_RES 40`,
  `MENTAL_RES 20`, `HEALTH_RES 30`, no source cited — **FAN-GUESS**) plus
  `FORTITUDE`, a player already sits at a non-trivial resist.
- **Damage type is hardcoded `DT_PHYSICAL` on every live path** —
  `damage_apply/mod.rs:157-175` (both the HEALTH and FOCUS calls) and
  `cell/effects/pulsing/tick.rs:239`. The comment at `tick.rs:234-238` is
  explicit: *"Per-effect damage type (DT_ENERGY for staff weapons, etc.)
  requires plumbing a damage_type column onto effects — flagged as a follow-up"*.
  So the Energy/Hazmat/Psionic branches are dead in live play.
- **Subtypes (fire/ice/plasma/radiation/sonic): NOT IMPLEMENTED**, consistent
  with §4 — there is nothing to implement. `EMitigationType` is referenced
  nowhere in Rust.

**A live wire-value divergence this survey exposes.** `EDamageType` is **not**
zero-based. `entities/defs/enumerations.xml:272-281` gives
`DT_Untyped = 13`, `DT_Physical = 14`, `DT_Energy = 15`, `DT_Hazmat = 16`,
`DT_Psionic = 18` (17 is skipped). **ORIGINAL-DATA.**
`crates/entity/src/abilities/defs.rs:82-86` declares `UNTYPED 0, ENERGY 1,
HAZMAT 2, PHYSICAL 3, PSIONIC 4`, and `pipeline.rs:101` sends that value to the
client as `damage_code`. Every damage packet therefore carries a `damage_code`
the client cannot map. Worth verifying against
`docs/reverse-engineering/findings/` pcap data before changing — pcap takes
precedence over both the def file and my reading of it.

---

## 5. Focus — effect on accuracy and on Health damage

### Verified data

**Focus damage is authored at ~10× Health damage.** All eight original damage
effects in `effect_nvps` ([query A.3](#a3-effect_nvps-full-dump)):

| Effect | Ability | Name | `HealthDamage` | `FocusDamage` | Ratio |
|---|---|---|---|---|---|
| `641` | 579 | Pistol Auto Attack Damage | 10 | 100 | 10 |
| `654` | 592 | Pistol Shot Damage | 15 | 150 | 10 |
| `4467` | 3091 | Pistol Shot Damage | 15 | 150 | 10 |
| `3091` | 2228 | Effect on Target (template) | 15 | 150 | 10 |
| `621` | 559 | AW Auto Attack Damage | 20 | 200 | 10 |
| `646` | 584 | Staff Auto Attack Damage | 25 | 250 | 10 |
| `3834` | 2635 | Staff Auto Attack Damage | 25 | 250 | 10 |
| `264` | 221 | Energy Blast Damage | 16 | 80 | **5** |

**ORIGINAL-DATA.** Seven of eight are exactly 10:1. The outlier is the only
`RangedEnergyDamage` entry.

The same ratio is repeated in hundreds of `effect_desc` strings in the
`-NNNF / -NNH` shorthand: `-100F / -10H`, `-300F / -30H`, `-500F / -50H`,
`-800F / -80H`, `-1000F / -100H`, `-1500F / -150H`. **DESIGNER-TEXT**, and it
agrees with the NVP rows.

**Base pools are authored at ~2:1 Focus:Health, rising to 7:1 per level.**
`resources.archetypes` ([query A.19](#a19-archetype-base-stats)):

| archetype | coordination | engagement | fortitude | morale | perception | intelligence | health | focus | healthPerLevel | focusPerLevel |
|---|---|---|---|---|---|---|---|---|---|---|
| Soldier | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Commando | 4 | 4 | 2 | 3 | **5** | **3** | 760 | 1570 | 10 | 70 |
| Scientist | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Archeologist | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Asgard | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Goa'uld | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Shol'va | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Jaffa | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |
| Any | 5 | 4 | 3 | 4 | 3 | 2 | 760 | 1570 | 10 | 70 |

**ORIGINAL-DATA.** Two things to note. First, **the shipped archetype table is
almost entirely undifferentiated** — only Commando has distinct attributes, and
health/focus/per-level are identical for all nine rows. Second, the per-level
gain ratio (70:10 = 7:1) is close to the damage ratio (10:1) while the level-1
pool ratio (1570:760 ≈ 2.07:1) is far from it.

**INFERENCE (flagged):** at level 1 a Focus pool 2.07× the Health pool takes 10×
the damage, so Focus empties roughly 4.8× faster than Health; by high level the
7:1 pool ratio brings that closer to parity. This is the shape of a "Focus is the
shield that breaks first" design. It is a reading of two tables, not a formula.

**Focus is used as a gate on damage abilities.** Several abilities run a "Focus
Check" effect before their damage effect ([query A.20](#a20-focus-effects)):

| Effect | Ability | Name | Description |
|---|---|---|---|
| `1410` | 1247 | Focus Check | `Single Target  Target <25% Focus` |
| `1608` | 1359 | Focus % Check | `Single Target  Target <25% Focus Check` |
| `1602` | 1357 | Low Focus Check | `Target  Low Focus Check` |
| `2577` | 1978 | Executioner's Fire Focus Check | `<50% Focus` |
| `2579` | 1978 | Executioner's Fire Damage | `Single Target w/ < 50% Focus  -800F/-80H` |

**ORIGINAL-DATA (the chain) + DESIGNER-TEXT (the thresholds).** Ability `1978`
*Executioner's Fire* is the clean case, and it uses the same co-sequencing
mechanic as the resist rolls (§4):

| `effect_sequence` | Effects in that step |
|---|---|
| 0 | `Executioner's Fire Damage` (2578) — `-300 F -30 H`, flags 0 |
| 1 | `Executioner's Fire Focus Check` (2577) — `<50% Focus` + `Executioner's Fire Damage` (2579) — `-800F/-80H`, flags 64 |

Baseline damage always lands; a second, ~2.7× larger damage effect is gated by a
Focus check in its own sequence step. Thresholds seen across the content:
**25%** (effects 1410, 1608) and **50%** (effect 2577).

### Not in the data

- **Nothing connects Focus to Accuracy.** Searched all effect and ability text
  for any statement pairing focus with accuracy, hit chance or QR. There is none.
  `EStats` lists `focus` and `accuracy` as independent entries with no derived
  relationship expressed anywhere.
- **Nothing states what happens at zero Focus** beyond the `<25%` / `<50%` gates
  above — no "Focus must be depleted before Health takes damage" rule appears in
  the data, despite the 10:1 authoring ratio implying one.
- **No Focus→Health damage-bleed formula.** The `HealthDamage` and `FocusDamage`
  NVPs are two independent numbers applied to two stats; nothing in the data says
  one converts into the other.

### Current Rust behaviour + provenance

**Focus → accuracy: NOT IMPLEMENTED.** `FOCUS` (id 8) does not appear in
`calculate_qr` (`damage/qr.rs:50-73`). Correct — §5 shows the data does not link
them either.

**Focus → Health damage: IMPLEMENTED, but only inside the effect scripts, and
with a formula the data does not support.**
`crates/services/src/cell/effects/scripts.rs:608-610` (`RangedPhysicalDamage`)
and `:298-300` (`MeleePhysicalDamage`):

```
remaining_pct        = focus_overflow.saturating_mul(100) / focus_damage
spillover            = remaining_pct.saturating_mul(focus_damage) / 300
final_health_damage  = spillover + health_damage
```

If `focus_overflow == 0`, **no Health damage is dealt at all**
(`scripts.rs:589`, `:280`) — i.e. Focus must be depleted before Health takes a
hit. **FAN-GUESS**, though a better-sourced one than most: the code cites
`scripts.rs:604-605` *"matches the legacy Atrea script graph (Node 6 → 9 → 10 →
12)"* and `:554-555` *"Reference: deprecated/python/cell/effects/
RangedPhysicalDamage.py and deprecated/data-scripts/scripts/effects/
RangedPhysicalDamage.script"*. The `.script` file is original Atrea authoring
data, which puts the *shape* on firmer ground than the python alone — but the
magic `/ 300` is not in anything this survey examined. §5 found **no** shipped
statement of a Focus→Health bleed rule.

`RangedEnergyDamage` (`scripts.rs:648-694`) has no gate at all — it hits HEALTH
and FOCUS in parallel. That is a real behavioural split between the two scripts;
note that effect 264, the only original `RangedEnergyDamage` row, is also the
only one whose Focus:Health ratio is 5:1 rather than 10:1 (§5).

**Focus-check gating: NOT IMPLEMENTED.** Nothing reads a target's Focus
percentage to gate a subsequent effect, so the `<25%` / `<50%` executioner
pattern (effects 1410, 1608, 2577) does not work. This is the same missing
mechanism as the resist rolls in §4 — both rely on one effect in a sequence step
gating its co-sequenced siblings.

Player Focus pool `1570 + 70/level`
(`crates/services/src/mercury/world_data/stats.rs`) — **ORIGINAL-DATA-BACKED**,
matches `resources.archetypes` exactly. Regen uses `FOCUS_REGEN` with `.max(1)`
(`cell/service/ticks/regen.rs:81`).

---

## 6. Buff / debuff stacking order and caps

### Verified data

**Effects declare their own lifecycle, and the flags are the whole model.**
`EEffectFlag` (25 labels, `db/resources/Effects/Types/EEffectFlag.sql:6-32`) is
the complete original vocabulary for effect behaviour:

```
EF_Beneficial_Effect, EF_Offline_Time_Counts, EF_ClearOnDeath, EF_ClearOnDamage,
EF_DontUseQR, EF_HasInductionBar, EF_SequenceOnFinish, EF_SequenceOnPulse,
EF_SequenceOnFail, EF_SequenceOnStart, EF_SequenceOnRemove, EF_ClearOnRez,
EF_HideIconOnClient, EF_OnlySendToGroup, EF_OnlySendToSelf,
EF_RemoveOnDisguiseZeroed, EF_RemoveOnBandolierSlotChange,
EF_ResolveOnAbilityUser, EF_DisableDisguiseWhenRemoved, EF_AlwaysPersist,
EF_Response, EF_RemoveOnStealthZeroed, EF_CalculateQRFromTarget,
EF_PromptConfirmationDialog, EF_SequenceOnConfirmation
```

**ORIGINAL-DATA.** There is **no** stacking flag, no stack-count field, no
diminishing-returns flag, no immunity-timer flag and no cap field. The removal
conditions that exist are all *event*-driven (`ClearOnDeath`, `ClearOnDamage`,
`ClearOnRez`, `RemoveOnDisguiseZeroed`, `RemoveOnStealthZeroed`,
`RemoveOnBandolierSlotChange`) or unconditional (`AlwaysPersist`).

This corroborates the independent finding in
[effect-execution-model.md](effect-execution-model.md) that the client
implements no diminishing returns, no immunity timers and no stack caps.

**Immunity is modelled as a moniker, not a timer.** Eight effects exist purely
to test or grant an immunity tag ([query A.21](#a21-stacking-and-immunity-text)):

| Effect | Name / description |
|---|---|
| `849`, `4571`, `4573`, `4668`, `4673`, `4676`, `4840` | `Check for IMMUNITY_Calm moniker` |
| `4575` | `Adds Immunity_Calm moniker to target` |
| `4703` | `Remove Moniker: EFFECT_CoverDebuff` |
| `3056`, `3055` | `Remove Effect of moniker EFFECT_Stance` |

**ORIGINAL-DATA + DESIGNER-TEXT.** The pattern is: a named moniker is applied by
one effect, checked by another, and removed by a third. Mutual exclusion between
stances is implemented the same way — every stance ability (`1451`, `1729`,
`2201`) begins with a `Remove Effect of moniker EFFECT_Stance` effect before
applying its buff. **That is the shipped stacking model: explicit
remove-then-apply, authored per ability.**

### Designer text

**Exactly two statements in the entire database mention stacking**
([query A.21](#a21-stacking-and-immunity-text)):

| Effect | Name | Description |
|---|---|---|
| `3663` | `Interruption Resistance +10%` | `Stacks with Remove Distractions` |
| `3664` | `Interruption Resistance +20%` | `Stacks with Reduce Distractions` |

**DESIGNER-TEXT.** Both are affirmative and both name a *specific* other
ability. **INFERENCE (flagged):** a designer writing "Stacks with X" on two
effects out of 3,216 implies stacking was the *exception* worth documenting, not
the default — but two rows is thin evidence and I would not build a rule on it.

### Not in the data

- **No stacking order.** Nothing states whether additive stat buffs apply before
  or after percentage ones, or whether AF and mitigation compose additively or
  multiplicatively.
- **No caps.** Zero rows mention a stat ceiling, a stack limit, or a maximum
  number of concurrent effects.
- **No diminishing returns.** Zero rows. The word does not appear; effect `4549`
  *Diminish: Concentration* is a Focus-drain debuff, unrelated.
- **No refresh-vs-stack rule.** `effects` has no `SecondaryId`-equivalent column;
  the `SecondaryId`-keyed refresh behaviour documented in
  [effect-execution-model.md](effect-execution-model.md) is a wire-level
  observation, not a content-data one.

### Current Rust behaviour + provenance

**Effect-*instance* stacking: IMPLEMENTED. Stat-*modifier* stacking and ordering:
NOT IMPLEMENTED.**

- `crates/services/src/cell/effects/pulsing/register.rs:88-113` — same
  `effect_id` + same `invoker_id` **refreshes**
  (`existing.remaining_pulses = existing.remaining_pulses.max(remaining)`,
  `next_pulse_at = next_at`); a *different* invoker pushes a new
  `ActiveEffectInstance`. **FAN-GUESS**, self-described: `:91-92` *"This matches
  the Python reference and prevents trivial DoT stacking from a single attacker
  spam-casting the same bleed"*, `:97-98` *"Matches the Python reference's
  'refresh extends, never shortens' rule"*. §6 found nothing in the shipped data
  that speaks to per-invoker keying either way.
- Stun uses the ref-counted `state_flag_counts` path
  (`cell/effects/scripts.rs:448-453`), which is the correct shape for the
  two-source-stun problem.
- `AbsorbShield` caps its pool at `max = 1000` (`scripts.rs:369`) and drains on
  removal (`:390-414`). **FAN-GUESS** for the constant.
- Channel caps: `cell/effects/pulsing/mod.rs` `MAX_CHANNEL_DURATION_SECS = 30.0`
  (for `pulse_count == 0`), `CHANNEL_INTERRUPT_DISTANCE = 0.5`. **FAN-GUESS.**
- **No buff or debuff script modifies a combat stat.** There is no
  additive-vs-multiplicative ordering anywhere in `cell/`. `Stat`
  (`crates/entity/src/stats/stat.rs`) has base and dynamic triples plus
  `change_by_percent`, but nothing computes them from a modifier list. The only
  ordering implementation in the repo is dead code:
  `crates/game/src/combat/stats.rs:99`
  `StatBlock::get = (base + Σflat) * Π multiplier`.
- Consequently the ~300 `+N Accuracy` / `+N CoverDefense` / `+N% Mitigation`
  designer statements catalogued in §1, §2 and §4 have **no runtime effect** —
  there is no mechanism to apply them.
- The moniker-based exclusivity model that §6 identifies as the shipped stacking
  answer (`Remove Effect of moniker EFFECT_Stance` before applying a stance) is
  not modelled. Neither are `EF_ClearOnDamage` / `EF_ClearOnDeath`.
- Default stat ranges to be aware of before wiring any of this up
  (`crates/entity/src/stats/stat_list.rs`): `ACCURACY -1000/0/1000` (`:50`),
  `DEFENSE 0/0/0` (`:51`), `QR_MOD 0/0/0` (`:52`), `MITIGATION 0/0/0` (`:118`),
  `*_AF 0/0/50000` (`:55-58`), `*_RES 0/0/2000` (`:61-63`),
  `DAMAGE -100/0/100` (`:80`), `PENETRATION -100/0/100` (`:81`),
  `ABSORB_* 0/0/1000` (`:134-148`). **`DEFENSE`, `QR_MOD` and `MITIGATION` have
  `max = 0`**, so `set_current` / `change` clamp them to zero and only `update()`
  can move them — a buff that raises Defense would silently do nothing today.

---

## 7. AoE vs cover / line of sight

### Verified data

**Target collection shipped as a 7-value method plus two *symbolic* size bands.**
`ETargetCollectionMethod`: `TCM_None`, `TCM_Single`, `TCM_AERadius`,
`TCM_AECone`, `TCM_Group`, `TCM_Aura`, `TCM_RandomSingle`. Distribution over the
3,216 effects ([query A.22](#a22-target-collection-distribution)):

| Method | `tcm_param1` values (count) |
|---|---|
| `TCM_Single` | (none) 2,559 · Medium 119 · Short 38 · Long 34 · Melee 31 · Extreme 10 · Weapon 3 |
| `TCM_AERadius` | Short 122 · Medium 101 · Melee 60 · Long 12 · Extreme 5 |
| `TCM_AECone` | Weapon 40 · Medium 35 · Short 16 · Melee 6 · Long 3 |
| `TCM_Group` | Medium 5 · Long 4 · Extreme 3 |
| `TCM_Aura` | Medium 5 · Extreme 1 · Long 1 · Short 1 |
| `TCM_RandomSingle` | Short 2 |

**ORIGINAL-DATA.** `tcm_param2` carries the cone angle band: `Narrow` (143),
`Medium` (61), `Wide` (26), `Beam` (8), `Extreme` (1).

The bands are named, not measured. `ETargetCollectionParams` declares the full
16-value vocabulary — `AE_RADIUS_Melee/Short/Medium/Long/Extreme`,
`AE_CONE_Melee/Short/Medium/Long/Extreme`,
`AE_ANGLES_Beam/Narrow/Medium/Wide/Extreme`, `AE_CONE_STARTSIZE` — and, like
every other combat enum, is **bound to no column**. The metres and degrees each
band resolved to are not in the data.

**Line of sight is part of the cover enum.** `ECoverHeight` includes
`HEIGHT_LOS` alongside `HEIGHT_Low/Mid/High`, and 4 cover nodes are tagged with
it (§2). **ORIGINAL-DATA.** This is the only appearance of line-of-sight
anywhere in the resource database.

### Designer text

Essentially nothing. A regex for `line of sight` / `\mLOS\M` across all 3,216
effect names and descriptions returns **zero rows**
([query A.23](#a23-line-of-sight-search)).

The only texts touching AoE geometry modulation are three effects that shrink a
radius or cone as a *debuff on the attacker*: effect `4872` `AoE -30`,
effect `4738` `AoE radius -30`, effect `4708` `Cone -15`, effect `2612`
`Cone Narrow -100`. **DESIGNER-TEXT**, and they say nothing about cover.

One relevant negative from the cover side: ability `808` *Cover Denial*
(`LMG: AOE Medium Radius Attack w/ DOA … Unihabitable Area: 20 seconds`) and
effects `853` / `2822` *Cover Denial AE Damage* (`Medium Radius AE -300F / -30H`)
exist to *deny* cover as a gameplay verb — but neither carries any data linking
the AoE to the cover-node graph.

### Not in the data

- **No rule for whether AoE ignores, respects, or is reduced by cover.** Checked
  every effect and ability description and every column of `effects`,
  `abilities`, `cover_nodes` and `cover_sets`. Nothing connects the two systems.
- **No line-of-sight rule of any kind** beyond the existence of the `HEIGHT_LOS`
  tag on 4 geometry nodes.
- **No numeric AoE radius or cone angle.** Only the symbolic bands above.
- **No falloff.** Nothing states whether AoE damage decays with distance from
  the centre; there is no field that could hold a falloff curve.

### Current Rust behaviour + provenance

**AoE target collection: IMPLEMENTED. Cover/LOS interaction: NOT IMPLEMENTED.**

- Ground-target radius —
  `crates/services/src/cell/abilities/dispatch.rs:106-136`: hostile NPCs only
  (`npc.faction != HOSTILE_FACTION` skipped), same space, alive, 3-D
  `dist_sq <= radius_sq`. Radius from the first effect's `Radius` NVP, default
  `DEFAULT_GROUND_TARGET_RADIUS = 5.0` (`:23`), max range default `30.0`.
  **FAN-GUESS** for the constants — §7 shows the shipped data has only symbolic
  bands, so any metre value is invented. Full damage to every target: **no
  falloff, no LOS, no cover.**
- Cone — `crates/services/src/cell/abilities/cone_aoe/geometry.rs:90-111`:
  planar X/Z, `dist_sq_xz <= length_sq && dot >= cos(half_angle)`. Length and
  half-angle come from `EffectDef::tcm_range_meters` /
  `tcm_half_angle_radians` (`crates/entity/src/abilities/defs.rs:227-262`;
  unknown tier defaults to Medium `8.0 m` / `45°`). **This is the right
  architecture** — it resolves the shipped symbolic bands
  (`Short`/`Medium`/`Long`/`Extreme`/`Weapon` × `Narrow`/`Medium`/`Wide`/`Beam`)
  to metres in one place, which is exactly where the missing original numbers
  would go. The band→metre table itself is **FAN-GUESS**.
  The comment at `geometry.rs:93-97` is honest about the 2-D simplification:
  *"the original game's cones are effectively 2D (cylindrical sections in 3D)…
  pre-navmesh-LOS"*.
- Each cone secondary rolls its own QR and takes full damage
  (`cone_aoe/fan_out.rs:119-146`). PvE only (`all_npc_entity_ids`).
- **LOS exists but is wired only to NPC AI.**
  `crates/services/src/cell/space_manager/spatial.rs:15-37`
  `has_line_of_sight` → `navmesh.raycast`, returning `true` when there is no
  navmesh/space/position. Called from exactly two places, both AI:
  `cell/service/npc_ai/fight.rs:241` and `:354`. Player `useAbility` and every
  AoE path bypass it. **NOT IMPLEMENTED** for combat.

The shipped data (§7) has no AoE-vs-cover rule to port, so this gap cannot be
closed by reading the resource DB. It needs either pcap evidence or a design
decision recorded as a deliberate divergence.

---

## Appendix A — extraction queries

All run read-only against the seeded resource database:

```bash
PGPASSWORD=w-testing psql -h localhost -p 5433 -U w-testing -d <resources-db>
```

### A.1 Enum usage

```sql
SELECT t.typname AS enum_type,
       coalesce(c.table_name||'.'||c.column_name,'<<UNREFERENCED>>') AS used_by
FROM pg_type t
JOIN pg_namespace n ON n.oid = t.typnamespace
LEFT JOIN information_schema.columns c
       ON c.udt_name = t.typname AND c.table_schema = 'resources'
WHERE n.nspname = 'resources'
  AND t.typname IN ('EDamageType','EMitigationType','EStats','EStatType',
                    'EStatLevel','EStatResultCode','EStatValueType',
                    'ECombatantState','ECoverHeight','ECoverQuality',
                    'EStateField','ETargetCollectionParams','EWeaponType',
                    'EWeaponRange','EItemType')
ORDER BY 1, 2;
```

### A.2 Character-schema stat columns

```sql
SELECT table_name, column_name, data_type
FROM information_schema.columns
WHERE table_schema = 'public'
  AND column_name ~* '(stat|accur|defen|armor|mitig|penetr|resist|cover|focus|health)'
ORDER BY table_name, ordinal_position;
-- returns only sgw_mission.status and sgw_player.state_field
```

### A.3 `effect_nvps` full dump

```sql
SELECT n.nvp_id, n.effect_id, n.name, n.value,
       e.name AS effect_name, e.effect_desc, e.ability_id
FROM resources.effect_nvps n
JOIN resources.effects e USING (effect_id)
ORDER BY n.effect_id, n.name;
```

Original vs. Cimmeria-added split:

```bash
git show 4b8dea9f:db/resources/Effects/Seed/effect_nvps.sql | grep -c INSERT   # 17
git log --oneline -- db/resources/Effects/Seed/effect_nvps.sql
```

### A.4 Effect flag decode

```sql
SELECT (flags & 16)      <> 0 AS ef_dontuseqr,            count(*) FROM resources.effects GROUP BY 1;
SELECT (flags & 4194304) <> 0 AS ef_calculateqrfromtarget, count(*) FROM resources.effects GROUP BY 1;
SELECT effect_id, ability_id, name, effect_desc, flags
FROM resources.effects WHERE (flags & 4194304) <> 0 ORDER BY effect_id;
```

### A.5 `EF_DontUseQR` vs damage

```sql
SELECT (flags & 16) <> 0 AS dontuseqr,
       (effect_desc ~* '(-[0-9]+ *[FH]|damage|dot|dd)') AS looks_damage,
       count(*)
FROM resources.effects GROUP BY 1, 2 ORDER BY 1, 2;
```

### A.6 Accuracy/Defense statements

```sql
SELECT regexp_replace(replace(replace(effect_desc,chr(13),' '),chr(10),' '),'\s+',' ','g') AS d,
       count(*) AS n, min(effect_id) AS example, string_agg(DISTINCT flags::text,',') AS flags
FROM resources.effects
WHERE effect_desc ~* '([+-] ?[0-9]+ ?%? ?(Accuracy|ACC|Defense|DEF)\M)'
GROUP BY 1 ORDER BY n DESC, example;
```

### A.7 Point↔percent pairs

```sql
SELECT effect_id, ability_id, name, effect_desc
FROM resources.effects
WHERE effect_desc ~ '\([0-9.]+ *%' OR effect_desc ~ '% *\)'
ORDER BY effect_id;
-- returns exactly 2 rows: 2004 and 2005
```

### A.8 Cover-node distribution

```sql
SELECT height, quality, count(*) FROM resources.cover_nodes GROUP BY 1,2 ORDER BY 1,2;
SELECT count(*) AS nodes, count(DISTINCT chunk_id) AS chunks,
       min(length(tail)) AS tail_min, max(length(tail)) AS tail_max
FROM resources.cover_nodes;
```

### A.9 Cover `tail` blob

```sql
SELECT encode(tail,'hex') AS tail_hex, count(*)
FROM resources.cover_nodes GROUP BY 1 ORDER BY 2 DESC LIMIT 20;
-- decode as little-endian f32: db0fc9bf = -1.5708, 490e49c0 = -3.1416, ...
```

### A.10 Cover ability→effect chains

```sql
SELECT ability_id, name, description, effect_ids
FROM resources.abilities
WHERE ability_id IN (1450,1451,1452,1454,1487,1729,2130,2201,637,847,1642);

SELECT effect_id, ability_id, name, effect_desc, flags, target_collection_method
FROM resources.effects
WHERE ability_id IN (1450,1451,1452,1454,1487,1729,2130,2201,637,847,1642)
ORDER BY ability_id, effect_sequence;
```

### A.11 Cover and QR text

```sql
SELECT effect_id, name, effect_desc, flags
FROM resources.effects
WHERE (name||' '||effect_desc) ~* '(cover|crouch|\mqr\M|quality rating)'
ORDER BY effect_id;

SELECT ability_id, name, description
FROM resources.abilities
WHERE (name||' '||description) ~* '(cover|crouch|penetrat|accuracy|\mACC\M|defense|\mDEF\M|tech comp|tier|mitigat)'
ORDER BY ability_id;
```

### A.12 Moniker dump

```sql
SELECT count(*) FROM resources.monikers;                        -- 53
SELECT moniker_id, name, coalesce(description,'') FROM resources.monikers ORDER BY name;
SELECT count(*) AS total,
       count(*) FILTER (WHERE moniker_ids <> '{}') AS with_monikers,
       count(*) FILTER (WHERE description ~ '[0-9]') AS desc_has_digit
FROM resources.items;
```

### A.13 Weapon tier × quality × tech_comp

```sql
SELECT tier, quality_id, count(*) AS n,
       min(tech_comp) AS tc_min, max(tech_comp) AS tc_max,
       round(avg(tech_comp)::numeric,1) AS tc_avg,
       min(clip_size) AS clip_min, max(clip_size) AS clip_max,
       min(max_ranged_range) AS r_min, max(max_ranged_range) AS r_max
FROM resources.items
WHERE 3901383057 = ANY(moniker_ids)   -- CATEGORY_Weapons
GROUP BY tier, quality_id ORDER BY tier, quality_id;
```

Sample rows:

```sql
SELECT item_id, name, tier, quality_id, tech_comp, applied_science_id,
       clip_size, max_ranged_range, default_ammo_type
FROM resources.items
WHERE 3901383057 = ANY(moniker_ids) AND clip_size > 0
ORDER BY tier, tech_comp, item_id LIMIT 14;
```

| item_id | name | tier | quality | tech_comp | sci | clip | range | ammo |
|---|---|---|---|---|---|---|---|---|
| 21 | SGHC 6 SMG | 1 | Normal | 1 | 3 | 30 | 30 | Bullet_Default |
| 55 | SI 3 9mm Pistol | 1 | Normal | 1 | 1 | 15 | 20 | Bullet_Default |
| 3126 | SGHC 6 SMG | 1 | Normal | 3 | 3 | 30 | 30 | Bullet_Default |
| 3260 | SK37 LMG | 1 | Normal | 3 | 3 | 250 | 30 | Bullet_Default |
| 3299 | Nenz 24 Rifle | 1 | Normal | 3 | 4 | 30 | 40 | Bullet_Default |
| 3145 | AR 21 Assault Rifle | 1 | Normal | 5 | 4 | 30 | 30 | Bullet_Default |
| 3173 | DMR 12 Assault Rifle | 1 | Normal | 5 | 4 | 30 | 30 | Bullet_Default |
| 3217 | Kotchner V17 SMG | 1 | Normal | 5 | 3 | 30 | 30 | Bullet_Default |

Note the duplicate names at differing `tech_comp` (SGHC 6 SMG at tc 1, 3 and 5)
with **identical** clip size and range — the only thing that varies between those
three rows is `tech_comp`.

### A.14 Quality distribution

```sql
SELECT quality_id, count(*) FROM resources.items GROUP BY 1 ORDER BY 2 DESC;
SELECT item_id, name, tier, quality_id, tech_comp
FROM resources.items WHERE quality_id <> 'ITEM_QUALITY_Normal'
ORDER BY quality_id, tier;
```

### A.15 Discipline tech competency

```sql
SELECT "row", count(*) AS n, string_agg(DISTINCT tech_competency::text, ',') AS tc
FROM resources.disciplines GROUP BY 1 ORDER BY 1;

SELECT applied_science_id, count(*) AS n,
       min(tech_competency) AS tc_min, max(tech_competency) AS tc_max,
       min(racial_paradigm_level) AS rpl_min, max(racial_paradigm_level) AS rpl_max
FROM resources.disciplines GROUP BY 1 ORDER BY 1;

SELECT id, name FROM resources.applied_science ORDER BY id;
```

### A.16 Resist effect families

```sql
SELECT lower(regexp_replace(name,'\s+',' ','g')) AS nm, count(*) AS n,
       string_agg(DISTINCT flags::text, ',') AS flagset, min(effect_id) AS example
FROM resources.effects WHERE name ~* 'resist'
GROUP BY 1 ORDER BY n DESC;
```

### A.17 Mitigation and penetration text

```sql
SELECT effect_id, name, effect_desc
FROM resources.effects
WHERE (name||' '||effect_desc) ~* '(penetrat|mitigat|negation|absorb|\mAF\M|armor factor)'
ORDER BY effect_id;
```

### A.18 Damage-subtype word search

```sql
SELECT w.k, count(*) AS n, min(e.effect_id) AS example
FROM (VALUES ('fire'),('flame'),('incendiary'),('ice'),('cold'),('frost'),
             ('thermal'),('radiation'),('plasma'),('sonic'),('electrical'),
             ('particle'),('biological'),('chemical'),('impact'),('concussive'),
             ('slashing'),('piercing'),('contamination'),('hazmat'),('psionic'),
             ('untyped'),('acid'),('poison'),('disease')) w(k)
JOIN resources.effects e ON (e.name||' '||e.effect_desc) ~* ('\m'||w.k)
GROUP BY w.k ORDER BY n DESC;
-- repeat against resources.abilities (name||' '||description)
```

Word-boundary (`\m`) is essential: an unanchored `ice` matches *Device*,
*Service* and *Practice* and reports 17 false positives.

### A.19 Archetype base stats

```sql
SELECT * FROM resources.archetypes ORDER BY archetype;
```

### A.20 Focus effects

```sql
SELECT effect_id, ability_id, name, effect_desc, flags
FROM resources.effects
WHERE (name||' '||effect_desc) ~* '(focus check|% *focus|focus (roll|gate|threshold))'
ORDER BY effect_id;
```

### A.21 Stacking and immunity text

```sql
SELECT effect_id, name, effect_desc
FROM resources.effects
WHERE (name||' '||effect_desc) ~* '(stack|diminish|immun|refresh|cap\M)'
ORDER BY effect_id;
```

### A.22 Target-collection distribution

```sql
SELECT target_collection_method, tcm_param1, count(*)
FROM resources.effects GROUP BY 1,2 ORDER BY 1,3 DESC;

SELECT tcm_param1, tcm_param2, count(*)
FROM resources.effects GROUP BY 1,2 ORDER BY 3 DESC;
```

### A.23 Line-of-sight search

```sql
SELECT effect_id, name, effect_desc
FROM resources.effects
WHERE (name||' '||effect_desc) ~* '(line of sight|\mlos\M)';
-- 0 rows
```

### A.24 Resist-roll sequence pairing

Tests whether every resist roll shares an `effect_sequence` step with at least
one non-roll effect on the same ability. Result: **79 of 79**.

```sql
WITH r AS (
  SELECT ability_id, effect_sequence
  FROM resources.effects
  WHERE name ~* 'resist (roll|check)'
)
SELECT count(*) AS roll_rows,
       count(*) FILTER (WHERE peers > 0) AS rolls_sharing_seq_with_another_effect
FROM (
  SELECT r.*,
         (SELECT count(*) FROM resources.effects e
          WHERE e.ability_id = r.ability_id
            AND e.effect_sequence = r.effect_sequence
            AND e.name !~* 'resist (roll|check)') AS peers
  FROM r
) x;
```

Per-step view:

```sql
SELECT e.ability_id, e.effect_sequence,
       string_agg(e.name, ' + ' ORDER BY e.effect_id) AS effects_in_step
FROM resources.effects e
WHERE e.ability_id IN (SELECT ability_id FROM resources.effects WHERE name ~* 'resist roll')
GROUP BY 1, 2 ORDER BY 1, 2;
```

### A.25 Resist type → CC family mapping

```sql
WITH r AS (
  SELECT ability_id, effect_sequence,
         CASE WHEN name ~* 'kinetic' THEN 'kineticRes'
              WHEN name ~* 'mental'  THEN 'mentalRes'
              WHEN name ~* 'health'  THEN 'healthRes' END AS roll
  FROM resources.effects
  WHERE name ~* 'resist (roll|check)'
)
SELECT r.roll,
       lower(regexp_replace(regexp_replace(p.name,'[:;].*$',''),'\s+',' ','g')) AS paired_effect,
       count(*) AS n
FROM r
JOIN resources.effects p
  ON p.ability_id = r.ability_id
 AND p.effect_sequence = r.effect_sequence
 AND p.name !~* 'resist (roll|check)'
GROUP BY 1, 2 HAVING count(*) >= 2
ORDER BY 1, n DESC;
```

### A.26 Canonical numeric enum values

The `.sql` enum files carry **no** numbers. Read the wire values from the
canonical defs instead:

```bash
sed -n '272,281p' entities/defs/enumerations.xml   # EDamageType  (13,14,15,16,18)
sed -n '404,425p' entities/defs/enumerations.xml   # EMitigationType (1..15)
sed -n '468,554p' entities/defs/enumerations.xml   # EStats (sparse, 0..111)
```

---

## Appendix B — the shipped `EStats` enumeration

**The SQL enum's declaration order is NOT the stat index.**
`db/resources/Combat/Types/EStats.sql` is a Postgres `CREATE TYPE … AS ENUM`
and carries no numbers. The canonical wire indices are in
`entities/defs/enumerations.xml:468-554` (`<Type>UINT8</Type>`), and they are
**sparse**. Anyone reading the `.sql` file alone will derive wrong indices.
**ORIGINAL-DATA.**

82 named stats, ids 0–111:

| id | Stat | id | Stat |
|---|---|---|---|
| 0 | `coordination` | 62 | `tracking` |
| 1 | `engagement` | 63 | `stabilization` |
| 2 | `fortitude` | 64 | `awareness` |
| 3 | `morale` | 65 | `interruptRes` |
| 4 | `perception` | 66 | `coverAccuracy` |
| 5 | `intelligence` | 67 | `coverDefense` |
| 6 | `movementSpeedMod` | 68 | `crouchingAccuracy` |
| 7 | `health` | 69 | `crouchingDefense` |
| 8 | `focus` | 70 | `stealthMovement` |
| 9 | `healthRegen` | 71 | `revealRating` |
| 10 | `focusRegen` | 72 | `negation` |
| 11 | `accuracy` | 73 | `PhysicalDamagePercent` |
| 12 | `defense` | 74 | `EnergyDamagePercent` |
| 13 | `qrMod` | 75 | `HazmatDamagePercent` |
| **18** | `physicalAF` | 76 | `PsionicDamagePercent` |
| **23** | `energyAF` | 77 | `UntypedDamagePercent` |
| **24** | `hazmatAF` | 78 | `disguiseRating` |
| **28** | `psionicAF` | 79 | `disguiseDetection` |
| **29** | `kineticRes` | 80 | `mitigation` |
| **34** | `mentalRes` | 81 | `rotationSpeedMod` |
| **40** | `healthRes` | 82 | `energy` |
| 46 | `stealthRating` | 83 | `energyRegen` |
| 47 | `rangeModifier` | 89–93 | `absorbPhysical/Energy/Hazmat/Psionic/Untyped` |
| 48 | `coverQRModifier` | 94–98 | `absorb*Item` (same five) |
| 49–53 | `ammoSlot1`…`ammoSlot5` | 99–103 | `absorb*Energy` (same five) |
| 54 | `deploymentBarAmmo` | 104 | `speedReload` |
| 55 | `response` | 105 | `speedGrenade` |
| 56 | `damage` | 106 | `speedDeploy` |
| 57 | `penetration` | 107 | `speedAttack` |
| 58–61 | `physicalDensity`, `energyDensity`, `hazmatDensity`, `psionicDensity` | 108 | `recovery` |
| | | 109 | `restoration` |
| | | 110 | `subtlety` |
| | | 111 | `speedPet` |

**30 ids are reserved and unnamed**, in seven runs:
**14–17** (after `qrMod`), **19–22** (after `physicalAF`), **25–27** (between
`hazmatAF` and `psionicAF`), **30–33** (after `kineticRes`), **35–39** (after
`mentalRes`), **41–45** (after `healthRes`), **84–88** (between `energyRegen`
and `absorbPhysical`). **ORIGINAL-DATA.**

Every reserved run sits immediately after an armor-factor or resistance stat,
which is suggestive — but the run lengths (4, 4, 3, 4, 5, 5, 5) do not match the
`EMitigationType` subtype counts (4 Physical, 4 Energy, 4 Environmental), so the
"reserved for damage subtypes" reading does **not** survive contact with the
numbers. **The data does not say what these 30 slots were for.**

Stats named here that appear in **no** designer text anywhere in the content
database: `qrMod`, `coverQRModifier`, all four `*Density` entries,
`stabilization`, `restoration`, `rangeModifier`, `response` (one mention),
all fifteen `absorb*` entries, and all five `*DamagePercent` entries.

### `EMitigationType` numeric values

`entities/defs/enumerations.xml:404-425` (`UINT8`) — the subtype hierarchy with
its real wire values. **ORIGINAL-DATA.**

| Value | Token | Value | Token | Value | Token |
|---|---|---|---|---|---|
| 1 | `MITIGATION_Physical` | 6 | `MITIGATION_Energy` | 11 | `MITIGATION_Environmental` |
| 2 | `_Physical_Impact` | 7 | `_Energy_Plasma` | 12 | `_Enviro_Biological` |
| 3 | `_Physical_Concussive` | 8 | `_Energy_Radiation` | 13 | `_Enviro_Chemical` |
| 4 | `_Physical_Slashing` | 9 | `_Energy_Electrical` | 14 | `_Enviro_Thermal` |
| 5 | `_Physical_Piercing` | 10 | `_Energy_Particle` | 15 | `_Enviro_Cold` |

`_Enviro_Thermal` (14) and `_Enviro_Cold` (15) are the closest thing to "fire"
and "ice" that shipped. No `Sonic` token exists at any level.
