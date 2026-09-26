---
name: dialog-set-engine-gaps
description: Two blocking content-engine gaps for dialog-set chains — NULL-dialog_id dialog_set_map rows are dropped at load, and the dialog_set_open trigger has no dispatch site.
metadata:
  type: project
---

# Dialog-set engine gaps (confirmed 2026-09-17)

## 1. `dialog_set_maps` rows with `dialog_id IS NULL` are dropped at load

`load_dialog_set_maps` (`crates/cell-catalog/src/cell/spawner/dialogs.rs:28`) builds
`DialogSetMapEntry { dialog_id: i32, interaction_flags: i64 }` and skips any row
whose `dialog_id` is NULL. Pinned by
`load_dialog_set_maps_drops_rows_with_null_dialog_id`
(`crates/cell-catalog/src/cell/spawner/tests/live_db_loaders.rs:269`).

Consequence: `add_dialog_set <dsm_id>` on such a row hits the
`"dialog_set_maps cache miss for add_dialog_set"` warn branch
(`executor/dialog.rs`) and silently does nothing.

Affected Castle rows: dsm **3062** (set 649, flags 16777216 — the one
`Castle.py` binds for mission 701), 3071 (654), 3073 (656, flags 16),
5828 (1571), 5829 (1572), 5846 (656), 5863 (656), 5748 (1358).

These NULL-dialog rows are the **topic/indicator carriers** — they exist to put
the quest icon on the NPC and hold the topic text, not to open a dialog. The
engine has no way to express "bind the indicator only" today.

Workarounds: bind a sibling dsm row in the same set that *has* a dialog_id
(e.g. 3061 → dialog 2574, same flags 16777216), or add an
`Option<i32>` dialog_id to `DialogSetMapEntry` plus a client-facing
interaction-only path.

## 2. `dialog_set_open` trigger never fires

`Trigger::OnDialogSetOpen` is authorable (`loader/trigger.rs:60`,
`event_type = 'dialog_set_open'`) and matches (`triggers/matching.rs:169`),
but **no `fire_*` site in `crates/services/src/cell/content/event_dispatch/`
ever constructs a `TriggerType::DialogSetOpen` event**. Zero dispatch sites
repo-wide. Any chain authored on it is dead.

Python scripts that subscribe to `dialog_set.open::<dsm_id>` (e.g. `Castle.py`
n65 on 3062) have no direct port. Substitute `interact_tag` on the NPC plus
step-status conditions.

## 3. `complete_objective` does NOT advance the step

`cell/missions/progression.rs:176-213`: when every non-optional objective of
the current step is complete, it calls `mission.complete()` — **completing the
whole mission**, not advancing to the next step. Only `advance_step` moves
between steps, and it is unconditional (it force-completes every active
non-completed objective of the current step first, `progression.rs:57-66`).

So on a multi-step mission, `complete_objective` on the last required
objective of a mid-mission step ends the mission early. Use `advance_step`
for step transitions and reserve `complete_objective` for optional/tracked
objectives or for the terminal step. See [[castle-cellblock-chains]] chain
1107's seed comment, which documents the same trap for mission 688.
