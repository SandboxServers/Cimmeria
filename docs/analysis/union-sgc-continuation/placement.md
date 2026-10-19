# SGC_W1 placement points (US-01)

> **Status:** sealed for SGC_W1 (CIMMERIA_RECONSTRUCTION). Probed against `data/spaces/sgc_w1.nav` (NA26 navmesh, advisory mode), `--h-tol 1.0 --v-tol 1.0`. Packets US-11, US-15, US-16, US-20, US-22, US-30, US-31 and US-32 use the **Sealed** column, not the index proposals.
> **Probe set:** `probes/sgc-w1-points.txt` (sealed values, grouped by component).

## Sealed table

| Point | Use | Proposed (index) | Sealed (x, y, z, heading) | Evidence (nav_inspect) | Notes |
|---|---|---|---|---|---|
| P01 | Harriman, Control Room chair | (172.082, -6.399, 46.335), chair yaw -180 | (172.08, -6.40, 47.30), heading 3.1416 | comp 47, poly 1261, h 0.20, dy -0.04, ok | Moved +0.97 m in z: the chair centre is a navmesh hole. npc-ai-spawn-advisor pass not run here: lab check owed for the seated pose. |
| P02 | Woolsey beside Hammond | (197.5, 1.311, 43.6) | (197.50, 1.31, 43.60) | comp 42, poly 1309, h 0.00, dy -0.13, ok | Kept. NPC review owed (see below). |
| P04 | Carter virus terminal (standing point for US-22) | target (-49.48, 2.32, 36.38) | standing (-47.50, 1.31, 34.40); target unchanged (-49.48, 2.32, 36.38) | comp 26, poly 1054, h 0.00, dy -0.13, ok (standing) | The target centre is in a navmesh hole; the standing point is 2.8 m from it (within 5.0 m server range). |
| P05 | Carter at the Failsafe, end of escort | (147.0, 1.30, -197.0) | (147.00, 1.30, -197.00) | comp 3, poly 496, h 0.00, dy -0.14, ok | Kept. |
| P06 | Failsafe Device (interact target) | target (151.184, 1.28, -193.376) | standing (153.20, 1.31, -191.40); target unchanged | comp 3, poly 531, h 0.00, dy -0.13, ok (standing) | Target centre is in a navmesh hole; standing point 2.8 m away. |
| P07 | FailsafeRoom region centre (cylinder r 8, h 6) | (150.0, 1.3, -196.0) | (150.00, 1.30, -196.00) | comp 3, poly 496, h 0.00, dy -0.14, ok | Kept. Region reach check (corridor) owed to the region-trigger packet. |
| P08 | Carter in the Gate Room (1651) | (170.5, -10.9, 12.5) | (170.50, -10.90, 12.50) | comp 47, poly 884, h 0.50, dy -0.68, ok | Kept. |
| P09 | GateControl region centre (cylinder r 10, h 6) | (173.0, -6.4, 55.0) | (173.00, -6.40, 55.00) | comp 47, poly 1261, h 0.00, dy -0.04, ok | Kept. Covers the Control Room (x 156-190, z 44-70). |
| P10a-f | Requisitions crates (six points) | y -10.95, XZ as proposed | y -3.70, XZ as proposed | comp 45, polys 1047/1053, h 0.00, dy 0.00 to +0.06, ok | Moved y: the room floor is y -3.70, not -10.95. Crate XZ unchanged. |
| P11 | 1566 Jaffa pair, lead (template 34) | (60.0, 1.311, -120.0) | (120.00, 1.18, -180.00) | comp 3, poly 537, h 0.00, dy -0.26, ok | Moved about 85 m: the proposal was 46-52 m off the navmesh. Sits on the Failsafe-side corridor (comp 3), 32 m from P05. Aggro radius vs the Failsafe room and out-of-sight check owed (npc-ai-spawn-advisor, lab check owed). |
| P12 | 1566 Jaffa pair, second | (64.0, 1.311, -124.0) | (116.00, 1.18, -176.00) | comp 3, poly 539, h 0.00, dy -0.26, ok | Moved; 5.7 m from P11 (the proposal spacing was 5.7 m). Same owed checks as P11. |
| P13 | H3 arrival (K5); the player's teleport point after the gate | (172.79, -10.9, 12.0) | (170.79, -10.90, 11.00) | comp 47, poly 884, h 0.79, dy -0.83, ok | Moved 2.24 m. The proposal was off the navmesh (nearest poly comp 39 at y 8.4). 1.5 m from P08. Never on the gate row (GATE27 at (172.79, -8.47, 6.01)). |
| P14 | Hammond home (spawn 52) | (199.989, 1.311, 43.608) | (199.99, 1.31, 43.61) | comp 42, poly 1304, h 0.00, dy -0.13, ok | Kept (spawnlist row 52). |
| P15 | Player after 1568 (Hammond's office) | (182.0, 1.311, 43.5) | (182.00, 1.31, 43.50) | comp 42, poly 1292, h 0.00, dy -0.13, ok | Kept. |
| P16 | Carter after 1568 | (179.5, 1.311, 45.5) | (179.50, 1.31, 45.50) | comp 42, poly 1292, h 0.00, dy -0.13, ok | Kept. |
| P03 | (dropped by F12) | — | — | — | Not probed. |
| ARRIVAL | Gate arrival reference | (172.79, -10.9, 12.0) | = P13 sealed value (170.79, -10.90, 11.00) | comp 47 | Use P13 for H3 and the arrival. |

## Component groups (why the probe runs per group)

The SGC_W1 navmesh has 68 components. `nav_inspect` exits 3 if the probes land in different components, so the packet's same-component rule is checked per group, not for the whole set:

| Group | Component | Points | Result |
|---|---|---|---|
| Gate Room | 47 | P01, P08, P09, P13 | exit 0 |
| Briefing | 42 | P02, P14, P15, P16 | exit 0 |
| Failsafe | 3 | P05, P06, P07, P11, P12 | exit 0 |
| Requisitions | 45 | P10a-f | exit 0 |
| Terminal | 26 | P04 | exit 0 |

Last check for each group: `[lane] status=ok exit=0` (groups 47, 42, 3, 45, 26; rerun of group 3 after the P12 move).

**Cross-group routes are not verified.** `--gaps` finds no route within 3 m between components 3, 42 and 47. The player walking from the arrival (group 47) to the briefing (42), the requisitions (45), the terminal (26) or the Failsafe (3) may need the elevator (ElevatorButton rows 55 and 76, unverified). Lab check owed for US-U1/US-U2: walk the route from P13.

## Owed (not done in this packet)

- **npc-ai-spawn-advisor pass** (P01 heading and idle versus patrol, P02, P05, P08, P11, P12, P14, P16; the 1562 attackers' aggro radius at rows 71 and 77): not run. Lab check owed for P11/P12 out-of-sight and aggro-radius checks, and for P01's seated pose.
- Elevator or other route between navmesh groups: lab check owed (above).
- Terminal and device target centres sit in navmesh holes; the standing points carry the interaction. The US-22 packet must use the standing point, not the target centre, for its range check.

## Changes from the proposals

- P01: z 46.335 to 47.30.
- P04: standing point (-47.50, 1.31, 34.40) added; target unchanged.
- P06: standing point (153.20, 1.31, -191.40) added; target unchanged.
- P10a-f: y -10.95 to -3.70 (XZ unchanged).
- P11: (60.0, 1.311, -120.0) to (120.00, 1.18, -180.00).
- P12: (64.0, 1.311, -124.0) to (116.00, 1.18, -176.00).
- P13 and ARRIVAL: (172.79, -10.9, 12.0) to (170.79, -10.90, 11.00).
- P03: not used (dropped by F12).
- Other points kept (P02, P05, P07, P08, P09, P14, P15, P16).
