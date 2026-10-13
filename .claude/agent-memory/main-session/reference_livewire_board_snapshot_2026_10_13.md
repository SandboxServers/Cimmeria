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
with the registry entry while holding the registry lock, and clear the copy
when that exact session is removed. The lab response omits the ticket.

Sources: `crates/minigame/src/minigame/games/livewire/mod.rs`,
`crates/minigame/src/minigame/server/session_loop.rs`, and
`crates/minigame/src/minigame/session.rs`.
