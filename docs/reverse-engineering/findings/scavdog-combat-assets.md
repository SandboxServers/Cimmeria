# ScavDog combat assets and audio ownership (NC-05)

> **Campaign**: [npc-combat-reference](../../analysis/npc-combat-reference/README.md), packet NC-05
> **Date**: 2026-11-27
> **Sources**: the owner's QA client at `..\SGW\Stargate Worlds-QA\Working`, read-only. Packages under `SGWGame\CookedPC\Packages\`, cooked data under `SGWGame\SourceCache.en-us\`, audio under `SGWGame\Content\audio\mobs\`. No client file was copied into the repo.
> **Tools**: `upk_info` (`crates/upk`, `--imports --names --exports --properties N`), Python `zipfile` over the cooked `.pak` files (they are ZIP archives of XML, [cooked-data-pipeline.md](../../engine/cooked-data-pipeline.md)), and a whole-tree byte search for the ScavDog and `mob_scav` strings.
> **Related**: [being-eye-heights.md](being-eye-heights.md) (ScavDog mesh bounds), [../../client/audio-voice-inventory.md](../../client/audio-voice-inventory.md) §7 (mob FEV projects).

## Summary

The ScavDog is a small asset set. `MOB_ScavDog.upk` holds one skeletal mesh, one AnimSet with eight sequences, one AnimTree, one BodySet, one PhysicsAsset and four local Kismet hooks. It has **one** attack animation, `sAttack1` (1.133 s). No sequence carries an AnimNotify, the package imports no AnimNotify or sound class, and `BodySounds` is empty. Nothing in the cooked client names the `mob_scav.fev` events, so which code plays them is still unknown. The package's Kismet hooks cover death and the five hit reactions only; none covers an attack.

The cooked ability catalogue has no event-set field. The `1433 Mob Strike → event set 300` link exists only in the seed, and the seed gives 983 Bite the same set 300. Neither link can be confirmed or refuted from client files. Nothing in the client ties 983 Bite, or 1433, to the ScavDog. The ScavDog's own stance data sets `MeleeDistance` to 0, and both abilities have `MinRange="0" MaxRange="0"`, so no client asset states an attack reach.

## Evidence classes

Every row below carries one of the six classes from the campaign spec: ORIGINAL CLIENT VERIFIED (cooked client or `SGW.exe`, file and export cited), EARLIEST LEGACY DATA (`db/resources` seeds or the Python server), CURRENT CIMMERIA IMPLEMENTATION, STRONGLY INFERRED, RECONSTRUCTED and UNRESOLVED.

## 1. Package inventory

`MOB_ScavDog.upk` (916,314 B; Epic 486, licensee 8, engine 3004; 286 names, 40 imports, 189 exports).

| Object | Export | Facts | Class |
|---|---|---|---|
| AnimSet `ScavDog_Anim` | 23 | 8 sequences (exports 15-22); `PreviewSkelMeshName = MOB_ScavDog.MOB_ScavDog` | ORIGINAL CLIENT VERIFIED |
| AnimTree `MOB_ScavDog_AnimTree` | 24 | Root `BlendBySpeed` (export 8, 3 children) → `PlaybackChooser` (AnimNodeBlendList, export 9) → `IdleRandomizer` (AnimNodeRandom, export 10, children Idle1/Idle2 = exports 13/14), `Walk` and `run` loops (exports 11/12), and `Playback` (`SGWGame.SGWAnim_SequenceListPlayer`, export 171, default `AnimSeqName = sHit`) | ORIGINAL CLIENT VERIFIED |
| BodySet `BS_MOB_ScavDog` | 26 | `AnimationTreeName = MOB_ScavDog.MOB_ScavDog_AnimTree`, `Physics = ScavDog_Physics`, one `BodyStances` entry (see §4), one `BodyMaterials` entry with slots `Main`, `Abilities`, `Loot` | ORIGINAL CLIENT VERIFIED |
| BodyComponent `MOB_ScavDog_00` | 25 | `BodySetList = [BS_MOB_ScavDog]`; `BodySounds`, `BodySockets`, `BodyParticles` and `CompositeMaterials` all have count 0; 6 `KismetData` entries (§3) | ORIGINAL CLIENT VERIFIED |
| PhysicsAsset `ScavDog_Physics` | 35 | 33 `RB_BodySetup` (exports 70-102), 32 constraints, `DefaultSkelMesh = MOB_ScavDog` | ORIGINAL CLIENT VERIFIED |
| SkeletalMesh `MOB_ScavDog` | 173 | Bip01 biped-derived rig plus `Bip01_Tail`/`Tail1`/`Tail2`, `L/R_HorseLink`, `Bone_AllFur`, `Bone_L/R_Ear`; textures `Scavenger_Ice_Dog00_*`, `Tol-Scavenger_Dog00_*` | ORIGINAL CLIENT VERIFIED |
| SkeletalMesh `MOB_Pitbull` | 172 | A second mesh in the same package (`Pitbull_*` textures, socket export 174). `MOB_ScavDog_00.CompositeMeshes[0].SkelMesh` is ref 174 = export 173 `MOB_ScavDog`, so the Pitbull mesh is not on the ScavDog body | ORIGINAL CLIENT VERIFIED |
| ObjectRedirector `ScavDog` | 34 | Redirector inside the package | ORIGINAL CLIENT VERIFIED |
| `GLB_Components.upk` import 15 | — | `MOB_ScavDog.BS_MOB_ScavDog`: the global component list is the only other cooked package that names the ScavDog | ORIGINAL CLIENT VERIFIED |

`VFX-ScavDog.upk` (488,807 B; 157 names, 50 imports, 328 exports) holds three ParticleSystems and their mesh: `VFX-Attack` (export 321), `VFX-Idle1Drool` (322), `VFX-Idle2Drool` (323) and StaticMesh `Par-Scavdog00` (324). It imports particle classes only: no AnimNotify, no SoundCue, no Kismet class. **ORIGINAL CLIENT VERIFIED.** No cooked file references `VFX-ScavDog` or `VFX-Attack` from outside that package, and `MOB_ScavDog_00.BodyParticles` is empty, so how `VFX-Attack` gets spawned is **UNRESOLVED**.

## 2. Animation sequences by role

All rows are `MOB_ScavDog.upk` → `ScavDog_Anim.<SequenceName>`. None of the eight has a `Notifies` property, and the name table has no `Notifies` or `AnimNotify*` entry.

| Role | Export | SequenceName | Length (s) | Frames | Priority | Notes | Class |
|---|---|---|---|---|---|---|---|
| Idle | 19 | `Idle1` | 1.500 | 45 | 2 | via `IdleRandomizer` | ORIGINAL CLIENT VERIFIED |
| Idle | 20 | `Idle2` | 1.700 | 51 | 2 | via `IdleRandomizer` | ORIGINAL CLIENT VERIFIED |
| Move (walk) | 22 | `Walk` | 1.067 | 32 | 8 | `MovementCycleDistance = 200` | ORIGINAL CLIENT VERIFIED |
| Move (run) | 21 | `run` | 0.667 | 20 | 8 | `MovementCycleDistance = 650` | ORIGINAL CLIENT VERIFIED |
| Combat idle | — | none | — | — | — | No combat-idle sequence exists; `BodyStances[0].Combat = 0` | ORIGINAL CLIENT VERIFIED |
| Attack | 16 | `sAttack1` | 1.133 | 34 | 12 | The only attack sequence; no bite-specific variant | ORIGINAL CLIENT VERIFIED |
| Hit reaction | 18 | `sHit` | 0.900 | 27 | 10 | Default of `Playback` | ORIGINAL CLIENT VERIFIED |
| Death | 17 | `sDeath1` | 1.700 | 51 | 100 | | ORIGINAL CLIENT VERIFIED |
| Death pose | 15 | `sDeath1Pose` | 0.033 | 1 | 100 | Held corpse pose | ORIGINAL CLIENT VERIFIED |

The `s` prefix marks the sequences that `SGWAnim_SequenceListPlayer` plays on demand; idle and movement run off the blend tree. That reading is **STRONGLY INFERRED** from the tree layout, not traced in `SGW.exe`.

## 3. Kismet hooks

`MOB_ScavDog_00.KismetData` links six `KD_MOBs` imports. The package also carries four local `ComponentKismetData` exports. Both sets cover death and hits only.

| Source | Entry | Target | SequenceEvent / KeyName | Class |
|---|---|---|---|---|
| BodyComponent `KismetData[0]` (slot `Abilities`) | import 26 | `KD_MOBs.MOB-Hit` | — | ORIGINAL CLIENT VERIFIED |
| `KismetData[1]` (`Abilities`) | import 27 | `KD_MOBs.MOB-HitCrit` | — | ORIGINAL CLIENT VERIFIED |
| `KismetData[2]` (`Abilities`) | import 28 | `KD_MOBs.MOB-HitCrit2x` | — | ORIGINAL CLIENT VERIFIED |
| `KismetData[3]` (`Abilities`) | import 29 | `KD_MOBs.MOB-HitGlance` | — | ORIGINAL CLIENT VERIFIED |
| `KismetData[4]` (empty slot) | import 30 | `KD_MOBs.MOB-HitMiss` | — | ORIGINAL CLIENT VERIFIED |
| `KismetData[5]` (`Abilities`) | import 25 | `KD_MOBs.MOB-Death` | — | ORIGINAL CLIENT VERIFIED |
| Export 27 | `Mul_MOB_ScavDog_Death` | local | 15 / `Death` | ORIGINAL CLIENT VERIFIED |
| Export 28 | `Mul_MOB_ScavDog_Hit` | local | 7 / `Hit` | ORIGINAL CLIENT VERIFIED |
| Export 29 | `Mul_MOB_ScavDog_HitCrit` | local | 8 / `Hit` | ORIGINAL CLIENT VERIFIED |
| Export 30 | `Mul_MOB_ScavDog_HitCrit2x` | local | 9 / `Hit` | ORIGINAL CLIENT VERIFIED |

Each local export has a `Recoil` subobject. No entry names an attack.

## 4. Stance, sockets and collision

**Stance.** `BS_MOB_ScavDog.BodyStances[0]` = `{AnimationSet = export 24, Combat = 0, MeleeDistance = 0, Weapon = 0, PadValue = 0}` (all bytes). **ORIGINAL CLIENT VERIFIED.** What the client reads `MeleeDistance` for is **UNRESOLVED**: it was not traced in `SGW.exe`.

**Sockets** (SkeletalMeshSocket exports 174-181):

| Export | Socket | Bone | RelativeLocation (UU) | Class |
|---|---|---|---|---|
| 174 | `Loot` | `Origin` | 0 | ORIGINAL CLIENT VERIFIED (on `MOB_Pitbull`) |
| 175 | `ground` | `Origin` | 0 | ORIGINAL CLIENT VERIFIED |
| 176 | `Priority` | `Origin` | (0, 0, 226.75) | ORIGINAL CLIENT VERIFIED |
| 177 | `Loot` | `Origin` | 0 | ORIGINAL CLIENT VERIFIED |
| 178 | `Buff` | `Origin` | (85, 0, 176) | ORIGINAL CLIENT VERIFIED |
| 179 | `DeBuff` | `Origin` | (-85, 0, 179.5) | ORIGINAL CLIENT VERIFIED |
| 180 | `Top` | `Bip01_Head` | (11.43, 21.15, 0), yaw 6095 | ORIGINAL CLIENT VERIFIED |
| 181 | `Center` | `Bone_AllFur` | (0.71, 51.16, 3.42) | ORIGINAL CLIENT VERIFIED |

There is no mouth, jaw or weapon socket. `Buff` and `Center` are the socket names ability Kismet passes (`pfx1Socket`, `pfx2Socket`) in the cooked event sets, so ability effects attach at those sockets, not at the head.

**Collision.** Every `RB_BodySetup` holds one `KBoxElem`. The head and neck boxes are the largest:

| Export | Bone | Box X / Y / Z (UU) | Class |
|---|---|---|---|
| 98 | `Bip01_Head` | 103.49 / 33.07 / 50.43 | ORIGINAL CLIENT VERIFIED |
| 97 | `Bip01_Neck` | 39.76 / 68.30 / 62.51 | ORIGINAL CLIENT VERIFIED |
| 81 | `Bip01_Spine` | 28.94 / 33.62 / 29.87 | ORIGINAL CLIENT VERIFIED |
| 70 | `Bip01_Pelvis` | 21.38 / 34.17 / 25.76 | ORIGINAL CLIENT VERIFIED |

The head box offset in bone space is (-2.67, 13.99, 0) UU. Reading X/Y/Z as full edge lengths follows stock UE3 `KBoxElem` and is **STRONGLY INFERRED** for this licensee build. Turning the head box into a world-space snout position needs the mesh reference pose, which was not decoded.

## 5. `mob_scav.fev` ownership

`Content\audio\mobs\mob_scav.fev` defines **five** events under `characters`: `attack`, `breathe`, `death`, `hit`, `shake`. `attack` is **one** event with five wave variants (`m_mob_scav_stance_attack1.wav` … `attack5.wav`), not five events. `breathe` has 2 variants, `death` 3, `hit` 5, `shake` 3. **ORIGINAL CLIENT VERIFIED** (strings in the FEV).

A byte search of the whole client tree for `mob_scav` finds it in only three places: the FEV and FSB themselves, and `Engine\Config\GameplayEngine.ini` (`mob_scav=mobs\mob_scav.fev`, the project registration). Every `mob_scav` hit inside a `.upk` is part of `MOB_ScavDog`. Cooked packages here are uncompressed (`GLB_Components.upk` header: compression 0), so a byte search reaches their name tables.

| Event | Referenced by a ScavDog notify? | Referenced anywhere in cooked data? | Class |
|---|---|---|---|
| `attack` (5 variants) | No: no notify exists | No | ORIGINAL CLIENT VERIFIED (absence) |
| `breathe` | No | No | ORIGINAL CLIENT VERIFIED (absence) |
| `death` | No | No | ORIGINAL CLIENT VERIFIED (absence) |
| `hit` | No | No | ORIGINAL CLIENT VERIFIED (absence) |
| `shake` | No | No | ORIGINAL CLIENT VERIFIED (absence) |
| Who plays them | — | — | UNRESOLVED |

The event path is presumably built at run time, for example by `KD_MOBs` Kismet or code that joins a project key with an event name, but that is a guess. Settling it means tracing the FMOD event-get call sites in `SGW.exe` or decompiling `KD_MOBs`. This contradicts [audio-voice-inventory.md](../../client/audio-voice-inventory.md) §7 ("Referenced: Yes — mob animation notifies trigger these. Confidence: High") for `mob_scav`; that row should be downgraded.

## 6. Abilities 1433 and 983 in cooked data

From `SourceCache.en-us\CookedDataAbilities.pak` and `CookedDataEffects.pak` (entry names `_<id>`):

| Ability | Cooked fields | Effect | Class |
|---|---|---|---|
| 1433 `Mob Strike` | `AbilityTypeId=5`, `TargetTypeId=2`, `IsRanged=false`, `MinRange=0`, `MaxRange=0`, `CooldownSeconds=2`, `Flags=261`, `Velocity=100`, desc "Melee Attack -125F -15H" | 1717 `Strike Damage` | ORIGINAL CLIENT VERIFIED |
| 983 `Bite` | same shape, `CooldownSeconds=3`, `Flags=1` | 1092 `Bite Damage` | ORIGINAL CLIENT VERIFIED |
| 984 `Infected Bite` | `CooldownSeconds=25` | 1758 `Infected Bite Effect` (4 × 25 H), 1093 `Bite Damage` | ORIGINAL CLIENT VERIFIED |

The cooked ability record has **no event-set attribute and no creature field**. No ability, effect or Kismet record mentions a ScavDog, dog or `mob_scav`. **ORIGINAL CLIENT VERIFIED.**

Id collisions in other cooked categories are unrelated namespaces: `CookedDataKismetSetEvent` `_983` is a revive particle set, `_1433` is "Focus radius 5m deployable target", and `CookedDataKismetSeqEvent` `_983`/`_1433`/`_1092` are dialog-VO sequences. They are not evidence for either ability.

Cooked event set 300 (`CookedDataKismetSetEvent.pak` `_300`) = `KIS-abilities_human.KIS-SA_Melee_Source` (EventID 1001, seq 346) plus `KIS-abilities_human.KIS-SA_Effect_Source` (EventID 1002, seq 2959, `sfx1 = abil_hum/statecast/interupt`). **ORIGINAL CLIENT VERIFIED.** It is a generic source-side melee set with no creature or ScavDog content.

The seeds give both abilities `event_set_id = 300` (`db/resources/Abilities/Seed/abilities.sql`, rows 1433 and 983; set 300 is named "Generic melee weapon source" in `db/resources/Events/Seed/event_sets.sql`). **EARLIEST LEGACY DATA.**

## 7. Answers to the packet questions

1. **Is 1433 → event set 300 supported, contradicted or unknown?** **Unknown from the client.** The cooked ability carries no event-set field, so the client neither supports nor contradicts it. The link exists only in the seed (EARLIEST LEGACY DATA). Event set 300 is a generic melee source set and is internally consistent with a melee ability, which is the most the client can show.
2. **Is 983 Bite tied to ScavDog?** **No client evidence.** No cooked record or package links 983 to the ScavDog. The ScavDog has a single generic attack animation and no bite-specific sequence, VFX or sound. Pairing them stays RECONSTRUCTED at best.
3. **Physical attack reach in metres?** **Not derivable from client assets.** `BodyStances[0].MeleeDistance = 0` and both abilities have `MinRange=0 MaxRange=0` (ORIGINAL CLIENT VERIFIED). The only physical bounds are the collision boxes (head box 103.49 UU long, about 1.03 m at 100 UU per metre) and the mesh height (0.3-113.5 UU, [being-eye-heights.md](being-eye-heights.md)). Neither is a reach. Any reach value the server uses is RECONSTRUCTED.

## 8. Corrections to existing docs

- [audio-voice-inventory.md](../../client/audio-voice-inventory.md) §7: the `mob_scav` row says notifies trigger its events at High confidence. For the ScavDog that is contradicted: there is no notify, and the "5 attack" count is 5 variants of one `attack` event.
- [npc-combat-reference README](../../analysis/npc-combat-reference/README.md) F6: "983 Bite (effect 1092, no event set and no script)" disagrees with the seed, which sets `event_set_id = 300` for 983 as for 1433. The cooked client has no event-set field for either.

Neither file is in NC-05's edit set; the corrections are left to their owners.

## Open questions

- Which code plays `mob_scav/characters/*` events: `KD_MOBs` Kismet, a hard-coded path in `SGW.exe`, or the stance `Weapon`/`Combat` bytes? Next step: decompile the `KD_MOBs` package and trace FMOD `getEvent` callers in Ghidra.
- What triggers `VFX-ScavDog.VFX-Attack`?
- What `SGW.exe` does with `BodyStance.MeleeDistance` and whether 0 means "use a default".
