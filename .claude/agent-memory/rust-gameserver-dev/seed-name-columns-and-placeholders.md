---
name: seed-name-columns-and-placeholders
description: Which resources column holds a human name (mission_defn, not mission_label), which name tables are nearly empty, and why "UNUSED.*" is too broad a placeholder rule
metadata:
  type: project
---

Learned building the NameBook (`crates/names`, named telemetry NT-01, 2026-10-04):

- **`missions.mission_defn` is the mission's name; `mission_label` is its zone or
  group** (`Harset`, `General`) and 202 rows carry `NO MISSION LABEL`. The console
  authoring lookup already uses `mission_defn AS name`.
- **Nearly empty name columns in the seed:** `dialogs.name` (6 of 5,412, all sandbox
  rows), `dialog_sets.name` (2 of 1,178), `speakers.name` (134 of 602),
  `mission_objectives.display_log_text` (~800 of 4,037), `texts.text` (~13.5k of
  29k), `entity_templates.name` (all NULL; `template_name` is the designer name and
  `name_id` -> `texts` the player-facing one). `spawn_sets` has **zero** seed rows.
  The NameBook names a dialog from its first `dialog_set_maps.topic_text` instead.
- **`UNUSED.*` is not a safe placeholder rule:** mission 819 is really called
  "Unused Explosive". The stand-ins are `UNUSED`, `UNUSED.`, `UNUSED DIALOG*`,
  `UnusedDialog`, `UNUSED ERROR*`, `NO … NAME` / `NO … LABEL`, and the deletion
  markers: `DELETE`, `DELETE.`, `DELETED`, `DELETED.`, `DELETE THIS. UNUSED.`,
  `DELETED NOT USED`, `DELETEDONOTUSE`, `DELETE_ME`, `Delete me`, `DELETE <old name>`
  (mission 1421, mission_steps 3377/3770, texts 10991/14662/14931 among them).
  Match `DELETE` as a word, not a prefix ("Deleterious" is a name). The
  `UNUSED DIALOGUE` strings are in `dialog_screens`/`dialog_set_maps`, not `dialogs`.
  Grep the seed (`grep -rhoiE "'(DELETE|UNUSED|NO )[^']*'" db/resources`) before
  trusting any placeholder list, this one included.
- **`archetype` ints are `EArchetype` ordinals:** 0 Any … 6 Goauld, 7 Sholva,
  8 Jaffa. `base-session::archetype_name` had 7 = Jaffa and no 8 until NT-01.

**How to apply:** pick the column before trusting a spec's "x.name"; pin seed gaps
by id (`crates/names/src/namebook_gaps.txt`, bless with `NAMEBOOK_GAPS_BLESS=1`).
Related: [[seeds-and-content-chains-index]].
