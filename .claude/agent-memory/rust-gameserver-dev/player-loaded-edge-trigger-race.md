# `player_loaded` + a state gate = a chain that never fires for the player who just opened the gate

A chain triggered on `player_loaded <World>` and gated on a mission/step
condition only fires when the player CROSSES into that world. If the
gating state flips while the player is already standing inside it, the
chain never runs — there is no re-evaluation on state change.

Worked example (Harset H41, chain 6118): the mission-742 offer bound
Anat's topic on `player_loaded Harset_CmdCenter` gated
`mission_status 1200 eq completed`. Mission 1200 *completes at Anat*, and
Anat stands in world 68 — the Command Center. So the gate opened at the
exact moment the player was already inside, and the offer would not have
appeared until they walked out and back in.

This is the general form of the 2026-09-18 Castle playtest finding H9
(the `enter_region` version). Anything edge-keyed has it: `enter_region`,
`player_loaded`, cover enter.

**Fix pattern: add a second trigger row on the state-change event
itself.** N `content_triggers` rows on one chain materialize N in-memory
`Chain`s sharing id, conditions and actions, so no condition row changes.

`mission_completed` / `mission_accepted` are safe partners for a
mission-state gate: `fire_mission_completed` and `fire_mission_accepted`
(`cell/content/event_dispatch/mission.rs`) both call
`populate_world_context` (which sets `ctx.world_id`, so a `world`
condition still works), `populate_mission_context`, and stamp
`archetype` — and they run AFTER the instance mutation, so
`mission_status <id> eq completed` is already true.

Two consequences:

- The replay guard MUST use `load_chain_expansions_for_test`;
  `load_single_chain_for_test` returns only the first expansion and
  silently drops the new trigger from the test.
- A second bind can accumulate across events (completion binds, later
  door hop binds again). Benign when the unbind is
  `remove_dialog_set`, which is `retain`-based and removes every copy
  (`executor/dialog/mod.rs:254`), and `available_interactions[t].first()`
  returns the same dsm either way.

**How to spot it in review:** for each `player_loaded` chain, ask where
the gating state is set. If the setter runs in the same world the chain
keys on, it is this bug.

See also [[content-chain-dispatch-traps]],
[[content-chain-condition-context-gaps]].
