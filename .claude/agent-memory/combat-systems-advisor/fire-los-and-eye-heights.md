---
name: fire-los-and-eye-heights
description: NA31 player fire-time LoS (onErrorCode 39, tolerance rays, auto-cycle one-shot notice) and where per-being eye heights come from (body_sets.eye_height from ref-mesh bounds)
metadata:
  type: project
---

Player fire-time line of sight shipped in NA31 (D-NA14, 2026-09-25): `use_ability/fire_los.rs`, refused with `onErrorCode(0, ability, 39)`. `ErrorStrings.pak` `_39` is the only LoS entry with text; `_40` NoLOS and `_42` OutsideWeaponRange carry only their monikers. It is a no-op without an occluder or on `Unknown`, because the navmesh must never refuse a player. NPC launches are not re-checked, since the fight tick already checked them.

Eye heights come from `resources.body_sets.eye_height`: the reference skeletal mesh's `FBoxSphereBounds` top less 0.12 m. Human male 1.81, Jaffa male 2.12, rat 0.15. The script pawn is stock UE3 (`BaseEyeHeight` 64, `CollisionHeight` 78) for every being and is useless for this. `BS_JaffaMale` had no `body_sets` row before NA31.

**Why:** anyone touching LoS, cover or the refusal code needs these sources and limits.
**How to apply:** read eyes through `SpaceManager::eye_height_of`, never a constant. Keep new being body sets measured, or the live-DB guard `every_spawned_being_body_set_has_an_eye_height` fails. Evidence is in `docs/reverse-engineering/findings/being-eye-heights.md`. Related: [[navmesh-los-reliability]].
