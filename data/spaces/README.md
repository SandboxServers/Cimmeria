# Navmeshes (`data/spaces/*.nav`)

One XRC `.nav` per world, loaded at space creation by
`crates/services/src/cell/space_manager/lifecycle.rs`. The file name is the
world name from `entities/spaces.xml`, lower-cased, with spaces turned into
underscores. It is **not** `resources.worlds.client_map`: `SandBox` (world 2)
plays on the `Harset_CmdCenter` client map but loads `sandbox.nav`, which is
a byte copy of `harset_cmdcenter.nav`. A world with no file has no navmesh:
NPCs path in straight lines and the position and line-of-sight checks fail
open.

How a mesh is built, and the Recast limits the parameters below are chosen
around: [docs/engine/navmesh-build-pipeline.md](../../docs/engine/navmesh-build-pipeline.md)
and [docs/engine/navbuilder-recast-limits.md](../../docs/engine/navbuilder-recast-limits.md).
Which worlds enforce containment, and why the rest are advisory:
[docs/architecture/navmesh-containment-modes.md](../../docs/architecture/navmesh-containment-modes.md).

## Provenance

Every mesh except the two Castle ones was built by NA26 on 2026-09-25 from the
cooked client maps: StaticMesh (with prefab archetype resolution and the
mirrored-instance winding fix), Terrain and BSP, with the buried BSP hull-cap
filter, through `extract_map` and the tree-built `NavBuilder.exe`
(Recast 1.6.0). Common to every row: `partition=watershed agentHeight=1.8
agentClimb=0.6 agentRadius=0.6 ch=0.2` (Tollana: `ch=0.3`, see below).
`minRegionSize` is scaled with `cs` so the smallest kept region stays about
52 m² (24 at `cs=0.3`, 16 at 0.45, 12 at 0.6). `bounds` is the crop in
BigWorld metres, `minX,minZ,maxX,maxZ`; where it equals the chunk grid plus
20 m it is only there to clip skybox and backdrop geometry.

"Probes" is `NavMesh::is_point_valid` over the world's seeded `spawnlist`,
`respawners`, `ring_transport_regions`, `stargates` and `point_set_points`
rows, leaving out the `(0, 0, 0)` placeholder gates. Many of those rows are
area-set corners and prefab origins rather than places to stand, so read the
column as a comparison between meshes, not as a coverage figure.

| File | cs / mse / crop | verts / polys / edges | spans | comps | size | build | probes | mode |
|---|---|---|---|---|---|---|---|---|
| `agnos.nav` | 0.45 / 2.5 / `-800,-300,200,700` | 41,720 / 18,323 / 53,960 | 8.8 M | 1,538 | 2.3 MB | 37 s | none seeded | advisory |
| `agnos_library.nav` | 0.3 / 1.3 / chunk grid | 4,283 / 2,228 / 6,074 | 4.5 M | 57 | 0.25 MB | 4 s | none seeded | advisory |
| `beta_site_evo_1.nav` | 0.3 / 2.5 / `-100,0,1200,1200` | 47,628 / 23,497 / 64,959 | 14.2 M | 601 | 3.9 MB | 34 s | 7/7 | advisory |
| `castle.nav` | 0.3 / 2.5 / chunk grid | 40,068 / 19,815 / 55,101 | — | 549 | 3.4 MB | — | 62/78 | advisory |
| `castle_cellblock.nav` | 0.3 / 1.3 / ±400 | 3,039 / 1,658 / — | — | 17 | 0.18 MB | — | 46/92 | **enforce** |
| `dakara_e1.nav` | 0.6 / 3.0 / chunk grid | 44,181 / 22,824 / 62,092 | 7.8 M | 845 | 3.7 MB | 28 s | 3/3 | advisory |
| `dakara_e1_storyrm.nav` | 0.3 / 1.3 / chunk grid | 239 / 115 / 325 | 0.1 M | 6 | 15 KB | 0.1 s | none seeded | advisory |
| `harset.nav` | 0.3 / 1.3 / chunk grid | 29,768 / 15,287 / 42,379 | 8.6 M | 372 | 1.9 MB | 9 s | 51/67 | advisory |
| `harset_cmdcenter.nav`, `sandbox.nav` | 0.3 / 1.3 / chunk grid | 1,130 / 570 / 1,580 | 0.4 M | 14 | 64 KB | 0.6 s | 14/20 | advisory |
| `harset_market.nav` | 0.3 / 1.3 / chunk grid | 1,352 / 683 / 1,897 | 1.2 M | 29 | 76 KB | 0.8 s | 3/5 | advisory |
| `harset_storagerm.nav` | 0.3 / 1.3 / chunk grid | 1,195 / 560 / 1,618 | 0.2 M | 16 | 65 KB | 0.5 s | 5/5 | advisory |
| `ihpet_crater_dark.nav` | 0.3 / 2.5 / chunk grid | 29,770 / 14,838 / 40,970 | 16.7 M | 370 | 2.6 MB | 20 s | 3/3 | advisory |
| `ihpet_crater_light.nav` | 0.3 / 2.5 / `-300,-1300,800,40` | 30,059 / 15,126 / 41,563 | 16.1 M | 371 | 2.6 MB | 19 s | 3/3 | advisory |
| `lucia.nav` | 0.45 / 2.5 / `-900,-400,700,900` | 41,726 / 20,811 / 57,548 | 9.1 M | 568 | 3.4 MB | 48 s | 20/26 | advisory |
| `menfa_dark.nav` | 0.6 / 2.5 / chunk grid | 45,305 / 23,308 / 63,895 | 10.4 M | 629 | 3.0 MB | 28 s | 32/36 | advisory |
| `menfa_light.nav` | 0.6 / 2.5 / chunk grid | 37,058 / 19,449 / 52,748 | 9.4 M | 526 | 2.5 MB | 17 s | 0/1 (gate prefab origin) | advisory |
| `omega_site.nav` | 0.3 / 1.3 / chunk grid | 24,621 / 12,892 / 35,542 | 3.8 M | 247 | 1.7 MB | 7 s | 15/15 | advisory |
| `omega_site_cmdcenter.nav` | 0.3 / 1.3 / chunk grid | 4,728 / 2,409 / 6,660 | 3.6 M | 45 | 0.27 MB | 2.5 s | 3/3 | advisory |
| `sgc.nav` | 0.3 / 1.3 / chunk grid | 1,064 / 506 / 1,412 | 0.2 M | 21 | 57 KB | 0.5 s | 2/2 | advisory |
| `sgc_w1.nav` | 0.3 / 1.3 / chunk grid | 3,091 / 1,508 / 4,195 | 0.5 M | 68 | 0.17 MB | 1.1 s | 22/28 | advisory |
| `sewer_falls.nav` | 0.3 / 1.3 / chunk grid | 13,317 / 6,831 / 18,727 | 9.7 M | 139 | 0.77 MB | 7.5 s | none seeded | advisory |
| `tollana.nav` | 0.45 / 2.5 / `-800,-600,300,400`, `ch=0.3` | 33,172 / 16,537 / 45,625 | 7.3 M | 417 | 2.0 MB | 30 s | 5/5 | advisory |
| `tollana_curia.nav` | 0.3 / 1.3 / chunk grid | 64 / 33 / 95 | 0.1 M | 1 | 4 KB | 0.1 s | none seeded | advisory |

Recast's caps, for reading the table: spans ≤ 16,777,215, contour vertices
< 65,534, adjacency edges ≤ 65,535. Build time is NavBuilder alone; loading
the chunk OBJs adds 5-30 s on the big maps.

### Kept, not rebuilt

- `castle_cellblock.nav` and `castle.nav` were built from the client maps on
  2026-09-19 with this pipeline. NA26 rebuilt both and measured the result
  against them: Cellblock 3,041 verts / 1,661 polys, covering 1,657 of the 1,658
  old polygon centroids, and the same 64/64 accepted telemetry positions and 176/182 rejected ones;
  Castle 39,640 verts / 19,584 polys, probes 61/78 against 62/78. Neither is an
  improvement, so the committed files stay, and so do the dozens of tests
  pinned to them. Cellblock is the only meshed world on `enforce`: its mesh
  has been walked under containment since 2026-09-19.

### Replaced

- `harset.nav` (2012, 19,345 polys, 1,939 components). Against real
  positions from SigNoz (`movement.validation_reject`, September 2026,
  round-number teleport points removed): of the 9,344 distinct rejected
  client positions the old mesh accepts 497 and the new one 3,719; of the 43
  distinct `last_valid` positions the old mesh accepts all 43 and the new one
  37. The six it loses are raised platforms at about y -59 / -61
  (for example (9.5, -58.8, 59.4) and (34.6, -61.3, -70.7)) where
  `obj_slab` finds no source geometry at all, so an actor class the
  extractor does not decode is the likely cause. Probes go from 18/67 to
  51/67; all five ring pads and the Command Center door are now on-mesh.
- `harset_storagerm.nav` (2012, agent 0.6 / 0.9). Of 24 distinct accepted
  positions the new mesh keeps 16 against the old 23. Three of the seven it
  loses are near-origin login noise (|x|, |z| < 2); the other four sit 1-2 m
  above or below the new floor, where the old mesh had an under-floor layer
  and prop tops its 0.6 m agent could stand on. Rejected
  positions: 614 against 631 of 1,069. A regression by the letter of the
  check, which is one of the reasons world 70 is advisory.
- `sgc_w1.nav` (2012, agent 0.6 / 0.9). No navmesh rejects in the
  telemetry to compare. Probes 18/28 → 22/28; 69 % of the old polygons'
  centroids are on the new mesh, the rest are low-clearance and prop-top
  surfaces a 1.8 m agent cannot use.
- `agnos.nav` (2012). The old file covered 1,603 m² in one strip at
  z -1000..-800. It is outside the new crop.

### Why some maps are cropped or coarse

A single `rcPolyMesh`, which is all the XRC format and
`crates/entity/src/navigation/load.rs` hold, cannot carry the big outdoor
maps at `cs=0.3`. Each was taken down a ladder (finest first:
`cs` 0.3 at mse 1.3 then 2.5, 0.45, 0.6 at mse 2.5 then 3.0, 0.75, 0.9)
and shipped at the first rung that fit every cap:

- Whole map at `cs=0.6`: Dakara_E1, Menfa_Dark, Menfa_Light.
- Cropped, because no whole-map rung fits even at `cs=0.9`: Agnos, Lucia,
  Tollana, Beta_Site_Evo_1 (whole map fits only at `cs=0.9`, so the crop at
  0.3 was preferred). The crop is the window with the most seeded content
  and chunk geometry. Positions outside it have no mesh, which in an advisory
  world means NPCs there path in straight lines, as before NA26.
- Tollana needed `ch=0.3`: one prop at y -1728 puts the city more than
  8,191 height cells (the 13-bit `rcSpan` limit) above `bmin` at `ch=0.2`,
  and Recast flattened everything onto one sheet at y -90. NavBuilder now
  refuses that input.

Covering these maps whole needs a tiled Detour mesh, which needs a
multi-tile `.nav` format and loader work. That is the follow-up.

## Worlds without a mesh

The server only creates spaces for the 24 worlds in `entities/spaces.xml`,
and all 24 now have a file. The other 67 `resources.worlds` rows (mission
test maps, `Tol-Alpha` / `Ca-Alpha` pockets, and worlds whose map is not in
the 2009 client such as Egypt, Pen-Lai, Pertho, SGC_W2 and Dakara_E2/E3)
have no client map directory under `CookedPC/Maps`, so there is nothing to
build from. `Login_Map` is skipped on purpose.

## Size

`data/spaces` went from 6.7 MB (6 files) to 36.5 MB (24 files). The largest
file is `beta_site_evo_1.nav` at 4.0 MB.
