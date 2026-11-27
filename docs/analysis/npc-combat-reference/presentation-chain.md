# NPC presentation chain for the reference NPCs (NC-04)

> **Packet**: NC-04, wave 0. Ledger: [README.md](README.md). Rules and contract:
> [work-packets.md](work-packets.md).
> **Scope**: the six abilities and six effects that the four reference NPCs
> use (ScavDog, Jaffa, PRU, NID), traced through the seed to the sequences
> the client plays. Seed-only, no client read.
> **Method**: rows read from `db/resources/Abilities/Seed/`,
> `db/resources/Effects/Seed/` and `db/resources/Events/Seed/`, then each
> ability link cross-checked against
> [`ability-animation-links.md`](../../reverse-engineering/findings/ability-animation-links.md).
> Line numbers are file lines at commit `3a20a96a0`.

## How the chain works

An ability plays its presentation through `abilities.event_set_id`. The set
needs an `Ability_End` sequence (event 1001) to animate at all. An effect
names a target set through `effects.event_set_id`; its sequences are events
2002-2006. The server reads only the ability's set (`use_ability/sequence.rs`
sends events 1000, 1001 and 1002), and nothing reads
`effects.event_set_id`, so whether the client plays the target set is
UNRESOLVED (NC-06).
Each sequence names a Kismet script (`sequences.kismet_script_name`), and
some sequences carry NVPs (`sequences_nvp`). The client resolves the set
by id and never reads the ability (animation-links.md, "The client has no
ability-keyed lookup").

Two patterns repeat in this data:

- every ability sits on a **source** set (3, 15, 300, 802) and every effect
  on a **target** set (262, 289, 301, 803). The four source sets each carry
  an `Ability_End` (1001) sequence; the four target sets carry none;
- every sequence in these chains names `KIS-abilities_human.*`. The client
  package names are generic per mechanic, so this is a package name and not
  evidence of a family (see § Gaps).

## Evidence classes used here

| Link | Class | Source |
|---|---|---|
| Ability to set, Giza seed (584→3, 559→15, 710→300, 221→802) | EARLIEST LEGACY DATA (present before PR #822, which did not touch these rows; 584, 559 and 710 match the family rule, animation-links.md:35-37) | `abilities.sql`; commit `f153138b5` changes none of them |
| Ability to set, by judgement (1433→300, 983→300) | RECONSTRUCTED (PR #822 Tier 3; both NULL before) | animation-links.md:74 |
| Effect to set (all six) | EARLIEST LEGACY DATA | seed only; the doc covers ability links and grenade blast sets (:73) |
| Kismet package names (`KIS-SA_*`, `KIS-abilities_human`) | ORIGINAL CLIENT VERIFIED (existence, cited second-hand from animation-links.md:23; this packet read no client file) | animation-links.md, "The client has no ability-keyed lookup" (CLIENT-PROVEN) |
| Sequence-to-set pairing and NVPs | EARLIEST LEGACY DATA | seed rows (`event_sets_sequences`, `sequences`, `sequences_nvp`) |
| Effect-set playback (does the client play 2002-2006?) | UNRESOLVED | no seed, server or RE source; NC-06 |

## Ability and effect chains

Sequences in each table are in event-id order (the file order differs). The
Kismet column drops the `KIS-abilities_human.` prefix; the real
`kismet_script_name` is that prefix plus the shown name, as in the first row
of set 3. "Line" gives the seed file line for the row.

### Ability 584 Staff Auto Attack (Jaffa ranged staff; set 4)

Ability row `abilities.sql:1452`: `ABILITY_TYPE_DD`, cooldown 3, ranged,
`max_range` 0, moniker `ITEM_Staff`, `event_set_id` 3, effects `{646}`.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 3 | ability source (`Generic single-shot weapon use`) | 2182 | 1000 | `KIS-SA_Sing_Source` | - | sequences.sql:1687 |
| 3 | | 2617 | 1004 | `KIS-SA_Sing_Source` | - | sequences.sql:2305 |
| 3 | | 2619 | 1006 | `KIS-SA_Sing_Source` | - | sequences.sql:2309 |
| 3 | | 2618 | 1007 | `KIS-SA_Sing_Source` | - | sequences.sql:2307 |
| 3 | | 3 | 1001 (Ability_End) | `KIS-SA_Sing_Source` | - | sequences.sql:3063 |
| 3 | | 2969 | 1002 | `KIS-SA_Effect_Source` | `sfx1=abil_hum/statecast/interupt` | sequences.sql:3001, sequences_nvp.sql:3665 |

Effect 646 `effects.sql:1243`: name `Staff Auto Attack Damage`, desc `Single Target -250F -25H`, script
`RangedPhysicalDamage`, `event_set_id` 262 (`Generic single-shot weapon target`).
NVPs `HealthDamage` 25 and `FocusDamage` 250 (`effect_nvps.sql:25, 27`).

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 262 | effect target | 263 | 2002 | `KIS-SA_Sing_Target` | - | sequences.sql:2331 |
| 262 | | 309 | 2003 | `KIS-SA_Sing_Target` | - | sequences.sql:3125 |
| 262 | | 310 | 2004 | `KIS-SA_Sing_Target` | - | sequences.sql:3127 |
| 262 | | 311 | 2005 | `KIS-SA_Sing_Target` | - | sequences.sql:3129 |
| 262 | | 312 | 2006 | `KIS-SA_Sing_Target` | - | sequences.sql:3131 |
| 262 | | 1805 | 2008 | `KIS-SA_Sing_Source` (a source in a target set) | - | sequences.sql:1229 |

### Ability 559 Automatic Weapon Auto Attack (SMG and AR ranged; NID set)

Ability row `abilities.sql:5252`: `ABILITY_TYPE_DD`, cooldown 1.5, ranged,
moniker `ITEM_Automatic_Weapon`, `event_set_id` 15, effects `{5310}` (was `{621}` until NC-25),
`required_ammo` 3.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 15 | ability source (`Generic burst weapon use (channelable)`) | 2608 | 1000 | `KIS-SA_Charge_Source` | - | sequences.sql:2287 |
| 15 | | 2606 | 1004 | `KIS-SA_Channel_Source` | - | sequences.sql:2283 |
| 15 | | 2616 | 1006 | `KIS-SA_Channel_Source` | - | sequences.sql:2303 |
| 15 | | 2607 | 1007 | `KIS-SA_Channel_Source` | - | sequences.sql:2285 |
| 15 | | 15 | 1001 (Ability_End) | `KIS-SA_Burst_Source` | - | sequences.sql:785 |
| 15 | | 2963 | 1003 | `KIS-SA_Effect_Source` | `sfx1=abil_hum/statecast/interupt` | sequences.sql:2989, sequences_nvp.sql:3655 |

Effect 5310 `effects.sql:7824`: name `AW Auto Attack Damage`, desc `Single Target -100F -10H`, script
`RangedPhysicalDamage`, `event_set_id` 289 (`Generic burst weapon target`).
NVPs `HealthDamage` 10 and `FocusDamage` 100 (`effect_nvps.sql:1667, 1668`),
RECONSTRUCTED from the 559 tooltip (D-NC8, D-NC11, NC-25). Giza effect 621
(`effects.sql:7817`, -200F/-20H, NVPs 20/200 at `effect_nvps.sql:37, 39`) is kept
unchanged and no ability uses it.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 289 | effect target | 313 | 2002 | `KIS-SA_Burst_Target` | - | sequences.sql:3133 |
| 289 | | 314 | 2003 | `KIS-SA_Burst_Target` | - | sequences.sql:3135 |
| 289 | | 315 | 2004 | `KIS-SA_Burst_Target` | - | sequences.sql:3137 |
| 289 | | 316 | 2005 | `KIS-SA_Burst_Target` | - | sequences.sql:3139 |
| 289 | | 317 | 2006 | `KIS-SA_Burst_Target` | - | sequences.sql:3141 |
| 289 | | 1105 | 2008 | `KIS-SA_Burst_Source` (a source in a target set) | - | sequences.sql:211 |

### Ability 710 Staff Melee AA (Jaffa melee half; set 4)

Ability row `abilities.sql:5119`: `ABILITY_TYPE_DD`, cooldown 2, melee,
moniker `ITEM_Staff`, `event_set_id` 300 (`Generic melee weapon source`),
effects `{736}`. It sits in set 4 beside 584.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 300 | ability source | 2959 | 1002 | `KIS-SA_Effect_Source` | `sfx1=abil_hum/statecast/interupt` | sequences.sql:2981, sequences_nvp.sql:3647 |
| 300 | | 346 | 1001 (Ability_End) | `KIS-SA_Melee_Source` | - | sequences.sql:3169 |

Effect 736 `effects.sql:7522`: name `Strike Damage`, desc `-100F -10H`,
**no script**, `event_set_id` 301
(`Generic melee weapon target`), **no NVP rows**.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 301 | effect target | 348 | 2002 | `KIS-SA_Melee_Target` | - | sequences.sql:3171 |
| 301 | | 349 | 2003 | `KIS-SA_Melee_Target` | - | sequences.sql:3173 |
| 301 | | 350 | 2004 | `KIS-SA_Melee_Target` | - | sequences.sql:3175 |
| 301 | | 351 | 2005 | `KIS-SA_Melee_Target` | - | sequences.sql:3177 |
| 301 | | 352 | 2006 | `KIS-SA_Melee_Target` | - | sequences.sql:3179 |

The effect names a complete target set but deals no damage (no NVP, no
script). Whether the target set plays at all is UNRESOLVED (the server never
sends it). See § Gaps and NC-12.

### Ability 1433 Mob Strike (ScavDog melee; set 6 is proposed by NC-13)

Ability row `abilities.sql:2047`: `ABILITY_TYPE_DD`, cooldown 2, melee,
`max_range` 0, `moniker_ids` `{1470900795}`, `event_set_id` 300, effects `{1717}`.
No `ability_set_abilities` row names it, so no set holds it today.
`moniker_ids` is `{1470900795}`, which has no `monikers` row (the only other
use is test item 2143 'test 2', items.sql:12437), and `item_monikers` is `{}`,
so the weapon-family rule has no input.

The sequences are the same set 300 as the Jaffa melee ability 710
(table above). Only the link ability to set is RECONSTRUCTED (PR #822 Tier 3), from
animation-links.md:74.

Effect 1717 `effects.sql:3012`: name `Strike Damage`, desc `-125F -15H`,
**no script**, `event_set_id` 301, **no NVP rows**. Its target sequences are
the set 301 rows above (348-352). Effect and ability sets differ: the
ability is on 300 and the effect on 301, which is the source/target pair.

### Ability 983 Bite (ScavDog natural melee; unassigned)

Ability row `abilities.sql:5036`: `ABILITY_TYPE_DD`, cooldown 3, melee,
`moniker_ids` `{1470900795}`, `event_set_id` 300, effects `{1092}`. No
`ability_set_abilities` row names it. Like 1433, it is RECONSTRUCTED (PR #822 Tier 3) at
set 300 (animation-links.md:74, "creature bodies need a playtest").

Effect 1092 `effects.sql:2471`: name `Bite Damage`, desc `-125F -15H`,
**`event_set_id` NULL**, **`script_name` NULL**, **no NVP rows**. The chain
ends at the ability: the server sends set 300's End sequence (346), and the
effect names no target set.

### Ability 221 Energy Shock (PRU attack; ability set 2 and Straegis set 350)

Ability row `abilities.sql:5078`: `ABILITY_TYPE_DD`, cooldown 2, ranged,
`max_range` 0, no moniker, `event_set_id` 802, effects `{264}`. Set 2
(`ability_set_abilities.sql`, the PRU set) and set 350 (Straegis pet) both
hold it.

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 802 | ability source (`Energy shock ability source`) | 1866 | 1001 (Ability_End) | `KIS-SA_Beam_Source` | `beamWeapon=EnergyShock` | sequences.sql:1269, sequences_nvp.sql:1139 |
| 802 | | 2894 | 1002 | `KIS-SA_Effect_Source` | `sfx1=abil_hum/statecast/interupt` | sequences.sql:2851, sequences_nvp.sql:3515 |

Effect 264 `effects.sql:7813`: name `Energy Blast Damage`, desc `Energy Blast Effect`, script
`RangedEnergyDamage`, `event_set_id` 803 (`Energy shock effect target`).
NVPs `HealthDamage` 16 and `FocusDamage` 80 (`effect_nvps.sql:33, 35`).

| Set | Role | Sequence | Event | Kismet | NVP | Line |
|---|---|---|---|---|---|---|
| 803 | effect target | 1867 | 2002 | `KIS-SA_Sing_Target` | `hitDamage=EnergyShock` | sequences.sql:1271, sequences_nvp.sql:1141 |
| 803 | | 1868 | 2003 | `KIS-SA_Sing_Target` | `hitDamage=EnergyShock` | sequences.sql:1273, sequences_nvp.sql:1143 |
| 803 | | 1869 | 2004 | `KIS-SA_Beam_Source` (a source in a target set) | `beamWeapon=EnergyShock` | sequences.sql:1275, sequences_nvp.sql:1145 |
| 803 | | 1870 | 2005 | `KIS-SA_Sing_Target` | `hitDamage=EnergyShock` | sequences.sql:1277, sequences_nvp.sql:1147 |
| 803 | | 1871 | 2006 | `KIS-SA_Sing_Target` | `hitDamage=EnergyShock` | sequences.sql:1279, sequences_nvp.sql:1149 |

## Gaps

1. **Chain ends with no sequence: 1092 (Bite, ability 983).** Its effect has
   no `event_set_id` and no script. The ability's own set 300 plays, and the
   effect names no target set. No packet owns 1092: NC-12 covers 736 and
   1717, NC-13 assigns 1433 only, and D-NC5 keeps 983 unassigned until NC-05.
   NC-31 records it as UNRESOLVED.
2. **Damage-inert target sets: 736 and 1717.** Both effects name set 301,
   which has five target sequences, but neither has a script or an NVP row.
   The target set is complete in the seed while the damage is zero; the server
   does not read it. This is the
   `effect_inert` case that NC-12 and NC-07 (item 6) cover.
3. **Source sequences inside target sets.** Set 289 holds `1105`
   (`KIS-SA_Burst_Source`, event 2008), set 262 holds `1805`
   (`KIS-SA_Sing_Source`), and set 803 holds `1869` (`KIS-SA_Beam_Source`).
   A target set should not carry a source animation. Whether it plays on the
   target is unknown. NC-06 checks it in the client.
4. **Human packages on creature and construct chains.** Every chain above
   names `KIS-abilities_human.*`. The ScavDog chains (1433 on sets 300 and 301,
   983 on set 300 only) and the PRU chain (221 on sets 802 and 803) are not
   human. The client packages are generic per mechanic, so the prefix is a
   package name and not proof of a family. The beam on 802 and 803 is the PRU's
   only presentation. NC-05 (ScavDog) and NC-06 (PRU, NID) decide whether a
   creature-appropriate sequence exists.
5. **No weapon-family rule for 1433 and 983.** Their `item_monikers` is
   empty and their `moniker_ids` value 1470900795 has no `monikers` row, so
   the family cannot be read from the seed. Set 300 is a judgement (RECONSTRUCTED, D-NC5).
6. **Ability and effect sets differ by design.** 1433 is on 300 and its
   effect 1717 on 301. 983 is on 300 but its effect 1092 has no set (Gap 1).
   A source/target pair is not a mismatch. The same holds for 559/5310 (15/289), 584/646 (3/262),
   710/736 (300/301) and 221/264 (802/803).
7. **Not presentation: 559 `required_ammo` 3.** NPCs have no ammo gate
   (F4 in README). Recorded for NC-11; no effect on the sets above.

## Next step

NC-06 checks the gaps in the client: the staff sockets and the
`KIS-SA_Melee_Target` rows for 301, the PRU beam for 802 and 803, and the
source sequences in target sets (gap 3). NC-05 checks gap 4 for ScavDog and
the 1433 and 983 links (RECONSTRUCTED, PR #822 Tier 3, unless a client record names the ability).
