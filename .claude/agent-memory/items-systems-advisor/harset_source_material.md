---
name: harset-source-material
description: Where the original SGW design/content for Harset zone survives in this repo — one mission script, two space scripts, and narrative text in the deprecated monolithic SQL dump
metadata:
  type: reference
---

For any future Harset zone/mission/vendor work, these are the authoritative legacy sources found during the 2026-09-17 evidence pass:

- `deprecated/data-scripts/scripts/missions/Harset/GivingTheWallsEars.script` — the **only** surviving original Harset mission script (Atrea visual-script XML, mission id 742). Covers: Petbe dialog → equip Jaffa Disguise (item 2819) → plant 3 Scarab Listening Devices (item 2820, granted qty 3 at mission start node 52, removed 1-at-a-time via a `Counter_Int` gate to 3 at node 30) → talk to Anat again → give Scarab Listening Device Map (item 2864, granted node 40, removed node 44) to Nerus. This single file is what resolved the Scarab Listening Device (2820 vs 6818) duplicate — 6818 appears nowhere in it or anywhere else in `deprecated/`.
- `deprecated/data-scripts/scripts/spaces/Harset.script` and `Harset_CmdCenter.script`, plus their Python twins `deprecated/python/cell/spaces/Harset.py` and `Harset_CmdCenter.py` — space/zone-level scripts (not mission scripts), not yet read in depth this pass.
- `deprecated/db-monolithic-sql/db-deprecated/resources.sql` — the pre-split monolithic SQL dump. `dialog_screens` rows in here carry Harset narrative text not present in the current `db/resources/Dialogs/Seed/` split (e.g. the Romney's Files quest text at lines ~29305-29306, ~30476-30478 — "Bring me Romney's Files" / Zuritska interrogation subplot). Useful for recovering quest text for items/mission beats that exist in `items.sql` but have no live content-engine chain yet.
- Castle_CellBlock's `deprecated/python/cell/missions/Castle_CellBlock/ArmYourself.py` and the matching `.script` reference item 3730 (Frost's Letter) directly — this is Castle content, not Harset, but Frost's Letter is a mission-bag item (`container_sets={2}`, MISSION) that a Harset campaign would need to carry across the Castle→Harset zone hop; see `db/resources/Content/Seed/castle_cellblock_chains.sql:107` for the live `add_item` grant (Romney's Files 2698 has no equivalent live grant anywhere yet — narrative-only today).

Item-use ability/effect intent for Harset items (from legacy `items_event_sets` + `abilities`/`effects` seed, cross-referenced against this script) is written up in [[item_use_trigger_mechanism]].
