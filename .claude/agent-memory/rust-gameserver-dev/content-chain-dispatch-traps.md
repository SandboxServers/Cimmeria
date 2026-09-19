# Content-chain dispatch traps

Ten facts about the content engine that are not visible from the seed
SQL and have each cost a real bug. Verified 2026-09-18 against the Castle
mission 701 port (packets CA01/CA03); 6-8 added 2026-09-19 from the
Harset H30/H31 packet.

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

## 6. One dialog-carrying bind per template slot, and the cross-mission collision

`interactions::dispatch::interact.rs` looks a bind up as
`available_interactions[template_id]` then `find_map`s for the first
entry whose `dialog_id` is non-NULL (interaction-only binds are skipped
so a flag-only indicator cannot swallow the click). **Not `.first()`** —
an older comment said so and is wrong. A second *dialog-carrying* bind on
one slot is permanently unreachable.

The collision that actually happens is **cross-mission**, not within one
mission: two different missions' restore chains binding the same NPC's
template slot, each gated only on its own step. Harset 2026-09-19: chain
6502 (mission 1360, dsm 5356) and chain 6526 (mission 1361, dsm 5253)
both bind Col. Marsh's slot 10. Within a mission the "one
`current_step_id`" argument makes step chains disjoint for free; across
missions nothing does.

Fix shape: give the lower-precedence bind chain the same disjointness
condition the interact chain already carries (`step_status <other
mission>/<step> neq active`), **plus** a hand-back chain, because…

## 7. The `player_loaded` form of the edge-trigger race

A step that becomes current while the player is already standing in the
destination world gets **no second `player_loaded`** — `fire_player_loaded`
runs from `service/base_messages/player_init`, i.e. once per world entry
(login or a `cross_world_teleport`, which destroys and rebuilds the cell
entity). So a restore chain alone can never paint an indicator for a
transition that happens in-place.

Rules that follow:

- **Same world:** bind in-chain *and* in the restore chain.
- **Across a world boundary:** do NOT bind in-chain (the bind dies with
  the cell entity); leave it to the destination world's restore chain.
- **Cross-mission:** this is where the shape actually bites, because
  "one mission has one current step" stops being an argument. Either add
  a `mission_completed '<id>'` **second trigger row** to the chain whose
  gate opens (`build_chains_from_rows` emits one Chain per trigger row,
  sharing conditions and actions), or add a separate
  `mission_completed`-triggered hand-back chain. `fire_mission_completed`
  (executor/mission.rs, gated on a real active→completed transition)
  populates world + mission + archetype context *after* the mutation and
  runs *after* the completing chain's own `remove_dialog_set`, so the
  slot ends the event holding exactly one bind.

**The sweep to run**, for every `player_loaded` chain in a packet: name
the chain that opens its gate, and ask whether it runs in the same world.
Same world → the handing chain must bind in-chain or carry a second
trigger row. Different world → the crossing covers it, and binding
in-chain would be a silent no-op. Harset H30/H31 had two misses out of
ten, both cross-mission, and one of them was on the guaranteed
first-visit path.

## 9. Negatives that omit the trigger key pass vacuously

`resolve_event` checks `chain.trigger.matches(event)` **before** it
evaluates a single condition, and the keyed triggers read their key out
of the event params: `OnInteractTag` → `entity_tag`, `OnDialogChoice` →
`dialog_id`, `OnMissionCompleted` / `OnMissionAccepted` → `mission_id`,
`OnItemUse` → `item_id`. A negative test whose context omits the key
resolves nothing for a reason unrelated to the gate it claims to test,
passes, and **keeps passing when the gate is deleted**.

Guard shape: assert each chain's trigger `matches()` the context its
negatives are perturbations of, and that the satisfying context really
resolves actions. `Trigger::matches` and `Chain.trigger` are both public.
A revert run also catches it: a vacuous negative stays green there.

## 10. Testing a chain that ships `enabled = false`

`resolve_event` filters on `chain.enabled` before anything else, so a
parked chain resolves nothing and its *logic* is untestable through the
normal path. `Chain`'s fields are all `pub`: load it, set
`chain.enabled = true`, register it. Pair that with a separate test
asserting the shipped row really is disabled, so both facts are pinned —
inert today, correct when flipped.

Multi-trigger chains need `load_chain_expansions_for_test`;
`load_single_chain_for_test` returns only the FIRST expansion, so the
second trigger row silently goes unasserted.

This matters more than it sounds: `entity_templates.interaction_type = 0`
with `static_interaction_sets = '{}'` is the norm for dialog NPCs, so
with no bind the client never registers an interaction and the
`interact_tag` chain's right-click is **never sent**. A missing bind is a
dead mission, not a missing icon.

## 8. `entity_interactions` has no Rust consumer

`grep -rn entity_interactions crates/` returns nothing. The shipped 2009
table (static per-template NPC interactions gated by
`missions_not_accepted` etc., e.g. Anat's 742 offer) never reaches the
runtime. Do not reason about a collision between it and a per-player
bind — there is none.
