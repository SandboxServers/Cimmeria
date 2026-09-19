---
name: dialog-set-bind-carries-no-dialog-id
description: An add_dialog_set bind's only client-visible push is InteractionType(UINT64 TypeId) — no dialog id on the wire; the dialog id is server-side state read on the click paths, and onInitialInteraction (104) is declared but never emitted
metadata:
  type: reference
---

**`add_dialog_set` / `add_dialog` never put a dialog id on the wire.** The bind's direct
client-visible effect is `SGWSpawnableEntity.InteractionType`, client method index **3**, whose
signature is a single `UINT64 TypeId` (`entities/defs/SGWSpawnableEntity.def:114-116`;
`docs/protocol/client-method-dispatch-table.md:80`). The payload is the merged interaction flags as
8-byte LE: `npc.interaction_type_flags | entry.interaction_flags`.

Two emit sites, same payload shape:

- `cell/content/executor/dialog.rs::send_interaction_update_if_visible` — on bind, per witnessing
  player, for every entity sharing the template.
- `cell/space_manager/aoi.rs` (~157-179) — the `dynamicUpdate` half on AoI entry, for a bind
  installed while the NPC was out of view. The AoI dynamic update also emits method 152.

**Consequences worth remembering:**

- A `dialog_set_maps` row with `dialog_id IS NULL` is currently dropped by the loader. It is
  neither bindable nor dispatchable; do not substitute 0, which opens an empty dialog box on the
  client.
- The dialog id is read only on the **click** paths: `interactions/dispatch/interact.rs`
  (per-player binds beat the static `interaction_type`) and `initial_response.rs` (matches the
  `DialogSetMapID` the client echoes, `SGWPlayer.def:629-632`).
- **`onInitialInteraction` (index 104, `ARRAY<DialogChoices>`) is declared and named in the wire
  log but has no send site anywhere in `crates/`.** So the original's "offer several bound topics,
  player picks" flow does not exist; we shortcut to the first bound dialog that has one, and
  `dialog_set_maps.topic_text` is dead data. This is why `handle_interact` has to choose a dialog
  instead of sending a list.
- `interact_tag` / `interact_template` chains are dispatched **before** `handle_interact`
  (`cell_methods/player/interaction/interact.rs` ~161-203) and short-circuit it when they match.
- `remove_dialog_set` matches on `dialog_set_map_id` and re-folds the remaining flags. A null
  dialog map is a loader cache miss, so it cannot be removed or dispatched as a bound interaction.

Related: [[witness-entity-method-dual-fn]], [[method-idx-duplicate-table-drift]],
[[chain-replay-executor-guards]].
