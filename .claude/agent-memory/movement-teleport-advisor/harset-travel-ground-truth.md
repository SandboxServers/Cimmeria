---
name: harset-travel-ground-truth
description: Ground truth from the deprecated Python for every Harset world transition, and the four places the V5 Harset spec diverges from it
metadata:
  type: project
---

Recovered 2026-09-17 during the read-only evidence pass for a Harset zone restoration campaign (modelled on `docs/analysis/castle-cellblock-rebuild/`). The deprecated Python is the authority the spec lacked.

**Only two Harset space scripts ever existed:** `deprecated/python/cell/spaces/Harset.py` (70 lines) and `Harset_CmdCenter.py` (30 lines), with Atrea sources at `deprecated/data-scripts/scripts/spaces/Harset{,_CmdCenter}.script`. There is **no** `Harset_Market.py` or `Harset_StorageRm.py`, and `worlds.sql` sets `has_script = false` for worlds 69 and 70 (true for 57 and 68). So Market and StorageRm transitions are **new content authoring, not restoration** — the original server never had them.

**The transition coordinates (`Act_Teleport` → `moveTo(x,y,z, worldName=…)`):**

| Direction | Trigger region (`client_hinted_region::`) | Destination world | Destination coordinate | Source |
|---|---|---|---|---|
| Harset → CmdCenter | `Harset.CommandCenterTransition` | `Harset_CmdCenter` | `(0, 0.355, -20)` | `Harset.py:15,57` |
| CmdCenter → Harset | `Harset_CmdCenter.HarsetTransition` | `Harset` | `(0, -67.600, -231)` | `Harset_CmdCenter.py:15,22` |

**Where the V5 spec is wrong — four corrections:**

1. **CmdCenter arrival is `-20` on Z, not `-25`.** `Harset.py:15` is `str2vec('0,0.355,-20')`.
2. **The return target is not "unresolved".** It is `(0, -67.600, -231)`, `Harset_CmdCenter.py:15`.
3. **The ring interact tags are not what the spec describes.** `Harset.py` subscribes `entity.interact.tag::` on exactly `HarsetRingLeftBottom`(→region 4), `HarsetRingRightBottom`(5), `HarsetRingLeft`(6), `HarsetRingLeftTop`(7), `HarsetRingRight`(8). These byte-match `spawnlist.tag` for spawns 4/127/128/129/130. The spec's `"HarsetinRingRightRegion"` typo lives only in `ring_transport_regions.tag` (region 8) — a column that is **never copied into `RingTransporter`, never wire-encoded and never string-matched**. Cosmetic; routing is by `point_set_id`.
4. **Trigger point set 1001 has `tag = NULL` in `generic_regions`.** The name `Harset.Stargate` comes from `point_sets`, not `generic_regions`. And `generic_regions` has only 28 rows total (ids 1–245, 1001–1011) — there is **no** row for 2078/2079/2052–2056. `point_sets` is the live table; `generic_regions.handler` is dead (`spawner/regions.rs` reads `point_sets WHERE type='AreaSet'`).

**The trigger regions are client-hinted.** `GenericRegion.py:174-183` shows the original dispatch order: `REGION_FLAG_Stargate` (bit 2) short-circuits to `entity.stargatePassed()` *before* `region.callback`, so the `handler` string `cell.SGWPlayer.stargateRegionTriggered` was never actually invoked for stargate-flagged regions. Flags are `ClientHinted=1, Stargate=2, PvPZone=4, NonPvPZone=8` (`Atrea/enums.py:1202-1205`); point set 1001 carries `flags=3` = ClientHinted|Stargate. In Cimmeria today only `REGION_FLAG_CLIENT_HINTED = 1` is defined and **bit 2 is never tested**, which is why walking through a gate does nothing.

**Design detail worth preserving:** both authored arrival coordinates sit deliberately *outside* the opposing trigger box, so there is no ping-pong. PS 2078 spans Z −243.52…−238.41 and the return arrival is Z −231; PS 2079 spans Z −37.58…−29.48 and the outbound arrival is Z −20. Roughly 8 units of clearance each way. Any re-pin of these coordinates must preserve that margin.

**The DHD "interaction 6891" in the spec is a `dialog_set_maps` row, not an `interactions` row:** `dialog_set_map_id 6891, dialog_set_id 1916, topic_text 'Dial Home Device (DHD)'`. The DHD entity is `spawnlist` spawn 37, tag `Harset_DHD`, template 1.

See [[arrival-coordinate-offnavmesh]] — the recovered coordinates above are *authored* values and none of them has been checked against `data/spaces/harset.nav`; the gate arrival demonstrably fails.
