# T-01 offline placement: Copplemann and the Repair Facility props (mission 568)

> **Title**: T-01 offline placement for mission 568
> **Type**: Reference
> **Audience**: Tollana campaign engineers, T-02 and T-03a coders, the UAT driver
> **Last updated**: 2026-10-17
> **Companions**: [T-01 contract](T-01-source-map-contract.md), [placement register](../tollana-placement-contract.csv), [ID register](../tollana-id-register.csv), [map study](../../debug-area/map-selection-study.md)

Label: `CIMMERIA_RECONSTRUCTION`. Final in-client check: **UATPending**.
Nothing here is client-verified. No lab, client, debugger or server was used;
no seed, code or CSV was edited. Numbers come from the repo navmesh
`data/spaces/tollana.nav` (probed with `nav_inspect`) or the offline cooked
map, or are labelled as assumptions.

## Result

World 19. Columns as in `spawnlist`: `x, y, z, heading` (y up; heading in
radians, the same units as the seeded DHD row). `y` is the navmesh floor
height at that spot (see "Height").

**Primary set** (north-west of the gate ramp, flat plaza floor):

| spawn_id | template | role | x | y | z | heading |
|---|---|---|---|---|---|---|
| 19001 | 420 | Captain Copplemann | 206.0 | -0.58 | -543.0 | 2.315 |
| 19002 | 421 | Food Duplication Unit, Core | 199.5 | -0.93 | -540.0 | 2.025 |
| 19003 | 421 | Food Duplication Unit, Nest | 195.5 | -1.01 | -540.0 | 2.025 |
| 19004 | 421 | Food Duplication Unit, Fungus | 197.5 | -1.01 | -536.54 | 2.025 |

**Fallback set** (north-east, the other side of the gate; use whole, never
mixed with the primary):

| spawn_id | template | role | x | y | z | heading |
|---|---|---|---|---|---|---|
| 19001 | 420 | Captain Copplemann | 205.0 | -0.56 | -544.0 | 2.114 |
| 19002 | 421 | Food Duplication Unit, Core | 218.5 | -1.01 | -536.8 | -2.709 |
| 19003 | 421 | Food Duplication Unit, Nest | 214.5 | -1.01 | -536.8 | -2.709 |
| 19004 | 421 | Food Duplication Unit, Fungus | 216.5 | -1.01 | -533.34 | -2.709 |

Tags and set names are T-02/T-03a decisions and are not chosen here. The ID
register already claims 19001 and template 420; 19002-19004 and template 421
are as briefed and not yet in the register.

| Label | Meaning |
|---|---|
| MEASURED | Produced by a `nav_inspect` run or a cooked-map extract in this session |
| DERIVED | Arithmetic on measured values |
| ASSUMED | Not measured; the in-client checks below settle it |

## Method

1. `nav_inspect` (`crates/navmesh-extractor/src/bin/nav_inspect.rs`) on
   `data/spaces/tollana.nav` through the build lane. Exit codes: 0 ok, 2 a
   probe out of tolerance, 3 probes in different components. `dy` in its
   output is `probe.y - surface.y`, so the snapped floor is `probe.y - dy`.
2. A 63 x 63 grid at 1 m (3,969 probes, probe y = -1, x 170..232, z -562..-500)
   to map floor height, holes and components around the gate. A scratch
   script (not committed) turned it into a clearance map and a Dijkstra walk
   estimate.
3. Candidate points, 8-point rings (1.5 m Copplemann, 2.0 m props) and route polylines at `--h-tol 0.5 --v-tol 1.0` (segments `--h-tol 0.3`).
4. `tools/extract_actors.py` on the cooked QA `Maps\Tollana` tiles (read-only)
   for actors near the gate. UE to BigWorld mapping:
   `BW = (UE.y/100, UE.z/100, UE.x/100)`, which reproduces both ring rigs.

## Anchors (MEASURED)

| Probe | Component | Poly | Snapped floor y | Note |
|---|---|---|---|---|
| gate 4 (211.045, 0.606, -547.644) | 89 | 17328 | -0.03 | seed y is 0.64 above the floor |
| DHD 39 (209.748, -1.008, -552.898) | 89 | 17320 | -0.64 | seed y is 0.37 below the floor |
| ring 155 (235.244, -1.109, -571.308) | 89 | 17384 | -1.13 | |

Component 89 is the large main component (1,126,323 m^2). Gate, DHD, ring and
every candidate are on it, so all are mutually connected on the mesh. The
urban pad/centre and walkway anchors in the contract and map study were not
re-probed here. The west gallery (-770, 9, 380) is the failed void-fall probe
and is not used anywhere.

## Local floor and holes (MEASURED, 1 m grid)

- Open plaza floor is -1.01. A ramp rises east to the gate: -0.93 at
  x = 199.5, -0.58 at x = 206, about -0.03 at the arrival. A raised platform
  (floor above -0.45) spans roughly x 205..219, z -551..-544.
- Sunken floor (-2.2) fills the north-west quadrant (x < 199, z -506..-538)
  and the east side (x > 224). The nearest sunken cell to each candidate is at
  least 8.5 m away (Fungus 8.5, Nest 9.6, Core 12.4; fallback Core 8.9, Nest
  12.3, Fungus 8.7).
- Cells more than 0.4 m from any polygon ("holes", 796 of 3,969) near the
  gate: the gate body and a block north-east of it at x 211..218, z -553..-542;
  a west-arm strip at x 182..192, z -552..-544; an east-arm block at
  x 226..232, z -551..-545; a north-arm block at x 205..212, z -531..-519; thin
  divider strips at x 199..200, 208..209 and 218..219, z -528..-519; and
  step-edge strips on the sunken boundaries.
- Distance from each candidate to the nearest hole cell: Copplemann 4.5 m
  (gate-side hole at (210,-545)); Core 8.9; Nest 7.2; Fungus 5.0 (a
  sunken-step strip at (194,-533)). Fallback: Copplemann 5.1, Core 5.4, Nest
  5.2, Fungus 5.5. No candidate or its 8-point ring touches a hole.

## Probe results for the candidate points (MEASURED)

| Point | Component | Poly | h | dy | Floor y | Ring (8 points) |
|---|---|---|---|---|---|---|
| 19001 | 89 | 17345 | 0.00 | 0.00 | -0.58 | all on 89, max floor deviation 0.09 m |
| 19002 | 89 | 17345 | 0.00 | -0.04 | -0.93 | all on 89, max 0.11 m |
| 19003 | 89 | 17345 | 0.00 | 0.00 | -1.01 | all on 89, max 0.06 m |
| 19004 | 89 | 17345 | 0.00 | 0.00 | -1.01 | all on 89, max 0.00 m |
| FB 19001 | 89 | 17345 | 0.00 | 0.00 | -0.56 | all on 89, max 0.09 m |
| FB 19002 | 89 | 17339 | 0.00 | 0.00 | -1.01 | all on 89, max 0.00 m |
| FB 19003 | 89 | 17339 | 0.00 | 0.00 | -1.01 | all on 89, max 0.00 m |
| FB 19004 | 89 | 17339 | 0.00 | 0.00 | -1.01 | all on 89, max 0.00 m |

The primary points share polygon 17345, so there is no polygon edge or step
between the NPC and the props. Same-floor clearance from the 1 m grid (largest
circle of cells within 0.35 m of the point's floor): Copplemann 3 m, Core 6,
Nest 6, Fungus 4; fallback 4, 4, 4, 5.

## Connectivity to gate 4 (MEASURED, polylines)

Every probe on every route below is on component 89 and inside tolerance
unless stated.

| Segment | Length (m) | Probes | Result |
|---|---|---|---|
| arrival (211.045, -547.644) -> Copplemann | 6.9 | 7 | all ok |
| Copplemann -> prop centroid (197.5, -538.85) | 9.5 | 10 | all ok |
| Copplemann -> Core | 7.2 | 15 (0.5 m) | all ok |
| Core -> Fungus, Fungus -> Nest, Nest -> Core | 4.0 each | 8-9 each | all ok |
| arrival -> fallback Copplemann | 7.1 | 8 | all ok |
| arrival straight to fallback props | 13.4 | 14 (1 m) | 5 probes out of tolerance, 0.57-1.48 m off |

A second, finer straight run toward the fallback cluster had 3 of 34 probes
0.66-0.83 m off. Both clip the hole north-east of the gate (x 213..217,
z -545..-542), which is why the fallback route length below is a grid estimate
that bends around it.

## Spacing (DERIVED, horizontal)

Primary:

| Pair | Distance (m) | Requirement |
|---|---|---|
| arrival -> Copplemann | 6.9 | 4-8 m: met |
| Copplemann offset from the arrival line (z = -547.644) | 4.6 | off the line: met |
| Copplemann -> DHD 39 | 10.6 | not blocking: met |
| Copplemann -> ring 155 | 40.7 | not blocking: met |
| Core -> DHD 39 | 16.5 | met |
| Core, Nest, Fungus pairwise | 4.00 each | 3-5 m apart: met |
| Copplemann -> Core / Fungus / Nest | 7.2 / 10.7 / 10.9 | |
| centroid (197.5, -538.85) -> each prop | 2.3 | within 5.0 m of all three: met |

`MAX_INTERACT_DISTANCE` is 5.0 m (`crates/cell-world/src/cell/space_manager/
interact_range.rs`). A player at the centroid is 2.3 m from each prop; one
standing 1.5 m in front of any prop is about 5.5 m from the far two, so the
"within range of each other's approach" ask holds near the centroid, not from
every approach direction. Whether the client applies a smaller model-specific
range is ASSUMED unknown.

Fallback: arrival -> Copplemann 7.1; props 4.0 m apart (equilateral);
Copplemann -> nearest prop (Nest) 11.9, Core 15.3; props 11.4-15.3 m from the
arrival; Core -> DHD 18.3.

## Walking route length (DERIVED)

Primary, measured straight polylines:

| Leg | Length (m) |
|---|---|
| arrival -> Copplemann | 6.9 |
| Copplemann -> Core | 7.2 |
| Core -> Fungus | 4.0 |
| Fungus -> Nest | 4.0 |
| **arrival -> Copplemann -> Core -> Fungus -> Nest** | **22.1** |
| arrival -> Copplemann -> centroid | 16.4 |

Direct arrival -> prop, 1 m grid Dijkstra (8-neighbour, step limit 0.45 m per
metre of run; a stand-in for the real agent slope): Core 14.3, Fungus 17.6,
Nest 18.3. Fallback (grid only): Copplemann 7.7, Nest 13.9, Core 16.7, Fungus
18.7. These are not Detour paths. At run speed the route is a few seconds.

## AoI notes (DERIVED)

- Gate AoI is 150 m enter, 175 m leave. Every actor in both sets is at most
  17.5 m (straight line) from the arrival point, so each is inside the enter
  radius at arrival and about 157 m inside the leave radius: no enter/leave
  churn while walking this route.
- Added to the gate neighbourhood: 4 entities (1 NPC, 3 props), static. No
  per-tick AoI refresh is needed. If Copplemann later gets a patrol or leash,
  hand route shape and leash radius to npc-ai-spawn-advisor; none is set here.

## Cooked-map actors near the gate (MEASURED, then DERIVED mapping)

| Actor | BW x, y, z | Distance to arrival |
|---|---|---|
| gate prefab | 211.5, -1.0, -547.8 (yaw 180) | 0.5 m |
| `TriggerVolume` | 208.2, 2.1, -548.0 | 2.8 m |
| `InterpActor` stack (16, gate animation) | 214.9, 3.4, -547.8 | 3.8 m |
| unnamed `PrefabInstance` P1 | 210.5, -1.0, -541.0 | 6.7 m |
| unnamed `PrefabInstance` P2 | 210.2, -1.0, -536.5 | 11.2 m |
| east arm prefab + `Trigger` | 229.0, -0.2, -547.8 | 18.0 m |
| north arm prefab + `Trigger` | 208.6, -0.2, -527.4 | 20.4 m |
| south arm prefab + `Trigger` | 208.7, -0.2, -568.3 | 20.7 m |
| west arm prefab + `Trigger` | 188.2, -0.2, -547.9 | 22.8 m |
| outer ring of 12 prefab + `Trigger` pairs | 179..241, -518..-580 | 33-38 m |
| ring rig prefab + `InterpActor`s | 236.0, -1.3, -575.0 | 37.1 m |

- The extractor reports only the class name `PrefabInstance`; archetype names
  are not resolved. **No Tollan furniture, Repair Facility or food-unit
  geometry was identified.** The Repair Facility is a reconstructed location
  on open floor, not a found building. No named quest actor exists here (same
  as the T-01 contract).
- The `TriggerVolume` at (208.2, 2.1, -548) is 2.8 m from the arrival, and
  each arm and outer prefab has a `Trigger` about 3.5 m above its prefab.
  Their purpose and radius are UNRESOLVED; check 8 covers them.

## Height (ASSUMED, with measured input)

The y values are navmesh floor heights. Existing seeds are not floor heights
(DHD 39 is 0.37 m below its floor, gate 4 is 0.64 m above), because authored
values are model origins. Whether templates 420 and 421 have origins at the
feet is ASSUMED. If a prop floats or sinks, adjust y for that template only;
x and z stay. Core sits on a 0.11 m local slope; if it looks tilted, move it
1.0 m west and re-probe.

## Heading (ASSUMED, with code evidence)

- Forward is `(sin h, cos h)` in `(x, z)`: `crates/cell-console/src/cell/
  console/abilities/dummy.rs:269` states the convention. Under it gate 4 (yaw
  -1.571) faces west, matching the measured layout (arrival on the west face).
- Copplemann's heading points from him to the arrival point. Primary props
  face Copplemann (bearing from the centroid, 2.026, rounded to 2.025).
  Fallback props face the arrival point (-2.709). A prop model's front axis is
  unknown, so prop facing is the weakest assumption; it is cosmetic. If an
  actor faces away, add pi (3.1416).

## Why these spots

- Primary: the widest flat component-89 pocket within 20 m of the arrival,
  with 6-7 m clearance at the props and a short ramp walk. It is on the
  opposite side of the arrival from the DHD (south) and the ring (south-east),
  so nothing queues across a console.
- Fallback is on the other side of the gate, on a different polygon (17339)
  and floor patch, so one hidden collision cannot fail both sets. Its
  weakness is the route around the gate-side hole.

## What was not checked

- No client render, collision, interaction or AoI observation.
- No Detour path query and no live `nav_path`.
- No static-mesh or brush collision beyond the navmesh holes.
- Prefab identities (P1, P2, arms, triggers) are unresolved.
- Whether mission 568 steps point at these exact spots is a T-03a decision;
  this document places actors and binds nothing.

## In-client checks the user must run later (UATPending)

Ask before any lab use. Reach the gate by dialling gate 4, or a GM
same-world jump to (211.045, 0.606, -547.644). Do not use the west gallery.

1. **Arrival and Copplemann.** Entering world 19 through gate 4, Copplemann is
   visible within about 7 m, feet on the floor, clear of the arrival, facing
   the arrival. Fail: floating, sunk, invisible, facing away. Remedy: adjust y
   or add pi to the heading.
2. **Route.** Walk arrival -> Copplemann -> props with normal controls. No
   invisible wall, void or snap-back. Fail: blocked or corrected. Remedy:
   fallback set.
3. **Props.** All three visible, on the floor, individually targetable, no
   z-fighting; Core, Nest and Fungus labels match the text.
4. **Interaction range.** Stand at about (197.5, -538.85): each prop is in
   range with no "too far" message. Then stand 1.5 m in front of Core and try
   Nest and Fungus. Record the distance at which the client refuses.
5. **DHD and ring unobstructed.** Click DHD 39 from its normal spot and dial;
   Copplemann does not block click or camera. Ring console 155 is reachable.
6. **Two clients.** A second character at the gate sees Copplemann and all
   three props. A character at the urban pad lead (570, -1.04, -530) does
   not; walking back toward the gate they appear and stay after a
   leave-and-return.
7. **No drift.** After 60 s Copplemann and the props have not sunk, rotated or slid.
8. **Trigger interaction.** Walking the route does not fire the `TriggerVolume`
   at the arrival or any arm trigger in a way that moves or hides the actors.
9. **Server log.** No `CorrectionSuppressed` for the player on this route and
   no navmesh rejection for the spawns.
10. **Screenshots** of the arrival view and the facility view for the register.

If checks 1 to 5 fail on the primary set, swap in the whole fallback set and repeat.

## Probe exit codes

| Run | Exit | Meaning |
|---|---|---|
| Canonical lane run, primary set + gate, DHD, ring (`--h-tol 0.5 --v-tol 1.0 --gaps`) | 0 | all ok |
| Canonical lane run, fallback set + gate | 0 | all ok |
| Candidate + ring + route file | 2 | all actor and ring probes ok; only the 5 deliberately straight fallback route probes (`RFN_2..6`) are out of tolerance |
| Finer straight run toward the fallback props (34 probes) | 2 | 3 probes out of tolerance |
| Route segment run (Copplemann, Core, Fungus, Nest) | 0 | all ok |
| 1 m grid (3,969 probes) | 3 | spans components 89, 586 and 587; 796 cells are hole cells |

No blocker. The non-zero exits are a property of the straight test routes and
of the survey grid, not of any candidate point.
