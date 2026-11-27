# Reference NPC weapon presentation assets (NC-06)

This finding records which client assets draw the attacks of the reference NPCs in
the NPC combat reference campaign. The NPCs are the Jaffa with the serpent staff, the
NID guard with the SMG and the PRU drone (`MOB_DroneFlyer00`). For each one it
records the weapon component, the mesh sockets and bones, the per-weapon Kismet data
(particle systems, sounds and animation names), and the seed event sets that start
the effects.

Every row was read from the owner's cooked client tree with the repo's UPK reader
(`tools/upk_parser.py` `PackageReader`, plus `inspect-export --props --prop-offset 4`
and `extract_kismet --graph` from `crates/upk`). No client files were copied into the
repo.

- **Client root:** `..\SGW\Stargate Worlds-QA\Working\SGWGame\CookedPC`. Package
  paths below are relative to it.
- **Character packages:** `Packages\Character\`.
- **Kismet packages:** `Packages\`.

Related findings:

- [npc-attack-presentation.md](npc-attack-presentation.md): why NPC shots draw
  nothing at runtime (the BSF_InCombat and SequenceManager path).
- [ability-animation-links.md](ability-animation-links.md): how an ability reaches
  its event set and its Kismet sequence.
- [`docs/analysis/npc-combat-reference/`](../../analysis/npc-combat-reference/): the
  campaign, including the NC-04 seed chain.

## Evidence classes

Each table row has one class from the campaign spec:

- **ORIGINAL CLIENT VERIFIED (OCV):** read directly from a cooked client package
  export.
- **EARLIEST LEGACY DATA (ELD):** a seed row under `db/resources/`. Those seeds are
  Project Giza's reconstruction, so a seed row alone is never original.
- **STRONGLY INFERRED (SI):** follows from client data, but no single export states it.
- **UNRESOLVED (UNR):** not established.

The spec's other two classes (CURRENT CIMMERIA IMPLEMENTATION, RECONSTRUCTED) are not
used here.

## How the presentation data fits together

The client draws a weapon attack from four kinds of export:

1. **The `BodyComponent`** (the `WP_*` export) names the following.
   - The body sets it fits (`BodySetList`).
   - The socket it attaches to on the body (`BodySockets[].SocketName`) and its mesh.
   - An array of `KismetData` entries. Each entry is
     `{BodySlot, ComponentKismetData}`.
2. **`ComponentKismetData` (KD)** is one per-weapon effect record. Its fields:
   - `SequenceEvent`: a byte.
   - `KeyName`.
   - `ComponentObjectVar`: a ParticleSystem.
   - `ComponentFloatVar`.
   - `ComponentStringVar`: an FMOD event path.
   - `ComponentNameVar` and `ComponentNameVar2`: a bone or animation name.
   - `ComponentRecoilVar`.
3. **The generic Kismet templates** in `KIS-abilities_human.upk` are typed by
   mechanic (`KIS-SA_Sing_Source`, `KIS-SA_Burst_Target`, `KIS-SA_Melee_Source`,
   `KIS-SA_Beam_Source` and so on). Each holds `SeqAct_ComponentSlotData` nodes whose
   `KeyName` and `SlotName` arrive on variable links, so the template has no weapon
   asset of its own (OCV, see the table below).
4. **The `BodySet`** maps a stance to an AnimSet through `BodyStances[]`. Each stance
   is `{AnimationSet, Combat, MeleeDistance, Weapon}`.
   - The stance's `Weapon` byte matches the `BodyComponent`'s `Weapon` value.
   - The KD animation names (`sMAttack1` and so on) resolve in the AnimSet that the
     current stance selects.

A seed sequence therefore names only a generic template. The particle systems,
sounds and animations come from the KD entries of whatever weapon component the NPC
wears. That is why every reference NPC, including the Jaffa and the drone, can run
`KIS-abilities_human` scripts.

### `KIS-abilities_human.upk` template nodes

| Template | `SeqAct_ComponentSlotData` exports | Other nodes | Class |
|---|---|---|---|
| `KIS-SA_Sing_Source` | 784–788 | `SeqAct_ComponentSlotAttachment` 725 | OCV |
| `KIS-SA_Sing_Target` | 797–803 | `SeqAct_SpawnEmitter` redirectors 651–652 | OCV |
| `KIS-SA_Burst_Source` | 750–753 | `SeqAct_ComponentSlotAttachment` 715 | OCV |
| `KIS-SA_Burst_Target` | 754–760 | — | OCV |
| `KIS-SA_Melee_Source` | 774–777 | — | OCV |
| `KIS-SA_Melee_Target` | 778–783 | `SeqAct_SpawnEmitter` redirectors 553–557 | OCV |
| `KIS-SA_Beam_Source` | 747 | `SeqAct_ComponentSlotAttachment` 713, `SeqAct_Gate` 999, `SeqAct_Delay` 819–821 | OCV |

- Each `SeqAct_ComponentSlotData` dumped here (747, 750–753, 774, 776–778, 784–788
  and 797–800) has a variable link with `PropertyName = KeyName` and another with
  `PropertyName = SlotName` (OCV).
- Export 775 is a 264-byte stub with no links (OCV).
- **UNR:** which literal key each node is fed. The `SeqVar` values were not decoded,
  and `extract_kismet --graph` prints node counts only.

## SequenceEvent byte values

These byte values were observed in the KD entries. A byte's reading is marked SI where
the names agree in at least two packages and UNR where only one package carries it.

| Byte | KD names that carry it | Reading | Class |
|---|---|---|---|
| 1 | `*-ChargeStart_*` | charge start | SI |
| 2 | `*-Sing_*`, `*-BurstASR_*`, `*-Melee_0n`, `HM-Tracer_Standard`, `Mul_CA-DroneFlyerEnergyShock` (the staff tracer carries no byte) | fire or swing (source) | SI |
| 7 | `*-Hit_*` | hit | SI |
| 8 | `Mul_CA-HitCrit_EnergyShock` | critical hit | UNR (drone package only; reading taken from the entry name) |
| 9 | `Mul_CA-HitCrit2x_EnergyShock` | double critical | UNR (drone package only; reading taken from the entry name) |
| 10 | `Mul_CA-HitGlances_EnergyShock` | glancing hit | UNR (drone package only; reading taken from the entry name) |
| 11 | `*-Miss_*`, `*-HitMiss_*` | miss | SI |
| 15 | `Mul_CA-DroneFlyerDeath` | death | UNR (drone package only; reading taken from the entry name) |

- **UNR:** how these bytes line up with the seed `event_id` values 2002–2006 used by
  the target sequences. The seed has five target events and the KD has five hit
  kinds (hit, crit, crit2x, glance, miss), but no client export pairs them.

## Jaffa serpent staff: `WP-Jaffa.WP_Staff_Plasma_4A`

### Seed chain

| Event set | Name | Sequences, with event and `kismet_script_name` | Class |
|---|---|---|---|
| 3 | Generic single-shot weapon use (channelable) | 3 (1001, `KIS-SA_Sing_Source`); 2182 (1000), 2617 (1004), 2618 (1007), 2619 (1006): all `KIS-SA_Sing_Source`; 2969 (1002, `KIS-SA_Effect_Source`) | ELD |
| 300 | Generic melee weapon source | 346 (1001, `KIS-SA_Melee_Source`); 2959 (1002, `KIS-SA_Effect_Source`) | ELD |
| 301 | Generic melee weapon target | 348–352 (events 2002–2006, `KIS-SA_Melee_Target`) | ELD |

- **Sources:** `db/resources/Events/Seed/event_sets.sql` lines 765, 431 and 771;
  `event_sets_sequences.sql` lines 1495–1519; `sequences.sql` lines 1687, 2305–2309,
  3001, 3063, 2981 and 3169–3179.
- Every script named is in `KIS-abilities_human`. No seed row names
  `KIS-abilities_jaffa`.
- **UNR:** the seed gives no ranged target set for the staff. Hit effects for a
  single-shot weapon come from the target set of the ability or effect (see NC-04),
  not from set 3.

### Component (`Packages\Character\WP-Jaffa.upk`, export 435)

| Fact | Value | Class |
|---|---|---|
| `Weapon` | 3 | OCV |
| `AimProfileIndex` | 4 | OCV |
| `BodySetList` | Goauld Female and Male, Human Female and Male, Jaffa Female and Male | OCV |
| Body socket | `BodySlot Weapon`, `SocketName StaffActive` | OCV |
| AnimTree | `WP-Jaffa.JM_Staff_Master_Animtree` | OCV |
| Skeletal mesh | export 530 `JF-StaffWeapon_01` | OCV |
| `BodyParticles`, `BodySounds` | empty | OCV |
| `KismetData` | 20 entries, all in `KD_WP-Jaffa` (listed below) | OCV |

### Mesh sockets and bones (`JF-StaffWeapon_01`, export 530)

| Item | Detail | Class |
|---|---|---|
| Grip sockets | Exports 571–575, all on bone `LHand`: `LHand` (0, −7.7, −0.5), `LHandJaffa` (−0.3, −9.72, 2.45), `LHandGM` (−5.3, −9.36, 0.1), `LHandGF` (−8.65, −7.95, −2.1), `LHandHF` (0, −5.85, −2.35) | OCV |
| Muzzle socket | None. No `SkeletalMeshSocket` on this mesh is named `Muzzle`. | OCV |
| `Muzzle` bone | The FName `Muzzle` sits in export 530's serial data at byte 340, among bone-table names. The staff AnimSets 303 (`JF-StaffWeapon_01_Closed`) and 304 (`_OpenAnimset`) list the track bones Origin, LHand, Muzzle, Head and Dummy01. | SI |

- Only the ChargeStart KD entries (export 30 here) name `Muzzle`, so the charge effect
  attaches at `Muzzle` (SI). The muzzle-flash and tracer entries carry no name var, so
  their attach point is UNR.
- `JM_Plasma_Staff` (export 554, sockets 678–682) is a different mesh. This component
  does not use it.

### Kismet data (`Packages\Character\KD_WP-Jaffa.upk`)

| Export | Entry | SequenceEvent | Key | Particle system | Float | Sound | Name vars | Class |
|---|---|---|---|---|---|---|---|---|
| 30 | `JF-ChargeStart_Staff_Plasma_4A` | 1 | ChargeStart | `VFX-Weapons.Jaffa.WFX-StaffCharge` | — | `staff_plasma_1a/hand/chargeASR` | `Muzzle` | OCV |
| 467 | `JF-Sing_Staff_Plasma_4A` | 2 | sing | `VFX-Weapons.Jaffa.WFX-SerpentStaffMuzzleFlash` | 0.15 | `staff_plasma_4a/atk/sing` | — | OCV |
| 545 | `JF-Tracer_Staff_Plasma_4A` | — | Standard | `VFX-Weapons.Jaffa.WFX-SerpentStaffTracer` | — | — | — | OCV |
| 76 | `JF-Hit_Staff_Plasma_4A` | 7 | Hit | `VFX-Weapons.Explosions.WFX-ExploPlasmaSmall` | 0.222 | `staff_plasma_4a/atk/hit` | `cHit`, `sHit` | OCV |
| 388 | `JF-Miss_Staff_Plasma_4A` | 11 | Hit | — | — | `staff_plasma_4a/atk/miss` | — | OCV |
| 514 | `JF-Staff_Melee_01` | 2 | Melee1 | `VFX-Weapons.Jaffa.WFX-Archmelee02` | 0.13 | `melee_punch/atk/miss` | `sMAttack1` | OCV |
| 515 | `JF-Staff_Melee_02` | 2 | Melee2 | `VFX-Weapons.Jaffa.WFX-Archmelee01` | 0.15 | `melee_punch/atk/miss` | `sMAttack2` | OCV |
| 443 | `JF-Sing_MELSM` | 2 | singmelee | — | 2.0 | `melee_gen/h2h/miss` | — | OCV |
| 52 | `JF-Hit_MELSM_Sing` | 7 | Hitmelee | `VFX-Weapons.SGC.WFX-melee` | — | `melee_gen/sm/hit` | `cHit`, `sHit` | OCV |
| 595 | `Rec_StaffSing_Heavy_Loc` | — | RecoilLoc | — | 0.75 | — | `xRecoilBlend` | OCV |

- The remaining component entries were not dumped field by field. They are
  `JF-HitCrit2x`, `JF-HitCrit`, `JF-HitGlance` and `JF-Equip_SX` / `JF-Unequip_SX`,
  `Rec_StaffSing_Heavy_Rot`, and the `_MELSM_Sing` crit, glance and miss entries
  (listed in the component, OCV; fields not read, UNR).
- **The projectile is a particle tracer, not a projectile actor** (SI). The staff's
  ranged shot uses:
  - a muzzle flash, `WFX-SerpentStaffMuzzleFlash`;
  - a tracer, `WFX-SerpentStaffTracer`;
  - an impact, `WFX-ExploPlasmaSmall`.
  - No projectile class is referenced from the component or its KD.

## Jaffa body: `BS_JaffaMale`

`Packages\Character\BS_JaffaMale.upk`, export 52 (BodySet).

| Fact | Value | Class |
|---|---|---|
| AnimTree | `SGWAnimTrees.JaffaAnimTree` | OCV |
| Reference mesh | export 266 `JM_RefSkelMesh` | OCV |
| `DefaultComponents` | `WP-Human.WP_HandToHand_1A`, `BS_HumanMale.BS_HM_VOXSlot_00` | OCV |

### Body stances

| Stance | AnimSet | Combat | MeleeDistance | Weapon | Class |
|---|---|---|---|---|---|
| 0 | `JM_Animation.Generic` | 0 | 0 | 0 | OCV |
| 1 | `JM_Animation.Relaxed` | 2 | 0 | 0 | OCV |
| 2 | `JM_Animation.OneHanded` | 0 | 0 | 1 | OCV |
| 3 | `JM_Animation.TwoHanded` | 0 | 0 | 2 | OCV |
| 4 | `JM_Animation.Hand2Hand` | 1 | 0 | 4 | OCV |
| 5 | `JM_Animation.StaffMelee` | 0 | 1 | 3 | OCV |
| 6 | `JM_Animation.StaffRanged` | 0 | 2 | 3 | OCV |
| 7 | `JM_Animation.StaffShared` | 0 | 0 | 3 | OCV |
| 8 | `JM_Animation.Knife` | 0 | 0 | 5 | OCV |

The staff has `Weapon = 3`. A staff-armed Jaffa therefore selects `StaffMelee` at
melee distance 1 and `StaffRanged` at melee distance 2 (SI, from the matching bytes).

### Melee AnimSequences (`Packages\Character\JM_Animation.upk`)

| SequenceName | Exports (AnimSet) | Class |
|---|---|---|
| `sMAttack1` | 1101 (Hand2Hand), 1167 (Knife), 1264 (OneHanded), 1430 (StaffMelee), 1512 (StaffRanged), 1617 (TwoHanded) | OCV |
| `sMAttack2` | 1091 (Hand2Hand), 1161 (Knife), 1431 (StaffMelee) | OCV |
| `sMAttack3` | 1139 (Hand2Hand), 1447 (StaffMelee) | OCV |

- `BS_JaffaMale.upk` holds no `sMAttack` names of its own. The sequences live in the
  stance AnimSets.

**Answer: yes.** The body has melee AnimSequences that set 300 can drive (SI: the
stance AnimSets and the KD animation names are OCV; that `KIS-SA_Melee_Source` selects
those KD entries by key is UNR, see Unresolved).

- `KIS-SA_Melee_Source` reads the weapon's melee KD.
- The staff KD names `sMAttack1` and `sMAttack2` (exports 514 and 515).
- `sMAttack1` exists in StaffMelee and StaffRanged; `sMAttack2` exists in StaffMelee
  only (so `JF-Staff_Melee_02` has nothing to play in StaffRanged).

### Unarmed Jaffa (`WP-Human.WP_HandToHand_1A`)

The seed Jaffa guard (160) and Jaffa lieutenant (159) carry no `WP` component:
`db/resources/Entities/Seed/entity_templates.sql` lines 316 and 326 (ELD). Those
NPCs fall back to the body set's default component.

`Packages\Character\WP-Human.upk`, export 74:

| Fact | Value | Class |
|---|---|---|
| `BodySetList` | Human Female and Male, Jaffa Female and Male, `MOB_AN_Android.BS_AN_Android` | OCV |
| Socket | `2HActive`, no mesh | OCV |
| `KismetData` | 12 entries in `KD_WP-Human` (listed below) | OCV |

The 12 `KismetData` entries:

- `HM-Hand2Hand_Melee_01` to `_04`;
- `HM-Hit`, `HM-HitCrit2x`, `HM-HitCrit`, `HM-HitGlance` and `HM-HitMiss`, each with
  the suffix `_MELH2H_Sing`;
- `HM-Sing_MELH2H`;
- `HM-Equip_HH` and `HM-Unequip_HH`.

| KD export | Entry | SequenceEvent | Key | Name vars | Sound | Class |
|---|---|---|---|---|---|---|
| 218 | `HM-Hand2Hand_Melee_01` | 2 | Melee1 | `sMAttack1`, `Top` | — | OCV |
| 219 | `HM-Hand2Hand_Melee_02` | 2 | Melee2 | `sMAttack2` | — | OCV |
| 788 | `HM-Sing_MELH2H` | 2 | singmelee | — | `melee_gen/h2h/miss` | OCV |

`WP_HandToHand_1A` has no serialized `Weapon` byte (OCV: export 74 lists only
`BodyVisuals`), so which stance an unarmed Jaffa uses is UNR. The `Hand2Hand` stance
(`Weapon = 4`, `Combat = 1`) and its `sMAttack1`/`sMAttack2` sequences exist (OCV), but
nothing read links the component to it. It has no ranged KD at all, so set 3 has
nothing to draw for templates 159 and 160 (SI).

## NID guard SMG: `WP-Human.WP_SMG_1A`

### Seed chain

Seed template 24 (NID Guard) carries `WP-Human.WP_SMG_1A`:
`db/resources/Entities/Seed/entity_templates.sql` line 370 (ELD).

| Event set | Name | Sequences, with event and `kismet_script_name` | Class |
|---|---|---|---|
| 15 | Generic burst weapon use (channelable) | 15 (1001, `KIS-SA_Burst_Source`); 2606 (1004), 2607 (1007), 2616 (1006): `KIS-SA_Channel_Source`; 2608 (1000, `KIS-SA_Charge_Source`); 2963 (1003, `KIS-SA_Effect_Source`) | ELD |
| 289 | Generic burst weapon target | 313–317 (events 2002–2006, `KIS-SA_Burst_Target`); 1105 (2008, `KIS-SA_Burst_Source`) | ELD |

- **Sources:** `event_sets.sql` lines 763 and 427; `event_sets_sequences.sql` lines
  1271–1281 and 1457–1467; `sequences.sql` lines 785, 2283–2287, 2303, 2989, 211 and
  3133–3141.
- Sequence 1105 is a Source script inside the target set 289 (ELD). The client side
  of this was not checked.

### Component (`Packages\Character\WP-Human.upk`, export 137)

| Fact | Value | Class |
|---|---|---|
| `Weapon` | 2 | OCV |
| `BodySetList` | Human Female and Male, Jaffa Female and Male, `BS_AN_Android` | OCV |
| Body socket | `2HActive`, no AnimTree | OCV |
| Skeletal mesh | export 458 `WP_SMG_1A_psk` | OCV |
| `KismetData` | 85 entries in `KD_WP-Human` (listed below) | OCV |

The 85 `KismetData` entries cover:

- the `SBR` hit, crit and miss entries for `burstASR` and `Sing`;
- the ARPC, EMP, EXP, HLPT and INCD T1 ammo variants;
- equip and unequip;
- the 2HRepeat and 2HSingle recoil entries;
- the tracers;
- the two-handed melee entries;
- the generic charge entries.

### Mesh sockets and bones (`WP_SMG_1A_psk`, export 458)

| Item | Detail | Class |
|---|---|---|
| Grip sockets | Exports 857–862, all on `LHand`: `LHandRX`, `LHandJaffa`, `LHandJaffaRX`, `LHand`, `LHandJFRX`, `LHandHFRX` | OCV |
| Muzzle socket | None | OCV |
| `Muzzle` bone | The FName `Muzzle` (name 1008) sits in export 458's serial data at byte 296, after `Aim` in the bone-name run. | SI |

- `MuzzleG` (name 1009) is in the package name table but not in this mesh (OCV).
- Only the ChargeStart KD entry (export 83 here) names `Muzzle`, so the charge effect
  attaches at `Muzzle` (SI). The muzzle-flash and tracer entries carry no name var, so
  their attach point is UNR.

### Kismet data (`Packages\Character\KD_WP-Human.upk`)

| Export | Entry | SequenceEvent | Key | Particle system | Float | Sound | Name vars | Class |
|---|---|---|---|---|---|---|---|---|
| 56 | `HM-BurstASR_SMG_1A` | 2 | burstASR | `VFX-Weapons.SGC.WFX-autosust` | 0.067 | `smg_1a/atk/burstASR` | — | OCV |
| 815 | `HM-Sing_SMG_1A` | 2 | sing | `VFX-Weapons.SGC.WFX-autoBurst` | — | `smg_1a/atk/sing` | — | OCV |
| 855 | `HM-Tracer_Standard` | 2 | Standard | `VFX-Weapons.SGC.WFX-tracer` | — | — | — | OCV |
| 289 | `HM-Hit_SBR_burstASR` | 7 | HitBurst | `VFX-Weapons.SGC.WFX-bulletburst` | — | `alertXBR/sbr/burst/hit` | `cHitS`, `sHitS` | OCV |
| 561 | `HM-HitMiss_SBR_burstASR` | 11 | HitBurst | — | — | `alertXBR/sbr/burst/miss` | — | OCV |
| 582 | `HM-Rec_2HRepeat_Light_Loc` | — | recoillocburst | — | 0.3 | — | — | OCV |
| 857 | `HM-TwoHanded_Melee_01` | 2 | Melee1 | — | 0.1 | — | `sMAttack1` | OCV |
| 83 | `HM-ChargeStart_Generic` | 1 | ChargeStart | `PFX-abilities.PFX-AimWarmup` | — | — | `Muzzle` | OCV |

- **Flash:** `WFX-autosust` for a sustained burst and `WFX-autoBurst` for a single
  shot (OCV).
- **Tracer:** `WFX-tracer` (OCV). The ammo-specific tracers are separate KD entries
  (listed in the component, OCV; fields not read, UNR).
- **Burst timing:** `ComponentFloatVar 0.067` on `burstASR`. That is about 15 shots
  per second if the float is the repeat interval (SI on the meaning, OCV on the value).
- Which hit entry an NPC shot reaches (SBR or an ammo variant) is UNR; `WFX-bulletburst`
  is the SBR burst entry only (SI that it is the default).

## PRU drone: `MOB_DroneFlyer00`

Seed template 4 is the PRU: `entity_templates.sql` line 9 (ELD). The package is
`Packages\Character\MOB_CA_DroneTank.upk`.

### Seed chain

| Event set | Name | Sequences, with event and `kismet_script_name` | Class |
|---|---|---|---|
| 802 | Energy shock ability source | 1866 (1001, `KIS-SA_Beam_Source`); 2894 (1002, `KIS-SA_Effect_Source`) | ELD |
| 803 | Energy shock effect target | 1867 (2002), 1868 (2003), 1870 (2005), 1871 (2006): `KIS-SA_Sing_Target`; 1869 (2004, `KIS-SA_Beam_Source`) | ELD |

- **Sources:** `event_sets.sql` lines 803 and 913; `event_sets_sequences.sql` lines
  3193–3205; `sequences.sql` lines 1269–1279 and 2851.

### Component (export 44, BodyComponent)

| Fact | Value | Class |
|---|---|---|
| `BodySetList` | export 47 `BS_MOB_DroneFlyer` | OCV |
| Mesh | `CompositeMeshes[0].SkelMesh` = export 260 `CADRoneFlyer`, BodySlot `Main` | OCV |
| `BodySockets` | none: the drone's weapon is part of the body mesh | OCV |
| `KismetData` | 7 entries, all exports of this same package | OCV |

The seven `KismetData` entries:

| BodySlot | Exports |
|---|---|
| `Abilities` | 49 |
| `Weapon` | 51, 52, 53, 54, 55 and 50 |

### Mesh sockets and bones (`CADRoneFlyer`, export 260)

| Item | Detail | Class |
|---|---|---|
| Sockets on `Origin` | Exports 264–277: `Priority`, `Buff`, `Miss`, `BeingName`, `DeBuff`, `ground`, `Text`, `Loot` | OCV |
| Sockets on `Body` | `Top`, `Center`, `LHigh`, `RHigh`, `LLow`, `RLow` | OCV |
| Weapon emitter socket | None. No socket is named for a barrel or muzzle. | OCV |
| `Barrel` bone | The FName `Barrel` is in export 260's serial data at byte 520. It also appears in the `Flyer_Animset` (export 40) track list (Body, TurretBase, Turret, Barrel, LFin02, RFin02, RFin01) and in `PhysicsAssetInstance` export 80. | SI |

**Emitter attachment.** Both drone weapon KD entries carry
`ComponentNameVar2 = Barrel`. So the beam's emitter point is the `Barrel` bone on
the turret, not a socket (SI).

### Kismet data (exports in `MOB_CA_DroneTank.upk`)

| Export | Entry | SequenceEvent | Key | Particle system | Float | Sound | Name vars | Class |
|---|---|---|---|---|---|---|---|---|
| 50 | `Mul_CA-DroneFlyerEnergyShock` | 2 | EnergyShock | `CA_Drones-VFX.CA-TazzerLongRange` | 1.8 | `zat_elec_1a/atk/sing` | `Barrel` (NameVar2) | OCV |
| 51 | `Mul_CA-DroneFlyerStatsisBeam` | 2 | Stasis | `VFX-Weapons.Mob.WFX-StasisBeam` | 1.8 | `zat_elec_1a/atk/sing` | `Barrel` (NameVar2) | OCV |
| 52 | `Mul_CA-Hit_EnergyShock` | 7 | EnergyShock | `VFX-Weapons.Explosions.WFX-ExploElectricSmall` | — | `zat_elec_1a/atk/hit` | — | OCV |
| 54 | `Mul_CA-HitCrit_EnergyShock` | 8 | EnergyShock | `WFX-ExploElectricSmall` | — | `zat_elec_1a/atk/hitcrit` | — | OCV |
| 53 | `Mul_CA-HitCrit2x_EnergyShock` | 9 | EnergyShock | `WFX-ExploElectricSmall` | — | `zat_elec_1a/atk/hitcrit` | — | OCV |
| 55 | `Mul_CA-HitGlances_EnergyShock` | 10 | EnergyShock | `WFX-ExploElectricSmall` | — | `zat_elec_1a/atk/hitglance` | — | OCV |
| 49 | `Mul_CA-DroneFlyerDeath` | 15 | Death | `CA_Drones-VFX.CA-FlyerDeath00` | 1.0 | — | `Body` | OCV |

- **Beam or projectile:** a beam. `KIS-SA_Beam_Source` draws `CA-TazzerLongRange`
  from the `Barrel` bone (SI from the seed script name plus the KD key, the bone and
  the beam asset).
  - The float 1.8 is probably the beam's lifetime in seconds (UNR).
  - `Mul_CA-DroneFlyerStatsisBeam` (`WFX-StasisBeam`) is a second beam under the key
    `Stasis`. No seed set examined here uses it (UNR).
- **Target set 803:** its `KIS-SA_Sing_Target` sequences find the four hit-kind
  entries (52–55) under key `EnergyShock` (SI). There is no miss entry
  (SequenceEvent 11) on the drone, so a missed energy shock has no impact effect
  (OCV that the entry is absent, SI that nothing draws).
- Six `ObjectRedirector`s in the package (69–74) carry older names:
  `Mul-CA_DroneFlyerTazzer`, `Mul_CA-HitGlance_EnergyShock` and others. They point
  at renamed entries and carry no data (OCV).

## Answers to NC-04 § Gaps

| Gap | Finding | Class |
|---|---|---|
| Effect 1092 has no event set | Nothing in the client attaches to a missing event set. The KD keys are reached only through a sequence, so 1092 draws nothing however the assets are set. | SI |
| Jaffa, drone and NID chains name `KIS-abilities_human` scripts, never `KIS-abilities_jaffa` | Not a family mismatch. The `KIS-SA_*` templates are generic: their `SeqAct_ComponentSlotData` nodes take `KeyName` and `SlotName` by variable link, and the assets come from the wearer's `ComponentKismetData`. The staff (`KD_WP-Jaffa`) and drone (`MOB_CA_DroneTank`) KD entries supply Jaffa- and drone-specific assets. | SI (KD and slot-node exports OCV; the KeyName each node is fed is UNR) |
| Sequence 1869 (event 2004) is `KIS-SA_Beam_Source` inside target set 803 | Seed anomaly: 1869 is a Source-type script (`KIS-SA_Beam_Source`) in a target set where the other four events use `KIS-SA_Sing_Target`. Whether that is a slip or intended is UNR, because the event-to-KD mapping is UNR. | UNR (the anomaly itself is ELD) |
| Sequence 1105 is `KIS-SA_Burst_Source` inside target set 289 | The same pattern as 1869, on the SMG. Not checked in the client. | UNR |
| Jaffa templates 159 and 160 have no `WP` component | The body default `WP-Human.WP_HandToHand_1A` applies: melee KD only. Sets 300 and 301 may draw (melee stance selection UNR, see above); set 3 cannot. | SI (ranged part), UNR (melee stance) |

## Unresolved

- The literal `KeyName` each `KIS-SA_*` slot node is fed (the `SeqVar_Named` or
  `SeqVar_Name` values). A Kismet variable decode or a live client probe of
  `SeqAct_ComponentSlotData` would settle it.
- The mapping of seed target events 2002–2006 to KD `SequenceEvent` bytes 7–11.
- Whether `ComponentFloatVar` means a delay, a repeat interval or a lifetime. It
  varies by entry (0.067 for the SMG burst, 1.8 for the beams, 2.0 for `JF-Sing_MELSM`).
- `KIS-abilities_jaffa.upk` was not dumped for script content. The seed never
  references it.
- Whether `Barrel` and `Muzzle` are bones or virtual sockets resolved by name in
  code. A Ghidra lookup of the `ComponentNameVar` attach path, or a live probe, would
  confirm it.
- The attach point of the muzzle flash and tracer entries, which carry no name var.
- Which hit entry an NPC shot reaches (SBR or an ammo variant) is UNR; `WFX-bulletburst`
  is the SBR burst entry only.
- None of this was checked against a running client. Everything is static package
  reading.
