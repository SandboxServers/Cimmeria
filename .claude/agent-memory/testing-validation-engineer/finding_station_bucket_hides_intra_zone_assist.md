---
name: finding_station_bucket_hides_intra_zone_assist
description: Reach/isolation guards that bucket NPCs by tag prefix skip intra-zone pairs; template assist_radius overrides (NID Guard 24 = 26 u) and the 4 u vertical band are the usual near-misses
metadata:
  type: project
---

2026-10-04, PR #1222 (Debug Area DA-03): `no_station_reaches_another` grouped every `DebugArea_Yard_*` tag as one station, so the damageable pinned-NEUTRAL faction-10 Jaffa vs the hostile pen was never checked. Template 24 (NID Guard) carries `assist_radius` 26 (not the 10 u default), 24.1 u away; only the 4.0 u `AGGRO_VERTICAL_BAND` held, with runtime (navmesh-snapped) dy 4.052. A +0.1 u scene nudge made the shot pull the guard.

**Why:** assist recruits by the *assister's* radius, same faction, HOSTILE-to-players assister; the victim's own override doesn't matter. Seed y is not runtime y: `grounded_spawn_position` snaps non-stationary NPCs onto the navmesh, so compute dy from the scene, not the SQL.

**How to apply:** when reviewing placement/isolation guards, (1) list template-level `assist_radius`/`aggro_radius` overrides among placed templates, (2) check the station-bucketing key doesn't merge a damageable row with a hostile one, (3) probe margins by nudging scene positions (get_entity_mut().position) rather than trusting seed coordinates. Also: `npc_aggression_toward` reads only the viewer's override, so a NEUTRAL-pinned faction-10 NPC is still an NPC-vs-NPC *target*. Related: [[workflow_revert_audit]].
