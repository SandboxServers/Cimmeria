---
name: gm-feedback-cell-base
description: How GM-command feedback lines reach the GM client across the cell/base split, and the notify_gm / gm_feedback_to gating pattern for shared Grant* messages
metadata:
  type: project
---

# GM feedback across cell/base

GM action handlers confirm results to the GM via `onPlayerCommunication`
(method index **28**, `crate::mercury::method_idx::ON_PLAYER_COMMUNICATION`) on
chat channel **CHAN_FEEDBACK = 9**, speaker "SYSTEM", flags 0.

(Was 8 when this note was first written. It was changed because the client only
registers the channels in the base's `DEFAULT_CHAT_CHANNELS` and an
*unregistered* channel falls back to its **red unknown-channel splash popup**;
9 is the registered `tell` channel. Reasoning is in `gm_feedback.rs:14-22`.)

## Method 28 is the one non-modal text route — and its constants are a mess

Verified 2026-09-21 while adding the `npc_bark` content action (DU-03). Three
separate facts worth having before touching anything method-28 shaped:

1. **The serializer exists FOUR times** — `cell/chat.rs`
   (`serialize_on_player_communication`, now `pub(crate)` so
   `cell/content/executor/bark.rs` reuses it), `cell/cell_methods/gm/feedback.rs`
   (private copy), and an inline `write_wstring`-based build in
   `cell/console/net.rs`. Wire shape is WSTRING speaker (u32 UTF-16 code-unit
   count + N×2B LE), UINT8 SpeakerFlags, UINT8 Channel, WSTRING text. Reuse
   `cell::chat::serialize_on_player_communication`; do not add a fifth.
2. **`cell/chat.rs`'s channel constants diverge from
   `entities/defs/enumerations.xml` for every channel ≥7.** The `.def` is
   `CHAN_server=8, CHAN_feedback=9, CHAN_tell=10, CHAN_splash=11`; `chat.rs` is
   `CHAN_SERVER=7, CHAN_FEEDBACK=9, CHAN_TELL=9, CHAN_SPLASH=10`. The feedback
   value is a deliberate, documented client-reality override; `CHAN_SPLASH=10`
   looks like a plain off-by-one against canon and has no reader today. **Only
   `CHAN_say = 0` is agreed by both sources** — trust nothing else without
   re-verifying in the client.
3. **`Action::SystemMessage` is a log-only stub and must stay that way.** An
   earlier attempt routed its message id through method 28 and produced garbled
   `"[] says"` chat plus client freezes. An empty speaker WSTRING is what causes
   that, which is why `npc_bark` rejects a blank `speaker` at load.

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

## notify_gm gating for SHARED messages — and the P05 gm_feedback_to split

`GrantXP`, `GrantCash`, `GrantItem`, `RemoveInventoryItem` are sent by BOTH GM
and non-GM flows (mob-kill XP, loot, content chains, player drops). `GrantItem`
and `RemoveInventoryItem` still carry a plain `notify_gm: bool`: only the GM
`gm/give.rs` handlers set `true`; the base handler fires
`send_gm_feedback_to_client` only `if notify_gm`, on the true post-commit
success path, and it always targets the message's own `entity_id`.

**`GrantXP`/`GrantCash` were changed (P05, legacy-command-parity) to
`gm_feedback_to: Option<u32>`** instead of `notify_gm: bool`, because a GM can
now grant to a *selected target* different from themself (`.givecash`/
`.givexp` dot commands) — `entity_id` on these two messages is the grant's
DB/UI recipient (the target), which is NOT necessarily who should get the GM
feedback line (the caller). `Some(gm_entity_id)` tells the base handler to
send the definitive line to that entity specifically (not `entity_id`);
`None` means no GM feedback (mob-kill XP, loot pickup — unchanged semantics,
just a renamed variant). Native `gm/give.rs` paths (`gmGiveXp`/`gmGiveCash`,
caller grants to self) pass `Some(entity_id)` — same value as before, zero
observable behavior change.

**`GrantItem`/`RemoveInventoryItem`/`GrantExpertise`/
`GrantAppliedSciencePoints` have the identical latent conflation** the moment
a selected-target dot command is added for them (P06 `.giveitem` will hit the
`GrantItem` case immediately) — apply the same `gm_feedback_to: Option<u32>`
pattern rather than reinventing one. Non-GM senders (find via
`rg 'CellToBaseMsg::(GrantXP|GrantCash|GrantItem|RemoveInventoryItem)\s*\{'`):
- GrantXP: `cell/abilities/damage_apply/mod.rs` (`gm_feedback_to: None`)
- GrantItem: `cell/interactions/loot.rs`, `cell/content/executor/inventory.rs` (still `notify_gm: false`)
- GrantCash: `cell/interactions/loot.rs` (`gm_feedback_to: None`)
- RemoveInventoryItem: `cell/content/executor/inventory.rs`, `cell/cell_methods/inventory/item_ops.rs` (still `notify_gm: false`)

All Grant* construction sites live in `cimmeria-services` (none in `crates/server`).

When a live-DB test needs to prove the caller/target recipient split (not
just that DB persistence is correct), a `notify_gm: bool`-shaped assertion
isn't enough — build two distinct fully-connected `ConnectedClientState`
sessions at two addresses (see `world_entry/methods/inventory/appearance.rs`'s
test module for the full struct-literal fixture) and assert
`TestTransport::send_count_to(addr)` per address, not just that a DB write
happened. A test that only checks the DB row would pass even if the feedback
line were silently misrouted to the wrong client.

## Gotchas

- The base `handle_grant_item` / `handle_remove_inventory_item` have MANY early
  returns (advisory lock fail, merge-candidate lookup, reserve, commit). Fire
  the GM feedback only on the true success path — `handle_grant_item` needs it
  in BOTH the stack-merge return AND the new-slot path (after commit, before the
  bandolier/visual epilogue so all remaining returns are post-commit).
- `npc_ai::stationary_no_los_or_range_emits_structured_decision_log` is a
  pre-existing parallel-only flake (global tracing `LogCapture` race); passes in
  isolation and under `--test-threads=1`. Not caused by feedback changes.
