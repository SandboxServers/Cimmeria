---
name: navmesh-los-reliability
description: Measured reliability of the navmesh LoS ray vs extracted collision geometry (45% false Blocked), the PRU/desk S11 cause, rejected heuristics, and the fire-time LoS decision
metadata:
  type: project
---

Navmesh line of sight (`NavMesh::line_of_sight`) cannot tell furniture from walls: Recast cuts a hole around every obstacle and keeps no height for it. Measured in NA16 (2026-09-25): 4,000 same-storey Cellblock pairs 4-30 m apart, checked against the extracted OBJ collision geometry at 1.5 m eye heights. 269 of 601 `Blocked` verdicts were false (45%). 5 of 3,399 `Clear` verdicts were false. Treat `Clear` as reliable and `Blocked` as a coin flip.

- S11 (the Find Ambernol drone, spawn 10, never fires at 12-16 m): the med-station desk under the vial is a roughly 7 × 2.5 m navmesh hole. The desk top is at about Y 66.5 and the floor at 65.6. Eye height and off-mesh projection were not the cause.
- Rejected heuristics, with numbers. The "path ≤ 1.15 × straight line" rule gave 251 fewer false blocks and 235 new false clears. The "segmented ray, small hole you can walk around" rule gave 52-148 new false clears and still missed the desk.
- Shipped: `LineOfSight::permits_stationary_attack(dy)`, which ignores `Blocked` for stationary attackers within 4 u of the NPC's height. It is wired through `SpaceManager::attack_line_of_sight` in `fight.rs`.
- (Superseded by NA31: players now get an occluder-only fire-time check, see [[fire-los-and-eye-heights]].) There was no fire-time LoS check in `use_ability`. The client has `CONDITION_FEEDBACK_NoLOS = 40` and the GM `toggleCombatLOS`, so the original server probably checked. Add the check for players only once a geometry occluder exists, or players will get false "no LoS" errors behind desks.

**Why:** anyone tempted to gate aggro, abilities or cover on navmesh `Blocked` needs these error rates first.
**How to apply:** a real occluder is a per-space collision sidecar. Raw Cellblock OBJs are 1.53M triangles, and a span grid at 0.5 m is about 40 MB before cropping, so it needs an owner decision on the shipped artifact. Local OBJs are at `%TEMP%\cimmeria-castle\navmesh\integration\cellblock\chunks_v2`, and the probe scripts are described in the NA16 report. Related: [[pvp-duel-readiness]].
