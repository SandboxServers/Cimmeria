# NA40: bake the static `InterpActor`s, per actor

Measured 2026-09-26 against the cooked `SGWGame/CookedPC` client tree, with
the NA26-NA28 toolchain: `extract_map` and `occluder_extract` from this
branch, and the tiled `NavBuilder.exe` under `bin64\` (untracked).

## Why

NA36 taught the extractor to walk `InterpActor`, UE3's Matinee-driven
mover. The switch was all or nothing, and off by default, because a
mover's cooked pose is its design-time pose: bake a closed door and the
doorway is sealed in the `.nav` and opaque in the `.occ`. Only Harset was
rebuilt with the switch on. NA36's census found 754 `InterpActor`s that
resolve a mesh across the 23 client maps: 7% doors or Stargate parts, 16%
security-camera heads, and 77% static-shaped dressing.

NA40 replaces the switch with a decision per actor.

## The classifier

`crates/navmesh-extractor/src/interp_actor/`. Each `InterpActor` whose
mesh resolves gets one of three verdicts. Only `include` is baked.

1. **Name net.** A mesh name containing `door`, `stargate`, `chevron` or
   `securitycam` is excluded, whatever the evidence says (`name_rules.rs`).
2. **Kismet evidence.** Every `SeqAct_Interp` in the chunk is followed to
   the actors its groups drive, and each group's `InterpTrackMove` keys are
   read (`kismet_evidence.rs`, `move_track.rs`). A chunk is its own level,
   so its exports hold every Kismet reference that can reach its actors.
3. **Verdict** (`classify.rs`, first match wins):
   - no Kismet reference: include (`kismet:unreferenced`);
   - a reference it does not understand, an unreadable group, a keyless
     move track, or a track class that can change more than the actor's
     look or sound: undecided;
   - a move track that starts or ends more than 1 cm from the cooked pose:
     exclude (`matinee:leaves-rest`);
   - a rotation over 5 degrees: exclude (`matinee:rotates`);
   - a slide over 50 cm sideways: exclude (`matinee:slides`);
   - otherwise include (`matinee:rest-anchored`, with the largest rise).

Undecided actors are not baked, and `extract_map`'s summary and class
census keep flagging `InterpActor` as a collision risk while any are
undecided. `--interp-actors off` (both tools) reproduces a pre-NA36 build;
`classify` is the default. Every classified actor is logged, with the rule
that fired and its evidence, to `<out>/interp_actors.tsv`.

The review list is checked in:
[../evidence/na40-interp-actor-decisions.tsv](../evidence/na40-interp-actor-decisions.tsv),
738 rows across the 14 maps that carry a resolving `InterpActor`, with
UE3 and BigWorld coordinates.

One more fix rode along. `extract_chunk_from_package` grouped instances
in a `HashMap`, so two extractions of the same chunk wrote the same
triangles in a different order, and NavBuilder turned them into
different `.nav` bytes. It is a `BTreeMap` now. With it, rebuilding
Dakara_E1, Lucia and both Menfa maps with `--interp-actors off`
reproduces the committed `.nav` byte for byte. Agnos, Beta_Site_Evo_1,
Tollana, Harset and the two Castle maps do not reproduce: their committed
files came from a `HashMap` order.

## Verdicts

Zero undecided anywhere.

| Map | Baked | Excluded | Detail |
|---|---:|---:|---|
| Agnos | 2 | 0 | 2 Humvees (rest-anchored, 6 cm idle) |
| Beta_Site_Evo_1 | 30 | 64 | 30 rings; 62 camera heads (name), 2 radar dishes (rotate 45°) |
| Castle | 2 | 12 | 1 ring, 1 shelf box (both unreferenced); 11 camera heads (name), 1 radar dish (rotates) |
| Castle_CellBlock | 21 | 32 | 21 rings (20 rest-anchored, 1 unreferenced); 12 doors, 20 camera heads (name) |
| Dakara_E1 | 10 | 0 | 10 rings |
| Harset | 25 | 6 | 25 rings; 6 camera heads (name) |
| Harset_CmdCenter | 0 | 1 | 1 camera head (name) |
| Lucia | 206 | 23 | 55 rings, 130 street lamps, 1 floating light, 20 Humvees; 20 camera heads (name), 2 radar dishes and a swinging cargo box (rotate) |
| Menfa_Dark | 125 | 0 | 125 rings |
| Menfa_Light | 50 | 0 | 50 rings |
| Omega_Site | 0 | 4 | 4 camera heads (name) |
| SGC_W1 | 0 | 22 | 22 doors (name) |
| Sewer_Falls | 0 | 2 | 2 fan rotors (rotate 2,160°) |
| Tollana | 101 | 0 | 15 rings, 81 street lamps, 5 floating lights |
| **Total** | **572** | **166** | |

Rest-anchored rises: rings 74 / 138 / 202 / 266 / 330 cm (the five rings
of a stack, lifted during an activation and put back), street lamps 17 cm,
Humvees 6 cm, floating lights 4 cm.

**What the name net adds over the evidence.** A scratch build with the net
disabled excludes every door and camera head on evidence alone, except
nine Castle_CellBlock prison-cell doors: no Kismet references them, so
evidence says `kismet:unreferenced` and they would be baked. The net is
what keeps those cells open. The other 12 doors leave rest (96-560 cm),
and every camera head rotates 45°.

**The "ring-transport platforms" are rings.** `GLB-RingTransporter00` is
one ring of a five-ring stack (332 rings, 68 stacks), not the platform.
At rest the ring lies on the platform: a hollow torus 30-34 cm tall and
2.45 m in radius, inside the 0.6 m `agentClimb`. The platform itself is an
ordinary `StaticMeshActor`, baked before NA36.

## Rebuilds

Parameters are NA26's (single mesh) and NA28's (tiled, `tile=128`), per
[data/spaces/README.md](../../../../data/spaces/README.md).

Probe sets:

- **seeded**: the world's `spawnlist`, `respawners`,
  `ring_transport_regions`, `stargates` and `point_set_points` rows
  (NA26's generator, current seeds).
- **telemetry**: SigNoz accepted and rejected positions and Harset's
  `harset_lastvalid_probes.txt`. They exist for Harset and Castle_CellBlock
  only; SigNoz was unreachable this session, so no new set was mined.
- **ring tops**: the top of each baked actor's geometry at its centre and
  at 80% of its radius.

Each set is scored with `NavMesh::is_point_valid` against the committed
file, an `off` rebuild and a `classify` rebuild.

| Map | Seeded, committed → `classify` | Telemetry, committed → `classify` | `.nav` | Verdict |
|---|---|---|---|---|
| Harset | 51/67 → 51/67 | accepted 37/43 → 37/43, rejected 3,719/9,344 → 3,719/9,344, last-valid 35/41 → 35/41 | 29,772 / 15,289 → 29,768 / 15,287 verts / polys | rebuilt: the six camera heads NA36 baked come out, and the rings add nothing (`classify` = `off`, byte for byte) |
| Dakara_E1 | 3/3 → 3/3 | none | counts unchanged, +68 B | rebuilt |
| Menfa_Light | 0/1 → 0/1 | none | counts unchanged, same size | rebuilt |
| Menfa_Dark | 32/36 → 32/36 | none | counts unchanged, +212 B | rebuilt |
| Tollana | 5/5 → 5/5 | none | 96,197 / 45,497 → 96,191 / 45,493 | rebuilt |
| Agnos | none seeded | none | 187,678 / 78,567 → 187,682 / 78,571, 6,075 → 6,076 components | rebuilt |
| Beta_Site_Evo_1 | 7/7 → 7/7 | none | 150,161 / 71,141 → 150,162 / 71,138 | rebuilt |
| Lucia | 25/26 → 25/26 | none | 142,195 / 67,365 → 142,304 / 67,409 | rebuilt |
| Castle | 62/78 → 61/78 | none | `classify` = `off`, byte for byte | **`.nav` kept**: any rebuild loses `spawn_245_Castle_SurrenderGuard`, with or without `InterpActor` |
| Castle_CellBlock | 46/92 → 47/92 (loses `spawn_16_Preparation_Terminal`, gains two) | accepted 64/64, rejected 176/182, both unchanged | `classify` = `off`, byte for byte | **`.nav` kept**: not strictly better, and the loss is the rebuild, not `InterpActor` |

On every rebuilt map no seeded or telemetry probe the committed file
accepts is rejected. Against the committed files' own polygon centroids
(dropped onto the surface), the rebuilds keep 100% on six maps, 78,370 of
78,372 on Agnos and 65,426 of 65,442 on Lucia. All 18 lost centroids are
next to a baked Humvee: vehicle floor that is now under a vehicle.

The same Humvees account for the only "included" origin probes that
change: 17 on Lucia and 1 on Agnos stop being valid, because the probe is
the vehicle's own origin.

**Ring tops.** Every ring stack's top was already walkable on the
committed meshes and still is, on every map, because the ring sits within
climb height. The five stacks whose tops fail do so on every build: four
on Menfa_Light sit 4.6 m above the nearest polygon and one on Menfa_Dark
11.5 m below it, so none stands on a walkable floor. So the rebuild does
not make ring tops walkable; they already were.

Maps with no baked actor (Harset_CmdCenter, Omega_Site, SGC_W1,
Sewer_Falls) are unchanged. Their `classify` extraction is their `off`
extraction.

## Occluders

All ten maps with a baked actor got a new `.occ`: the eight rebuilt maps
against their new `.nav`, and Castle and Castle_CellBlock against their
kept one. Every build passed the `paged == unpaged` self-check on 5,000
segments. Harset's is now built like every other world (`--nav`, entry
points, 15 m margin); NA36 had built it untrimmed with a 2 m margin.

The bytes drift from NA27's even for an `off` rebuild (Castle_CellBlock:
337,467 against 339,440), most likely because the triangles now arrive
in the new order. A scratch diff compared sight verdicts over 20,000 random
eye-height pairs of navmesh points within 30 m:

| Map | Committed vs rebuild |
|---|---|
| Castle, Castle_CellBlock | 0 differ (`off` and `classify`) |
| Dakara_E1, Tollana, Beta_Site_Evo_1 | 0 differ |
| Harset, Menfa_Dark, Agnos | 1 differ |
| Menfa_Light | 2 differ |
| Lucia | 15 differ, 13 of them clear → blocked by a Humvee or lamp |

## Size

`data/spaces` shrinks by 491,751 bytes (208,281,883 → 207,790,132). The
`.nav` files move by +4,339 bytes in total (Lucia +4,290). The `.occ`
files move by -496,090 bytes, mostly from the same drift (Agnos
-266,955, Tollana -87,608, Dakara_E1 -65,191). Lucia's `.occ` grows by
30,342 bytes: its 206 baked actors are the most of any map.

## Tests

- `interp_actor::classify_tests` and `interp_actor::evidence_tests`
  (synthetic packages through the real reader). They cover the verdicts
  above, and `a_door_or_gate_part_is_never_included_whatever_the_evidence_says`
  plus `a_door_driven_by_a_rest_anchored_matinee_is_still_not_baked`
  guard that a door or gate part is never baked.
- `staticmesh::emit_order_tests` pins the `BTreeMap` emit order.
- `coverage::tests::the_class_census_flags_interp_actor_while_any_is_undecided`.
- `tests/it/interp_actor_castle_cellblock.rs` runs against the cooked
  client (and a `PackageIndex` via `CIMMERIA_PACKAGE_INDEX`, else it
  self-skips): all 53 Castle_CellBlock actors are classified, the 12 doors
  and 20 camera heads are left out, the 21 rings are baked, and nothing is
  undecided. With the name net disabled it fails on `InterpActor_4
  (EM-Door_Prison00)`.
- Revert proofs: removing the name net, the leaves-rest check or the
  rotation check, dropping undecided from the census, going back to the
  `HashMap`, or walking `InterpActor` in `off` mode each fails its guard.

No size-pinned test needed updating. `cimmeria-navmesh-extractor`
449/449. The crates whose tests read `data/spaces` (`cimmeria-entity`,
`-cell-world`, `-cell-catalog`, `-cell-combat`, `-cell-content`, `-cell`)
pass 2,194/2,194 against the new files.

## Open

- **Castle and Castle_CellBlock `.nav`.** They keep their 2026-09-19
  builds. Their `InterpActor`s add nothing to the `.nav`, so NA40 has no
  reason to replace them. The spawn 245 and spawn 16 losses belong to the
  toolchain, and so does any future rebuild.
- **Telemetry for the other eight maps.** Harset and Castle_CellBlock are
  the only worlds with real-position probe sets. Mining SigNoz for the
  rest would turn "seeded probes only" into a real check.
- **Street lamps and floating lights** are baked at their cooked pose.
  They collide in the client (`bCollideActors`), so they are real
  obstacles, but nobody has checked in game that a floating light low
  enough to trim the floor under it (1.8 m agent height) blocks a player
  there too.

## Cross-references

- [NA36 worknote](na36-extractor-mesh-actor-gap.md): the class gap and the
  opt-in this packet replaces
- [navmesh-build-pipeline.md §12](../../../engine/navmesh-build-pipeline.md#12-per-actor-interpactor-classification-na40-2026-09-26)
- [data/spaces/README.md](../../../../data/spaces/README.md): provenance
- [work-packets.md NA40](../work-packets.md#na40)
