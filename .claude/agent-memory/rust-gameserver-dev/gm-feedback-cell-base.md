---
name: gm-feedback-cell-base
description: How GM-command feedback lines reach the GM client across the cell/base split, and the notify_gm gating pattern for shared Grant* messages
metadata:
  type: project
---

# GM feedback across cell/base

GM action handlers confirm results to the GM via `onPlayerCommunication`
(method index **28**, `crate::mercury::method_idx::ON_PLAYER_COMMUNICATION`) on
chat channel **CHAN_FEEDBACK = 8**, speaker "SYSTEM", flags 0.

Two delivery helpers (the wire serializer is duplicated in both — precedent for
duplicating small serializers across cell/base):

- **Cell-side**: `crate::cell::cell_methods::gm::feedback::send_gm_feedback(entity_id, &str, tx)`
  — emits a `CellToBaseMsg::EntityMethodCall{ method_index: 28 }` that the base
  relays to the entity's own client. `pub(crate)` so the cell `GmSpawnNpcReady`
  handler can use it too. Used for pre-dispatch rejections AND cell-confirmed
  completions (e.g. the actual NPC spawn).
- **Base-side**: `crate::base::gm_feedback::send_gm_feedback_to_client(entity_id, &str, transport, connected, entity_to_addr)`
  — `send_to_witness_reliable(...)` wrapping `build_player_entity_method_packet(..., ON_PLAYER_COMMUNICATION, &payload)`.
  Mirrors `progression::handle_grant_cash`'s client-send shape. No-ops gracefully
  if the entity has no connected client.

## "Trust, but verify": definitive vs optimistic

Base-round-trip GM commands (give/crafting/spawn) must send **definitive**
feedback AFTER the DB write commits, not optimistic "requested" from the cell.
Pattern:

- gm-only commands (`gmGiveExpertise`, `gmGiveAppliedSciencePoints`, `gmSpawnByCmd`):
  the cell removes its optimistic success line; the base handler feeds back on
  success (and failure) post-commit. For spawn, the base feeds back the
  "template not found" failure; the **cell** feeds back the "spawned npc <id>"
  success from its `GmSpawnNpcReady` handler (it's the layer that knows the new
  id and whether the spawn took). `BaseToCellMsg::GmSpawnNpcReady` carries
  `requester_entity_id` so the cell knows whom to notify.

## notify_gm gating for SHARED messages

`GrantXP`, `GrantCash`, `GrantItem`, `RemoveInventoryItem` are sent by BOTH GM
and non-GM flows (mob-kill XP, loot, content chains, player drops). Each carries
a `notify_gm: bool`. Only the GM `gm/give.rs` handlers set `true`; the base
handler fires `send_gm_feedback_to_client` only `if notify_gm`, on the true
post-commit success path. Non-GM senders (find via
`rg 'CellToBaseMsg::(GrantXP|GrantCash|GrantItem|RemoveInventoryItem)\s*\{'`):
- GrantXP: `cell/abilities/damage_apply/mod.rs`
- GrantItem: `cell/interactions/loot.rs`, `cell/content/executor/inventory.rs`
- GrantCash: `cell/interactions/loot.rs`
- RemoveInventoryItem: `cell/content/executor/inventory.rs`, `cell/cell_methods/inventory/item_ops.rs`

All Grant* construction sites live in `cimmeria-services` (none in `crates/server`).

## Gotchas

- The base `handle_grant_item` / `handle_remove_inventory_item` have MANY early
  returns (advisory lock fail, merge-candidate lookup, reserve, commit). Fire
  the GM feedback only on the true success path — `handle_grant_item` needs it
  in BOTH the stack-merge return AND the new-slot path (after commit, before the
  bandolier/visual epilogue so all remaining returns are post-commit).
- `npc_ai::stationary_no_los_or_range_emits_structured_decision_log` is a
  pre-existing parallel-only flake (global tracing `LogCapture` race); passes in
  isolation and under `--test-threads=1`. Not caused by feedback changes.
