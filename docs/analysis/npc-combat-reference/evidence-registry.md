# NPC combat reference: evidence registry

Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md).
This registry is the C6 evidence row for the four reference NPCs. NC-02 writes
the skeleton: the seed and code facts, and everything else as UNRESOLVED with
the packet that settles it. NC-31 fills the runtime results at the end.

Seed rows are `db/resources/` (Project Giza's approximation, so
EARLIEST LEGACY DATA). Code rows cite `crates/`. Line numbers are as of this
commit.

## Classes

Every row uses exactly one of these six classes (from README § Evidence classifications).

| Class | Meaning here |
|---|---|
| ORIGINAL CLIENT VERIFIED | Read from the owner's cooked client (UPK, pak, FEV) or SGW.exe RE, with the file and export cited |
| EARLIEST LEGACY DATA | The `db/resources` seeds (the Project Giza extraction) or the 2009 Python server, not checked against the client |
| CURRENT CIMMERIA IMPLEMENTATION | What the Rust server does today, cited by file and line |
| STRONGLY INFERRED | Indirect client evidence, such as a matching body family plus a generic KIS set, with no direct link |
| RECONSTRUCTED | Behaviour this campaign supplies, such as a damage payload taken from the ability's own description |
| UNRESOLVED | Unknown; listed in the registry's open list |

Rows follow the C6 layout: `Field | Value | Class | Source | Packet`.

## NID Guard (template 24)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 24 `NID Guard`, name_id 7417, class `mob` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:370` | NC-02 |
| Body set | `BS_HumanMale.BS_HumanMale` | EARLIEST LEGACY DATA | `entity_templates.sql:370` | NC-02 |
| Weapon item and component ids | `weapon_item_id` 21; components `AR_H_SGC.AR_HM_SB1_SH100`, `AR_H_SGC.AR_HM_SH1_SH100`, `AR_H_SGC.AR_HM_SL1_SL101SB100SH100`, `AR_H_SGC.AR_HM_ST1_ST103`, `BS_HumanMale.BS_HM_Hands_00`, `BS_HumanMale.BS_HM_Head_01`, `WP-Human.WP_SMG_1A` | EARLIEST LEGACY DATA | `entity_templates.sql:370` | NC-06 |
| Weapon item 21 | `SGHC 6 SMG` ("High Capacity SMG"), visual_component `WP-Human.WP_SMG_1A`, moniker_ids `{3175425141,728213066,3901383057}`, clip_size 30; it matches the template's `WP-Human.WP_SMG_1A` component | EARLIEST LEGACY DATA | `db/resources/Items/Seed/items.sql:4733`; D-NC6 (README F4) | NC-06 |
| Ability set | 3 `NID guard (SMG) ability set`; only ability 559 | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/ability_sets.sql:11`; `ability_set_abilities.sql:11` | NC-02 |
| Ability 559 | `Automatic Weapon Auto Attack`, type `ABILITY_TYPE_DD`, ranged, cooldown 1.5, min/max range 0, required_ammo 3, event set 15, effects `{621}`, monikers `{3175425141}` | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:5252` | NC-02 |
| Moniker 3175425141 | `ITEM_Automatic_Weapon` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/monikers.sql:51` | NC-02 |
| Effect 621 | `AW Auto Attack Damage`, ability 559, script `RangedPhysicalDamage`, event set 289, desc `-200F -20H` | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:7815` | NC-02 |
| Effect 621 NVPs | HealthDamage 20 (H), FocusDamage 200 (F), so the effect executes 20H/200F | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effect_nvps.sql:37`, `:39` | NC-02 |
| Event sets | Template event set 570 `Players default event set`; ability event set 15 `Generic burst weapon use (channelable)`; effect event set 289 `Generic burst weapon target` | EARLIEST LEGACY DATA | `entity_templates.sql:370`; `db/resources/Events/Seed/event_sets.sql:1323`, `:763`, `:427` | NC-04 |
| Sequence ids | Event-set to sequence rows (`event_sets_sequences`) not looked up in NC-02 | UNRESOLVED | `db/resources/Events/Seed/event_sets_sequences.sql` | NC-04 |
| Loot | `loot_table_id` 2 `Cellblock NID guard default` | EARLIEST LEGACY DATA | `db/resources/Loot/Seed/loot_tables.sql:9` | NC-02 |
| Faction and level | faction 10 (integer; the enum ordinal mapping is not checked here), level 1 | EARLIEST LEGACY DATA | `entity_templates.sql:370` | NC-02 |
| Animation package and export | Not checked | UNRESOLVED | (client) | NC-06 |
| Audio | Not checked | UNRESOLVED | (client) | NC-06 |
| Client build | Not recorded | UNRESOLVED | (owner client) | NC-06 |
| Server path | Ability choice uses set 3 (`known_count()` is non-zero, so no 592 fallback) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:63-67` | NC-02 |
| Runtime test result | Not run | UNRESOLVED | (Debug Area, Z12) | NC-31 |

## Praxis Jaffa Guard and Lieutenant (templates 160 and 159)

The two tables share the same body, ability set and effects. The value
columns are Guard (160) and Lieutenant (159).

| Field | Guard 160 | Lieutenant 159 | Class | Source | Packet |
|---|---|---|---|---|---|
| Template id | 160 `Praxis Jaffa Guard`, name_id 8917 | 159 `Praxis Jaffa Lieuternant` (sic), name_id 8918 | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:316`, `:326` | NC-02 |
| Body set | `BS_JaffaMale.BS_JaffaMale` | `BS_JaffaMale.BS_JaffaMale` | EARLIEST LEGACY DATA | `entity_templates.sql:316`, `:326` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; no `WP-Jaffa` component (the components are `AR_J_Praxis.*` and `BS_JaffaMale.*`) | `weapon_item_id` NULL; no `WP-Jaffa` component; the torso/tail variant is `AR_JM_PT1_PT100PT101PC100PS100` | EARLIEST LEGACY DATA | `entity_templates.sql:316`, `:326` | NC-22 (D-NC3) |
| Ability set | 4 `Jaffa staff ability set`: abilities 584 and 710 | same | EARLIEST LEGACY DATA | `ability_sets.sql:39`; `ability_set_abilities.sql:40`, `:49` | NC-02 |
| Ability 584 | `Staff Auto Attack`, ranged, event set 3, effects `{646}`, monikers `{1383013887}` | same | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:1452` | NC-02 |
| Effect 646 | `Staff Auto Attack Damage`, script `RangedPhysicalDamage`, event set 262, desc `-250F -25H`; NVPs HealthDamage 25, FocusDamage 250 (`effect_nvps.sql:25`, `:27`, nvp ids 10 and 11) | same | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:1243` | NC-02 |
| Ability 710 | `Staff Melee AA`, melee, event set 300, effects `{736}`, monikers `{1383013887}` | same | EARLIEST LEGACY DATA | `abilities.sql:5119` | NC-02 |
| Effect 736 | `Strike Damage`, event set 301, script NULL; no NVP rows | same | EARLIEST LEGACY DATA | `effects.sql:7522`; `effect_nvps.sql` (no `736` row) | NC-12 |
| Moniker 1383013887 | `ITEM_Staff` | same | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/monikers.sql:15` | NC-02 |
| Event sets | Template event set 570 `Players default event set`; ability event sets 3 and 300; effect event sets 262 and 301 | same | EARLIEST LEGACY DATA | `entity_templates.sql:316`, `:326`; `event_sets.sql:1323` | NC-04 |
| Loot | NULL | NULL | EARLIEST LEGACY DATA | `entity_templates.sql:316`, `:326` | NC-02 |
| Faction and level | faction 1, level 50 | faction 1, level 50 | EARLIEST LEGACY DATA | `entity_templates.sql:316`, `:326` | NC-02 |
| Sequence ids | Not looked up in NC-02 | Not looked up in NC-02 | UNRESOLVED | `event_sets_sequences.sql` | NC-04 |
| Animation package, audio, client build | Not checked | Not checked | UNRESOLVED | (client) | NC-06 |
| Server path | Ability choice uses set 4 (`known_count()` is non-zero, so no 592 fallback); ids are sorted ascending and the first off-cooldown one is chosen, so 584 wins over 710 even in melee reach and 710 fires only while 584 is on cooldown | same | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:63-76` | NC-12 |
| Runtime test result | Not run | Not run | UNRESOLVED | (Debug Area, Z12) | NC-31 |

## ScavDog (template 76)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 76 `ScavDog`, class `mob`, name_id NULL | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:153` | NC-02 |
| Body set | `MOB_ScavDog.BS_MOB_ScavDog` | EARLIEST LEGACY DATA | `entity_templates.sql:153` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; components `MOB_ScavDog.MOB_ScavDog_00` | EARLIEST LEGACY DATA | `entity_templates.sql:153` | NC-05 |
| Ability set | `ability_set_id` NULL: no set | EARLIEST LEGACY DATA | `entity_templates.sql:153` | NC-13 (D-NC5) |
| Empty-set fallback | An empty set adds 592 Pistol Shot (a humanoid firearm) through `NPC_DEFAULT_ABILITY` | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-world/src/cell/space_manager/spawn.rs:314-320`; `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:63-67` | NC-13 (D-NC5) |
| Unassigned ability 1433 | `Mob Strike`, melee, event set 300, effects `{1717}`, monikers `{1470900795}` | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:2047` | NC-13 |
| Effect 1717 | `Strike Damage`, event set 301, script NULL, desc `-125F -15H`, no NVP rows | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:3012`; `effect_nvps.sql` (no `1717` row) | NC-13 |
| Unassigned ability 983 | `Bite`, melee, event set 300, effects `{1092}`, monikers `{1470900795}` | EARLIEST LEGACY DATA | `abilities.sql:5036` | NC-05 |
| Effect 1092 | `Bite Damage`, event set NULL, script NULL, desc `-125F -15H` | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:2471` | NC-05 |
| Moniker 1470900795 | Not in `db/resources/Entities/Seed/monikers.sql` (see Open) | UNRESOLVED | `monikers.sql` | NC-13 |
| Event sets | Template event set 570 `Players default event set`; ability event set 300 `Generic melee weapon source`; effect event set 301 `Generic melee weapon target` | EARLIEST LEGACY DATA | `entity_templates.sql:153`; `event_sets.sql:431`, `:771` | NC-04 |
| Loot, faction, level | loot NULL; faction 10; level 1 | EARLIEST LEGACY DATA | `entity_templates.sql:153` | NC-02 |
| Sequence ids | `mob_scav.fev` has five attack variants, said to be referenced by mob notifies; per-notify ownership unverified | UNRESOLVED | (client FEV) | NC-05 |
| Animation package, audio, client build | Not checked | UNRESOLVED | (client UPK, pak) | NC-05 |
| Server path | Fires 592 through the empty-set fallback | CURRENT CIMMERIA IMPLEMENTATION | `spawn.rs:315-320`; `ability_select.rs:63-67` | NC-13 |
| Runtime test result | Not run | UNRESOLVED | (Debug Area, Z12) | NC-31 |

## Prisoner Retrieval Unit (template 4)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 4 `Prisoner retrieval unit`, name_id 7599, class `mob` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:9` | NC-02 |
| Body set | `MOB_CA_DroneTank.BS_MOB_DroneFlyer` | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; components `MOB_CA_DroneTank.MOB_DroneFlyer00` | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-07 |
| Ability set | 2 `Prisoner retrieval unit ability set`: ability 221 only | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/ability_sets.sql:9`; `ability_set_abilities.sql:7` | NC-02 |
| Ability 221 | `Energy Shock`, ranged, `ABILITY_TYPE_DD`, event set 802, effects `{264}`, no monikers, desc `80F 16H` | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:5078` | NC-02 |
| Effect 264 | `Energy Blast Damage`, script `RangedEnergyDamage`, event set 803 | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:7813` | NC-07 |
| Effect 264 NVPs | HealthDamage 16 (H), FocusDamage 80 (F) | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effect_nvps.sql:33`, `:35` | NC-02 |
| Damage type | Script `RangedEnergyDamage` maps to `DT_ENERGY`; the ammo-shot path defaults to `DT_PHYSICAL` when no shot is present, so which one a PRU hit uses is open | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/combat/damage/absorb.rs:87-90`; `crates/cell-combat/src/cell/abilities/damage_apply/mod.rs:221` | NC-07 |
| Event sets | Template event set 570 `Players default event set`; ability event set 802 `Energy shock ability source`; effect event set 803 `Energy shock effect target` | EARLIEST LEGACY DATA | `entity_templates.sql:9`; `db/resources/Events/Seed/event_sets.sql:803`, `:913` | NC-04 |
| Loot, faction, level | loot NULL; faction 10; level 1; `use_cover` false | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-02 |
| Sequence ids, animation, audio, client build | Not checked | UNRESOLVED | (client) | NC-06 |
| Server path | Ability choice uses set 2 (`known_count()` is non-zero, so no 592 fallback) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:63-67` | NC-14 |
| Runtime test result | Not run | UNRESOLVED | (Debug Area, Z12) | NC-31 |

## Open (UNRESOLVED)

- **O1 (D-NC8).** Ability 559's description reads `-100F/-10H`
  (`abilities.sql:5252`), but effect 621 executes `-200F -20H`
  (`effects.sql:7815`, NVPs 20H/200F). No change; the effect row executes.
  Record only. The same tooltip says "1 Ammo" while `required_ammo` is 3 (`abilities.sql:5252`); no ammo gate exists for NPCs (O9). Packet: NC-11.
- **O2.** Moniker 1470900795 (used by abilities 1433, 983 and 592) is not in
  `db/resources/Entities/Seed/monikers.sql`. Its name is unknown. Packet: NC-13.
- **O3 (D-NC6).** NID `weapon_item_id` 21 is the item `SGHC 6 SMG` (`items.sql:4733`). It has no server consumer (README F4), so the `WP-*` component is the weapon evidence. Packet: NC-06.
- **O4 (D-NC3).** The real Jaffa 159 and 160 have no `WP-Jaffa` staff
  component. Clones 1380 and 1381 carry it in the Debug Area. Packet: NC-22.
- **O5 (D-NC5).** ScavDog has no set. Ability 1433 has no set and effect 1717
  has no script or NVPs. Ability 983 is unassigned, and effect 1092 has no
  event set or script. Packets: NC-13, NC-05.
- **O6.** Effect 736 (ability 710) has no script and no NVPs, so it is inert
  in the seeds. Packet: NC-12.
- **O7.** The damage type of effect 264 (PRU) is open: the script maps to
  `DT_ENERGY` (`absorb.rs:87-90`), but the ammo path defaults to
  `DT_PHYSICAL` (`damage_apply/mod.rs:221`). Packet: NC-07.
- **O8.** Runtime suspects, from README F10 and not re-checked here: NPC
  mitigation 0/0, damage type forced to `DT_PHYSICAL`, DoT death firing no
  `entity_death`, and NPC LOS and target validation. Packet: NC-07.
- **O9.** No ammo gate and no reload exist for NID, per README F4. Not
  re-checked here. Packet: NC-11.
- **O10.** All four templates (4, 24, 76, 159/160) use event set 570
  `Players default event set` (`event_sets.sql:1323`). A player event set on
  NPCs is suspicious, and presentation follows the event set (F11). Packet: NC-04.
- **O11.** Effect 646 (ability 584) uses event set 262, while the ability row
  uses event set 3. Which one governs presentation is open. Packet: NC-04.
- **O12.** Faction integers 1 and 10 are seed values. The mapping to the
  `EFaction` enum is not verified (`db/resources/Social/Types/EFaction.sql`).
  Packet: NC-02 follow-up.
- **O13.** Sequence ids (`event_sets_sequences`), animation packages, audio and
  client builds for all four references. Packets: NC-04, NC-05, NC-06.
- **O14.** Mob Strike and Bite: per-notify ownership of the five `mob_scav.fev`
  attack variants (README F6). Packet: NC-05.

## Runtime results

Filled by NC-31.

| Reference | Test name | Packet | Result |
|---|---|---|---|
