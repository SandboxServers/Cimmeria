---
title: "Ancient asset inventory across the QA client"
type: analysis
audience: map authors, engineers
last_updated: 2026-10-06
---

# Ancient asset inventory across the QA client

The inventory swept the entire local QA `CookedPC` tree: **361 `.upk` files**
and **5,078 `.umap` files**. It searched package metadata for `AN-`, `ANM-`,
`AGN-`, `ATL-`, `Ancient`, `Atlantis`, `Pegasus` and `Laro` names, parsed the 62
matching packages and 336 matching chunks, and wrote metadata-only
[asset exports](ancient-inventory/ancient-assets.tsv),
[placed mesh counts](ancient-inventory/ancient-placements.tsv) and
[totals](ancient-inventory/summary.json). Reproduce it with
`tools/map-lab/ancient_asset_inventory.py <CookedPC> <upk_info> <output-dir>`.
The prefix search is a **naming inventory**, not a visual classification of
every unlabelled mesh or proof that any particular prefab functions when
transplanted.

## What exists

| Export class | Count |
|---|---:|
| StaticMesh | 675 |
| ParticleSystem | 67 |
| Prefab | 167 |
| SGWPrebuild | 5 |
| SkeletalMesh | 7 |
| Material, material instance, texture | 794 |
| AnimSet | 1 |
| **Total** | **1,716** |

There is a coherent Ancient construction kit, larger than the few Agnos
objects initially sampled:

- **Rooms and corridors:** `AN-Interior` has small and large hallway straights
  (512/1024/2048 units), elbows, 3-way and 4-way intersections, ramps,
  open/closed doorways, room corners, walls, molding and columns.
- **Structure:** `AN-Arch` has arches, pillars, bridges, canal pieces, hollow
  walls, railings, trim, lighting strips and gate-shaped wall decoration.
- **Buildings and environment:** `AN-Buildings` has a greenhouse, an
  atmosphere generator, storage and defense buildings; `AGN-Skydome` and
  `ANM-SkyDome` supply existing outdoor sky sources.
- **Janus-lab fixtures:** `AN-Props` includes `AN-TheChair00`, a holographic
  research terminal, communications terminals, a matter device, power
  generator, drone weapons and racks, medical and robot-repair stations.
- **Cover and effects:** `AN-Cover` includes distinct low/medium/high forms;
  `AN-Props.AN-MatterGenActivate` and `GLB-VFX.GLB-AncientSuperweapon` are
  available effects. Their names alone do not imply transporter behavior.

## Where the client places them

| World folder | Ancient-named placed StaticMeshActors |
|---|---:|
| Agnos | 16,398 |
| Agnos_Library | 1,747 |
| Harset | 22 |
| Tollana | 21 |
| Menfa_Dark / Menfa_Light | 1 each |
| Omega_Site | 1 |

The non-Agnos **architecture** is in Tollana: `AN-HollowWall_TOP00` and
small Ancient hallway straights, 3-ways and 4-ways. Harset's matches are
Ancient-named plant props; Menfa's are rock cover; Omega Site's is a waterfall.
Thus Ancient assets really are distributed beyond Agnos, but Agnos and its
library remain the strongest donor maps for a full lab. The count is placed
`StaticMeshActor` references in matching chunks; it is not a count of unique
meshes or loaded actors at one moment.

## Stargate variants and the next build

The same whole-client literal scan found **no `Atlantis` or `Pegasus` name**
in a `.upk` or `.umap`. `GLB-Global.upk` contains `GLB-Stargate01`, its
chevrons/spinner, event horizon, DHD, and `GLB-Stargate_Prefab`; it also has
SGC and frost variants. There is no **identified dedicated Atlantis/Pegasus
gate asset** in this QA build. That is a naming-level conclusion; visual
inspection would be needed to rule out an unlabelled alternate model.

`Agnos-fffffff7.umap` gives a placed donor assembly: `GLB-Stargate01`
(actor export 541), two spinner actors (658/659), `GLB-DHD_00` (662) and a
base (753). A local clone of the ring and spinners into the scratch lab
passes package reopening and property-name checks. Cloning DHD/base currently
stops safely on the source `CoverNodeArray` property; those actors need a
verified transform/remap or a donor without that property before installation.
The Stargate visual is also separate from the server gate row, entry region,
dial sequence and safe arrival proof.

The immediate visual test should use **one centered temporary floor** (the
earlier four SGC pieces overlap) plus Ancient arches, a terminal and the
stock gate ring. Authored continuous Ancient ground and a functional gate are
subsequent acceptance gates. The [client-load log](client-load-test.md) records
why the temporary floor remains a construction aid.
