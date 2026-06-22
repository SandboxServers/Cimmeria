# Ignore enforcement (chat + AoI)

Implemented on branch `feat/ignore-enforcement` (worktree Cimmeria-ignore), commit ee3e576.

## Wire / method facts
- `chatIgnore` SGWPlayer base method = **0xC5** (`sgw_player_base::CHAT_IGNORE` in
  `base/dispatch/mod.rs`). Args: WSTRING playerName + UINT8 flag (1=add, 0=remove).
  The dispatch table lists only the WSTRING; the `.def` carries the flag — flag is authoritative.
- Ignore contact list: name `'Ignore'`, **flags=301** (Friends=300). Created by
  `contact_list::persistence::ensure_system_lists` (returns `(friends_id, ignore_id)`).

## Architecture (single source of truth = base Ignore contact list)
- Base side: `ConnectedClientState::ignore_set: HashSet<String>` (in `base/mod.rs`).
  Seeded in `push_contact_lists_on_login` (filters lists by flags==301), updated by
  the chatIgnore handler and by contact-list add/remove on the Ignore list.
- Cell side: `CellEntity::ignore_names: HashSet<String>` (entity_struct.rs / construction.rs).
  This is the names THIS player ignores.
- Plumbing: `BaseToCellMsg::UpdateIgnoreList { entity_id, ignore_names }`. Sent from
  (1) `world_entry_appearance::client_ready.rs` right after push_contact_lists_on_login,
  (2) chatIgnore handler, (3) `resync_ignore_after_member_change` (called from
  contact_list_dispatch route after add/remove members). Handled inline in
  `cell/service/base_messages/mod.rs` → sets `entity.ignore_names`.

## The unifier: AoI exclusion gates everything downstream
- `compute_player_aoi` (cell/space_manager/aoi.rs) excludes a PLAYER candidate if EITHER
  side ignores the other. Symmetry works because each player's own AoI pass checks both
  its own set (player ignores candidate) AND the candidate's set (candidate ignores player).
  Snapshot player's `character_name` + `ignore_names` BEFORE the candidates loop to avoid
  double-borrow of `space.entities`.
- Because witness set is filtered, all witness-iterating broadcasts (say/emote/yell,
  combat/death fanout, EntityMoved) are gated for free.
- NPCs NEVER filtered (only `other.is_player`).
- Belt-and-suspenders: `broadcast_to_witnesses` (cell/chat.rs) re-checks the symmetric
  pair per witness at send time (covers the gap between periodic AoI ticks).
- Tells (channel 9): base-side filter in `handle_send_player_communication` (base/dispatch/chat.rs).
  Tell delivery to recipients is still unimplemented (cell drops chan 9), so the guard is a
  no-op today but the seam is correct for when delivery lands.

## Gotcha
- `ConnectedClientState` has NO constructor — 5 literal construction sites must all get the
  new field: login/mod.rs, test_support.rs, world_entry/methods/inventory/appearance.rs,
  world_entry/gate_travel/tests.rs, world_entry/play_character.rs. Grep `ConnectedClientState {`.
- Console-authoring tests emit `crates/services/logs/seed-authoring-*.sql` as a side effect —
  do NOT commit these (gitignored intent; they appear as untracked after a test run).
