---
name: add-dialog-set-no-dedupe-first-wins
description: add_dialog_set appends to available_interactions without dedupe and handle_interact takes .first(), so two concurrent missions bound to one NPC template collide — the first bind wins and starves the second.
metadata:
  type: project
---

# `add_dialog_set` — no dedupe, first-bound-wins (confirmed 2026-09-18)

## Append, never dedupe

`executor/dialog.rs:146-150` —
`player.available_interactions.entry(slot).or_default().push((dsm_id, dialog_id, flags))`.
A bare `push`. Re-binding the same dsm on the same slot (e.g. an accept chain
plus a `player_loaded` restore chain that both bind it) appends a duplicate.

Benign-ish: `remove_dialog_set` (`:208`) uses
`entries.retain(|&(dsm_id,_,_)| dsm_id != dialog_set_id)`, which removes **all**
copies. So duplicates of the *same* dsm are self-healing on removal and don't
change which entry `.first()` picks.

## `.first()` wins over everything else

`cell/interactions/dispatch/interact.rs:95-111` resolves
`available_interactions[template].first()` and returns that dialog id, ahead of
the DHD check and the static `interaction_type` dispatch.

So **two different dsm ids on the same template slot is a real starvation bug**:
whichever mission bound first owns the NPC until its `remove_dialog_set` runs.
Ordering within one event is `resolve_event`'s priority-descending order
(`chain/mod.rs:102-105`, stable sort → ties fall back to loader insertion
order), so chain `priority` is the lever: give the mission that should win the
NPC the higher priority on its bind chain.

## Flag push is asymmetric

`send_interaction_update_if_visible` (`:356`) pushes
`base_flags | entry.interaction_flags` for **only the entry just added**, while
`remove_dialog_set` (`:216`) folds the OR of **all remaining** entries. So after
a second bind the client's icon reflects only the newest dsm's flags; after a
removal it reflects the union. Cosmetic, but it explains icon flicker when two
missions share a template.

## Dispatch order around it

`cell_methods/player/interaction/interact.rs:157-203`:
trainer → `fire_interact_tag` → `fire_interact_template` → `handle_interact`.
Each returns "handled" only when a chain actually matched, so an `interact_tag`
chain pre-empts the dialog-set path entirely for tagged props.
