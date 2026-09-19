---
name: shipped-data-combat-evidence
description: What the ORIGINAL shipped SGW resource DB does and does not contain for combat formulas — stat taxonomy shipped, stat values did not; the 17-row effect_nvps; EF_DontUseQR as the QR gate; cover carries no numbers
metadata:
  type: project
---

Mined 2026-09-18 for the combat-formula evidence survey. Full writeup:
`docs/reverse-engineering/findings/combat-formulas-game-data-evidence.md`.

## The bounding fact

**The shipped resource DB contains the combat stat *taxonomy* and almost none of
the stat *values*.** `EStats` (82 entries), `EDamageType` (5), `EMitigationType`
(15), `EStatResultCode`, `ETargetCollectionParams`, `EStateField` are all
declared in the dump and **bound to no column in any table**. Only
`ECoverHeight` / `ECoverQuality` (on `cover_nodes`) are actually used.
`public.sgw_player` persists no combat stat either — combat stats were derived
at runtime from level + archetype + disciplines + equipment and never stored.

**Why:** stops any future "surely the numbers are in the DB somewhere" search.
**How to apply:** when asked where an original combat number lives, the answer is
almost always "nowhere in the shipped data" — say so instead of hunting.

## The only numeric combat payload: 17 rows

`resources.effect_nvps` has 21 rows, of which **17 are original** (`nvp_id` 1–17,
present at commit `4b8dea9f`) and 4 are Cimmeria-authored (18–19 via PR #493,
100 via #496, 200 via #497). Three names only: `HealthDamage`, `FocusDamage`,
`HealPercentage`. Nine effects out of 3,216 carry a number.

`effects.script_name` is **NOT original evidence** — all 16 non-NULL values are
Cimmeria's own `EffectScript` registry keys (`cell/effects/registry.rs`).

## Confirmed original relationships worth reusing

- **FocusDamage : HealthDamage = 10 : 1** in 7 of 8 original damage effects
  (641, 646, 654, 621, 3091, 3834, 4467). The outlier is 264 `Energy Blast`
  at 5:1 — the only `RangedEnergyDamage` entry.
- **`EF_DontUseQR` (bit 4, value 16) is the QR gate.** Set on 754 of 3,216
  effects. Damage effects have it clear (flags 0); buffs/debuffs have it set.
  717 damage-looking effects roll QR, only 25 skip it.
- **`EF_CalculateQRFromTarget` (bit 22, value 4194304)** on 39 effects — grenade
  AoE damage, CC (knockdown/stun/snare/disorient), and the resist-roll effects.
- **Resistance is a separate QR roll, not flat reduction.** 188 resist-named
  effects; the big families are literally `Kinetic Resist Roll` (26),
  `Mental Resist Roll` (26), `Health Resist Roll` (19). They sit as their own
  entries in an ability's effect chain and gate what follows.
- **"Cover Penetration" == the `coverAccuracy` stat.** Proven by ability→effect
  linkage: ability 1450 "Cover Penetration / +100 Cover Penetration" invokes
  effect 1741 "+100 CoverAccuracy". Same for 2130→2895. There is no
  `coverPenetration` entry in `EStats`.
- **Cover is denominated in QR.** Ability 1487 "Ignores 2 QR of Cover";
  effect 4995 "+1 QR Cover Penetration". Matches the `coverQRModifier` stat.
- **10 stat points = 1 percentage point**, stated exactly twice and nowhere else:
  effect 2004 `+50 (5%) Mental Resist`, effect 2005 `Subtlety -100 (10% increase
  to threat)`. n=2 — do not over-build on it.
- **Archetype base stats are undifferentiated**: all nine `resources.archetypes`
  rows are health 760 / focus 1570 / healthPerLevel 10 / focusPerLevel 70. Only
  Commando has distinct attributes.

## Clean negatives (checked, absent)

- No weapon damage/accuracy/penetration anywhere. `resources.items` has 23
  columns, none a combat stat; `resources.monikers` (53 rows) are pure category
  tags with zero stat payload.
- No numeric bonus on cover nodes. The unexplained 4-byte `tail` decodes as a
  little-endian f32 angle (−π/2, −π, π/2, −3π/2), not a bonus.
- Fire / Ice / Radiation / Plasma / Sonic do **not** exist as damage types.
  Word-boundary search: `ice`, `cold`, `frost`, `thermal`, `sonic`, `acid` = 0
  hits in both effects and abilities. "fire" is the verb (Cover Fire) or ability
  flavour. Use `\m` word boundaries — unanchored `ice` matches *Device*.
- Zero line-of-sight mentions in 3,216 effects. The only LOS artefact anywhere is
  the `HEIGHT_LOS` value of `ECoverHeight` on 4 cover nodes.
- No stacking order, no caps, no diminishing returns, no immunity timers.
  Exactly two rows mention stacking (3663, 3664, both "Stacks with <named
  ability>"). Immunity is a **moniker** (`IMMUNITY_Calm`), applied/checked/removed
  by dedicated effects. Stance exclusivity is authored as an explicit
  "Remove Effect of moniker EFFECT_Stance" effect before the buff.
- AoE sizes are symbolic bands only (`Short`/`Medium`/`Long`/`Extreme`/`Weapon`
  radius; `Narrow`/`Medium`/`Wide`/`Beam` angle). No metres, no degrees, no
  falloff field.
- Item `quality_id` is effectively unused: 5,907 of 6,059 items are Normal;
  1,435 of 1,436 weapons are Normal.

## Query access

Resource DBs live on `localhost:5433` (user/pass `w-testing`), one per worktree.
psql at `external\postgresql_server\bin\psql.exe`. **Read-only** — other sessions
share the server; never reload or drop.
