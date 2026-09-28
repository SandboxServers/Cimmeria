---
name: reference-map-arrival-points
description: "Surveyed 2026-09-27: only Agnos, Beta_Site_Evo_1 and Tollana cooked maps have a PlayerStart; no map has SGWStargate/SGWTeleporter actors; seed arrival points (gate rows, respawners) are the real source. Read before placing arrivals or trusting a gate row."
metadata:
  type: reference
---

Survey of every `entities/spaces.xml` world's cooked client map (2026-09-27, `.gotolocation <world>` work):

- **PlayerStart is rare.** One each in Agnos (20.640, 34.939, 15.890 — the Ancient platform deck), Beta_Site_Evo_1 (635.761, -60.134, 388.448, 18 m from gate 6) and Tollana (-22.080, 5.920, 193.280, a Goa'uld camp 777 m from the gate). Every other world has none — Castle included (matches CA05). The original server placed players from its own tables, not map actors.
- **No map has `SGWStargate` or `SGWTeleporter` actors.** Stargates are `PrefabInstance`s of `GLB-Global.GLB-Stargate_Prefab`; teleporters are plain UE3 `Teleporter`, which `cimmeria_upk::extract_actors` skips. So only `PlayerStart` ever contributes to the occluder's `map_entry_actors`.
- **Conversion confirmed:** BigWorld = (UE.y, UE.z, UE.x) / 100 (`ue3_to_bw`); all 11 gated worlds' prefab positions match their `stargates.sql` rows within ~0-7 m horizontally, the row ~1.6 m above the prefab base.
- **Bad gate rows found:** Agnos gate 15 shipped all zeros (fixed: arrival pinned to its PlayerStart); Menfa_Light gate 22 is ~192 m below `menfa_light.nav` (unfixed, needs an in-client look); SGC_W1 gate 27 is ~17 m under the floor (masked by the new-character start). Most worlds are navmesh `advisory`, so a bad gate row is never caught at runtime.
- Tollana_Curia's cooked map is a stub (one flat 100 m terrain tile, no room).
