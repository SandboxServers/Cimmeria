# HA-03a: Mission 1325 (Rin'la) and the Mala'c duel, provisional design

Status: PROVISIONAL, AI-authored under owner decision O18-10 (see
[owner-decisions-2026-10-18.md](owner-decisions-2026-10-18.md)). Docs only: no
seed, code, ID or client change is made by this note. Mission 1326 stays
dependent on 1325. Every ID below is "ID needed: <kind>" except candidates that
[id-allocation.md](../id-allocation.md) already lists (HA-03 / 1325 owns chains
6311-6330).

Labels used throughout:

- Provenance: RESTORED_SOURCE (text or data the original shipped),
  CIMMERIA_RECONSTRUCTION (a seed row Giza reconstructed),
  CIMMERIA_AUTHORING (new here).
- Certainty: FACT (read in the repo at tip 29f9d3fdb), INFERENCE (with
  confidence), DECISION (owner), PROVISIONAL (this design).

## 1. What the seeds already say

| Fact | Evidence | Label |
| --- | --- | --- |
| Mission 1325 exists with four steps, indexes 0-3, all `step_enabled = false`. | `db/resources/Missions/Seed/missions.sql:259`, `mission_steps.sql:954-960` | FACT |
| Step 3957 "Challenge Mala'c in the Market." | `mission_steps.sql:954` | FACT |
| Step 3958 "Move into position for Rin'la." | `mission_steps.sql:956` | FACT |
| Step 3959 "Commence Rin'la." | `mission_steps.sql:958` | FACT |
| Step 4036 "Return to Moh'katan" (id out of sequence, so a late addition). | `mission_steps.sql:960` | FACT |
| One objective per step (4547, 4548, 4549, 4649), all disabled, none optional, one type-1 task each (5537, 5538, 5539, 5684). | `mission_objectives.sql:1001-1007`, `mission_tasks.sql:1043-1049` | FACT |
| "Rin'la" is the name of the staff duel ritual, not a person. Moh'katan gives the mission; Mala'c is the opponent. | dialogs 4369, 4370 (below) | FACT |
| Mala'c is entity template 200: level 15, faction 10 (hostile), about 950 HP, speaker 957, respawn 300 s, components cloned from template 159. | `Entities/Seed/entity_templates.sql:640-662` | FACT |
| Zero content chains exist for 1325. 1326 offers 6331, 6333 and 6341 gate on `mission_status 1325 eq completed`. | `Content/Seed/harset_jaffa_chains.sql:13-19, 631-771` | FACT |
| No python script exists for 1325. | `deprecated/python/cell/missions/` has no Harset duel | FACT (high) |

Dialog set 1392 is seeded and carries the whole story (RESTORED_SOURCE text,
`Dialogs/Seed/dialog_set_maps.sql:4963-4969`, `dialog_screens.sql`):

| Map id | Dialog | Flags | Screen type | Content | Speaker |
| --- | --- | --- | --- | --- | --- |
| 5155 | 4368 | 0x08000000 | `DUIST_None` | Topic only, "Engage in Rin'la with Mala'c." | 945 |
| 5156 | 4369 | 0 | Default | Offer: "You must re-assert yourself ... Mala'c waits for you in the Marketplace. I expect you to acquit yourself honorably." | 945 (Moh'katan, INFERENCE high) |
| 5157 | 4370 | 0x10000000 | Default | Pre-duel: "We use only the traditional Rin'la staff and we do not move as we engage." | 957 (Mala'c) |
| 5158 | 4372 | 0x20000000 | Default | Turn-in: "Your station within our clan is again secured. You bleed, do you require medical attention?" | 945 |

Reading of the flag column (INFERENCE, medium): 0x08000000 is the
mission-available marker, 0 the plain topic, 0x10000000 the in-progress topic
and 0x20000000 the turn-in topic, matching the same three values in sets 310
and 1391. Nothing in the repo states the meaning; HA-03b should confirm in the
dialog-set code before binding.

Two consequences:

- The movement lock is RESTORED_SOURCE behaviour. Mala'c states it
  ("No movement?" "Correct ... we do not move as we engage"). The player is
  rooted and so is Mala'c.
- Set 1392 mixes Moh'katan and Mala'c lines, yet speaker labels are per dialog.
  So 4369 and 4372 belong on Moh'katan's bind and 4370 on Mala'c's bind, from
  the same set id. Whether the dialog-set code renders that cleanly is
  untested (INFERENCE, low); HA-03b checks it. No set may be split here
  (that would be "ID needed: dialog_set", a new ID).

## 2. Step graph (CIMMERIA_AUTHORING over RESTORED_SOURCE steps)

```text
not_active
  -- accept (Moh'katan, dialog 4369, after 1324 completed) -->
3957 Challenge Mala'c         [talk to Mala'c, dialog 4370]
  -- 4370 completes -->
3958 Move into position       [walk to the duel mark; movement lock starts]
  -- arrival at the mark -->
3959 Commence Rin'la          [duel: Mala'c aggressive; 30% health ends it]
  -- Mala'c yields -->
4036 Return to Moh'katan      [turn-in dialog 4372 -> CompleteMission]
  -- completed
```

Back edge: player death or relog/abandon during 3959 returns the mission to
3958, not 3957 (O18-10 as briefed). A back edge is the only non-obvious move;
see section 5.

Step roles are INFERENCE from the display text (confidence high for 3957,
3959 and 4036; medium for 3958, which could instead be a plain
"stand here" region or a Mala'c prompt). The turn-in objective 4649 follows
the repo rule that a turn-in objective is closed only by `complete_mission`
(see `executor/mission.rs:231`), while `advance_step` force-completes the
outgoing objective (`executor/mission.rs:262-355`) (FACT).

## 3. The duel mechanics

Everything below is the chain role plus the engine primitive it uses. Chain
ids are not assigned; roles map to "ID needed: chain" slots inside 6311-6330
(section 7).

### 3.1 Spawn and ownership

- `spawn_entity` (`actions.rs:112`, `executor/dispatch.rs:273`,
  `executor/spawn/mod.rs:55`) spawns template 200 with the tag `Rinla_Malac`
  into the acting player's own instance of world 69 (the private Market).
  It is tag-mandatory, one-shot and same-tag idempotent, and it also matches a
  corpse of the same tag (FACT, high). There is no `respawn_secs`.
- Consequence for shared worlds 57 and 68: nothing is mutated per player on
  shared actors. Moh'katan gets only per-player `add_dialog_set` binds, never
  `set_interaction_type` (CIMMERIA_AUTHORING, follows the standing hub rule).
- Spawn point: `ID needed: position (duel mark and Mala'c mark)`. The duel
  position stays a placeholder until HA-01a/HA-02 evidence lands
  (see [placements.md](../placements.md); Market respawner 22 and region 2121
  are the only Market anchors listed there).
- Whether world 69 is a per-player instance for this spawn path, and its
  teardown on abandon or relog, is read from the spawn code as "spawns into
  the acting player's instance" but is not exercised for the Market
  (INFERENCE, medium). HA-03b owns that check.

### 3.2 Aggression and the start

- Spawn neutral. `set_aggression` (`actions.rs:245`, `dispatch.rs:220`)
  writes an EMobAggressionLevel override and broadcasts
  `onAggressionOverrideUpdate`. The enum value for "neutral" is unresolved:
  the template comment says 3, the content doc says 0 is passive/neutral
  (FACT that they disagree). HA-03b must resolve the value before authoring a
  row; this note writes "neutral" and "hostile" only.
- Faction 10 is hostile, which is what lets player abilities target Mala'c.
  Right-click on an alive faction-10 NPC becomes auto-attack, so the challenge
  conversation (4370) cannot ride the normal interaction while he is hostile.
  Design: keep him at a non-attackable aggression level while 3957/3958 are
  active, so the dialog bind is clickable, and drive the start by chain.
  Whether the neutral override suppresses the right-click-to-attack route is
  INFERENCE (medium) and is the first lab check for the implementation.
- Start of the duel (step 3959 becomes active, role C-START):
  `set_aggression Rinla_Malac hostile` then `generate_threat` against the
  acting player so Mala'c engages without an unrelated pull. Both actions
  exist (`actions.rs:245`, action name `generate_threat` in
  `loader/action.rs`).

### 3.3 The 30% surrender (role C-YIELD)

- Trigger `entity_health_below`, event key `Rinla_Malac:30`. Rules: strict
  downward crossing, pct in 1..=99, attacker must be a player
  (`loader/trigger.rs:127-182`, `event_dispatch/lifecycle/mod.rs:271-488`)
  (FACT, high).
- Conditions: `step_status 1325 = 3959 eq active` (so a stray crossing in any
  other state does nothing) and `entity_tag_state Rinla_Malac eq alive`.
- Actions, in this order (state changes before mission changes, per the
  known timing trap):
  1. `set_npc_ai_state Rinla_Malac submit` (the inert surrender state,
     `actions.rs:320`).
  2. `npc_bark` for Mala'c's yield line (text ID needed, see section 8).
  3. `remove_effect` for the player's movement lock (executor arm missing,
     section 4).
  4. `advance_step 1325 4036`.
  5. `add_dialog_set` Moh'katan bind carrying 4372 as the turn-in topic.
- Mala'c stays on the field in `submit` until cleanup (section 5) so the
  duel visibly ends; 4372 is spoken by Moh'katan, not Mala'c.
- The lethal-hit edge: one hit can take Mala'c from above 30% straight to
  zero, and then the engine routes it to `entity_dead_tag`, not health-below
  (FACT, high). The fallback is role C-DEAD:
  `entity_dead_tag Rinla_Malac`, conditions `step_status 1325 = 3959 eq
  active`. Disjointness with C-YIELD (both can match on different events but
  must never match the same event) holds because they key on different
  triggers, and each carries the `step_status` gate so whichever fires first
  moves the step to 4036 and the other then fails its own condition on the
  next event. The player-kills-NPC dead trigger only fires when a player
  scored the kill (`lifecycle/mod.rs:189`), which is the only way this fight
  can end that way.
- Actions for C-DEAD: same as C-YIELD minus the `set_npc_ai_state` and bark
  (the corpse cannot yield), plus `despawn_entity Rinla_Malac` after the step
  move.

Order caveat (FACT, high): conditions are frozen when the event resolves and
every matching chain fires, so C-YIELD and C-DEAD must never share a trigger
key. They do not.

## 4. Effect inventory (input to HA-03b)

Nothing in `db/resources/Effects` is seeded for a duel: no movement lock, no
invulnerability, no duel marker (FACT, high, searched by name). The engine
facts that shape every row:

- `apply_effect` (`executor/dispatch.rs:588`, `effect_apply.rs:142`) targets
  only the acting entity, i.e. the player. It cannot target Mala'c.
- `launch_ability` (`dispatch.rs:550`) takes an `entity_tag` target, so the
  only route to put an effect on Mala'c is a content-launched ability.
- `remove_effect` is parsed (`actions.rs:62`) but has no executor arm; it falls
  to "Content: unhandled action" (`dispatch.rs:620`) (FACT, high).

| Effect | Recipient | Applied when | Must be removed when | Mechanism | Gap |
| --- | --- | --- | --- | --- | --- |
| Movement lock ("root") | Player | Arrival at the duel mark (entering 3959) | C-YIELD, C-DEAD, player death, abandon, relog | `apply_effect` self | Effect row ID needed; `remove_effect` arm missing |
| Movement lock ("root") | Mala'c | Spawn (stationary by RESTORED_SOURCE text) | Never; Mala'c is despawned | Not available: `apply_effect` self-only | Either a template-level immobile flag, or a content-launched ability; MAINTAINER/HA-03b decision |
| Aggression override hostile | Mala'c | C-START | Despawn clears it; reset sets neutral | `set_aggression` | Neutral enum value unresolved |
| Threat on the player | Mala'c | C-START | Despawn, or `submit` ignores it | `generate_threat` | None |
| `submit` AI state | Mala'c | C-YIELD | Despawn | `set_npc_ai_state` | None |
| Invulnerability | Player | Not used | n/a | n/a | Deliberately omitted: no seeded effect exists and a death-reset covers failure |
| Mala'c full heal | Mala'c | Reset to 3958 | n/a | `despawn_entity` then `spawn_entity` (fresh tag) | `spawn_entity` also matches corpses; order matters |

If the player's movement lock cannot be removed, the player is rooted
indefinitely. Mitigation, in preference order: a self-expiring effect with a
finite duration (no `remove_effect` needed, CIMMERIA_AUTHORING, medium
confidence it is expressible); the `remove_effect` executor arm; a relog,
which clears transient effects. Which of these exists is an HA-03b fact to
establish, not an assumption of this note.

## 5. Failure, death, abandon and relog

There is no player-death content trigger and there are no timers (FACT, high,
`engine_events.rs`, `lifecycle/mod.rs`). Cross-world respawn tears the
instance down; same-world respawn keeps the instance and Mala'c alive
(`cell-interactions/src/cell/respawn/mod.rs:110`) (FACT, medium: read, not
exercised).

### 5.1 Player death (the gap)

Options, with the recommendation first:

- Option A (recommended, MAINTAINER DECISION on scope): a server-side
  `player_died` content trigger. It needs no opcode, no client UI and no map
  patch; it is a new content-engine trigger fed from the cell's death path.
  Role C-DEATH, conditions `step_status 1325 = 3959 eq active`, actions:
  1. `remove_effect` movement lock (or let it expire).
  2. `set_npc_ai_state Rinla_Malac idle`, `set_aggression` neutral.
  3. `despawn_entity Rinla_Malac` then `spawn_entity` Rinla_Malac (full health,
     neutral, at the mark).
  4. `advance_step 1325 3958`.
  Back-step caveat (INFERENCE, medium): `AdvanceStep` takes an explicit step
  id (`actions.rs:216`), so a backward target is expressible, but whether the
  executor guards against it, and whether objective 4548 reopens as
  not-completed, is not read. The objective may need a `fail_objective`
  (`actions.rs:395`) first. Persistence caveat (FACT): completed steps do not
  persist as completed, and finished rows persist a NULL step, so the reset
  must be tested through a save/reload, not only in memory.
- Option B (no engine change): do nothing on death. Same-world respawn leaves
  Mala'c alive, so the mission stays on 3959 and the player re-engages by
  attacking him. This does NOT reset to 3958, contradicts O18-10 as briefed,
  and leaves the player's root effect to be handled by relog/expiry. Recorded
  only as the fallback if Option A is declined.

### 5.2 Abandon (role C-ABANDON)

Trigger `mission_abandoned 1325` (fires after `abandon_mission`; FACT, high,
`executor/dispatch.rs:334`). Actions: `despawn_entity Rinla_Malac`, remove the
Moh'katan in-progress bind, remove the movement lock. Condition is not
needed; the trigger is mission-keyed.

### 5.3 Relog (role C-RELOG)

`player_loaded` is the restore site. Gate on `mission_status 1325 eq active`
and the step: if the step is 3957 or 3958, respawn `Rinla_Malac` neutral and
re-bind 4370; if it is 3959, behave as a death (reset to 3958 and respawn);
if it is 4036, re-add the turn-in bind and spawn nothing. Cross-world
respawn and logout tear the instance down, so the spawned Mala'c is gone on
relog and must be recreated (INFERENCE, medium). The effect list in section 4
needs the same pass: a relog is the only certain way a lock is cleared today.

Every chain above is a separate trigger key from its siblings except
`player_loaded`, whose rows must be pairwise disjoint on `mission_status` and
`step_status` (FACT, the multi-chain dispatch rule).

### 5.4 Re-trigger and one-shot guards

- Offer and turn-in dialogs re-fire on re-display. Gate with
  `mission_status 1325 eq not_active` (offer) and `step_status 1325 = 4036 eq
  active` (turn-in).
- `once` is per player per cell-entity life and is not persisted, so it is not
  a once-per-character guard; completion is.

## 6. Offer and turn-in (Moh'katan)

- Offer, role C-OFFER: dialog 4369 on Moh'katan's per-player bind, shown when
  `mission_completed 1324` and `mission_status 1325 eq not_active`. This is the
  same partner pattern as the 1324 chains (`harset_jaffa_chains.sql:425,
  706-707`). No `dialogs.accepts_mission_id` is set on any 1392 dialog (all
  NULL, FACT), so acceptance is an explicit `accept_mission 1325` on the
  closing choice, not a dialog side effect.
- Mission-accepted, role C-ARM: on `mission_accepted 1325`, spawn and prepare
  Mala'c (section 3.1), `add_dialog_set` 1392 on `Rinla_Malac`, and remove the
  offer topic from Moh'katan. Note the repo wart that an accept can land on the
  dialog close; the same-trigger, same-key ordering is as in the other offers.
- Talk to Mala'c, role C-TALK: dialog 4370 on the `Rinla_Malac` bind, gated on
  `step_status 1325 = 3957 eq active` and `open_dialog_id`; on its completion
  `advance_step 1325 3958`. The exact completion trigger (last-screen
  `dialog_choice` versus a close event) is the implementation's to confirm; the
  rule that `dialog_choice` is gated on `open_dialog_id` and keyed by
  `screen_id` holds.
- Move into position, role C-MARK: needs a region or proximity trigger
  (`ID needed: region` for the duel mark, placeholder per HA-01a/HA-02). On
  arrival: apply the player movement lock, then `advance_step 1325 3959`, then
  C-START. If no usable region trigger exists, fall back to completing 3958
  from the end of the 4370 dialog and treating the mark as wherever Mala'c
  stands; that removes the region ID at the cost of the "move" step being a
  no-op (INFERENCE; HA-03b picks).
- Turn-in, role C-TURNIN: dialog 4372 on Moh'katan, gated on
  `step_status 1325 = 4036 eq active`, closing with `complete_mission 1325`
  (rewards: ID needed, see section 8), then `despawn_entity Rinla_Malac`.
  Order: despawn and state changes first, `complete_mission` last, because
  condition snapshots are frozen but later actions see the new state.
- Visible first-press feedback (rule): every interaction in this mission ends
  in a dialog, bark or log change. Required tie-ins: a log update when 3957
  starts; a bark (or monologue dialog) when the lock is applied and when it is
  lifted; a bark at the yield. Not `system_message`: FACT, it is a log-only stub
  (`executor/dispatch.rs:312-315`). No silent press.

## 7. Chain slots by role (no IDs assigned)

All slots come out of the block already reserved for HA-03 / 1325, chains
6311-6330 ([id-allocation.md](../id-allocation.md)). This note allocates
nothing; a role below is "ID needed: chain" until the owner of that file
assigns it.

| Role | Trigger | Disjoint from | Notes |
| --- | --- | --- | --- |
| C-OFFER | Moh'katan dialog (4369) | C-TURNIN | Gates on `1324` completed and 1325 `not_active` |
| C-ARM | `mission_accepted 1325` | none | Spawns Mala'c, binds 1392 |
| C-TALK | Mala'c dialog 4370 | C-OFFER, C-TURNIN | `step_status 3957 active` |
| C-MARK | region entry (ID needed) or 4370 completion | C-TALK | Applies lock, advances to 3959 |
| C-START | `step 3959 begins` (same action list as C-MARK is simplest) | none | Folds into C-MARK to avoid a second chain on one event |
| C-YIELD | `entity_health_below Rinla_Malac:30` | C-DEAD | Surrender |
| C-DEAD | `entity_dead_tag Rinla_Malac` | C-YIELD | Lethal-hit fallback |
| C-DEATH | `player_died` (engine seam, Option A) | C-RELOG | Resets to 3958 |
| C-RELOG | `player_loaded` | C-DEATH | Pairwise-disjoint rows by step |
| C-ABANDON | `mission_abandoned 1325` | none | Cleanup |
| C-TURNIN | Moh'katan dialog (4372) | C-OFFER | `complete_mission` last |

Eleven roles; the reserved block has 20 slots, so there is room (FACT). 1326's
offers 6331, 6333 and 6341 already gate on `mission_status 1325 eq completed`
and stay untouched; they depend on 1325 completing, and nothing in this design
changes that. A guard to carry into review: because 1325 can complete only
through C-TURNIN, its failure paths must never leave the mission at
`completed`, or the 1326 chain fires early.

## 8. Items this note cannot fill (explicit)

- Duel and Mala'c mark positions: `ID needed: position`, placeholder (HA-01a,
  HA-02).
- Rewards for 1325 (XP, naquadah, items): not seeded and not stated by any
  source read here. Left to the owner; INFERENCE (low) that the original gave a
  small reward.
- Yield bark and failure bark text: no restored line exists for them. They
  are CIMMERIA_AUTHORING and need `ID needed: npc_bark text`.
- Effect rows: `ID needed: effect` (player movement lock, optionally a
  Mala'c immobile effect).
- Mission step and objective enable flags in the seeds are all false. Whether
  the content engine or a seed flag enables them at accept is not checked
  here; HA-03b verifies it against another seeded mission before authoring.

## 9. Maintainer decisions (design stops here)

1. Option A, a server-side `player_died` content trigger. This is a content
   engine addition, not an opcode or client change, but it is the owner's call
   (or the engine owner's) whether to take it.
2. A Mala'c immobile mechanism if neither a template flag nor a launched
   ability is acceptable.
3. Any client change to the Rin'la dialog, a visual duel marker or a root
   indicator. None is proposed.
4. Rewards.

## Tests the implementation owes

Each test must fail when the named behaviour is reverted.

1. Health trigger: crossing from 31% to 29% with `step_status 3959 active`
   fires C-YIELD exactly once; a crossing in 3957/3958 fires nothing. Revert:
   drop the `step_status` condition and the second case fails.
2. Lethal hit: a single hit from above 30% to zero fires C-DEAD and not
   C-YIELD; the step ends at 4036 in both orders. Revert: remove the dead
   fallback and the mission stalls at 3959.
3. Disjointness: the loader or a fixture test proves C-YIELD, C-DEAD, C-DEATH
   and C-RELOG never match the same event.
4. Spawn idempotency: a second `spawn_entity Rinla_Malac` after a kill does not
   create a duplicate and does not resurrect; the death reset despawns first.
5. Death reset: with Option A, a player death in 3959 returns the persisted step
   to 3958, restores Mala'c at full health and neutral, and the movement lock
   is gone. Verified through a save/reload, because completed steps do not
   persist as completed.
6. Back-step: `advance_step` to an earlier step reopens its objective and
   does not complete the mission.
7. Abandon: after `abandon_mission 1325`, no `Rinla_Malac` remains, the lock
   is gone and the offer can be taken again.
8. Relog: logging out in each of 3957, 3958, 3959 and 4036 and back in lands
   in the documented state, with no duplicate Mala'c and no stuck lock.
9. 1326 gate: the 1326 offers 6331, 6333 and 6341 do not fire for a player who
   failed or abandoned 1325, and fire after C-TURNIN.
10. Shared-world rule: a test or review check that no chain here mutates a
    shared world 57/68 actor for other players (binds are per player).
11. Visible feedback: each step transition emits at least one log, bark or
    bark or monologue dialog.
12. Effect removal: a lock applied then removed (or expired) restores
    movement; this needs the `remove_effect` executor arm or the finite
    duration alternative, whichever HA-03b chooses.

## Telemetry the implementation owes

- Per chain: fired, role name, mission 1325, step before and after, player
  id.
- The duel: start, yield, dead-fallback, player-death reset, abandon cleanup,
  relog repair, each with the Mala'c entity tag and instance id.
- Effects: every `apply_effect`/`remove_effect` (or expiry) on the lock with
  recipient and outcome, and a warning when a lock is still present after a
  step change away from 3959.
- Spawn: `spawn_entity` for `Rinla_Malac` with the instance and whether it
  matched an existing tag (so a duplicate or corpse match is visible).
- A counter of 1325 resets to 3958, so a stuck loop is visible from the
  telemetry alone.
