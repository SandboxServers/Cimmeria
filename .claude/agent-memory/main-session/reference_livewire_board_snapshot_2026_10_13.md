---
name: Livewire board snapshot ownership
description: Why lab board reads are published from the SmartFox game instance into the session registry
type: reference
---

As of 2026-10-13, the generated Livewire wires live only in `LivewireGame`,
held by `Box<dyn MinigameInstance>` in `minigame/server/session_loop.rs`.
`SessionRegistry` previously held session parameters and tickets, but no board.
The lab read therefore needs a bounded copy published by the game loop after
`started`, `message`, and `tick`. Compare the publishing connection's ticket
with the registry entry while holding the registry lock. The lab response
omits the ticket. A capped command/lifecycle ring accompanies the board, and
the final outcome remains readable for ten minutes after teardown. A new
registration for the same entity replaces that retained view.

Sources: `crates/minigame/src/minigame/games/livewire/mod.rs`,
`crates/minigame/src/minigame/server/session_loop.rs`, and
`crates/minigame/src/minigame/session.rs`.

Live lab check (2026-10-13): a Praxis Commando entered Cellblock Region2,
finished Prisoner 329's dialog, and interacted with the door at mission 638
step 2115. The real client opened Livewire. `server_minigame_state` returned a
generated two-goal board (12 wires, including `g1`, `g2`, and `pe1`), with
library names, positions, and depths. Clicking the SWF's Open button changed
the snapshot from `ready` to `running` and recorded `opendoor`; the ended
session retained its outcome and event history. Staging directly on step 2115
did not enable the door because that omits dialog chain 1015's interaction
action, so use the normal Region2/dialog route for subsequent live checks.
