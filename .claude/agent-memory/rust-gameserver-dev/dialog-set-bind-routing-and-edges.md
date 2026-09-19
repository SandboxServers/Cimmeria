# Dialog-set binds: click routing, NULL-dialog rows, and the edge-trigger trap

Learned authoring the Harset H20/H22 talk chains (2026-09-19). None of this is
visible from the seed SQL, and three of the four cost a real defect.

## `add_dialog_set` `target_id` is a `dialog_set_map_id`, NOT a `dialog_set_id`

The loader field is literally named `dialog_set_id`
(`content-engine/src/loader/action.rs`) and the executor looks it up in
`space_mgr.dialog_set_maps`, which
`cell/spawner/dialogs.rs::load_dialog_set_maps` keys by `dialog_set_map_id`.
Passing a real `dialog_sets.dialog_set_id` is a silent cache miss: the chain
resolves, the action runs, `warn!("dialog_set_maps cache miss")`, no icon.

## NULL `dialog_id` rows are KEPT since Castle packet CA02

Pre-CA02 `load_dialog_set_maps` dropped them. Now they load as
`DialogSetMapEntry { dialog_id: None }` so a bind can raise an indicator bit
with no dialog behind it. Any comment or test asserting "a NULL dialog_id is
dropped and the bind is a silent cache miss" is **stale** — check the loader
before repeating it.

Consequence for the click path: `interactions/dispatch/interact.rs` uses
`entries.iter().find_map(|&(_, dialog_id, _)| dialog_id)` — the first bound
entry that HAS a dialog, not `entries.first()`. Interaction-only rows are
stepped over; if no bound entry has a dialog the click does nothing at all.

**Prefer an interaction-only row whenever the dialog is supplied by an
`interact_tag` chain.** A bind fans to EVERY entity of the template in the
player's AoI (`send_interaction_update_if_visible`), so on a template with two
spawns that must open *different* dialogs, naming one of them on the dsm row
means the wrong NPC can open it whenever the `interact_tag` chains stop
matching. NULL turns that into a dead click instead of a wrong outcome.

## `player_loaded` is an EDGE — pair it with a level trigger

A `player_loaded` chain gated on mission/step state only fires on login, gate
travel and the cross-world hop. If the gated state becomes true **while the
player is already standing in that world** (e.g. mission N+1's offer unlocks
when mission N is turned in to an NPC in the same world), the icon never
appears until a world round-trip. Castle playtest finding H9, same shape.

Fix: a sibling chain on `mission_completed '<N>'`.
`fire_mission_completed` (`content/event_dispatch/mission.rs`) runs AFTER
`complete_mission_direct` flips the status and populates world, archetype and
the full mission context, so the partner chain can carry the *identical*
condition set including `mission_status <N> eq completed`.

Use two chain ids, not one chain with two trigger rows:
`load_single_chain_for_test` returns only the first expansion, so a test
written the obvious way guards one trigger and silently ignores the other.

## Route A vs Route B decides whether a later `display_dialog` can work

`fire_interact_tag` runs BEFORE `interactions::handle_interact` and
short-circuits it when any chain matched. `handle_interact` is the only write
site repo-wide for `last_interaction_target`, and a `display_dialog` on a
`dialog_choice` / `dialog_open` trigger has no `target_entity_id` of its own —
the pin is the only thing it can resolve its NPC through.

So an offer that needs a follow-up conversation must have **no**
`interact_tag` chain in the offer state: the bind alone opens it (Route B) and
pins on the way. Guard it with a test asserting the interact resolves NOTHING
engine-wide, not with a test asserting a presence.

See also [[content-chain-condition-context-gaps]],
[[dialog-set-bind-carries-no-dialog-id]], [[content-chain-dispatch-traps]].
