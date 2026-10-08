# Placements A: world 61 (Dakara_E1) story rows

> Type: reference (placement ledger). Cluster **A** of the Dakara placement pass (packet DK-02). Written 2026-10-06 against `main` @ `ddd549797`. Method, evidence classes and the NO-IDEA rule: the [Harset METHOD](../../harset-rebuild/placements/METHOD.md), reused unchanged. Census and navmesh facts behind every row: [C-landmark-census.md](C-landmark-census.md). Index and headline findings: [README.md](README.md).

Every coordinate below is a **guess with a stated evidence class and confidence**, not a pin. Nothing here was walked in a client. BigWorld metres, Y up; heading is `atan2(dx, dz)` radians, 0 = +Z, the convention the Harset ledger uses. Things with no usable evidence are in [`## No idea` in the README](README.md#no-idea), not here. Seed ids are not assigned yet: DK-04 and DK-05 take these rows, so "how to correct" names the packet whose seed row to edit.

"Path to gate plaza" means the point is in navmesh component **279** of `data/spaces/dakara_e1.nav`, the component that holds the gate row, the DHD and the CS-02 start point (100, -17.4, 230). A point in another component cannot be reached by an NPC path from the plaza on the shipped mesh, however close it is.

## Camp A: the command, healing and Moh'katan tents (A-01 to A-05)

Rak'nor's line in dialog 6110 puts Bra'tac's command tent "just to the east of the Stargate, next to the healing tent". The map holds four furnished Tau'ri-style military camps with identical contents (three `JF-MilitaryTent00`, `EM-` crates, medical boxes, microscope, computer, view screens, sleep racks). They are camps A to D in the [census](C-landmark-census.md#the-four-furnished-camps). The nearest one to the gate, **camp A**, is 51 m from the gate row at (96.2, 253.2). It is also the nearest camp that holds `EM-MedicalBox00` props (three of them), so it is the best fit for "next to the healing tent".

Camp A has three tents of the same mesh:

| Tent | Mesh position (x, y, z) | What is in it | Reading |
|---|---|---|---|
| T1 | 132.7, -21.5, 270.8 | `EM-ComputerTower00`, two `EM-ViewScreen02`, three `EM-MedicalBox00`, `EM-Microscope00`, tables, chairs, cabinets, four `JF-SleepRack01` | the equipped tent: the only computer in the camp |
| T2 | 129.5, -21.5, 292.9 | table, chairs, cabinet, one sleep rack, merchant-basket stacks | a briefing or sleeping tent |
| T3 | 152.6, -21.2, 282.1 | ammo crates, baskets, pallets, brazier, sleep racks | a supply tent |

The three are 20 to 26 m apart: **adjacent**, which is what DK-04 needed to know (one shared exit point is possible, row A-04). Which tent is the command tent and which the Med Tent is **not recoverable from the map**: the contents overlap. The assignment below is the reading with the most evidence per tent, and any permutation is a one-line edit.

The mesh has no door geometry the extractor can see (the doorway markers of the prefab tents are collision-off stubs, and these three tents are not prefabs), so the door side is unknown. A flap row is therefore an **approach point on the plaza side of the tent**, 12.5 m from the tent centre toward the gate (T3 toward the court, because T1 sits between it and the gate). A flap is a prop that opens a chain; it needs no door in the mesh.

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-01 | Command Tent Entrance flap (`_ToCommand`), tent T1 | 61 | 121.50 | -19.04 | 265.20 | 4.2487 | SPEC-DESCRIPTIVE (dialog 6110 "just to the east of the Stargate") + MAP-LANDMARK (camp A, T1 holds the only `EM-ComputerTower00`/`EM-ViewScreen02` pair in the camp) + MAP-GEOMETRY | LOW | on-mesh, component **279**, h 0.00 m, dy -0.08 m; floor -19.04 is a low platform on the plaza side (benches and displays at y -19.1); heading faces the gate row; path to plaza: **yes** | Walk from the gate toward the first camp, 40 m away on the +x, +z side; the tent with the computer | DK-04 flap spawn `_ToCommand` |
| PL-DK-A-02 | Med Tent Entrance flap, tent T2; also the spot for respawner 611 `Med Tent` | 61 | 121.60 | -21.08 | 283.20 | 3.8302 | SPEC-DESCRIPTIVE (dialog 6110 "next to the healing tent") + MAP-LANDMARK (camp A holds three `EM-MedicalBox00`; T2 is a furnished sleeping tent) | LOW | on-mesh, component **279**, h 0.00 m, dy -0.35 m; floor -21.08 (`obj_slab`); heading faces the gate row; path to plaza: **yes** | The tent 22 m to the +z side of the command tent | DK-04 respawner 611 and flap |
| PL-DK-A-03 | Moh'katan's Tent Entrance flap (`_ToMohkatan`), tent T3 | 61 | 164.00 | -21.33 | 276.00 | 5.1933 | INFERRED (the third tent of camp A) + MAP-GEOMETRY | LOW | on-mesh, component **279**, h 0.00 m, dy -0.28 m; this point is on the +x side of T3, away from the other two flaps; heading faces the court between the tents; path to plaza: **yes** | The third tent, 20 m to the +x side of the equipped one | DK-04 flap spawn `_ToMohkatan` |
| PL-DK-A-04 | Shared return point out of world 62 (use when the two tents are adjacent) | 61 | 141.00 | -20.80 | 288.00 | 4.0418 | INFERRED + MAP-GEOMETRY | LOW | on-mesh, component **279**, h 0.00 m, dy +0.15 m; the court between T2 and T3, 20 m from the A-02 flap, 26 m from A-03 and 30 m from A-01; heading faces the plaza | After either flap's exit, you stand in the open court of camp A | DK-04 `cross_world_teleport` target |
| PL-DK-A-05 | Command Terminal prop (`DN_ob_DakaraE1_int_CommandTerminal`) | 61 | 137.10 | -21.38 | 274.20 | 4.1891 | MAP-LANDMARK (`EM-ComputerTower00` at (137.1, -20.5, 274.2) with two `EM-ViewScreen02` at (137.6, -20.6, 272.5) and (137.3, -20.6, 273.5)) | LOW | floor -21.38 (`obj_slab`); **off-mesh** (the nearest polygon is the tent roof or table top, component 477, 5.5 m above the floor), fine for a static prop; heading faces the A-01 flap | Inside T1, on the table with the view screens | DK-03 prop template, DK-05 spawn |

If the correct reading is that the equipped tent is the Med Tent and the table tent the command tent, swap A-01 and A-02's tent names and keep the points; if the owner finds the true tents are in camp B, the same arithmetic applies there (camp B is 54 m from the gate on the other side).

## Rak'nor at the plaza (A-06)

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-06 | Rak'nor, gate-plaza greeter (dialogs 6110, 6111) | 61 | 103.50 | -16.80 | 238.50 | 3.5322 | SPEC-DESCRIPTIVE (a greeter at the gate who points to the tent) + MAP-GEOMETRY (plaza floor) + AUTHORED (DHD row and the CS-02 start) | MEDIUM for the plaza, LOW for the exact metre | on-mesh, component **279**, h 0.00 m, dy -0.04 m; floor -16.80 (`obj_slab`, the same slab as the DHD); 6.2 m from the DHD row (98.1, 237.3), 9.2 m from the start point (100, 230); heading faces the start point; path to plaza: **yes** (it is the plaza) | Arrive by gate or spawn; a Jaffa stands between the DHD and the start point and faces you | DK-05 spawn |

## SG-18 search sites (A-07 to A-09)

The client map has one Tau'ri-flavoured area: `HB-` meshes (`HB-Humvee_02`, `HB-Container00/01` x65, `HB-Cover_Shield_Med_I01`, `HB-Sandbag*`, `HB-Tent_Large00`, `HB-Tent_med00`/`HB-Tent_small00`, `HB-BPortal00`) at x 400 to 530, z 450 to 560, next to `JF-Temple02` and the `JF-Arena00`. No other `HB-` mesh exists elsewhere. A Tau'ri team's camp is the likeliest home of the three SG-18 remains. This is a **naming inference** (the prefix reads as Human Base or similar), so every row is LOW, and all three sit **outside the city wall**, in navmesh component 324 or an island, so no path reaches the plaza on the shipped mesh (see [the gate finding in the README](README.md#the-wall-and-its-gates)).

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-07 | SG-18 remains 1 ("Maj. Louis"), by the `HB-Humvee_02` | 61 | 483.90 | -9.10 | 460.60 | 3.1416 | MAP-LANDMARK (the map's only humvee, at (483.9, -6.3, 456.6), beside `JF-Temple02`) + MAP-GEOMETRY | LOW | on-mesh, component **619** (a small island, not 324 and not 279), h 0.00 m, dy -0.14 m; floor -9.10; path to plaza: **no**; 438 m from the gate | At the Humvee in the camp at x 480, z 457 | DK-03 props and the DK-10 chain |
| PL-DK-A-08 | SG-18 remains 2 ("Lt. Nguyn"), at `HB-Tent_Large00` | 61 | 503.90 | -9.10 | 462.00 | 0.0000 | MAP-LANDMARK (`HB-Tent_Large00` prefab at (503.9, -9.1, 466.0)) + MAP-GEOMETRY | LOW | on-mesh, component **324** (the outer region), h 0.00 m, dy -0.14 m; floor -9.10; path to plaza: **no** on the shipped mesh | The large tent in that camp | same |
| PL-DK-A-09 | SG-18 remains 3 ("Lt. Waters"), at `HB-Tent_small00` | 61 | 440.10 | -9.10 | 497.80 | 1.5708 | MAP-LANDMARK (`HB-Tent_small00` prefab at (444.1, -9.0, 497.8)) + MAP-GEOMETRY | LOW | on-mesh, component **324**, h 0.86 m, dy -0.14 m (the tent footprint blocks the centre; the point is 4 m toward -x); path to plaza: **no** | The small tent on the west edge of that camp | same |

Which remains belongs to which officer is arbitrary (the three name strings are in `texts.sql`, ids 26724 to 26726). The cover density agrees with "no fight here": **0** cover nodes within 100 m of (490, 480).

## The two gates (A-10, A-11)

Mission 1647 sends the player to "defend the Western Gate" and "the Eastern Gate". The map answers part of this. The city wall (components: inside is 279, outside is 324) has **three** arch openings, found because the shipped navmesh's two components approach each other only there, at 1.5 to 3.5 m:

| Opening | Arch prefabs | Gap in the nav between 279 and 324 |
|---|---|---|
| X-minus gate | `JF-HighWallArch00` (-209.3, -21.8, 135.7) + `JF-GuardWallArch00` (-215.0, -14.2, 135.6) | 1.50 m at (-214.2, -13.4, 132.0) |
| X-plus gate | `JF-HighWallArch00` (408.7, -21.8, 202.2) + `JF-GuardWallArch00` (414.3, -14.2, 203.4) | 1.82 m at (414.0, -13.2, 199.5) |
| north opening | `JF-HighWallArch00` (72.1, -21.1, 441.7) | 2.64 m at (74.5, -16.8, 436.6) |

The first two are a **matched pair** (both a high arch plus a guard arch, 623 m apart on the x axis, 67 m apart in z). The third is a single arch. "Western" and "Eastern" name the matched pair, but **which of the two is which is not in the map** (see [the axis question](README.md#the-axis-question)).

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-10 | Defence point inside the **X-minus** gate | 61 | -205.00 | -16.97 | 135.70 | 4.7124 | MAP-LANDMARK (matched arch pair) + MAP-GEOMETRY (the only nav approach between the city and the outer region) + SPEC-DESCRIPTIVE ("Western/Eastern Gate") | LOW-MEDIUM for "this is a gate", LOW for which name | on-mesh, component **279**, h 0.00 m, dy -0.30 m; 7 m inside the arch; heading faces outward through the arch (-x); path to plaza: **yes**; cover nodes within 60 m: 57 | Walk to the end of the city wall at x -212, z 136; a tall stone arch | DK-12 and the mission 1647 packet |
| PL-DK-A-11 | Defence point inside the **X-plus** gate | 61 | 405.00 | -16.97 | 202.80 | 1.5708 | same | LOW-MEDIUM for "this is a gate", LOW for which name | on-mesh, component **279**, h 0.00 m, dy -0.10 m; 6.5 m inside the arch; heading faces outward (+x); path to plaza: **yes**; cover nodes within 60 m: 38 | The matching arch at the other end of the wall, x 411, z 203 | same |

The matched pair is symmetric about x = 99.7, which is the gate prefab's x to within 3 m: the Stargate sits on the axis between them.

**Navmesh warning for DK-20 and the mission 1647 packet:** the shipped mesh does **not** connect 279 to 324 through any of the three arches (gap 1.5 m or more, no link). A hostile spawned outside the wall cannot path to these defence points, and a hostile spawned inside cannot path out. World 61 runs `advisory`, so players are not blocked, but NPC pathing is.

## The Ha'tak plazas (A-12, A-13)

The ring transporters are the exact rows. Each is a `GLB-RingTransporter00_Pf0` prefab with five `InterpActor` pieces at its origin and Kismet "Designer 0: Outgoing Ring Transport!" / "Designer 1: Incoming Ring Transport". Each stands 35 to 40 m from one `GA-Hatak00_Pf0` prefab hanging 370 m above the floor ([census](C-landmark-census.md#gate-dhd-ring-and-hatak-fixed-points)). The player stands on the ring platform surface, 0.19 m above the prefab origin at both.

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-12 | Ha'tak plaza 1, ring transporter (X-minus side; Ha'tak at (-111.7, 353.9, 78.1)) | 61 | -139.33 | -16.17 | 48.95 | 0.0000 | MAP-MARKER (prefab and its five `InterpActor` pieces, exact) + MAP-GEOMETRY | HIGH for the position, MEDIUM for "this is the mission 1652 plaza" | on-mesh, component **279**, h 0.00 m, dy -0.04 m at floor -16.17; prefab origin -16.37, platform 0.19 m above it; path to plaza: **yes**; 296 m from the gate | A ring platform inside the wall at x -139, z 49 | DK-12 and the mission 1652 packet |
| PL-DK-A-13 | Ha'tak plaza 2, ring transporter (X-plus side; Ha'tak at (367.2, 359.6, 136.1)) | 61 | 358.03 | -16.02 | 102.07 | 0.0000 | same | HIGH / MEDIUM | on-mesh, component **279**, h 0.00 m, dy +0.01 m at floor -16.02; prefab origin -16.21, platform 0.19 m above it; path to plaza: **yes**; 289 m from the gate | the matching platform inside the wall at x 358, z 102 | same |

## Superweapon courtyard (A-14 to A-16)

The strings name "Dakara Superweapon Courtyard" as a discovery area and mission 1653 has the player "reach the courtyard" and destroy **two** turret power supplies. The map has one place that fits: a courtyard 320 m from the gate toward -z, at z about -60 to -75, with a ring of twelve `JF-LargeBuilding_Column` meshes (x 100 to 159, z -57 to -73, y -12.4 to -14), a `JF-BridgeLarge` span (x 97 to 163, y -21), terrace statues, and **exactly two `Tol-IonCannon00`** (the map's only two) at (46.5, -12.3, -74.7) and (212.2, -12.3, -58.4), symmetric about x = 129.4. The cover density supports a fight area: 599 cover nodes within 60 m of the courtyard centre.

| ID | What | World | X | Y | Z | Heading | Evidence class | Confidence | Checks run | How to verify in-client | How to correct |
|---|---|---|---|---|---|---|---|---|---|---|---|
| PL-DK-A-14 | Superweapon Courtyard region centre (discovery area) | 61 | 129.40 | -21.53 | -66.50 | 0.0000 | MAP-LANDMARK (column ring, bridge, the two Ion Cannons) + SPEC-DESCRIPTIVE ("courtyard") + MAP-GEOMETRY | LOW-MEDIUM | on-mesh, component **279**, h 0.00 m, dy -0.52 m at floor -21.53; the terrace above it (floor -13.97) is component 336, a separate island; 320 m from the gate toward -z; path to plaza: **yes** | Far from the gate toward -z, between two cannons | DK-04 point set (radius is a DK-04 decision; the two cannons are 166 m apart) |
| PL-DK-A-15 | Turret Power Supply 1, beside Ion Cannon 1 | 61 | 56.50 | -12.36 | -73.70 | 1.4724 | MAP-LANDMARK (`Tol-IonCannon00` at (46.5, -12.3, -74.7)) | LOW | on-mesh, component **279**, h 0.10 m, dy -0.95 m; cannon platform floor -12.36 (`obj_slab`); the point is 10 m toward the courtyard from the cannon; heading faces the courtyard centre; path to plaza: **yes** | The cannon at x 46 | DK-03 prop, mission 1653 packet |
| PL-DK-A-16 | Turret Power Supply 2, beside Ion Cannon 2 | 61 | 202.20 | -12.36 | -59.40 | 4.6152 | same (`Tol-IonCannon00` at (212.2, -12.3, -58.4)) | LOW | on-mesh, component **279**, h 0.60 m, dy -0.60 m; same floor; heading faces the courtyard centre; path to plaza: **yes** | The cannon at x 212 | same |

"Vocuum" (`_int_Vocuum`, the optional objective) has no landmark: it is in the README's No idea list.

## Counts

16 candidate rows on world 61: 2 HIGH for the position (A-12, A-13), 1 MEDIUM for the plaza and LOW for the metre (A-06), 3 LOW-MEDIUM (A-10, A-11, A-14), and 10 LOW (A-01 to A-05, A-07 to A-09, A-15, A-16). Headings of props with no facing meaning (A-07 to A-09, A-12, A-13, A-14) are placeholders, not derived. World 62: [B-world62-tent-interior.md](B-world62-tent-interior.md).
