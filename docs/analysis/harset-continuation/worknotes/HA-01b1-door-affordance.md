# HA-01b1 — step 5225 right-click door feasibility

Date: 2026-10-18. Packet HA-01b1 (wave R1), mission-systems-advisor. Decision: O18-12 in the [O18 register](owner-decisions-2026-10-18.md#o18-12--1664-step-5225-door). Docs only: no code, seed, ID, lab, client, Ghidra or cargo was touched. Nothing here has been seen in the client.

Labels follow rule R4: **FACT** (file and line or row), **INFERENCE** (confidence), **DECISION**, **PROVISIONAL**. Every proposed beat is **CIMMERIA_AUTHORING** unless it says otherwise.

## Verdict

1. **A server-only right-click door is feasible on paper.** INFERENCE, medium confidence. The recipe is an existing, shipped pattern: a tagged world-object entity, an `interact_tag` chain, and a per-player interaction bind that makes the prop clickable only for players on the step. Nothing needs a new opcode, UI or map patch.
2. **6006 cannot be reused as the interaction.** FACT-backed, high confidence. It is a walk-in `enter_region` trigger and it touches no mission state. The answer to the O18-12 reuse question is **no**.
3. **No MAINTAINER DECISION is forced yet.** Two things are unproven and both need the client: that a prop with no baseline interaction bits shows a right-click cursor from a bind alone, and which existing mesh reads as a door. If either fails, this becomes a MAINTAINER DECISION (see [Open items](#open-items)). The door position is a placeholder: world-57 door positions are still BlockedEvidence.
4. **Two hazards matter more than the mechanism.** `complete_objective 6060` would end mission 1664 early, and the walk-in region 6006 stays open beside the right-click door. Both are in [Hazards](#hazards).

## Q1. What objectives 6059 and 6060 contain

| Item | Value | Label |
|---|---|---|
| Step 5225 | mission 1664, index 1, "Make your way to the Command Center." `step_enabled` false | FACT, `db/resources/Missions/Seed/mission_steps.sql:2881` |
| Objective 6059 | step 5225, **optional**, "(Optional) Follow the path on your mini-map to reach the command center." | FACT, `mission_objectives.sql:3166` |
| Objective 6060 | step 5225, **required**, "Interact (right-click) with the command center door to enter the command center." | FACT, `mission_objectives.sql:3168` |
| Tasks of 6059 | eight rows, task ids 7746–7753 | FACT, `mission_tasks.sql:3519-3533` |
| Task of 6060 | one row, task id 7745 | FACT, `mission_tasks.sql:3535` |
| Task type | `task_type = 1` on every one of the 4358 task rows in the seed, so it does not discriminate | FACT (count over `mission_tasks.sql`) |

What the client expects for 6060:

- **FACT:** the only client-visible content is the log text above plus the task and objective ids that arrive in mission frames. The seed has no machine-readable "interact with entity X" field, and the task-type strings are not a client-known taxonomy (see `spec.missions.task-primitives`).
- **INFERENCE, high:** the "right-click" is player-facing text. The server decides what credits 6060, so the client does not check that a right-click happened.
- **INFERENCE, medium:** the eight tasks on 6059 are mini-map path segments. They are never credited by a chain and need none, since 6059 is optional.
- **Name clash to avoid:** objective 5225 belongs to step 4490 (`mission_objectives.sql:2164`) and tasks 6471/6473 hang off it. Dialogs 6059 and 6060 (`dialogs.sql:10133,10135`) are Burtonol conversations and dialog-set-map ids 6059/6060 (`dialog_set_maps.sql:6385,6389`) belong to dialog set 1628. They share numbers with step 5225's objectives only by coincidence.

## Q2. Server-only mechanisms for a right-click at the door

### The shared click path (FACT)

A right-click arrives as `interact(target_entity_id)` and is handled in `crates/cell-methods/src/cell/cell_methods/player/interaction/interact.rs`:

1. Dead-actor and negative-id guards (lines 34-68).
2. A hostile alive NPC is rerouted to an attack (lines 76-89).
3. A **range gate** at `MAX_INTERACT_DISTANCE = 5.0` (`crates/cell-world/src/cell/space_manager/interact_range.rs:13`). It runs before every chain, and a too-far click gets a chat line (lines 108-125).
4. The target is pinned as `last_interaction_target` before any chain dispatch.
5. `fire_interact_tag(tag)` runs, then `fire_interact_template(name)` (about lines 161-198). It builds the event in `crates/cell-content/src/cell/content/event_dispatch/interaction.rs:25-60`, and `engine.resolve_event` returns the actions of **every** matching chain.
6. Only if no chain matched does `interactions::handle_interact` run (`crates/cell-interactions/src/cell/interactions/dispatch/interact.rs`). A bound dialog opens, else the static interaction type decides.

Older notes that say the `interact_tag` path skips the range gate and the pin are stale in this worktree: both now run first (comment at `interact.rs:91-124` and the pin call before the trainer check). Agent-memory note `interact-dialog-routing-traps.md` should be read with that in mind.

### Option A — server-placed tagged prop (recommended)

**What:** a world-object entity at the door, with a tag, a prop template, and one `interact_tag` chain. The prop is the right-click target. The client cannot right-click a map mesh that the server has no entity for.

- **FACT, precedent in the same world:** the Harset ring switches are tagged `entity_templates` props (template 3, mesh `GP-Props.GP-Ring_Trans_Console00`, `interaction_type` 32) spawned in world 57 (`db/resources/Worlds/Seed/spawnlist.sql:111`). Their tag fires a server action in `deprecated/python/cell/spaces/Harset.py:22-48` and in chains 6001–6005.
- **FACT, precedent for a door control:** template 22 is named "Door control panel" (mesh `GLB-RingTransporterSwitch_00`, `interaction_type` 0, `entity_templates.sql:55`). It is spawned in Cellblock as tag `329_CellDoorButton` (`spawnlist.sql:66`). Chain 1016 (`castle_cellblock_chains.sql:699`) turns its click into mission progress.
- **FACT, precedent for a baseline-0 prop made clickable per player:** `Castle_AccessPanel` (template 147, `interaction_type` 0) is clicked through `interact_tag` chain 1322 (`castle_706_708_chains.sql:318`). Agent-memory note `castle-cellblock-chains.md` records that a dialog-set bind whose map row carries `interaction_flags` makes a baseline-0 body clickable.
- **Credit path:** `interact_tag` chain, then `advance_step` (see [Q5](#q5-graph-sketch-for-step-5225)). The chain action is `advance_step`, not `complete_objective`.

**Two ways to place the prop:**

| Placement | Pro | Con |
|---|---|---|
| Static `spawnlist` row in world 57 | Always present, survives relog, same as every ring switch | A seed edit in the Harset spawn band; ID needed: spawn |
| `spawn_entity` action from the step-5225 entry chain | Prop appears only while someone is on the step | The spawn is **shared** and one-shot with no respawn (`action_spawn.rs:82-108`), and no shipped chain uses it (`harset_spawn_entity.rs:3-6`); a second player or a restart leaves it missing |

INFERENCE, medium-high: use the static row. A temporary shared prop adds despawn and relog problems that a permanent prop avoids.

**Per-player clickability (INFERENCE, medium):** bind a dialog-set map whose `dialog_id` is NULL and whose `interaction_flags` carry `INT_MissionWorldObject` (bit 30, `1073741824`) to the prop template with `add_dialog_set`. This is an interaction-only flag carrier. It gives a per-player cue and creates no shared state. `set_interaction_type` is the wrong tool: it mutates the shared entity and broadcasts to every witness, and the header of `castle_706_708_chains.sql:135-189` forbids clearing the last bit on a shared prop.

**Can the client right-click it?** Not provable offline. The bitfield "drives the client's right-click cursor" (`crates/entity/src/interaction_flags.rs:1-9`, FACT), but whether a baseline-0 prop shows the cursor from a bind alone is a client behavior. The Castle pattern is shipped, yet this worknote has no client-acceptance evidence for it. **Needs the client.**

### Option B — an existing map actor

**Not usable.** INFERENCE, medium-high.

- The Harset doors are map meshes and Kismet sequences, not BigWorld entities. HA-01a found that the interior maps mark doorways with `StaticMeshActor`s, and that `GA-small_doorway_open_a_00` carries only a Kismet prefab sequence (`HA-01a-bsp-door-probe.md`, "Storage (world 70)").
- The client sends `interact` for an entity id. A map mesh has none, so the server cannot receive or answer the click.
- No world-57 door actor was ever located (`HA-01a-bsp-door-probe.md` Limits).

### Option C — reuse chain 6006

Covered in [Q3](#q3-can-6006-give-the-same-interaction). It fails.

## Q3. Can 6006 give the same interaction?

**No.** High confidence.

| Test | 6006 | Needed for O18-12 | Label |
|---|---|---|---|
| Trigger | `enter_region` on `Harset.CommandCenterTransition` (point set 2078, `harset_space_chains.sql:209-210`) | A right-click | FACT |
| Player action | Walking into an axis-aligned box: x −7.07..6.58, y −67.66..−60.54, z −243.52..−238.41 (`point_set_points.sql:219-225`) | A deliberate interact within 5 units | FACT |
| Mission effect | None. Its only action is `cross_world_teleport` to `Harset_CmdCenter` at (0, 0.355, −20) (`harset_space_chains.sql:217-219`). No condition mentions 1664 | Credit step 5225 | FACT |
| Gate | World 57 only, for every player, always | On step 5225 for the credit | FACT |
| Source | The 2009 script also used a region callback for this door (`Harset.py:52-60`) | The step text says right-click | FACT, an unresolved conflict between sources |

Making 6006 credit the step would mean adding a mission action to a region trigger. That changes both the affordance (walk-in, not click) and the step's behavior. O18-12 allows 6006 only if it changes neither, so it is excluded.

Open conflict worth recording: the 2009 python used a region for the door while the 1664 objective text says right-click. INFERENCE, low: the Atrea authors let the client door open on a click and used the region as the zone line, which would mean the original had **both**. This cannot be settled offline.

## Q4. First-press feedback, and what needs the client

Rule: every interaction gets visible feedback on the first press.

| Click case | What the player sees | Label |
|---|---|---|
| On step 5225 | Objective-completed ticks for 6059/6060, the step and mission-log frames, then the world change to the Command Center | INFERENCE, medium. `advance_step` sends them (`crates/cell-content/src/cell/missions/progression.rs`, see agent-memory `advance-step-vs-complete-objective.md`); the teleport is the visible event |
| Anyone else (door open to all, parity with 6006) | The world change | INFERENCE, medium |
| Out of range | The generic out-of-range chat line | FACT, `interact.rs:108-125` |

Feedback constraints:

- **`system_message` is a stub.** FACT: `executor/dispatch.rs:312-315` only logs; its wire format is unknown and it once garbled chat. Do not use it for feedback.
- **Fallbacks that exist:** `npc_bark` (non-modal companion line) or a monologue `display_dialog` whose screens all have `speaker_id` 0. Either needs a text or dialog row: `ID needed: dialog` or `ID needed: bark text`.
- **Wrong-step case:** a 1664 player still on step 5224 who clicks the door is not refused. They teleport and keep 5224 active. That choice belongs to HA-01b2 and the owner.

**Client changes:** none are needed for the mechanism itself. Three client facts remain unproved and are listed in [Open items](#open-items). If the first one fails, it is a **MAINTAINER DECISION** and the design stops there.

## Q5. Graph sketch for step 5225

**PROVISIONAL.** Candidate chain band is the HA-01b band 6008–6020 in [id-allocation.md](../id-allocation.md); no ID inside it is allocated here. All chains are in world 57. Door position is a placeholder.

| Role | Trigger | Conditions | Actions (in this order) |
|---|---|---|---|
| Bind the cue on step entry | The Kaerlia debrief chain's `advance_step 5225` (owned by HA-01b2/HA-01b) | none new | `add_dialog_set` for the NULL-dialog door bind (`ID needed: dialog set`, `ID needed: dialog set map`). Order **before** the mission mutation if cleanup is involved, per the engine contract |
| Relog restore of the cue | `player_loaded` (`ID needed: chain`) | `step_status 1664 / 5225 eq active`, `world eq 57` | `add_dialog_set` for the same bind. Binds are in-memory only (agent-memory `castle-cellblock-chains.md`; Castle chain 1323 is the precedent) |
| Door click, on the step | `interact_tag` `ID needed: tag` (`ID needed: chain`) | `world eq 57`, `mission_status 1664 eq active`, `step_status 1664 / 5225 eq active` | 1. `remove_dialog_set` for the bind. 2. `advance_step 1664 → 5226` (force-credits 6059 and 6060). 3. `cross_world_teleport Harset_CmdCenter` to (0, 0.355, −20) |
| Door click, anyone else | same `interact_tag` (`ID needed: chain`) | `world eq 57`, and **not** the row above: `step_status 1664 / 5225 neq active` | `cross_world_teleport Harset_CmdCenter` to (0, 0.355, −20) |
| Abandon cleanup | the mission-abandon trigger used by sibling missions (`ID needed: chain`) | `world eq 57` | `remove_dialog_set` for the bind |

Guards, as the brief asks:

- **Active 1664, step 5225, world 57:** the conditions on rows 3 and 4. The two door-click chains share one trigger key, so they must be **pairwise disjoint** (`multi-chain-dispatch-semantics.md`). `neq` on `step_status` is a parsed operator (`condition.rs:20-31`); that it evaluates true for a player with no 1664 row is INFERENCE, medium, and needs a replay test.
- **Duplicate click:** after the first click the step is 5226 and the bind is gone, so the second click matches only the "anyone else" row. If the teleport was refused, the player is still in world 57 and gets the parity teleport, so there is no stuck state. The credit never repeats because the credit row needs 5225 active.
- **Relog:** the bind is in memory, so the restore chain re-adds it. A relog after the click finds the player in world 68 on step 5226, so nothing is owed.
- **Range:** the 5-unit gate is the engine's, not the chain's, so no chain condition is needed.
- **Teleport coordinates:** copy chain 6006's destination exactly (0, 0.355, −20). `cross_world_teleport` cannot set yaw (`harset_space_chains.sql:189-192`).
- **Cleanup order:** the bind is removed **before** `advance_step`, per the contract that item and bind cleanup precede the mission mutation.

## Hazards

1. **`complete_objective 6060` ends the mission.** FACT: `complete_objective` auto-completes the mission once every non-optional active objective is complete (`progression.rs:463-505`). Step 5225's only required objective is 6060, so the click would complete mission 1664 and skip step 5226 "Report for debriefing". Use `advance_step`, which does not run that check and force-credits both objectives (agent-memory `advance-step-vs-complete-objective.md`).
2. **6006 stays open and credits nothing.** FACT: 6006 is enabled for everyone and serves other content (Jaffa arc, `harset_jaffa_chains.sql:129`; the 6007 return door unparks mission 1361's acceptance chains, `harset_space_chains.sql:268-271`). It cannot be disabled. If the real door does not physically block the box, a player on step 5225 who walks in lands in world 68 with 5225 still active and no way to advance, because the credit lives only on the click. **Owner choice (see [Open items](#open-items)):** add a credit-only `enter_region` chain on the same step as a soft-lock guard, or accept the risk. A second chain on the same event also fires alongside 6006 (every matching chain fires), so it needs `step_status 5225 eq active` and must not re-teleport.
3. **Shared prop, per-player meaning.** The prop is visible to everyone. Do not mutate the shared actor per player. Use the per-player bind, and never clear the last shared interaction bit.
4. **Mesh may not look like a door.** Template 22's mesh is a ring-switch console. A console beside the door is a reasonable "door control" but is not the door. A door-shaped mesh has no seeded template, so a better look is `asset needed` and is not invented here.
5. **6059 needs no chain of its own.** The optional 6059 is credited by the force-complete in `advance_step`, so no chain needs to touch it.

## Open items

| Item | Needs | Why |
|---|---|---|
| Does a baseline-0 prop show a right-click cursor from a NULL-dialog bind with `INT_MissionWorldObject`? | Client (lab, ask the user first) | Castle uses this shape, but no client proof is cited. If it fails: **MAINTAINER DECISION** (a baseline `interaction_type` on a new template is a seed change, a cursor fix would be a client change) |
| Which mesh reads as the door, and where does the prop stand? | Client look + exterior probe | Template 22's console mesh is the only "door control" template. World-57 door position is BlockedEvidence; the 2078 box above bounds it but does not place it |
| Does the real door block walking into the 2078 box? | Client | Decides whether the soft-lock guard in hazard 2 is needed |
| Soft-lock guard on region entry: add it or not? | Owner | It adds a second credit path next to the right-click door |
| Wrong-step click (1664 active, step 5224) | Owner / HA-01b2 | The door is open to all, so Kaerlia's step can be skipped |
| `neq` on a missing step row | Replay test | Disjointness depends on it |

## Tests the implementation owes

Each test must fail when the named revert is applied.

| Test (chain replay and live-DB, in the style of `chain_replay_tests/harset_space.rs`) | Revert that must make it fail |
|---|---|
| Door click on step 5225 advances 1664 to 5226 and does **not** complete the mission | Swap `advance_step` for `complete_objective 6060` |
| Door click on step 5225 teleports to `Harset_CmdCenter` at (0, 0.355, −20) | Drop the `cross_world_teleport` action or change the coordinate |
| The two door-click chains are disjoint: every state in {no 1664, 1664 on 5224, on 5225, on 5226, completed} fires exactly one | Remove the `neq` gate from the "anyone else" chain |
| Duplicate click after a refused teleport does not credit twice and does not stick | Remove the 5225 gate from the credit chain |
| `player_loaded` on step 5225 re-adds the bind; on any other state it does not | Drop the restore chain or its step condition |
| The bind is removed before the mission mutation and on abandon | Reorder the actions, or delete the abandon chain |
| 6006 still teleports with no mission effect | Add a mission action to 6006 |
| The prop exists in world 57 with the sealed tag and template | Delete the spawn row |
| A click from beyond 5 units is refused with the out-of-range line and credits nothing | Remove the range gate |

## Telemetry the implementation owes

- Chain fire rows with chain id, tag, player, world and the matched condition set. `fire_interact_tag: matched` already logs the tag and action count.
- One structured row per credit: mission 1664, step 5225 to 5226, objectives 6059 and 6060, source `door_click`, and the teleport result.
- A distinct row for the parity teleport (no mission), so "door used without the step" is countable.
- A row when the cue bind is added, restored and removed, with the reason (`step_entry`, `relog`, `credited`, `abandon`).
- The existing `interaction.out_of_range` row, for the too-far case.
- If the soft-lock guard is built, a row when a region entry credits the step, so click credits and walk-in credits can be told apart in SigNoz.
