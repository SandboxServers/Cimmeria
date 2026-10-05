---
name: debug-area-npc-lineup-da10
description: DA-10 (2026-10-05) Debug Area NPC lineup - look definition, east-wing rows, occluder-scan traps (buried slabs, heightfield patch max), name column not loaded, unmeasurable body sets, live-db-test reload trap
metadata:
  type: project
---

DA-10 (branch content/debug-area-npc-gallery, 2026-10-05): one faction-1 clone per character
look, templates 1410-1599 / spawns 13870-14099, seeds `*_debug_area_lineup.sql`.

- **Look** = body_set + sorted components + primary/secondary colour + skin_tint +
  coalesce(static_mesh,''), excluding `GLB_Components.*` and `WP-Human.*`: 155 looks from 225
  templates (the lead's 156 counted NULL vs '' static_mesh twice: templates 26/166/167). Six
  body sets had no template: BS_RaJaff, HM_BodySet, BS_AN_Android, BS_MOB_DroneTank,
  BS_MOB_LennyBaby, BS_Degenerated_Asgard. Guard: `live_db_debug_area_lineup` fails on any new look.
- **Event set 570** is "Players default event set" (sequences), on 220 templates incl. creatures;
  not a scripted set, so clones keep it.
- **`entity_templates.name` is never loaded** by the spawner; the nameplate is `name_id` only, so a
  nameless source's clone cannot show its template_name. 82 of 161 clones are blank.
- **Unmeasurable body sets:** RaJaff (`Ra_500` has no export) and HM_BodySet (no ref mesh) have
  no eye height; spawning them tripped cell-world `live_db_eye_heights`, now `UNMEASURABLE`.
  Expected not to render; unverified in client.
- **Placement:** south compound east wing (x 297-392, z -969..-881, floor = occluder Terrain at
  y 6.58, nav within 0.35), doorway (300, -897), 15 rows; the only empty flat walkable floor near
  a ring. Inside it blocks/pavilions at (317-330, -918..-931), (348-375, -914..-931),
  (366-384, -942..-959). The slab at (212-300, z -632..-666, y 16-19) north of the pit is a
  separate nav component (partial paths) - unusable.
- **Scan traps:** `occ.column()` returns heightfield terrain as ONE span (patch min..max), and
  there are buried geometry slabs (y 15.2 under the y 23.06 terrace, -15/-18.9 under the
  basins). "Prefer geometry" picks the buried slab; pick the TOPMOST span the nav agrees with.
  A per-1 m scan via an #[ignore] scratch test in `debug_area/` recompiles in ~5 s.
- **live-db-test.sh reloads the worktree DB first**, so manual psql inserts are wiped; do
  revert proofs by editing the seed file.
- AoI: the lineup is within 150 m of Z1/plaza/lords, so every compound arrival creates ~160 more
  NPCs - client cost unmeasured.

Related: [[debug-area-map-survey]], [[debug-area-da03-stations]], [[template-seed-column-traps]].
