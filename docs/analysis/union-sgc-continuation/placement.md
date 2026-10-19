# SGC_W1 placement points (US-01)

> **Status:** sealed for SGC_W1 (CIMMERIA_RECONSTRUCTION). Probed against `data/spaces/sgc_w1.nav` (NA26 navmesh, advisory mode), `--h-tol 1.0 --v-tol 1.0`. Packets US-11, US-15, US-16, US-20, US-22, US-30, US-31 and US-32 use the **Sealed** column, not the index proposals.
> **Probe set:** `probes/sgc-w1-points.txt` (sealed values, grouped by component).

## Sealed table

| Point | Use | Proposed (index) | Sealed (x, y, z, heading) | Evidence (nav_inspect) | Notes |
|---|---|---|---|---|---|
| P01 | Harriman, Control Room chair | (172.082, -6.399, 46.335), chair yaw -180 | (172.08, -6.40, 46.70), heading 3.1416 | comp 47, poly 1261, h 0.80, dy -0.04, ok | Moved +0.37 m in z: the chair centre is a navmesh hole (the proposal probes 7.8 m off the Control Room floor; z 46.50 is the first z that passes, at h 1.00, so 46.70 keeps 0.2 m of margin). Seated pose: lab check owed. |
| P02 | Woolsey beside Hammond | (197.5, 1.311, 43.6) | (197.50, 1.31, 43.60) | comp 42, poly 1309, h 0.00, dy -0.13, ok | Kept. NPC data review done (see Owed). |
| P04 | Carter virus terminal (standing point for US-22) | target (-49.48, 2.32, 36.38) | standing (-47.50, 1.31, 34.40); target unchanged (-49.48, 2.32, 36.38) | comp 26, poly 1054, h 0.00, dy -0.13, ok (standing) | The target centre is in a navmesh hole; the standing point is 2.8 m from it (within 5.0 m server range). |
| P05 | Carter at the Failsafe, end of escort | (147.0, 1.30, -197.0) | (147.00, 1.30, -197.00) | comp 3, poly 496, h 0.00, dy -0.14, ok | Kept. |
| P06 | Failsafe Device (interact target) | target (151.184, 1.28, -193.376) | standing (153.20, 1.31, -191.40); target unchanged | comp 3, poly 531, h 0.00, dy -0.13, ok (standing) | Target centre is in a navmesh hole; standing point 2.8 m away. |
| P07 | FailsafeRoom region centre (cylinder r 8, h 6) | (150.0, 1.3, -196.0) | (150.00, 1.30, -196.00) | comp 3, poly 496, h 0.00, dy -0.14, ok | Kept. Region reach check (corridor) owed to the region-trigger packet. |
| P08 | Carter in the Gate Room (1651) | (170.5, -10.9, 12.5) | (164.50, -10.80, 18.00) | comp 47, poly 887, h 0.00, dy -0.04, ok | Moved 8.1 m: the proposal sat 0.5 m off a poly 0.68 m above it, beside the unwalkable strip x 172-175, z 8-18 on the gate axis (no floor poly there, probably the gate ramp). Open floor within 1.0 m, 2.5 m from P13 (same poly). |
| P09 | GateControl region centre (cylinder r 10, h 6) | (173.0, -6.4, 55.0) | (173.00, -6.40, 55.00) | comp 47, poly 1261, h 0.00, dy -0.04, ok | Kept. Covers the Control Room (x 156-190, z 44-70). |
| P10a-f | Requisitions crates (six points) | y -10.95, XZ as proposed | y -10.85, XZ as proposed; standing points (x as the crate, -10.85, 40.00) | standing points: comp 47, polys 1095/1098/1103/1106, h 0.00 to 0.60, dy -0.09 to -0.26, ok. Crate centres: not on a poly at this level (see Notes) | Moved y -10.95 to -10.85 to match the seeded crate (spawn 64, y -10.848) and the room floor (comp 47 polys at y about -10.76, same component as the arrival P13). The crate centres (z 41.7-43.7) sit at the edge of the y -10.85 floor, where nav_inspect picks the poly overhead at y -3.75 (comp 45, a different storey; dy -7.1) instead of the floor poly: do not read that as the floor. The stand-at points are 1.7-3.7 m from the crates (within the 4 m rule). Lab check owed: crate clearance against the wall/shelving. |
| P11 | 1566 Jaffa pair, lead (template 34) | (60.0, 1.311, -120.0) | (120.00, 1.18, -180.00) | comp 3, poly 537, h 0.00, dy -0.26, ok | Moved about 85 m: the proposal was 46-52 m off the navmesh. Sits on the Failsafe-side corridor (comp 3), 32 m from P05. Default 18 u aggro does not reach the Failsafe room (31.9 m to P05); out-of-sight is a lab check owed. |
| P12 | 1566 Jaffa pair, second | (64.0, 1.311, -124.0) | (116.00, 1.18, -176.00) | comp 3, poly 539, h 0.00, dy -0.26, ok | Moved; 5.7 m from P11 (the proposal spacing was 5.7 m). Same checks as P11. |
| P13 | H3 arrival (K5); the player's teleport point after the gate | (172.79, -10.9, 12.0) | (167.00, -10.80, 18.00) | comp 47, poly 887, h 0.00, dy -0.04, ok | Moved 8.3 m, 13.3 m from the gate row. The proposal was off the navmesh (nearest poly comp 39 at y 8.4); the first pass (170.79, -10.90, 11.00) was in tolerance but 0.79 m off a poly 0.83 m above it, with the unwalkable gate-axis strip 1.2 m away, so it failed the 1.5 m clear rule. Every probe cell within 1.5 m of the sealed point is on comp 47 floor (h <= 0.2, dy within 0.15 of -0.1). Never on the gate row (GATE27 at (172.79, -8.47, 6.01)). The three Gate Room Jaffa (rows 65-67, faction 10, default 18 u aggro, spawned at (172.92, -8.48, 5.35)) are 14.0 m away: if alive on arrival they aggro, as the 1568 design intends; the C2 return despawns them (US-34). |
| P14 | Hammond home (spawn 52) | (199.989, 1.311, 43.608) | (199.99, 1.31, 43.61) | comp 42, poly 1304, h 0.00, dy -0.13, ok | Kept (spawnlist row 52). |
| P15 | Player after 1568 (Hammond's office) | (182.0, 1.311, 43.5) | (182.00, 1.31, 43.50) | comp 42, poly 1292, h 0.00, dy -0.13, ok | Kept. |
| P16 | Carter after 1568 | (179.5, 1.311, 45.5) | (179.50, 1.31, 45.50) | comp 42, poly 1292, h 0.00, dy -0.13, ok | Kept. |
| P03 | (dropped by F12) | — | — | — | Not probed. |
| ARRIVAL | Gate arrival reference | (172.79, -10.9, 12.0) | = P13 sealed value (167.00, -10.80, 18.00) | comp 47 | Use P13 for H3 and the arrival. |

## Component groups (why the probe runs per group)

The SGC_W1 navmesh has 68 components. `nav_inspect` exits 3 if the probes land in different components, so the packet's same-component rule is checked per group, not for the whole set:

| Group | Component | Points | Result |
|---|---|---|---|
| Gate Room | 47 | P01, P08, P09, P13 | exit 0 |
| Briefing | 42 | P02, P14, P15, P16 | exit 0 |
| Failsafe | 3 | P05, P06, P07, P11, P12 | exit 0 |
| Requisitions (floor y -10.85) | 47 (shared with the Gate Room) | P10a-f standing points | exit 0 with P13 in the same run |
| Terminal | 26 | P04 | exit 0 |

Last check for each group: `[lane] status=ok exit=0` (groups 47 including the requisitions standing points, 42, 3, 26; rerun of group 3 after the P12 move). Component 45 (y -3.75, x 251-289, z 6-45) is an upper storey above the requisitions room; no point is sealed on it.

**Cross-group routes are not verified.** `--gaps` finds no route within 3 m between components 3, 42 and 47. The player walking from the arrival (group 47, which also holds the requisitions room) to the briefing (42), the terminal (26) or the Failsafe (3) may need the elevator (ElevatorButton rows 55 and 76, unverified). Lab check owed for US-U1/US-U2: walk the route from P13.

## Owed (not done in this packet)

- **NPC review (data-level, from `spawnlist.sql` and `docs/gameplay/npc-ai.md`; not a live check).** Proximity aggro uses `entity_templates.aggro_radius`, default 18 u (NULL; template 34 and 35 set none), vertical band 4 u. P11 to P05 is 31.9 m, to P07 34.0 m, to the Failsafe device 33.9 m, so the Jaffa pair at (120,-180) never reaches the Failsafe room at the default radius; its nearest seeded neighbour is row 63 (123.7, 1.31, -216.0), 36 m away. Rows 57/58 (the 1565 fight, about (-52..-69, -45)) are about 215 m from P11, out of sight. The 1562 attackers (rows 71 and 77, about (-9.5, 42.2) and (-5.0, 47.6)) are 39 m and 44 m from the P04 standing point and 24 m from Carter row 56, so outside 18 u. P14 is Hammond's own spawn (row 52); P02 is 2.5 m from it and P16 3.1 m from Tealc row 53 (at least 1.0 m: ok). P01 is 3.4 m from the gate terminal row 68. Headings: P01 3.1416 (seated); the rest idle, no patrol path. Still owed to the lab: the seated pose, and whether the Jaffa pair is out of sight in the client.
- Elevator or other route between navmesh groups: lab check owed (above).
- Terminal and device target centres sit in navmesh holes; the standing points carry the interaction. The US-22 packet must use the standing point, not the target centre, for its range check.

## Changes from the proposals

- P01: z 46.335 to 46.70.
- P04: standing point (-47.50, 1.31, 34.40) added; target unchanged.
- P06: standing point (153.20, 1.31, -191.40) added; target unchanged.
- P10a-f: y -10.95 to -10.85 (XZ unchanged); standing points (x as the crate, -10.85, 40.00) added for the 4 m interact rule.
- P11: (60.0, 1.311, -120.0) to (120.00, 1.18, -180.00).
- P12: (64.0, 1.311, -124.0) to (116.00, 1.18, -176.00).
- P08: (170.5, -10.9, 12.5) to (164.50, -10.80, 18.00).
- P13 and ARRIVAL: (172.79, -10.9, 12.0) to (167.00, -10.80, 18.00).
- P03: not used (dropped by F12).
- Other points kept (P02, P05, P07, P09, P14, P15, P16).
