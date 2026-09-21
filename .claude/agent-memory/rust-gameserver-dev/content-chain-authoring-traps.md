# Content-chain authoring traps (content engine, `db/resources/Content/Seed/*.sql`)

Found while authoring Castle missions 702/703/704 (packets CA06/CA07). All
verified against source, not inferred.

## `display_dialog` silently drops NPC dialogs on non-interact triggers

`executor/dialog.rs::display` needs an NPC entity id to bind the client's
portrait lookup. Resolution order:

1. the chain's `target_entity_id` param — **only `interact_tag` and
   `interact_template` stamp it** (`event_dispatch/interaction.rs`),
2. the player's `last_interaction_target` pin (set in `handle_interact`),
3. a **monologue fallback** for dialogs whose *every* screen has an explicit
   `speaker_id = 0` (`spawner/dialogs.rs::load_monologue_dialog_ids`),
4. otherwise `warn!` and **return** — nothing reaches the client.

So a `display_dialog` on an `enter_region`, `player_loaded`, `entity_dead_tag`
or minigame-victory chain works **only** if the dialog is a monologue. Check
`dialog_screens.speaker_id` for every screen of the dialog before authoring
one on a non-interact trigger. The failure is silent: the chain resolves, the
step advances, the voice line just never plays.

## `set_interaction_type` is global on the entity, not per player

The arm mutates `CellEntity::interaction_type_flags` and fans the value to
every witness (`executor/world/mod.rs`). Also stated at
`docs/content/interaction-flags.md:190`. Fine in an instanced zone; in a
persistent shared world one player's clear un-clicks the actor for everyone
still on that step. Mitigations that cost nothing:

- a `player_loaded <World>` restore chain per active step (the usual one), **and**
- a region **re-entry repair** chain with the same step gate, so the cure is
  walking out and back in rather than a relog. `|` is idempotent.

The genuinely per-player alternative is `add_dialog_set` (writes the player's
`available_interactions`). `set_follow_target` has the same shared-entity
problem with no alternative at all — one `follow_target_id` field per NPC.

### The zero-baseline rule — never clear an entity's LAST interaction bit

Established on Castle 706/708 (PR #668) after both the TVE and Copilot reviews
hit it independently. **`entity_templates.interaction_type` IS the spawn-time
value of the runtime `interaction_type_flags` bitfield**
(`space_manager/spawn.rs:127`) — they are the same number, not two systems. So:

- Clearing a cue on an entity whose template column is `0` drops it to flags
  `0`. `EInteractionNotificationType` drives the client's right-click cursor and
  context menu (`entity/src/interaction_flags.rs:1-9`), so the entity becomes
  unclickable — zone-wide, for every witness, with no recovery but a relog into
  a `player_loaded` restore chain.
- Clearing a cue on an entity with a non-zero baseline is safe: it reverts to
  the baseline. Castle's DHD (template 162, `interaction_type = 16` =
  `INT_Dhd`) is the worked example — clearing `INT_MinigameLivewire` leaves 16.

**Rule: clear a mission cue only when the target template's `interaction_type`
is non-zero.** One column lookup, no judgement call. Setting a bit is always
safe (a bystander sees an extra cue); clearing is the dangerous direction.

Nothing server-side saves you — `handle_interact` reads the template's
`NpcInteractionType` and `available_interactions`, **never**
`interaction_type_flags` (`interactions/dispatch/interact.rs:93-131`), and
`fire_interact_tag` runs on any interact with a tagged entity
(`cell_methods/player/interaction/interact.rs:218-238`). The break is entirely
client-side, so **no chain-replay test can observe it** — it has to be caught
by review, or by a two-player UAT. Chain-replay (TESTING.md type 6) is
single-player by construction.

Known live instance of the bug: Cellblock chains 1053/1054 clear `!`-class bit
8388608 off `Preparation_ColMarsh`, spawn 7 on **template 10** — the same
template as `Castle_ColMarsh` (`spawnlist.sql:7` vs `:181`), baseline `0`.
Restore chain 1063 repaints only on login.

## Verify respawn configuration before relying on `entity_dead_tag`

`cell/spawner/npcs.rs` resolves respawn duration with
`COALESCE(s.respawn_secs, t.respawn_secs)`. Both `spawnlist.sql` and
`entity_templates.sql` now define the column, so inspect the relevant rows
before treating an `entity_dead_tag` mission as repeatable. A NULL duration
leaves that spawned NPC without a scheduled respawn; the DB requires explicit
values to be at least 3 seconds.

Related: a damage-over-time killing blow fires no death event at all —
`cell/effects/pulsing/tick.rs::fire_pulse` has no alive→dead detection and
`mark_npc_dead` is only reachable from `abilities/damage_apply` and
`abilities/death.rs`.

## `set_follow_target` shapes

- `{"use_player": true}` — follow the chain's triggering player. The only way
  to follow a player at all (players carry no spawnlist tag). Guarded: it
  refuses and warns when the triggering entity is not a player.
- `{}` (no `target_tag`, no `use_player`) — **clear**: `follow_target_id =
  None`, `AiState::Idle`, `nav_path.clear()`.
- Re-arm the follow on `player_loaded`: a relog destroys the player entity
  and releases its id for reuse, so the stored id is not a safe handle.
- Follow is preemptable into Fighting and `npc_ai_leash` ends at Idle, never
  back to Follow — one stray hit ends an escort until a chain re-fires.
- Default `move_speed` is 0.6 units/tick = 6.0 u/s vs a player's 8.125 u/s,
  so a follower on the default never catches up. Set it on the template.

## Victory chains evaluate no conditions

`start_minigame`'s `on_victory_chains` are fired by id with
`ResolvedActions::default()`. A `content_conditions` row on a victory chain
is a silent no-op that *reads* like a guard. Put the step gate on the
launching `interact_tag` chain; the victory chain's own `advance_step` is
what shuts it.

## `increment_counter` / `reset_counter` read the name from `target_key`

`loader/action.rs`'s arms do `row.target_key.as_deref()?` for `counter_name`
and take only `amount` from `params`. Putting `counter_name` in the `params`
JSON makes `convert_action` return `None`, the row is **dropped with a
`warn!`**, and the chain still loads — with zero actions. Symptom in a
chain-replay test: the trigger resolves fine, the `.expect` on the loaded
chain passes, and `resolved.actions.len()` is 0.

Worth checking per verb before writing a seed row: several arms mix
`target_id` / `target_key` / `params` in non-obvious ways.

Corollary: a live-DB replay test that reports `ok` with no `DATABASE_URL`
**self-skipped** via `require_db_or_skip!` and proved nothing. Always
confirm through `live-db-test.sh` (or nextest's run/skip counts) before
believing a chain-replay guard. Found on CA10, where three replay tests
read green in the no-DB run and all three failed on the first live run.

## Chain-replay test trap: the label-signature assert masks later asserts

The common pattern maps each action to a short label and `assert_eq!`s the
vector. Any unexpected action maps to `"OTHER"`, so that assert trips
first — which makes a later "must NOT contain X" or "exactly one grant"
assertion unreachable. Put invariant assertions **before** the signature
check.

## Region entry is client-hinted and never re-evaluated at login

`enter_region` is raised only from `triggerClientHintedGenericRegion`. A
player who logs out inside the volume logs back in with no advance. There is
no `InRegion` condition to gate a `player_loaded` advance on, so a
region-advanced step has no server-side recovery path.

## The interact-tag linter checks two different things

`crates/content-engine/tests/interact_tag_linter.rs` has
`every_interact_tag_chain_has_set_interaction_type` (per-file, tag-based,
allowlistable) **and** `every_chain_region_key_matches_a_seeded_point_set`,
which requires each `enter_region` key to byte-match a `point_sets.name` row
in `db/resources/Events/Seed/point_sets.sql`. The second one fails for any
chain authored against a region another packet still owns.

## Never trust a prose doc for WHICH region a chain should bind

`docs/content/mission-chains.md` labels `Castle_Cellblock.Region9` "the mess hall" in
four places. It is not: Region9 is a ~15x17 unit box at the topside ring pad, and the
Mess Hall is **Region3**, which is the box that actually contains both `MessHall_Guard*`
spawns. Binding a bark to the doc's region would have put a line about "that table" on
the ring pad, 70 units and one floor away from the table.

**How to identify a region for real, in this order:**
1. `db/resources/Events/Seed/point_sets.sql` gives `set_id` + `flags` (1 =
   client-hinted, i.e. the edge fires at all); `point_set_points.sql` gives the corners -
   a BoundingBox is 4 rows, and min/max over x/y/z is the volume.
2. Check which `spawnlist.sql` rows fall inside it. The room is the one holding its NPCs.
3. Decisive: the original `deprecated/python/cell/spaces/<Space>.py` fires
   `onSystemCommunication(11, <textId>, '', [])` on ENTERING each named area, and that
   `textId` resolves in `texts.sql` to `string_<Zone>_Discovery_DisplayName_<Room>`.
   That is the 2009 build literally naming the room.

Also check enter-vs-exit against the Python: Cimmeria's chain 1073 binds Region9's ENTER
edge while `MessHall.py` bound its EXIT edge (`if args['entering']: pass / else: ...`).
Undocumented divergence, so do not assume a shipped chain's edge matches the reference.
