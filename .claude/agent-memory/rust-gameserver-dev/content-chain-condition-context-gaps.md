# Content-chain authoring: what each dispatcher actually puts in the context

Read before authoring any `content_*` seed rows. Every item here cost real
investigation during the Castle 706/708 packet (2026-09-18) and none of it is
visible from the seed SQL.

## `archetype` is NOT populated on dialog chains — `neq` fails OPEN

`crates/services/src/cell/content/event_dispatch/dialog.rs` (`fire_dialog_open`
/ `fire_dialog_choice`) sets `dialog_id`, `button_id` and `populate_mission_context`
— and **nothing else**. No `archetype`.

`Condition::Archetype` reads a missing key as `-1`
(`crates/content-engine/src/conditions.rs`, `.unwrap_or(-1)`), so on a
`dialog_open` / `dialog_choice` chain:

- `archetype eq 8` is permanently **false** → the chain is dead.
- `archetype neq 8` is permanently **true** → a condition that reads like a
  guard and silently **fails open**. Worse than writing no condition at all.

Dispatchers that DO populate `archetype`: `interaction.rs` (both
`fire_interact_tag` and `fire_interact_template`), `lifecycle.rs`
(`fire_player_loaded`, `fire_entity_death` — from the KILLER),
`region.rs`, `inventory.rs`, `cover.rs`, `mission.rs`
(`fire_mission_accepted` / `fire_mission_completed`), `stargate.rs`
(`fire_stargate_dialed` / `fire_stargate_crossed`, added by CA10).

**Pattern:** put the archetype gate on the chain that DISPLAYS the dialog, and
let dialog id carry the branch on the choice chain. That is
server-authoritative because `handle_dialog_button_choice`
(`cell_methods/player/interaction/dialog.rs`, the #479 gate) rejects a
`dialogButtonChoice` unless `open_dialog_id` matches — the server must have
displayed it.

**Caveat that makes the gate real:** `initialResponse`
(`cell/interactions/dispatch/initial_response.rs`) scans the player's WHOLE
`available_interactions` map for a client-supplied `dialog_set_map_id`,
unscoped by template — so if you `add_dialog_set` both factions' rows to a
player, the client can ask for the other faction's dialog and the #479 gate
will happily pin it. `available_interactions` is written only by
`add_dialog_set` (`executor/dialog.rs`) and one GM console command. Don't bind
faction-specific rows unconditionally.

## Zero-button dialogs DO fire `dialog_choice`, with `button_id = -1`

Verified against the client (CEGUI + Lua, uncooked at
`…\SGWGame\Content\UI\Core\Dialog\`) during the 706/708 packet:

- `Dialog.lua` wires `selectActiveDialogChoice` only to buttons in
  `DialogMod.dialogButtonMap`, so a zero-button dialog has no click path.
- Done / Decline / the window X route to `onDialogDoneClicked` →
  `discardAvailableDialog` → native `FUN_00ad86c0` → `FUN_00d249c0`, which
  counts every screen's buttons and, **iff the total is zero**, sends
  `Event_NetOut_DialogButtonChoice` with `ButtonId = 0xFFFFFFFF` (-1).
- A dialog that HAS buttons sends **nothing** on close; it fires only on an
  Accept/Generic click.

Consequences:

- Keying a chain on `dialog_choice` for a button-less dialog is correct and
  precedented (`Castle.py` did it for 2574/2575; shipped chains 1020/1021 do it
  for 2300/5020).
- **Adding a button to a dialog that a chain keys on silently kills that
  chain.** Check `dialog_screen_buttons.sql` by `screen_id` (the THIRD column),
  not by `dialog_id` — a naive grep on the dialog id matches button ids and
  lies.
- On the click path, `button_id` is the 1-based index within the current
  screen's button list (`DialogSetup.lua` `setID(i)`), not the DB `button_id`.
  Nothing in the matcher reads it anyway (`triggers/matching.rs` keys
  `OnDialogChoice` on `dialog_id` only).

## `advance_step` vs `complete_objective` (`cell/missions/progression.rs`)

- `advance_step` force-completes the leaving step's objectives by calling
  `MissionInstance::complete_objective` **directly**, bypassing the
  module-level `all_required_complete` check. It can never auto-complete a
  mission. Safe to use as "close this step".
- `complete_objective` DOES auto-complete when every `!optional` objective of
  the current step is done. **CORRECTED 2026-09-19 (H50): the status byte was
  never wrong.** `onMissionUpdate` carries the two-value `STATUS_*` enum
  (0 active / 1 completed), not the four-value `MISSION_*` enum, and all five
  emitters agree — `accept_mission` 0, `abandon_mission` 1, `serialize_resend`
  0, `complete_mission_direct` 1, auto-complete 1. `MISSION_ACTIVE` and
  `STATUS_COMPLETED` are both `1`, so the old source named the wrong constant
  while the wire was right; the name is now fixed. Do NOT "correct" it to
  `MISSION_COMPLETED` (2) — that would break the client. `MISSION_*` is
  server-side only (`MissionInstance.status`, `sgw_mission.status`), which is
  the field `CellToBaseMsg::MissionUpdate` carries.
- Auto-completing through `complete_objective` is now safe for persistence too:
  as of H50 that branch fires `mission_completed` and its `MissionUpdate`
  carries the post-transition status and bumped `repeats`. Prefer
  `complete_mission` → `complete_mission_direct` when you want every objective
  ticked on the wire, since that path emits onObjectiveUpdate(COMPLETED) per
  objective and onStepUpdate first.
- Ordering inside one action list: `complete_objective` must come BEFORE
  `advance_step`. Reversed, the objective is looked up in the NEW step's list,
  misses, and early-returns — the client never sees the tick.
- **Vacuous-truth footgun:** `all_required_complete` is `.all()` over a filtered
  iterator, so a step whose objectives are ALL optional completes the mission on
  the first `complete_objective`. **37 seeded steps have that shape** (333, 491,
  553, 875, 3429, 4490, 4612, 4962, …) — counted 2026-09-19 over
  `db/resources/Missions/Seed/mission_objectives.sql`, so "every step has ≥1
  required objective" is false, not just unasserted. Since H50 this is reachable
  on the relog path too (hydration carries the real `optional`), and
  `complete_objective` emits a `warn!` when it auto-completes with
  `required_count == 0`. Pin any new all-optional step with a test that asserts
  `mission.status == MISSION_ACTIVE` after the chain runs.

## `delay_ms > 0` QUEUES the action — it does not run it

`executor/mod.rs::execute_actions` forks on `action_delays[i] > 0` into
`space_mgr.schedule_content_action`. Any chain whose guard is "the same action
list advances the step out from under a repeat trigger" is broken by a nonzero
`delay_ms` on the `advance_step` row: the gate stays satisfied across the delay
window. Assert `delay_ms == 0` per chain in the replay test when the step gate
is the guard.

## Loader shapes that surprise

- **Multi-trigger OR:** N `content_triggers` rows on one chain materialize N
  in-memory `Chain`s sharing id/conditions/actions
  (`content-engine/src/loader/mod.rs`). `load_single_chain_for_test` returns
  only the FIRST — use `load_chain_expansions_for_test` or the test silently
  stops guarding trigger rows 2..N.
- **Trigger-less chains are legal:** zero trigger rows gets a synthetic
  `OnCustomEvent { "__direct_invoke_<id>" }`, so the chain loads and is inert
  until fired by id. That is how `on_victory_chains` targets work.
- **Minigame victory chains skip condition evaluation entirely**
  (`event_dispatch/mod.rs` fires them with `ResolvedActions::default()`). Any
  gate must live on the LAUNCHING chain. A condition row on a victory chain is
  dead weight that reads like a guard.
- `content_triggers.scope`, `content_chains.scope_type` / `scope_id` and
  `once` are read into the loader row structs and then **never consulted**.
  They are documentation. Only conditions gate a chain.
- All matching chains' conditions are evaluated against ONE pre-action context
  snapshot, then the action lists are concatenated and run in order
  (`chain/mod.rs::resolve_event`). So chain B's `step_status X eq active` still
  passes when chain A advances past X earlier in the same batch. This is what
  makes "one chain does the work, a sibling chain paints the cue" safe — and
  what makes the Cellblock counter chains use `gte target-1`.

## `set_interaction_type` is ZONE-WIDE — clearing can break other players

`executor/world/mod.rs::set_interaction_type` mutates the shared
`CellEntity.interaction_type_flags` and broadcasts to every witness. There is no
per-player interaction state short of a dialog-set bind (design gate GCA1).

Setting a bit is harmless to bystanders (an extra cue). **Clearing one can strip
an entity's only clickable bit out from under another player mid-step** — check
the template's `interaction_type` column in `entity_templates.sql` first. Many
mission props ship `interaction_type = 0` (e.g. template 147, the Castle Access
Panel), so the mission-set bit is the ONLY thing making them clickable, and a
clear renders them scenery zone-wide. In a shared (non-instanced) world, prefer
"set, never clear" for props; clear only NPC cue bits, where a stale `!` is the
worse outcome.

Related: `accept_mission` returns early when the offer guard refuses
(`executor/mission.rs`) and **skips `fire_mission_accepted`**, so any
"clear the bit, then accept the mission whose accept-chain re-sets it" ordering
leaves the bit cleared forever on that path.

See also [[chain-replay-executor-guards]], [[stat-with-no-consumer-trap]].
