# Being Eye Heights and the No-Line-of-Sight Feedback Code

> **Last updated**: 2026-09-25
> **Source**: cooked client packages under `SGWGame\CookedPC\Packages\Character\` and `SGWGame\Content\FRScript\` (`Engine.u`, `SGWGame.u`), read with `crates/upk-objects` (`query-index scan`, `inspect-export --props --hex`); `SourceCache.en-us\ErrorStrings.pak`; the seeded `resources` schema
> **Confidence**: HIGH for the mesh bounds, the unit scale and the pawn defaults (byte-exact reads, self-checking record). MEDIUM for "eye = top of the reference mesh less 0.12 m" as the eye rule (derived, not recovered). HIGH for error code 39 as the only line-of-sight feedback with text; MEDIUM for how the client presents it (the `onErrorCode` handler is untraced).
> **Issue/packet**: NA31 (`docs/analysis/npc-ai-restoration/work-packets.md`), decision D-NA14
> **Related findings**: [combat-formulas-client-evidence.md](combat-formulas-client-evidence.md) (E8: the LoS feedback text), [cover-system.md](cover-system.md) (`ECoverHeight`), [../../engine/navmesh-build-pipeline.md](../../engine/navmesh-build-pipeline.md) §1 (100 UE3 units per BigWorld metre)

---

## TL;DR

The client data has no per-being eye height, but every body set has a
measurable standing height. The script pawn is one class,
`SGWGamePawn extends Pawn`, and it inherits the stock UE3 defaults for every
being: `BaseEyeHeight` 64, `CollisionHeight` 78 (a half-height), and
`CollisionRadius` 34. Those are Unreal Tournament numbers, not SGW ones. The
reference skeletal mesh of each body set carries its own bounds. A human male
is 1.93 m tall, a Jaffa male 2.24 m, an Asgard 1.37 m and a rat 0.27 m.

Cimmeria seeds `resources.body_sets.eye_height` from those bounds. The value
is the top of the bounds less 0.12 m, or the centre of the bounds when that is
higher. Line of sight casts between the two beings' eyes. Where a body set has
no reference mesh (props and terminals), the default is 1.5 m.

The client's feedback for a refused shot is `onErrorCode` `ErrorCodeID` 39,
`CONDITION_FEEDBACK_LOS`. Its text is "You do not have Line of Sight to your
target". Code 40 (`CONDITION_FEEDBACK_NoLOS`) has only its moniker as text.

---

## 1. The pawn defaults: one value for every being

| Export | Property | Value (UU) | Metres |
|---|---|---|---|
| `Engine.u` `[10292]` `Default__Pawn` | `BaseEyeHeight` | 64.0 | 0.64 |
| | `EyeHeight` | 54.0 | 0.54 |
| | `CrouchHeight` | 40.0 | 0.40 |
| `Engine.u` `[12549]` `CollisionCylinder` | `CollisionHeight` (half) | 78.0 | 0.78 |
| | `CollisionRadius` | 34.0 | 0.34 |
| `SGWGame.u` `[553]` `SGWGamePawn.TargetingCylinder` | `CollisionHeight` / `CollisionRadius` | 78.0 / 34.0 | 0.78 / 0.34 |

- The tagged properties of both cylinder exports start at offset 16. That is
  not one of `inspect-export`'s default candidate offsets, so the read needs
  `--prop-offset 16`.
- `SGWGame.u` has three pawn classes: `SGWGamePawn extends Pawn`,
  `SGWOfflineGamePawn extends SGWGamePawn` and
  `SGWSequenceTargetDummy extends SGWGamePawn`. There is no `SGWPlayerPawn`
  or mob pawn class.
- `Default__SGWGamePawn` (`[535]`) overrides no eye property. Its
  `CollisionCylinder` (`[551]`) has no properties of its own.

So the scripts give every being the same cylinder: 1.56 m tall, with eyes
0.64 m above its centre, at 1.42 m. A human male mesh is 1.93 m tall, so these
values were never tuned for SGW, and they cannot tell a rat from a Jaffa.

## 2. The body sets' reference meshes

`resources.body_sets` maps a body set to its reference skeletal mesh
(`ref_skeletal_mesh`). `USkeletalMesh::Serialize` writes `Bounds`
(`FBoxSphereBounds`: `Origin` 3 × f32, `BoxExtent` 3 × f32, `SphereRadius`
f32) as the first native field after the tagged properties. The record checks
itself: `SphereRadius` equals `|BoxExtent|`. The scan accepts only a 28-byte
window that satisfies that to within 1%. That check is how the parser-misread
exports (`MOB_AMBRat`, `JM_RefSkelMesh`) were still located. It also rejects
the all-zero records on some reference meshes that are only a skeleton.

UE3 Z is up, and 100 UU is one metre (navmesh-build-pipeline §1). The mesh
origin is at the soles. For example, `BS_Asgard` `BS_AM_Base_Feet00` spans
z 0.0-9.7.

| Body set | Reference mesh | z (UU) | Eye (m) | Templates |
|---|---|---|---|---|
| `BS_HumanMale` | `HM_RefSkelMesh` | 0.0-193.0 | 1.81 | 37 |
| `BS_HumanFemale` | `HF_RefSkelMesh` | -0.2-182.7 | 1.71 | 7 |
| `BS_JaffaMale` | `JM_RefSkelMesh` | 0.0-224.0 | 2.12 | 50 |
| `BS_JaffaFemale` | `JF_RefSkelMesh` | -0.2-214.0 | 2.02 | 30 |
| `BS_GoauldMale` | `GM_RefSkelMesh` | 0.0-210.8 | 1.99 | 12 |
| `BS_GoauldFemale` | `GF_RefSkelMesh` | 0.0-192.8 | 1.81 | 4 |
| `BS_Asgard` | `AM_RefSkelMesh` | 0.0-137.3 | 1.25 | 3 |
| `NPC_Child` | `NPC_Child` | -0.2-138.4 | 1.26 | 5 |
| `MOB_AMBRat` (`BS_MOB_Rat`) | `MOB_AMBRat` | -0.3-27.0 | 0.15 | 1 |
| `MOB_ScavDog` | `MOB_ScavDog` | 0.3-113.5 | 1.01 | 1 |
| `MOB_AN_Android` | `AN_Android` | -0.6-182.8 | 1.71 | 0 |
| `MOB_CA_DroneTank` (`BS_MOB_DroneFlyer`) | `CADRoneFlyer` | 173.4-343.2 | 3.31 | 2 |
| `MOB_Goauld_Drone` | `MOB_GoauldDrone00` | 185.3-362.8 | 3.51 | 1 |
| `MOB_AncientDrone` | `AncientDrone_Mesh` | 202.6-445.1 | 4.33 | 1 |

The seed has the other 21 measured rows, and the script output is reproducible
(see [Method](#method)). The drones' meshes float 1.7-2.0 m above their
origin, which is where the client draws them, so their eyes are that high too.

**Eye rule.** The humanoid head meshes put the head's centre 0.11-0.13 m below
the top of the reference mesh:

| Head mesh | Centre z (UU) | Reference top (UU) | Offset (m) |
|---|---|---|---|
| `BS_HM_Base_Head00` | 182.1 | 193.0 | 0.11 |
| `BS_HF_Head00` | 171.9 | 182.7 | 0.11 |
| `BS_JM_Base_Head00` | 210.8 | 224.0 | 0.13 |
| `BS_JF_Head00` | 201.4 | 214.0 | 0.13 |
| `BS_GM_Base_Head` | 198.8 | 210.8 | 0.12 |
| `BS_GF_Base_Head00` | 181.5 | 192.8 | 0.11 |
| `BS_AM_Base_Head00` | 124.9 | 137.3 | 0.12 |

`eye_height = max(top - 0.12, centre)`. The `centre` floor is for tiny
creatures: without it, `LennyBaby` (0.21 m tall) would get its eyes at
0.09 m.

**Not in the data.** Nothing seeded or scripted scales a body set per
template. `body_sets`, `body_component_visuals` and `entity_templates` have no
`DrawScale` column. If a map or a Kismet sequence rescales a pawn, the eye
height is off by the same factor.

**Positions are at the feet.** Server positions for NPCs and players are at
floor height: `Hallway01_Guard`'s seed row is at y 39.552, and Lomiada's
`npc_ai.los` rows there read y 39.5515. So "above the position" means above
the soles.

## 3. `HEIGHT_LOS` is not an eye height

`ECoverHeight` (`entities/defs/enumerations.xml`) is `HEIGHT_Low` 0,
`HEIGHT_Mid` 1, `HEIGHT_High` 2 and `HEIGHT_LOS` 3. The cover prefab spawner
maps it to 0.71 m, 1.067 m, 1.524 m and 2.524 m
([combat-formulas-client-evidence.md](combat-formulas-client-evidence.md) Q2).
It classifies cover-node height, so `HEIGHT_LOS` is the tier tall enough to
block line of sight.

## 4. The no-line-of-sight feedback code

`ErrorStrings.pak` (QA build `SourceCache.en-us`; the server build in
`data/cache` matches), zip members `_39` to `_43`, `COOKED_ERROR_TEXT`
records:

| `ErrorID` | `MonikerName` | `Text` |
|---|---|---|
| 39 | `CONDITION_FEEDBACK_LOS` | "You do not have Line of Sight to your target" |
| 40 | `CONDITION_FEEDBACK_NoLOS` | the moniker, with a trailing space |
| 41 | `CONDITION_FEEDBACK_InsideWeaponRange` | the moniker |
| 42 | `CONDITION_FEEDBACK_OutsideWeaponRange` | the moniker |
| 43 | `CONDITION_FEEDBACK_OutsideDistanceCheck` | the moniker |

Only 39 has authored text. Cimmeria's out-of-range refusal already sends 42,
and its text is also only the moniker. The refusal is
`onErrorCode(SystemID 0 = ERRORCODE_SYSTEM_Ability, InstanceID = ability id,
ErrorCodeID 39)`, seven bytes: `00 <ability id, i32 LE> 27 00`.

`Event_NetIn_onErrorCode` is registered at `0x00d77f00`, and
`CallbackImpl<Event_NetIn_onErrorCode>` exists in the RTTI, so a subscriber
exists. Where its handler shows the text (chat, the error line, the ability
widget) has not been traced. The UAT for NA31 observes it.

## Method

- **Pawn defaults.** `query-index scan <Engine.u|SGWGame.u> <Class>` and
  `inspect-export <pkg> <index> --props [--prop-offset 16]`.
- **Mesh bounds.** Scratch scripts `skel_bounds.py` and
  `measure_body_sets.py` (session scratchpad, not committed). For each
  `body_sets` row, they find the package named by the body set's prefix under
  `CookedPC`, find the export named by `ref_skeletal_mesh` with
  `query-index scan <pkg> SkeletalMesh`, and scan the
  `inspect-export --props --hex` dump for the first self-consistent
  `FBoxSphereBounds`. `AR_J_Ra.BS_RaJaff` (`Ra_500`, no export) and
  `WP-Human.BS_Mine` (`WP_Invisible_00_psk`, no bounds) are left NULL.
- **Error strings.** `ErrorStrings.pak` is a zip of XML members named `_<id>`.

## Open questions

- **Crouch.** No crouched eye height is modelled. The server does not drive a
  crouch or cover pose (D-NA10), so a being in cover looks from its standing
  eyes.
- **Per-instance scale.** See "Not in the data" in section 2.
- **Presentation of code 39.** The UAT will show where it appears.
