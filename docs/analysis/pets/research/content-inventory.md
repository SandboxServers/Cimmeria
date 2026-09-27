# Pets — content / data inventory (read-only survey, origin/main, 2026-09-26)

Sources read: `db/resources/**` seeds on origin/main, `entities/defs/{enumerations,SGWPet,SGWPlayer}.def|xml`,
`deprecated/python/**`, `docs/gameplay/pet-system.md`, `docs/reverse-engineering/findings/pet-restoration.md`,
handoff pack v1.2 (`SGW_Jaffa_Goauld_Ability_Audit_v1.md`, `SGW_All_Classes_Progression_Final_v1.md`,
`abilities_full_recovered_master.json`), the QA client tree (`<client install>/SGWGame`:
`SourceCache.en-us/*.pak`, `CookedPC/Packages/**`, `Content/UI/Core/Pet/*.lua`, `binaries/SGW.exe` strings).
Parsed data dumps: the parsed seed dumps (not committed) (`data.json`, `pet_abilities.txt`).

Flag decode key: EAbilityFlags WeaponBar=1 DeploymentBar=2 UseWeaponRange=4 Toggled=8 Response=16
PetToggled=32 PetTrained=64 UsableWithDisguise=128 ForceStanding=256 DoNotActivate_AutoCycle=512
Deactivate_AutoCycle=1024 SpeedGrenade=2048 SpeedDeploy=4096 SpeedAttack=8192 **SpeedPet=16384**
ForceTargetGround=32768 **PetCommand=65536**. EEffectFlag: 1 Beneficial, 4 ClearOnDeath, 16 DontUseQR,
64 SeqOnFinish, 256 SeqOnFail, **512 SeqOnStart**, 131072 ResolveOnAbilityUser, 524288 AlwaysPersist.

## Headline corrections to existing docs

1. **No ability anywhere carries `PetCommand` (65536)** — not in the seed (1,886 rows) and not in the QA
   `CookedDataAbilities.pak` (1,886 entries, scanned). The client routes an ability to the pet *command* row
   only when `abilityInfo.isPetCommand` (PetInfo.lua:113 / PetContainer.lua:231). As shipped, every pet
   ability lands in the *ability* row and the command row is empty. Any "command" semantics must come from
   us (seed flag change) or be dropped.
2. **Summon abilities carry no effects at all.** 1643 Summon Jaffa, 1644 Lo'taur, 1645 Prime, 2825 Ashrak,
   2826/3491/3493/3495/3496/3497 Straegis, 1134/3136/3211 Summon Turret, 1352/3097 Prototype: `effect_ids = {}`
   in seed and cooked data. Only the Scientist turret line (962–966) has "Turret Spawn" effects, and those are
   description-only (no script, no NVP, no template id). The "Spawn Mob" effects (4099, 4104, 4109, 4113,
   5030…) exist only on `MS0xx_*` Atrea editor template/test abilities, flag 512 (SequenceOnStart), no
   `script_name`, no `effect_nvps` row, no event set. Cooked XML (e.g. `_4099`) has the same fields — **no
   parameter anywhere encodes a creature template.** The summon→template binding must be authored by us.
3. `docs/gameplay/pet-system.md` says "no Lo'taur / Prime / Ashrak / Turret entity templates exist by name" —
   partly wrong: templates **211 "Ashrak Assassin"** (hostile mob) and **219 "Storage Lo'taur"** (Harset banker)
   exist, but neither is a pet. More importantly, the **original pet template display names survive in
   `texts.sql`** (template `name_id` = `texts.moniker_id`, verified on 160/78/211/219/35/10):

   | text moniker | moniker_name | text |
   |---|---|---|
   | 8087 | DN_Pet_Jaffa_Tier_1 | Jaffa Soldier |
   | 28891 | DN_Pet_Lo'Taur_Tier_1 | Lo'Taur Servant |
   | 28892 | DN_Pet_Prime_Tier_1 | Jaffa Prime |
   | 28893 | DN_Pet_Ashrak_Tier_1 | (empty) |
   | 28894 | DN_Pet_Straegis_Tier_1 | (empty) |
   | 8088 | DN_Pet_SGU_Turret_General_3-13 | Prototype Turret |
   | 8089 / 8407 | DN_Pet_Turret_Prototype / DN_Pet_SGU_Turret_Prototype | (empty) |
   | 28879 / 28881 / 28882 | `DN_Pet_Turret_Offensive` / `_Defensive` / `_Advanced` | Offensive / Defensive / Advanced Turret |
   | 8060 | DN_Mb_Lucia_Summoned_Turret_18-30 | Burtonol Scientist's Turret (mob-summoned) |
   | 7961 | DN_Mb_Lucia_Summoned_Drone_18-30 | High Tech Drone (mob-summoned) |
   | 27377 | DN_Mb_Ms_Agnos_Summoned_Straegis_Fighter_Force_43 | Summoned Straegis Fighter (mob) |

   So the original had dedicated pet templates ("Tier_1" implies tiers); only their names were recovered.
   New pet templates should reuse these `name_id`s.
4. "Straegis is the one fully-wired example" is overstated: templates 77/78/79 exist with MOB_ bodies, but
   **faction 10 (hostile), no ability_set, no pet flags, never spawned by any spawnlist row**, and Summon
   Straegis is the Servant Lord **level-50 capstone**. Its render/animation path has never been exercised
   by our server.
5. Asgard "Drone" abilities (1174 Drone Shot, 1186 Strike, 1209 Melee AA, 1216 Ranged AA; `BS_Drones.upk`
   Attack/Defense/Scientific drones) are the **Asgard weapon/program system**, not SGWPet. Exclude.
6. `Col Marsh (pet)` (template 10, flags 24 = NoPetLeveling|NoPetTargeting, class `being`) is the Castle/Harset
   **escort companion**, not an SGWPet. Its follow AI (NA24, `npc_ai/follow.rs`) is the nearest reusable code.

## 1. Pet-related abilities

### 1a. Summon abilities (player)

| id | name | flags (decoded) | target | cd / warmup | effects | tree / access | notes |
|---|---|---|---|---|---|---|---|
| **1643** | Summon Jaffa | 18192 Response·ForceStanding·DoNotActivate_AC·Deactivate_AC·SpeedPet | Self (1) | 5 / **6** | none | **Goa'uld Servant Lord root, L1** (tree idx 1, trainer list 1) | display "Jaffa Soldier" (8087) |
| 1644 | Summon Lo'taur | 17936 Response·DNA_AC·Deact_AC·SpeedPet | Self | 5 / 6 | none | Servant Lord L10 (pre 1643) | "Lo'Taur Servant" (28891); healer |
| 1645 | Summon Prime | 17936 | Self | 5 / 6 | none | Servant Lord L15 (pre 1644) | "Jaffa Prime" (28892) |
| 2825 | Summon Ashrak | 16384 SpeedPet | Self | 5 / 6 | none | **not in any tree** | description text is a copy of Holy Warrior (seed bug) |
| 2826 | Summon Straegis | 18192 | Self | 5 / 6 | none | **Servant Lord L50 capstone** (pre 1643, 2069, 2846) | 3491/3493/3495/3496/3497 are warmup-0 duplicates |
| **962** | Summon Turret: Prototype | 18178 DeploymentBar·ForceStanding·DNA_AC·Deact_AC·SpeedPet | Ground (3), AERadius Melee, 0–500 | 0 / 4 | 5019 "Turret Spawn: If <1 Turret" (fl 272), 5018 "Dual Turret Spawn: If <2 Turrets, Dual Turrets Moniker" (fl 16) | **Scientist Robotics root, L1** | "Prototype Turret" (8088) |
| 963 | Summon Turret: Offensive | 18178 | Ground | 0 / 4 | 5025, 5024 (same pattern) | Robotics L10 | "Offensive Turret" (28879) |
| 964 | Summon Turret: Defensive | 18178 | Ground | 0 / 4 | 5027, 5026 | Robotics L15 | "Defensive Turret" (28881) |
| 965 | Summon Turret: Detection | 3 WeaponBar·DeploymentBar | Self | 15 / 0 | none | Robotics L20 | → ENTITYFLAG_DetectionPet |
| 966 | Summon Turret: Advanced ("Jammer") | 18178 | Ground | 0 / 4 | 5029, 5028 | Robotics L30 | "Advanced Turret" (28882) |
| 1213 | Summon Turret: Dual Turrets | 0 | Self | 0 / 0 | 5017 "Dual Turret Moniker" (fl 17) | Robotics L50 capstone | passive-style: grants the moniker that lifts cap to 2 |
| 1352 / 3097 / 3319 | Summon Turret: Prototype (variants) | 1808 / 1810 / 1808 | Self | 15 / 4 | none / none / 4915 "Effect" | none | revisions |
| 1134 / 3136 / 3211 | Summon Turret | 0 | Self | 360 / 10 / 10 | none | none, no grant found | mission-style, orphan |
| 937 | Call Drone | 0 | Self | 360 / 4 | 2026 "Drone Effect" | none, no grant found | orphan |
| 1862 | MS019_Summon Jaffa | 0 | Ground | 5 / 6 | none | none | editor revision |
| 1688, 1869, 1955, 2385, 2386, 2388, 2830, 2832–2834, 3372, 3379–3381, 3445–3448 | MS0xx_* summon templates | 18 / 1552 / 1680 | Ground AERadius | 0 | "Spawn Mob"/"Spawn Jaffa"/"Spawn Turret Prototype"/"Create Turret Device" (fl 512) + "Play/Attach PDA" (fl 533) | none | Atrea editor templates (dated 2008-06..11); descriptions copied from "Spawns an Anti-Personnel Mine" |

Every summon has `icon = IconMissing` except 962/1352/3097/3319 (`AbilityIcons001:Buff_Setup_Mastery`).
Cooked entries exist for all (same data). Summon abilities' `event_set_id` is NULL — but unwired VFX event
sets exist (§1e).

### 1b. Pet kit / pet-side abilities

**Jaffa:** 1652 Jaffa: Double Blast (fl 117 WeaponBar·UseWeaponRange·Response·PetToggled·**PetTrained**,
Target, 0–3000, cd 4, E2015 2 pulses −250F/−25H) — Servant Lord L20. Ability set 4 "Jaffa staff" (584, 710)
already drives Praxis Jaffa NPC combat.
**Lo'taur:** 1653 Heal Health (fl 1552, 0–800, cd 4, E4065 heal 10%), 3326 (E4924 focus heal 10%),
3327 (E4926 focus-regen buff 25 s), 3328 (E4928 defense +100 25 s), 3329 (E4930 defense −100 debuff).
None in a tree; 1653 not PetTrained.
**Prime:** 1654 Prime: Focus Degeneration (fl 84 UseWeaponRange·Response·PetTrained, E4086 −10% focus × 8) — Servant Lord L20.
**Ashrak:** 1613 Back Slash, 1620 Onslaught, 1621 Paralyze, 2857 Crippling Slash, 2858 Dervish (all es 300
melee, moniker 2650822895 = Ashrak branch), 2828 Assassin's Strike (fl 341 incl **PetTrained**, texts name it
`Pet:AssassinStrike`), MS020 2375–2379 dagger templates. 1613/1620/1621/2857/2858 are **Goa'uld Ashrak
branch player** abilities in the tree — only 2828 is pet-flagged.
**Straegis (mob kit):** 1156 Straegis: Disengage (es 1499), 2847 Dissonance (es 1497, 20-pulse PBAoE psi),
1240 Straegis Explode (es 1507).
**Turret (pet-internal):** 519 Fire Turret; 969 Suppression; 970/3308 Draw Fire; 973 Snare; 1152 Turret Burst
(ammo-moniker branching: TURRET_EnergyAmmo / TURRET_ContamAmmo); 2301/2302 Turret Burst (PetToggled|PetTrained);
1205 Cone; 1211 AOE; 3309 **Turret Self Destruct** (E4901 "Despawn Mob" + E4900 −800F AoE).
**Turret toggle "modes"** (PetToggled, some PetTrained; names from texts `DISPLAY_NAME_Pet_Turret_*`):
3296/3310/3314 Standard Attacks, 3311 Draw Fire Abilities, 3312 Crippling Attacks, 3313 Cone Attacks,
3315 Area of Effect Attacks. All `effect_ids = {}`.
**Unnamed pet toggles** (fl 48/96/112, "NO ABILITY DISPLAY NAME!"): 3291–3295, 3297–3303, 3307, 3332–3342;
3299 carries the Servant Lord moniker. Texts `DN_abil_Pet_Jaffa_{Offensive,Defensive,Balanced,Default}`
(28685/28694–28696: "Pet Offensive/Defensive/Balanced/Default") are the probable names of four of these —
mapping to ability ids not recovered.

### 1c. Owner abilities that act on pets

| id | name | fl | effects | tree |
|---|---|---|---|---|
| 1646 | Health Heal ("Pet Support" in workbook) | 0 | heal 10% | Servant Lord L5 — also universal starter (D-AT09 collision) |
| 1647 / 1651 | Focus Regeneration / Focus Heal: Target | 145 | — | Servant Lord L5 / L10 |
| 1648 / 2831 | Defend Your God | 145 | Defense +100; threat↑ group (1648) or regen +15% group (2831) | not in tree |
| 1650 | Lord's Concentration | 145, TCM_Group | **none** | Servant Lord L15 |
| 2824 | Holy Warrior (toggle, pet +100 Acc / −100 Def) | 1560 | E4220 AERadius Long, E4087 stance removal | Servant Lord L25 |
| 2852 | Heed Our Calling "Summons Chosen Pet Instantly" | 0 | E4968 "Pet Summon Speed increase" (fl 524288 AlwaysPersist) | Servant Lord L25 |
| 2839 | To The Death (pet +400 Acc 60 s, then pet dies) | 16 | E4121 acc, E4119 60 s timer, E4122 "Pet Death" (seq 1, fl 80) | Servant Lord L40 |
| 967 / 968 / 1207 / 1214 | Repair Turret: Percentage / Regenerate / Full / Restoration (revive) | — | E3211 "Heal Pet: Health", E3230, E3350, E3356 "Revive Turret Pet at Full Health" | Robotics L10/20/45/35 |
| 971 / 972 / 1206 / 1212 | Enhance Turret: RoF / Energy / Contamination / Shield | 112 PetToggled·PetTrained | buffs + TURRET_*Ammo monikers | Robotics L25/15/35/40 |
| 3071 | MS021 DefensiveUpgrade: ImprovedDroneLink | — | template | none |

### 1d. Monikers (unresolved names; from handoff §10 + co-occurrence)

`3748251909` Servant Lord branch (very strong), `17962629` Scientist Robotics / turret (strong — on every
turret row), `2650822895` Ashrak branch, `2684233211` Battle Lord, `1470900795` near-universal class marker,
`2936348458` = Commando_Demolitions (seeded name; stamped on the MS0xx summon templates because they were
cloned from a mine ability).

### 1e. Unwired VFX event sets ready for summons (`event_sets_sequences` → `sequences_nvp`)

| event set | name | sequence → Kismet / params |
|---|---|---|
| 1121 | Goauld summon source | 2292 KIS-SA_Effect_Target pfx `PFX-abilities.PFX-GoauldSummon` @Buff; 2904 interrupt sfx |
| 1122 | Goauld summon target | 2293 pfx `PFX-GoauldSummonTarget` @ground |
| 855 | Goauld pet source effect | 1928 pfx `PFX-GoauldPet` @Buff + `PFX-GoauldPetTarget` @ground; 2946 |
| 1120 | Goauld pet target | 2291 pfx `PFX-GoauldPetTarget` @ground |
| 1461 | Create turret effect source | 2775 pfx `PFX-CreateTurret` + `cTinker_Begin/Idle/End` anims; 2937 |
| 1454 / 744 | Create turret device target / Turret deploy target | 2762 / 1748 pfx `PFX-CreateTurretdevice`, sfx `abil_hum/obj/Turret1` |
| 1472, 1483–1485, 1489–1491 | repair / enhance / RoF / shield turret | KIS-SA_Effect_* |

No ability or effect references 855/1120/1121/1122 today; all PFX exist in `PFX-abilities.upk`.

## 2. The "Spawn Mob" effect and legacy scripts

- `effects` rows for spawn: `name/effect_desc = "Spawn Mob" | "Spawn Jaffa" | "Spawn Drone" | "Spawn Turret Prototype" | "Create Turret Device"`,
  `flags = 512` (EF_SequenceOnStart), `delay = 4` in cooked (`_4099 Delay="4"`), `tcm = TCM_Single`, no
  `event_set_id`, no `script_name`, no `effect_nvps`. `effect_nvps` has 21 rows total, none for pets.
- `EEffectClass` has `EFFECT_CLASS_Summon = 2` (enumerations.xml:391), but no seed/cooked column stores an
  effect class.
- Legacy Python: `cell/effects/` has only RangedEnergy/RangedPhysicalDamage/Reload/TestEffect; nothing
  summons. `cell/SGWPet.py` only sends empty ability/stance lists; `common/defs/PetCommand.py` loadAll=pass.
- `SGWPet.def` comment: *"If abilityToResolve is set for this entity during creation, the pet will resolve this
  ability when its spawn timer is up"* — i.e. the summon warmup/effect Delay is the pet's spawn timer.
- **Conclusion:** no parameter in any source encodes the creature template. The binding must be a new
  server-side table/mapping (ability → template), authored in `db/resources/`.

## 3. Candidate templates / models per summon

| Summon | Candidate | Evidence | Confidence |
|---|---|---|---|
| Jaffa (1643) | New template cloned from **160 Praxis Jaffa Guard** (BS_JaffaMale + AR_J_Praxis, ability set 4, faction 1) or **97/98 Praxis Jaffa**; name_id 8087 "Jaffa Soldier" | "Jaffa Pet" literal; Praxis = Goa'uld-allied look; template 160 spawned 11× in Harset and fights today; 35 SGC Ba'al Jaffa spawned 11× | **High** (body), Medium (exact armour set) |
| Prime (1645) | Jaffa officer look: **159 Praxis Jaffa Lieutenant** / 143 Ra's Officer; name_id 28892 "Jaffa Prime" | name "Jaffa Prime"; same proven body | High (body), Low (armour) |
| Lo'taur (1644) | **BS_GoauldMale/Female + `AR_G_Underlings`** (Goa'uld slave dress: AR_GM_U*/S*, textures `Goauld\Slaves\*`); alt. human body like 219 Storage Lo'taur; name_id 28891 "Lo'Taur Servant" | Lo'taur = human servant in lore; Underlings package is servant clothing | Medium-Low |
| Ashrak (2825) | **BS_GoauldMale + `AR_G_Ashrak`** armour (AR_GM_A{B,G,H,L,...}1–5, 5 tiers) + WP-Goauld blade; base template 211 (bare body, ability set 5 ribbon) | armour package exists but no template uses it; name 28893 text empty | Medium |
| Straegis (2826) | **78 Straegis Fighter** (`MOB_StraegisFighter`); alt. 79 Titan, 77 Beacon | "Summoned Straegis Fighter" mob text; Fighter has combat anim set | Medium; never spawned by us |
| System Lord (MS020 2388/2834) | none | only an editor template name "SummonStraegis/SystemLord" | Low — treat as Straegis |
| Turret family (962–966, 1213) | **No turret body set or mesh found** in CookedPC by name. | VFX complete (`VFX-Weapons.upk` WFX-Turret{Stan,Off,Def}T1[Amb/Tracer], WFX-TurretDeathT1, WFX-ExploTurretDestr; `PFX-abilities.upk` create/repair/enhance/shield; audio `weapTau/turret`). Kismet `KIS-abilities_scientist_m16` has KIS-turret_01..03, KIS-mobile_turret. Nearest meshes: `MOB_CA_DroneTank` (has Turret/TurretBase bones; = Prisoner retrieval unit 4/145), `GA-Props` Anti-AirTurret00–03 static meshes | **Blocker — model unresolved (Low)**; needs a Ghidra/UE package pass or maps grep for a static-mesh turret |
| Detection turret (965) | as above + ENTITYFLAG_DetectionPet (512) | — | Low |

Client Jaffa/Unas kits: `AR_J_*` sets (Bull, Asian, Cat, Cobra, Croc, Demon, Dragon, Eagle, Falcon, Horse,
Hyena, Jackal, Mayan, Morrigan, Naga, Praxis, Ra, Standard, Svarog, Tiki, Viking, Unas) × male/female
templates 82–139 all present. Unas 105–110/132–137 are Jaffa-body + AR_J_Unas (no Unas pet ability exists).

## 4. Archetype access

| Archetype (char_defs) | Branch | Pet abilities (level) |
|---|---|---|
| **Goa'uld** (char_def 10 M / 19 F, Praxis, Castle_CellBlock start) | Servant Lord (tree_index 2) | 1643 Summon Jaffa **L1 root** · 1646/1647/1651 L5–10 · 1644 Lo'taur L10 · 1645 Prime L15 · 1650 L15 · 1652 Double Blast L20 · 1654 L20 · 2824 L25 · 2852 L25 · 2839 L40 · **2826 Straegis L50 capstone** |
| Goa'uld | Ashrak (tree_index 0) | player dagger kit only (1613/1620/1621/2857/2858); **Summon Ashrak 2825 is in no tree** |
| **Scientist** (char_defs 20/22 Praxis, 21/23 SGU) | Robotics (tree_index 2) | **962 Prototype L1 root** · 963 L10 · 967 L10 · 964 L15 · 972 L15 · 965 L20 · 968 L20 · 971 L25 · 966 L30 · 1206/1214 L35 · 1212 L40 · 1207 L45 · **1213 Dual Turrets L50 capstone** |
| Commando | Stealth / Infiltration | 880 Sweep Detector L20 ("Pet: detects stealthed targets in a cone" — flags 0, no effects) |
| all others | — | none |

Trainer: only list 1 (debug, template 25) exists; it offers every tree node, so 962–966, 1213, 1643–1645,
1652, 1654, 2826 are trainable there. No `char_creation_abilities` row grants any summon. No item, mission
chain or dialog grants 937/1134/3136/3211/2825. Trainer topics "Train Goa'uld (Servant Lord)" (6258) and
"Train Scientist (Robotics)" (6241) exist in `dialog_set_maps`. The ability-tree campaign (PR #805) seeded
these as project reconstruction (`FINAL_V1`, secondary-evidence branch names); D-AT09 notes Goa'uld UAT is
not scheduled.

## 5. Pet data tables / enums

- `EPetStance` (seed + enumerations.xml:199): Passive 0, Defensive 1 (default), Aggressive 2. Client reserves
  5 stance slots; `PetContainer.lua:283/258` sets each stance **button's ID to its 1-based slot index** and
  sends `changePetStance(unitId, stanceId=window:getID())` — so the stance byte on the wire may be the slot
  index, not the enum value (hand to the code/RE agent; resolves open Q5).
- `ENTITYFLAG_*` (EEntityFlags): NoPetLeveling 8, NoPetTargeting 16, **DespawnOnOwnerLeash 32**, NoPassive 64,
  NoDefensive 128, NoAggressive 256 (per-template stance availability!), DetectionPet 512, **Pet 1024**,
  DespawnOnLeashFromOwner 32768 (mob-owned pets), PetUseOwnFaction 65536, PetWaitToDespawn 131072,
  DoNotLeaveDefaultState 262144. **No seeded template sets 1024**; only template 10 uses 8|16.
- `GENERICPROPERTY_PetOwnerId = 5`; `EUnitType` PetOwner 11 / PetOwnerTarget 15; `EBehaviorEventFlags.PetCommand = 1`;
  `ELogEvent LE_Pet = 8`; `EErrorAIStateReason EAI_ERROR_STATE_PetWithoutOwner = 8`;
  condition feedbacks EntityHasPet 189 / DoesNotHavePet 190 / PetCount{NotEqual..LessThan} 191–196 /
  IsPetOwner 235 / IsNotPetOwner 236; stat `speedPet = 111` (EStat).
- `RESOURCE_PetCommand` resource type exists; **no pet-command table/seed** anywhere; no cooked PAK for it.
- `SGWPlayer.def:80` **`knownPetAbilities` ARRAY<INT32> CELL_PUBLIC** — owner-side list of pet-trained
  abilities; pairs with `SGWPet.sendPetInfoToOwner(mailbox, ownerPetAbilities)`. Abilities flagged
  PetTrained (64) are the natural population: 971, 972, 1206, 1212, 1652, 1654, 2301, 2302, 2828, 3292–3295,
  3299, 3300, 3302, 3307, 3311–3315, 3332–3334, 3338, 3342.
- Text 871 "Invalid pet id" (error string, empty text).
- Items: Kit: Turret TC1–TC50 (2044–2054), Kit: Mobile Turret TC35–50 (2076–2079), pet mods 783 (pet ACC),
  2166 (pet damage), 2190 (pet cooldown), 2194 (summoned max health), 2197 (pet regen), 2198 (turret weapon
  swap), 2199 ("technological pets"): all mechanic-less (no monikers/effects).

## 6. Design intent recovered

- Goa'uld branches Ashrak / Battle Lord / **Servant Lord = "pet/minion master: summoning, commanding,
  protecting and enhancing servants/Jaffa/Ashrak"** (handoff audit, public GDC-era sources).
- Scientist Robotics = summon/repair/upgrade tree; "turret-internal attacks are not player tree nodes".
- **Pet cap:** turret effects encode "If <1 Turret … spawn" and "If <2 Turrets, Dual Turrets Moniker" → one
  turret, two with the Dual Turrets capstone. Client UI has 4 own-pet slots (Unit.Pet1..4) and 6 party-pet
  targeting slots. `PetCount*` condition feedbacks exist.
- **Summon timing:** 6 s warmup (Goa'uld), 4 s (turret); `SpeedPet` ability flag + `speedPet` stat 111 +
  "Pet Summon Speed increase" (2852 Heed Our Calling "Summons **Chosen** Pet Instantly") → SpeedPet very
  likely means "warmup scaled by speedPet", parallel to SpeedGrenade/SpeedDeploy/SpeedAttack. "Chosen pet"
  implies one selected active pet type.
- **Death/despawn:** To The Death kills the pet after 60 s; 3309 Self Destruct despawns; 1214 revives a
  destroyed turret (so a dead turret persists as a corpse/revivable state); `DespawnOnOwnerLeash` and
  `PetWaitToDespawn` flags; `onOwnerDeath/Leash/Respawn(shouldDespawn)`.
- **Leveling/XP:** `setPetLevel`, `NoPetLeveling` flag (pets level, presumably to owner), `transferXP = 1.0`.
- **Stances per template:** NoPassive/NoDefensive/NoAggressive flags → `onPetStanceList` is the per-pet
  filtered subset of EPetStance.
- No docs/design directory exists on main; no tutorial/dialog text mentions pets.

## 7. Persistence

None. No `pets` table in `db/sgw/` or `db/database.sql`; `SGWPet` props are all CELL_*, none Persistent;
`knownPetAbilities` is CELL_PUBLIC (not persistent); `saveToDB(playerDbId)` has no schema; legacy
`base/SGWPet.py` empty. The ability-driven summon + revive + "chosen pet" design reads as **ephemeral per
session** (re-summon after login); only "which pet is chosen" and trained pet abilities would need saving.

## Owner decisions needed

1. First pet: Goa'uld Jaffa (1643, template exists-in-kind) vs Scientist turret (962, no model).
2. Template binding: new `db/resources` table/column (ability → pet template) — seed-only, no migration.
3. Visuals: which Jaffa armour set; Lo'taur on Goa'uld-slave vs human body; Ashrak with AR_G_Ashrak.
4. Pet commands: add PetCommand (65536) flag to chosen seed abilities (e.g. Attack/Follow/Stay) or ship
   stances + abilities only (command row stays empty, as the 2009 data would).
5. Persistence: ephemeral (despawn on logout/zone) vs saved; one pet vs 4 slots; turret cap 1/2.
6. Faction: set PetUseOwnFaction or copy owner faction; XP split (transferXP 1.0 = owner gets full XP?).
7. Leash/teleport distance and despawn-on-owner-leash default (flags exist; values are x64dbg-only).
8. Goa'uld UAT is unscheduled (D-AT09 starter collision 1646) — shipping 1643 pulls Goa'uld UAT forward.
