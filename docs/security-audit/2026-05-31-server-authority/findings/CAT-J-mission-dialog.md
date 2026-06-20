# CAT-J — Mission / Dialog / Interaction

**Overall trust posture**: The wire surface for this category is **broader than
the implemented surface**. The Rust dispatch hooks up `INTERACT` (74),
`DIALOG_BUTTON_CHOICE` (75), `INITIAL_RESPONSE` (76) on the SGWPlayer interface
and `ABANDON_MISSION` (52), `SHARE_MISSION` (53), `SHARE_MISSION_RESPONSE` (54)
on the Missionary interface. Every other client-emitted name in CAT-J
(`Event_NetOut_MissionAdvance`, `MissionComplete`, `MissionAssign`,
`MissionReset`, `MissionClear`, `MissionClearActive`, `MissionClearHistory`,
`MissionList`, `MissionListFull`, `MissionDetails`, `MissionSetAvailable`,
`ChosenRewards`, `DebugInteract`, `AbandonMission` as a slash-cmd variant) is
registered in `register_NetOut_*` on the client (confirmed via Ghidra string
search + `register_NetOut_*` decompile showing `Direction: NetOut (client ->
server)`), and a non-trivial subset of them also has a sibling
`Event_SlashCmd_*` registration routed through `SGWTextCommandMgr` — so they
are emittable from console as well as from UI affordances. None of those names
hit a dedicated cell-method handler today; they fall through to the unhandled
`warn!` at `crates/services/src/cell/dispatch/router.rs:101` (`"Unhandled cell
method call -- no registered handler for this index; client behaviour may
diverge silently"`). That's good defense-in-depth for *server* state — these
won't mutate anything — but it leaves them as latent exploit shapes that a
future "wire it up" PR will inherit unless the validation is designed in now.

The substantive trust violations live in the *implemented* paths. The
strongest finding is **CAT-J-01**: `DIALOG_BUTTON_CHOICE` fires a chain-engine
event keyed on a client-supplied `dialog_id` with **no server-side check that
the dialog is or was ever open for this player**. Chain actions reachable via
`OnDialogChoice` include `GrantXP`, `GrantItem`, `Teleport`, `AcceptMission`,
`CompleteMission`, `AdvanceStep`, `AbandonMission` — so a single forged
packet can drive any of those for any `dialog_id` an attacker discovers. The
client-side `DialogPortrait` widget hides which dialog_ids exist, but they're
not secret — they're in cooked client data (`SGWGame/CookedData/*`) and
recoverable via Ghidra. **CAT-J-04** is the matching gap on `MissionAssign`
prereqs (no level / faction / prior-mission gate is checked on the cell-side
`accept_or_advance`). **CAT-J-02** is the analogous `INITIAL_RESPONSE` gap
(dialog selection unscoped from `last_interaction_target`).

Note: the trace from CAT-J chains back into CAT-D (Inventory) and CAT-C
(Combat) on the *action* side — a finding closed in CAT-J that ends up
delegating to `GrantItem` or `Teleport` is only safe to the extent CAT-D
and CAT-O cover their own input validation. Mission-systems-advisor and
movement-physics-advisor should consult-review on **CAT-J-01** and **CAT-J-04**.

---

### CAT-J-01 — DialogButtonChoice fires arbitrary content chain with no "is this dialog open?" check

**Status**: ✅ RESOLVED (#479) — `CellEntity::open_dialog_id` is pinned by
`send_dialog_display` (the single choke point all display paths route
through) and matched on strict equality in the `DIALOG_BUTTON_CHOICE`
handler; a forged/replayed choice for an un-opened `dialog_id` is dropped
with a `warn!` and never fires the chain. The pin is cleared one-shot on a
valid choice (mirrors python `SGWPlayer.displayedDialogs`), which also
makes a replayed choice idempotent (closes the replay sub-finding). The
`button_id` stays unvalidated by design — `OnDialogChoice` matches
`dialog_id` only. **Does not** address CAT-J-04 (no level/faction/prereq
gate on `accept_or_advance`); a legitimately-opened reward dialog still
accepts without prereq checks — that's the separate follow-up finding.

**Severity**: Critical
**Class**: Missing server-side state precondition; chain-engine event spoofing
**Wire surface**: `Event_NetOut_DialogButtonChoice` (cell method 75)
**Demonstrable / Likely-theoretical**: Likely-theoretical (Ghidra confirms
client emits `dialog_id:i32, button_id:i32` payload via
`SGWNetworkManager::EventHandler<Event_NetOut_DialogButtonChoice>` at
`0x00d68050`; Rust handler obeys without precondition check)

**Trust violation**
The client sends `(dialog_id: i32, button_id: i32)` and the server processes
it by calling `fire_dialog_choice(entity_id, player_id, dialog_id, button_id,
…)` (`cell/cell_methods/player/interaction.rs:247-250`). That helper
unconditionally builds a `TriggerEvent { trigger_type: DialogChoice, params:
{dialog_id, button_id, …} }` and resolves it against the chain engine. The
engine matches any registered `OnDialogChoice { dialog_id }` chain
(`crates/content-engine/src/triggers.rs:287`) and executes its actions via
the executor. There is **no server-side bookkeeping of "is dialog X currently
open for player Y?"** — `CellEntity` carries `last_interaction_target` but
not a `current_dialog_id`, and `available_interactions` is consulted only by
the `INTERACT` and `INITIAL_RESPONSE` *open* paths, not by the *choice*
handler. The button_id field has no validation at all — `OnDialogChoice` only
matches on `dialog_id`, the button_id is just stuffed into params and is
available to chain conditions if any choose to read it.

The action surface reachable via `OnDialogChoice` includes (see
`crates/content-engine/src/actions.rs:20–`): `GrantXP`, `GrantItem`,
`RemoveItem`, `Teleport`, `CrossWorldTeleport`, `AcceptMission`,
`AdvanceMission`, `CompleteMission`, `AdvanceStep`, `AbandonMission`,
`CompleteObjective`, `RollLootTable`, `SpawnEntity`, `DespawnEntity`,
`TriggerChain`. An attacker who replays `DialogButtonChoice(dialog_id=N)` for
a `dialog_id` bound to any of these actions gets the action's side-effect
without ever opening the dialog, talking to the giver NPC, being in the
giver's space, or meeting any of the gating conditions the chain author
*expected* would be enforced by the precondition of having opened the dialog.

**Evidence**
- Ghidra: `0x019b3e30` `Event_NetOut_DialogButtonChoice` — string + class
  registration confirms client→server direction; `0x00d68050`
  `SGWNetworkManager::EventHandler<Event_NetOut_DialogButtonChoice>::vfunc_0`
  is the server-side bundle deserializer. Payload from the cell handler:
  `dialog_id: i32 LE | button_id: i32 LE` (8 bytes total, parsed at
  `interaction.rs:239-240`).
- Client behavioral log: n/a (no symbol-named log entries for dialog choice
  in `SGWDebugLog.log`).
- Cross-ref to Rust handler (for the fix author, NOT as truth):
  `crates/services/src/cell/cell_methods/player/interaction.rs:237-253`
  (handler) → `crates/services/src/cell/content/event_dispatch/dialog.rs:69-111`
  (`fire_dialog_choice`).

**Attack scenario**
1. Attacker enumerates dialog_ids referenced by `OnDialogChoice` chains by
   inspecting content seeds (the chain table is loaded from PG; for an
   external attacker this is recoverable via Ghidra dialog-id constants
   plus knowledge of which missions have reward dialogs).
2. Attacker crafts a Mercury cell-method bundle for method_index = 75 (the
   SGWPlayer `DIALOG_BUTTON_CHOICE` index — pinned by
   `cell_methods/player/dispatch.rs` test
   `cell_method_constants_pin_expected_values`) with `args =
   dialog_id_LE || button_id_LE` (8 bytes).
3. Server's `dispatch_cell_method` routes to `player::dispatch` →
   `interaction::dispatch::DIALOG_BUTTON_CHOICE` arm → `fire_dialog_choice`.
4. Content engine executes any chain bound to that `dialog_id`. **Observable
   effect**: the chain's actions fire with no precondition — most directly,
   `GrantXP` / `GrantItem` give the attacker the reward bundle of any
   dialog-completion chain; `AcceptMission` / `CompleteMission` skip the
   intended quest path; `Teleport` / `CrossWorldTeleport` moves the player
   without the dialogue-context the chain author assumed.

**Suggested remediation (one line)**
Pin the currently-open dialog on the player (e.g. `player.open_dialog_id:
Option<i32>`) at `send_dialog_display` time, clear it on choice or on a new
dialog open, and reject `DIALOG_BUTTON_CHOICE` whose `dialog_id !=
player.open_dialog_id` with a `warn!` (consult mission-systems-advisor on
multi-dialog overlap rules).

**Would benefit from x64dbg trace?**
Yes — confirm the precise wire shape (8 bytes vs longer encoding for the
button_id), and confirm `SGWNetworkManager::EventHandler<Event_NetOut_
DialogButtonChoice>::vfunc_0` doesn't append a session token the Rust
handler is dropping.

---

### CAT-J-02 — InitialResponse dialog selection is not scoped to the interacted NPC

**Severity**: Medium
**Class**: Missing scope check on per-player state lookup
**Wire surface**: `Event_NetOut_InitialResponse` (cell method 76, name on the
wire is `initialResponse` per `dispatch/names.rs:108`)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`handle_initial_response` (`crates/services/src/cell/interactions/dispatch.rs:
207–287`) takes the client-supplied `interaction_set_map_id: i32` and searches
**every** template-keyed entry in `player.available_interactions` for a
matching `dsm_id` (`for entries in p.available_interactions.values() { for
&(dsm_id, dialog_id, _) in entries { if dsm_id == interaction_set_map_id {
return Some(dialog_id); } } }`). The pin on `last_interaction_target` is
consulted only to populate the wire `EntityId` field of `onDialogDisplay` —
the lookup of *which dialog* to open is **unscoped from the actor**. A
client who has ever had `AddDialogSet`/`AddDialog` register multiple
interaction sets across multiple templates can send any
`interactionSetMapId` from any of them, regardless of which NPC they
just right-clicked.

This is bounded by the fact that the dsm_id must already exist in the
player's `available_interactions` (a server-side chain must have added it),
so it's not a full dialog-id-enumeration exploit. But the actor mismatch
lets a player open a chain-fired dialog *bound to one NPC* while interacting
with a *different NPC*, breaking the chain author's assumption that the
`OnDialogOpen` follow-up runs with the correct dialog-NPC pairing — and
specifically the `last_interaction_target` will reflect the **wrong** NPC
(the one the player most recently clicked, not the one the dsm_id belongs
to).

**Evidence**
- Ghidra: confirms client emits `interactionSetMapId: i32`; routing to method
  76 is pinned by `cell_method_constants_pin_expected_values` test.
- Cross-ref to Rust handler: `crates/services/src/cell/interactions/dispatch.rs:
  215-224` (unscoped dsm_id lookup loop).

**Attack scenario**
1. Server fires chain Z which calls `Action::AddDialogSet` adding entry
   `(dsm_z, dialog_z, _)` under `available_interactions[template_Z]` —
   intended to open when the player interacts with an NPC of template Z.
2. Server fires chain W which adds `(dsm_w, dialog_w, _)` under
   `available_interactions[template_W]` — intended for an NPC of template W.
3. Attacker INTERACTs with an NPC of template W (pins
   `last_interaction_target = NPC_W`).
4. Attacker sends `InitialResponse(dsm_z)`. Server scans
   `available_interactions.values()`, finds dsm_z, returns `dialog_z`, and
   sends `onDialogDisplay(EntityId=NPC_W, DialogId=dialog_z)` — the W NPC
   speaks Z's dialog, the chain follow-up fires with mismatched
   actor/dialog pairing.

**Suggested remediation (one line)**
Scope the dsm_id lookup to `available_interactions[template_of(last_
interaction_target)]` only, not the full map of all templates.

**Would benefit from x64dbg trace?**
No — the wire shape is short (just `interactionSetMapId: i32`), the bug is
purely in the server-side lookup scope.

---

### CAT-J-03 — Interact has no liveness check on the acting player

**Severity**: Low
**Class**: Missing actor-state precondition
**Wire surface**: `Event_NetOut_Interact` (cell method 74)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
The `INTERACT` arm at `crates/services/src/cell/cell_methods/player/
interaction.rs:22-235` validates `MAX_INTERACT_DISTANCE` and the target's
faction/aliveness for the auto-attack reroute, but **never checks whether
the acting player itself is alive** (no `is_dead_state(e.state_field)`
guard on `entity_id`). A dead player can right-click an NPC to open a
dialog, open a vendor, interact with a loot corpse, or trigger
`fire_interact_tag` / `fire_interact_template` chains, all while their own
`BSF_DEAD` is set. The downstream handlers (`send_store_open`,
`send_loot_display`, `fire_interact_*`) don't guard either.

The exploit shape is bounded — once the player is dead they can't actually
use most of the resulting UI (their movement and ability inputs are gated
elsewhere) — but mission-completion dialogs, mission-accept dialogs, and
content chains tied to `OnInteractTag` will fire and persist state changes
(mission accepts, counter bumps, `last_interaction_target` updates) even
while the player is in BSF_DEAD. That state then persists across respawn
and affects the chain-evaluation context.

**Evidence**
- Ghidra: `Event_NetOut_Interact` at `0x019bf184` (string ref) and
  `0x00d68010` (server-side handler vfunc_0) confirm client→server
  direction.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/player/
  interaction.rs:22-235` — no `is_dead_state` guard on the acting entity.

**Attack scenario**
1. Player takes lethal damage; server marks them BSF_DEAD but the death
   animation / respawn window is still running.
2. Before respawning, client sends `Interact(targetEntityId=NPC_GIVER)`.
3. Server resolves dialog via `handle_interact`, sets
   `last_interaction_target = NPC_GIVER`, fires
   `OnInteractTag`/`OnInteractTemplate` chains.
4. Chain `Action::AcceptMission` or `IncrementCounter` runs against a
   dead player, persisting state that "shouldn't" have happened until
   after respawn.

**Suggested remediation (one line)**
Drop `INTERACT` (and `DIALOG_BUTTON_CHOICE`, `INITIAL_RESPONSE`) at the
top of their handlers when `is_dead_state(player.state_field) == true`,
matching the implicit assumption that a dead player can't talk.

**Would benefit from x64dbg trace?**
No — the wire shape is well-known (`targetEntityId: i32`); the bug is
absence of a guard, not a wire-format ambiguity.

---

### CAT-J-04 — accept_or_advance has no mission-prerequisite validation

**Severity**: High
**Class**: Missing server-side precondition (level / faction / prior-mission gating)
**Wire surface**: Chain-driven via `Action::AcceptMission`. Reachable from the
client via `DIALOG_BUTTON_CHOICE` (75) when a chain is keyed on
`OnDialogChoice` with an `AcceptMission` action, or via `INITIAL_RESPONSE`
(76) → `fire_dialog_open` → `OnDialogOpen` chain with an `AcceptMission`
action. **Not directly reachable** from `Event_NetOut_MissionAssign` because
that cell method has no handler today (falls through to the unhandled
`warn!`), but the *intent* of `MissionAssign` is what's described here.
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`crates/services/src/cell/content/executor/mission.rs::accept_or_advance`
(lines 27-100) accepts any `mission_id` whose `space_mgr.mission_defs`
entry exists, with **no check on player level, archetype, faction,
required prior missions, instance flags, or cooldown**. It directly calls
`accept_mission` (which writes the in-memory mission tracker) and emits
`MissionUpdate` for persistence. There is no `prereq_ok(player,
mission_def)` predicate anywhere in the flow. The chain author is the
*only* guard — if a content chain fires `AcceptMission { mission_id: 688 }`
unconditionally, the player gets mission 688 regardless of any gating the
designer might have intended to enforce in the dialog tree.

Combined with **CAT-J-01** (DialogButtonChoice with no open-dialog check),
the practical exploit is: discover a `dialog_id` whose `OnDialogChoice`
chain calls `AcceptMission`, send `DialogButtonChoice(dialog_id, button_id)`
without ever opening that dialog, and the server hands the player the
mission. From there, mission completion through the legitimate objective
path grants the rewards the mission was guarding.

**Evidence**
- Ghidra: `Event_NetOut_MissionAssign` at `0x019b33e0` confirms the client
  emits such a message (8 references); the Rust dispatch has no handler
  for it, but the same validation gap applies to chain-driven accepts.
- Cross-ref to Rust handler: `crates/services/src/cell/content/executor/
  mission.rs:27-100`. The `if let Some(def) = space_mgr.mission_defs.get(
  &mission_id)` is the only precondition.

**Attack scenario**
1. Per **CAT-J-01**, attacker forges `DIALOG_BUTTON_CHOICE(dialog_id=X)`
   for a dialog_id bound to an `OnDialogChoice` chain with action
   `AcceptMission { mission_id: 999 }`.
2. Server fires the chain unconditionally; `accept_or_advance` runs.
3. `mission_defs[999]` exists → mission is accepted, regardless of level,
   prior-mission completion, or faction gating that the chain author
   *would have written into the dialog tree* if they could trust
   `DIALOG_BUTTON_CHOICE` to only fire after the dialog was actually
   shown.

**Suggested remediation (one line)**
Route `accept_or_advance` through a `validate_can_accept(player,
mission_def)` predicate that checks min_level, faction, prereq_mission_ids,
and one-instance ownership; consult mission-systems-advisor on which
fields belong on `MissionDef`.

**Would benefit from x64dbg trace?**
No — the gap is fully visible in Rust.

---

### CAT-J-05 — ChosenRewards is unimplemented; reward selection is missing entirely

**Severity**: High (when wired up; latent today)
**Class**: Missing handler (latent server-authority gap)
**Wire surface**: `Event_NetOut_ChosenRewards` (cell method 87, `chosenRewards`)
**Demonstrable / Likely-theoretical**: Likely-theoretical (the bug is the
absence, plus the shape a naïve implementation would take)

**Trust violation**
`CHOSEN_REWARDS` is in the SGWPlayer constant table (`cell_methods/player/
constants.rs:25`) but the handler in `world/mod.rs:202-205` just logs
`UNIMPLEMENTED: chosenRewards` and returns `true`. The intent of the
message — based on the name + the slash-cmd absence (no `Event_SlashCmd_
ChosenRewards`, so this is purely from the mission-turn-in UI) — is to
let the client tell the server "I picked reward N from the offered list."

This is the classic "client picks index, server hands out item" trust
violation. When the handler is wired up, the *only* secure shape is:
the server remembers what reward set was offered to this player for this
mission turn-in (server-authoritative offered list), and the client's
choice is validated as an index into *that* list — not as an item_id
the client supplied. A naïve "client sends item_id, server grants
item_id" path is a dupe + cross-mission reward pick exploit.

Filing now to lock in the design constraint before the implementation
PR ships.

**Evidence**
- Ghidra: `Event_NetOut_ChosenRewards` at `0x019baed4` (string ref) +
  `0x00d67c10` (server-side handler vfunc_0) — client→server direction
  confirmed by the `register_NetOut_*` convention.
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/
  player/world/mod.rs:202-205` (`UNIMPLEMENTED: chosenRewards`).

**Attack scenario**
1. (Latent — requires the handler to be wired up.) Implementation PR adds
   `CHOSEN_REWARDS` arm that takes `(item_id: i32)` or `(reward_index:
   i32)` from the client and grants the corresponding reward.
2. If keyed by item_id: attacker sends any item_id from the canonical
   reward pool — bypasses single-pick scoping, gets every reward.
3. If keyed by index but the offered-list isn't pinned per-player +
   per-mission server-side: attacker can submit an index against a
   different (richer) mission's reward list.

**Suggested remediation (one line)**
Pin per-(player, mission) offered-reward state server-side at
turn-in-dialog-open time, key `ChosenRewards` by an index into THAT list,
and clear the pin after the first successful choice (or on dialog
cancel).

**Would benefit from x64dbg trace?**
Yes — capture an actual `ChosenRewards` packet from a stock client to
confirm whether the field is `item_id`, `reward_index`, or a richer
multi-pick array. Determines whether the eventual handler is single-
or multi-pick.

---

### CAT-J-06 — Missionary.SHARE_MISSION / SHARE_MISSION_RESPONSE are unimplemented but accept payloads

**Severity**: Medium (when wired up; latent today)
**Class**: Missing peer / group / consent validation (latent)
**Wire surface**: `Event_NetOut_ShareMission`,
`Event_NetOut_ShareMissionResponse` (Missionary cell methods 53, 54)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`crates/services/src/cell/cell_methods/missionary.rs` parses the payload
(`mission_id: i32` for SHARE_MISSION, `choice: i8` for SHARE_MISSION_
RESPONSE) but only logs `UNIMPLEMENTED: shareMission` /
`UNIMPLEMENTED: shareMissionResponse`. The wire shape — and the absence
of a target player_id in the payload as currently parsed — is suspect:
the client almost certainly sends a target (group member to share with);
the Rust handler doesn't read it. When wired up the requirements are:
(1) verify the sharer **owns** the mission (`get_mission(mission_id)` !=
None), (2) the mission is **shareable** (mission_def flag),
(3) the target is in the sharer's **group/squad** (server-tracked
membership, not a client-supplied target id), (4) the recipient must
**accept** (not auto-accept) and the recipient must independently
**pass prereqs** before AcceptMission fires.

This pre-files the design constraints so the implementation PR has a
checklist.

**Evidence**
- Ghidra: `Event_NetOut_ShareMission` and `Event_NetOut_ShareMissionResponse`
  present in `Event_NetOut_Mission*` enumeration at `0x019b347c` /
  `0x019b34bc` area (8 refs each — class + emit-info + handler vfunc).
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/
  missionary.rs:27-40` (UNIMPLEMENTED logs).

**Attack scenario**
1. (Latent.) Implementation PR wires up `SHARE_MISSION(mission_id,
   target_player_id)`.
2. If `target_player_id` is client-supplied and not validated against
   server-side group membership: attacker can push a quest pop-up
   onto any online player (chat spam / griefing) regardless of group
   relationship.
3. If recipient's `SHARE_MISSION_RESPONSE` auto-accepts on a yes-choice
   without recipient-side prereq validation: cross-faction mission
   assignment, level skip, etc.

**Suggested remediation (one line)**
Verify ownership server-side (sharer has the mission), validate target
against server-side group membership (not client-supplied target id),
require recipient explicit consent, and re-run prereq validation on the
recipient before `accept_or_advance` fires.

**Would benefit from x64dbg trace?**
Yes — capture the actual payload to confirm whether SHARE_MISSION
carries a target_player_id or whether the server is supposed to broadcast
to the sharer's whole group.

---

### CAT-J-07 — abandon_mission has no DB cleanup; mission re-materialises on relog

**Severity**: Low
**Class**: State inconsistency between in-memory tracker and persisted row
**Wire surface**: `Event_NetOut_MissionAbandon` (Missionary method 52)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
`crates/services/src/cell/missions.rs::abandon_mission` removes the
`MissionInstance` from the player's in-memory `Missions` table and emits
an `ON_MISSION_UPDATE` wire frame with `STATUS_COMPLETED` so the client
log clears. It does **not** send a `CellToBaseMsg::MissionUpdate` or a
`MissionDelete` to base, so the `sgw_mission` DB row keeps its prior
status (typically MISSION_ACTIVE = 1). On the player's next login,
`query_saved_missions` (`crates/services/src/base/world_entry/methods/
missions.rs:7-76`) re-reads the row and re-seeds the in-memory tracker
— the "abandoned" mission is back.

This isn't a privilege escalation, but it is a server-authority gap: the
client thinks the mission is gone (the wire frame says so), the server's
in-memory tracker agrees, the *DB* doesn't, and the post-relog state
silently disagrees with both. A player could use this to effectively
"undo" a dialog choice tree by abandoning a mission they'd progressed
into a dead-end, then relog to get it back at the original step.

**Evidence**
- Ghidra: confirms `Event_NetOut_MissionAbandon` is a client→server
  message; the Rust dispatch routes method 52 to `missionary::dispatch`
  which calls `abandon_mission` with the client-supplied mission_id.
- Cross-ref to Rust handler: `crates/services/src/cell/missions.rs:282-312`
  (no DB-write side; only wire emit). `base/world_entry/methods/missions.rs`
  has no `delete_mission` function.

**Attack scenario**
1. Player accepts mission X at step 1.
2. Player progresses to step 3, finds the rewards aren't as good as
   expected.
3. Player abandons mission X. Server clears in-memory state; DB still
   has `status=1, current_step_id=2`.
4. Player relogs; server re-seeds mission X at the persisted step 2.
   The player is back in the mission *and* has the chain-fired side
   effects from the prior partial progression (counters, dialog sets)
   still applied.

**Suggested remediation (one line)**
On abandon, send a `CellToBaseMsg::MissionDelete { player_id, mission_id }`
that the base bridges to a `DELETE FROM sgw_mission WHERE player_id=$1
AND mission_id=$2`, ensuring the persisted state matches the cleared
in-memory state.

**Would benefit from x64dbg trace?**
No — this is a server-side persistence gap, not a wire-format question.

---

### CAT-J-08 — Mission*/Debug* names fall through the unhandled warn — latent server-authority hole

**Severity**: Medium (latent / advisory)
**Class**: Latent dispatch hole — future "wire it up" PRs inherit no
validation skeleton
**Wire surface**: `Event_NetOut_MissionAdvance`, `MissionComplete`,
`MissionAssign`, `MissionReset`, `MissionClear`, `MissionClearActive`,
`MissionClearHistory`, `MissionList`, `MissionListFull`, `MissionDetails`,
`MissionSetAvailable`, `DebugInteract`, `AbandonMission` (the slash-cmd
variant, distinct from MissionAbandon at method 52)
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Every name above has a `register_NetOut_*` symbol in the SGW client whose
Ghidra-decompiled comment reads `Direction: NetOut (client -> server)`.
None of them has a cell-method dispatch arm in the Rust server today —
they all fall through to `dispatch_cell_method`'s unhandled `warn!` at
`crates/services/src/cell/dispatch/router.rs:101`. That is safe **today**,
but is exactly the bug shape a future "wire up MissionAdvance" PR will
inherit: the implementer writes a handler, parses a `mission_id: i32`
off the wire, and calls `advance_step` or `complete` directly — no GM
gate, no ownership check ("does this player own this mission?"), no
step-validity check ("is the requested step actually the next legal step
for this mission's current state?").

`DebugInteract` is the clearest GM-only candidate by name — when wired up
it MUST be GM-gated (read GM bit from server-side session state, not from
the inbound payload — per CAT-N convention). The Mission*/Clear/Reset
variants are mixed: some are "client requests view" (MissionList,
MissionListFull, MissionDetails — generally safe if they only read), but
**MissionAdvance, MissionComplete, MissionReset, MissionClear,
MissionClearActive, MissionClearHistory, MissionSetAvailable** are all
state-mutating shapes that require server-side validation of (1) GM bit
for the clear/reset/setAvailable family if they're admin-only, and (2)
ownership + step-validity for any per-player advance/complete that's
meant to be reachable from non-GM UI.

Filing as a single finding because the remediation is structural — the
dispatch table needs a pre-implementation review row for each before the
handler is added, not after.

**Evidence**
- Ghidra: `Event_NetOut_Mission*` enumeration at `0x019b33e0–0x019b35f4`
  (12 strings), `Event_NetOut_DebugInteract` at `0x019b3a08`,
  `Event_NetOut_AbandonMission` at `0x019baea4` — all decompile to
  `Direction: NetOut (client -> server)` `register_NetOut_*` functions.
- Cross-ref to Rust handler: no handler; falls through
  `crates/services/src/cell/dispatch/router.rs:101` unhandled-warn arm.

**Attack scenario**
Latent. If a future PR wires up `MissionAdvance(mission_id, step_id)`
without ownership + step-validity validation, attacker sends arbitrary
`(mission_id, step_id)` pairs to skip directly to mission completion
steps, claim rewards, fire `OnMissionCompleted` chains for missions
they never started.

**Suggested remediation (one line)**
Before wiring any of these up, add a row in
`docs/protocol/client-method-dispatch-table.md` naming the server-side
validation each requires; for `DebugInteract` + the Mission*Clear /
Reset / SetAvailable family, gate on
`session.gm_flag` read from server-side session state (not from the
inbound packet) per CAT-N convention.

**Would benefit from x64dbg trace?**
Yes — capture each payload from a known-good (GM-flag-on) client to
pin the field layouts before the implementation PR lands; otherwise
the implementer guesses and the wire format ends up undocumented.

---

### CAT-J-09 — fire_dialog_choice/fire_dialog_open populate context with chain-author-trusted player_id but no entity-vs-player linkage check

**Severity**: Low
**Class**: Trust-context shape (defensive, not directly exploitable today)
**Wire surface**: `Event_NetOut_DialogButtonChoice` (75),
`Event_NetOut_Interact` (74) downstream of dialog open
**Demonstrable / Likely-theoretical**: Likely-theoretical

**Trust violation**
Both `fire_dialog_choice` and `fire_dialog_open` (`crates/services/src/
cell/content/event_dispatch/dialog.rs:24-111`) take `player_id` as a
parameter and pass it as the chain `ExecutionContext`'s source binding,
**but the call sites read `player_id` from `space_mgr.get_entity(entity_
id).player_id` and fall back to `0`** (`interaction.rs:243-246`,
`interaction.rs:218-221`). Falling back to `0` would attribute chain
side-effects (`GrantItem`, `IncrementCounter`, `AcceptMission`) to a
non-existent player. The same `Some/None` pattern is checked in
`handle_initial_response` (`interactions/dispatch.rs:231-242`) which
warn-and-returns when player_id is None — but the DIALOG_BUTTON_CHOICE
and INTERACT arms in `cell_methods/player/interaction.rs` use
`.unwrap_or(0)` instead.

This is the same bug shape PR #105 + Copilot caught for `handle_initial_
response` (regression test at `interactions/dispatch.rs:593-632`) — the
fix wasn't propagated to the DIALOG_BUTTON_CHOICE / INTERACT call sites.
A future test or code path that lets a non-player CellEntity reach
`DIALOG_BUTTON_CHOICE` (e.g. some NPC AI control path forwarded to the
dialog choice handler in error) would attribute chain side-effects to
player_id=0. Today that path doesn't exist, so this is a defense-in-depth
finding.

**Evidence**
- Cross-ref to Rust handler: `crates/services/src/cell/cell_methods/
  player/interaction.rs:243-246` (`unwrap_or(0)` for DIALOG_BUTTON_CHOICE),
  `:218-221` (`unwrap_or(0)` for INTERACT → fire_dialog_open).
- Companion fix already in place: `crates/services/src/cell/interactions/
  dispatch.rs:231-242` (warn-and-return for missing player_id) — same
  shape, applied at `handle_initial_response` only.

**Attack scenario**
Not directly exploitable today (the only entity that reaches the
SGWPlayer cell-method range is a CellEntity with `is_player = true` and
`player_id = Some`). Filing as a defense-in-depth alignment with the
PR #105 fix.

**Suggested remediation (one line)**
Replace `.unwrap_or(0)` with the same `Some(id) => id, None => warn! +
return` pattern used in `handle_initial_response`, in both the
DIALOG_BUTTON_CHOICE and INTERACT (`fire_dialog_open`) call sites.

**Would benefit from x64dbg trace?**
No — pure code-shape alignment.

---

## Not Filed

- **Dialog button_id is unvalidated** — folded into **CAT-J-01**. The
  separate exploit shape would be "send a button_id that wasn't in the
  rendered dialog," but since OnDialogChoice's only built-in matcher is
  `dialog_id` (not button_id), the practical impact is identical to
  CAT-J-01: any chain bound to the dialog fires. Worth noting once that
  the button_id is just passed into chain params and any chain
  *condition* that reads it is the only thing that can validate it.

- **`MAX_INTERACT_DISTANCE = 5.0` could be tightened / spec'd from
  Ghidra** — the value is sourced from a comment referencing python
  `common/Constants.py: MAX_INTERACT_DISTANCE = 5`, not from Ghidra.
  Spec verification needed but not an authority gap.

- **Range check uses `distance_squared_to(&target_pos).sqrt()`** —
  could be optimized to `< MAX_INTERACT_DISTANCE * MAX_INTERACT_DISTANCE`
  without the sqrt; not a security issue.

- **Negative target_entity_id in INTERACT is dropped with a warn** —
  `interaction.rs:30-39`. That's already a defensive check the code does
  right; calling it out only as a positive example for other handlers
  that *don't* do this.

- **abandon_mission doesn't validate ownership before calling
  remove_mission** — looked at this. `remove_mission` is keyed on the
  player's own missions HashMap, so a mission_id the player doesn't own
  is a silent no-op. No privilege escalation; just a stale-target
  log-noise opportunity. Not filed.

- **`MissionList` / `MissionListFull` / `MissionDetails` falling through
  unhandled** — these are read-only inquiry shapes (the client wants
  the server to send back its mission state). Folded under **CAT-J-08**
  as part of the dispatch-table review. The latent risk is much smaller
  than the state-mutating variants, but they share the same
  pre-implementation review row.

- **The `MissionUpdate` channel send is best-effort (`if let Err(e) =
  tx.send(...)`)** in `accept_or_advance` and `complete`. A channel
  closure means the chain has run + in-memory state is mutated but
  persistence didn't queue. That's a data-loss/consistency bug shape
  but not an exploit — the comment at `executor/mission.rs:78-92`
  documents the design ("in-process state is authoritative"). Not
  filed; sits in the same category as **CAT-J-07** (state-consistency
  gaps that aren't security findings).

- **DialogButtonChoice + InitialResponse have no rate limit** — minor
  spam vector (each event reads the chain table and may trigger a
  GrantItem/GrantXP). Not filed because the underlying issue is
  CAT-J-01's "no precondition" — a rate limit on a wide-open handler
  treats the symptom not the cause.

- **Chain replay (sending the same DialogButtonChoice twice quickly)** —
  if the chain has a `mission_completed N` guard that uses
  `transitioned_from_active` (mission/executor.rs:130-179), the second
  call won't re-fire. But for chains with no such guard (e.g. simple
  `IncrementCounter` actions), the second call **does** double-fire.
  Folded under CAT-J-01: same root cause (no open-dialog tracking +
  no idempotency on the choice).
