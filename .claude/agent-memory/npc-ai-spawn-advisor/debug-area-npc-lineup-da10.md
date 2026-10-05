---
name: debug-area-npc-lineup-da10
description: DA-10 (2026-10-05) Z10 Visual NPC Lineup - 161 looks rule, display_name/onBeingNameUpdate nameplates, east-wing rows + ring pad, occluder-scan traps, unmeasurable body sets, live-db-test reload trap
metadata:
  type: project
---

DA-10 (branch content/debug-area-npc-gallery, PR #1260, 2026-10-05): Z10 "Visual NPC Lineup",
161 passive faction-1 display actors, templates 1410-1599 / spawns 13870-14099, seeds
`*_debug_area_lineup.sql`, tags `DebugArea_VisualLineup_<source id>` / `..._NoTemplate_<bs>`.

- **Look** (owner-approved 161) = body_set + sorted components + both colours + skin_tint +
  coalesce(static_mesh,''): 155 looks from 225 templates. Exact columns give 156: only Nerus
  (53, NULL) vs Sandbox Greeting NPC (166, '') split, and they draw alike (doc "Coverage delta"). Six body sets had no template: BS_RaJaff,
  HM_BodySet, BS_AN_Android, BS_MOB_DroneTank, BS_MOB_LennyBaby, BS_Degenerated_Asgard. Guard
  `live_db_debug_area_lineup` counts looks, not templates; props go to a planned Z11.
- **Nameplates:** `name_id` resolves to client-shipped text only; `entity_templates.name` is never
  loaded. Literal labels need `onBeingNameUpdate(WSTRING)` (SGWBeing idx 17, bound for every
  being class): DA-10 added `entity_templates.display_name` -> SpawnRecord -> CellEntity ->
  NpcAoIData, sent after onBeingNameIDUpdate in the cascade. Unseen on a mob in the client.
- **Non-combat:** faction 1 is the switch (player damage gate is faction 10, nothing is hostile
  to 1). `training_dummy` was rejected: it shows 500k/1M Health on non-hostile dummies and DA-02's
  plaza guard pins the marked set to 1310-1314. Event set 570 = "Players default event set"
  (sequences); owner wanted NO event set on the actors.
- **Unmeasurable body sets:** RaJaff (`Ra_500` has no export) and HM_BodySet (no ref mesh) have
  no eye height; spawning them tripped cell-world `live_db_eye_heights` (now `UNMEASURABLE`).
- **Placement:** south compound east wing (x 297-392, z -969..-881, floor occluder Terrain
  y 6.58), doorway (300, -897), 15 rows, full. Ring pad for Z10 (handed to another worker):
  (287.0, 6.80, -914.0), console (290.0, 6.58, -915.3), courtyard NE corner, clear disc 8 m.
  The slab at (212-300, z -632..-666, y 16-19) north of the pit is a separate nav component.
- **Scan traps:** `occ.column()` returns heightfield terrain as ONE span (patch min..max), and
  there are buried geometry slabs (y 15.2 under the y 23.06 terrace, -15/-18.9 under the
  basins). Pick the TOPMOST span the nav agrees with, never "prefer geometry". An #[ignore]
  scratch test in `debug_area/` recompiles in ~5 s.
- **live-db-test.sh reloads the worktree DB first**, so manual psql inserts are wiped; do revert
  proofs by editing the seed file (spawnlist.tag is UNIQUE: a renamed tag must be new).
- AoI: the lineup is within 150 m of Z1/plaza/lords, so every compound arrival creates 161 more
  NPCs - client cost unmeasured. Lab 2026-10-05: the watchdog killed the client twice while
  world 1300 loaded. The heartbeat is answered on the main thread, and 5 failed polls of 1 s + 5 s
  timeout (about 30 s) terminate it; wait_for probes can't keep it alive. The fff80002 chunk was 011's
  output (rz10). Enter via Castle_CellBlock then `.gotolocation`; direct login into 1300 dies.

- **Client memory is the real cap (lab 2026-10-05, #1260+#1274):** with all 223 world-1300 NPCs
  delivered at Z1, the 32-bit client's working set went 1.1 -> 3.2 GB in 10 s (1-2 s hitch per
  frame) and died on `CreateTexture E_OUTOFMEMORY`. ~150 NPCs (wedged run) survived at 43 FPS.
  Unique-costume NPCs cost texture memory each; never put 160 distinct looks in one 150 m AoI.
- **Oversize cascades:** Petbe #221's createOnClient cascade is 1504 B encrypted with its label;
  the AoI cascade send was unguarded until #1274 (oversize_fragmented). Measure cascade size with
  `build_create_entity_cascade` from the seed when adding labels or big kits.
Related: [[debug-area-map-survey]], [[debug-area-da03-stations]], [[template-seed-column-traps]].
