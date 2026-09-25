# Placement B: world 57 population and named regions

> Type: reference (placement ledger). Cluster **B** of the Harset placement pass
> described in [METHOD.md](METHOD.md). Covers packets **H14** (world-57
> population and props) and **H15** (world-57 named regions). Written
> 2026-09-19. Worlds 68/69/70, arrivals, respawners, rings and the
> Market/Storage doors are NOT in this cluster.

Every coordinate below is a **guess with a stated evidence class and
confidence**, not a pin. Correct the rows; do not re-derive them. Items with no
usable evidence are in [`## No idea`](#no-idea) and are deliberately unseeded.

## The one finding that shaped every row in this cluster

`data/spaces/harset.nav` does not model the hub's upper quarters at the floor
height the geometry actually has.

Evidence: a ring probe (`nav_inspect --h-tol 1.2 --v-tol 4.0`, the same gate
`NavGraph::is_point_valid` applies) was run at radii 0-12 m around **every**
named Jaffa Zone landmark - the `JF-Tent00/01/02/03` rows, `GA-Barracks01`,
`JF-MilitaryTent00`, `JF-HighWallArch00`, both `TOL-FluidPlaneCircle_Flat00`
fountains and `FirstBug`'s own authored position - at y = -41.28, the floor
`obj_slab` reports at every one of those columns (a consistent 80-290 m2 slab
in the `y[-42, -41]` band). **Not one landmark had an on-mesh point within
12 m.** The nearest polygons sit 7-11 m above or below the floor. The same holds
at all three `GA-Tow*` shield towers, on the palace terrace and at the
`GA-Viewscreens00` console floor.

Two independent confirmations that this is the mesh and not the placements:

- the two **AUTHORED** rows in that quarter are themselves off-mesh - Petbe
  (spawn 223, `dy = -10.99 m`) and `FirstBug` (spawn 224, `dy = -10.37 m`);
- so are four of the five authored **ring pads**, including
  `HarsetRingLeft` (spawn 128, `dy = -9.37 m`), which players demonstrably
  reach because the ring puts them there.

World 57 runs `navmesh_mode = 'advisory'` (H53), so an off-mesh spawn is not
rubber-banded. What it does cost is NPC pathing, which is why every row in this
cluster is `is_stationary = true`.

**Consequence for the H14 acceptance line.** "`is_point_valid` on every
world-57 spawn" is not satisfiable and is not the right bar. The guard shipped
instead is a biconditional over an explicit exception table
(`world57_placements_match_their_recorded_navmesh_verdict`): a row recorded
on-mesh must be on-mesh, and a row recorded off-mesh must still be off-mesh. The
second half is the load-bearing one - when GH1 rebuilds the mesh and a quarter
becomes walkable, the guard fails and forces this ledger and the
`is_stationary` decision to be revisited instead of going quietly stale. A
control assertion on ring region 4's pad keeps a mesh that failed to load from
making the off-mesh half vacuous.

## Second opinion: the rebuilt mesh

A rebuilt Harset navmesh (`mse13.nav`, humanoid agent, **374** components
against the shipped mesh's 1,939) arrived from the Castle-nav session after
this cluster was first placed. It is **not** what the tests load - they load
the shipped `data/spaces/harset.nav`, and `data/spaces` is untouched - and it
is not used to validate a coordinate, because it loses 12 positions real
players stood on. It is used for exactly one question the shipped mesh cannot
answer: *could a player walk here from where they arrive?*

The answer is good. **14 of the 15 rows are on-mesh on the rebuilt mesh, and
all 14 are on ONE component (11)** together with the gate, the plaza exit,
all four reachable ring pads, Petbe (spawn 223) and `FirstBug` (spawn 224).
Nothing in this cluster is stranded in a sealed pocket.

Two rows were moved because of that check rather than left where the first
pass put them:

- **`SecondBug` (306)**, z 117.5 -> 118.5. It sat 1.40 m outside the
  containment gate at the mesh edge; a 1 m nudge puts it inside.
- **Shield tower 2's console (309)**, off the tower's own -31.1 pad and up
  onto the -28.25 terrace beside it. The rebuilt mesh puts that pad on
  component **280 - an island**, not connected to the plaza. That is the
  Castle "Romney was placed into a sealed wing" failure shape, caught before
  it shipped. The terrace is component 11 and is a real floor (272 m2 at
  `y[-28.5, -28.0]` from `obj_slab`), and it is the same terrace
  `Harset.PetbeQuarters` covers.

The one row still outside component 11 is **shield tower 1's console (308)**,
whose hillside Y was already flagged LOW - see PL-B-09.

Per-row mesh verdicts are in the `Checks run` column: "shipped mesh" always
means `data/spaces/harset.nav` and is what the tests assert; "rebuilt mesh"
is this second opinion and is asserted nowhere.

## Conventions used in the rows below

- `heading` is `atan2(dx, dz)` radians, 0 = +Z
  (`cell/service/npc_ai/fight.rs:633`). Every value is derived from an approach
  direction or from the landmark the entity faces. None is 0 - that is the
  Castle lesson (reconstructed rows all faced walls).
- "floor" always means the `obj_slab --levels` band with real triangle count at
  that column, never a prefab origin, except where a row says otherwise.
- "component N" is the `harset.nav` connected component from
  `nav_inspect`. 187 is the hub (plaza + merchant street, 24,771 m2); 1441 is
  the north Jaffa / Bank fragment (10,743 m2).

## Population (H14) - `db/resources/Worlds/Seed/spawnlist.sql`

| ID | What | World | X | Y | Z | heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-B-01 | Hansen, spawn 300, template 212, tag `Harset_Hansen` | 57 | -24.00 | -68.9228 | 14.00 | 0.7836 | SPEC-DESCRIPTIVE ("left of the gate, walking outward") + MAP-GEOMETRY + AUTHORED (plaza guard rows 225/234 give the floor y) | MEDIUM | on-mesh, component **187** (hub), h 0.00 m, dy -0.09 m; floor -69.2 from `obj_slab` (64 m2, 128 tris); clear of the guard line at x=-18.6; heading faces the gate at (-0.076, 38.011) | Walk out of the gate and turn left before the guard posts: two Tau'ri stand off the walkway facing back at the gate | `spawnlist.sql` spawn 300 |
| PL-B-02 | Jacobs, spawn 301, template 213, tag `Harset_Jacobs` | 57 | -26.50 | -68.9228 | 18.50 | 0.9350 | same as PL-B-01 | MEDIUM | on-mesh, component **187**, h 0.00 m, dy -0.16 m; same floor slab; 5.1 m from Hansen | Beside Hansen | `spawnlist.sql` spawn 301 |
| PL-B-03 | Lo'rak, spawn 302, template 201, tag `Harset_Lorak` | 57 | 22.00 | -68.90 | -30.00 | 5.7090 | SPEC-DESCRIPTIVE ("bazaar") + MAP-LANDMARK (`GA-MerchantTent00/01/03` at x 28-35, z -13 to -38) + MAP-GEOMETRY | MEDIUM | on-mesh, component **187**, h 0.00 m, dy -0.30 m; floor -69.2 (`obj_slab`, 200 m2, 319 tris). Chose the plaza-level merchant street over the y=-60 lower-level cluster because only this one is on the hub component, i.e. the one a player reaches on foot; heading faces the plaza exit at (0, 4) | Leave the plaza south down the tent street; Lo'rak faces you as you arrive | `spawnlist.sql` spawn 302 |
| PL-B-04 | 1326 Lan'toc Jaffa (accepts), spawn 303, template 204, tag `Harset_FormerRaJaffa` | 57 | -160.00 | -41.28 | 84.00 | 4.6335 | MAP-LANDMARK (`JF-Tent03` row x -154.5, `GA-Barracks01` -198.1/84.4) + MAP-GEOMETRY + AUTHORED (chain 6335 expects the tag) | MEDIUM for the camp, LOW for the exact metre | **off-mesh** (nearest poly 6.72 m below, component 1364) - see the finding above; floor -41.3 confirmed (`obj_slab`, 120 m2, 241 tris at `y[-42, -41]`); heading faces the `HarsetRingLeft` pad (-194.5, 81.3), the way a player arrives in this quarter | Ring to the Jaffa Zone (left ring); two Jaffa stand in the camp street between the tent row and the barracks | `spawnlist.sql` spawn 303 |
| PL-B-05 | 1326 Lan'toc Jaffa (refuses), spawn 304, template 204, tag `Harset_FormerRaJaffa2` | 57 | -160.00 | -41.28 | 89.00 | 4.4921 | same as PL-B-04 (chain 6336) | MEDIUM / LOW | **off-mesh**, same slab, 5.0 m from PL-B-04 so they read as two men | Beside PL-B-04 | `spawnlist.sql` spawn 304 |
| PL-B-06 | Suspicious Jaffa, spawn 305, template 205, tag `Harset_SuspiciousJaffa` | 57 | -159.00 | -41.28 | -26.00 | 5.5862 | MAP-LANDMARK (south Jaffa camp, `JF-Tent02` at -156.1/-29.4) + MAP-GEOMETRY | MEDIUM | **on-mesh**, component **853**, h 0.00 m, dy +0.85 m - a rectangular mesh fragment (x -163 to -158, z -30 to -21) that coincides with a tent floor; `obj_slab` confirms 100 m2 at `y[-42, -41]`. Component 853 is NOT the hub component, so NPC pathing cannot leave the tent; heading faces the south camp fountain at (-176.6, -5.0) | Ring to the Jaffa Zone and walk south; he lurks in a tent in the south camp | `spawnlist.sql` spawn 305 |
| PL-B-07 | `SecondBug` basket, spawn 306, template 164 | 57 | -186.00 | -41.28 | 118.50 | 2.8993 | AUTHORED area (742 step 2504 text: "Hide the listening devices **in the Jaffa area** in Harset") + AUTHORED sibling (`FirstBug` spawn 224 at -176.7/125.3) + MAP-LANDMARK (`JF-Tent01` -185.4/114.9, `JF-Tent03` -184.5/121.8) | MEDIUM | **off-mesh on the shipped mesh** (2.06 m h, 10.48 m below component 1441) - the same relationship the authored `FirstBug` has; **on-mesh, component 11 on the rebuilt mesh** after a 1 m nudge in z (at z=117.5 it sat 1.40 m outside the containment gate there); floor -41.3 confirmed (`obj_slab`, 286 m2, 462 tris); heading faces `JF-Tent01` | With 742 active, the north Jaffa camp holds three baskets a few metres apart | `spawnlist.sql` spawn 306 |
| PL-B-08 | `ThirdBug` basket, spawn 307, template 164 | 57 | -147.00 | -41.28 | 104.50 | 3.6932 | same as PL-B-07 (`JF-Tent03` row at -146.4/106.3) | MEDIUM | **off-mesh** (3.61 m h, 7.88 m below, component 1429); floor -41.3 confirmed (`obj_slab`, 102 m2, 749 tris) | Same camp, east end of the tent row | `spawnlist.sql` spawn 307 |
| PL-B-09 | Shield tower 1 console, spawn 308, template 243, tag `Harset_ShieldTower1` | 57 | -223.00 | -41.36 | 37.72 | 4.7124 | MAP-LANDMARK (`GA-TowTall01`, and the map has exactly three `GA-Tow*` instances for a mission that wants exactly three towers) | MEDIUM for XZ, **LOW for Y** | **off-mesh** (10.75 m below component 653). Y is the tower prefab's own origin, not a floor: this column has `flat<=5deg 0.0 m2` - it is a hillside, and `obj_slab` returns 20+ thin ramp slices between -53 and -31 with no dominant level. The console may float or sink by a metre or two | Ring to the Jaffa Zone and walk west past the camp; the tall tower is the far-west landmark | `spawnlist.sql` spawn 308; re-pin Y from a `.location` reading at the tower base |
| PL-B-10 | Shield tower 2 console, spawn 309, template 243, tag `Harset_ShieldTower2` | 57 | -168.00 | -28.25 | 233.50 | 1.2094 | MAP-LANDMARK (`GA-TowMed00` at -166.1/234.8) + MAP-GEOMETRY reachability | MEDIUM | **off-mesh on the shipped mesh** (1.10 m h, 6.45 m below component 1763); **on-mesh, component 11 on the rebuilt mesh**. Moved off the tower's own -31.1 pad after the rebuilt mesh put that pad on component **280, an island** - the Castle "Romney in a sealed wing" shape exactly. Now on the -28.25 terrace beside it, which `obj_slab` gives 272 m2 at `y[-28.5, -28.0]`; heading still faces the tower pivot | Ring to `HarsetRingLeftTop` (the palace terrace); the medium tower stands on it | `spawnlist.sql` spawn 309 |
| PL-B-11 | Shield tower 3 console, spawn 310, template 243, tag `Harset_ShieldTower3` | 57 | -3.00 | -30.72 | 285.50 | 0.7378 | MAP-LANDMARK (`GA-TowShort01` at 0.0/288.8) | MEDIUM for XZ, LOW-MEDIUM for Y | **off-mesh** (24.61 m below component 1919 - the largest off-mesh gap in the cluster); the column holds two overlapping terrain sheets, `y[-31.0, -30.5]` (1,840 m2) and `y[-29.5, -29.0]` (1,858 m2), and the prefab origin -30.72 lies in the lower one; offset 3 m so the derived heading is not exactly 0 | Far north of the map, on the axis through the gate | `spawnlist.sql` spawn 310 |
| PL-B-12 | Shield Controls, spawn 311, template 248, tag `Harset_ShieldControls` | 57 | -90.00 | -30.70 | 213.90 | 1.5557 | INFERRED from MAP-LANDMARK: `GA-Props:GA-Viewscreens00` is the map's **only** Goa'uld control-panel prop (one instance) and the shield towers are Goa'uld tech | **LOW** - the weakest row in the cluster | **off-mesh** (9.09 m above component 1441); Y is a real constructed floor, 128 m2 / 502 tris at `y[-31.0, -30.5]`; prop stands 1.7 m west of the screens facing them | Walk/ring north-west of the plaza to the screen bank at (-88, 214) | `spawnlist.sql` spawn 311. If nothing is there, the competing reading is the Command Center (world 68) - **delete this row** rather than nudging it, and hand the objective to H12 |
| PL-B-13 | Bank anchor, spawn 312, template 248, tag `Harset_BankAnchor` | 57 | -184.50 | -41.28 | 162.27 | 4.7492 | MAP-LANDMARK (`GA-Bank00` + `CA-Courtyard_Str00` pair) + MAP-GEOMETRY tie-break | MEDIUM | **on-mesh**, component **1441**, h 0.00 m, dy -0.13 m. `GA-Bank00` has five instances and three sit on a courtyard structure, so the name alone is weak; the tie-break is that only this instance is on the mesh (the other four are 5-68 m off) and `obj_slab` gives it a 180 m2 floor at `y[-42, -41]`. A walkable courtyard is what a bank needs | Ring to the Jaffa Zone and walk north to the courtyard at z=162 | `spawnlist.sql` spawn 312 |
| PL-B-14 | Storage Lo'taur (the banker), spawn 314, template 219, tag `Harset_StorageLotaur` | 57 | -186.00 | -41.28 | 160.00 | 0.5814 | same as PL-B-13 + the tag registry and audit defect 15 ("Storage Lo'taur (Bank)") | MEDIUM | **on-mesh**, component **1441**, h 0.03 m, dy +0.03 m - the best-fitting point in the cluster; 2.7 m from the anchor, facing the courtyard centre | Beside PL-B-13 | `spawnlist.sql` spawn 314 |
| PL-B-15 | Petbe's quarters search object, spawn 313, template 244, tag `Harset_PetbeQuarters` | 57 | -160.50 | -28.25 | 232.60 | 5.0039 | MAP-LANDMARK (`HP-Props:HP-Brazier00`, the map's only two instances, both here) + MAP-MARKER (the `HarsetRingLeftTop` ring pad 12 m away) + MAP-GEOMETRY | LOW-MEDIUM | **off-mesh** (14.45 m below component 2); floor -28.2 confirmed (`obj_slab`, 272.6 m2 / 65 tris at `y[-28.5, -28.0]`, spanning x[-178, -155.5] z[224.2, 244.7]), with the ring pad's own step at -27.2 alongside; heading faces the ring arrival point | Ring to `HarsetRingLeftTop`; the search object is on the brazier terrace east of the tower | `spawnlist.sql` spawn 313. The competing site is beside Petbe's own authored spawn 223 at (-165.5, -41.3, 99.4) - see the decision note below |

### Decision note: Petbe's quarters, two sites

Neither candidate is decisive, so the reasoning is recorded rather than hidden.

- **(a) the brazier terrace at (-165, -28.2, 233)** - seeded. A unique art
  family (`HP-*`) appears exactly once in a cooked map when it belongs to one
  named place, and a ring pad implies a destination that matters.
- **(b) beside Petbe's authored spawn 223** in the Jaffa Zone. Rejected on the
  grounds that where a Goa'uld stands on duty is not where he sleeps - but it is
  the cheaper correction if (a) turns out to be bare terrace.

Note that shield tower 2 (PL-B-10) stands on the same terrace, so
`Harset.PetbeQuarters` and `Harset.ShieldTower2` deliberately overlap. A tower
in a palace forecourt is coherent; if the playtest says the terrace is a
military post and not a residence, move PL-B-15 and PL-B-19 to site (b)
together.

## Named regions (H15) - `point_sets.sql` / `point_set_points.sql`

All eight are `type = 'AreaSet'`, world 57, names dotted and world-prefixed so
the cross-file region-key linter sees them. `BoundingBox` rows carry four
corners in ring order with the fourth raised to the ceiling (the same deliberate
asymmetry rows 2078/2079/2083-2085 use); `Cylinder` rows carry one point plus
radius and height and are expanded by the loader. `is_point_in_region` fails
closed on any count but four.

| ID | What | World | Extent (floor Y, ceiling Y) | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|
| PL-B-16 | `Harset.JaffaZone`, set 2100, points 2500-2503 | 57 | x[-208, -128] z[-70, 140], floor -42.0, ceiling -30.0 | MAP-LANDMARK (every `JF-*` instance, `GA-Barracks01`, `JF-HighWallArch00`, both fountains) + MAP-GEOMETRY | MEDIUM-HIGH for the area, MEDIUM for the edges | encloses `JF-Tent03` (-154.86/77.74) and `GA-Barracks01` (-198.09/84.35), both asserted by test; excludes the shield-tower-1 console at x=-223 and the Bank courtyard at z=162 (each has its own region); Y band covers the -41.3 floor and the tent-roof level | 1343 step 3974 should tick when you ring in and walk the camp | `point_set_points.sql` rows 2500-2503 |
| PL-B-17 | `Harset.OpCoreZone`, set 2101, points 2504-2507 | 57 | x[126, 256] z[-64, 164], floor -42.0, ceiling -30.0 | MAP-LANDMARK (`EM-Bunker00`, `EM-Quartermaster01`, infirmary generator, the `EM-Tent_*` rows, both `EM-Cover_Guardpost_Med01`, `GA-Base_Simp00`, `EM-WaterTower00`) + **TELEMETRY** | MEDIUM-HIGH | encloses `EM-Bunker00` (202.08/-9.04) and `EM-Quartermaster01` (188.51/156.71), both asserted; 10 of the 13 real-player `last_valid` positions in this quarter fall inside the volume. (Recounted against the cleaned 38-point set `harset_last_valid_probes.txt`; the first pass said 13 of 15 and was wrong. ~77% of the raw reject volume is one entity parked at (0,0,0)/(1,1,1) and is excluded there.) The three that do not are deliberate: `lv24` at y=-47.8 on a slope, and the `lv11`/`lv12` pair at y=-66.3, on the lower level running *under* the camp - the Market-door approach, worker A's region. The Y band is therefore the camp deck only | Ring right; the volume should cover the tent camp but not the level below it | `point_set_points.sql` rows 2504-2507 |
| PL-B-18 | `Harset.Bank`, set 2102, points 2508-2511 | 57 | x[-195, -178] z[155, 170], floor -42.0, ceiling -34.0 | MAP-LANDMARK + MAP-GEOMETRY (see PL-B-13) | MEDIUM | encloses both `GA-Bank00` (-187.92/162.40) and `CA-Courtyard_Str00` (-184.51/162.27), asserted; sized to the 180 m2 `obj_slab` floor over x[-193.4, -180.0] z[157.7, 167.0] plus margin | 1374 objective 4088 should tick in the courtyard | `point_set_points.sql` rows 2508-2511 |
| PL-B-19 | `Harset.PetbeQuarters`, set 2103, points 2512-2515 | 57 | x[-176, -154] z[222, 246], floor -29.0, ceiling -21.0 | MAP-LANDMARK + MAP-MARKER + MAP-GEOMETRY (see PL-B-15) | LOW-MEDIUM | encloses both `HP-Brazier00` instances, asserted; sized to the 272.6 m2 floor slab; overlaps `Harset.ShieldTower2` on purpose | Ring to `HarsetRingLeftTop` | `point_set_points.sql` rows 2512-2515; moves with PL-B-15 |
| PL-B-20 | `Harset.ShieldControls`, set 2104, point 2516, Cylinder r=6 h=4 | 57 | centred (-88.34, -30.70, 213.925) | INFERRED from MAP-LANDMARK (see PL-B-12) | LOW | centred on the `GA-Viewscreens00` prefab origin, not on the prop row, so moving the prop during the in-client pass does not move the region; contains the prop; enclosure asserted | With 1374 active, the scanner objective should arm at the screens | `point_set_points.sql` row 2516; delete with PL-B-12 if the site is wrong |
| PL-B-21 | `Harset.ShieldTower1`, set 2105, point 2517, Cylinder r=8 h=6 | 57 | centred (-226.04, -41.36, 37.72) | MAP-LANDMARK | MEDIUM | centred on the `GA-TowTall01` prefab origin; contains the console (PL-B-09); enclosure asserted | 1240 step 3606 | `point_set_points.sql` row 2517 |
| PL-B-22 | `Harset.ShieldTower2`, set 2106, point 2518, Cylinder r=5 h=6 | 57 | centred (-166.118, -31.13, 234.797) | MAP-LANDMARK | MEDIUM | centred on `GA-TowMed00`; radius reduced to 5 so the three towers can never be confused; contains the console (PL-B-10) | 1240 step 3606 | `point_set_points.sql` row 2518 |
| PL-B-23 | `Harset.ShieldTower3`, set 2107, point 2519, Cylinder r=8 h=6 | 57 | centred (0.0, -30.72, 288.80) | MAP-LANDMARK | MEDIUM | centred on `GA-TowShort01`; contains the console (PL-B-11) | 1240 step 3606 | `point_set_points.sql` row 2519 |

## Tag linkage: what the merged chains expect

Every `interact_tag` key that a merged world-57 chain dispatches on now has a
spawnlist row carrying it byte-exactly. The check is not a hand-written list -
`world57_interact_tags_from_merged_chains_all_have_a_spawn_row` derives the
expected set from `content_triggers` joined to a `world eq 57` condition, so a
transposed tag fails rather than silently dead-ending a mission.

| Tag | Chain(s) | Source seed | Status |
|---|---|---|---|
| `FirstBug` | 6104, 6107 | `harset_goauld_chains.sql` | already authored (spawn 224) |
| `SecondBug` | 6105, 6108 | `harset_goauld_chains.sql` | **placed** (spawn 306, PL-B-07) |
| `ThirdBug` | 6106, 6109 | `harset_goauld_chains.sql` | **placed** (spawn 307, PL-B-08) |
| `Harset_FormerRaJaffa` | 6335 | `harset_jaffa_chains.sql` | **placed** (spawn 303, PL-B-04) |
| `Harset_FormerRaJaffa2` | 6336 | `harset_jaffa_chains.sql` | **placed** (spawn 304, PL-B-05) |

No world-57 `interact_tag` key is left without a row. The remaining
`interact_tag` chains in the merged Harset seeds (`CmdCenter_Baal`,
`CmdCenter_Mohkatan`, `CmdCenter_Marsh`, `CmdCenter_Copplemann`) are world 68
and belong to packet **H12**, not to this cluster.

Nothing merged yet consumes the H15 region names - `Harset.JaffaZone` and the
rest are forward-looking for 1343, 1240, 1243, 1362 and 1374, whose chains are
unauthored. The only `enter_region` keys any merged chain uses today are
`Harset.CommandCenterTransition` and `Harset_CmdCenter.HarsetTransition`
(packet H10).

## No idea

Seven things in the H14/H15 scope have no usable evidence and are therefore
**unseeded**, per the METHOD NO-IDEA rule. Each entry says which evidence was
searched and what was missing.

1. **`Harset.Bar` region and `Harset_BarAnchor` prop** (1352 objective 4008,
   1374 anchor). Searched: all 91 distinct mesh names in
   `Harset_arch_meshes.tsv` for anything tavern-shaped (bar, tavern, pub, inn,
   keg, stool, table, seat); the `extract_actors` dump; the audit and worknotes.
   Missing: nothing in the map names a bar, and `GA-Bank00` - the only prop
   whose name suggests interior furniture - is already spoken for and co-locates
   with courtyard architecture. Guessing "somewhere in the merchant street"
   would put a mission objective in a 90 m long volume with nothing to confirm
   it. Point-set id **2108** is reserved.
2. **`Harset.HoldingPens` region** (and any holding-pen prop). Searched: the
   mesh census (`GA-Fence00/02/03` exist, 16 + 4 + 8 instances, but they fence
   the whole hub and no cluster reads as a pen), the tag registry - which has
   **no tag for holding pens at all** - and the audit. Missing: both a location
   and an entity to put in it. Point-set id **2109** is reserved. Worth raising
   with the coordinator: if holding pens are needed by a mission, the tag
   registry needs a row first.
3. **Blackstock in world 57**. The tag registry allows either
   `Harset_Blackstock` (57) or `CmdCenter_Blackstock` (68). The audit puts him
   behind a desk - 1243's Scarab anchor list says "Blackstock's **office**" and
   1374's last step is "report Blackstock". An office is the Command Center.
   Left to packet **H12** rather than placed twice; if H12 decides he is
   outdoors, this cluster can add him next to Hansen and Jacobs, who are his
   OP-CORE colleagues.
4. **Vendor and trainer NPCs as inert props.** Searched: H11's template block
   (200-248) for generic vendor/trainer templates - there are none; GH2 is still
   open, so no `buy_item_list` / `trainer_ability_list_id` exists to hang on
   them. The spec observation "Tau'ri vendors on the OP-CORE side lower level"
   gives a *side of the map*, not positions, and the OP-CORE lower level is a
   ~130 x 230 m deck. Nothing to place and nothing for it to do.
5. **`Harset_Lethander`** (template 46, "hub stall", packets H24/H46). No stall
   mesh is distinguishable among the 34 merchant tents, and no spec line narrows
   it. Owned by H24 anyway; recorded here so the coordinator can see that the
   H14 "vendor NPCs" line does not silently cover him.
6. **`Harset_HaughtyGoauld`** (template 210, "Market side", 1374) and
   **`Harset_AngryJaffa`** (template 206, "Storage side", 1374). Both are
   positioned relative to the Market and Storage doors, which are **worker A's**
   scope in this pass and not yet pinned. Once A's door regions exist these are a
   two-row follow-up: stand each one a few metres out from its door on the hub
   side. Deliberately not guessed ahead of A.
7. **The six 1243 `Harset_Scarab_*` anchors and the four 1362
   `Harset_OpsCenter/Research/Guardhouse/ScienceTent` anchors.** Outside the
   lead's list for this cluster, so not seeded - but the evidence for several of
   them was found while working and should not be lost:
   - "Fountain" -> `TOL-FluidPlaneCircle_Flat00`, 11 instances at three sites:
     the Jaffa Zone pair at (-176.615, ~-41.0, 80.07) and (-176.615, ~-41.0,
     -4.96) (three stacked water planes each) and a five-instance cluster at
     x -120 to -141, y -36.25, z -36 to -77;
   - "Bookseller's stalls" -> the merchant-tent street (see PL-B-03);
   - "Guardhouse" -> `EM-Cover_Guardpost_Med01` at (131.70, -40.80, 133.18) and
     (154.13, -40.80, -57.35), or `GA-GuardPost00` (6 instances);
   - "Science tent" -> the `EM-Tent_med00` / `EM-Tent_small00` rows at x 154-193;
   - "Operations Center yard" -> `EM-Quartermaster01` at (188.51, -41.22,
     156.71);
   - "Blackstock's office" -> world 68, see item 3.

## How this cluster was checked

| Tool | Invocation | What it answered |
|---|---|---|
| `nav_inspect` | `nav_inspect data/spaces/harset.nav --probes <file> --h-tol 1.2 --v-tol 4.0` | on-mesh verdict, connected component, horizontal and vertical distance. The tolerances mirror `NavGraph::is_point_valid` (`agent_radius * 2` = 1.2 m horizontal, +4.0 m `JUMP_HEIGHT_TOLERANCE`); `locate_within` is tolerance-first, so a buried sheet does not out-rank the floor |
| `obj_slab` | `obj_slab $O\Harset --levels 0.5 --at NAME=X,Y,Z,HALF_XZ,HALF_Y` | true floor Y per column, by near-horizontal area and triangle count; sealed-vs-open geometry; whether a column is a slope (`flat<=5deg 0.0 m2`) rather than a floor |
| `archetype_census` data | `placements/data/Harset_arch_positions.tsv` (941 instances) | landmark instance positions, and the uniqueness arguments (`HP-*` twice, `GA-Viewscreens00` once, `GA-Tow*` three times) |
| SigNoz `last_valid` | `placements/data/harset_lastvalid_probes.txt` | 33 server-accepted player positions. All 23 re-probed here are on-mesh, which is what makes them usable as a walkability control; the OP-CORE cluster (x 195-222, y -38 to -41, z -12 to +6) is the only TELEMETRY-class evidence in this cluster |

## Tests

All in `crates/services/src/cell/spawner/tests/harset/world57_placement.rs`
(live-DB, `require_db_or_skip!`):

| Test | Fails when |
|---|---|
| `world57_placement_rows_are_seeded_with_their_tags_and_templates` | any of the 15 rows is removed, renumbered, retagged, repointed at another template, made mobile, or loses `respawn_secs` |
| `no_world57_placement_is_hostile` | a placed row points at a hostile template (D-H03) - the realistic slip is reaching for the hostile twin 221/222/223 |
| `world57_interact_tags_from_merged_chains_all_have_a_spawn_row` | a merged world-57 chain dispatches on a tag no spawn row carries. Derived from `content_triggers`, so it catches a transposition a hand-written list cannot |
| `world57_placements_match_their_recorded_navmesh_verdict` | a row recorded on-mesh goes off-mesh, **or** a row recorded off-mesh becomes on-mesh (which is the GH1 tripwire, and also the signal to revisit `is_stationary`). Controlled by a ring-pad assertion so a mesh that failed to load cannot pass it |
| `world57_named_regions_load_and_enclose_their_landmarks` | a region is removed, renamed, changes shape, leaves the `AreaSet` type, resolves to anything but four points, or stops containing the landmark instance position it was sized for |
| `dotted_harset_regions_all_belong_to_world_57` | a `Harset.`-prefixed set is not an `AreaSet` or points at another world - including one added later on the reserved ids 2108/2109 |
