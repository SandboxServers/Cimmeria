---
name: minigame-session-lifecycle
description: SmartFox minigame session ticket ownership, client-initiated close via endCurrentMinigame, and the late onEndMinigame race
metadata:
  type: reference
---

- The authenticated SmartFox task owns the session ticket. Its teardown uses `SessionRegistry::remove_if_ticket`; an entity-only remove can delete a replacement board after a quick close and reopen.
- The SGW client's minigame X sends `endCurrentMinigame` on the Mercury cell channel. Hiding the SWF window does not necessarily close its SmartFox TCP socket, so cancellation must explicitly signal the task and free the registry entry. The task reports Canceled (0), distinct from Defeat (2). See `crates/cell-methods/src/cell/cell_methods/minigame.rs` and `crates/minigame/src/minigame/server/session_loop.rs`.
- For a client-initiated window close, suppress the late `onEndMinigame`: it can close a newly opened board. A simultaneous SmartFox FIN can race the Mercury close, so prioritize the cancel watch and check it again before reporting an aborted result. Carry the session ticket in `CellToBaseMsg::MinigameResult`; Base skips UI teardown if a replacement ticket is registered, covering channel backpressure after FIN. Other outcomes still notify the client.
- `SetInteractionType` mutates a space entity and fans to every witness. Clearing the Preparation terminal's Livewire cue in one player's victory chain prevents another player at mission 641 step 3564 from launching. Keep the shared cue; the per-player step condition gates the trigger.
