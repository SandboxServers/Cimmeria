---
name: project-ignore-enforcement
description: Server-side symmetric ignore enforcement — AoI exclusion + chat/tell filter. Commit ee3e5768 on feat/ignore-enforcement.
metadata:
  type: project
---

Symmetric ignore enforcement shipped in worktree `C:\Users\Steve\source\projects\Cimmeria-ignore`, branch `feat/ignore-enforcement`, commit `ee3e5768`.

## Design

**Unifier**: `compute_player_aoi` in `cell/space_manager/aoi.rs` excludes ignored player-pairs from each other's witness set. Every downstream broadcast that iterates witnesses (say/emote/yell, #278 combat/death fanout, EntityMoved, WitnessEntityMethod) is automatically gated. NPCs are never filtered (`other.is_player` gate).

**Data flow (base→cell)**:

- `ConnectedClientState::ignore_set: HashSet<String>` — base-side in-memory cache, flags=301 Ignore contact list members.
- `CellEntity::ignore_names: HashSet<String>` — cell-side mirror, seeded via `BaseToCellMsg::UpdateIgnoreList`.
- `UpdateIgnoreList` is sent from: (1) `world_entry_appearance/client_ready.rs` after `push_contact_lists_on_login`; (2) `base/dispatch/ignore.rs::handle_chat_ignore`; (3) `resync_ignore_after_member_change` for UI-driven contact-list edits to the Ignore list.

**Symmetry mechanism**: `compute_player_aoi` for player A checks: is candidate B's name in A's `ignore_names`? AND is A's name in B's `ignore_names`? Either true → exclude. B's own AoI pass runs the mirror check.

## Surfaces gated

| Surface | How |
|---|---|
| Say/emote/yell | AoI witness set (auto) + belt-and-suspenders in `broadcast_to_witnesses` |
| Combat/death fanout | AoI witness set (auto, iterates witnesses) |
| EntityMoved position | AoI witness set (auto) |
| Tells (channel 9) | Base-side filter in `handle_send_player_communication` before cell route |
| Other channels (team/squad/etc.) | Not implemented yet; guard location is channel routing in base |

## chatIgnore handler

- Wire id: `0xC5` (WSTRING playerName, UINT8 flag; 1=add, 0=remove)
- File: `crates/services/src/base/dispatch/ignore.rs`
- Reuses `handle_add_members`/`handle_remove_members` so CM 87/88 echoes update the contact-list UI
- Updates `ignore_set` in `ConnectedClientState` then sends `UpdateIgnoreList` to cell

## Key file locations

- `crates/services/src/base/dispatch/ignore.rs` — chatIgnore handler + `resync_ignore_after_member_change`
- `crates/services/src/base/dispatch/mod.rs` — CHAT_IGNORE = 0xC5 const + dispatch arm
- `crates/services/src/base/mod.rs` — `ignore_set` field on `ConnectedClientState`
- `crates/entity/src/cell_entity/entity_struct.rs` — `ignore_names` field on `CellEntity`
- `crates/services/src/cell/messages/base_to_cell.rs` — `UpdateIgnoreList` variant
- `crates/services/src/cell/service/base_messages/mod.rs` — `UpdateIgnoreList` handler
- `crates/services/src/cell/space_manager/aoi.rs` — AoI filter (lines ~70–130) + AoI tests
- `crates/services/src/cell/chat.rs` — belt-and-suspenders filter in `broadcast_to_witnesses`

## Caveat: Cimmeria addition

AoI-hide of players is a Cimmeria extension beyond original SGW behavior (which was chat-only enforcement). Recorded in commit body; needs note in social/contact-list spec chapter when drafted.

## Tests

- `cell::space_manager::aoi::tests::ignore_symmetric_excludes_pair_from_aoi`
- `cell::space_manager::aoi::tests::ignore_npc_never_filtered`
- `cell::chat::tests::ignored_witness_skipped_in_broadcast`
- `cell::chat::tests::ignore_symmetry_in_broadcast`
- `cell::chat::tests::non_ignored_witness_receives_broadcast`
- `base::dispatch::ignore::tests::tell_dropped_when_sender_ignores_target`
- `base::dispatch::ignore::tests::tell_dropped_when_target_ignores_sender`
- `base::dispatch::ignore::tests::tell_to_third_party_not_dropped`
- `base::contact_list::persistence::tests::chat_ignore_add_then_remove_updates_ignore_list` (live-DB)

**Why:** [[reference-witness-fanout-helper]]
