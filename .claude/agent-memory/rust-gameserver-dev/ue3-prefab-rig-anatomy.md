---
name: ue3-prefab-rig-anatomy
description: UE3 cooked-map facts confirmed while scoping a ring-transporter rig clone - component prop offset 8, PrefabInstance ArchetypeToInstanceMap layout, IMF_RelativeToInitial Matinee keys, FMOD-string sound refs, and which CookedPC packages are compressed.
metadata:
  type: project
---

Confirmed 2026-09-19 against SGW QA CookedPC (Epic 486 / licensee 8) while scoping
"can we patch a ring rig into a map chunk". Complements [[ue3-staticmesh-extraction]]
and [[ue3-absent-property-defaults]].

**Why:** these facts each decide whether a whole class of patch work is cheap or
impossible, and most are invisible unless you decode an export by hand.

**How to apply:** read before any `.umap`/`.upk` authoring, splicing, or property-decode task.

- **`ActorComponent` tagged properties start at byte 8**, not 0/4/32. `inspect-export
  --props` used to probe `[0, 4, 32]` and take the FIRST non-empty parse; offset 0 on a
  `StaticMeshComponent` resolves one garbage FName pair (it printed a bogus `TabName`)
  and stops, so the real property list was never reached and `StaticMesh` looked absent.
  Fixed by scoring candidates `[0,4,8,12,32]` on property count, plus a `--prop-offset N`
  override. If a component "has no properties", suspect the offset, not the data.
- **A cooked prefab instance's component carries NO `StaticMesh`.** The mesh lives on the
  archetype, which is an *import* (`GLB-Global.<Prefab>.<Arc>.StaticMeshComponent0`), so it
  is unreadable from the map alone — resolve the export-table `archetype` field and open
  the source package. Standalone (non-prefab) actors DO carry an explicit `StaticMesh`.
- **`PrefabInstance.ArchetypeToInstanceMap` is post-property binary**: `i32 count` then
  `count` x (`i32 archetypeRef`, `i32 instanceRef`). It is the only thing that ties a
  prefab's level actors together; there is no naming convention to fall back on
  (every rig actor is just named `InterpActor` / `StaticMeshActor`).
- **Cooking does NOT strip `Prefab` objects.** `GLB-Global.upk` still holds the full
  `Prefab` export (with `PrefabArchetypes` / `RemovedArchetypes` / `PrefabSequence`),
  its archetype actors, their components, and a `Sequence` template.
- **Matinee move tracks in the ring rigs are `MoveFrame = 1` (`IMF_RelativeToInitial`)**,
  and `EulerTrack` is all zeros. Relocating a cloned rig therefore only needs the actor
  `Location` properties rewritten — the several-KB `InterpTrackMove` key blobs copy verbatim.
- **`SeqAct_PlaySound` references sound by FMOD event *string*** (`PlaySoundEvent`, e.g.
  `"prp_gen/rings/transport"`), not a `SoundCue` object. No audio import to merge.
- **Compression split:** every `CookedPC\Packages\*.upk` sampled is UNCOMPRESSED
  (`compression_flags = 0`); only `Maps\**\*.umap` chunks are LZO (flag 2). The shipping
  client demonstrably loads uncompressed packages, so a patcher can skip recompression
  (unproven for `.umap` specifically — same FArchive path, but not tested).
- **`crates/upk` is read-only.** The only package *writer* in the repo is
  `crates/navmesh-extractor/src/test_support/package_bytes.rs` — an uncompressed Epic-486
  encoder that assumes empty `ComponentMap`/`GenNetObjCount` and licensee 0. Usable as a
  skeleton, not as a round-trip writer.
- **The same rig ships in two shapes.** `Castle_CellBlock-fffefffd` hosts it as a
  `PrefabInstance` (needs 16 `GLB-Global` archetype imports + `Engine.Prefab`);
  `-fffeffff` hosts an equivalent rig fully **de-prefabbed** — standalone actors with
  explicit `StaticMesh` properties and zero archetype imports. The de-prefabbed copy is
  the cheaper clone template by a wide margin.
