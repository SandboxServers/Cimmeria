# Content-chain dispatch traps

Five facts about the content engine that are not visible from the seed
SQL and have each cost a real bug. Verified 2026-09-18 against the Castle
mission 701 port (packets CA01/CA03).

## 1. `display_dialog` needs an interact in the player's history

`executor/dialog.rs::display` resolves the wire `EntityId` of
`onDialogDisplay` (the client's portrait-lookup key) in this order:

1. `params["target_entity_id"]` — stamped **only** by `fire_interact_tag`
   / `fire_interact_template`, i.e. only on the chain fired directly off
   the click.
2. the player's `last_interaction_target` pin.
3. the monologue cache (every screen `speaker_id = 0`) → binds the player.
4. else `warn!` + **return without emitting a frame**.

So any *follow-up* chain — `dialog_choice`, minigame victory, deferred
drain — has only the pin. `fire_chain_by_id` builds `ResolvedActions`
with empty params by construction, so a victory chain has nothing else.

**The trap:** `interactions::dispatch::interact.rs` (the writer of the
pin) used to be the only one, and it runs solely in the `if !handled`
fall-through of `cell_methods/player/interaction/interact.rs`. A
chain-handled interact therefore left the pin stale and every follow-up
NPC-speaker dialog silently never opened; only monologues worked. Fixed
by pinning before the chain dispatch in `interact.rs` (mirrors python
`SGWPlayer.interact()`, which pins first). Guard:
`chain_handled_interact_pins_target_for_a_later_chain_dialog`.

Residual: a chain fired from a trigger that follows *no* interact
(`player_loaded`, region entry, timer) still cannot resolve a speaker.
Use a monologue dialog there or accept the warn.

## 2. `fire_dialog_choice` does not populate `archetype`

`content/event_dispatch/dialog.rs` sets only `dialog_id` and `button_id`
plus `populate_mission_context`. An `archetype` condition on a
`dialog_choice` chain can **never** match. Archetype splits must live on
the `interact_tag` chain and the per-archetype follow-ups are keyed by
*dialog id* instead (Cellblock chains 1018-1021, Castle 1204/1205).

`interact_tag`, `interact_template`, `player_loaded`, `item_use`,
`entity_death` and the cover dispatchers all DO populate it.

Also: `dialog_choice` matches on `dialog_id` only — `button_id` reaches
the dispatcher but there is no authorable condition for it, so one chain
per dialog id is the only available shape.

## 3. `enabled = false` is not a kill switch for a triggerless chain

`ChainEngine::get_chain_actions` (used by `fire_chain_by_id`, i.e. every
`on_victory_chains` entry) looks the chain up by id and returns its
actions **without checking `chain.enabled` and without evaluating any
condition**. Disabling a victory chain in the seed does nothing. Put the
gate on the launching chain; to disable a victory chain, remove it from
the launcher's `on_victory_chains` array.

A zero-trigger chain is still reachable: the loader gives it a synthetic
inert `OnCustomEvent { "__direct_invoke_<id>" }` so it lands in
`chains_by_trigger`.

## 4. Deferred actions survive death, not disconnect

`content_actions.delay_ms > 0` queues on `SpaceManager`, drained by
`deferred_content_action_tick`. The queue is scrubbed by
`destroy_entity` / `disconnect_entity`.

Player **death does not scrub it**: `resolve_respawn_target` matches a
respawner by world name, and a same-world respawn takes the in-place
branch (`cell_methods/player/combat/respawn.rs`) which never destroys the
entity. Only the cross-world respawn branch destroys. So don't assume
"death cancels the timer" — check which branch the world takes.

Paths that DO scrub (logout, cross-world hop) are all followed by a
`player_loaded`, so a `player_loaded` restore chain is the recovery.

**Testing a deferred action:** don't sleep. Push the resolved actions
through `execute_actions`, assert nothing was sent and the queue holds
N entries, call `deferred_content_action_tick` once to prove the delay is
honoured, then rewind each `pending.fire_at` into the past and tick
again. `pending_content_actions` and `PendingContentAction`'s fields are
`pub(crate)`. Pattern lives in `executor/tests/deferred.rs`.

## 5. Button-less dialogs still raise `dialog_choice`

Plenty of shipped dialogs have zero `dialog_screen_buttons` rows
(Cellblock 2300/5020/5021, Castle 2574/2575/5862) yet shipped chains
trigger on their `dialog_choice`. The client evidently sends
`dialogButtonChoice` on a button-less dialog's close. This is an
inference from shipped precedent, **not** a client observation — worth a
UAT confirmation before hanging a mission tail off one.

Note `handle_dialog_button_choice` gates on `open_dialog_id` (#479), so a
unit test driving it must set that pin first.

## Seeing it in a test

`load_single_chain_for_test` returns only the FIRST expansion when a
chain has multiple trigger rows — use `load_chain_expansions_for_test`
for OR-semantics chains or the 2nd+ trigger silently goes unasserted.
