# NPC combat reference: evidence registry

Ledger: [README.md](README.md). Packets: [work-packets.md](work-packets.md).
This registry is the C6 evidence row for the four reference NPCs. NC-02 wrote
the skeleton; NC-31 filled the rows from the NC-04..07 docs and the NC-11..14
results, and filled the runtime results and the open list at the end.

Seed rows are `db/resources/` (Project Giza's approximation, so
EARLIEST LEGACY DATA). Code rows cite `crates/`. Line numbers are as of the
W2 integration head (`2a1887107`).

## Classes

Every row names one or more of these six classes (from README § Evidence classifications). A split row names each class against the part it covers, for example a client-verified export with an unresolved KeyName.

| Class | Meaning here |
|---|---|
| ORIGINAL CLIENT VERIFIED | Read from the owner's cooked client (UPK, pak, FEV) or SGW.exe RE, with the file and export cited |
| EARLIEST LEGACY DATA | The `db/resources` seeds (the Project Giza extraction) or the 2009 Python server, not checked against the client |
| CURRENT CIMMERIA IMPLEMENTATION | What the Rust server does today, cited by file and line |
| STRONGLY INFERRED | Indirect client evidence, such as a matching body family plus a generic KIS set, with no direct link |
| RECONSTRUCTED | Behaviour or data Cimmeria chose by judgement, in this campaign or an earlier PR, such as a damage payload taken from the ability's own description |
| UNRESOLVED | Unknown; listed in the registry's open list |

Rows follow the C6 layout: `Field | Value | Class | Source | Packet`.

## NID Guard (template 24)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 24 `NID Guard`, name_id 7417, class `mob` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:371` | NC-02 |
| Body set | `BS_HumanMale.BS_HumanMale` | EARLIEST LEGACY DATA | `entity_templates.sql:371` | NC-02 |
| Weapon item and component ids | `weapon_item_id` 21; components `AR_H_SGC.AR_HM_SB1_SH100`, `AR_H_SGC.AR_HM_SH1_SH100`, `AR_H_SGC.AR_HM_SL1_SL101SB100SH100`, `AR_H_SGC.AR_HM_ST1_ST103`, `BS_HumanMale.BS_HM_Hands_00`, `BS_HumanMale.BS_HM_Head_01`, `WP-Human.WP_SMG_1A` | EARLIEST LEGACY DATA | `entity_templates.sql:371`; the `WP-Human.WP_SMG_1A` component is checked by `nid_reference_carries_smg_evidence` (NC-11) | NC-06 |
| Weapon item 21 | `SGHC 6 SMG` ("High Capacity SMG"), visual_component `WP-Human.WP_SMG_1A`, moniker_ids `{3175425141,728213066,3901383057}`, clip_size 30; it matches the template's `WP-Human.WP_SMG_1A` component. `weapon_item_id` is informational (D-NC6, decided 2026-11-27) | EARLIEST LEGACY DATA | `db/resources/Items/Seed/items.sql:4733`; README F4 | NC-06 |
| Ability set | 3 `NID guard (SMG) ability set`; only ability 559 | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/ability_sets.sql:11`; `ability_set_abilities.sql:11` | NC-02 |
| Ability 559 | `Automatic Weapon Auto Attack`, type `ABILITY_TYPE_DD`, ranged, cooldown 1.5, min/max range 0, required_ammo 3, event set 15, effects `{621}`, monikers `{3175425141}`. The tooltip reads `-100F/-10H` (O1) | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:5252` | NC-02 |
| Moniker 3175425141 | `ITEM_Automatic_Weapon` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/monikers.sql:51` | NC-02 |
| Effect 621 | `AW Auto Attack Damage`, ability 559, script `RangedPhysicalDamage`, event set 289, desc `-200F -20H`. To be rewritten to 10H/100F as RECONSTRUCTED (D-NC8, O1). | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:7817` | NC-02 |
| Effect 621 NVPs | HealthDamage 20 (H), FocusDamage 200 (F), so the effect executes 20H/200F | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effect_nvps.sql:37`, `:39` | NC-02 |
| Event sets | Template event set 570 `Players default event set`; ability event set 15 `Generic burst weapon use (channelable)`; effect event set 289 `Generic burst weapon target` | EARLIEST LEGACY DATA | `entity_templates.sql:371`; `db/resources/Events/Seed/event_sets.sql:1323`, `:763`, `:427` | NC-04 |
| Sequence ids | Traced to the event sets by NC-04 (`presentation-chain.md`, NID section). The KeyName each KIS slot node is fed, and the 1105 burst-source anomaly in set 289, are UNRESOLVED | UNRESOLVED | `presentation-chain.md`; [reference-npc-presentation.md](../../reverse-engineering/findings/reference-npc-presentation.md) § NID guard SMG, § Answers to NC-04 § Gaps | NC-04, NC-06 |
| Loot | `loot_table_id` 2 `Cellblock NID guard default` | EARLIEST LEGACY DATA | `db/resources/Loot/Seed/loot_tables.sql:9` | NC-02 |
| Faction and level | faction 10 (integer; the enum ordinal mapping is O12), level 1 | EARLIEST LEGACY DATA | `entity_templates.sql:371` | NC-02 |
| Animation package and export | `WP-Human.upk` (component, export 137), `WP_SMG_1A_psk` (export 458) and `KD_WP-Human.upk` read; the `KIS-SA_*` slot KeyNames are UNRESOLVED | ORIGINAL CLIENT VERIFIED (exports); UNRESOLVED (KeyName) | [reference-npc-presentation.md](../../reverse-engineering/findings/reference-npc-presentation.md) § NID guard SMG | NC-06 |
| Audio | Not checked for the NID in NC-06 | UNRESOLVED | (client) | NC-06 |
| Client build | Not recorded (no build stamp on the owner's copy in the repo) | UNRESOLVED | (owner client) | NC-06 |
| Server path | Ability choice uses set 3 (`known_count()` is non-zero, so no 592 fallback) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:59-69` | NC-02 |
| Runtime test result | Unit and live-DB tests green on the W2 tree (§ Runtime results). Live run at Z12 not done | UNRESOLVED | (Debug Area, Z12) | NC-31 (NC-40) |

## Praxis Jaffa Guard and Lieutenant (templates 160 and 159)

The two tables share the same body, ability set and effects. The value
columns are Guard (160) and Lieutenant (159).

| Field | Guard 160 | Lieutenant 159 | Class | Source | Packet |
|---|---|---|---|---|---|
| Template id | 160 `Praxis Jaffa Guard`, name_id 8917 | 159 `Praxis Jaffa Lieuternant` (sic), name_id 8918 | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:317`, `:327` | NC-02 |
| Body set | `BS_JaffaMale.BS_JaffaMale` | `BS_JaffaMale.BS_JaffaMale` | EARLIEST LEGACY DATA | `entity_templates.sql:317`, `:327` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; no `WP-Jaffa` component (the components are `AR_J_Praxis.*` and `BS_JaffaMale.*`). Staff to be added in place by NC-22 (D-NC3, decided yes; the staff is RECONSTRUCTED, not recovered original data) | `weapon_item_id` NULL; no `WP-Jaffa` component; the torso/tail variant is `AR_JM_PT1_PT100PT101PC100PS100`. Staff to be added by NC-22 | EARLIEST LEGACY DATA | `entity_templates.sql:317`, `:327` | NC-22 (D-NC3) |
| Ability set | 4 `Jaffa staff ability set`: abilities 584 and 710 | same | EARLIEST LEGACY DATA | `ability_sets.sql:39`; `ability_set_abilities.sql:40`, `:49` | NC-02 |
| Ability 584 | `Staff Auto Attack`, ranged, event set 3, effects `{646}`, monikers `{1383013887}` | same | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:1452` | NC-02 |
| Effect 646 | `Staff Auto Attack Damage`, script `RangedPhysicalDamage`, event set 262, desc `-250F -25H`; NVPs HealthDamage 25, FocusDamage 250 (`effect_nvps.sql:25`, `:27`, nvp ids 10 and 11) | same | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:1243` | NC-02 |
| Ability 710 | `Staff Melee AA`, melee, event set 300, effects `{736}`, monikers `{1383013887}` | same | EARLIEST LEGACY DATA | `abilities.sql:5119` | NC-02 |
| Effect 736 | `Strike Damage`, event set 301, script `MeleePhysicalDamage`, desc `-100F -10H`; NVPs HealthDamage 10 and FocusDamage 100 (nvp 470, 471). The script and NVPs were added by NC-12 (D-NC2, closed) | same | RECONSTRUCTED (script and NVPs, NC-12, from the description; name and event set are EARLIEST LEGACY DATA) | `db/resources/Effects/Seed/effects.sql:7524`; `effect_nvps.sql:163-166` | NC-12 |
| Moniker 1383013887 | `ITEM_Staff` | same | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/monikers.sql:15` | NC-02 |
| Event sets | Template event set 570 `Players default event set`; ability event sets 3 and 300; effect event sets 262 and 301 | same | EARLIEST LEGACY DATA | `entity_templates.sql:317`, `:327`; `event_sets.sql:1323` | NC-04 |
| Loot | NULL | NULL | EARLIEST LEGACY DATA | `entity_templates.sql:317`, `:327` | NC-02 |
| Faction and level | faction 1, level 50 | faction 1, level 50 | EARLIEST LEGACY DATA | `entity_templates.sql:317`, `:327` | NC-02 |
| Sequence ids | Traced by NC-04 for sets 300 and 301 (`presentation-chain.md`). Whether set 300 draws in melee stance is UNRESOLVED | Same as 160 | UNRESOLVED (melee stance) | `presentation-chain.md`; [reference-npc-presentation.md](../../reverse-engineering/findings/reference-npc-presentation.md) § Jaffa body, § Answers to NC-04 § Gaps | NC-04, NC-06 |
| Animation, audio, client build | Staff chain and melee AnimSequences read (`WP-Jaffa.upk` export 435, `JF-StaffWeapon_01` export 530, `JM_Animation.upk`). Audio not checked. Client build not recorded | Same | ORIGINAL CLIENT VERIFIED (exports); UNRESOLVED (audio, build) | [reference-npc-presentation.md](../../reverse-engineering/findings/reference-npc-presentation.md) § Jaffa serpent staff, § Melee AnimSequences | NC-06 |
| Server path | Ability choice uses set 4 (`known_count()` is non-zero, so no 592 fallback). Inside `NPC_MELEE_RANGE` (3.0 m, `aggro.rs:112`) an off-cooldown melee ability wins (D-NC4, NC-20), so 710 fires at melee reach. Beyond it the lowest-id off-cooldown ability wins, so 584 fires at range and while 710 cools | same | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:59-69`, `:149-184` | NC-20 |
| Runtime test result | Unit and live-DB tests green on the W2 tree (§ Runtime results). Live run at Z12 not done | Same | UNRESOLVED | (Debug Area, Z12) | NC-31 (NC-41) |

## ScavDog (template 76)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 76 `ScavDog`, class `mob`, name_id NULL | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:154` | NC-02 |
| Body set | `MOB_ScavDog.BS_MOB_ScavDog` | EARLIEST LEGACY DATA | `entity_templates.sql:154` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; components `MOB_ScavDog.MOB_ScavDog_00`. No weapon component, so no firearm evidence | EARLIEST LEGACY DATA | `entity_templates.sql:154` | NC-05 |
| Ability set | `ability_set_id` 6 `ScavDog: Mob Strike` (1433 only), set by NC-13 in place of the old NULL. The set is RECONSTRUCTED (D-NC5, decided yes) | RECONSTRUCTED | `entity_templates.sql:154`; `db/resources/Abilities/Seed/ability_sets.sql:43-44`; `ability_set_abilities.sql:127` | NC-13 (D-NC5) |
| Empty-set fallback | No longer reached by ScavDog. An empty set still adds 592 Pistol Shot (a humanoid firearm) through `NPC_DEFAULT_ABILITY` for other templates while `PLACEHOLDER_POLICY` is `Legacy` (D-NC1; NC-30 flips it) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-world/src/cell/combat/combat_profile/mod.rs:85-106`, `:116-121`; `crates/cell-world/src/cell/space_manager/spawn.rs:314-334`; `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:59-69` | NC-13 (D-NC5) |
| Ability 1433 | `Mob Strike`, melee, event set 300 (PR #822 Tier 3, see the link row), effects `{1717}`, monikers `{1470900795}`. Set membership in set 6 is RECONSTRUCTED | EARLIEST LEGACY DATA (row); RECONSTRUCTED (set 6) | `db/resources/Abilities/Seed/abilities.sql:2047` | NC-13 |
| Effect 1717 | `Strike Damage`, event set 301, script `MeleePhysicalDamage`, desc `-125F -15H`; NVPs HealthDamage 15 and FocusDamage 125 (nvp 472, 473), added by NC-12 (D-NC2, closed) | RECONSTRUCTED (script and NVPs, NC-12); name, desc and event set EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:3012-3013`; `effect_nvps.sql:167-170` | NC-12 |
| Unassigned ability 983 | `Bite`, melee, event set 300 (PR #822 Tier 3, see the link row), effects `{1092}`, monikers `{1470900795}`. Still in no set (D-NC5: stays unassigned until NC-05 finds evidence; it found none) | EARLIEST LEGACY DATA | `abilities.sql:5036` | NC-05 |
| Effect 1092 | `Bite Damage`, event set NULL, script NULL, desc `-125F -15H`. No target sequence, so the chain ends (presentation-chain.md, Gaps 1) | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:2471` | NC-05 |
| Link 1433 and 983 → event set 300 | Both NULL in the Giza seed; PR #822 set 300 as Tier-3 judgement calls | RECONSTRUCTED | [ability-animation-links.md](../../reverse-engineering/findings/ability-animation-links.md) § Tier 3; commit `f153138b5` | NC-05 |
| Moniker 1470900795 | Not in `db/resources/Entities/Seed/monikers.sql` (see O2) | UNRESOLVED | `monikers.sql` | NC-13 |
| Event sets | Template event set 570 `Players default event set`; event set 300 `Generic melee weapon source`; effect event set 301 `Generic melee weapon target` (its link from 1433 and 983 is the RECONSTRUCTED row above) | EARLIEST LEGACY DATA | `entity_templates.sql:154`; `event_sets.sql:431`, `:771` | NC-04 |
| Loot, faction, level | loot NULL; faction 10; level 1 | EARLIEST LEGACY DATA | `entity_templates.sql:154` | NC-02 |
| Sequence ids | `mob_scav.fev` holds five variants of one `attack` event. No notify in `MOB_ScavDog.upk` references them (the package has no AnimNotify), and no cooked package holds `mob_scav` as an event path (the bytes occur as the `MOB_ScavDog` object-name prefix in three packages, in the FEV and FSB files, and in the `GameplayEngine.ini` project registration). How the event path is built at run time is UNRESOLVED | UNRESOLVED | [scavdog-combat-assets.md](../../reverse-engineering/findings/scavdog-combat-assets.md) § Events (`:110-119`) | NC-05 |
| Animation package, audio, client build | `MOB_ScavDog.upk` (eight sequences; one attack, `sAttack1`, 1.133 s) and `VFX-ScavDog.upk` (`VFX-Attack` unreferenced by any cooked file) read; `BodySounds` empty. Stance `MeleeDistance` is 0 (value ORIGINAL CLIENT VERIFIED), but what the client reads it for is UNRESOLVED, so it is not a reach. Client build not recorded | ORIGINAL CLIENT VERIFIED (package contents); UNRESOLVED (attack spawn, MeleeDistance use, build) | [scavdog-combat-assets.md](../../reverse-engineering/findings/scavdog-combat-assets.md) lines 11, 35-39, 76, 143 | NC-05 |
| Server path | Ability choice uses set 6 (1433). Its reach is the melee reach through `effective_max_range` (`ability_select.rs:28`), so the ScavDog bites at melee range only. No 592 fallback | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:28`, `:59-69` | NC-13 |
| Runtime test result | Unit and live-DB tests green on the W2 tree (§ Runtime results). Live run at Z12 not done | UNRESOLVED | (Debug Area, Z12) | NC-31 (NC-42) |

## Prisoner Retrieval Unit (template 4)

| Field | Value | Class | Source | Packet |
|---|---|---|---|---|
| Template id | 4 `Prisoner retrieval unit`, name_id 7599, class `mob` | EARLIEST LEGACY DATA | `db/resources/Entities/Seed/entity_templates.sql:9` | NC-02 |
| Body set | `MOB_CA_DroneTank.BS_MOB_DroneFlyer` | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-02 |
| Weapon item and component ids | `weapon_item_id` NULL; components `MOB_CA_DroneTank.MOB_DroneFlyer00` | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-07 |
| Ability set | 2 `Prisoner retrieval unit ability set`: ability 221 only | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/ability_sets.sql:9`; `ability_set_abilities.sql:7` | NC-02 |
| Ability 221 | `Energy Shock`, ranged, `ABILITY_TYPE_DD`, event set 802, effects `{264}`, no monikers, desc `80F 16H` | EARLIEST LEGACY DATA | `db/resources/Abilities/Seed/abilities.sql:5078` | NC-02 |
| Effect 264 | `Energy Blast Damage`, script `RangedEnergyDamage`, event set 803 | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effects.sql:7815` | NC-07 |
| Effect 264 NVPs | HealthDamage 16 (H), FocusDamage 80 (F) | EARLIEST LEGACY DATA | `db/resources/Effects/Seed/effect_nvps.sql:33`, `:35` | NC-02 |
| Damage type | Direct hits use `DT_PHYSICAL` whenever no special-ammo shot is present, which is every NPC shot, so 221 lands as physical (confirmed, [runtime-blockers.md](runtime-blockers.md) § 2). Only DoT pulses use `script_damage_type` (`RangedEnergyDamage` gives `DT_ENERGY`) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/combat/damage/absorb.rs:87-90`; `crates/cell-combat/src/cell/abilities/damage_apply/mod.rs:221`; [runtime-blockers.md](runtime-blockers.md) | NC-07 |
| Event sets | Template event set 570 `Players default event set`; ability event set 802 `Energy shock ability source`; effect event set 803 `Energy shock effect target` | EARLIEST LEGACY DATA | `entity_templates.sql:9`; `db/resources/Events/Seed/event_sets.sql:803`, `:913` | NC-04 |
| Loot, faction, level | loot NULL; faction 10; level 1; `use_cover` false | EARLIEST LEGACY DATA | `entity_templates.sql:9` | NC-02 |
| Sequence ids, animation | Sequence 1869 (event 2004) is `KIS-SA_Beam_Source` inside target set 803, a seed anomaly. The KD byte map and the slot KeyNames are UNRESOLVED. Animation package read as a mechanical construct with an integrated weapon (NC-14, `pru_reference_has_integrated_weapon`) | UNRESOLVED | [reference-npc-presentation.md](../../reverse-engineering/findings/reference-npc-presentation.md) § PRU drone, § Unresolved | NC-04, NC-07 |
| Audio, client build | Not checked | UNRESOLVED | (client) | NC-06 |
| Server path | Ability choice uses set 2 (`known_count()` is non-zero, so no 592 fallback) | CURRENT CIMMERIA IMPLEMENTATION | `crates/cell-combat/src/cell/service/npc_ai/ability_select.rs:59-69` | NC-14 |
| Runtime test result | Unit and live-DB tests green on the W2 tree (§ Runtime results). Live run at Z12 not done | UNRESOLVED | (Debug Area, Z12) | NC-31 (NC-43) |

## Open (UNRESOLVED)

The open list is the brief's deliverable 7: the original behaviour the campaign
has not settled. Each item names the packet or decision that owns it. Items
closed in Wave 2 stay listed, struck through, so the trail is kept.

- **O1 (D-NC8, decided 2026-11-27: follow the 559 tooltip).** Ability 559's description
  reads `-100F/-10H` (`abilities.sql:5252`), but effect 621 still executes `-200F -20H`
  (`effects.sql:7817`, NVPs 20H/200F at `effect_nvps.sql:37`, `:39`). The owner chose the tooltip: a follow-up packet rewrites effect 621 to 10H/100F, and that rewrite is RECONSTRUCTED.
  Until it merges, the seed rows in this registry are the current data. The same tooltip says "1 Ammo" while
  `required_ammo` is 3; no ammo gate exists for NPCs (O9). Packet: NC-11 (recorded); follow-up packet (to be numbered).
- **O2.** Moniker 1470900795 (used by abilities 1433, 983 and 592) is not in
  `db/resources/Entities/Seed/monikers.sql`. Its name is unknown. Packet: NC-13 (open).
- **O3 (D-NC6, decided 2026-11-27: informational).** NID `weapon_item_id` 21 is
  the item `SGHC 6 SMG` (`items.sql:4733`). It has no server consumer, so the
  `WP-*` component is the weapon evidence. Recorded; no change. Packet: NC-06 (closed).
- **O4 (D-NC3, decided 2026-11-27: yes, in place, RECONSTRUCTED).** The real Jaffa 159 and 160
  have no `WP-Jaffa` staff component. The seed row is NC-22's to add. Clones
  1380 and 1381 carry it in the Debug Area. Open until NC-22 merges. Packet: NC-22.
- **O5 (D-NC5).** ScavDog set 6 holds 1433 Mob Strike (RECONSTRUCTED, NC-13).
  Ability 983 Bite is unassigned, and effect 1092 has no event set and no
  script, so the Bite chain ends. Open for 983. Packets: NC-13 (1433 done), NC-05 (983: no evidence found).
- **O6 (closed by NC-12, D-NC2).** ~~Effect 736 (ability 710) has no script and no NVPs.~~
  Effects 736 and 1717 now carry `MeleePhysicalDamage` and NVPs 470-473
  (RECONSTRUCTED, from each effect's description). Packet: NC-12 (closed).
- **O7.** Confirmed physical for direct hits by NC-07 ([runtime-blockers.md](runtime-blockers.md) § 2); the fix is a proposed packet. Packet: NC-07.
- **O8.** Runtime suspects, re-verified by NC-07 in [runtime-blockers.md](runtime-blockers.md): NPC mitigation 0/0 confirmed (§ 1), damage type forced physical confirmed for direct hits (§ 2), DoT kill event fixed (§ 3), NPC LOS and target validation not a bug with one edge unverified (§ 4). Packet: NC-07.
- **O9.** No ammo gate and no reload exist for NID, per README F4. Not
  re-checked here. Packet: NC-11.
- **O10.** All four templates (4, 24, 76, 159/160) use event set 570
  `Players default event set` (`event_sets.sql:1323`). A player event set on
  NPCs is suspicious, and presentation follows the event set (F11). Open. Packet: NC-04.
- **O11.** Effect 646 (ability 584) uses event set 262, while the ability row
  uses event set 3. Which one governs presentation is open. Packet: NC-04.
- **O12.** Faction integers 1 and 10 are seed values. The mapping to the
  `EFaction` enum is not verified (`db/resources/Social/Types/EFaction.sql`).
  Packet: NC-02 follow-up.
- **O13.** Sequence ids (the KeyName each KIS slot node is fed, and the seed
  target events 2002-2006 to KD bytes 7-11), audio and client builds for all
  four references. Packets: NC-04, NC-05, NC-06.
- **O14 (NC-05 done; event path UNRESOLVED).** ~~Per-notify ownership of the five
  `mob_scav.fev` attack variants.~~ The package has no notify that references
  them, and the five variants are one `attack` event. Who plays the event for
  983 Bite and 1433 Mob Strike is still unknown. The `audio-voice-inventory.md`
  §7 "notifies trigger these" row is contradicted. Packet: NC-05.
- **O15 (ScavDog stance).** What the client reads `BodyStance.MeleeDistance` (0)
  for is not traced in `SGW.exe`. The ScavDog's reach is therefore not a
  client value; the server reach is `NPC_MELEE_RANGE` (RECONSTRUCTED). Packet: NC-05.
- **O16 (D-NC7, approved in principle 2026-11-27).** Live-client UAT in the lab
  is not run. Each run is asked first; Z12 is the place. Packets: NC-40..44.
- **O17.** The 1105 burst-source anomaly (`KIS-SA_Burst_Source` in target set
  289, the NID SMG) is not checked in the client. Packet: NC-06.

## Runtime results

Filled by NC-31 from the tests in the W2 integration tree (`2a1887107`). The
W2 review ran the full cell-world, cell-combat and cell nextest and live-DB
suites green on that tree (RESUME.md, 2026-11-27). NC-31 did not re-run them,
and no live Z12 run has happened (W4, D-NC7).

| Reference | Test name | Packet | Result |
|---|---|---|---|
| NID 24 | `nid_reference_carries_smg_evidence`, `nid_reference_takes_cover` | NC-11 | Green on W2 tree (unit) |
| NID 24 | `live_db_reference_nid_smg_damages_a_player`, `live_db_reference_nid_loot_and_respawn` | NC-11 | Green on W2 tree (live DB) |
| Jaffa 159/160 | `live_db_reference_jaffa_melee_effects_carry_damage`, `live_db_reference_jaffa_staff_swing_damages_in_reach` | NC-12 | Green on W2 tree (live DB) |
| Jaffa 159/160 | `live_db_reference_jaffa_swings_staff_in_reach` (`melee_preference.rs`) | NC-20 | Green on W2 tree (live DB) |
| ScavDog 76 | `scavdog_reference_is_melee_only`, `live_db_reference_scavdog_set_6_loads`, `live_db_reference_scavdog_bites_in_reach_only` | NC-13 | Green on W2 tree (unit and live DB) |
| PRU 4 | `pru_reference_has_integrated_weapon`, `live_db_reference_pru_energy_shock_defs`, `live_db_reference_pru_shock_damages_a_player` | NC-14 | Green on W2 tree (unit and live DB) |
| Z12 range | `reference_range_holds_the_four_references`, `live_db_debug_area_reference_rows_load` | NC-03 | Green on W2 tree |
| Harset Jaffa and pets | `live_db_regression_harset_jaffa_profiles`, `live_db_regression_jaffa_pet_sets_carry_staff` | NC-24 | Green on W2 tree (live DB) |
| Tutorial and CellBlock | `live_db_regression_cellblock_profiles_unchanged`, `live_db_regression_tutorial_no_set_mobs_keep_firing_under_legacy`, `live_db_regression_pru_cellblock_activation_rows_intact` | NC-23 | Green on W2 tree (live DB) |
| Z12 in the game | Live UAT at Z12 for NID, Jaffa, ScavDog and PRU | NC-40..43 | Not run (D-NC7) |
