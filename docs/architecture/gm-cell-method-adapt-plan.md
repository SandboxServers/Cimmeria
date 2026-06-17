---
title: "GM Cell-Method ADAPT Roadmap"
type: explanation
audience: engineers
last_updated: 2026-06-16
---

# GM Cell-Method ADAPT Roadmap

The native GM command surface (SGWGmPlayer cell-method tail, flattened indices
109–225) is being filled in incrementally. The full 117-method inventory, with
each method's wire args, the stock client console command, the Cimmeria
primitive, and a status (**DONE** / **REUSE** / **ADAPT** / **NEW**), lives in
the dispatch table:
[cell-method-dispatch-table.md](../protocol/cell-method-dispatch-table.md#full-gm-cell-method-inventory--cimmeria-handler-status).

- **DONE** (21) — handlers wired in [`crates/services/src/cell/cell_methods/gm/`](../../crates/services/src/cell/cell_methods/gm/),
  via #518: the 3 originally-verified (`gmGiveItem`/`gmGotoXYZ`/`gmKillTarget`),
  the 16 observable-effect REUSE commands, and — now that the feedback channel
  is landed (`gm/feedback.rs`) — the two query consumers `gmUsers` (166) and
  `testLOS` (216).
- **REUSE** (0 remaining) — all thin-handler commands are done.
- **ADAPT** (52) — this document.
- **NEW** (44) — out of scope here (new subsystems: god-mode flags, respec,
  stance, cover regen, etc.).

This doc is the **plan for the developer-useful ADAPT commands** — the ones a
content/combat/world engineer would actually reach for during iteration. It is
deliberately *not* an exhaustive spec of all 52; the goal is to identify the
small number of shared pieces of infrastructure that unblock most of them, and
to sequence the high-value handlers behind those pieces.

## How to read "ADAPT"

A command is ADAPT (not REUSE) when a close primitive exists but one of the
following stands between it and a thin handler:

- **(F) Feedback** — the command's whole purpose is to *show* the GM something;
  it needs a server→client text channel that doesn't exist on this branch yet.
- **(V) Visibility** — the primitive is `pub(super)`/private and must be widened
  (the pattern `handle_respawn` already followed for #518's `gmRespawn`).
- **(P) Param shape** — the primitive exists but for a different caller/target
  shape (e.g. acts on *self* but the GM command names a target; or debits a
  resource the GM grant must not).
- **(R) Reload granularity** — a bulk loader exists but the command wants a
  single-id reload (the `loadX` content hot-reload family).
- **(N) Name→id resolution** — the command takes a `WSTRING nameOrID` and we
  only have the numeric path wired in the cell.

Most ADAPT rows carry exactly one of these tags. Knock out the shared blocker
and a whole cluster collapses to REUSE-level effort.

---

## 1. The feedback channel — landed; it unblocks the whole query surface

**This was the single highest-leverage piece of infrastructure, and it is now
in place.** Roughly a third of the ADAPT rows are SHOW/LIST/PRINT "tell me
something" commands. They all need the same thing: a way to send a line of text
to **one** GM client. None of them are hard now that it exists.

The landed helper is [`gm/feedback.rs`](../../crates/services/src/cell/cell_methods/gm/feedback.rs)
`send_gm_feedback(entity_id, &str, tx)` — an `EntityMethodCall` to the GM's own
client carrying `onPlayerCommunication` on `CHAN_FEEDBACK` (channel 8). It was
ported from the abandoned chat-command PR #517 (whose chat *interception* layer
is superseded by the native console — the client consumes `/`-commands
client-side — but whose feedback channel is reusable infra). Its first two
consumers, `gmUsers` (166) and `testLOS` (216), shipped with it.

A higher-fidelity alternative for specific commands is the native `onShow*`
client-method tail (SGWGmPlayer client indices 157+), where the client has
bespoke renderers — more wire surface per command, adopt per-command if the
plain feedback line isn't rich enough.

### Query cluster (now unblocked by the landed feedback helper)

`gmUsers` and `testLOS` are **DONE**. The rest of the cluster is a batch of
read-the-field + `send_gm_feedback(...)` one-liners:

| Idx | Method | Reads | Tags |
|-----|--------|-------|------|
| 166 | `gmUsers` — **DONE** | `all_player_entity_ids` (space-scoped) | — |
| 216 | `testLOS` — **DONE** | `has_line_of_sight` | — |
| 113 | `gmMissionList` | `entity.missions.active_missions` | F |
| 114 | `gmMissionListFull` | `entity.missions.all_missions` | F |
| 115 | `gmMissionDetails` | `entity.missions.get_mission` | F, N |
| 121 | `gmShowTargetLocation` — **DONE** | `CellEntity.position` | — |
| 122 | `gmShowRotation` — **DONE** | `CellEntity.direction` | — |
| 123 | `listAbilities` | `entity.abilities.known_ability_ids` | F |
| 125 | `gmShowFlag` | `state_field` bit test | F |
| 126 | `gmListInteractions` | `available_interactions` | F |
| 127 | `gmGetMobAttribute` | `get_entity` (hand-mapped attrs) | F, P |
| 128 | `gmShowMobCount` | iterate space entities | F |
| 131 | `gmShowPlayer` — **DONE** | entity-info dump (FanMMORPG `.info`) | — |
| 168 | `gmPrintStats` | per-entity `stat_list` | F |
| 180 | `gmDebugMobData` | `get_entity` dump | F |

**Effort after the helper exists:** each is read-the-field + format + one
`gm_feedback(...)` call. `gmGetMobAttribute` (127) additionally needs a small
hand-written attribute name→field map (no reflection in Rust); start with the
attrs a designer actually inspects (health, level, faction, aiState).

---

## 2. Spawn / teleport-by-name (the daily-driver mutate commands)

These are the commands a world/content dev uses constantly. They reuse
primitives #518 already touches; the adaptation is mostly name/id resolution
and one wrapper for "act on another entity."

| Idx | Method (args) | Primitive | Tags | Note |
|-----|---------------|-----------|------|------|
| 185 | `gmSpawnByCmd(WSTRING DesignId, FLOAT xOff, zOff)` | `spawn_npc_from_record_in_space` | N | Resolve DesignId→`SpawnRecord`; spawn at caller pos + offset. Highest-value spawn command. |
| 160 | `gmGoto(WSTRING nameOrID)` | `same_world_teleport` | N | Numeric-id → same path as `gmGotoXYZ` to the target's position; name resolution later. |
| 161 | `gmSummon(WSTRING nameOrID)` | `same_world_teleport` (applied to the *other* entity) | N, P | Inverse of `gmGoto`: move the named entity to the caller. Needs a "move other" wrapper around the teleport primitive. |
| 109 | `gmMissionAssign(WSTRING DesignID, UINT8 popup)` | `accept_mission` | N | DesignID→id, then the existing accept path. |
| 118 | `gmMissionComplete(WSTRING DesignID, INT8 turnIn)` | `complete_mission_direct` | N, P | Does **not** fire rewards today; if `turnIn` is set, must route the reward dispatch. |
| 111 | `gmMissionClearActive()` | loop `abandon_mission` over `active_missions()` | — | Pure REUSE-of-a-loop; trivially promotable. |

`gmSpawnByCmd` (185), `gmGoto` (160), and `gmMissionAssign` (109) are the top
three to land — they cover spawn / move / quest, the three things a dev pokes at
most. All three share the **name→id resolution (N)** blocker; a single
`resolve_design_id(table, &str) -> Option<i32>` helper (numeric fast-path +
optional name lookup against the relevant def cache) services all of them.

---

## 3. Set target/self state (param-shape wrappers)

Mutators where the primitive exists but acts on the wrong shape, or touches a
resource the GM path must handle differently.

| Idx | Method | Primitive | Tags | Note |
|-----|--------|-----------|------|------|
| 146 | `gmSetSpeed(FLOAT mult)` | `set_current(MOVEMENT_SPEED_MOD)` | — | Same shape as #518's `gmSetHealth`; reuse `gm/stats.rs::set_stat` with a float→stat mapping. Near-REUSE. |
| 152 | `gmSetLevel(INT32 level)` | `scale_for_level` + level write + recompute | P | No single "set level" fn; needs level write → `scale_for_level` → stat recompute → burst. |
| 136 | `gmGiveAbility(INT32 abilityID)` | `handle_train_ability` | P | Existing path **debits a training point**; GM grant needs a no-debit variant. |
| 151 | `gmSetFlag(INT32 flagId, UINT8 force)` | `set_state_flag` | P | Ref-counted setter; a raw force-set needs care (force==2 is "toggle"). |
| 188 | `gmSetMobAttribute(INT32 target, WSTRING attr, ..., INT32 val)` | `get_entity_mut` | P | Write side of `gmGetMobAttribute`; same hand-mapped attr table. |
| 187 | `gmRechargeItem(INT32 itemId)` | vendor `recharge.rs` | V, P | Base-scoped; needs a GM cell→base route (new `CellToBaseMsg`). |
| 212 | `spawnEntityLoot(INT32 entity, LootTableID)` | `generate_loot_on_death` (`pub(super)`) | V | Widen visibility; call against a named entity. |

`gmSetSpeed` (146) is the quick win here — it is structurally identical to the
set-stat handlers #518 already shipped.

---

## 4. Debug toggles & AI state (small, self-contained)

| Idx | Method | Primitive | Tags | Note |
|-----|--------|-----------|------|------|
| 170 | `gmDebugCombat()` | `ability_manager.rs` stub | — | Promote the existing log-only stub to a real per-caller debug flag. |
| 172 | `gmDebugHeal()` | `combatant.rs` stub | — | Same. |
| 206 | `enterErrorAIState()` | `npc_ai.rs` + `AiState::Error` | V | Set the target NPC's `ai_state`; primitive exists via content `SetNpcAiState`. |
| 207 | `exitErrorAIState()` | clear `AiState::Error` | V | Inverse of 206. |
| 211 | `gmShowNavigation(INT8 on)` | `navmesh` readable | F | Overlay needs a client callback; pair with the feedback work. |
| 222 | `sendGMShout(UINT8 global, WSTRING text)` | `broadcast_to_witnesses` | P | Needs a space-wide / all-shard variant of the chat broadcast. |

206/207 are a natural pair and useful for AI debugging; they only need the
`AiState::Error` write exposed to a GM entry point.

---

## 5. Content hot-reload (`loadX` family) — high dev value, shared blocker

The `loadX` commands let a designer reload a single def **without a server
restart** — extremely valuable for content iteration. Every one of them has a
**bulk** loader today; the adaptation (tag **R**) is to add a single-id reload
path alongside the existing full reload.

| Idx | Method | Bulk loader (full reload exists) |
|-----|--------|----------------------------------|
| 197 | `loadAbility(INT32 id)` | `spawner/abilities.rs load_ability_defs` |
| 199 | `loadAbilitySet(INT32 id)` | `spawner/abilities.rs load_archetype_ability_trees` |
| 202 | `loadDialogSet(INT32 id)` | `spawner/dialogs.rs load_dialog_set_maps` |
| 203 | `loadItem(INT32 id)` | `spawner/loot.rs load_item_defs` (weapons only today) |
| 204 | `loadMission(INT32 id)` | `spawner/missions.rs load_mission_defs` |

**Pragmatic first cut:** wire each `loadX(id)` to the *existing bulk reload*
(ignore the id, reload everything) and `log!`/feedback that it did a full
reload. That delivers the dev value (pick up edited content live) immediately,
with a single-id fast-path as a later optimization. Mark the id-ignoring clearly
so it isn't mistaken for a targeted reload.

---

## Recommended sequencing

1. ~~**`gm/feedback.rs`** (`CHAN_FEEDBACK` helper)~~ — **done** (#518); `gmUsers`
   + `testLOS` shipped on it.
2. **Query cluster** (§1) — batch of read-the-field + `send_gm_feedback` one-liners,
   now unblocked.
3. **`resolve_design_id` helper + `gmSpawnByCmd` / `gmGoto` / `gmMissionAssign`**
   (§2) — the daily-driver mutate commands.
4. **`gmSetSpeed`** (§3) — quick win, mirrors the shipped set-stat handlers.
5. **`loadX` family** (§5) — full-reload first cut for live content iteration.
6. Remaining §3/§4 param-shape wrappers as needed.

## Conventions for any new handler

Per the dispatch-table "How to add a handler" note and #518's established
pattern:

- Add the index constant + a match arm in
  [`gm/mod.rs`](../../crates/services/src/cell/cell_methods/gm/mod.rs), and pin
  the offset in `tests::gm_indices_match_def_document_order`.
- Put the handler in the right family submodule (`give`/`stats`/`missions`/
  `travel`/`world`, or a new `feedback`/`query`/`spawn` module).
- **Do not re-check access level** — the `gm_gate` already authorized the caller
  for every index ≥ 109.
- Apply defense-in-depth input validation (clamp, reject non-finite, NPC-only,
  same-space) even though the channel is GM-only — a modified client can still
  send garbage on it.
- Reuse the canonical primitive; widen its visibility (`pub(crate)`) rather than
  duplicating its logic, exactly as `gmRespawn` reuses combat `handle_respawn`.
