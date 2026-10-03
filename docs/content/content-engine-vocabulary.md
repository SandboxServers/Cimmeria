---
title: "Content engine — vocabulary"
type: reference
audience: engineers
last_updated: 2026-10-03
---

# Content engine — vocabulary

The triggers, conditions and actions a content chain is written in. Split out of [content-engine.md](content-engine.md) §3 on 2026-10-03, text unchanged; that file keeps the architecture, execution model, schema, lifecycle and tests.

## Triggers, conditions and actions

### Triggers — *what fires the chain*

Defined at [triggers/mod.rs:28-146](../../crates/content-engine/src/triggers/mod.rs#L28-L146). Filterable by an optional second key (entity type, item id, region key, etc.) per variant. The DB `event_type` string each variant is authored as lives in [loader/trigger.rs](../../crates/content-engine/src/loader/trigger.rs) — only variants with a match arm there are reachable from seed data.

| Variant | Fires when |
|---|---|
| `OnEntityCreated { entity_type? }` | Entity spawns (filterable by template-string type) |
| `OnEntityDestroyed { entity_type? }` | Entity removed |
| `OnEntityDeath { entity_type?, entity_tag? }` | Entity dies; tag wins over type when both set |
| `OnEntityHealthBelow { entity_tag, pct }` | A tagged entity's health crosses `pct` **downward** on a single damaging hit. Seed `event_key` is `"<tag>:<pct>"` (e.g. `"Rinla_Malac:30"`), parsed with `rsplit_once(':')` so a tag containing a colon still resolves; `pct` must be `1..=99` or the trigger row is dropped with a `health_pct_out_of_range` warn ([loader/trigger.rs](../../crates/content-engine/src/loader/trigger.rs)) — 100 is excluded because the band test's upper half is strict, so a full-health entity can never satisfy `before > 100` and `:100` would load a chain that never fires. See the band-test note below |
| `OnAbilityUsed { ability_id? }` | Any entity uses an ability |
| `OnInteraction { interaction_type? }` | Generic right-click |
| `OnRegionEnter { region_key }` | Player enters a Kismet region (string key like `Castle_CellBlock.Region2`). Also **replayed by the server** when a mission step activates with the player already standing in the volume — see "Step-activation replay" below |
| `OnRegionExit { region_key }` | Player exits region |
| `OnMissionStep { mission_id, step }` | Mission advances to a specific step |
| `OnItemAcquired { item_id? }` | Item enters inventory |
| `OnTimer { timer_name }` | Named timer expires (defined; see §10) |
| `OnCustomEvent { event_name }` | Generic invoke escape hatch / synthetic for triggerless chains |
| `OnPlayerLoaded { world_name? }` | Player completes mapLoaded |
| `OnDialogOpen { dialog_id }` | Server sent `onDialogDisplay` |
| `OnDialogChoice { dialog_id }` | Player clicked a dialog button. **Server-gated**: the `DialogButtonChoice` handler rejects the event unless `dialog_id` is in `CellEntity::offered_dialog_ids` — the bounded set of dialogs actually displayed to this player via `send_dialog_display`. A valid choice removes the id (one-shot), so a forged or replayed choice is dropped with a `warn!` and never fires the chain (CAT-J-01 / #479, widened to a set by DU-08 because the client evicts an open dialog and answers it late). A close for a `DUIST_DefaultTutorial` dialog the client opened by itself (5863, the inventory help) is rejected the same way but logged at DEBUG, since it is expected client behaviour rather than a forgery. |
| `OnInteractTag { entity_tag }` | Right-click on tagged NPC/object |
| `OnInteractTemplate { template_name }` | Right-click on entity from named template |
| `OnItemUse { item_id }` | Player double-clicked inventory item |
| `OnItemEquipped { item_id? }` | Player moved a stack into the bandolier (`container_id = 3`) from any other container. `item_id` is the design / `type_id`, not the inventory instance id; `NULL` `event_key` is a wildcard that fires for any equip |
| `OnTeleportIn { region_id }` | Player arrived via ring transporter |
| `OnStargateDialed { destination_world? }` | Player successfully dialled a stargate — fired from `handle_dial_gate` when the four-second gate-open timer is armed. `event_key` is the destination world name (`resources.worlds.world`, e.g. `Harset`); `NULL` is a wildcard that fires for any destination |
| `OnStargateCrossed { destination_world? }` | Player stepped through an open stargate, fired immediately before the world transition tears the cell entity down. Same `destination_world` filter as `OnStargateDialed` |
| `OnEffectInit / PulseBegin / PulseEnd / Removed` | Effect lifecycle hooks (unit variants) |
| `OnMissionCompleted { mission_id }` | Mission marked complete |
| `OnDialogSetOpen { dialog_set_name }` | Dialog set opened |
| `OnMissionAccepted { mission_id }` | Mission just accepted or advanced (fired from the executor's combined `Action::AcceptMission \| Action::AdvanceMission` branch after the cell-side state commit; used by chains that highlight quest objects on mission start — e.g. chain 1097 for Aftermath) |
| `OnMissionAbandoned { mission_id }` | Mission just abandoned, fired **after** the instance is removed so `mission_status <id> eq not_active` already holds. Seed `event_type` = `mission_abandoned`, `event_key` = the mission id; there is no wildcard. Fires from all three abandon paths — the client-callable `abandonMission` cell method (Missionary index 52), the `abandon_mission` chain action, and `gmMissionClear` / `gmMissionAbandon` — and only when a mission was really removed. Used by offer chains that must repaint their giver and clear a stranded dialog-set bind: Harset chains 6308 (1324), 6342 (1326) and 6120 (742) |
| `OnPlayerEnteredCover { cover_set_id? }` | Player entered proximity of a cover set (`resources.cover_sets`). One event per set; a player can be in several at once. Wildcard (`NULL`) fires for any set. Also **replayed by the server** when a mission step activates with the player already in the set — see "Step-activation replay" below |
| `OnPlayerLeftCover { cover_set_id? }` | Player left a cover set's proximity — the symmetric partner of `OnPlayerEnteredCover` |
| `OnPlayerInCoverDuration { cover_set_id?, seconds }` | Player has been continuously in a cover set for ≥ `seconds`. Debounced: leaving and re-entering resets the timer. Seed `event_key` convention is `"<seconds>"` or `"<seconds>:<set_id>"` ([loader/trigger.rs:87-100](../../crates/content-engine/src/loader/trigger.rs#L87-L100)) |
| `OnNpcFlanked { npc_template? }` | An NPC occupying a cover slot was flanked — its top-threat target moved outside the cover's defensive arc (orientation ± π/2) |
| `OnPlayerFlankedNpc { npc_template? }` | Player-perspective twin of `OnNpcFlanked`, fired from the same AI decision when the top-threat is a **player**. Unlike `OnNpcFlanked` (actions run on the NPC with player id 0), the chain's actions execute against the flanking player with that player's mission context, so mission-scoped chains (`objective_status`, `complete_objective`) work. Seed `event_type` = `player_flanked_npc`, `event_key` = the NPC template name (`entity_templates.template_name`, e.g. `NID Guard`) or `NULL` for any. Used by the Castle Cellblock flank objectives 2725/2731 (chains 1141/1142) |

Within a single chain's bucket, `Trigger::matches` ([triggers/matching.rs:43](../../crates/content-engine/src/triggers/matching.rs#L43)) decides whether the event matches the chain's specific trigger variant + filter. Bucketing is by **`TriggerType` discriminant** — see §6.

**`entity_health_below` is a stateless band test.** The damage path emits **one event per damaging hit**, carrying the target's health percentage before and after the hit; `Trigger::matches` fires the chain iff `pct_before > pct && pct_after <= pct`. Nothing tracks which thresholds have already been crossed, so the cost is one event and one hash lookup per hit no matter how large the hit was, and the dispatch site stays independent of what content seeded. Consequences worth knowing before you author one:

- A hit from 60% to 40% fires a `:50` chain once; the next hit, 40% → 25%, does not (`pct_before > 50` is false). Healing back above the threshold re-arms it.
- Two thresholds on one tag are independent bands: a hit spanning both fires both.
- **`:100` is refused at load, not silently unmatchable.** The upper half of the band test is strict, so a full-health entity (`pct_before == 100`) never satisfies `pct_before > 100`, and every later hit starts from below it. `:0` is the mirror image — an entity at 0% is dead and routes to `entity_dead_tag`. The loader drops both with a `health_pct_out_of_range` warn rather than registering a chain that reads as wired and never runs.
- **A killing blow never fires it.** The suppression is at the dispatcher (`fire_health_below_for_hit`), not in the predicate — a `31% → 0%` hit satisfies the band test, so the dispatcher drops any hit whose target ends dead. Deadness is read from `BSF_DEAD` rather than `health.cur <= 0`, because an effect script runs after the damage path and can heal a corpse back above zero. The kill goes to `entity_dead_tag` instead; a chain author gets exactly one of the two per hit.
- **Every player damage path fires it**, not just single-target shots: AoE and cone secondaries, damage-over-time pulses, and the `apply_effect` content action all cross the same two health-application seams. `pct_before` is sampled at the seam and queued on the `SpaceManager`; the callers that hold a `ChainEngine` drain the queue right after the hit (`fire_pending_health_below`), with a per-tick safety drain in the cell loop as a backstop. A DoT on a threshold-gated NPC is therefore safe — before this landed it silently and permanently disarmed the chain, because once the tick carried the target past the threshold no later hit could satisfy `pct_before > pct` again.

**Not reachable from seed data.** `OnEntityCreated`, `OnEntityDestroyed`, `OnAbilityUsed`, `OnInteraction`, `OnMissionStep`, `OnItemAcquired`, and `OnTimer` have no match arm in [loader/trigger.rs](../../crates/content-engine/src/loader/trigger.rs), so no `content_triggers` row can bind them — a chain authored with those `event_type` strings is dropped with a `warn!`. `OnCustomEvent` has no arm either but is generated internally, as the synthetic `__direct_invoke_<id>` trigger for chains with zero trigger rows (§6). `OnEntityDeath` is reachable only through the `entity_dead_tag` (tag-filtered) form; there is no `event_type` that binds the `entity_type` form.

**Authorable but never dispatched — the other half of the gap.** Five trigger types have a loader arm (so a `content_triggers` row binds cleanly and the chain registers) but **no `fire_*` site anywhere in the cell service constructs their `TriggerType`**, so they can never fire:

| Seed `event_type` | Variant | Why it matters |
|---|---|---|
| `dialog_set_open` | `OnDialogSetOpen` | The 2009 scripts used `dialog_set.open::<id>` as the "player interacted with a bound NPC" hook — mission 742's bug-planting step is written against it. Port that shape to `interact_tag` instead |
| `effect_init` | `OnEffectInit` | No effect-lifecycle dispatch exists |
| `effect_pulse_begin` | `OnEffectPulseBegin` | " |
| `effect_pulse_end` | `OnEffectPulseEnd` | " |
| `effect_removed` | `OnEffectRemoved` | " |

This is the trigger-side mirror of the action-side gap catalogued below, and it is the reason `apply_effect`'s one seeded row cannot fire: the row sits on an `effect`-scoped chain whose trigger is one of these.

### Step-activation replay of `enter_region` and `player_entered_cover`

`enter_region` is an **edge** event. The client reports a volume crossing once, and a chain gated on a step that is not yet active sees that edge, fails its gate, and never gets another one until the player physically leaves and comes back. The 2026-09-18 Castle playtest lost objective 2484 to this ordering race, and the Harset seed lanes found four more instances of the same shape.

The server closes the `enter_region` half of it. Whenever a mission step activates — `Action::AcceptMission` / `Action::AdvanceMission` (first step), `Action::AdvanceStep`, `gmMissionAssign`, `gmMissionAdvance` — every client-hinted region of the player's world that contains the player's **server-known** position is re-fired through the normal trigger path. Implementation: [`content::event_dispatch::step_activation`](../../crates/cell-content/src/cell/content/event_dispatch/step_activation/mod.rs). Log line: `reason = "already_inside_on_step_activation"`, plus a `region_replay` entry in the player journal that a `.bug` report picks up.

What an author needs to know:

- **Only mission-gated chains are replayed.** A chain is eligible when at least one of its conditions is `mission_status`, `step_status` or `objective_status` (`Chain::is_mission_gated`). Those are idempotent under a double delivery: the chain's own actions move the state its gate reads, so when the client's real hint lands a moment later the gate is closed. A chain with no mission gate — a bare `enter_region` → `display_dialog` — is **refused** and logged at `debug` with `reason = "filtered_out"`, because replaying it would show the dialog twice. If your chain should replay, give it the `step_status` gate it wanted anyway.
- **`world` and `archetype` are not mission gates.** Neither changes when the chain runs, so neither makes a re-fire safe. A chain gated only on `world` is not replayed.
- **Containment is the same test the client hint uses** (`spawner::is_point_in_region`, the tolerance band including its vertical arm), against the position the *server* accepted — never a client-supplied coordinate.
- **Only client-hinted volumes replay.** A region without `REGION_FLAG_CLIENT_HINTED` was never handed to the client, so there is no hint to stand in for.
- **Content chains only.** Ring-transporter forwarding and `REGION_FLAG_STARGATE` passage hang off the same client call but are sequenced by the dispatch arm in `cell_methods::player::world`, *after* `fire_enter_region`. A replay never starts a ring transport and never carries a player through a gate.
- **Bounded.** A replayed chain can itself advance a step, which activates another step, which replays again. A depth cap plus a per-activation visited set of `(entity, mission, step)` triples stops the recursion; a refusal is a `warn!` naming `replay_depth_exceeded` or `step_already_replayed`, which reads as "this chain is looping".

**`player_entered_cover` replays too.** It is the same edge from a different source — the 1 Hz cover-detection tick rather than a client hint — and objective 2484 was this form. On 2026-09-20 it reproduced on the colo: the Ambernol vial sits inside the med-station desk's 5 m cover radius (set 1381 then, 1200001 since the NA21 cover re-extraction), so the tick spent the enter edge 1.4 s before picking the vial up activated step 2144, chains 1132 / 1133 failed their step gate, and the player stood on the "take cover" marker with the drone dead and nothing happening. The same step-activation hook now re-fires `player_entered_cover` for each cover set the player is in. Implementation: [`step_activation::cover_replay`](../../crates/cell-content/src/cell/content/event_dispatch/step_activation/cover_replay.rs). Log line: `step-activation cover replay: matched` with the same `reason`, plus a `cover_replay` journal entry. It differs from the region form in one way:

- **It replays from the detection table, not from position.** A set in the table has already had its enter edge; a set the player walked into since the last tick has not, and the tick delivers that one itself with the step already active. Replaying only the table's sets covers exactly the edges that cannot recur and never races the tick into a double fire. Containment is still re-checked against the server-known position before each fire, so a player who walked out since the last tick is not credited.

The mission-gated-only rule, the `world` / `archetype` rule and the recursion bound apply unchanged. `player_left_cover` and the cover-duration milestones are **not** replayed.

`player_loaded` is **not** replayed and keeps the seed-side second-trigger rule; the abandon case has its own trigger, `mission_abandoned`, rather than a replay.

### The abandon edge

Abandoning a mission returns it to not-active with the player already past every edge that set the scene up. The offer gate reopens and the `player_loaded` chain that paints the offer has no edge left to fire, and whatever dialog-set binding the mission installed is stranded on its NPC — abandoning Harset's mission 1324 in the Command Center leaves Ba'al with a stale marker that replays the council dialog on click. Both self-heal on the next world transition, which is why the gap went unreported for so long.

`OnMissionAbandoned` closes it. The dispatcher lives beside `fire_mission_accepted` / `fire_mission_completed` in [`content::event_dispatch::mission`](../../crates/cell-content/src/cell/content/event_dispatch/mission.rs) and follows the same contract: world, archetype and mission context are populated **after** the mutation, so a repaint chain carries the offer chain's own `mission_status <id> eq not_active` gate verbatim. Populating before the removal would leave the status `active` and every repaint chain would fail closed.

Authoring notes:

- Gate the repaint chain on the **world where the stale state is observable**, usually the one holding the NPC. An abandon from anywhere else needs nothing: `available_interactions` is rebuilt empty on every world entry and the offer chain's `player_loaded` row repaints on the way back in.
- **Unbind before you rebind.** The interact dispatcher takes the first bound entry on a template that carries a dialog, so a `remove_dialog_set` ordered after the `add_dialog_set` leaves the NPC handing out the old dialog. `remove_dialog_set` on a slot that holds nothing is a safe no-op, so clear every bind the mission could have had live.
- The step is gone by the time the chain runs, so a repaint cannot tell which step was active. Clear all the candidates rather than trying to choose.

### Conditions — *gates that AND together*

Defined at [conditions.rs:12-95](../../crates/content-engine/src/conditions.rs#L12-L95). All conditions on a chain are AND'd ([chain.rs:161](../../crates/content-engine/src/chain.rs#L161)). For OR, author multiple chains.

| Variant | Predicate |
|---|---|
| `PropertyEquals { property, value }` | `ctx.params[property] == value` |
| `PropertyInRange { property, min, max }` | numeric in `[min, max]` |
| `HasItem { item_id, min_count? }` | reads `item_<id>_count` from ctx — **populator missing today; see §10** |
| `HasAbility { ability_id }` | reads `ability_<id>` bool |
| `InRegion { region_id }` | reads `current_region` |
| `FactionCheck { faction, relation }` | reads `faction_<name>` — **populator missing today** |
| `MissionStatus { mission_id, op, expected }` | `not_active` / `active` / `completed`; missing key defaults to `not_active` ([conditions.rs:194](../../crates/content-engine/src/conditions.rs#L194)) |
| `StepStatus { mission_id, step_id, op, expected }` | three-state per step |
| `ObjectiveStatus { mission_id, objective_id, op, expected }` | string compare on objective state |
| `Archetype { op, archetype_id }` | reads `archetype` i64 |
| `Counter { counter_name, op, value }` | reads `counter_<name>` |
| `StatBelowMax { stat_id }` | `stat_<id>_cur < stat_<id>_max`. **Fail-closed** on missing params ([conditions.rs:255-268](../../crates/content-engine/src/conditions.rs#L255-L268)) |
| `CustomExpression { expression }` | bool-key lookup, escape hatch |
| `World { op, world_id }` | `ctx.world_id == world_id` (`eq`/`neq` only; ordered operators never match). Reads the typed `ExecutionContext.world_id`, not a param key. **Fail-closed** when unset — unlike the mission conditions, which fall back to `not_active` and can fail *open* |
| `EntityTagState { tag, op, expected }` | Is any **living** entity in the acting player's space carrying spawn tag `tag`? `expected` is `alive` or `dead`; `eq`/`neq` only. Reads the typed `ExecutionContext.live_tags` set, which `populate_world_context` fills from `SpaceManager::live_tags_in_space_of` (a corpse with `BSF_DEAD`, zero health, a despawned entity and a tag that never spawned all read as dead). **Fail-closed** when unset. The *state* counterpart of the `entity_dead_tag` event, for backstops that must see a kill made before the chain was live (the Cellblock controllers 1181-1190, Decision (@Cadacious, 2026-09-28)). Use it only for tags the space spawns at creation: a tag a content action spawns later reads as dead until then |

**Only eight are authorable.** [loader/condition.rs](../../crates/content-engine/src/loader/condition.rs) has match arms for exactly `mission_status`, `step_status`, `archetype`, `objective_status`, `counter`, `stat_below_max`, `world` (`target_id` = `resources.worlds.world_id`, `operator` = `eq`/`neq`; `target_key` and `value` unused), and `entity_tag_state` (`target_key` = the spawn tag, `value` = `alive` or `dead`, `operator` = `eq`/`neq`; a malformed row still loads, as a condition that never matches, so it cannot publish its chain ungated). The other seven variants (`PropertyEquals`, `PropertyInRange`, `HasItem`, `HasAbility`, `InRegion`, `FactionCheck`, `CustomExpression`) cannot be named by a `content_conditions` row at all — a seed row using them is dropped with a `warn!`. `HasItem` and `FactionCheck` are doubly dead: even reached from Rust, no populator writes the `item_<id>_count` / `faction_<name>` keys they read (§9).

### Actions — *side effects*

Defined at [actions.rs:20-323](../../crates/content-engine/src/actions.rs#L20-L323). **`Action::execute` is a stub** ([actions.rs:363-376](../../crates/content-engine/src/actions.rs#L363-L376)); only `TriggerChain` self-executes. Everything else is dispatched by [executor/mod.rs](../../crates/cell-content/src/cell/content/executor/mod.rs).

An action has to clear **two** hurdles to do anything. It needs a match arm in [loader/action.rs](../../crates/content-engine/src/loader/action.rs) (otherwise no `content_actions` row can name it) *and* a match arm in [executor/mod.rs](../../crates/cell-content/src/cell/content/executor/mod.rs) (otherwise it resolves and then falls through to a `debug!` no-op at [mod.rs:453-455](../../crates/cell-content/src/cell/content/executor/mod.rs#L453-L455)). The table below is the authoritative catalog; the "Seed rows" column counts `content_actions` rows across [db/resources/Content/Seed/](../../db/resources/Content/Seed/) as of 2026-07-25.

#### Authorable and executed

| Seed verb | `Action` variant | Seed rows |
|---|---|---|
| `accept_mission` | `AcceptMission` | 49 |
| `complete_mission` | `CompleteMission` | 17 |
| `abandon_mission` | `AbandonMission` | 1 |
| `advance_step` | `AdvanceStep` | 23 |
| `complete_objective` | `CompleteObjective` | 2 |
| `display_dialog` | `DisplayDialog` | 33 |
| `add_dialog` | `AddDialog` | 10 |
| `add_dialog_set` | `AddDialogSet` | 6 |
| `remove_dialog_set` | `RemoveDialogSet` | 2 |
| `npc_bark` | `NpcBark` | 3 |
| `add_item` | `GrantItem` | 14 |
| `remove_item` | `RemoveItem` | 2 |
| `grant_xp` | `GrantXP` | 0 |
| `change_stat` | `ChangeStat` | 3 |
| `increment_counter` | `IncrementCounter` | 9 |
| `reset_counter` | `ResetCounter` | 3 |
| `play_sequence` | `PlaySequence` | 15 |
| `set_interaction_type` | `SetInteractionType` | 70 |
| `set_visible` | `SetVisible` | 1 |
| `destroy_entity` | `DestroyTaggedEntity` | 1 |
| `spawn_entity` | `SpawnEntity` | 0 |
| `despawn_entity` | `DespawnEntity` | 0 |
| `generate_threat` | `GenerateThreat` | 3 |
| `set_aggression` | `SetAggression` | 1 |
| `set_npc_poi` | `SetNpcPoi` | 0 |
| `set_follow_target` | `SetFollowTarget` | 0 |
| `set_npc_ai_state` | `SetNpcAiState` | 0 |
| `move_waypoint` | `MoveWaypoint` | 5 |
| `move_entity` | `MoveEntity` | 7 |
| `set_active_slot` | `SetActiveSlot` | 0 |
| `start_minigame` | `StartMinigame` | 4 |
| `trigger_transporter` | `TriggerTransporter` | 2 |
| `cross_world_teleport` | `CrossWorldTeleport` | 1 |
| `launch_ability` | `LaunchAbility` | 3 |
| `apply_effect` | `ApplyEffect` | 1 |
| `grant_stargate_address` | `GrantStargateAddress` | 1 |
| `send_system_mail` | `SendSystemMail` | 1 |
| `open_black_market` | `OpenBlackMarket` | 0 |
| `open_loot` | `OpenLoot` | 8 |

`open_black_market` sends `onBMOpen(auctioneerEntityId)` (client method 90)
to open the client's Black Market window. It takes no params: the executor
arm ([`executor/black_market.rs`](../../crates/cell-content/src/cell/content/executor/black_market.rs))
resolves the auctioneer from the interact trigger's `target_entity_id`, then
the player's `last_interaction_target` (the same sources `DisplayDialog`
uses), and aborts with a `warn!` when neither resolves. It then refuses, with a
chat line and `bm.open_refused`, unless that NPC is an auctioneer in the
player's space and within 5 units (BM-07). An auctioneer is an NPC whose
**template** carries `INT_Auction` (mask 4): the bit is read at spawn, so a
`set_interaction_type` on another NPC's tag changes its cursor but never makes
it one. Seed the bit on the auctioneer's template; that also gives the client
its interact cursor. Every open also sends the player a chat line, because a
stock client drops method 90 until the client patch ships — see
[../architecture/black-market.md](../architecture/black-market.md). The one
seeded use is chain 5030, the stasis-room auctioneer
([debug-hub.md](debug-hub.md#black-market-auctioneer-template-305)).

`launch_ability` and `apply_effect` do **not** route through the combat
pipeline. They call a separate server-authoritative entry point,
[`cell/content/effect_apply.rs`](../../crates/cell-content/src/cell/content/effect_apply.rs),
which goes straight to the effect layer. `handle_use_ability` is the *client*
entry point and rejects a scripted debuff three ways — the caster has not
trained the ability, a self-target trips the friendly-fire gate, and the path
resolves unconditionally as damage. The helper deliberately bypasses all
three, so it is private to `cell::content` and takes no client-supplied id;
see [../architecture/abilities-and-effects-system.md](../architecture/abilities-and-effects-system.md)
for the full rationale and the constraints that must not be widened.

`grant_stargate_address` is the port of 2009's Atrea authoring node
`Act_StargateAddress`, and it is the only content verb that writes a
player's stargate address book. `target_id` is
`resources.stargates.stargate_id` — the address itself, not a world id and
not the repeating `address_origin` glyph. The executor arm
([`executor/stargate.rs`](../../crates/cell-content/src/cell/content/executor/stargate.rs))
does three things per grant: appends to the acting player's in-memory
`CellEntity::known_stargates` (which is what the dial handler enforces
against), sends the client `updateStargateAddress` (client method 66, the
only way a mid-session grant becomes visible — the full book is handed over
just once, at map load), and asks the base to persist an idempotent append
to `sgw_player.known_stargates`. A grant for an id with no `stargates` row,
a grant by a non-player actor, and either failed send all warn. A second
grant of an address the player already holds is a complete no-op: no write,
no client method, no base round trip.

`move_entity` and `move_waypoint` are the two repositioning verbs.
`move_entity` with `use_player: true` is a player-facing teleport that goes
through the forced-position snap; with a `target_key` it repositions a
tagged NPC. `move_waypoint` is the space-script spelling of the NPC form —
an instant snap, not a path or an animation: `speed` is parsed from the
seed row but the executor does not use it (see
[proposed-extensions.md](proposed-extensions.md) for the escort-movement
gap this leaves). The snap is broadcast to the moved NPC's current
witnesses immediately as a per-witness `EntityMoved`, so a chain-driven
reposition (escort arrival, tutorial staging) is visible on the next frame
rather than waiting for the next 100 ms AoI tick; witnesses the move
leaves behind get their `LeftAoI` from that same tick.

Three caveats for authors:

- **`add_dialog_set` / `add_dialog` can bind a row that has no dialog.** A
  `dialog_set_maps` row with `dialog_id IS NULL` is an *interaction-only* bind:
  it contributes its `interaction_flags` bit to the per-player indicator over
  the NPC's head (`!`, `?`, quest glow — see
  [interaction-flags.md](interaction-flags.md)) and nothing else. Clicking the
  NPC then displays no dialog; pair the bind with an `interact_tag` chain if
  the click should say something. This works because a bind's only
  client-visible effect is `SGWSpawnableEntity.InteractionType(UINT64 TypeId)`
  ([dispatch table](../protocol/client-method-dispatch-table.md), method 3) — a
  lone flags bitfield with no dialog field, so the dialog id never leaves the
  server. The seed has **626** such rows across every zone; Castle content binds
  seven of them (3062, 3071, 3073, 5828, 5829, 5846, 5863). Before CA02 the
  loader dropped all 626 and every one of those binds was a silent cache miss.
- **`apply_effect` cannot fire today.** Its only seeded row is on an
  `effect`-scoped chain, and no `effect_*` trigger is dispatched anywhere in
  the cell service. The arm is correct and will work as soon as that
  dispatch lands.
- **A single-shot, script-less effect is a legitimate no-op.** An effect with
  `pulse_count = 1` and `script_name = NULL` registers no active instance and
  runs nothing — that is the effect definition's own doing, not a failure of
  the action. Effect 1634, the sole effect on the Castle Cellblock wake-up
  ability 1372, is exactly this shape.

##### `start_minigame` params

`target_key` names the minigame type (`Livewire`, `Hack`, ...). Two params:

| Param | Required | Meaning |
|---|---|---|
| `on_victory_chains` | no (defaults to `[]`) | Chain ids fired when the player wins. They are invoked directly, not through `resolve_event`, so **no conditions on them are evaluated** — put the gate on the launcher chain. |
| `difficulty` | no (defaults to `1`) | Difficulty tier, integer 1-5. |

`difficulty` is **rejected, not clamped**: a row outside 1-5 is dropped at
load time with a `warn!` naming the chain id, so an authoring mistake shows
up as a missing minigame rather than a silently different tier. The 1-5
range is what the original content layer asserted
(`deprecated/python/cell/Minigame.py`). Note that every per-game difficulty
table only has rows 1-4, so an authored `5` reaches the game and is clamped
down to 4 with a `warn!` — 1-4 is the range content should actually use.

A victory chain needs no `content_triggers` row; the loader gives a
triggerless chain a never-firing `OnCustomEvent` placeholder so it stays
reachable only through `on_victory_chains`.

##### `npc_bark` params

A **bark** is a companion line spoken into the triggering player's chat
window with no window to close — Col. Marsh's "Let's move out!" while the
player keeps moving and firing. It exists because the client has no
non-modal dialog path at all: its lowest screen type (`DUIST_None`, 0) is
registered to the modal Blurb window under a "TEMP HACK" comment, and every
other type registers the same modal `DialogWin`. A companion line sent as a
dialog stops the player dead.

Barks ride the one non-modal text route the client honours,
`onPlayerCommunication(Speaker, SpeakerFlags, Channel, Text)` (client method
28 — [dispatch table](../protocol/client-method-dispatch-table.md)), through
the **same serializer the chat broadcaster uses**
([`cell/console/chat/`](../../crates/cell-console/src/cell/console/chat/mod.rs)). Deliberately not
`system_message`, whose wire format is still unknown and whose earlier
attempt at method 28 produced garbled `"[] says"` chat (§10).

`target_id` and `target_key` are both unused. Three params:

| Param | Required | Meaning |
|---|---|---|
| `screen_id` | **yes** | A `resources.dialog_screens` row. The executor resolves the line text server-side, so an author names the shipped 2009 line by id and never retypes it |
| `speaker` | **yes** | The name the chat window prefixes the line with. Explicit rather than a `speakers` lookup because the bark screens carry `speaker_id = 0` |
| `channel` | no (defaults to `say`) | `EChannel` name, case-insensitive. **Only `say` is accepted.** `CHAN_splash` is not: its native trigger has never been traced |

All three are **rejected, not defaulted through** — a bad row is dropped at
load with a `warn!` naming the chain. The failure modes here are not "the
line is missing" but "the line is visibly wrong": a blank `speaker` renders
as the client's empty-name prefix, the server channel (8) opens the client's
modal "Server Message" prompt, and an id the client has no channel for (7)
shows nothing.

The line goes to the **triggering player only** — not the say-chat witness
fan-out and not the sender echo. A bark is per-player mission feedback;
fanning it out would speak one player's escort line into a stranger's chat
window in a shared world.

Three executor refusals, each a `warn!` with a stable `reason`
([executor/bark.rs](../../crates/cell-content/src/cell/content/executor/bark.rs)),
all of which send nothing at all:

1. **`screen_not_cached`** — no `dialog_screens` row for that `screen_id`,
   or the startup cache failed to load.
2. **`empty_text`** — the row exists but its text is blank.
3. **`actor_not_player`** — the chain fired from an NPC, so "the triggering
   player" is undefined and method 28 resolves to no client address.

The text catalogue is `SpaceManager::dialog_screen_text`, loaded once at cell
startup by `spawner::load_dialog_screen_text` (13,467 rows; `screen_id` is
globally unique). It is a startup cache for the same reason
`spawn_entity` caches `entity_templates`: the executor has no DB pool at
action time, and a cell→base round trip mid-chain would break the chain's
ordered action list.

##### `open_loot` params

Opens the corpse loot window on a **live** container, without killing it
(Decision (@Cadacious, 2026-09-28)). Fire it from an `interact_tag` chain on
the container; the executor arm
([`executor/loot.rs`](../../crates/cell-content/src/cell/content/executor/loot.rs))
takes the container from the trigger's `target_entity_id` (else the player's
interaction pin) and requires it in range and in the player's space.

| Column | Meaning |
|---|---|
| `target_id` | The `loot_tables` id to roll. `NULL` never rolls: it reopens this player's pending roll, or says the container is empty. Use it for the fallback chain that answers presses outside the loot gate, so no press is silent |
| `params.once_per_character` | Optional, default `false`. `true` rolls once per character, ever: the key goes into `sgw_player.looted_containers` (via `CellToBaseMsg::ContainerLooted`) when the window opens with loot, and survives relog and respawn |
| `params.container_key` | Optional; defaults to the container's spawn tag. 1-64 printable ASCII characters. Name it explicitly when two tags should share one flag |

The roll is per looter (stored on the container under the player's id), so
two players never share or take each other's roll; without
`once_per_character` every open re-rolls. Archetype splits and mission gates
are ordinary chain conditions. How the window and `lootItem` behave is in
[loot-system.md, Live containers](../gameplay/loot-system.md#live-containers).
A bad param drops the row at load with `reason = "malformed_open_loot"`.

| Event | Target | Level | When |
|---|---|---|---|
| `loot.container_opened` | `loot` | INFO | rolled and opened: `container_entity_id`, `container_key`, `loot_table_id`, `item_count`, `once_per_character` |
| `loot.container_reopened` | `loot` | INFO | a pending roll reopened (`reason=pending_reopened`) |
| `loot.container_refused` | `loot` | INFO | nothing rolled: `reason` = `no_container`, `out_of_range`, `no_container_key`, `nothing_pending`, `already_looted`, `unknown_loot_table` or `empty_roll`; the player gets a chat line |

Each row carries `entity_id`, `account_id`, `player_id` and `chain_id`.

##### `send_system_mail` params

Mails the chain's player one **system mail** (social-systems SS-U3): no
sender character, so it can never be returned, and the cash and the item are
minted, not taken from anyone. The base writes it through the one writer
every server mail uses (`mail::system`, SS-U1), so the player takes the cash
and the item from the mail window like any other attachment. The first user
is the Gate Mail Clerk in the stasis-room debug hub (chain 7011,
[debug-hub.md](debug-hub.md#gate-mail-clerk-template-390)).

`target_id` and `target_key` are unused. Every param is in `params`:

| Param | Required | Meaning |
|---|---|---|
| `sender` | **yes** | The name shown as the sender. One line, 1-128 characters |
| `subject` | **yes** | One line, 1-128 characters |
| `body` | no (empty) | Up to 1,000 characters |
| `cash` | no (0) | Naquadah, `0` to `2147483647` |
| `item_id` | no | A `resources.items` id, minted into the mail's escrow row |
| `qty` | no (1) | Stack size of the item, at least 1. The writer refuses more than the item's `max_stack_size` |
| `cooldown_secs` | no | At least 1. Each player gets at most one mail from this chain per window |

A bad value drops the row at load with a `warn!` naming the chain, the same
reject-not-default rule as `npc_bark`: the base would refuse the same mail
on every firing, and the player would be told about an authoring mistake.
`qty` without `item_id` is also rejected.

The cell does not write the mail. The executor arm
([`executor/mail.rs`](../../crates/cell-content/src/cell/content/executor/mail.rs))
sends the base one `CellToBaseMsg::ContentSystemMail` carrying the player's
ids from the cell entity. The base
([`mail/content.rs`](../../crates/base-methods/src/base/world_entry/methods/mail/content.rs))
then runs one transaction:

1. locks the player's `sgw_player` row;
2. with `cooldown_secs`, claims the cooldown in `sgw_player_content_cooldown`
   (key `send_system_mail/<chain_id>`) with one conditional upsert that
   succeeds only when the last claim is at least `cooldown_secs` old;
3. writes the mail.

The claim and the mail commit together. A refused mail leaves the previous
claim in place, and deleting the mail after taking its attachments does not
reopen the window. The window also survives a relog and a server restart.

This is the one content verb with a built-in per-player limit, and it needs
one, because the "Idempotency" rule in
[extend-the-content-engine.md](../guides/extend-the-content-engine.md) is
about re-runs of one chain, and a dialog button can be pressed again. Author
a `cooldown_secs` on any chain a player can re-trigger at will.

Every firing sends the player one feedback line. A sent mail names the mail
id and what it carries. A refusal gives the time left on the cooldown ("You
can ask again in 7 minutes", rounded up), or the reason. Telemetry:

| Event | Target | Level | When |
|---|---|---|---|
| `content.send_system_mail` `outcome=requested` | `content` | INFO | the cell forwarded the firing |
| `content.send_system_mail` `outcome=sent` | `content` | INFO | the base committed the mail; carries `mail_id`, `item_id`, `cooldown_key` |
| `mail.system_sent` | `mail` | INFO | the writer's own row, after the commit |
| `content.send_system_mail` `reason=...` | `content` | WARN | nothing sent: `cooldown` (with `last_used_at`, `remaining_secs`), `no_player`, `base_channel_closed`, `no_db_pool`, or the writer's reason (`unknown_item_type`, `recipient_not_found`, ...) |

Each row carries `entity_id`, `account_id`, `player_id` and `chain_id`.

#### Entity-lifecycle verbs

`spawn_entity` instantiates an `entity_templates` row into the **acting player's current space**. The seed row never names a space, because a chain authored for a per-player instance cannot know which instance the firing player is in — so the space is read off the triggering entity.

| Column | Meaning |
|---|---|
| `target_id` | `entity_templates.template_id`. Mandatory |
| `target_key` | The spawn tag `entity_dead_tag` / `interact_tag` / `despawn_entity` chains address it by. Mandatory, non-empty |
| `params.x` / `.y` / `.z` | Mandatory and finite. A missing or `NaN` coordinate drops the action rather than spawning at the world origin |
| `params.heading` | Optional, defaults to `0.0` |
| `params.is_stationary` | Optional. No template column exists, so absent means `false`, not "inherit" |
| `params.aggression` | Optional `EMobAggressionLevel` override (1 hostile ... 5 default; `0`, the pre-NA13 "passive", is neutral). Absent means the faction reaction decides, so a faction-10 template is hostile on sight (NA13). A value outside 0-5 is ignored with `reason = "invalid_aggression"` |
| `params.allow_shared` | Optional. `true` opts out of the shared-world refusal below |
| `params.respawn_secs` | **Not a parameter.** A row that supplies it still loads and still spawns; the loader warns once (`respawn_secs_not_honoured`) — see below |

Four refusals, each with a `warn!` carrying a stable `reason` ([executor/spawn/mod.rs](../../crates/cell-content/src/cell/content/executor/spawn/mod.rs)):

1. **`actor_not_player`** — the acting entity is an NPC, so "the acting player's space" is undefined. Cover-node and NPC-death chains fire this way.
2. **`template_not_cached`** — the template id is not in the cell-side `entity_templates` cache. The cache is populated at startup precisely so the spawn is synchronous: a cell-to-base round trip would break the ordering of the `set_aggression` / `add_dialog_set` actions that follow a spawn in the same chain.
3. **`shared_world_refused`** — the acting player's world is not instanced and `allow_shared` is not `true`. This is the guardrail behind the campaign rule "mission-scoped hostile NPCs go into the player's own instance, never into the shared hub".
4. **`tag_already_live`** — an entity with that tag is already in this space. Relog-restore chains re-fire their step's actions by design, so a second spawn with the same tag is a no-op rather than a second NPC. The lookup matches **dead** entities too: a corpse still holds its tag, and resurrecting an NPC the player already killed would re-open completed content.

`respawn_secs` is forced to `None` regardless of the template column ([space_manager/spawn.rs:106-118](../../crates/cell-world/src/cell/space_manager/spawn.rs#L106-L118)). The respawn tick keys on `(ai_state, respawn_at)` and has no instance-lifetime awareness, so a revived mission NPC would re-fire its `entity_dead_tag` chain and complete a kill objective twice. Content spawns are always one-shot, so `Action::SpawnEntity` carries no respawn field at all — a `respawn_secs` param is dropped at load with a single `warn!` (`reason = "respawn_secs_not_honoured"`) rather than warning on every fire. The row is not rejected: a mission NPC that appears without respawn beats one that never appears.

One more warn worth recognising in a log: **`aggressive_spawn_faction_zero`**. Auto-aggro compares the NPC's faction against the player's, and players are always faction 0, so a hostile template with `faction = 0` or `NULL` never attacks. The fix is in the `entity_templates` row, not the chain.

Because the idempotence guard is per-tag, **wave content needs one tag per spawn** (`ra_infiltrator_1`, `_2`, …) with one `spawn_entity` row each. A shared tag both trips the guard and makes `despawn_entity` reach only one of them — `entity_dead_tag` matching is exact, not prefixed.

`despawn_entity` takes the tag in `target_key`. It and `destroy_entity` are now the same behaviour: both route through `SpaceManager::despawn_npc`, which fans `LeftAoI` to every current witness and scrubs the witness sets before destroying. `destroy_entity` previously called the bare `destroy_entity`, which dropped the entity from the space and the spatial grid but left its id in every observer's witness set — the client kept rendering a ghost until an AoI tick happened to visit that player. That is the #582 invisible-corpse shape.

`set_visible` now fans out over the target's witness list. It used to send one message addressed to the target itself, which for an NPC resolves to no client address, so **every seeded `set_visible` row was a silent no-op**. Hide and show deliberately use different primitives, mirroring the engine's `leaveAoI` / `enterAoI`: hide sends `BASEMSG_ENTITY_INVISIBLE (0x0B)` per witness, show sends `onVisible(1)` per witness. Neither direction is recorded on the entity and the AoI create cascade unconditionally appends `onVisible(1)`, so **a hide is not durable across an AoI re-entry** — it holds only for witnesses who stay in range.

#### Authorable but NOT executed — seeded rows that silently no-op

These have a loader arm, so the seed accepts them and the engine resolves them, but **[executor/mod.rs](../../crates/cell-content/src/cell/content/executor/mod.rs) has no match arm** — every one falls through to the `debug!` catch-all and does nothing. This is a live correctness gap, not a roadmap item: 4 seeded rows are currently dead.

| Seed verb | `Action` variant | Seed rows | Consequence |
|---|---|---|---|
| `qr_combat_damage` | `QrCombatDamage` | 2 | Scripted damage is never applied |
| `remove_effect` | `RemoveEffect` | 1 | No chain can strip an effect |
| `fail_objective` | `FailObjective` | 1 | Objective-fail branches never fire |

`system_message` (`SystemMessage`, **1 seeded row**: chain 1013, `castle_cellblock_chains.sql`) is a third state: it has an executor arm, but the arm only emits an `info!` log. The client wire format is still unknown — see §10.

#### Not authorable — defined in the enum, no loader arm

No `content_actions` row can name these; they are reachable only from Rust (or not at all). `PlayAnimation`, `PlaySound`, `ModifyProperty`, `RollLootTable`, `SpawnLootBag`, `StartTimer`, `CancelTimer`, `ExecuteCustom`. None has an executor arm either, so wiring any of them is a two-sided job. `GrantXP` used to head this list; it was wired on both sides in issue #611 and now appears in the executed table above with **0 seed rows** — the plumbing exists, no content uses it yet, and the seed still has `reward_xp = 0` on all 1,040 mission rows (§9). `SpawnEntity` and `DespawnEntity` left it in Harset H03, wired on both sides in the same change. `GrantStargateAddress` was added on both sides in Harset H55 and is the one entry in the table whose seed row is load-bearing on day one: it is the only way content can unlock a stargate destination, and Castle mission 708's dial step is unreachable without it.

Four variants have an executor arm but no seed verb, reached only as internal aliases or from Rust: `AdvanceMission` (aliased onto the `AcceptMission` arm), `StartDialog` (aliased onto `DisplayDialog`), `Teleport` (same-space teleport; only `cross_world_teleport` is authorable), and `TriggerChain` (resolved by the engine, re-dispatched by the caller). `SendMessage` has an arm that only logs.

See [proposed-extensions.md](proposed-extensions.md) for the wiring plan.
