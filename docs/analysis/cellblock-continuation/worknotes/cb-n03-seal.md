# CB-N03 seal check: flank and cover reachability (681 / 686)

> **Type:** Reference
> **Audience:** Cellblock coordinator and the CB-N03 owners
> **Last updated:** 2026-10-16

Packet: CB-N03s (`npc-ai-spawn-advisor`, static: no lab, no Ghidra, no code, no build). Date 2026-10-16. Worktree base `e10fd1f0a`.

## Verdict: No static fault

Nothing in the seed or the AI code, read statically, stops `player_flanked_npc` from firing for any of the four guards. CB-N03 therefore has no sealed miss to hand a coder. **CB-N03a (seed fix) and CB-N03b (AI defect) are not sealed and must not be dispatched.** CB-N03 becomes BlockedEvidence on CB-U05, consistent with route-baseline row CB-B06 (no current defect) and the unmeasured CB-U05.

## Evidence

### The four guards (all template 24 "NID Guard", faction 10, `use_cover = true`, ability set 3 / ability 559)

World 12 cover nodes, horizontal distance from the authored spawn (spawn-hold radius is `COVER_ARRIVE_RADIUS` = 1.5 u, defined at `crates/cell-cover/src/cell/cover/ai_integration.rs:61`; the vertical cap `SPAWN_HOLD_MAX_DY` = 2.0 u is at `crates/cell-world/src/cell/cover/stance.rs:40`). Set id = chunk id, 1200000 + n.

| Spawn | Tag | Spawn position | Nearest nodes (chunk/node, dist, dy, height/quality, orient) | Spawn-hold |
|---|---|---|---|---|
| 29 | MessHall_Guard1 | -96.25, 34.591, -91.59 | 1200046/0 0.63 u, -0.03, High/Best, 0.304 rad; next 1200053/0 7.56 u; 1200055/0 9.20 u | Holds 1200046/0 (pinned by `npc_ai_cover_seed.rs`) |
| 28 | MessHall_Guard2 | -95.89, 34.591, -98.808 | 1200053/0 0.96 u, -0.03, High/Best, 0.009 rad; next 1200016/0 7.07 u; 1200046/0 7.61 u | Holds 1200053/0 (computed; no test pins it) |
| 32 | Hallway05_Guard1 | -100.16, 24.67, -43.895 | 1200056/1 0.77 u, -0.03, Mid/Better, 6.281 rad; 1200056/0 0.97 u, 3.148 rad; next 1200042/1 5.26 u | Holds 1200056/1 (nearest free; computed) |
| 33 | Hallway05_Guard2 | -101.5, 24.67, -51.3 | 1200044/3 1.19 u, -0.06, Mid/Better, 6.245 rad; /2 1.39 u; /0 1.64 u; /1 1.78 u; next 1200050/x 6.7 u but dy +14.85 (upper floor, excluded) | Holds 1200044/3 (computed; /2 is the fallback if /3 is taken) |

All four are within 1.5 u of at least one eligible node on the same floor (|dy| < 0.1), so each guard spawns holding a slot. They do not depend on the seek radius (`MAX_COVER_DISTANCE` 30 u, `IN_RANGE_MAX_MOVE` 10 u). A guard that loses its slot picks again among nodes within 10 u (in range) or 30 u, `PICK_RANGE_MARGIN` 2 u inside ability reach, with a navmesh shot check (NA23).

### Can `player_flanked_npc` fire?

1. Raise point: `crates/cell-combat/src/cell/service/npc_ai/fight_cover.rs`, `CoverDecision::Released { reason: Flanked }` calls `fire_flank_triggers` (line 200-201): `npc_flanked`, then `player_flanked_npc`, with `npc_template` = `entity.npc_name`.
2. `npc_name` is set to `record.template_name` at spawn (`crates/cell-world/src/cell/space_manager/spawn.rs:244`) = "NID Guard", which equals the chain filter (chains 1141 and 1142 trigger key `'NID Guard'`).
3. Release rule (`crates/cell-cover/src/cell/cover/ai_integration.rs` step 1): the NPC holds a slot and `is_flanked(node.pos, node.orient, threat_pos)` is true when the facing dot is below -0.342, i.e. the target is more than 110 degrees off the node's facing. The flank test runs before the out-of-range test, so a flank wins even when the target is also out of reach.
4. Required AI state: Fighting with a top-threat target (the cover branch runs from the fight tick). The target must be a player entity with a DB `player_id`, else `fire_player_flanked_npc` returns (`cover.rs:362-374`).
5. Chain gates: 1141 needs mission 681 active and objective 2725 not completed; 1142 needs 686 active and 2731 not completed. The replay tests `mission_681_686_flank.rs` (364 lines) and `mission_681_686_backstop.rs` (334 lines) and `cover_flank_tests.rs` already guard the chain side, the cross-room isolation, and that completing only the flank objective leaves the mission active.
6. Order of the two events and the Released arm are pinned by `npc_ai_cover.rs` (`npc_ai_fight_flanked_npc_raises_both_flank_events_in_order`, and the release test at ~290-313). `npc_ai_cover_seed.rs::flanking_the_guard_releases_its_slot_and_stance` pins the real-data release for MessHall_Guard1 only.

### Gaps that static reading cannot close

- No test pins the spawn-hold or the flank release for MessHall_Guard2 or the two Hallway05 guards on real data. The computed distances above say they hold, but that is arithmetic over the seed, not a run. This is a test-coverage gap, not a fault, and the packet forbids writing code.
- Whether a player can stand in the flank arc of each held node, on the navmesh, reachable, and inside the guard's reach/aggro, is a geometry question. Node facings are about +x for all four slots (Mess Hall 0.30 and 0.01 rad; Hallway05 about 0 / pi). The flank arc is the half-space behind each node (x less than the node x, roughly) plus 20 degrees. Whether that space is walkable and not blocked by the long table or walls is not decided here.
- `npc_ai_cover_seed.rs` shows the Mess Hall tables block most lines across the room (NA23), so a player flanking from behind a table may drop the guard's shot, but the flank release does not need a shot: it needs only the held slot and the threat's position.
- Known limits already recorded beside chain 1141/1142 still apply: credit goes only to the top-threat player; flank is non-gating, so a ranged kill still completes the mission.

## CB-U05 lab steps (for the lab-lease window, not run here)

Preconditions: fresh character at the Mess Hall step (681 active), then at Hallway05 (686 active). SigNoz service `cimmeria-cell` (or the cell service name in use); filter on world `Castle_CellBlock`.

1. Before any shot, confirm the hold: log rows `target = cover.hold`, `event = spawn_reserved`, for npc names MessHall_Guard1 (1200046/0), MessHall_Guard2 (1200053/0), Hallway05_Guard1 (1200056/1), Hallway05_Guard2 (1200044/3).
2. Mess Hall: engage from the front (east of the guards, about 12 u), confirm `decision_outcome = stay_in_cover`. Move around behind the long table to roughly west of x = -97 on the guards' row, inside their reach. Watch `cover.flank_check` rows with `flanked = true`, then `decision_outcome = cover_released_flanked`, then `fire_player_flanked_npc: matched` and the onObjectiveUpdate for 2725. Repeat for Guard2 (1200053/0).
3. Hallway05: same, objective 2731. Guard1 flank point west of x about -100, near z -43.9; Guard2 west of x about -100.4, near z -50.9. Check the standing point is on the navmesh (the client can walk there) and within about 28 u (ability reach) of the guard.
4. Negative checks: kill both guards from the front and never flank, confirm the mission still completes (non-gating); confirm a flank in Hallway05 does not complete 2725 (isolation).
5. Record per guard: held slot, `reason` on release, whether `fire_player_flanked_npc: matched` appeared, and whether 2725/2731 completed. Any guard whose release never reaches `cover_released_flanked` while the player is demonstrably behind the node is the first real fault; classify it as seed (node orient or position wrong) or AI (release or event not raised) from the `cover.flank_check` row fields (`flanked`, `at_slot`, `reaches`).

## Child packets

None sealed. CB-N03a and CB-N03b stay BlockedDependency; the architect pass should mark CB-N03 BlockedEvidence on CB-U05. If the architect wants a static follow-up that needs no lab, the one candidate is a test-only extension of `npc_ai_cover_seed.rs` that spawns the other three guards from real seed rows and asserts hold plus flank release. That is a coverage addition, not a fix for a miss.

## Measurement

About 60k tokens and 12 tool calls, static reading only. No build, lab, Ghidra or seed edits. Largest cost: the fight-cover and cover integration reads. Packet estimate was 40k; the overrun is the cover-node distance table, which the packet required.
